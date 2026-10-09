//! A vector tile drawn by a style (docs/adr/0208 §9): each of the style's
//! layers in order (its order is the batches' level), its features filtered
//! and clipped to the tile, taken to the project's system by the caller's
//! transformation, and given to the style engine's batch sink: areas as
//! fills (an outline as a hairline), lines as strokes (dashes in line
//! widths, a gap as a wider band), circles as markers; a symbol layer's
//! text becomes label candidates for the frame's placement (`labels`). The
//! paint is evaluated at the frame's zoom (`zoom`), so a tile is built again
//! when the zoom moves on by a quarter level.

use kentos_geometry_core::Vec2;
use kentos_style_core::style::batch::{BatchSink, Batches};
use kentos_style_core::style::prim::{
    Common, FillPaint, Look, MarkerStyle, PrimUnit, Sink, StrokeStyle,
};

use super::color::{self, Rgba};
use super::expr::{Ctx, Feature, Val, eval};
use super::{LayerKind, Style, StyleLayer};
use crate::labels::{Anchor, Candidate, LabelStyle, Placement};
use crate::mvt::{self, Geometry, P};

/// A tile's features as the expressions read them.
struct View<'a> {
    layer: &'a mvt::Layer,
    feature: &'a mvt::Feature,
    kind: &'static str,
}

impl Feature for View<'_> {
    fn property(&self, key: &str) -> Option<Val> {
        self.layer.get(self.feature, key).map(|v| match v {
            mvt::Value::Text(s) => Val::Str(s.clone()),
            mvt::Value::Number(n) => Val::Num(*n),
            mvt::Value::Bool(b) => Val::Bool(*b),
        })
    }

    fn geometry_type(&self) -> &str {
        self.kind
    }

    fn id(&self) -> Option<f64> {
        self.feature.id.map(|i| i as f64)
    }
}

fn kind_of(g: &Geometry) -> &'static str {
    match g {
        Geometry::Points(p) if p.len() > 1 => "MultiPoint",
        Geometry::Points(_) => "Point",
        Geometry::Lines(l) if l.len() > 1 => "MultiLineString",
        Geometry::Lines(_) => "LineString",
        Geometry::Polygons(p) if p.len() > 1 => "MultiPolygon",
        Geometry::Polygons(_) => "Polygon",
        Geometry::None => "Unknown",
    }
}

/// What a tile is built with.
pub struct TileInput<'a> {
    pub layers: &'a [mvt::Layer],
    pub style: &'a Style,
    /// The vector source the tile is of.
    pub source: &'a str,
    /// The zoom the paint is evaluated at (512-pixel tiles' zoom, as MapLibre's).
    pub zoom: f64,
    /// A point of the tile, `(u, v)` from its top left (0 to 1), in the
    /// project's system; none where it has no place there.
    pub to_project: &'a dyn Fn(f64, f64) -> Option<Vec2>,
    /// The batches' origin (the drawing's anchor, docs/adr/0157).
    pub origin: Vec2,
    /// The tile's identity for its labels.
    pub tile: u64,
}

/// A tile built: its batches and its label candidates.
pub struct TileOutput {
    pub batches: Batches,
    pub labels: Vec<Candidate>,
}

fn num(layer: &StyleLayer, paint: bool, name: &str, cx: &Ctx<'_>, default: f64) -> f64 {
    let e = if paint {
        layer.paint(name)
    } else {
        layer.layout(name)
    };
    e.and_then(|e| eval(e, cx).num()).unwrap_or(default)
}

fn col(layer: &StyleLayer, name: &str, cx: &Ctx<'_>, default: Rgba) -> Rgba {
    layer
        .paint(name)
        .and_then(|e| eval(e, cx).color())
        .unwrap_or(default)
}

fn text(layer: &StyleLayer, name: &str, cx: &Ctx<'_>) -> Option<String> {
    layer.layout(name).map(|e| eval(e, cx).text())
}

/// Clips a ring to the square [0, extent]² (Sutherland–Hodgman).
fn clip_ring(ring: &[P], extent: f64) -> Vec<[f64; 2]> {
    let mut poly: Vec<[f64; 2]> = ring
        .iter()
        .map(|p| [f64::from(p[0]), f64::from(p[1])])
        .collect();
    let edges: [(usize, f64, bool); 4] = [
        (0, 0.0, true),
        (0, extent, false),
        (1, 0.0, true),
        (1, extent, false),
    ];
    for (axis, at, low) in edges {
        if poly.is_empty() {
            break;
        }
        let inside = |p: &[f64; 2]| if low { p[axis] >= at } else { p[axis] <= at };
        let mut out = Vec::with_capacity(poly.len() + 4);
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            let (ia, ib) = (inside(&a), inside(&b));
            if ia {
                out.push(a);
            }
            if ia != ib {
                let t = (at - a[axis]) / (b[axis] - a[axis]);
                out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
        poly = out;
    }
    poly
}

/// Clips a line to the square, into the pieces inside (Liang–Barsky per segment).
fn clip_line(line: &[P], extent: f64) -> Vec<Vec<[f64; 2]>> {
    let mut pieces: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut current: Vec<[f64; 2]> = Vec::new();
    for w in line.windows(2) {
        let (a, b) = (
            [f64::from(w[0][0]), f64::from(w[0][1])],
            [f64::from(w[1][0]), f64::from(w[1][1])],
        );
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut keep = true;
        for (p, q) in [
            (-dx, a[0]),
            (dx, extent - a[0]),
            (-dy, a[1]),
            (dy, extent - a[1]),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    keep = false;
                    break;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    t0 = t0.max(r);
                } else {
                    t1 = t1.min(r);
                }
            }
        }
        if !keep || t0 > t1 {
            if current.len() >= 2 {
                pieces.push(std::mem::take(&mut current));
            }
            current.clear();
            continue;
        }
        let s = [a[0] + dx * t0, a[1] + dy * t0];
        let e = [a[0] + dx * t1, a[1] + dy * t1];
        if current.last() != Some(&s) {
            if current.len() >= 2 {
                pieces.push(std::mem::take(&mut current));
            }
            current.clear();
            current.push(s);
        }
        current.push(e);
        if t1 < 1.0 {
            if current.len() >= 2 {
                pieces.push(std::mem::take(&mut current));
            }
            current.clear();
        }
    }
    if current.len() >= 2 {
        pieces.push(current);
    }
    pieces
}

/// The pole of inaccessibility of a polygon (Mapbox's polylabel): the point
/// inside farthest from its edges, to `precision`; the label of an area.
pub fn pole(rings: &[Vec<[f64; 2]>], precision: f64) -> Option<[f64; 2]> {
    let outer = rings.first()?;
    if outer.len() < 3 {
        return None;
    }
    let (mut x1, mut y1, mut x2, mut y2) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in outer {
        x1 = x1.min(p[0]);
        y1 = y1.min(p[1]);
        x2 = x2.max(p[0]);
        y2 = y2.max(p[1]);
    }
    let size = (x2 - x1).min(y2 - y1);
    if size <= 0.0 {
        return Some([x1, y1]);
    }
    // Signed distance from p to the polygon (positive inside).
    let dist = |p: [f64; 2]| {
        let mut inside = false;
        let mut best = f64::INFINITY;
        for ring in rings {
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                if (a[1] > p[1]) != (b[1] > p[1])
                    && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
                {
                    inside = !inside;
                }
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let len = dx * dx + dy * dy;
                let t = if len > 0.0 {
                    (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (qx, qy) = (a[0] + dx * t - p[0], a[1] + dy * t - p[1]);
                best = best.min(qx * qx + qy * qy);
            }
        }
        if inside { best.sqrt() } else { -best.sqrt() }
    };
    struct Cell {
        c: [f64; 2],
        h: f64,
        d: f64,
        max: f64,
    }
    let cell = |c: [f64; 2], h: f64| {
        let d = dist(c);
        Cell {
            c,
            h,
            d,
            max: d + h * std::f64::consts::SQRT_2,
        }
    };
    let mut queue: Vec<Cell> = Vec::new();
    let h = size / 2.0;
    let mut x = x1;
    while x < x2 {
        let mut y = y1;
        while y < y2 {
            queue.push(cell([x + h, y + h], h));
            y += size;
        }
        x += size;
    }
    let mut best = cell([(x1 + x2) / 2.0, (y1 + y2) / 2.0], 0.0);
    let mut steps = 0;
    while !queue.is_empty() && steps < 4096 {
        steps += 1;
        // The cell that could hold the farthest point.
        let i = queue
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.max.total_cmp(&b.1.max))
            .map_or(0, |(i, _)| i);
        let c = queue.swap_remove(i);
        if c.d > best.d {
            best = Cell {
                c: c.c,
                h: 0.0,
                d: c.d,
                max: c.d,
            };
        }
        if c.max - best.d <= precision {
            continue;
        }
        let h = c.h / 2.0;
        for (dx, dy) in [(-h, -h), (h, -h), (-h, h), (h, h)] {
            queue.push(cell([c.c[0] + dx, c.c[1] + dy], h));
        }
    }
    Some(best.c)
}

/// Draws a tile's features by the style into batches and label candidates.
pub fn build(input: &TileInput<'_>) -> TileOutput {
    let mut sink = BatchSink::new(input.origin);
    let mut labels = Vec::new();
    let zoom_cx = Ctx::new(input.zoom, None);
    let mut projected: Vec<Vec2> = Vec::new();
    for (index, layer) in input.style.layers.iter().enumerate() {
        if !layer.shows_at(input.zoom) {
            continue;
        }
        let level = index as f64;
        if layer.kind == LayerKind::Background {
            let c = col(layer, "background-color", &zoom_cx, [0.0, 0.0, 0.0, 1.0]);
            let o = num(layer, true, "background-opacity", &zoom_cx, 1.0) * c[3];
            let ring: Vec<Vec2> = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
                .iter()
                .filter_map(|&(u, v)| (input.to_project)(u, v))
                .collect();
            if ring.len() == 4 && o > 0.0 {
                sink.fill(
                    &FillPaint::Solid {
                        color: color::hex(c),
                        opacity: o,
                        level,
                    },
                    &[ring],
                );
            }
            continue;
        }
        if layer.source.as_deref() != Some(input.source) {
            continue;
        }
        let Some(source_layer) = layer.source_layer.as_deref() else {
            continue;
        };
        let Some(tile_layer) = input.layers.iter().find(|l| l.name == source_layer) else {
            continue;
        };
        let extent = f64::from(tile_layer.extent);
        let to = |p: [f64; 2]| (input.to_project)(p[0] / extent, p[1] / extent);
        for f in &tile_layer.features {
            let view = View {
                layer: tile_layer,
                feature: f,
                kind: kind_of(&f.geometry),
            };
            let cx = Ctx::new(input.zoom, Some(&view));
            if let Some(filter) = &layer.filter
                && !matches!(eval(filter, &cx), Val::Bool(true))
            {
                continue;
            }
            match (layer.kind, &f.geometry) {
                (LayerKind::Fill | LayerKind::FillExtrusion, Geometry::Polygons(polys)) => {
                    let extrusion = layer.kind == LayerKind::FillExtrusion;
                    let (cname, oname) = if extrusion {
                        ("fill-extrusion-color", "fill-extrusion-opacity")
                    } else {
                        ("fill-color", "fill-opacity")
                    };
                    if layer.paint("fill-pattern").is_some() && layer.paint(cname).is_none() {
                        continue;
                    }
                    let c = col(layer, cname, &cx, [0.0, 0.0, 0.0, 1.0]);
                    let o = num(layer, true, oname, &cx, 1.0) * c[3];
                    let outline = (!extrusion)
                        .then(|| {
                            layer
                                .paint("fill-outline-color")
                                .and_then(|e| eval(e, &cx).color())
                        })
                        .flatten();
                    for poly in polys {
                        let rings: Vec<Vec<Vec2>> = poly
                            .iter()
                            .map(|r| {
                                clip_ring(r, extent)
                                    .into_iter()
                                    .filter_map(to)
                                    .collect::<Vec<_>>()
                            })
                            .filter(|r: &Vec<Vec2>| r.len() >= 3)
                            .collect();
                        if rings.is_empty() {
                            continue;
                        }
                        if o > 0.0 {
                            sink.fill(
                                &FillPaint::Solid {
                                    color: color::hex(c),
                                    opacity: o,
                                    level,
                                },
                                &rings,
                            );
                        }
                        if let Some(oc) = outline {
                            for r in &rings {
                                sink.stroke(&hairline(oc, o, level), r, true);
                            }
                        }
                    }
                }
                (LayerKind::Line, Geometry::Lines(_) | Geometry::Polygons(_)) => {
                    let c = col(layer, "line-color", &cx, [0.0, 0.0, 0.0, 1.0]);
                    let o = num(layer, true, "line-opacity", &cx, 1.0) * c[3];
                    let width = num(layer, true, "line-width", &cx, 1.0).max(0.0);
                    let gap = num(layer, true, "line-gap-width", &cx, 0.0).max(0.0);
                    let total = if gap > 0.0 { gap + 2.0 * width } else { width };
                    if o <= 0.0 || total <= 0.0 {
                        continue;
                    }
                    let dash = layer
                        .paint("line-dasharray")
                        .map(|e| eval(e, &cx))
                        .and_then(|v| match v {
                            Val::Arr(a) => {
                                let d: Vec<f64> = a
                                    .iter()
                                    .filter_map(Val::num)
                                    .map(|x| x * width.max(1.0))
                                    .collect();
                                (d.len() >= 2 && d.iter().any(|x| *x > 0.0)).then_some(d)
                            }
                            _ => None,
                        });
                    let cap = text(layer, "line-cap", &cx).unwrap_or_else(|| "butt".into());
                    let join = text(layer, "line-join", &cx).unwrap_or_else(|| "miter".into());
                    let style = StrokeStyle {
                        color: color::hex(c),
                        opacity: o,
                        width: total,
                        unit: PrimUnit::Px,
                        dash,
                        dash_offset: 0.0,
                        cap,
                        join,
                        blur: num(layer, true, "line-blur", &cx, 0.0).max(0.0),
                        level,
                    };
                    let lines: Vec<Vec<P>> = match &f.geometry {
                        Geometry::Lines(l) => l.clone(),
                        Geometry::Polygons(p) => p
                            .iter()
                            .flatten()
                            .map(|r| {
                                let mut c = r.clone();
                                if let Some(first) = r.first() {
                                    c.push(*first);
                                }
                                c
                            })
                            .collect(),
                        _ => Vec::new(),
                    };
                    for line in &lines {
                        for piece in clip_line(line, extent) {
                            projected.clear();
                            projected.extend(piece.into_iter().filter_map(to));
                            if projected.len() >= 2 {
                                sink.stroke(&style, &projected, false);
                            }
                        }
                    }
                }
                (LayerKind::Circle, Geometry::Points(points)) => {
                    let c = col(layer, "circle-color", &cx, [0.0, 0.0, 0.0, 1.0]);
                    let o = num(layer, true, "circle-opacity", &cx, 1.0) * c[3];
                    let r = num(layer, true, "circle-radius", &cx, 5.0).max(0.0);
                    let sw = num(layer, true, "circle-stroke-width", &cx, 0.0).max(0.0);
                    let sc = col(layer, "circle-stroke-color", &cx, [0.0, 0.0, 0.0, 1.0]);
                    if o <= 0.0 || r <= 0.0 {
                        continue;
                    }
                    let style = MarkerStyle {
                        look: Look::Shape {
                            shape: "circle".into(),
                            size: 2.0 * r,
                            height: 2.0 * r,
                            fill: Some(color::hex(c)),
                            stroke: (sw > 0.0).then(|| color::hex(sc)),
                            stroke_width: sw,
                            params: [0.0; 4],
                        },
                        common: Common {
                            unit: PrimUnit::Px,
                            opacity: o,
                            offset: [0.0, 0.0],
                            anchor: "center".into(),
                            rotation: 0.0,
                            level,
                        },
                    };
                    for p in points {
                        let q = [f64::from(p[0]), f64::from(p[1])];
                        if (0.0..extent).contains(&q[0])
                            && (0.0..extent).contains(&q[1])
                            && let Some(at) = to(q)
                        {
                            sink.marker(&style, at, 0.0);
                        }
                    }
                }
                (LayerKind::Symbol, geometry) => {
                    if let Some(c) = candidate(layer, index, &cx, geometry, extent, &to, input.tile)
                    {
                        labels.extend(c);
                    }
                }
                _ => {}
            }
        }
    }
    TileOutput {
        batches: sink.finish(),
        labels,
    }
}

fn hairline(c: Rgba, opacity: f64, level: f64) -> StrokeStyle {
    StrokeStyle {
        color: color::hex(c),
        opacity: opacity * c[3],
        width: 1.0,
        unit: PrimUnit::Px,
        dash: None,
        dash_offset: 0.0,
        cap: "butt".into(),
        join: "miter".into(),
        blur: 0.0,
        level,
    }
}

/// `{name}` tokens of a legacy text field replaced by the feature's values.
fn tokens(text: &str, cx: &Ctx<'_>) -> String {
    if !text.contains('{') {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        match rest[open..].find('}') {
            Some(close) => {
                let key = &rest[open + 1..open + close];
                if let Some(f) = cx.feature {
                    out.push_str(&f.property(key).map(|v| v.text()).unwrap_or_default());
                }
                rest = &rest[open + close + 1..];
            }
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// A symbol layer's labels of a feature.
fn candidate(
    layer: &StyleLayer,
    index: usize,
    cx: &Ctx<'_>,
    geometry: &Geometry,
    extent: f64,
    to: &dyn Fn([f64; 2]) -> Option<Vec2>,
    tile: u64,
) -> Option<Vec<Candidate>> {
    let field = layer.layout("text-field")?;
    let mut t = tokens(&eval(field, cx).text(), cx);
    match text(layer, "text-transform", cx).as_deref() {
        Some("uppercase") => t = t.to_uppercase(),
        Some("lowercase") => t = t.to_lowercase(),
        _ => {}
    }
    let t = t.trim().to_owned();
    if t.is_empty() {
        return None;
    }
    let fonts = layer
        .layout("text-font")
        .map(|e| eval(e, cx))
        .map(|v| v.text().to_ascii_lowercase())
        .unwrap_or_default();
    let size = num(layer, false, "text-size", cx, 16.0).max(1.0);
    let c = col(layer, "text-color", cx, [0.0, 0.0, 0.0, 1.0]);
    let halo_width = num(layer, true, "text-halo-width", cx, 0.0).max(0.0);
    let halo = (halo_width > 0.0).then(|| {
        (
            col(layer, "text-halo-color", cx, [0.0, 0.0, 0.0, 0.0]),
            halo_width,
        )
    });
    let anchor = Anchor::from_name(
        text(layer, "text-anchor", cx)
            .as_deref()
            .unwrap_or("center"),
    );
    let offset = layer
        .layout("text-offset")
        .map(|e| eval(e, cx))
        .and_then(|v| match v {
            Val::Arr(a) if a.len() == 2 => Some([a[0].num()?, a[1].num()?]),
            _ => None,
        })
        .unwrap_or([0.0, 0.0]);
    let style = LabelStyle {
        size,
        color: c,
        opacity: num(layer, true, "text-opacity", cx, 1.0),
        halo,
        bold: fonts.contains("bold"),
        italic: fonts.contains("italic"),
        letter_spacing: num(layer, false, "text-letter-spacing", cx, 0.0),
        max_width: num(layer, false, "text-max-width", cx, 10.0),
        anchor,
        offset,
        padding: num(layer, false, "text-padding", cx, 2.0),
        allow_overlap: matches!(
            layer.layout("text-allow-overlap").map(|e| eval(e, cx)),
            Some(Val::Bool(true))
        ),
        priority: index as u32,
        layer: layer.id.clone(),
    };
    let placement = text(layer, "symbol-placement", cx).unwrap_or_else(|| "point".into());
    let spacing = num(layer, false, "symbol-spacing", cx, 250.0).max(1.0);
    let inside = |p: &[f64; 2]| (0.0..extent).contains(&p[0]) && (0.0..extent).contains(&p[1]);
    let mut out = Vec::new();
    match geometry {
        Geometry::Lines(lines) if placement != "point" => {
            for line in lines {
                for piece in clip_line(line, extent) {
                    let path: Vec<Vec2> = piece.into_iter().filter_map(to).collect();
                    if path.len() >= 2 {
                        out.push(Candidate {
                            text: t.clone(),
                            style: style.clone(),
                            place: Placement::Line {
                                path,
                                spacing,
                                center: placement == "line-center",
                            },
                            tile,
                        });
                    }
                }
            }
        }
        Geometry::Points(points) => {
            for p in points {
                let q = [f64::from(p[0]), f64::from(p[1])];
                if inside(&q)
                    && let Some(at) = to(q)
                {
                    out.push(Candidate {
                        text: t.clone(),
                        style: style.clone(),
                        place: Placement::Point(at),
                        tile,
                    });
                }
            }
        }
        Geometry::Polygons(polys) => {
            for poly in polys {
                let rings: Vec<Vec<[f64; 2]>> = poly
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|p| [f64::from(p[0]), f64::from(p[1])])
                            .collect()
                    })
                    .collect();
                if let Some(q) = pole(&rings, 16.0).filter(inside)
                    && let Some(at) = to(q)
                {
                    out.push(Candidate {
                        text: t.clone(),
                        style: style.clone(),
                        place: Placement::Point(at),
                        tile,
                    });
                }
            }
        }
        Geometry::Lines(lines) => {
            for line in lines {
                if let Some(first) = line.first() {
                    let q = [f64::from(first[0]), f64::from(first[1])];
                    if inside(&q)
                        && let Some(at) = to(q)
                    {
                        out.push(Candidate {
                            text: t.clone(),
                            style: style.clone(),
                            place: Placement::Point(at),
                            tile,
                        });
                    }
                }
            }
        }
        Geometry::None => {}
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mvt::tests::tile;

    #[test]
    fn clipping_keeps_what_is_inside() {
        let r = clip_ring(&[[-10, -10], [20, -10], [20, 20], [-10, 20]], 10.0);
        assert_eq!(r.len(), 4);
        assert!(
            r.iter()
                .all(|p| (0.0..=10.0).contains(&p[0]) && (0.0..=10.0).contains(&p[1]))
        );
        let l = clip_line(&[[-5, 5], [15, 5], [15, 20], [5, 20], [5, 5]], 10.0);
        assert_eq!(
            l,
            vec![vec![[0.0, 5.0], [10.0, 5.0]], vec![[5.0, 10.0], [5.0, 5.0]]]
        );
        let p = pole(
            &[vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]],
            0.1,
        )
        .unwrap();
        assert!((p[0] - 5.0).abs() < 0.2 && (p[1] - 5.0).abs() < 0.2);
    }

    #[test]
    fn a_tile_by_a_style() {
        let style = super::super::parse(
            r##"{"version":8,"sources":{"s":{"type":"vector","tiles":["https://x/{z}/{x}/{y}.pbf"]}},"layers":[
              {"id":"bg","type":"background","paint":{"background-color":"#f8f4f0"}},
              {"id":"roads","type":"line","source":"s","source-layer":"l","filter":["==","class","primary"],
               "paint":{"line-color":"#ff0000","line-width":["interpolate",["linear"],["zoom"],10,2,14,6],"line-dasharray":[2,1]}},
              {"id":"names","type":"symbol","source":"s","source-layer":"l","layout":{"text-field":"{class}","symbol-placement":"line","text-size":12}}]}"##,
            "https://x/style.json",
        )
        .unwrap();
        let lines = [9, 4, 4, 18, 0, 16, 16, 0];
        let layers = mvt::read(&tile("l", 2, &lines, "primary")).unwrap();
        let to = |u: f64, v: f64| Some(Vec2::new(1000.0 + u * 4096.0, 2000.0 - v * 4096.0));
        let out = build(&TileInput {
            layers: &layers,
            style: &style,
            source: "s",
            zoom: 12.0,
            to_project: &to,
            origin: Vec2::new(0.0, 0.0),
            tile: 1,
        });
        let described: serde_json::Value = serde_json::from_str(&out.batches.json).unwrap();
        let kinds: Vec<&str> = described
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, vec!["fill", "stroke"]);
        let stroke = &described[1]["style"];
        assert_eq!(stroke["width"], 4.0);
        let dash: Vec<f64> = stroke["dash"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_f64)
            .collect();
        assert_eq!(dash, vec![8.0, 4.0]);
        assert_eq!(out.labels.len(), 1);
        assert_eq!(out.labels[0].text, "primary");
        assert!(matches!(out.labels[0].place, Placement::Line { .. }));
    }
}

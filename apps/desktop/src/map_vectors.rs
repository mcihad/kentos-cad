//! A map frame's drawing as vectors for the PDF (docs/sheet/design.md §9a,
//! `MapContent::Vector`), from the same styled layers the web's map frames
//! draw (the style engine at the map's scale on the paper's palette): the web's
//! app/sheet/mapVectors.ts and the paths' part of its pdfExport.ts `corePaths`,
//! line for line, so both platforms give the core the same paths:
//!
//! - a stroke batch's segments joined into its paths (the batch marks where a
//!   path ends), a path back where it began a ring; its colour, width, dashes
//!   and caps in paper mm;
//! - a solid fill's triangles as the rings bounding them (each triangle
//!   turned counter-clockwise, the edges no other triangle shares, chained),
//!   outer rings with the holes inside them;
//! - a hatch as its lines, cut to the triangles;
//! - a marker of the drawing's shapes as its outline (the render crate's
//!   shapes, the web's `canvasShapes.ts`), placed, turned and anchored as the
//!   shader places it; a ring with its dot.
//!
//! What has no vector form here (a pattern or picture fill, a picture or text
//! marker, a soft-edged line) is named: the map then goes to the PDF as a
//! picture (`MapContent::Raster`).

use std::collections::BTreeMap;

use kentos_native_style::batches::{
    BatchKind, Cap, FillPaintBatch, MarkerLook, StyledBatch, StyledLayer, Unit,
};
use kentos_sheet::pdf::{MapPath, MapStroke};
use kentos_sheet::style::{LineCap, LineJoin};

/// The map's scale and the numbers' origin.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VectorScale {
    /// The scale's denominator (1/N): metres of ground per metre of paper.
    pub scale: f64,
    /// The document's origin: the batches' numbers are relative to it.
    pub origin: [f64; 2],
}

/// A colour of the engine (0…1) as `#rrggbb`, with its opacity as `#rrggbbaa` below 0.999 (the
/// web's `hex` and `color`).
fn color(c: [f64; 4], opacity: f64) -> String {
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let rgb = format!("#{:02x}{:02x}{:02x}", byte(c[0]), byte(c[1]), byte(c[2]));
    if opacity >= 0.999 {
        rgb
    } else {
        format!("{rgb}{:02x}", (opacity.max(0.0) * 255.0).round() as u8)
    }
}

/// Paper mm of a length in a batch's unit: metres of ground at the map's scale, or CSS px.
pub(crate) fn paper_mm(v: f64, unit: Unit, scale: f64) -> f64 {
    match unit {
        Unit::World => v * 1000.0 / scale,
        Unit::Px => v * 25.4 / 96.0,
    }
}

fn cap(c: Cap) -> LineCap {
    match c {
        Cap::Butt => LineCap::Butt,
        Cap::Round => LineCap::Round,
        Cap::Square => LineCap::Square,
    }
}

/// The ground offset of a batch's numbers: the document's origin and the batch's tile.
fn base(o: &VectorScale, b: &StyledBatch) -> [f64; 2] {
    [o.origin[0] + b.origin[0], o.origin[1] + b.origin[1]]
}

/// A part of a path: its points on the ground and whether it closes.
#[derive(Clone, Debug, PartialEq)]
struct Part {
    points: Vec<[f64; 2]>,
    closed: bool,
}

/// How a path fills: non-zero (outer rings counter-clockwise, holes clockwise) or even-odd.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Rule {
    NonZero,
    EvenOdd,
}

/// One look's parts: the web's `VecPath`.
struct Shape {
    parts: Vec<Part>,
    stroke: Option<MapStroke>,
    fill: Option<(String, Rule)>,
}

/// Twice a ring's signed area: positive counter-clockwise.
fn signed(p: &[[f64; 2]]) -> f64 {
    let n = p.len();
    (0..n)
        .map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1])
        .sum()
}

/// Whether a point is inside a ring (even-odd crossing).
fn inside(x: f64, y: f64, p: &[[f64; 2]]) -> bool {
    let mut at = false;
    let n = p.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (p[i], p[j]);
        if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
            at = !at;
        }
        j = i;
    }
    at
}

/// A look's parts as the core's paths (the web's `corePaths`): a filled one as rings with their
/// holes, a line as it is.
fn core_paths(s: Shape) -> Vec<MapPath> {
    let Some((fill, rule)) = s.fill else {
        return s
            .parts
            .into_iter()
            .map(|p| MapPath {
                points: p.points,
                closed: p.closed,
                holes: Vec::new(),
                stroke: s.stroke.clone(),
                fill: None,
            })
            .collect();
    };
    let closed: Vec<&Part> = s.parts.iter().filter(|p| p.closed).collect();
    let (outers, holes): (Vec<&Part>, Vec<&Part>) = match rule {
        Rule::NonZero => closed.iter().partition(|p| signed(&p.points) > 0.0),
        Rule::EvenOdd => (closed.clone(), Vec::new()),
    };
    let mut out: Vec<MapPath> = outers
        .iter()
        .map(|p| MapPath {
            points: p.points.clone(),
            closed: true,
            holes: Vec::new(),
            stroke: s.stroke.clone(),
            fill: Some(fill.clone()),
        })
        .collect();
    for h in holes {
        let Some(first) = h.points.first() else {
            continue;
        };
        if let Some(owner) = outers
            .iter()
            .position(|o| inside(first[0], first[1], &o.points))
        {
            out[owner].holes.push(h.points.clone());
        }
    }
    // Lines among a filled look's parts are drawn as lines.
    if s.stroke.is_some() {
        for p in s.parts.iter().filter(|p| !p.closed) {
            out.push(MapPath {
                points: p.points.clone(),
                closed: false,
                holes: Vec::new(),
                stroke: s.stroke.clone(),
                fill: None,
            });
        }
    }
    out
}

/// A stroke batch's paths: segments joined where one ends and the next begins.
fn stroke_paths(layer: &StyledLayer, b: &StyledBatch, o: &VectorScale) -> Option<Shape> {
    let BatchKind::Stroke {
        color: c,
        width,
        unit,
        dash,
        dash_offset: _,
        cap: line_cap,
        ..
    } = &b.kind
    else {
        return None;
    };
    let [bx, by] = base(o, b);
    let s = layer.data.get(b.range.clone())?;
    let mut parts: Vec<Part> = Vec::new();
    let mut open = false;
    for seg in s.chunks_exact(6) {
        let (ax, ay) = (f64::from(seg[0]) + bx, f64::from(seg[1]) + by);
        let (ex, ey) = (f64::from(seg[2]) + bx, f64::from(seg[3]) + by);
        let flags = seg[5] as u32;
        let joins = open
            && parts
                .last()
                .and_then(|p| p.points.last())
                .is_some_and(|q| q[0] == ax && q[1] == ay);
        if flags & 1 != 0 || !joins {
            parts.push(Part {
                points: vec![[ax, ay]],
                closed: false,
            });
        }
        if let Some(p) = parts.last_mut() {
            p.points.push([ex, ey]);
        }
        open = flags & 2 == 0;
    }
    for p in &mut parts {
        // A path that comes back to where it began is a ring (its corner then joins, not caps).
        let n = p.points.len();
        if n > 3 && p.points[0] == p.points[n - 1] {
            p.points.pop();
            p.closed = true;
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(Shape {
        parts,
        stroke: Some(MapStroke {
            color: color(*c, c[3]),
            width: paper_mm(*width, *unit, o.scale),
            dash: dash
                .as_ref()
                .filter(|d| !d.is_empty())
                .map(|d| d.iter().map(|x| paper_mm(*x, *unit, o.scale)).collect())
                .unwrap_or_default(),
            cap: cap(*line_cap),
            join: LineJoin::Round,
        }),
        fill: None,
    })
}

/// The rings bounding a triangle list: each triangle counter-clockwise, its unshared edges chained.
fn triangle_rings(t: &[f32], dx: f64, dy: f64) -> Vec<Part> {
    type Key = (u32, u32);
    let key = |x: f32, y: f32| (x.to_bits(), y.to_bits());
    // Each point's outgoing edges, in the order they came (the web's Map keeps insertion order).
    let mut order: Vec<Key> = Vec::new();
    let mut edges: BTreeMap<Key, Vec<Key>> = BTreeMap::new();
    let mut point: BTreeMap<Key, (f32, f32)> = BTreeMap::new();
    let mut add = |a: (f32, f32), b: (f32, f32)| {
        let (ka, kb) = (key(a.0, a.1), key(b.0, b.1));
        if ka == kb {
            return;
        }
        // An edge another triangle runs the other way is inside: both go.
        if let Some(back) = edges.get_mut(&kb)
            && let Some(at) = back.iter().position(|k| *k == ka)
        {
            back.remove(at);
            return;
        }
        point.insert(ka, a);
        point.insert(kb, b);
        if !edges.contains_key(&ka) {
            order.push(ka);
        }
        edges.entry(ka).or_default().push(kb);
    };
    for tri in t.chunks_exact(6) {
        let a = (tri[0], tri[1]);
        let mut b = (tri[2], tri[3]);
        let mut c = (tri[4], tri[5]);
        let cross = (f64::from(b.0) - f64::from(a.0)) * (f64::from(c.1) - f64::from(a.1))
            - (f64::from(b.1) - f64::from(a.1)) * (f64::from(c.0) - f64::from(a.0));
        if cross == 0.0 {
            continue;
        }
        if cross < 0.0 {
            std::mem::swap(&mut b, &mut c);
        }
        add(a, b);
        add(b, c);
        add(c, a);
    }
    let mut rings = Vec::new();
    for start in order {
        loop {
            if edges.get(&start).is_none_or(Vec::is_empty) {
                break;
            }
            let mut ring: Vec<[f64; 2]> = Vec::new();
            let mut at = start;
            for _ in 0..10_000_000 {
                let Some(p) = point.get(&at) else {
                    break;
                };
                ring.push([f64::from(p.0) + dx, f64::from(p.1) + dy]);
                let next = edges.get_mut(&at).and_then(Vec::pop);
                match next {
                    Some(n) if n != start => at = n,
                    _ => break,
                }
            }
            if ring.len() >= 3 {
                rings.push(Part {
                    points: ring,
                    closed: true,
                });
            }
        }
    }
    rings
}

/// The parts of a line `n·p = c` inside the triangles, as parameter intervals along `d` (merged).
fn line_in_triangles(t: &[f32], n: [f64; 2], d: [f64; 2], c: f64) -> Vec<[f64; 2]> {
    let mut spans: Vec<[f64; 2]> = Vec::new();
    for tri in t.chunks_exact(6) {
        let mut pts: Vec<f64> = Vec::new();
        for k in 0..3 {
            let (ax, ay) = (f64::from(tri[2 * k]), f64::from(tri[2 * k + 1]));
            let (bx, by) = (
                f64::from(tri[(2 * k + 2) % 6]),
                f64::from(tri[(2 * k + 3) % 6]),
            );
            let sa = n[0] * ax + n[1] * ay - c;
            let sb = n[0] * bx + n[1] * by - c;
            if (sa < 0.0 && sb < 0.0) || (sa > 0.0 && sb > 0.0) || sa == sb {
                continue;
            }
            let f = sa / (sa - sb);
            let (x, y) = (ax + (bx - ax) * f, ay + (by - ay) * f);
            pts.push(d[0] * x + d[1] * y);
        }
        if pts.len() >= 2 {
            let lo = pts.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = pts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if hi > lo {
                spans.push([lo, hi]);
            }
        }
    }
    spans.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut out: Vec<[f64; 2]> = Vec::new();
    for s in spans {
        match out.last_mut() {
            Some(last) if s[0] <= last[1] + 1e-9 => last[1] = last[1].max(s[1]),
            _ => out.push(s),
        }
    }
    out
}

/// A fill batch: solid as its rings, a hatch as its cut lines; the reason when it has no vector form.
fn fill_paths(
    layer: &StyledLayer,
    b: &StyledBatch,
    paint: &FillPaintBatch,
    o: &VectorScale,
) -> Result<Vec<Shape>, &'static str> {
    let [bx, by] = base(o, b);
    let t = layer.data.get(b.range.clone()).unwrap_or_default();
    match paint {
        FillPaintBatch::Solid { color: c } => {
            let parts = triangle_rings(t, bx, by);
            Ok(if parts.is_empty() {
                Vec::new()
            } else {
                vec![Shape {
                    parts,
                    stroke: None,
                    fill: Some((color(*c, c[3]), Rule::NonZero)),
                }]
            })
        }
        FillPaintBatch::Hatch {
            color: c,
            angle,
            spacing,
            width,
            offset,
            dash,
            unit,
            ..
        } => {
            if *spacing <= 0.0 {
                return Ok(Vec::new());
            }
            let a = angle.to_radians();
            let d = [a.cos(), a.sin()];
            let n = [-d[1], d[0]];
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for p in t.chunks_exact(2) {
                let v = n[0] * f64::from(p[0]) + n[1] * f64::from(p[1]);
                lo = lo.min(v);
                hi = hi.max(v);
            }
            // Lines where n·p − offset is a whole number of spacings (the shader's), in the batch's frame.
            let first = ((lo - offset) / spacing).ceil();
            let last = ((hi - offset) / spacing).floor();
            if last - first > 20_000.0 {
                return Err("çok sık tarama");
            }
            let mut parts = Vec::new();
            let mut j = first;
            while j <= last {
                let c = offset + j * spacing;
                for [s0, s1] in line_in_triangles(t, n, d, c) {
                    parts.push(Part {
                        points: vec![
                            [d[0] * s0 + n[0] * c + bx, d[1] * s0 + n[1] * c + by],
                            [d[0] * s1 + n[0] * c + bx, d[1] * s1 + n[1] * c + by],
                        ],
                        closed: false,
                    });
                }
                j += 1.0;
            }
            Ok(if parts.is_empty() {
                Vec::new()
            } else {
                vec![Shape {
                    parts,
                    stroke: Some(MapStroke {
                        color: color(*c, c[3]),
                        width: paper_mm(*width, *unit, o.scale),
                        dash: dash
                            .as_ref()
                            .filter(|d| !d.is_empty())
                            .map(|d| d.iter().map(|x| paper_mm(*x, *unit, o.scale)).collect())
                            .unwrap_or_default(),
                        cap: LineCap::Butt,
                        join: LineJoin::Miter,
                    }),
                    fill: None,
                }]
            })
        }
        FillPaintBatch::Pattern { .. } => Err("desen dolgusu"),
        FillPaintBatch::Tile { .. } => Err("resimli dolgu"),
    }
}

/// The shapes drawn as lines only (their fill colour strokes them): `canvasShapes.ts` `OPEN_SHAPES`.
const OPEN_SHAPES: [&str; 6] = ["cross", "x", "line", "arrow", "chevron", "arc"];

/// A marker batch of the drawing's shapes as outlines on the ground; the reason when its look
/// has no vector form here.
fn marker_paths(
    layer: &StyledLayer,
    b: &StyledBatch,
    o: &VectorScale,
) -> Result<Vec<Shape>, &'static str> {
    let BatchKind::Marker {
        unit,
        look,
        offset,
        anchor,
        opacity,
        ..
    } = &b.kind
    else {
        return Ok(Vec::new());
    };
    let MarkerLook::Shape {
        shape,
        fill,
        stroke,
        stroke_width,
        params,
    } = look
    else {
        return Err(match look {
            MarkerLook::Image {
                image: kentos_native_style::AtlasImage::Text { .. },
                ..
            } => "yazı simgesi",
            _ => "resimli simge",
        });
    };
    let [bx, by] = base(o, b);
    let open = OPEN_SHAPES.contains(&shape.as_str());
    let stroke_color = stroke.or(if open { *fill } else { None });
    // Sizes in the batch's unit; ground metres per unit: 1 for world, a CSS px's paper mm at the scale for px.
    let per_unit = match unit {
        Unit::World => 1.0,
        Unit::Px => 25.4 / 96.0 * o.scale / 1000.0,
    };
    let (mut fills, mut strokes, mut dots) = (Vec::new(), Vec::new(), Vec::new());
    let v = layer.data.get(b.range.clone()).unwrap_or_default();
    for m in v.chunks_exact(5) {
        let w = f64::from(m[3]);
        let h = if m[4] > 0.0 { f64::from(m[4]) } else { w };
        let (sa, ca) = f64::from(m[2]).sin_cos();
        // The shape's centre from its point: the anchor and the offset, turned with it.
        let qx = offset[0] - anchor[0] * w;
        let qy = offset[1] - anchor[1] * h;
        let (px, py) = (f64::from(m[0]) + bx, f64::from(m[1]) + by);
        let place = |x: f64, y: f64| -> [f64; 2] {
            [
                px + (ca * (x + qx) - sa * (y + qy)) * per_unit,
                py + (sa * (x + qx) + ca * (y + qy)) * per_unit,
            ]
        };
        for (points, closed) in
            kentos_render_wgpu::styled::raster::shape_outline(shape, w / 2.0, h / 2.0, *params)
        {
            let part = Part {
                points: points.iter().map(|p| place(p[0], p[1])).collect(),
                closed,
            };
            if open {
                strokes.push(part);
            } else {
                fills.push(part);
            }
        }
        if shape == "ring" && stroke_color.is_some() {
            let r = stroke_width.max(w.min(h) * 0.08);
            for (points, closed) in
                kentos_render_wgpu::styled::raster::shape_outline("circle", r, r, [0.0; 4])
            {
                dots.push(Part {
                    points: points.iter().map(|p| place(p[0], p[1])).collect(),
                    closed,
                });
            }
        }
    }
    let line = |c: [f64; 4]| MapStroke {
        color: color(c, c[3] * opacity),
        width: paper_mm(stroke_width.max(0.0), *unit, o.scale),
        dash: Vec::new(),
        cap: LineCap::Butt,
        join: LineJoin::Miter,
    };
    let mut out = Vec::new();
    if !fills.is_empty() && ((fill.is_some() && !open) || stroke_color.is_some()) {
        out.push(Shape {
            parts: fills,
            stroke: stroke_color.map(line),
            fill: fill
                .filter(|_| !open)
                .map(|c| (color(c, c[3] * opacity), Rule::EvenOdd)),
        });
    }
    if !strokes.is_empty()
        && let Some(c) = stroke_color
    {
        out.push(Shape {
            parts: strokes,
            stroke: Some(line(c)),
            fill: None,
        });
    }
    if !dots.is_empty()
        && let Some(c) = stroke_color
    {
        out.push(Shape {
            parts: dots,
            stroke: None,
            fill: Some((color(c, c[3] * opacity), Rule::NonZero)),
        });
    }
    Ok(out)
}

/// Why a batch has no vector form here, if it has none (the web's `layerPaths` reasons): a
/// hatch of more than 20 000 lines, a pattern or picture fill, a picture or text marker, a
/// soft-edged line.
fn why_not(layer: &StyledLayer, b: &StyledBatch) -> Option<&'static str> {
    match &b.kind {
        BatchKind::Stroke { blur, .. } => (*blur > 0.0).then_some("yumuşak kenarlı çizgi"),
        BatchKind::Fill { paint } => match paint {
            FillPaintBatch::Solid { .. } => None,
            FillPaintBatch::Hatch {
                angle,
                spacing,
                offset,
                ..
            } => {
                if *spacing <= 0.0 {
                    return None;
                }
                let a = angle.to_radians();
                let n = [-a.sin(), a.cos()];
                let t = layer.data.get(b.range.clone()).unwrap_or_default();
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for p in t.chunks_exact(2) {
                    let v = n[0] * f64::from(p[0]) + n[1] * f64::from(p[1]);
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
                let lines = ((hi - offset) / spacing).floor() - ((lo - offset) / spacing).ceil();
                (lines > 20_000.0).then_some("çok sık tarama")
            }
            FillPaintBatch::Pattern { .. } => Some("desen dolgusu"),
            FillPaintBatch::Tile { .. } => Some("resimli dolgu"),
        },
        BatchKind::Marker { look, .. } => match look {
            MarkerLook::Shape { .. } => None,
            MarkerLook::Image {
                image: kentos_native_style::AtlasImage::Text { .. },
                ..
            } => Some("yazı simgesi"),
            MarkerLook::Image { .. } => Some("resimli simge"),
        },
    }
}

/// What of a built layer has no vector form where it reaches the ground box (each said once, in
/// the order met): what keeps a map that shows it from going to a PDF as vectors.
pub(crate) fn unsupported_in(
    layer: &StyledLayer,
    o: &VectorScale,
    within: Option<[f64; 4]>,
) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for b in &layer.batches {
        if batch_reaches(b, o, within)
            && let Some(why) = why_not(layer, b)
            && !out.contains(&why)
        {
            out.push(why);
        }
    }
    out
}

/// Whether a batch's geometry reaches the ground box. Its box is from the document's origin, as
/// every batch's (the style core's `batch.rs`), not from its tile: the tile moves only its numbers
/// (docs/adr/0157). With the tile added, a drawing a tile or more from the anchor lost its map.
fn batch_reaches(b: &StyledBatch, o: &VectorScale, within: Option<[f64; 4]>) -> bool {
    let Some(w) = within else {
        return true;
    };
    let [bx, by] = o.origin;
    let r = b.bounds;
    r[2] + bx >= w[0] && r[0] + bx <= w[2] && r[3] + by >= w[1] && r[1] + by <= w[3]
}

/// Whether a path's points reach the ground box.
fn reaches(points: &[[f64; 2]], w: [f64; 4]) -> bool {
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in points {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    x1 >= w[0] && x0 <= w[2] && y1 >= w[1] && y0 <= w[3]
}

/// A built layer's drawing as the core's paths in its draw order, and what has no vector form
/// (each said once). With `within`, what does not reach that ground box is left out.
pub(crate) fn layer_paths(
    layer: &StyledLayer,
    o: &VectorScale,
    within: Option<[f64; 4]>,
) -> (Vec<MapPath>, Vec<&'static str>) {
    let mut shapes: Vec<Shape> = Vec::new();
    let mut unsupported: Vec<&'static str> = Vec::new();
    let mut say = |why: &'static str| {
        if !unsupported.contains(&why) {
            unsupported.push(why);
        }
    };
    for b in &layer.batches {
        if !batch_reaches(b, o, within) {
            continue;
        }
        match &b.kind {
            BatchKind::Stroke { blur, .. } => {
                if *blur > 0.0 {
                    say("yumuşak kenarlı çizgi");
                }
                shapes.extend(stroke_paths(layer, b, o));
            }
            BatchKind::Fill { paint } => match fill_paths(layer, b, paint, o) {
                Ok(s) => shapes.extend(s),
                Err(why) => say(why),
            },
            BatchKind::Marker { .. } => match marker_paths(layer, b, o) {
                Ok(s) => shapes.extend(s),
                Err(why) => say(why),
            },
        }
    }
    // Only the parts that reach the box (the web's `within`).
    if let Some(w) = within {
        for s in &mut shapes {
            s.parts.retain(|p| reaches(&p.points, w));
        }
        shapes.retain(|s| !s.parts.is_empty());
    }
    let paths = shapes.into_iter().flat_map(core_paths).collect();
    (paths, unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two triangles of a square: one ring of four corners, counter-clockwise, the shared edge gone.
    #[test]
    fn a_square_s_triangles_are_its_ring() {
        let t: [f32; 12] = [
            0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 0.0, 10.0, 10.0, 0.0, 10.0,
        ];
        let rings = triangle_rings(&t, 100.0, 200.0);
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].points.len(), 4);
        assert!(signed(&rings[0].points) > 0.0);
        assert!(rings[0].points.contains(&[110.0, 210.0]));
    }

    /// A hole: a square with a square gap, triangulated as an engine triangulates it (no
    /// T-junctions), is an outer ring and a hole in it.
    #[test]
    fn a_ring_of_triangles_has_its_hole() {
        let (o, i) = (
            [[0.0f32, 0.0], [30.0, 0.0], [30.0, 30.0], [0.0, 30.0]],
            [[10.0f32, 10.0], [20.0, 10.0], [20.0, 20.0], [10.0, 20.0]],
        );
        let mut t: Vec<f32> = Vec::new();
        // Each side's trapezoid between the outer square and the gap, as two triangles.
        for k in 0..4 {
            let (a, b, c, d) = (o[k], o[(k + 1) % 4], i[(k + 1) % 4], i[k]);
            t.extend([a[0], a[1], b[0], b[1], c[0], c[1]]);
            t.extend([a[0], a[1], c[0], c[1], d[0], d[1]]);
        }
        let rings = triangle_rings(&t, 0.0, 0.0);
        assert_eq!(rings.len(), 2, "{rings:?}");
        let paths = core_paths(Shape {
            parts: rings,
            stroke: None,
            fill: Some(("#ff0000".into(), Rule::NonZero)),
        });
        assert_eq!(paths.len(), 1, "{paths:?}");
        assert_eq!((paths[0].points.len(), paths[0].holes.len()), (4, 1));
        assert_eq!(paths[0].fill.as_deref(), Some("#ff0000"));
    }

    /// A batch a tile east of the anchor (its numbers from its tile, its box from the anchor)
    /// reaches the ground box where its geometry is, and not one a tile further.
    #[test]
    fn a_far_tile_s_batch_reaches_where_its_geometry_is() {
        let o = VectorScale {
            scale: 1000.0,
            origin: [500_000.0, 4_400_000.0],
        };
        let batch = StyledBatch {
            range: 0..0,
            kind: BatchKind::Stroke {
                color: [0.0, 0.0, 0.0, 1.0],
                width: 0.35,
                unit: Unit::World,
                dash: None,
                dash_offset: 0.0,
                cap: Cap::Butt,
                blur: 0.0,
            },
            level: 0.0,
            key: 0,
            // The geometry 83.67 km east of the anchor; the tile is one 65.536 km step.
            bounds: [83_660.0, 10.0, 83_680.0, 30.0],
            origin: [65_536.0, 0.0],
            reach: 0.0,
            reach_unit: Unit::World,
            min_scale: None,
            max_scale: None,
        };
        let around = |x: f64, y: f64| Some([x - 50.0, y - 50.0, x + 50.0, y + 50.0]);
        assert!(batch_reaches(&batch, &o, around(583_670.0, 4_400_020.0)));
        assert!(!batch_reaches(
            &batch,
            &o,
            around(583_670.0 + 65_536.0, 4_400_020.0)
        ));
    }

    #[test]
    fn colours_and_lengths_are_the_web_s() {
        assert_eq!(color([1.0, 0.5, 0.0, 1.0], 1.0), "#ff8000");
        assert_eq!(color([0.0, 0.0, 0.0, 0.5], 0.5), "#00000080");
        assert!((paper_mm(2.0, Unit::World, 1000.0) - 2.0).abs() < 1e-12);
        assert!((paper_mm(96.0, Unit::Px, 1000.0) - 25.4).abs() < 1e-12);
    }
}

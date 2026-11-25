//! SVG symbols as vectors on the model screen (plan/render.md RM-11, docs/adr/9208).
//!
//! The owner (23 November 2026): “SVG çizgileri, stil çizgilerini” turned saw-toothed close up. A marker whose look is
//! an SVG is drawn from the atlas, a bitmap of at most 512 px in one 2048 px page (`styled/atlas.rs`); a symbol sized
//! on the ground grows with the zoom (the marker shader's `unitK`), and past its bitmap's size the bitmap is
//! magnified: its slanted edges become steps.
//!
//! Here such a batch becomes vectors drawn per placement, as a block's insert is (docs/adr/1063 §2): its SVG read once
//! (usvg), its fills triangulated (the geometry core's ear clipper) and its strokes outlined (tiny-skia's stroker) and
//! triangulated too, one solid batch a colour; each marker of the batch one instance, its linear part the marker's
//! turn and size, its translation the marker's place. The marker shader's own placing is followed exactly: the box
//! centred on the place, the anchor and the offset in it, the turn counter-clockwise in its pixels (y up, as the
//! ground's: `markerVertex` turns before `pxToClip`, which does not flip), so the vector symbol lies where the bitmap
//! lay. Only ground-sized markers are turned (`Unit::World`): a screen-sized one never grows past its bitmap. An SVG
//! that does not parse, a gradient-only or empty one keeps its bitmap.
//!
//! The marker batch stays and draws while the symbol is small: there its bitmap is the SVG rasterized at its size, a
//! stroke thinner than a pixel a faint continuous line, where the vectors' samples would break it into beads. Past
//! [`VECTORS_FROM_PX`] the vectors draw instead (`StyledBatch::legible`), never both.
//!
//! **Text markers too** (plan/render.md RM-11, docs/adr/9245): a ground-sized text marker's bitmap (`raster::text_image`)
//! is magnified the same way. Its glyphs' outlines, from the host's faces (`DecodeOptions::text_outlines`), are written
//! as the SVG the bitmap paints (`text_svg`: its halo a round-joined stroke under its fill, the letters at the box's
//! height over 1.25, their baseline at 78 % of it) and made vectors as an SVG symbol is. Its box is its height wide over
//! its aspect (the marker shader's `fit` 2), and the switch compares its height, as the bitmap's batch does.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use kentos_geometry_core::triangulate::triangulate;
use kentos_geometry_core::vec2::Vec2;
use resvg::tiny_skia;
use resvg::usvg;

use crate::batches::{
    AtlasImage, BatchKind, FillPaintBatch, INSTANCE_FLOATS, Instances, Mark, MarkerLook, StyledBatch, StyledLayer, Unit,
};
use crate::picture::{ImageSource, Segment};

/// Numbers a marker takes: its place (x, y from the batch's tile), its turn, its width and its height.
const MARKER: usize = 5;
/// Curves are flattened within this share of the symbol's larger side: a symbol 8 000 px wide strays under a pixel.
const FLATTEN: f64 = 1.0 / 8000.0;
/// Device pixels a symbol's width must pass for its vectors to draw instead of its bitmap. The atlas draws a bitmap up
/// to 512 px and halves that only when its page is full, so the bitmap is sharp up to here; and the serare's 2.22 %
/// stroke is 2.8 px here, wide enough for the drawing's 4× samples (at 24 px it was 0.5 px and broke into beads).
pub const VECTORS_FROM_PX: f64 = 128.0;

/// The vector batches of every ground-sized SVG marker batch, right after it in the draw order; the marker batch keeps
/// drawing its bitmap while the symbol is small.
pub fn add(layer: &mut StyledLayer, text: Option<&dyn ImageSource>) {
    if !layer.batches.iter().any(|b| svg_of(b).is_some() || text_of(b).is_some()) {
        return;
    }
    let has_low = !layer.low.is_empty();
    let mut batches = Vec::with_capacity(layer.batches.len());
    for b in std::mem::take(&mut layer.batches) {
        // A text marker's box is its height (the marker's fifth number) wide over its aspect.
        let mut by_height = false;
        let meshes = match (svg_of(&b), text_of(&b)) {
            (Some((svg, w, h)), _) => mesh(svg, w, h),
            (None, Some(t)) => {
                by_height = true;
                text.and_then(|source| text_svg(source, t)).and_then(|(svg, w, h)| mesh(&svg, w, h))
            }
            (None, None) => None,
        };
        let marks = match &b.kind {
            BatchKind::Marker { offset, anchor, opacity, extent, .. } => Some((*offset, *anchor, *opacity, extent[1])),
            _ => None,
        };
        let (Some(meshes), Some((offset, anchor, opacity, height))) = (meshes, marks) else {
            batches.push(b);
            continue;
        };
        let mut bitmap = b.clone();
        if let BatchKind::Marker { vectors, .. } = &mut bitmap.kind {
            *vectors = true;
        }
        let at = |i: usize| {
            f64::from(layer.data[b.range.start + i]) + if has_low { f64::from(layer.low[b.range.start + i]) } else { 0.0 }
        };
        let n = b.range.len() / MARKER;
        // In double precision: a translation's residual rides in the layer's low stream, as a marker's place did.
        let mut instances: Vec<f64> = Vec::with_capacity(n * INSTANCE_FLOATS);
        let mut largest = 0.0f64;
        for m in 0..n {
            let i = m * MARKER;
            let (x, y, turn) = (at(i), at(i + 1), at(i + 2));
            let w = if by_height { at(i + 4) / meshes.aspect } else { at(i + 3) };
            if !(w.is_finite() && w > 0.0) {
                continue;
            }
            // The marker shader turns its box by `turn` counter-clockwise in its pixels, y up as the ground's
            // (`markerVertex`): on the ground the linear part [[cos, -sin], [sin, cos]], by columns. Its offset (in the
            // batch's unit, the ground's) is added before the turn, so it turns with it.
            let (s, c) = turn.sin_cos();
            let (ox, oy) = (offset[0], offset[1]);
            let (tx, ty) = (x + c * ox - s * oy, y + s * ox + c * oy);
            instances.extend([w * c, w * s, -w * s, w * c, tx, ty, w, 0.0]);
            largest = largest.max(w);
        }
        let count = (instances.len() / INSTANCE_FLOATS) as u32;
        if count == 0 {
            batches.push(b);
            continue;
        }
        batches.push(bitmap);
        for (k, (rgba, triangles)) in meshes.fills.iter().enumerate() {
            // The mesh in the symbol's own units (its width 1), y up, its anchor already in it: the shader's
            // `local - anchor * (w, h)`, the image's top row (v 0) at local y +h/2.
            let start = layer.data.len();
            for p in triangles.chunks(2) {
                let (u, v) = (p[0] / meshes.width, p[1] / meshes.height);
                let mx = u - 0.5 - anchor[0];
                let my = (0.5 - v - anchor[1]) * meshes.aspect;
                layer.data.extend([mx as f32, my as f32]);
            }
            let mesh_len = layer.data.len() - start;
            if has_low {
                layer.low.resize(layer.data.len(), 0.0);
            }
            for (i, &v) in instances.iter().enumerate() {
                let high = v as f32;
                layer.data.push(high);
                if has_low {
                    // The translation's (4, 5) residual; the instance shader adds it (`toPxLow`).
                    let translation = matches!(i % INSTANCE_FLOATS, 4 | 5);
                    layer.low.push(if translation { (v - f64::from(high)) as f32 } else { 0.0 });
                }
            }
            let mut color = *rgba;
            color[3] *= opacity;
            batches.push(StyledBatch {
                range: start..layer.data.len(),
                kind: BatchKind::Fill { paint: FillPaintBatch::Solid { color } },
                key: b.key ^ (0x9e37_79b9_7f4a_7c15u64.wrapping_mul(k as u64 + 1)),
                lod: None,
                instances: Some(Instances {
                    mesh: mesh_len,
                    count,
                    // The switch compares what the bitmap's batch does (`StyledBatch::legible`): a text's height.
                    scale: if by_height { height } else { largest },
                    proxy: None,
                    mark: Some(Mark { anchor, aspect: meshes.aspect, boxes: k == 0 }),
                }),
                ..b.clone()
            });
        }
    }
    layer.batches = batches;
}

/// The boxes a symbol's markers had (a mask's shapes, docs/adr/0226 §3), from its first colour's instances: each box's
/// middle (from the batch's tile, metres), its width and height (metres) and its turn (radians, counter-clockwise),
/// as the marker shader placed it. Nothing for a batch that is not a symbol's first colour.
///
/// `data` and `low` are the layer's numbers (`StyledLayer::data`, `low`), wherever they are held.
pub fn boxes<'a>(
    data: &'a [f32],
    low: &'a [f32],
    b: &'a StyledBatch,
) -> impl Iterator<Item = ([f64; 2], [f64; 2], f64)> + 'a {
    let found = b.instances.as_ref().and_then(|i| Some((i.mesh, i.mark.filter(|m| m.boxes)?)));
    let (start, mark) = match found {
        Some((mesh, mark)) => (b.range.start + mesh, Some(mark)),
        None => (b.range.end, None),
    };
    let numbers = data.get(start..b.range.end).unwrap_or(&[]);
    numbers.as_chunks::<INSTANCE_FLOATS>().0.iter().enumerate().filter_map(move |(n, x)| {
        let mark = mark?;
        let low = |i: usize| f64::from(low.get(start + n * INSTANCE_FLOATS + i).copied().unwrap_or(0.0));
        let (wc, ws) = (f64::from(x[0]), f64::from(x[1]));
        let w = wc.hypot(ws);
        let h = w * mark.aspect;
        let turn = ws.atan2(wc);
        let (s, c) = turn.sin_cos();
        // The translation is the place and the turned offset; the box's middle is the anchor's way off it.
        let (ax, ay) = (mark.anchor[0] * w, mark.anchor[1] * h);
        let middle = [f64::from(x[4]) + low(4) - (c * ax - s * ay), f64::from(x[5]) + low(5) - (s * ax + c * ay)];
        Some((middle, [w, h], turn))
    })
}

/// A batch's SVG, width and height, when it is a ground-sized marker drawn from an SVG.
fn svg_of(b: &StyledBatch) -> Option<(&str, f64, f64)> {
    match &b.kind {
        BatchKind::Marker {
            unit: Unit::World,
            look: MarkerLook::Image { image: AtlasImage::Svg { svg, width, height, .. }, fit_height: false },
            ..
        } if b.instances.is_none() && b.lod.is_none() => Some((svg.as_str(), *width, *height)),
        _ => None,
    }
}

/// A ground-sized text marker batch's text and look (its bitmap's, `raster::text_image`).
fn text_of(b: &StyledBatch) -> Option<&AtlasImage> {
    match &b.kind {
        BatchKind::Marker {
            unit: Unit::World,
            look: MarkerLook::Image { image: image @ AtlasImage::Text { .. }, fit_height: true },
            ..
        } if b.instances.is_none() && b.lod.is_none() => Some(image),
        _ => None,
    }
}

/// The box height of a text marker's SVG: its own units (any would do; the instance scales it).
const TEXT_STEP: f64 = 100.0;
/// A text marker's box over its letters' size (`raster::TEXT_BOX`).
const TEXT_BOX: f64 = 1.25;
/// Where the baseline lies in the box, from its top (`raster::text_image`).
const BASELINE: f64 = 0.78;

/// A text marker as the SVG its bitmap paints (`raster::text_image`), its width and height: the box [`TEXT_STEP`]
/// high, the letters [`TEXT_BOX`] times smaller, the baseline at [`BASELINE`] of it, a halo the given share of the
/// letters' size each side, stroked round-joined under the fill. None when the host has no face for it.
pub(crate) fn text_svg(source: &dyn ImageSource, image: &AtlasImage) -> Option<(String, f64, f64)> {
    let AtlasImage::Text { text, font, weight, italic, color, halo, .. } = image else {
        return None;
    };
    let outline = source.text(text, font.as_deref(), *weight, *italic)?;
    let size = TEXT_STEP / TEXT_BOX;
    let s = size / f64::from(outline.size).max(1e-6);
    let halo_w = halo.as_ref().map_or(0.0, |(_, w)| w * size);
    let width = f64::from(outline.advance) * s + 2.0 * halo_w;
    if !(width > 0.0 && width.is_finite()) {
        return None;
    }
    let mut d = String::new();
    let at = |p: [f32; 2]| (halo_w + s * f64::from(p[0]), TEXT_STEP * BASELINE + s * f64::from(p[1]));
    for glyph in &outline.glyphs {
        for segment in glyph {
            let _ = match *segment {
                Segment::Move(p) => write!(d, "M{} {}", at(p).0, at(p).1),
                Segment::Line(p) => write!(d, "L{} {}", at(p).0, at(p).1),
                Segment::Quad(c, p) => write!(d, "Q{} {} {} {}", at(c).0, at(c).1, at(p).0, at(p).1),
                Segment::Cubic(c1, c2, p) => {
                    write!(d, "C{} {} {} {} {} {}", at(c1).0, at(c1).1, at(c2).0, at(c2).1, at(p).0, at(p).1)
                }
                Segment::Close => write!(d, "Z"),
            };
        }
    }
    if d.is_empty() {
        return None;
    }
    // A colour as the SVG reads it: its red, green and blue, and its opacity apart.
    let paint = |hex: &str| -> (String, f64) {
        let c = crate::color::parse_hex(hex, 1.0);
        let v = |x: f64| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        (format!("rgb({},{},{})", v(c[0]), v(c[1]), v(c[2])), c[3].clamp(0.0, 1.0))
    };
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{TEXT_STEP}" viewBox="0 0 {width} {TEXT_STEP}">"#
    );
    if let Some((halo_color, _)) = halo.as_ref().filter(|_| halo_w > 0.0) {
        let (rgb, a) = paint(halo_color);
        let w = 2.0 * halo_w;
        let _ = write!(
            svg,
            r#"<path d="{d}" fill="none" stroke="{rgb}" stroke-opacity="{a}" stroke-width="{w}" stroke-linejoin="round"/>"#
        );
    }
    let (rgb, a) = paint(color);
    let _ = write!(svg, r#"<path d="{d}" fill="{rgb}" fill-opacity="{a}" fill-rule="nonzero"/></svg>"#);
    Some((svg, width, TEXT_STEP))
}

/// A symbol's triangles by colour, in the SVG's own pixels (x right, y down), with its size.
pub(crate) struct Meshes {
    pub(crate) width: f64,
    pub(crate) height: f64,
    /// Height over width, as the marker shader's box has it (`fit` 1).
    pub(crate) aspect: f64,
    pub(crate) fills: Vec<([f64; 4], Vec<f64>)>,
}

pub(crate) fn mesh(svg: &str, width: f64, height: f64) -> Option<Meshes> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let (sw, sh) = (f64::from(size.width()), f64::from(size.height()));
    let (sw, sh) = if sw > 0.0 && sh > 0.0 { (sw, sh) } else { (width, height) };
    if !(sw > 0.0 && sh > 0.0) {
        return None;
    }
    let tolerance = sw.max(sh) * FLATTEN;
    // Colours in the SVG's order, so a later colour lies over an earlier one as the bitmap had it.
    let mut by_colour: BTreeMap<usize, ([f64; 4], Vec<f64>)> = BTreeMap::new();
    let mut order: Vec<[u32; 4]> = Vec::new();
    let mut put = |rgba: [f64; 4], tris: Vec<f64>| {
        if tris.is_empty() {
            return;
        }
        let key = rgba.map(|c| (c * 1000.0).round() as u32);
        let at = order.iter().position(|k| *k == key).unwrap_or_else(|| {
            order.push(key);
            order.len() - 1
        });
        by_colour.entry(at).or_insert_with(|| (rgba, Vec::new())).1.extend(tris);
    };
    walk(tree.root(), &mut |path| {
        if !path.is_visible() {
            return;
        }
        let ts = path.abs_transform();
        if let Some(fill) = path.fill()
            && let Some(rgb) = colour(fill.paint())
        {
            let rings = rings(path.data(), ts, tolerance);
            let even_odd = fill.rule() == usvg::FillRule::EvenOdd;
            put([rgb[0], rgb[1], rgb[2], f64::from(fill.opacity().get())], fill_rings(&rings, even_odd));
        }
        if let Some(stroke) = path.stroke()
            && let Some(rgb) = colour(stroke.paint())
            && let Some(outline) = path.data().stroke(&stroke.to_tiny_skia(), 1.0)
        {
            let rings = rings(&outline, ts, tolerance);
            // An outline is filled by its winding, as the stroker means it.
            put([rgb[0], rgb[1], rgb[2], f64::from(stroke.opacity().get())], fill_rings(&rings, false));
        }
    });
    let fills: Vec<_> = by_colour.into_values().collect();
    (!fills.is_empty()).then_some(Meshes { width: sw, height: sh, aspect: sh / sw, fills })
}

fn walk(group: &usvg::Group, visit: &mut impl FnMut(&usvg::Path)) {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk(g, visit),
            usvg::Node::Path(p) => visit(p),
            _ => {}
        }
    }
}

/// A paint's colour, 0–1; a gradient's first stop stands for it. None for a pattern.
fn colour(paint: &usvg::Paint) -> Option<[f64; 3]> {
    let c = match paint {
        usvg::Paint::Color(c) => *c,
        usvg::Paint::LinearGradient(g) => g.stops().first()?.color(),
        usvg::Paint::RadialGradient(g) => g.stops().first()?.color(),
        usvg::Paint::Pattern(_) => return None,
    };
    Some([f64::from(c.red) / 255.0, f64::from(c.green) / 255.0, f64::from(c.blue) / 255.0])
}

/// A path's contours as rings of points (the SVG's pixels after `ts`), curves flattened within `tolerance`.
fn rings(path: &tiny_skia::Path, ts: tiny_skia::Transform, tolerance: f64) -> Vec<Vec<Vec2>> {
    let map = |p: tiny_skia::Point| {
        let mut q = [p];
        ts.map_points(&mut q);
        Vec2::new(f64::from(q[0].x), f64::from(q[0].y))
    };
    let mut out: Vec<Vec<Vec2>> = Vec::new();
    let mut ring: Vec<Vec2> = Vec::new();
    let mut last = Vec2::new(0.0, 0.0);
    let steps = |a: Vec2, b: Vec2| ((a.x - b.x).hypot(a.y - b.y) / tolerance).sqrt().ceil().clamp(1.0, 64.0) as usize;
    for seg in path.segments() {
        match seg {
            tiny_skia::PathSegment::MoveTo(p) => {
                if ring.len() >= 3 {
                    out.push(std::mem::take(&mut ring));
                }
                ring.clear();
                last = map(p);
                ring.push(last);
            }
            tiny_skia::PathSegment::LineTo(p) => {
                last = map(p);
                ring.push(last);
            }
            tiny_skia::PathSegment::QuadTo(c, p) => {
                let (c, p) = (map(c), map(p));
                let n = steps(last, p).max(steps(last, c));
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    let u = 1.0 - t;
                    ring.push(Vec2::new(
                        u * u * last.x + 2.0 * u * t * c.x + t * t * p.x,
                        u * u * last.y + 2.0 * u * t * c.y + t * t * p.y,
                    ));
                }
                last = p;
            }
            tiny_skia::PathSegment::CubicTo(c1, c2, p) => {
                let (c1, c2, p) = (map(c1), map(c2), map(p));
                let n = steps(last, p).max(steps(last, c1)).max(steps(c2, p));
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    let u = 1.0 - t;
                    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    ring.push(Vec2::new(
                        a * last.x + b * c1.x + c * c2.x + d * p.x,
                        a * last.y + b * c1.y + c * c2.y + d * p.y,
                    ));
                }
                last = p;
            }
            tiny_skia::PathSegment::Close => {
                if ring.len() >= 3 {
                    out.push(std::mem::take(&mut ring));
                }
                ring.clear();
            }
        }
    }
    if ring.len() >= 3 {
        out.push(ring);
    }
    out
}

fn area(r: &[Vec2]) -> f64 {
    let n = r.len();
    (0..n).map(|i| r[i].x * r[(i + 1) % n].y - r[(i + 1) % n].x * r[i].y).sum::<f64>() / 2.0
}

fn inside(p: Vec2, r: &[Vec2]) -> bool {
    let mut c = false;
    let n = r.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (r[i], r[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
        j = i;
    }
    c
}

/// Rings filled by the SVG's rule. Inside a ring the winding is its own turn and its outer rings' summed: the nonzero
/// rule fills it where that is not 0, the even-odd rule where its depth is even. A filled ring whose outer ring is not
/// filled starts an area; its holes are the first rings inside it that are not filled. Each area is triangulated
/// once, so nothing is covered twice (a translucent symbol would darken where it was).
fn fill_rings(rings: &[Vec<Vec2>], even_odd: bool) -> Vec<f64> {
    // Rings no edge of which crosses another are nested as they are: the overlay only where one does (a glyph's
    // contours mostly do not; it made a glyph's program take 0.61 ms instead of 0.24, docs/adr/9240).
    if !even_odd && crossing(rings) && let Some(tris) = nonzero(rings) {
        return tris;
    }
    nested(rings, even_odd)
}

/// Whether an edge of the rings crosses or touches another, but where two edges following each other in a ring meet: a
/// sweep along x over the edges' spans.
fn crossing(rings: &[Vec<Vec2>]) -> bool {
    let mut edges: Vec<(Vec2, Vec2, usize, usize, usize)> = Vec::new();
    for (r, ring) in rings.iter().enumerate() {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let (a, b) = if (a.x, a.y) <= (b.x, b.y) { (a, b) } else { (b, a) };
            edges.push((a, b, r, i, n));
        }
    }
    edges.sort_by(|p, q| p.0.x.total_cmp(&q.0.x));
    let orient = |a: Vec2, b: Vec2, c: Vec2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let on = |a: Vec2, b: Vec2, c: Vec2| {
        c.x >= a.x.min(b.x) && c.x <= a.x.max(b.x) && c.y >= a.y.min(b.y) && c.y <= a.y.max(b.y)
    };
    let meet = |(a, b): (Vec2, Vec2), (c, d): (Vec2, Vec2)| {
        let (o1, o2, o3, o4) = (orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b));
        if ((o1 > 0.0 && o2 < 0.0) || (o1 < 0.0 && o2 > 0.0)) && ((o3 > 0.0 && o4 < 0.0) || (o3 < 0.0 && o4 > 0.0)) {
            return true;
        }
        (o1 == 0.0 && on(a, b, c)) || (o2 == 0.0 && on(a, b, d)) || (o3 == 0.0 && on(c, d, a)) || (o4 == 0.0 && on(c, d, b))
    };
    let mut active: Vec<usize> = Vec::new();
    for k in 0..edges.len() {
        let (a, b, r, i, n) = edges[k];
        active.retain(|&j| edges[j].1.x.max(edges[j].0.x) >= a.x);
        for &j in &active {
            let (c, d, rj, ij, _) = edges[j];
            // Edges following each other in a ring share an end: they cross only where one runs back over the other.
            if r == rj && (ij == (i + 1) % n || i == (ij + 1) % n) {
                let shared = [(a, b), (b, a)].into_iter().find_map(|(s, u)| {
                    [(c, d), (d, c)].into_iter().find(|(t, _)| t.x == s.x && t.y == s.y).map(|(_, v)| (s, u, v))
                });
                if let Some((s, u, v)) = shared {
                    if orient(s, u, v) == 0.0 && (u.x - s.x) * (v.x - s.x) + (u.y - s.y) * (v.y - s.y) > 0.0 {
                        return true;
                    }
                    continue;
                }
            }
            if meet((a, b), (c, d)) {
                return true;
            }
        }
        active.push(k);
    }
    false
}

/// The triangles (flat, x y each corner) of rings filled by their winding (nonzero), rings that cross themselves
/// included: an SVG's stroke outline, a glyph's contours (the glyphs' vectors, docs/adr/9239).
pub fn fill_nonzero(rings: &[Vec<Vec2>]) -> Vec<f64> {
    fill_rings(rings, false)
}

/// The rings' area by their winding (nonzero), resolved by the geometry core's overlay into areas whose rings do not
/// cross themselves, then ear-clipped. A stroke's outline crosses itself where the stroke turns sharply (the inside of
/// a join folds over), and ear clipping such a ring lost what lay past the crossing: a sprig's leaf tip and base came
/// out holed (plan/render.md RM-11, docs/adr/9239). None when the overlay gives nothing for rings with an area.
fn nonzero(rings: &[Vec<Vec2>]) -> Option<Vec<f64>> {
    use kentos_geometry_core::geom::arrangement::{Rule, Source};
    use kentos_geometry_core::geom::intersect::Edge;
    let edges: Vec<Edge> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .filter(|(a, b)| a.x != b.x || a.y != b.y)
        .map(|(a, b)| Edge::Seg { a, b })
        .collect();
    if edges.is_empty() {
        return Some(Vec::new());
    }
    let areas = kentos_geometry_core::geom::overlay::overlay(&[Source { edges, points: None, cut: None }], Rule::Any);
    if areas.is_empty() && rings.iter().any(|r| area(r).abs() > 0.0) {
        return None;
    }
    let mut out = Vec::new();
    for a in &areas {
        let holes: Vec<Vec<Vec2>> = a.holes.iter().map(|h| h.pts.clone()).collect();
        out.extend(triangulate(&a.outer.pts, &holes, Vec2::new(0.0, 0.0)));
    }
    Some(out)
}

/// The rings' area when they do not cross themselves: each ring's parent the smallest ring holding it, filled by the
/// nesting's winding (or its depth, even-odd).
fn nested(rings: &[Vec<Vec2>], even_odd: bool) -> Vec<f64> {
    let mut order: Vec<usize> = (0..rings.len()).filter(|&i| area(&rings[i]).abs() > 0.0).collect();
    order.sort_by(|&a, &b| area(&rings[b]).abs().total_cmp(&area(&rings[a]).abs()));
    // Each ring's parent: the smallest ring already placed that holds its first point.
    let mut parent: Vec<Option<usize>> = vec![None; rings.len()];
    let mut depth: Vec<usize> = vec![0; rings.len()];
    let mut winding: Vec<i32> = vec![0; rings.len()];
    for (k, &i) in order.iter().enumerate() {
        let p = rings[i][0];
        let turn = if area(&rings[i]) > 0.0 { 1 } else { -1 };
        match order[..k].iter().rev().find(|&&j| inside(p, &rings[j])) {
            Some(&j) => {
                parent[i] = Some(j);
                depth[i] = depth[j] + 1;
                winding[i] = winding[j] + turn;
            }
            None => winding[i] = turn,
        }
    }
    let filled = |i: usize| if even_odd { depth[i].is_multiple_of(2) } else { winding[i] != 0 };
    // Whether `h` lies in `i`'s area: every ring between them is filled.
    let within = |h: usize, i: usize| {
        let mut up = parent[h];
        while let Some(j) = up {
            if j == i {
                return true;
            }
            if !filled(j) {
                return false;
            }
            up = parent[j];
        }
        false
    };
    let mut out = Vec::new();
    for &i in &order {
        if !filled(i) || parent[i].is_some_and(filled) {
            continue;
        }
        let holes: Vec<Vec<Vec2>> =
            order.iter().filter(|&&h| !filled(h) && within(h, i)).map(|&h| rings[h].clone()).collect();
        out.extend(triangulate(&rings[i], &holes, Vec2::new(0.0, 0.0)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker_batch(svg: &str, markers: &[[f32; MARKER]]) -> StyledLayer {
        let data: Vec<f32> = markers.iter().flatten().copied().collect();
        StyledLayer {
            batches: vec![StyledBatch {
                range: 0..data.len(),
                kind: BatchKind::Marker {
                    unit: Unit::World,
                    look: MarkerLook::Image {
                        image: AtlasImage::Svg { key: "s".into(), svg: svg.into(), width: 10.0, height: 10.0 },
                        fit_height: false,
                    },
                    offset: [0.0, 0.0],
                    anchor: [0.0, 0.0],
                    opacity: 1.0,
                    extent: [2.0, 2.0],
                    vectors: false,
                },
                level: 0.0,
                key: 7,
                bounds: [0.0, 0.0, 10.0, 10.0],
                origin: [0.0, 0.0],
                reach: 2.0,
                reach_unit: Unit::World,
                min_scale: None,
                max_scale: None,
                group: 0,
                effect: None,
                weight_cap: 0.0,
                line_type_loss: None,
                lod: None,
                instances: None,
            }],
            data,
            low: Vec::new(),
            pictures: Vec::new(),
            dropped: 0,
            markers_left: 0,
            waves_left: 0,
            cells: Vec::new(),
        }
    }

    const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>"##;

    #[test]
    fn an_svg_marker_becomes_instanced_triangles_where_the_bitmap_lay() {
        // Two markers 2 m wide at (5, 1) and (8, 3), the second turned a quarter.
        let mut layer = marker_batch(SQUARE, &[[5.0, 1.0, 0.0, 2.0, 0.0], [8.0, 3.0, std::f32::consts::FRAC_PI_2, 2.0, 0.0]]);
        add(&mut layer, None);
        assert_eq!(layer.batches.len(), 2, "the marker batch, then one batch a colour");
        assert!(matches!(layer.batches[0].kind, BatchKind::Marker { vectors: true, .. }), "it keeps its bitmap");
        let b = &layer.batches[1];
        assert!(matches!(b.kind, BatchKind::Fill { paint: FillPaintBatch::Solid { color } } if color == [1.0, 0.0, 0.0, 1.0]));
        let inst = b.instances.as_ref().expect("instanced");
        assert_eq!((inst.count, inst.scale), (2, 2.0));
        let numbers = &layer.data[b.range.clone()];
        let (mesh, placed) = numbers.split_at(inst.mesh);
        assert_eq!(mesh.len(), 12, "a square: two triangles");
        // The mesh spans the symbol's width 1 round its middle.
        let xs: Vec<f32> = mesh.chunks(2).map(|p| p[0]).collect();
        assert_eq!(xs.iter().cloned().fold(f32::MAX, f32::min), -0.5);
        assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 0.5);
        // The first instance: no turn, 2 m wide, at its place.
        assert_eq!(&placed[..INSTANCE_FLOATS], &[2.0, 0.0, -0.0, 2.0, 5.0, 1.0, 2.0, 0.0]);
        // The second: a quarter turn in the shader's pixels (y up) is a quarter counter-clockwise on the ground.
        let q = &placed[INSTANCE_FLOATS..];
        assert!((q[0] - 0.0).abs() < 1e-6 && (q[1] - 2.0).abs() < 1e-6, "x axis up the ground: {q:?}");
        assert!((q[2] + 2.0).abs() < 1e-6 && (q[3] - 0.0).abs() < 1e-6);
    }

    /// Where the marker shader puts an image's point (`markerVertex` with `fit` 1, no twist), on the ground: the box
    /// centred on the place, the image's top row up, the anchor taken off and the offset added, then turned.
    fn shader_place(m: [f64; 5], anchor: [f64; 2], offset: [f64; 2], aspect: f64, u: f64, v: f64) -> [f64; 2] {
        let (w, h) = (m[3], m[3] * aspect);
        let q = [(u - 0.5) * w - anchor[0] * w + offset[0], (0.5 - v) * h - anchor[1] * h + offset[1]];
        let (s, c) = m[2].sin_cos();
        [m[0] + c * q[0] - s * q[1], m[1] + s * q[0] + c * q[1]]
    }

    #[test]
    fn the_vector_symbol_lies_where_the_shader_put_the_bitmap() {
        // A 20 × 10 symbol whose only fill is its top-left quarter, anchored and offset, turned 30°.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="10" height="5" fill="#000000"/></svg>"##;
        let m = [100.0f32, 50.0, 30f32.to_radians(), 4.0, 0.0];
        let mut layer = marker_batch(svg, &[m]);
        if let BatchKind::Marker { look: MarkerLook::Image { image: AtlasImage::Svg { width, height, .. }, .. }, anchor, offset, .. } =
            &mut layer.batches[0].kind
        {
            (*width, *height) = (20.0, 10.0);
            *anchor = [0.25, -0.5];
            *offset = [1.0, 2.0];
        }
        add(&mut layer, None);
        let b = &layer.batches[1];
        let inst = b.instances.as_ref().expect("instanced");
        let numbers = &layer.data[b.range.clone()];
        let (mesh, placed) = numbers.split_at(inst.mesh);
        let p = |x: f32, y: f32| {
            let (x, y) = (f64::from(x), f64::from(y));
            let t: Vec<f64> = placed.iter().map(|&v| f64::from(v)).collect();
            [t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5]]
        };
        let m64 = m.map(f64::from);
        let mut corners: Vec<[f64; 2]> = mesh.chunks(2).map(|q| p(q[0], q[1])).collect();
        corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
        corners.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-4);
        let mut want: Vec<[f64; 2]> = [(0.0, 0.0), (0.5, 0.0), (0.5, 0.5), (0.0, 0.5)]
            .iter()
            .map(|&(u, v)| shader_place(m64, [0.25, -0.5], [1.0, 2.0], 0.5, u, v))
            .collect();
        want.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(corners.len(), 4, "{corners:?}");
        for (a, b) in corners.iter().zip(&want) {
            assert!((a[0] - b[0]).hypot(a[1] - b[1]) < 1e-4, "{corners:?} != {want:?}");
        }
        // A mask's box: the image's middle, its size and its turn, as the shader placed the marker's box.
        let got: Vec<_> = boxes(&layer.data, &layer.low, b).collect();
        let middle = shader_place(m64, [0.25, -0.5], [1.0, 2.0], 0.5, 0.5, 0.5);
        assert_eq!(got.len(), 1);
        let (at, size, turn) = got[0];
        assert!((at[0] - middle[0]).hypot(at[1] - middle[1]) < 1e-4, "{at:?} != {middle:?}");
        assert!((size[0] - 4.0).abs() < 1e-5 && (size[1] - 2.0).abs() < 1e-5, "{size:?}");
        assert!((turn - 30f64.to_radians()).abs() < 1e-6);
    }

    #[test]
    fn a_symbol_draws_where_its_markers_did_and_keeps_their_precision() {
        // Two colours, the second drawn over the first; a place far out on its tile, its residual in the low stream.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/><rect width="5" height="5" fill="#0000ff"/></svg>"##;
        let mut layer = marker_batch(svg, &[[40_000.0, 30_000.0, 0.0, 2.0, 0.0]]);
        layer.low = vec![0.001_25, -0.000_75, 0.0, 0.0, 0.0];
        add(&mut layer, None);
        assert_eq!(layer.batches.len(), 3, "the marker batch, then a batch a colour");
        assert_eq!(layer.low.len(), layer.data.len());
        for (k, b) in layer.batches[1..].iter().enumerate() {
            let inst = b.instances.as_ref().expect("instanced");
            let mark = inst.mark.expect("a symbol's");
            assert_eq!(mark.boxes, k == 0, "the first colour gives the boxes");
            let t = b.range.start + inst.mesh + 4;
            let x = f64::from(layer.data[t]) + f64::from(layer.low[t]);
            let y = f64::from(layer.data[t + 1]) + f64::from(layer.low[t + 1]);
            assert!((x - 40_000.001_25).abs() < 1e-6 && (y - 29_999.999_25).abs() < 1e-6, "{x} {y}");
        }
        assert_eq!(boxes(&layer.data, &layer.low, &layer.batches[1]).count(), 1);
        assert_eq!(boxes(&layer.data, &layer.low, &layer.batches[2]).count(), 0);
        // Built in parts, the symbol keeps its marks' place after the lines.
        let mut line = layer.clone();
        line.batches.truncate(1);
        line.batches[0].kind = BatchKind::Stroke {
            color: [1.0; 4],
            width: 1.0,
            unit: Unit::World,
            dash: None,
            dash_offset: 0.0,
            cap: crate::batches::Cap::Butt,
            blur: 0.0,
        };
        line.batches[0].instances = None;
        line.batches[0].key = 1;
        let order = crate::batches::merged_order(&[&layer.batches, &line.batches]);
        assert_eq!(order, vec![(1, 0), (0, 0), (0, 1), (0, 2)], "the line, then the symbol's bitmap and colours");
    }

    #[test]
    fn a_small_symbol_shows_its_bitmap_and_a_grown_one_its_vectors() {
        // 2 m wide: VECTORS_FROM_PX device px at half as many px a metre.
        let mut layer = marker_batch(SQUARE, &[[5.0, 1.0, 0.0, 2.0, 0.0]]);
        add(&mut layer, None);
        let (bitmap, vectors) = (&layer.batches[0], &layer.batches[1]);
        let at = VECTORS_FROM_PX / 2.0;
        for (ppm, dpr) in [(at * 0.5, 1.0), (at, 1.0), (at * 1.01, 1.0), (at * 8.0, 2.0)] {
            let big = 2.0 * ppm > VECTORS_FROM_PX;
            assert_eq!(bitmap.legible(ppm, dpr), !big, "bitmap at {ppm} px/m");
            assert_eq!(vectors.legible(ppm, dpr), big, "vectors at {ppm} px/m");
        }
    }

    #[test]
    fn a_ring_with_a_hole_and_a_stroke_are_filled_by_colour() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
            <path d="M0 0 H20 V20 H0 Z M5 5 V15 H15 V5 Z" fill="#00ff00" fill-rule="evenodd"/>
            <line x1="0" y1="10" x2="20" y2="10" stroke="#0000ff" stroke-width="2"/></svg>"##;
        let m = mesh(svg, 20.0, 20.0).expect("parses");
        assert_eq!(m.fills.len(), 2, "green and blue");
        // The green square with its hole: its triangles' area is 400 − 100.
        let green = &m.fills[0].1;
        let area: f64 = green.chunks(6).map(|t| ((t[2] - t[0]) * (t[5] - t[1]) - (t[4] - t[0]) * (t[3] - t[1])).abs() / 2.0).sum();
        assert!((area - 300.0).abs() < 1e-6, "{area}");
        // The blue line's outline: 20 long, 2 wide.
        let blue = &m.fills[1].1;
        let area: f64 = blue.chunks(6).map(|t| ((t[2] - t[0]) * (t[5] - t[1]) - (t[4] - t[0]) * (t[3] - t[1])).abs() / 2.0).sum();
        assert!((area - 40.0).abs() < 0.5, "{area}");
    }

    /// plan/render.md RM-11: a stroke turning sharply is outlined by a ring that crosses itself (the join's inside folds
    /// over); ear-clipped as it was, the leaf's tip and base came out holed.
    #[test]
    fn a_stroke_turning_sharply_is_filled_past_its_crossing() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100"><path fill="none" stroke="#000000" stroke-width="5.56" stroke-linecap="round" stroke-linejoin="round" d="M50 91.67L50 36.11M50 36.11Q22.22 34.72 25 8.33Q47.22 9.72 50 36.11Z"/></svg>"##;
        let m = mesh(svg, 100.0, 100.0).expect("parses");
        let held = |p: [f64; 2]| {
            m.fills[0].1.chunks(6).any(|t| {
                let side = |a: usize, b: usize| (t[b] - t[a]) * (p[1] - t[a + 1]) - (t[b + 1] - t[a + 1]) * (p[0] - t[a]);
                let s = [side(0, 2), side(2, 4), side(4, 0)];
                s.iter().all(|&v| v >= 0.0) || s.iter().all(|&v| v <= 0.0)
            })
        };
        // The leaf's tip and its base, where the stroke turns back on itself; the stem's middle; nothing beside it.
        for p in [[25.0, 8.33], [26.0, 9.5], [50.0, 36.11], [48.5, 34.5], [50.0, 60.0]] {
            assert!(held(p), "{p:?} is inked");
        }
        assert!(!held([60.0, 60.0]), "the ground beside the stem");
    }

    /// Rings that cross go to the overlay; nested or apart ones are nested as they are.
    #[test]
    fn a_crossing_is_found_and_nesting_is_not_one() {
        let ring = |pts: &[(f64, f64)]| pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect::<Vec<_>>();
        let square = |x: f64, y: f64, s: f64| ring(&[(x, y), (x + s, y), (x + s, y + s), (x, y + s)]);
        assert!(!crossing(&[square(0.0, 0.0, 10.0)]), "a square");
        assert!(!crossing(&[square(0.0, 0.0, 10.0), square(3.0, 3.0, 2.0)]), "a square in a square");
        assert!(!crossing(&[square(0.0, 0.0, 2.0), square(5.0, 0.0, 2.0)]), "two apart");
        assert!(crossing(&[ring(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)])]), "a figure of eight");
        assert!(crossing(&[square(0.0, 0.0, 10.0), square(5.0, 5.0, 10.0)]), "two overlapping");
        assert!(crossing(&[ring(&[(0.0, 0.0), (10.0, 0.0), (5.0, 0.0), (5.0, 5.0)])]), "a ring running back over itself");
        assert!(crossing(&[square(0.0, 0.0, 10.0), ring(&[(10.0, 5.0), (15.0, 2.0), (15.0, 8.0)])]), "touching at a point");
    }

    #[test]
    fn nested_rings_follow_the_svg_fill_rule() {
        // Two squares turning the same way, one in the other: nonzero fills both once, even-odd cuts the inner out.
        let svg = |rule: &str| {
            format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><path d="M0 0 H20 V20 H0 Z M5 5 H15 V15 H5 Z" fill="#000000" fill-rule="{rule}"/></svg>"##
            )
        };
        let area_of = |rule: &str| {
            let m = mesh(&svg(rule), 20.0, 20.0).expect("parses");
            m.fills[0].1.chunks(6).map(|t| ((t[2] - t[0]) * (t[5] - t[1]) - (t[4] - t[0]) * (t[3] - t[1])).abs() / 2.0).sum::<f64>()
        };
        assert!((area_of("nonzero") - 400.0).abs() < 1e-6, "{}", area_of("nonzero"));
        assert!((area_of("evenodd") - 300.0).abs() < 1e-6, "{}", area_of("evenodd"));
    }

    #[test]
    fn screen_sized_and_unreadable_symbols_keep_their_bitmap() {
        let mut layer = marker_batch("<svg", &[[1.0, 1.0, 0.0, 2.0, 0.0]]);
        let before = layer.batches.clone();
        add(&mut layer, None);
        assert_eq!(layer.batches, before, "an SVG that does not parse keeps its bitmap");
        let mut layer = marker_batch(SQUARE, &[[1.0, 1.0, 0.0, 2.0, 0.0]]);
        if let BatchKind::Marker { unit, .. } = &mut layer.batches[0].kind {
            *unit = Unit::Px;
        }
        let before = layer.batches.clone();
        add(&mut layer, None);
        assert_eq!(layer.batches, before, "a screen-sized one never grows past its bitmap");
    }

    /// A face of one square letter, 50 px, 40 wide, its advance 50.
    struct Square;
    impl crate::picture::ImageSource for Square {
        fn picture(&self, _: &AtlasImage) -> Option<std::sync::Arc<crate::picture::Picture>> {
            None
        }
        fn text(&self, _: &str, _: Option<&str>, _: f64, _: bool) -> Option<std::sync::Arc<crate::picture::TextOutline>> {
            use crate::picture::Segment::{Close, Line, Move};
            let glyph = vec![Move([0.0, -40.0]), Line([40.0, -40.0]), Line([40.0, 0.0]), Line([0.0, 0.0]), Close];
            Some(std::sync::Arc::new(crate::picture::TextOutline { size: 50.0, advance: 50.0, glyphs: vec![glyph] }))
        }
    }

    /// docs/adr/9245: a text marker's SVG is its bitmap's layout: the box 100 high, the letters 80, the baseline at 78,
    /// the halo (a tenth of the letters' size each side) a round-joined stroke under the fill.
    #[test]
    fn a_text_marker_is_written_as_its_bitmap_paints_it() {
        let image = AtlasImage::Text {
            key: "t".into(),
            text: "A".into(),
            font: None,
            weight: 400.0,
            italic: false,
            color: "#ff0000".into(),
            halo: Some(("#ffffff".into(), 0.1)),
        };
        let (svg, width, height) = text_svg(&Square, &image).expect("an SVG");
        // The letters at 80 / 50 = 1.6 times the face's px: the advance 80, the halo 8 each side.
        assert_eq!((width, height), (96.0, 100.0));
        let halo = svg.find("stroke-width=\"16\"").expect("the halo's stroke, twice its share");
        let fill = svg.find("fill=\"rgb(255,0,0)\"").expect("the fill");
        assert!(halo < fill, "the halo under the fill");
        // The square from (8, 78 − 64) to (72, 78): its first corner.
        assert!(svg.contains("M8 14L72 14L72 78L8 78Z"), "{svg}");
        let meshes = mesh(&svg, width, height).expect("its triangles");
        assert_eq!(meshes.fills.len(), 2, "the halo's colour and the fill's");
        assert!((meshes.aspect - 100.0 / 96.0).abs() < 1e-9);
    }


    /// docs/adr/9245: a text marker made vectors is drawn one way at a time, and neither way while too small to read
    /// (the bitmap's batch keeps `MIN_TEXT_PX`).
    #[test]
    fn a_text_marker_is_drawn_one_way_or_none() {
        let image = AtlasImage::Text {
            key: "t".into(),
            text: "A".into(),
            font: None,
            weight: 400.0,
            italic: false,
            color: "#ff0000".into(),
            halo: None,
        };
        let data = vec![0.0f32, 0.0, 0.0, 10.0, 10.0];
        let batch = StyledBatch {
            range: 0..data.len(),
            kind: BatchKind::Marker {
                unit: Unit::World,
                look: MarkerLook::Image { image, fit_height: true },
                offset: [0.0, 0.0],
                anchor: [0.0, 0.0],
                opacity: 1.0,
                extent: [10.0, 10.0],
                vectors: false,
            },
            level: 0.0,
            key: 1,
            bounds: [-10.0, -10.0, 10.0, 10.0],
            origin: [0.0, 0.0],
            reach: 30.0,
            reach_unit: Unit::World,
            min_scale: None,
            max_scale: None,
            group: 0,
            effect: None,
            weight_cap: 0.0,
            line_type_loss: None,
            lod: None,
            instances: None,
        };
        let mut layer = StyledLayer { data, batches: vec![batch], ..Default::default() };
        add(&mut layer, Some(&Square));
        assert_eq!(layer.batches.len(), 2, "the bitmap's batch and the letter's colour");
        let (bitmap, vectors) = (&layer.batches[0], &layer.batches[1]);
        // 10 m high: 0.1 px/m makes it a pixel, 5 px/m 50 px, 20 px/m 200 px.
        assert!(!bitmap.legible(0.1, 1.0) && !vectors.legible(0.1, 1.0), "too small to read: neither");
        assert!(bitmap.legible(5.0, 1.0) && !vectors.legible(5.0, 1.0), "small: the bitmap");
        assert!(!bitmap.legible(20.0, 1.0) && vectors.legible(20.0, 1.0), "grown: the vectors");
    }

}

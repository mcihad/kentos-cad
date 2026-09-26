//! The drawing as the GPU draws it (TODOS.md REN-01, REN-07): GPU-ready
//! arrays built on the CPU from a drawing in the `.kcad` v1 terms (the
//! contracts' `LayerNode` and `Entity`), read through [`Drawing`]: a
//! `DocumentSnapshotV1`, or a host's live document without a copy. Nothing
//! here writes to the drawing; tessellation and triangulation are for
//! display only and come from the shared geometry core.
//!
//! A scene is two parts, uploaded and cached separately:
//! - the fixed part ([`build_fixed`]): points, straight lines and paths,
//!   straight polygons with their fills, hatches. It changes only with the
//!   document (or the palette).
//! - the curves ([`build_curves`]): circles, arcs, ellipses, splines and
//!   bulged paths, tessellated so that no chord strays from the curve by more
//!   than the on-screen tolerance at the current zoom. They are built again
//!   when the zoom leaves their band ([`lod`]), never per frame.
//!
//! Both parts list the same layers in the same order: the leaves of the layer
//! tree that are visible with all their ancestors, the top of the list drawn
//! last (the web's order). Colours are resolved here: the object's own
//! colour, else its layer's; theme tokens through the host's palette.
//! What is not drawn yet (text, dimensions, construction lines) is counted,
//! not guessed at.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use kentos_contracts::{
    DimensionStyle, DocumentSnapshotV1, DrawingFont, Entity, HatchEntity, HatchPatternType,
    LayerNode, LayerNodeType, PathEntity, PointSymbol, RingGeometry,
};
use kentos_geometry_core::entity::{HatchPattern, Shape, entity_bounds_in};
use kentos_geometry_core::geom::arc::sweep;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::bulge::has_bulges;
use kentos_geometry_core::geom::hatch::hatch_lines;
use kentos_geometry_core::tessellate::{
    arc_points, bulge_path, catmull_rom, circle_ring, ellipse_points,
};
use kentos_geometry_core::text::Font;
use kentos_geometry_core::triangulate::triangulate_into;

use crate::color::{Palette, Rgba8};
use crate::layout::{FillVertex, MarkerInstance, SegmentInstance, marker_shape};
use crate::precision::{MAX_SEGMENT_LENGTH, MAX_SEGMENT_PIECES, split_offset};
use crate::{Bounds, Vec2};

/// Diameter of a point mark when the layer's style names none (the web's), logical pixels.
pub const DEFAULT_MARK_SIZE: f32 = 7.0;
/// A solid hatch is its colour at this opacity (the web's).
const SOLID_HATCH_ALPHA: f64 = 0.45;
/// Tries at coarsening the curve tolerance (×4 each) to stay within the budget.
const BUDGET_ATTEMPTS: usize = 12;

static NEXT_PART: AtomicU64 = AtomicU64::new(1);

/// One drawn layer's share of a part: ranges into the part's arrays.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayerRanges {
    /// Fill vertices (whole triangles).
    pub fills: Range<u32>,
    pub segments: Range<u32>,
    pub markers: Range<u32>,
}

/// GPU-ready geometry of some of the drawing's objects.
#[derive(Clone, Debug, Default)]
pub struct ScenePart {
    /// Unique per build: the renderer uploads a part it has not seen by this id.
    pub id: u64,
    /// The local origin the GPU offsets are taken from.
    pub origin: Vec2,
    pub fills: Vec<FillVertex>,
    pub segments: Vec<SegmentInstance>,
    pub markers: Vec<MarkerInstance>,
    /// Per drawn layer, bottom first.
    pub layers: Vec<LayerRanges>,
    /// Chord tolerance the curves were tessellated with, world units; 0 in the fixed part.
    pub tolerance: f64,
    /// Objects on drawn layers that are not drawn yet, by kind (fixed part only).
    pub not_drawn: BTreeMap<&'static str, usize>,
}

impl ScenePart {
    /// Bytes the part takes on the GPU.
    pub fn byte_size(&self) -> u64 {
        (std::mem::size_of_val(self.fills.as_slice())
            + std::mem::size_of_val(self.segments.as_slice())
            + std::mem::size_of_val(self.markers.as_slice())) as u64
    }
}

/// What a scene is built from: read-only access to a drawing, whether a
/// `.kcad` snapshot or a host's live document (the desktop's). The scene
/// keeps nothing of it; a host rebuilds when the drawing's revision changes.
pub trait Drawing {
    /// The top of the layer tree.
    fn layer_tree(&self) -> &[LayerNode];
    /// Every object, in document order.
    fn objects(&self) -> impl Iterator<Item = &Entity>;
    /// The local anchor near the data (`DocumentSnapshotV1::origin`).
    fn anchor(&self) -> kentos_contracts::Vec2;
    /// The project's drawing typeface: text boxes count in the extents.
    fn drawing_font(&self) -> Option<DrawingFont>;
}

impl Drawing for DocumentSnapshotV1 {
    fn layer_tree(&self) -> &[LayerNode] {
        &self.layers
    }

    fn objects(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    fn anchor(&self) -> kentos_contracts::Vec2 {
        self.origin
    }

    fn drawing_font(&self) -> Option<DrawingFont> {
        self.settings.drawing_font
    }
}

/// The local origin a drawing's GPU offsets are taken from: its own anchor
/// (“the GPU works relative to it”, `DocumentSnapshotV1::origin`), or the
/// middle of its objects when the anchor is unusable. The float32 parts keep
/// full precision wherever it lies (`precision`); a near origin only keeps
/// the numbers small.
pub fn scene_origin<D: Drawing + ?Sized>(doc: &D) -> Vec2 {
    let o = doc.anchor();
    if o.x.is_finite() && o.y.is_finite() {
        return Vec2::new(o.x, o.y);
    }
    extents(doc).map_or(Vec2::new(0.0, 0.0), |b| {
        Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0)
    })
}

/// Points, straight lines and paths, straight polygons and their fills, and hatches.
pub fn build_fixed<D: Drawing + ?Sized>(doc: &D, palette: &Palette, origin: Vec2) -> ScenePart {
    let layers = draw_layers(doc.layer_tree(), palette);
    let groups = by_layer(doc, &layers);
    let mut b = Builder::new(origin);
    let mut not_drawn = BTreeMap::new();
    for (layer, entities) in layers.iter().zip(&groups) {
        let start = b.start();
        for entity in entities {
            let color = entity_color(entity, layer, palette);
            match entity {
                Entity::Point(p) => b.marker(v(&p.p), color, layer.mark_size, layer.mark_shape),
                Entity::Line(l) => b.segment(v(&l.a), v(&l.b), color),
                Entity::Polyline(p) if !has_bulges(p.bulges.as_deref()) => {
                    b.path(&points(&p.pts), false, color);
                }
                Entity::Polygon(p) if !polygon_curved(p) => {
                    let mut rings = vec![points(&p.pts)];
                    rings.extend(p.holes.iter().flatten().map(|h| points(&h.pts)));
                    b.polygon(&rings, color, layer.fill);
                }
                Entity::Hatch(h) => b.hatch(h, color),
                Entity::Text(_) | Entity::Dimension(_) | Entity::Xline(_) | Entity::Ray(_) => {
                    *not_drawn.entry(entity.kind()).or_insert(0) += 1;
                }
                // Curves and bulged paths: the curves part.
                _ => {}
            }
        }
        b.finish(start);
    }
    b.into_part(0.0, not_drawn)
}

/// Circles, arcs, ellipses, splines and bulged paths, tessellated with the
/// geometry core so that chords stray from the curve by at most `tolerance`
/// world units. When that would give more than `budget` chords (and fill
/// triangles), the tolerance is coarsened fourfold until it does not; the
/// part's `tolerance` says what was used.
pub fn build_curves<D: Drawing + ?Sized>(
    doc: &D,
    palette: &Palette,
    origin: Vec2,
    tolerance: f64,
    budget: usize,
) -> ScenePart {
    let layers = draw_layers(doc.layer_tree(), palette);
    let groups = by_layer(doc, &layers);
    let mut tol = if tolerance.is_finite() && tolerance > 0.0 {
        tolerance
    } else {
        1.0
    };
    for _ in 0..BUDGET_ATTEMPTS {
        if let Some(part) = curves(&layers, &groups, palette, origin, tol, budget) {
            return part;
        }
        tol *= 4.0;
    }
    // Each curve keeps its fewest chords (the core's minimum per turn) past the last try.
    curves(&layers, &groups, palette, origin, tol, usize::MAX).unwrap_or_default()
}

fn curves(
    layers: &[DrawLayer<'_>],
    groups: &[Vec<&Entity>],
    palette: &Palette,
    origin: Vec2,
    tol: f64,
    budget: usize,
) -> Option<ScenePart> {
    let mut b = Builder::new(origin);
    for (layer, entities) in layers.iter().zip(groups) {
        let start = b.start();
        for entity in entities {
            let color = entity_color(entity, layer, palette);
            match entity {
                Entity::Polyline(p) if has_bulges(p.bulges.as_deref()) => {
                    b.path(
                        &bulge_path(&points(&p.pts), p.bulges.as_deref(), false, tol),
                        false,
                        color,
                    );
                }
                Entity::Polygon(p) if polygon_curved(p) => {
                    let mut rings =
                        vec![bulge_path(&points(&p.pts), p.bulges.as_deref(), true, tol)];
                    rings.extend(
                        p.holes
                            .iter()
                            .flatten()
                            .map(|h| bulge_path(&points(&h.pts), h.bulges.as_deref(), true, tol)),
                    );
                    b.polygon(&rings, color, layer.fill);
                }
                Entity::Circle(c) if valid_radius(c.r) => {
                    b.path(&circle_ring(v(&c.c), c.r, tol), true, color);
                }
                Entity::Arc(a) if valid_radius(a.r) => {
                    b.path(
                        &arc_points(v(&a.c), a.r, a.a0, sweep(a.a0, a.a1), tol),
                        false,
                        color,
                    );
                }
                Entity::Ellipse(e) => {
                    let (pts, closed) =
                        ellipse_points(v(&e.c), v(&e.major), e.ratio, e.t0, e.t1, tol);
                    b.path(&pts, closed, color);
                }
                Entity::Spline(s) => {
                    b.path(
                        &catmull_rom(&points(&s.pts), s.closed, tol),
                        s.closed,
                        color,
                    );
                }
                _ => {}
            }
            if b.segments.len() + b.fills.len() / 3 > budget {
                return None;
            }
        }
        b.finish(start);
    }
    Some(b.into_part(tol, BTreeMap::new()))
}

/// How a highlight draws its objects: the web's overrides for the selection
/// and the hovered object (`uploadHighlight` in ViewportController:
/// `overrideColor`, `overrideFill`, `pointStyle`). Every stroke and mark in
/// one colour; closed areas and hatches outlined and, with `fill`, filled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Highlight {
    pub color: Rgba8,
    /// The fill of closed areas and hatches; none draws outlines only.
    pub fill: Option<Rgba8>,
    /// Point marks' diameter in logical pixels, and their shape (`marker_shape`).
    pub mark_size: f32,
    pub mark_shape: u32,
}

/// Objects drawn over the scene as a highlight (docs/adr/0029): the
/// selection, or the hovered object. Straight and curved geometry in one
/// part, curves tessellated within `tolerance` world units as the curves
/// part is. Its one layer comes after `below` empty ones, so every pass
/// draws it after the scene's layers (the scene has `below`). What the scene
/// does not draw yet (text, dimensions, construction lines) is not
/// highlighted either.
pub fn build_highlight<'a>(
    objects: impl IntoIterator<Item = &'a Entity>,
    style: &Highlight,
    origin: Vec2,
    tolerance: f64,
    below: usize,
) -> ScenePart {
    let tol = if tolerance.is_finite() && tolerance > 0.0 {
        tolerance
    } else {
        1.0
    };
    let mut b = Builder::new(origin);
    for _ in 0..below {
        let start = b.start();
        b.finish(start);
    }
    let start = b.start();
    let color = style.color;
    for entity in objects {
        match entity {
            Entity::Point(p) => b.marker(v(&p.p), color, style.mark_size, style.mark_shape),
            Entity::Line(l) => b.segment(v(&l.a), v(&l.b), color),
            Entity::Polyline(p) => {
                b.path(
                    &bulge_path(&points(&p.pts), p.bulges.as_deref(), false, tol),
                    false,
                    color,
                );
            }
            Entity::Polygon(p) => {
                let mut rings = vec![bulge_path(&points(&p.pts), p.bulges.as_deref(), true, tol)];
                rings.extend(
                    p.holes
                        .iter()
                        .flatten()
                        .map(|h| bulge_path(&points(&h.pts), h.bulges.as_deref(), true, tol)),
                );
                b.polygon(&rings, color, style.fill);
            }
            Entity::Hatch(h) => {
                let mut rings = vec![points(&h.ring)];
                rings.extend(h.holes.iter().flatten().map(|r| points(r)));
                b.polygon(&rings, color, style.fill);
            }
            Entity::Circle(c) if valid_radius(c.r) => {
                b.path(&circle_ring(v(&c.c), c.r, tol), true, color);
            }
            Entity::Arc(a) if valid_radius(a.r) => {
                b.path(
                    &arc_points(v(&a.c), a.r, a.a0, sweep(a.a0, a.a1), tol),
                    false,
                    color,
                );
            }
            Entity::Ellipse(e) => {
                let (pts, closed) = ellipse_points(v(&e.c), v(&e.major), e.ratio, e.t0, e.t1, tol);
                b.path(&pts, closed, color);
            }
            Entity::Spline(s) => {
                b.path(
                    &catmull_rom(&points(&s.pts), s.closed, tol),
                    s.closed,
                    color,
                );
            }
            _ => {}
        }
    }
    b.finish(start);
    b.into_part(tol, BTreeMap::new())
}

/// The box around every object, as zoom to extents takes it on the web: the
/// union of each object's box from the geometry core (`entity_bounds_in`, the
/// store's `extent`), hidden layers included, text measured in the project's
/// typeface. `None` for a drawing without a measurable object.
pub fn extents<D: Drawing + ?Sized>(doc: &D) -> Option<Bounds> {
    extents_of(doc, doc.objects())
}

/// The box around some of the drawing's objects (the ones an import added),
/// measured as [`extents`] measures them.
pub fn extents_of<'a, D: Drawing + ?Sized>(
    doc: &D,
    objects: impl IntoIterator<Item = &'a Entity>,
) -> Option<Bounds> {
    let font = Font::from_id(font_id(doc.drawing_font()));
    let mut out: Option<Bounds> = None;
    for entity in objects {
        let b = entity_bounds_in(&shape(entity), font);
        let usable = [b.min_x, b.min_y, b.max_x, b.max_y]
            .iter()
            .all(|c| c.is_finite())
            && b.min_x <= b.max_x
            && b.min_y <= b.max_y;
        if !usable {
            continue;
        }
        out = Some(match out {
            None => b,
            Some(o) => Bounds {
                min_x: o.min_x.min(b.min_x),
                min_y: o.min_y.min(b.min_y),
                max_x: o.max_x.max(b.max_x),
                max_y: o.max_y.max(b.max_y),
            },
        });
    }
    out
}

/// Curve level of detail, a first step towards REN-10. Curves are
/// tessellated for a zoom band: a band is the chord tolerance in world units
/// rounded down to a power of two. They are built one band finer than the
/// view needs, so zooming in by up to 4× keeps every chord within the
/// on-screen tolerance; past that, or after zooming out by more than 8×, they
/// are built again. Never per frame.
pub mod lod {
    /// The band a view at `scale` logical pixels per unit needs for chords within `tolerance_px`.
    pub fn band(scale: f64, tolerance_px: f64) -> i32 {
        let t = tolerance_px / scale;
        if t.is_finite() && t > 0.0 {
            t.log2().floor().clamp(-80.0, 80.0) as i32
        } else {
            0
        }
    }

    /// The band to build curves in for a view at `scale`: one finer than it needs.
    pub fn build_band(scale: f64, tolerance_px: f64) -> i32 {
        band(scale, tolerance_px) - 1
    }

    /// The world tolerance of a band.
    pub fn tolerance(band: i32) -> f64 {
        2f64.powi(band)
    }

    /// Whether curves built in band `built` must be built again for a view needing band `now`.
    pub fn stale(built: i32, now: i32) -> bool {
        now < built || now > built + 4
    }
}

/// A leaf layer that draws, and what its style gives its objects.
struct DrawLayer<'a> {
    node: &'a LayerNode,
    color: Rgba8,
    fill: Option<Rgba8>,
    mark_size: f32,
    mark_shape: u32,
}

/// The layers that draw, bottom first: leaves visible with every ancestor, in
/// tree order reversed (the web's `leaves()` filtered by `isVisible`,
/// reversed: the top of the list is drawn last).
fn draw_layers<'a>(nodes: &'a [LayerNode], palette: &Palette) -> Vec<DrawLayer<'a>> {
    fn walk<'a>(nodes: &'a [LayerNode], visible: bool, out: &mut Vec<&'a LayerNode>) {
        for node in nodes {
            match node.kind {
                LayerNodeType::Layer => {
                    if visible && node.visible {
                        out.push(node);
                    }
                }
                LayerNodeType::Group => walk(&node.children, visible && node.visible, out),
            }
        }
    }
    let mut leaves = Vec::new();
    walk(nodes, true, &mut leaves);
    leaves.reverse();
    leaves
        .into_iter()
        .map(|node| {
            let style = &node.style;
            let point = style.point.as_ref();
            DrawLayer {
                node,
                color: palette.resolve(&style.color).unwrap_or(palette.fg),
                fill: style.fill.as_deref().and_then(|f| palette.resolve(f)),
                mark_size: point
                    .map(|p| p.size)
                    .filter(|s| s.is_finite() && *s > 0.0)
                    .map_or(DEFAULT_MARK_SIZE, |s| s.clamp(1.0, 256.0) as f32),
                mark_shape: match point.map(|p| p.symbol) {
                    Some(PointSymbol::Cross) => marker_shape::CROSS,
                    Some(PointSymbol::Triangle) => marker_shape::TRIANGLE,
                    Some(PointSymbol::Ring) | None => marker_shape::RING,
                },
            }
        })
        .collect()
}

/// The objects of each drawn layer, in document order; objects of hidden or unknown layers are left out.
fn by_layer<'a, D: Drawing + ?Sized>(doc: &'a D, layers: &[DrawLayer<'_>]) -> Vec<Vec<&'a Entity>> {
    let index: HashMap<&str, usize> = layers
        .iter()
        .enumerate()
        .map(|(i, l)| (l.node.id.as_str(), i))
        .collect();
    let mut out = vec![Vec::new(); layers.len()];
    for entity in doc.objects() {
        if let Some(group) = index
            .get(entity.base().layer_id.as_str())
            .and_then(|&i| out.get_mut(i))
        {
            group.push(entity);
        }
    }
    out
}

/// The object's own colour, else its layer's (“katmana göre”).
fn entity_color(entity: &Entity, layer: &DrawLayer<'_>, palette: &Palette) -> Rgba8 {
    entity
        .base()
        .color
        .as_deref()
        .and_then(|c| palette.resolve(c))
        .unwrap_or(layer.color)
}

fn polygon_curved(p: &PathEntity) -> bool {
    has_bulges(p.bulges.as_deref())
        || p.holes
            .iter()
            .flatten()
            .any(|h| has_bulges(h.bulges.as_deref()))
}

fn valid_radius(r: f64) -> bool {
    r.is_finite() && r > 0.0
}

fn v(p: &kentos_contracts::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn points(pts: &[kentos_contracts::Vec2]) -> Vec<Vec2> {
    pts.iter().map(v).collect()
}

fn finite(p: Vec2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// Appends a part's arrays, layer by layer.
struct Builder {
    origin: Vec2,
    fills: Vec<FillVertex>,
    segments: Vec<SegmentInstance>,
    markers: Vec<MarkerInstance>,
    layers: Vec<LayerRanges>,
    // Scratch for triangulation.
    ring_pts: Vec<Vec2>,
    ring_ranges: Vec<Range<usize>>,
    indices: Vec<u32>,
}

impl Builder {
    fn new(origin: Vec2) -> Self {
        Self {
            origin,
            fills: Vec::new(),
            segments: Vec::new(),
            markers: Vec::new(),
            layers: Vec::new(),
            ring_pts: Vec::new(),
            ring_ranges: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn start(&self) -> [u32; 3] {
        [
            self.fills.len() as u32,
            self.segments.len() as u32,
            self.markers.len() as u32,
        ]
    }

    fn finish(&mut self, [fills, segments, markers]: [u32; 3]) {
        self.layers.push(LayerRanges {
            fills: fills..self.fills.len() as u32,
            segments: segments..self.segments.len() as u32,
            markers: markers..self.markers.len() as u32,
        });
    }

    fn into_part(self, tolerance: f64, not_drawn: BTreeMap<&'static str, usize>) -> ScenePart {
        ScenePart {
            id: NEXT_PART.fetch_add(1, Ordering::Relaxed),
            origin: self.origin,
            fills: self.fills,
            segments: self.segments,
            markers: self.markers,
            layers: self.layers,
            tolerance,
            not_drawn,
        }
    }

    /// A straight segment, cut into pieces no longer than `MAX_SEGMENT_LENGTH`.
    fn segment(&mut self, a: Vec2, b: Vec2, color: Rgba8) {
        if !(finite(a) && finite(b)) || a == b {
            return;
        }
        let length = (b.x - a.x).hypot(b.y - a.y);
        let pieces = if length > MAX_SEGMENT_LENGTH {
            ((length / MAX_SEGMENT_LENGTH).ceil() as usize).clamp(1, MAX_SEGMENT_PIECES)
        } else {
            1
        };
        let mut from = a;
        for i in 1..=pieces {
            let to = if i == pieces {
                b
            } else {
                let t = i as f64 / pieces as f64;
                Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
            };
            self.piece(from, to, color);
            from = to;
        }
    }

    fn piece(&mut self, a: Vec2, b: Vec2, color: Rgba8) {
        let (a_hi, a_lo) = split_offset(a, self.origin);
        let (b_hi, b_lo) = split_offset(b, self.origin);
        let parts = [a_hi, a_lo, b_hi, b_lo];
        if parts.iter().flatten().all(|c| c.is_finite()) {
            self.segments.push(SegmentInstance {
                a_hi,
                a_lo,
                b_hi,
                b_lo,
                color: color.0,
            });
        }
    }

    /// A path through `pts`; `closed` joins the last point back to the first.
    fn path(&mut self, pts: &[Vec2], closed: bool, color: Rgba8) {
        for pair in pts.windows(2) {
            self.segment(pair[0], pair[1], color);
        }
        if closed
            && pts.len() > 2
            && let (Some(&last), Some(&first)) = (pts.last(), pts.first())
        {
            self.segment(last, first, color);
        }
    }

    /// A polygon: its rings (outer first) as outlines, and filled when the layer has a fill.
    fn polygon(&mut self, rings: &[Vec<Vec2>], color: Rgba8, fill: Option<Rgba8>) {
        for ring in rings {
            self.path(ring, true, color);
        }
        if let Some(fill) = fill {
            self.fill(rings, fill);
        }
    }

    /// Triangles over the outer ring minus the holes (the geometry core's ear clipping).
    fn fill(&mut self, rings: &[Vec<Vec2>], color: Rgba8) {
        self.ring_pts.clear();
        self.ring_ranges.clear();
        self.indices.clear();
        for (i, ring) in rings.iter().enumerate() {
            let usable = ring.len() >= 3 && ring.iter().all(|p| finite(*p));
            if !usable {
                if i == 0 {
                    return;
                }
                continue;
            }
            let start = self.ring_pts.len();
            self.ring_pts.extend_from_slice(ring);
            self.ring_ranges.push(start..self.ring_pts.len());
        }
        triangulate_into(&self.ring_pts, &self.ring_ranges, &mut self.indices);
        let whole = self.indices.len() - self.indices.len() % 3;
        for &i in &self.indices[..whole] {
            if let Some(&p) = self.ring_pts.get(i as usize) {
                let (hi, lo) = split_offset(p, self.origin);
                self.fills.push(FillVertex {
                    hi,
                    lo,
                    color: color.0,
                });
            }
        }
    }

    fn marker(&mut self, p: Vec2, color: Rgba8, size: f32, shape: u32) {
        if !finite(p) {
            return;
        }
        let (hi, lo) = split_offset(p, self.origin);
        self.markers.push(MarkerInstance {
            hi,
            lo,
            color: color.0,
            size,
            shape,
        });
    }

    /// A hatch as the web draws it: a solid one filled at 45 %, lines and
    /// crosses as the geometry core's clipped hatch lines.
    fn hatch(&mut self, h: &HatchEntity, color: Rgba8) {
        let ring = points(&h.ring);
        let holes: Vec<Vec<Vec2>> = h.holes.iter().flatten().map(|r| points(r)).collect();
        match h.pattern.kind {
            HatchPatternType::Solid => {
                let mut rings = Vec::with_capacity(holes.len() + 1);
                rings.push(ring);
                rings.extend(holes);
                self.fill(&rings, color.with_alpha(SOLID_HATCH_ALPHA));
            }
            HatchPatternType::Lines | HatchPatternType::Cross => {
                let angles = [h.pattern.angle, h.pattern.angle + 90.0];
                let count = if h.pattern.kind == HatchPatternType::Cross {
                    2
                } else {
                    1
                };
                for &angle in &angles[..count] {
                    for [p, q] in hatch_lines(&ring, angle, h.pattern.spacing, &holes).segments {
                        self.segment(p, q, color);
                    }
                }
            }
        }
    }
}

fn font_id(font: Option<DrawingFont>) -> &'static str {
    match font {
        None | Some(DrawingFont::Barlow) => "barlow",
        Some(DrawingFont::Arimo) => "arimo",
        Some(DrawingFont::Overpass) => "overpass",
        Some(DrawingFont::Quicksand) => "quicksand",
        Some(DrawingFont::ArchitectsDaughter) => "architects-daughter",
        Some(DrawingFont::CourierPrime) => "courier-prime",
        Some(DrawingFont::PlexMono) => "plex-mono",
    }
}

fn ring(r: &RingGeometry) -> Ring {
    Ring {
        pts: points(&r.pts),
        bulges: r.bulges.clone(),
    }
}

/// An object's geometry as the geometry core takes it (its own `Shape`).
fn shape(entity: &Entity) -> Shape {
    match entity {
        Entity::Point(p) => Shape::Point { p: v(&p.p), z: p.z },
        Entity::Line(l) => Shape::Line {
            a: v(&l.a),
            b: v(&l.b),
        },
        Entity::Polyline(p) => Shape::Polyline {
            pts: points(&p.pts),
            bulges: p.bulges.clone(),
            holes: p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
        },
        Entity::Polygon(p) => Shape::Polygon {
            pts: points(&p.pts),
            bulges: p.bulges.clone(),
            holes: p.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
        },
        Entity::Circle(c) => Shape::Circle { c: v(&c.c), r: c.r },
        Entity::Arc(a) => Shape::Arc {
            c: v(&a.c),
            r: a.r,
            a0: a.a0,
            a1: a.a1,
        },
        Entity::Ellipse(e) => Shape::Ellipse {
            c: v(&e.c),
            major: v(&e.major),
            ratio: e.ratio,
            t0: e.t0,
            t1: e.t1,
        },
        Entity::Spline(s) => Shape::Spline {
            pts: points(&s.pts),
            closed: s.closed,
        },
        Entity::Xline(c) => Shape::Xline {
            p: v(&c.p),
            dir: v(&c.dir),
        },
        Entity::Ray(c) => Shape::Ray {
            p: v(&c.p),
            dir: v(&c.dir),
        },
        Entity::Text(t) => Shape::Text {
            p: v(&t.p),
            text: t.text.clone(),
            height: t.height,
            rotation: t.rotation,
        },
        Entity::Dimension(d) => Shape::Dimension {
            a: v(&d.a),
            b: v(&d.b),
            offset: d.offset,
            height: d.height,
            text: d.text.clone(),
            style: d.style.map(|s| {
                match s {
                    DimensionStyle::Aligned => "aligned",
                    DimensionStyle::Linear => "linear",
                    DimensionStyle::Angular => "angular",
                    DimensionStyle::Radius => "radius",
                    DimensionStyle::Diameter => "diameter",
                }
                .to_owned()
            }),
            angle: d.angle,
            c: d.c.as_ref().map(v),
        },
        Entity::Hatch(h) => Shape::Hatch {
            ring: points(&h.ring),
            holes: h
                .holes
                .as_ref()
                .map(|hs| hs.iter().map(|r| points(r)).collect()),
            pattern: HatchPattern {
                kind: match h.pattern.kind {
                    HatchPatternType::Solid => "solid",
                    HatchPatternType::Lines => "lines",
                    HatchPatternType::Cross => "cross",
                }
                .to_owned(),
                angle: h.pattern.angle,
                spacing: h.pattern.spacing,
            },
        },
    }
}

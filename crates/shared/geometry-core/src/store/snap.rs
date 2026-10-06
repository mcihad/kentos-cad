//! Object snap on the store (`PickIndex.snap`, `apps/web/src/viewport/picking.ts`):
//! endpoints, midpoints, centres, nodes, quadrants, intersections,
//! perpendicular and tangent points from the last point, nearest; a closed
//! area's centroid, an acquired end's extension, the parallel through the
//! last point to an acquired edge, Karelaj's grid nodes (docs/adr/0163). At
//! equal distance the more meaningful kind wins; "nearest" only when nothing
//! else does, a grid node only when not even that. A layer's objects take its
//! own kinds; the object being drawn is snapped to as one more object.
//! Candidates come in the document's order, as the TypeScript's did.

use super::Store;
use crate::entity::{Shape, area_parts, dimension_geom, ellipse_geom, entity_vertices};
use crate::geom::arc::{ArcGeom, arc_end, arc_mid, arc_start};
use crate::geom::bulge::{bulge_arc, bulge_at, segment_mid};
use crate::geom::centroid::areas_centroid;
use crate::geom::dimension::{is_new_kind, layout_dimension};
use crate::geom::ellipse::{
    EllipseGeom, closest_param, ellipse_point, ellipse_tangent_points, is_full_ellipse,
    line_ellipse, quadrant_params, tessellate_ellipse,
};
use crate::geom::intersect::{
    Edge, closest_on_edge, intersect_edges, on_edge_arc, perpendicular_foot, tangent_points,
};
use crate::geom::leader;
use crate::jsmath::{PI, atan2, js_hypot, js_max, js_round, js_sign};
use crate::ops::areas::areas_of_entity;
use crate::ops::edges::{entity_edges, entity_edges_in};
use crate::vec2::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    Endpoint,
    Midpoint,
    Center,
    Node,
    Quadrant,
    Intersection,
    Perpendicular,
    Tangent,
    Nearest,
    /// Ağırlık merkezi: a closed area's centroid (docs/adr/0163 §1).
    Centroid,
    /// Uzantı: on an acquired end's extension.
    Extension,
    /// Paralel: on the line through the last point parallel to an acquired edge.
    Parallel,
    /// Karelaj: the nearest node of the grid, wherever the cursor is.
    Grid,
}

impl SnapKind {
    /// Kinds in bit order: the TypeScript names them, the bits carry a set across the boundary.
    pub const ALL: [SnapKind; 13] = [
        SnapKind::Endpoint,
        SnapKind::Midpoint,
        SnapKind::Center,
        SnapKind::Node,
        SnapKind::Quadrant,
        SnapKind::Intersection,
        SnapKind::Perpendicular,
        SnapKind::Tangent,
        SnapKind::Nearest,
        SnapKind::Centroid,
        SnapKind::Extension,
        SnapKind::Parallel,
        SnapKind::Grid,
    ];

    pub fn bit(self) -> u32 {
        1 << (self as u32)
    }

    /// Distance multiplier when several snaps compete: at equal distance the more meaningful point wins.
    fn weight(self) -> f64 {
        match self {
            SnapKind::Endpoint | SnapKind::Node => 1.0,
            SnapKind::Intersection => 1.02,
            SnapKind::Center => 1.08,
            SnapKind::Quadrant => 1.1,
            SnapKind::Midpoint => 1.15,
            SnapKind::Perpendicular => 1.25,
            SnapKind::Tangent => 1.2,
            SnapKind::Centroid => 1.08,
            // On an extension or a parallel: soft, so that their crossings
            // (weighed as crossings) and the objects' own points win near them.
            SnapKind::Extension | SnapKind::Parallel => 2.0,
            // Taken only when nothing else is (`Store::snap_ex`).
            SnapKind::Nearest | SnapKind::Grid => f64::INFINITY,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapHit {
    pub kind: SnapKind,
    pub point: Vec2,
    /// The object snapped to; [`NO_OBJECT`] for the object being drawn, an
    /// acquired extension or parallel and a grid node.
    pub id: f64,
}

/// The id of a snap to no object of the drawing (docs/adr/0163).
pub const NO_OBJECT: f64 = -1.0;

/// Where an acquired end goes on (docs/adr/0163 §1): a straight edge's line
/// beyond the end, or the rest of an arc's circle as an arc of its own that
/// starts at the end rested on and goes away from the arc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extension {
    Line {
        end: Vec2,
        dir: Vec2,
    },
    Arc {
        c: Vec2,
        r: f64,
        a0: f64,
        sweep: f64,
    },
}

crate::json_tagged!(Extension, "kind", Line => "line" { end, dir }, Arc => "arc" { c, r, a0, sweep });

/// How near a point must lie to an extension to be on it, metres.
const ON: f64 = 1e-6;

impl Extension {
    /// How far `p` lies along it from its end (a line's distance beyond the
    /// end, an arc's length around the rest of the circle), when `p` is on
    /// it within 1 µm (docs/adr/0163 §2: the snap's tag and a typed
    /// distance).
    pub fn along(&self, p: Vec2) -> Option<f64> {
        match *self {
            Extension::Line { end, dir } => {
                let (dx, dy) = (p.x - end.x, p.y - end.y);
                let t = dx * dir.x + dy * dir.y;
                let off = dx * dir.y - dy * dir.x;
                (t >= -ON && off.abs() <= ON).then_some(js_max(t, 0.0))
            }
            Extension::Arc { c, r, a0, sweep } => {
                if (js_hypot(p.x - c.x, p.y - c.y) - r).abs() > ON {
                    return None;
                }
                // The angle travelled from the end in the sweep's sense, 0..2π.
                let a = atan2(p.y - c.y, p.x - c.x);
                let mut t = (a - a0) * js_sign(sweep);
                while t < 0.0 {
                    t += 2.0 * PI;
                }
                while t >= 2.0 * PI {
                    t -= 2.0 * PI;
                }
                (t * r <= sweep.abs() * r + ON).then_some(t * r)
            }
        }
    }

    /// The point `d` along it from its end; none before the end or past an
    /// arc's remainder.
    pub fn at(&self, d: f64) -> Option<Vec2> {
        if !d.is_finite() || d < 0.0 {
            return None;
        }
        match *self {
            Extension::Line { end, dir } => Some(Vec2::new(end.x + dir.x * d, end.y + dir.y * d)),
            Extension::Arc { c, r, a0, sweep } => {
                (d <= sweep.abs() * r).then(|| point_on(c, r, a0 + js_sign(sweep) * d / r))
            }
        }
    }
}

/// What a snap takes besides the drawing (docs/adr/0163 §1–§3): the acquired
/// extensions and parallel directions (unit), the object being drawn, and
/// Karelaj's spacing along x and y (metres).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapExtras {
    pub extensions: Vec<Extension>,
    pub parallels: Vec<Vec2>,
    pub draft: Vec<Shape>,
    pub grid: Option<[f64; 2]>,
}

/// An edge near the cursor, for crossings; `ell` when it is one of an ellipse's chords.
struct Nearby {
    id: f64,
    ed: Edge,
    ell: Option<EllipseGeom>,
    /// Its layer's snap kinds (`LayerFlags::snap`).
    mask: u32,
}

impl Nearby {
    /// A plain segment's ends: not an arc, not an ellipse's chord.
    fn segment(&self) -> Option<(Vec2, Vec2)> {
        match (self.ell, self.ed) {
            (None, Edge::Seg { a, b }) => Some((a, b)),
            _ => None,
        }
    }
}

/// The running choice: the best weighted snap and the nearest point.
struct Choice {
    p: Vec2,
    tol: f64,
    kinds: u32,
    /// The snap kinds of the layer whose object is being considered.
    mask: u32,
    best: Option<(SnapHit, f64)>,
    nearest: Option<(SnapHit, f64)>,
}

impl Choice {
    fn consider(&mut self, kind: SnapKind, q: Vec2, id: f64) {
        self.consider_as(kind, q, id, kind.weight());
    }

    /// `consider` with its own weight: an extension's crossing weighs as a crossing.
    fn consider_as(&mut self, kind: SnapKind, q: Vec2, id: f64, weight: f64) {
        if self.kinds & self.mask & kind.bit() == 0 {
            return;
        }
        // Farther than the aperture along an axis is farther in all (the hypotenuse is never shorter
        // than a side): refused without the hypotenuse. A NaN passes on to the check below, as before.
        if (q.x - self.p.x).abs() > self.tol || (q.y - self.p.y).abs() > self.tol {
            return;
        }
        let d = js_hypot(q.x - self.p.x, q.y - self.p.y);
        if d > self.tol {
            return;
        }
        let hit = SnapHit { kind, point: q, id };
        if kind == SnapKind::Nearest {
            if self.nearest.is_none_or(|(_, n)| d < n) {
                self.nearest = Some((hit, d));
            }
            return;
        }
        let w = d * weight;
        if self.best.is_none_or(|(_, b)| w < b) {
            self.best = Some((hit, w));
        }
    }

    /// Whether no crossing at least `g` from the cursor can change the
    /// choice: beyond the aperture it is refused; at `g` ≥ the best weighted
    /// distance its own weighted distance (×1.02, rounded, which never goes
    /// below `g`) cannot be smaller. NaN never counts as out of reach.
    fn out_of_reach(&self, g: f64) -> bool {
        g > self.tol || self.best.is_some_and(|(_, b)| g >= b)
    }
}

impl Store {
    /// The snap point near `p` among `kinds` (a set of `SnapKind::bit`s);
    /// `from` is the running command's last point (perpendicular, tangent).
    pub fn snap(&self, p: Vec2, tol: f64, kinds: u32, from: Option<Vec2>) -> Option<SnapHit> {
        self.snap_with(p, tol, kinds, from, &SnapExtras::default(), crossings)
    }

    /// `snap` with what the drawing does not hold (docs/adr/0163): the
    /// acquired extensions and parallels, the object being drawn, Karelaj.
    pub fn snap_ex(
        &self,
        p: Vec2,
        tol: f64,
        kinds: u32,
        from: Option<Vec2>,
        extras: &SnapExtras,
    ) -> Option<SnapHit> {
        self.snap_with(p, tol, kinds, from, extras, crossings)
    }

    /// `snap_ex` with the pairwise crossings given (the tests compare every pair).
    fn snap_with(
        &self,
        p: Vec2,
        tol: f64,
        kinds: u32,
        from: Option<Vec2>,
        extras: &SnapExtras,
        crossings: impl Fn(&mut Choice, &[Nearby], Vec2),
    ) -> Option<SnapHit> {
        // Where an edge can matter: a curve is split only near it (docs/adr/0149 §5.3).
        let reach = crate::geometry::Bounds {
            min_x: p.x - tol,
            min_y: p.y - tol,
            max_x: p.x + tol,
            max_y: p.y + tol,
        };
        let mut ch = Choice {
            p,
            tol,
            kinds,
            mask: u32::MAX,
            best: None,
            nearest: None,
        };
        let mut nearby: Vec<Nearby> = Vec::new();
        for it in self.near(p, tol) {
            let id = it.id;
            ch.mask = self.flags(it).snap;
            // A block's insertion point, then its pieces as objects of their own (docs/adr/0144).
            if it.expanded.is_some()
                && let Shape::Insert { p: q, .. } = &it.shape
            {
                ch.consider(SnapKind::Node, *q, id);
            }
            for e in it.shapes() {
                shape_snaps(&mut ch, &mut nearby, e, id, from, &reach);
            }
            // Ağırlık merkezi (docs/adr/0163 §1): a closed area's, holes and parts weighed.
            if kinds & SnapKind::Centroid.bit() != 0
                && matches!(it.shape, Shape::Polygon { .. } | Shape::Polyline { .. })
                && let Some(c) = areas_centroid(&areas_of_entity(&it.shape))
            {
                ch.consider(SnapKind::Centroid, c, id);
            }
        }
        // The object being drawn, as one more object (docs/adr/0163 §3).
        ch.mask = u32::MAX;
        for e in &extras.draft {
            shape_snaps(&mut ch, &mut nearby, e, NO_OBJECT, from, &reach);
        }
        // Acquired extensions and parallels (§1–§2).
        extension_snaps(&mut ch, &nearby, &extras.extensions);
        if let Some(f) = from {
            for u in &extras.parallels {
                let t = (p.x - f.x) * u.x + (p.y - f.y) * u.y;
                ch.consider(
                    SnapKind::Parallel,
                    Vec2::new(f.x + u.x * t, f.y + u.y * t),
                    NO_OBJECT,
                );
            }
        }

        if kinds & SnapKind::Intersection.bit() != 0 {
            crossings(&mut ch, &nearby, p);
        }
        let found = ch.best.or(ch.nearest).map(|(h, _)| h);
        // Karelaj: the nearest grid node when nothing else is (§1).
        found.or_else(|| {
            let [gx, gy] = extras.grid?;
            (kinds & SnapKind::Grid.bit() != 0).then(|| SnapHit {
                kind: SnapKind::Grid,
                point: Vec2::new(grid_coord(p.x, gx), grid_coord(p.y, gy)),
                id: NO_OBJECT,
            })
        })
    }

    /// The extensions of object `id`'s edges that end at `at`, within 1 µm
    /// (docs/adr/0163 §2): a straight edge's line beyond the end, an arc's
    /// circle beyond it (the rest of the circle). A corner where two edges
    /// meet gives both; a closed shape's every corner is such a corner.
    pub fn extensions_at(&self, id: f64, at: Vec2) -> Vec<Extension> {
        const SAME: f64 = 1e-6;
        let mut out = Vec::new();
        let Some(it) = self.get(id) else {
            return out;
        };
        let near = |q: Vec2| js_hypot(q.x - at.x, q.y - at.y) <= SAME;
        for e in it.shapes() {
            for ed in entity_edges(e) {
                match ed {
                    Edge::Seg { a, b } => {
                        let l = js_hypot(b.x - a.x, b.y - a.y);
                        if l <= SAME {
                            continue;
                        }
                        let u = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
                        if near(b) {
                            out.push(Extension::Line { end: b, dir: u });
                        }
                        if near(a) {
                            out.push(Extension::Line {
                                end: a,
                                dir: Vec2::new(-u.x, -u.y),
                            });
                        }
                    }
                    Edge::Arc { c, r, a0, sweep } => {
                        // The rest of the circle, from the end rested on away from the arc.
                        let rest = (2.0 * PI - sweep.abs()) * js_sign(sweep);
                        let (s, f) = (point_on(c, r, a0), point_on(c, r, a0 + sweep));
                        if rest.abs() <= 1e-12 {
                            continue;
                        }
                        if near(f) {
                            out.push(Extension::Arc {
                                c,
                                r,
                                a0: a0 + sweep,
                                sweep: rest,
                            });
                        } else if near(s) {
                            out.push(Extension::Arc {
                                c,
                                r,
                                a0,
                                sweep: -rest,
                            });
                        }
                    }
                }
            }
        }
        out
    }

    /// The direction of the straight edge nearest `p` within `tol` (Paralel's
    /// acquisition, docs/adr/0163 §2), its own way (a to b), unit.
    pub fn direction_at(&self, p: Vec2, tol: f64) -> Option<Vec2> {
        let reach = crate::geometry::Bounds {
            min_x: p.x - tol,
            min_y: p.y - tol,
            max_x: p.x + tol,
            max_y: p.y + tol,
        };
        let mut best: Option<(f64, Vec2)> = None;
        for it in self.near(p, tol) {
            for e in it.shapes() {
                for ed in entity_edges_in(e, &reach) {
                    let Edge::Seg { a, b } = ed else {
                        continue;
                    };
                    let c = closest_on_edge(&ed, p);
                    let l = js_hypot(b.x - a.x, b.y - a.y);
                    if c.d > tol || l <= 0.0 || best.is_some_and(|(d, _)| d <= c.d) {
                        continue;
                    }
                    best = Some((c.d, Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)));
                }
            }
        }
        best.map(|(_, u)| u)
    }
}

/// A point of a circle at an angle.
fn point_on(c: Vec2, r: f64, a: f64) -> Vec2 {
    Vec2::new(
        c.x + r * crate::jsmath::cos(a),
        c.y + r * crate::jsmath::sin(a),
    )
}

/// The grid coordinate nearest `v` at spacing `g`: a whole number of steps,
/// divided out when the spacing is a whole fraction of a metre (0.1, 0.5,
/// 0.25: the decimal coordinate's nearest double, not its product).
fn grid_coord(v: f64, g: f64) -> f64 {
    let per = js_round(1.0 / g);
    if per >= 1.0 && (1.0 / g - per).abs() < 1e-9 {
        js_round(v * per) / per
    } else {
        js_round(v / g) * g
    }
}

/// Snaps on the acquired extensions: the nearest point of each beyond its
/// end, and their crossings with the edges near the cursor and with each
/// other, weighed as crossings (docs/adr/0163 §1).
fn extension_snaps(ch: &mut Choice, nearby: &[Nearby], extensions: &[Extension]) {
    if extensions.is_empty() || ch.kinds & SnapKind::Extension.bit() == 0 {
        return;
    }
    let (p, tol) = (ch.p, ch.tol);
    let edges: Vec<Edge> = extensions
        .iter()
        .map(|x| match *x {
            Extension::Line { end, dir } => {
                let reach = js_hypot(p.x - end.x, p.y - end.y) + 2.0 * tol;
                Edge::Seg {
                    a: end,
                    b: Vec2::new(end.x + dir.x * reach, end.y + dir.y * reach),
                }
            }
            Extension::Arc { c, r, a0, sweep } => Edge::Arc { c, r, a0, sweep },
        })
        .collect();
    // Beyond the end by more than the aperture: nearer, the end itself is the
    // snap, so resting on it again releases it (docs/adr/0163 §2); behind the
    // end, the edge is the object's own.
    let beyond = |x: &Extension, q: Vec2| x.along(q).is_some_and(|t| t > tol);
    for (k, ed) in edges.iter().enumerate() {
        let x = &extensions[k];
        let c = closest_on_edge(ed, p);
        if beyond(x, c.p) {
            ch.consider(SnapKind::Extension, c.p, NO_OBJECT);
        }
        let crossing = SnapKind::Intersection.weight();
        for n in nearby {
            for h in intersect_edges(ed, &n.ed) {
                if beyond(x, h.p) {
                    ch.consider_as(SnapKind::Extension, h.p, NO_OBJECT, crossing);
                }
            }
        }
        for (j, other) in edges.iter().enumerate().skip(k + 1) {
            for h in intersect_edges(ed, other) {
                if beyond(x, h.p) && beyond(&extensions[j], h.p) {
                    ch.consider_as(SnapKind::Extension, h.p, NO_OBJECT, crossing);
                }
            }
        }
    }
}

/// Whether the box of a straight segment lies farther than `tol` from `p` along an axis, by more than
/// rounding can account for: then every point of the segment (its midpoint, its closest point to `p`,
/// both computed on it and within an ulp of its box) is farther than `tol` and is refused anyway.
/// Every comparison is false with a NaN, so such a segment is never left out here.
fn box_out_of_reach(a: Vec2, b: Vec2, p: Vec2, tol: f64) -> bool {
    let r = tol + tol * 1e-9 + 1e-6;
    (p.x < a.x - r && p.x < b.x - r)
        || (p.x > a.x + r && p.x > b.x + r)
        || (p.y < a.y - r && p.y < b.y - r)
        || (p.y > a.y + r && p.y > b.y + r)
}

/// Crossings of the edges near the cursor, pair by pair in their order
/// (at equal distance the first pair wins, as in the TypeScript). Close in
/// there are a handful; at the overview hundreds of edges lie within the
/// aperture (a contour map at 1:58 000: ~1 000 edges, ~500 000 pairs, which
/// took ~10 ms a pointer move) and nearly every pair is hopeless. So: two
/// plain segments cross, if at all, at the point `line_line` puts on the
/// first of them, never farther from the cursor than `crossing_gap` says;
/// when that cannot change the choice (`out_of_reach`, against the best so
/// far, which only improves) the first segment's pairs with plain segments
/// are skipped. Skipping a pair whose every crossing would be refused or
/// lose is a no-op, so the answer is the same bit for bit. Pairs with an
/// arc or an ellipse chord are always tried: `seg_arc` lets a NaN parameter
/// through (NaN arc data), and a chord's crossing is moved onto the ellipse.
fn crossings(ch: &mut Choice, nearby: &[Nearby], p: Vec2) {
    // A segment's crossing with an arc is also put on the segment (`seg_arc`), so the gap bounds it
    // as well; the arc was tried regardless only for a NaN crossing, which can win only while there
    // is no best yet. With one (the usual case: an end or a node in the aperture) far segments skip
    // arcs too: a parcel map's curved street fronts were otherwise met by every far edge.
    let arcs_count = ch.best.is_none();
    // The partners a far segment still has to meet, in order.
    let mut curved = Vec::new();
    for (k, e) in nearby.iter().enumerate() {
        if e.ell.is_some() || (arcs_count && e.segment().is_none()) {
            curved.push(k);
        }
    }
    let mut next = 0;
    for (i, a) in nearby.iter().enumerate() {
        while curved.get(next).is_some_and(|&k| k <= i) {
            next += 1;
        }
        let far = a
            .segment()
            .is_some_and(|(s, e)| ch.out_of_reach(crossing_gap(s, e, p)));
        // Every later edge, or only the curved ones.
        let (mut j, mut k) = (i + 1, next);
        loop {
            if far {
                let Some(&c) = curved.get(k) else { break };
                (j, k) = (c, k + 1);
            }
            let Some(b) = nearby.get(j) else { break };
            j += 1;
            if a.id == b.id && shares_vertex(&a.ed, &b.ed) {
                continue;
            }
            // Either object's layer must take crossings (docs/adr/0163 §4).
            if (a.mask | b.mask) & SnapKind::Intersection.bit() == 0 {
                continue;
            }
            // The crossing of two plain segments lies in the partner's box too, within its own margin
            // and the band on the first one: a partner out of reach is skipped the same way (the
            // overview of a parcel map has thousands of edges in the aperture, a few near the cursor).
            if let (Some((s, e)), Some((bs, be))) = (a.segment(), b.segment())
                && ch.out_of_reach(
                    crossing_gap(bs, be, p) - 1e-9 * ((e.x - s.x).abs() + (e.y - s.y).abs()),
                )
            {
                continue;
            }
            // An arc and a far segment after it: the crossing lies on the segment, as above.
            if !arcs_count
                && a.ell.is_none()
                && a.segment().is_none()
                && b.segment()
                    .is_some_and(|(bs, be)| ch.out_of_reach(crossing_gap(bs, be, p)))
            {
                continue;
            }
            for h in intersect_edges(&a.ed, &b.ed) {
                ch.consider(SnapKind::Intersection, refine_crossing(h.p, a, b), a.id);
            }
        }
    }
}

/// How near `p` a crossing on segment a→b can be, at least: `line_line`
/// puts it at a + t·(b − a) with t in [−1e-9, 1 + 1e-9] (NaN refused),
/// rounded, so it lies in the segment's box widened by `m`: 1e-9·|b − a|
/// for t's slack plus 1e-12·(|a| + |b|), far above the ~4ε·(|a| + |b|) the
/// rounding of b − a, t·(b − a) and the sum can add (ε = 2⁻⁵³). On the axis
/// where the box is farthest, P − (max + m) = g gives the crossing's
/// coordinate difference ≥ g after rounding too (rounding is monotonic),
/// and `js_hypot` is never below its larger argument. NaN or infinite ends
/// make `m`, and so every side's gap, NaN or −∞: no pruning.
fn crossing_gap(a: Vec2, b: Vec2, p: Vec2) -> f64 {
    let m = 1e-9 * ((b.x - a.x).abs() + (b.y - a.y).abs())
        + 1e-12 * (a.x.abs() + a.y.abs() + b.x.abs() + b.y.abs());
    let (lx, hx) = if a.x < b.x { (a.x, b.x) } else { (b.x, a.x) };
    let (ly, hy) = if a.y < b.y { (a.y, b.y) } else { (b.y, a.y) };
    let mut g = p.x - (hx + m);
    for side in [(lx - m) - p.x, p.y - (hy + m), (ly - m) - p.y] {
        if side > g {
            g = side;
        }
    }
    g
}

/// A crossing found on an ellipse's chords, moved onto the true curves:
/// exactly for a straight partner, by alternating projection otherwise.
fn refine_crossing(p: Vec2, a: &Nearby, b: &Nearby) -> Vec2 {
    if a.ell.is_none() && b.ell.is_none() {
        return p;
    }
    let (e, other) = if a.ell.is_some() { (a, b) } else { (b, a) };
    let Some(ell) = e.ell else { return p };
    if let (None, Edge::Seg { a: sa, b: sb }) = (other.ell, other.ed) {
        let mut best: Option<(Vec2, f64)> = None;
        for h in line_ellipse(&ell, sa, sb) {
            if h.u < -1e-9 || h.u > 1.0 + 1e-9 {
                continue;
            }
            // Taken on the straight partner, so a horizontal line keeps its exact Y.
            let q = Vec2::new(sa.x + (sb.x - sa.x) * h.u, sa.y + (sb.y - sa.y) * h.u);
            let d = js_hypot(q.x - p.x, q.y - p.y);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((q, d));
            }
        }
        return best.map_or(p, |(q, _)| q);
    }
    let mut q = p;
    for _ in 0..40 {
        q = ellipse_point(&ell, closest_param(&ell, q));
        q = match other.ell {
            Some(o) => ellipse_point(&o, closest_param(&o, q)),
            None => closest_on_edge(&other.ed, q).p,
        };
    }
    q
}

fn shares_vertex(a: &Edge, b: &Edge) -> bool {
    let (Edge::Seg { a: a1, b: a2 }, Edge::Seg { a: b1, b: b2 }) = (a, b) else {
        return false;
    };
    let eq = |p: &Vec2, q: &Vec2| (p.x - q.x).abs() < 1e-9 && (p.y - q.y).abs() < 1e-9;
    eq(a1, b1) || eq(a1, b2) || eq(a2, b1) || eq(a2, b2)
}

/// One shape's snap points near the cursor, and its edges near it for the
/// crossings, with the running choice's layer mask (`Choice::mask`).
fn shape_snaps(
    ch: &mut Choice,
    nearby: &mut Vec<Nearby>,
    e: &Shape,
    id: f64,
    from: Option<Vec2>,
    reach: &crate::geometry::Bounds,
) {
    let (p, tol) = (ch.p, ch.tol);
    match e {
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            ch.consider(SnapKind::Center, *c, id);
            for t in quadrant_params(&g) {
                ch.consider(SnapKind::Quadrant, ellipse_point(&g, t), id);
            }
            if !is_full_ellipse(&g) {
                ch.consider(SnapKind::Endpoint, ellipse_point(&g, *t0), id);
                ch.consider(SnapKind::Endpoint, ellipse_point(&g, *t1), id);
            }
            // Exact on the curve (its chords only serve crossings).
            ch.consider(
                SnapKind::Nearest,
                ellipse_point(&g, closest_param(&g, p)),
                id,
            );
            if let Some(f) = from {
                ch.consider(
                    SnapKind::Perpendicular,
                    ellipse_point(&g, closest_param(&g, f)),
                    id,
                );
                for t in ellipse_tangent_points(&g, f) {
                    ch.consider(SnapKind::Tangent, t, id);
                }
            }
            // Its chords only find crossings, which are then moved onto the curve
            // (`refine_crossing`): coarse ones do, and keep the pairs few.
            let chords = tessellate_ellipse(&g, 256.0);
            let n = chords.len();
            let count = if is_full_ellipse(&g) {
                n
            } else {
                n.saturating_sub(1)
            };
            for i in 0..count {
                let ed = Edge::Seg {
                    a: chords[i],
                    b: chords[(i + 1) % n],
                };
                if closest_on_edge(&ed, p).d <= tol {
                    nearby.push(Nearby {
                        id,
                        ed,
                        ell: Some(g),
                        mask: ch.mask,
                    });
                }
            }
            return;
        }
        // Every point of a multi-point object (docs/adr/0174).
        Shape::Point { .. } => {
            for part in area_parts(e).iter() {
                if let Shape::Point { p: q, .. } = part {
                    ch.consider(SnapKind::Node, *q, id);
                }
            }
            return;
        }
        Shape::Text { p: q, .. } | Shape::Insert { p: q, .. } => {
            ch.consider(SnapKind::Node, *q, id);
            return;
        }
        Shape::Circle { c, .. } => {
            ch.consider(SnapKind::Center, *c, id);
            for q in entity_vertices(e).into_iter().skip(1) {
                ch.consider(SnapKind::Quadrant, q, id);
            }
        }
        Shape::Arc { c, r, a0, a1 } => {
            let g = ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            };
            ch.consider(SnapKind::Center, *c, id);
            ch.consider(SnapKind::Endpoint, arc_start(&g), id);
            ch.consider(SnapKind::Endpoint, arc_end(&g), id);
            ch.consider(SnapKind::Midpoint, arc_mid(&g), id);
        }
        Shape::Spline { pts, closed } => {
            for (i, q) in pts.iter().enumerate() {
                let end = !closed && (i == 0 || i == pts.len() - 1);
                ch.consider(
                    if end {
                        SnapKind::Endpoint
                    } else {
                        SnapKind::Node
                    },
                    *q,
                    id,
                );
            }
        }
        // The new kinds where docs/adr/0147 §4 says: an ordinate's point
        // and its line's end, an arc length's ends and centre, a jogged
        // radius's point on the arc and its line's start, an azimuth's
        // and a slope's ends.
        Shape::Dimension {
            a,
            b,
            c,
            style: Some(style),
            ..
        } if is_new_kind(Some(style)) => match style.as_str() {
            "ordinate" => {
                ch.consider(SnapKind::Node, *a, id);
                ch.consider(SnapKind::Endpoint, *b, id);
            }
            "jogged" => {
                ch.consider(SnapKind::Node, *b, id);
                if let Some(c) = c {
                    ch.consider(SnapKind::Endpoint, *c, id);
                }
            }
            _ => {
                ch.consider(SnapKind::Node, *a, id);
                ch.consider(SnapKind::Node, *b, id);
                if let Some(c) = c {
                    ch.consider(SnapKind::Node, *c, id);
                }
            }
        },
        Shape::Dimension { a, b, c, .. } => {
            ch.consider(SnapKind::Node, *a, id);
            ch.consider(SnapKind::Node, *b, id);
            if let Some(c) = c {
                ch.consider(SnapKind::Node, *c, id);
            }
            if let Some(l) = dimension_geom(e).and_then(|g| layout_dimension(&g)) {
                ch.consider(SnapKind::Endpoint, l.d1, id);
                ch.consider(SnapKind::Endpoint, l.d2, id);
            }
        }
        // Its boundary is snapped through the outline object itself.
        Shape::Hatch { .. } => return,
        // The part shown's corners and its sides' middles (docs/adr/0192 §4).
        Shape::Image { .. } => {
            let ring = crate::geom::image::shown(e);
            for (i, &a) in ring.iter().enumerate() {
                let b = ring[(i + 1) % ring.len()];
                ch.consider(SnapKind::Endpoint, a, id);
                if !box_out_of_reach(a, b, p, tol) {
                    ch.consider(SnapKind::Midpoint, segment_mid(a, b, 0.0), id);
                }
            }
        }
        // Its corners and its lines' ends; its lines give nearest points and crossings below
        // (docs/adr/0184 §2).
        Shape::Table { .. } => {
            for q in entity_vertices(e) {
                ch.consider(SnapKind::Endpoint, q, id);
            }
            for [a, b] in crate::geom::table::lines_of(e) {
                if box_out_of_reach(a, b, p, tol) {
                    continue;
                }
                ch.consider(SnapKind::Endpoint, a, id);
                ch.consider(SnapKind::Endpoint, b, id);
            }
        }
        Shape::Line { .. }
        | Shape::Polyline { .. }
        | Shape::Polygon { .. }
        | Shape::Xline { .. }
        | Shape::Ray { .. }
        | Shape::Leader { .. } => {
            // A multi-part area part by part, each as one area (docs/adr/0143).
            for s in area_parts(e).iter() {
                let pts = entity_vertices(s);
                let bulges = match s {
                    Shape::Polyline { bulges, .. } | Shape::Polygon { bulges, .. } => {
                        bulges.as_deref()
                    }
                    _ => None,
                };
                for q in &pts {
                    ch.consider(SnapKind::Endpoint, *q, id);
                }
                // A leader's landing ends where its note begins (docs/adr/0146 §4).
                if let Some([_, end]) = leader::layout_of(s).and_then(|l| l.landing) {
                    ch.consider(SnapKind::Endpoint, end, id);
                }
                // A polygon's vertices include its holes', as the TypeScript walked them.
                let n = if matches!(s, Shape::Polygon { .. }) {
                    pts.len()
                } else {
                    pts.len().saturating_sub(1)
                };
                for i in 0..n {
                    let a = pts[i];
                    let b = pts[(i + 1) % pts.len()];
                    let bulge = bulge_at(bulges, i);
                    // A straight segment's midpoint lies in its box; a far box cannot offer one.
                    if bulge == 0.0 && box_out_of_reach(a, b, p, tol) {
                        continue;
                    }
                    ch.consider(SnapKind::Midpoint, segment_mid(a, b, bulge), id);
                    if let Some(arc) = bulge_arc(a, b, bulge) {
                        ch.consider(SnapKind::Center, arc.c, id);
                    }
                }
            }
        }
    }
    for ed in entity_edges_in(e, reach) {
        // Out at the overview a contour of hundreds of segments crosses the aperture with a few:
        // the rest are left out by their box before the closest point is worked out.
        if let Edge::Seg { a, b } = ed
            && box_out_of_reach(a, b, p, tol)
        {
            continue;
        }
        let c = closest_on_edge(&ed, p);
        if c.d > tol {
            continue;
        }
        nearby.push(Nearby {
            id,
            ed,
            ell: None,
            mask: ch.mask,
        });
        ch.consider(SnapKind::Nearest, c.p, id);
        if let Some(f) = from {
            if let Some(foot) = perpendicular_foot(&ed, f) {
                ch.consider(SnapKind::Perpendicular, foot, id);
            }
            if let Edge::Arc { c, r, a0, sweep } = ed {
                for t in tangent_points(f, c, r) {
                    if on_edge_arc(a0, sweep, atan2(t.y - c.y, t.x - c.x)) {
                        ch.consider(SnapKind::Tangent, t, id);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::arrangement::Ring;
    use crate::geom::intersect::seg_seg;
    use crate::jsmath::{PI, TAU, cos, exp, log, sin};
    use std::cell::Cell;

    /// Every pair, as before the pruning: the reference for `crossings`.
    fn every_pair(ch: &mut Choice, nearby: &[Nearby], _: Vec2) {
        for (i, a) in nearby.iter().enumerate() {
            for b in &nearby[i + 1..] {
                if a.id == b.id && shares_vertex(&a.ed, &b.ed) {
                    continue;
                }
                if (a.mask | b.mask) & SnapKind::Intersection.bit() == 0 {
                    continue;
                }
                for h in intersect_edges(&a.ed, &b.ed) {
                    ch.consider(SnapKind::Intersection, refine_crossing(h.p, a, b), a.id);
                }
            }
        }
    }

    /// Kind, point and id bit for bit (NaN included).
    fn bits(h: Option<SnapHit>) -> Option<(SnapKind, u64, u64, u64)> {
        h.map(|h| {
            (
                h.kind,
                h.point.x.to_bits(),
                h.point.y.to_bits(),
                h.id.to_bits(),
            )
        })
    }

    /// splitmix64, as numbers in [0, 1).
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
        }

        fn range(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * self.next()
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() * n as f64) as usize
        }
    }

    fn add(s: &mut Store, shape: Shape) {
        let id = (s.len() + 1) as f64;
        s.put(id, "a", false, shape);
    }

    fn line(a: Vec2, b: Vec2) -> Shape {
        Shape::Line { a, b }
    }

    /// Many edges within reach: families of long crossing lines, contour-like
    /// polylines, parcels sharing their corners (crossings that tie), curves,
    /// construction lines, overlapping and zero-length segments, NaN and
    /// infinite data. Returns the vertices, where cursors tie at distance 0.
    fn scene(rng: &mut Rng, o: Vec2) -> (Store, Vec<Vec2>) {
        let mut s = Store::new();
        let mut corners = Vec::new();
        let v = |x: f64, y: f64| Vec2::new(o.x + x, o.y + y);
        for _ in 0..1 + rng.below(3) {
            let ang = if rng.next() < 0.5 {
                rng.below(4) as f64 * PI / 4.0
            } else {
                rng.range(0.0, PI)
            };
            let (dx, dy) = (cos(ang), sin(ang));
            let spacing = rng.range(0.4, 5.0);
            let count = 2 + rng.below(16);
            let off = rng.range(-8.0, 8.0);
            for k in 0..count {
                let c = off + (k as f64 - count as f64 / 2.0) * spacing;
                let along = rng.range(-5.0, 5.0);
                let half = rng.range(15.0, 50.0);
                let (cx, cy) = (-dy * c + dx * along, dx * c + dy * along);
                let (a, b) = (
                    v(cx - dx * half, cy - dy * half),
                    v(cx + dx * half, cy + dy * half),
                );
                corners.push(a);
                add(&mut s, line(a, b));
            }
        }
        if rng.next() < 0.6 {
            let spacing = rng.range(0.3, 4.0);
            for k in 0..2 + rng.below(8) {
                let n = 6 + rng.below(18);
                let pts: Vec<Vec2> = (0..=n)
                    .map(|j| {
                        let u = -40.0 + 80.0 * j as f64 / n as f64 + rng.range(-1.0, 1.0);
                        let y = k as f64 * spacing + 3.0 * sin(u / 9.0 + k as f64) - 10.0;
                        v(u, y + rng.range(-0.2, 0.2))
                    })
                    .collect();
                corners.extend(&pts);
                add(
                    &mut s,
                    Shape::Polyline {
                        pts,
                        bulges: None,
                        holes: None,
                        parts: None,
                    },
                );
            }
        }
        if rng.next() < 0.5 {
            let (nx, ny) = (1 + rng.below(4), 1 + rng.below(4));
            let (w, h) = (rng.range(2.0, 12.0), rng.range(2.0, 12.0));
            let (x0, y0) = (rng.range(-30.0, 5.0), rng.range(-30.0, 5.0));
            let corner = |i: usize, j: usize| v(x0 + i as f64 * w, y0 + j as f64 * h);
            for i in 0..nx {
                for j in 0..ny {
                    let pts = vec![
                        corner(i, j),
                        corner(i + 1, j),
                        corner(i + 1, j + 1),
                        corner(i, j + 1),
                    ];
                    corners.extend(&pts);
                    let bulges =
                        (rng.next() < 0.2).then(|| vec![0.0, rng.range(-0.4, 0.4), 0.0, 0.0]);
                    add(
                        &mut s,
                        Shape::Polygon {
                            pts,
                            bulges,
                            holes: None,
                            parts: None,
                        },
                    );
                }
            }
        }
        for _ in 0..rng.below(10) {
            let c = v(rng.range(-35.0, 35.0), rng.range(-35.0, 35.0));
            let near = |rng: &mut Rng| {
                Vec2::new(c.x + rng.range(-12.0, 12.0), c.y + rng.range(-12.0, 12.0))
            };
            let unit = |a: f64| Vec2::new(cos(a), sin(a));
            let shape = match rng.below(12) {
                0 => Shape::Circle {
                    c,
                    r: rng.range(0.5, 15.0),
                },
                1 => Shape::Arc {
                    c,
                    r: rng.range(0.5, 15.0),
                    a0: rng.range(0.0, TAU),
                    a1: rng.range(0.0, TAU),
                },
                2 => {
                    let t0 = if rng.next() < 0.5 {
                        0.0
                    } else {
                        rng.range(0.0, TAU)
                    };
                    Shape::Ellipse {
                        c,
                        major: Vec2::new(rng.range(-12.0, 12.0), rng.range(-12.0, 12.0)),
                        ratio: rng.range(0.2, 1.0),
                        t0,
                        t1: if t0 == 0.0 { 0.0 } else { rng.range(0.0, TAU) },
                    }
                }
                3 => {
                    let n = 2 + rng.below(4);
                    Shape::Polyline {
                        pts: (0..n).map(|_| near(rng)).collect(),
                        bulges: Some(
                            (0..n)
                                .map(|_| {
                                    if rng.next() < 0.3 {
                                        0.0
                                    } else {
                                        rng.range(-1.0, 1.0)
                                    }
                                })
                                .collect(),
                        ),
                        holes: None,
                        parts: None,
                    }
                }
                4 => Shape::Xline {
                    p: c,
                    dir: unit(rng.range(0.0, TAU)),
                },
                5 => Shape::Ray {
                    p: c,
                    dir: unit(rng.range(0.0, TAU)),
                },
                6 => Shape::Spline {
                    pts: (0..3 + rng.below(3)).map(|_| near(rng)).collect(),
                    closed: rng.next() < 0.3,
                },
                7 => Shape::Point {
                    p: c,
                    z: None,
                    parts: None,
                },
                8 => line(c, c),
                9 => {
                    // Collinear and overlapping, and the same segment twice.
                    let b = near(rng);
                    add(&mut s, line(c, b));
                    add(&mut s, line(c, b));
                    line(
                        Vec2::new((c.x + b.x) / 2.0, (c.y + b.y) / 2.0),
                        Vec2::new(2.0 * b.x - c.x, 2.0 * b.y - c.y),
                    )
                }
                10 => match rng.below(5) {
                    0 => Shape::Circle { c, r: f64::NAN },
                    1 => Shape::Circle {
                        c: Vec2::new(f64::NAN, c.y),
                        r: 3.0,
                    },
                    2 => line(c, Vec2::new(f64::NAN, c.y)),
                    3 => line(c, Vec2::new(f64::INFINITY, c.y)),
                    _ => Shape::Arc {
                        c,
                        r: 4.0,
                        a0: f64::NAN,
                        a1: 1.0,
                    },
                },
                _ => Shape::Polygon {
                    pts: (0..4).map(|_| near(rng)).collect(),
                    bulges: None,
                    holes: Some(vec![Ring {
                        pts: (0..3).map(|_| near(rng)).collect(),
                        bulges: None,
                    }]),
                    parts: None,
                },
            };
            add(&mut s, shape);
        }
        if rng.next() < 0.7 {
            s.rebuild();
        }
        (s, corners)
    }

    #[test]
    fn skipped_crossings_never_change_the_snap() {
        let mut rng = Rng(0x5eed_0001);
        let (mut pruned, mut crossed, mut both, mut queries) = (0, 0, 0, 0);
        for round in 0..96 {
            let o = if round % 2 == 0 {
                Vec2::new(0.0, 0.0)
            } else {
                Vec2::new(486_000.0, 4_420_000.0)
            };
            let (s, corners) = scene(&mut rng, o);
            for _ in 0..50 {
                let p = if rng.next() < 0.25 && !corners.is_empty() {
                    corners[rng.below(corners.len())]
                } else {
                    Vec2::new(o.x + rng.range(-40.0, 40.0), o.y + rng.range(-40.0, 40.0))
                };
                let tol = match rng.below(10) {
                    0 => 0.0,
                    1 => rng.range(0.001, 0.3),
                    _ => exp(rng.range(log(0.3), log(40.0))),
                };
                let kinds = match rng.below(6) {
                    0 => 0xff,
                    1 => 0x1ff,
                    2 => SnapKind::Intersection.bit(),
                    3 => SnapKind::Intersection.bit() | SnapKind::Endpoint.bit(),
                    4 => SnapKind::Intersection.bit() | SnapKind::Nearest.bit(),
                    _ => SnapKind::Intersection.bit() | rng.below(512) as u32,
                };
                let from = (rng.next() < 0.6)
                    .then(|| Vec2::new(o.x + rng.range(-60.0, 60.0), o.y + rng.range(-60.0, 60.0)));
                let skippable = Cell::new(0);
                let want = s.snap_with(
                    p,
                    tol,
                    kinds,
                    from,
                    &SnapExtras::default(),
                    |ch, nearby, p| {
                        let far = nearby.iter().filter(|a| {
                            a.segment()
                                .is_some_and(|(u, w)| ch.out_of_reach(crossing_gap(u, w, p)))
                        });
                        skippable.set(far.count());
                        every_pair(ch, nearby, p);
                    },
                );
                let got = s.snap(p, tol, kinds, from);
                assert_eq!(
                    bits(got),
                    bits(want),
                    "round {round}: {p:?}, tol {tol}, kinds {kinds:#x}, from {from:?}: {got:?} ≠ {want:?}"
                );
                let crossing = want.is_some_and(|h| h.kind == SnapKind::Intersection);
                pruned += skippable.get();
                crossed += usize::from(crossing);
                both += usize::from(crossing && skippable.get() > 0);
                queries += 1;
            }
        }
        // The rule is exercised: rows skipped, answers that are crossings nonetheless.
        assert!(
            pruned > 3 * queries,
            "{pruned} rows skipped in {queries} queries"
        );
        assert!(
            both > queries / 50,
            "{both} of {crossed} crossings with rows skipped"
        );
    }

    #[test]
    fn finds_the_crossing_among_many_parallel_and_crossing_lines() {
        // 61 horizontal and 61 vertical lines a metre apart at TM coordinates:
        // with a 25 m aperture about a hundred are in reach, ~5 000 pairs.
        let o = Vec2::new(486_000.0, 4_420_000.0);
        let mut s = Store::new();
        for k in -30..=30 {
            let k = f64::from(k);
            add(
                &mut s,
                line(
                    Vec2::new(o.x - 60.0, o.y + k),
                    Vec2::new(o.x + 60.0, o.y + k),
                ),
            );
        }
        for k in -30..=30 {
            let k = f64::from(k);
            add(
                &mut s,
                line(
                    Vec2::new(o.x + k, o.y - 60.0),
                    Vec2::new(o.x + k, o.y + 60.0),
                ),
            );
        }
        s.rebuild();
        let p = Vec2::new(o.x + 0.3, o.y + 0.4);
        for (kinds, from) in [
            (0xff, None),
            (SnapKind::Intersection.bit(), None),
            (0x1ff, Some(Vec2::new(o.x - 7.5, o.y + 3.25))),
        ] {
            let hit = s.snap(p, 25.0, kinds, from).unwrap();
            assert_eq!(hit.kind, SnapKind::Intersection);
            assert!(
                js_hypot(hit.point.x - o.x, hit.point.y - o.y) < 1e-9,
                "{hit:?}"
            );
            // The first pair through that crossing: horizontal line y = 0 (id 31) and x = 0.
            assert_eq!(hit.id, 31.0);
            assert_eq!(
                bits(Some(hit)),
                bits(s.snap_with(p, 25.0, kinds, from, &SnapExtras::default(), every_pair))
            );
        }
        // Parcels' shared corners: crossings that tie at distance 0 go to the first pair.
        let mut t = Store::new();
        for (x, y) in [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)] {
            add(
                &mut t,
                Shape::Polygon {
                    pts: vec![
                        Vec2::new(o.x + x, o.y + y),
                        Vec2::new(o.x + x + 10.0, o.y + y),
                        Vec2::new(o.x + x + 10.0, o.y + y + 10.0),
                        Vec2::new(o.x + x, o.y + y + 10.0),
                    ],
                    bulges: None,
                    holes: None,
                    parts: None,
                },
            );
        }
        let corner = Vec2::new(o.x + 10.0, o.y + 10.0);
        let kinds = SnapKind::Intersection.bit();
        let hit = t.snap(corner, 30.0, kinds, None);
        assert_eq!(
            bits(hit),
            bits(t.snap_with(
                corner,
                30.0,
                kinds,
                None,
                &SnapExtras::default(),
                every_pair
            ))
        );
        assert_eq!(
            hit.map(|h| (h.kind, h.point, h.id)),
            Some((SnapKind::Intersection, corner, 1.0))
        );
    }

    #[test]
    fn no_crossing_lies_nearer_than_the_gap() {
        // The bound on real crossings: random segments at every scale up to TM
        // coordinates, crossed by random partners (and at t's extreme slack).
        let mut rng = Rng(0x5eed_0002);
        let mut met = 0;
        for i in 0..20_000 {
            let scale = [1e-3, 1.0, 1e3, 4.4e6][i % 4];
            let at = |rng: &mut Rng| {
                Vec2::new(
                    scale * rng.range(0.5, 1.0) + rng.range(-50.0, 50.0),
                    scale * rng.range(0.5, 1.0) + rng.range(-50.0, 50.0),
                )
            };
            let (a, b, c, d, p) = (
                at(&mut rng),
                at(&mut rng),
                at(&mut rng),
                at(&mut rng),
                at(&mut rng),
            );
            let g = crossing_gap(a, b, p);
            let check = |q: Vec2| {
                let dq = js_hypot(q.x - p.x, q.y - p.y);
                assert!(dq >= g, "{a:?}→{b:?}, {p:?}: {q:?} at {dq} < {g}");
            };
            // The partner's own gap, less the band on the first segment (the pair skip in `crossings`).
            let partner = |c: Vec2, d: Vec2, q: Vec2| {
                let gc = crossing_gap(c, d, p) - 1e-9 * ((b.x - a.x).abs() + (b.y - a.y).abs());
                let dq = js_hypot(q.x - p.x, q.y - p.y);
                assert!(
                    dq >= gc,
                    "{c:?}→{d:?} with {a:?}→{b:?}, {p:?}: {q:?} at {dq} < {gc}"
                );
            };
            if let Some(h) = seg_seg(a, b, c, d, 1e-9) {
                check(h.p);
                partner(c, d, h.p);
                met += 1;
            }
            // A nearly parallel partner through a point of the first segment, where the crossing is least well placed.
            let m = Vec2::new(a.x + 0.5 * (b.x - a.x), a.y + 0.5 * (b.y - a.y));
            let tilt = rng.range(-1e-7, 1e-7);
            let (u, v) = (b.x - a.x, b.y - a.y);
            let (c2, d2) = (
                Vec2::new(m.x - 0.3 * (u - tilt * v), m.y - 0.3 * (v + tilt * u)),
                Vec2::new(m.x + 0.4 * (u - tilt * v), m.y + 0.4 * (v + tilt * u)),
            );
            if let Some(h) = seg_seg(a, b, c2, d2, 1e-9) {
                check(h.p);
                partner(c2, d2, h.p);
                met += 1;
            }
            for t in [-1e-9, 0.0, 0.5, 1.0, 1.0 + 1e-9] {
                check(Vec2::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)));
            }
        }
        assert!(met > 10_000, "{met} crossings");
        // Non-finite ends never prune.
        let p = Vec2::new(0.0, 0.0);
        for b in [Vec2::new(f64::NAN, 1.0), Vec2::new(f64::INFINITY, 1.0)] {
            let g = crossing_gap(Vec2::new(5.0, 5.0), b, p);
            assert!(!(g >= 0.0), "{g}");
        }
    }
}

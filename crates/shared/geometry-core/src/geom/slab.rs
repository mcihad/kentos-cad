//! Slabs (döşeme, docs/adr/0722 §2): a slab's outline found from the walls, moved by an edge offset, the free sides its
//! edge (a railing or a parapet) runs along, and its pieces.
//!
//! - From the walls: their faces' line work (`space::wall_faces`); `outline` is the outer boundary of the piece of it
//!   that holds the point (the building's outline to the walls' outer faces, every room and wall inside it), `room` the
//!   room round the point to the walls' inner faces (a space's net face, docs/adr/0342 §2).
//! - Edge offset: every edge of every ring moved by the same distance, out of the slab when positive (the outer ring
//!   counter-clockwise and each hole clockwise, every edge to its right); consecutive moved edges meet where their
//!   lines cross (`space::moved_ring`). It fails when a ring turns over or crosses itself, a hole crosses another ring
//!   or leaves the outer ring, or an edge turns round.
//! - Free sides: an edge of a ring is free when its middle lies on no wall's face (within `space::ON_FACE`, the faces
//!   `space::wall_faces` gives); consecutive free edges of a ring make one run.
//! - Pieces: its rings, and along each run the edge: a railing's two lines `width` apart, a parapet's band (its two
//!   lines and its ends), on the slab's side of the outline.

use crate::entity::{Entity, Shape};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::bulge::segment_mid;
use crate::geom::intersect::closest_on_edge;
use crate::geom::region::{FaceIndex, orient_ring};
use crate::geom::space;
use crate::geom::wall_join::WallIn;
use crate::vec2::Vec2;

/// A run of a slab's free sides: an open polyline, one bulge per edge when an edge is an arc. A ring free all round is
/// one run that ends where it starts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
}

impl Run {
    /// Whether it goes all round a ring: it ends where it starts.
    pub fn closed(&self) -> bool {
        self.pts.len() > 2 && self.pts.first() == self.pts.last()
    }
}

/// How a slab is drawn beside its rings: no label; its edge, when it has one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    pub edge: Option<Edge>,
}

/// A slab's edge as drawn: a parapet or a railing, its width, its runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edge {
    pub parapet: bool,
    pub width: f64,
    pub runs: Vec<Run>,
}

/// Why an edge offset cannot be made: the part, the hole (none: the outer ring) and the words.
#[derive(Clone, Debug, PartialEq)]
pub struct OffsetFailure {
    pub part: usize,
    pub hole: Option<usize>,
    pub words: String,
}

fn line_work(walls: &[WallIn<'_>]) -> FaceIndex {
    let faces = space::wall_faces(walls);
    let lines: Vec<Entity> = faces.iter().map(|(s, _)| Entity::new(s.clone())).collect();
    FaceIndex::new(&[crate::ops::areas::line_source(&lines)])
}

/// The outline of the walls round `seed`: the outer boundary of their line work's piece that holds it, counter-
/// clockwise, without holes; none outside every piece.
pub fn outline(walls: &[WallIn<'_>], seed: Vec2) -> Option<Area> {
    let ring = line_work(walls).outline_at(seed)?;
    Some(Area {
        outer: orient_ring(&ring, true),
        holes: Vec::new(),
    })
}

/// The room round `seed` to the walls' inner faces, with its holes; none when they close nothing round it.
pub fn room(walls: &[WallIn<'_>], seed: Vec2) -> Option<Area> {
    space::found(walls, seed, space::Method::Net)
}

fn signed(r: &[Vec2]) -> f64 {
    let n = r.len();
    let mut s = 0.0;
    for i in 0..n {
        let (a, b) = (r[i], r[(i + 1) % n]);
        s += a.x * b.y - b.x * a.y;
    }
    s / 2.0
}

/// The ring's outline, its arcs as chords (what the crossing tests read).
fn chords(r: &Ring) -> Vec<Vec2> {
    crate::entity::polygon_ring(&r.pts, r.bulges.as_deref())
}

/// Whether two edges of one ring cross, but neighbours (and the last with the first).
fn self_crossing(pts: &[Vec2]) -> bool {
    let n = pts.len();
    for i in 0..n {
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let (a, b, c, d) = (pts[i], pts[(i + 1) % n], pts[j], pts[(j + 1) % n]);
            if crate::geom::intersect::seg_seg(a, b, c, d, 1e-12).is_some() {
                return true;
            }
        }
    }
    false
}

/// Whether any edge of one ring crosses or touches any of the other's.
fn rings_meet(a: &[Vec2], b: &[Vec2]) -> bool {
    let (n, m) = (a.len(), b.len());
    for i in 0..n {
        for j in 0..m {
            if crate::geom::intersect::seg_seg(a[i], a[(i + 1) % n], b[j], b[(j + 1) % m], 1e-12).is_some() {
                return true;
            }
        }
    }
    false
}

/// Why the moved ring is no ring: it turned over, went, an edge turned round, or two of its edges cross.
fn ring_failure(before: &Ring, after: &Ring, hole: bool) -> Option<&'static str> {
    let (b, a) = (chords(before), chords(after));
    let (sb, sa) = (signed(&b), signed(&a));
    if !(sa.abs() > 1e-12) || (sb > 0.0) != (sa > 0.0) {
        return Some(if hole {
            "Kenar ötelemesi deliği kapatıyor ya da ters çeviriyor."
        } else {
            "Kenar ötelemesi döşemeyi kapatıyor ya da ters çeviriyor."
        });
    }
    if before.pts.len() == after.pts.len() && before.bulges.is_none() && after.bulges.is_none() {
        let n = before.pts.len();
        for i in 0..n {
            let (p0, p1) = (before.pts[i], before.pts[(i + 1) % n]);
            let (q0, q1) = (after.pts[i], after.pts[(i + 1) % n]);
            let dot = (p1.x - p0.x) * (q1.x - q0.x) + (p1.y - p0.y) * (q1.y - q0.y);
            if !(dot > 0.0) {
                return Some("Kenar ötelemesi bir kenarı yok ediyor ya da ters çeviriyor; daha küçük bir uzaklık verin.");
            }
        }
    }
    if self_crossing(&a) {
        return Some("Kenar ötelemesi halkanın kenarlarını kesiştiriyor; daha küçük bir uzaklık verin.");
    }
    None
}

/// `parts` with every edge moved by `d`, out of the slab when positive; why it cannot be.
pub fn offset(parts: &[Area], d: f64) -> Result<Vec<Area>, OffsetFailure> {
    if d == 0.0 {
        return Ok(parts.to_vec());
    }
    let mut out = Vec::with_capacity(parts.len());
    for (k, part) in parts.iter().enumerate() {
        let outer = orient_ring(&part.outer, true);
        let moved_outer = space::moved_ring(&outer, &vec![d; outer.pts.len()]);
        if let Some(words) = ring_failure(&outer, &moved_outer, false) {
            return Err(OffsetFailure { part: k, hole: None, words: words.into() });
        }
        let outer_chords = chords(&moved_outer);
        let mut holes: Vec<Ring> = Vec::with_capacity(part.holes.len());
        let mut hole_chords: Vec<Vec<Vec2>> = Vec::new();
        for (h, hole) in part.holes.iter().enumerate() {
            let hole = orient_ring(hole, false);
            let moved = space::moved_ring(&hole, &vec![d; hole.pts.len()]);
            if let Some(words) = ring_failure(&hole, &moved, true) {
                return Err(OffsetFailure { part: k, hole: Some(h), words: words.into() });
            }
            let c = chords(&moved);
            let fails = rings_meet(&c, &outer_chords)
                || !crate::geometry::point_in_polygon(c[0], &outer_chords)
                || hole_chords.iter().any(|o| rings_meet(&c, o));
            if fails {
                return Err(OffsetFailure {
                    part: k,
                    hole: Some(h),
                    words: "Kenar ötelemesi deliği dış kenara ya da başka bir deliğe taşıyor.".into(),
                });
            }
            hole_chords.push(c);
            holes.push(moved);
        }
        out.push(Area { outer: moved_outer, holes });
    }
    Ok(out)
}

/// The runs of `parts`' rings no wall's face runs along (`faces`: `space::wall_faces`), the outer ring counter-
/// clockwise and the holes clockwise, so that the slab lies to the left of every run.
pub fn free_runs(parts: &[Area], faces: &[(Shape, f64)]) -> Vec<Run> {
    let edges: Vec<Vec<crate::geom::intersect::Edge>> = faces
        .iter()
        .filter(|(_, w)| *w > 0.0)
        .map(|(s, _)| crate::ops::edges::entity_edges(s))
        .collect();
    let mut out = Vec::new();
    for part in parts {
        let rings = std::iter::once(orient_ring(&part.outer, true))
            .chain(part.holes.iter().map(|h| orient_ring(h, false)));
        for r in rings {
            let n = r.pts.len();
            let bulge = |i: usize| r.bulges.as_ref().map_or(0.0, |b| b[i]);
            let free: Vec<bool> = (0..n)
                .map(|i| {
                    let m = segment_mid(r.pts[i], r.pts[(i + 1) % n], bulge(i));
                    !edges
                        .iter()
                        .any(|es| es.iter().any(|e| closest_on_edge(e, m).d <= space::ON_FACE))
                })
                .collect();
            if free.iter().all(|f| *f) {
                let mut pts = r.pts.clone();
                pts.push(r.pts[0]);
                out.push(Run {
                    pts,
                    bulges: r.bulges.clone(),
                });
                continue;
            }
            // Start just after a covered edge, so a run never wraps across the ring's first vertex unseen.
            let Some(first) = (0..n).find(|&i| !free[i]) else {
                continue;
            };
            let mut run: Option<Run> = None;
            for step in 1..=n {
                let i = (first + step) % n;
                if free[i] {
                    let r0 = run.get_or_insert_with(|| Run {
                        pts: vec![r.pts[i]],
                        bulges: Some(Vec::new()),
                    });
                    r0.pts.push(r.pts[(i + 1) % n]);
                    if let Some(b) = &mut r0.bulges {
                        b.push(bulge(i));
                    }
                } else if let Some(done) = run.take() {
                    out.push(tidy(done));
                }
            }
            if let Some(done) = run.take() {
                out.push(tidy(done));
            }
        }
    }
    out
}

/// A run without bulges when every edge of it is straight.
fn tidy(mut r: Run) -> Run {
    if r.bulges.as_ref().is_some_and(|b| b.iter().all(|x| *x == 0.0)) {
        r.bulges = None;
    }
    r
}

fn polyline(pts: Vec<Vec2>, bulges: Option<Vec<f64>>, closed: bool) -> Shape {
    Shape::Polyline {
        pts,
        bulges,
        holes: None,
        parts: None,
        closed,
        widths: None,
    }
}

/// The line `width` to the left of a run (into the slab): its points and bulges.
fn inner(run: &Run, width: f64) -> Option<(Vec<Vec2>, Option<Vec<f64>>)> {
    let closed = run.closed();
    let pts: Vec<Vec2> = if closed { run.pts[..run.pts.len() - 1].to_vec() } else { run.pts.clone() };
    match &run.bulges {
        Some(b) if b.iter().any(|x| *x != 0.0) => {
            let bs: Vec<f64> = if closed { b.clone() } else { b.iter().copied().chain(std::iter::once(0.0)).collect() };
            match crate::geom::offset::offset_bulge_path(&pts, &bs, width, closed) {
                crate::geom::offset::OffsetResult::Path { pts, bulges } => Some((pts, Some(bulges))),
                crate::geom::offset::OffsetResult::Error { .. } => None,
            }
        }
        _ => {
            let out = crate::geom::offset::offset_path(&pts, width, closed);
            (out.len() >= 2).then_some((out, None))
        }
    }
}

/// The edge's pieces along its runs: a railing's two lines, a parapet's two lines and its ends.
pub fn edge_pieces(edge: &Edge) -> Vec<Shape> {
    let mut out = Vec::new();
    for run in &edge.runs {
        let closed = run.closed();
        let (pts, bulges) = if closed {
            let n = run.pts.len() - 1;
            (run.pts[..n].to_vec(), run.bulges.clone())
        } else {
            (
                run.pts.clone(),
                run.bulges.as_ref().map(|b| b.iter().copied().chain(std::iter::once(0.0)).collect()),
            )
        };
        out.push(polyline(pts.clone(), bulges, closed));
        let Some((ipts, ibulges)) = inner(run, edge.width) else {
            continue;
        };
        if edge.parapet && !closed {
            let (Some(a0), Some(a1), Some(b0), Some(b1)) = (pts.first(), pts.last(), ipts.first(), ipts.last()) else {
                continue;
            };
            out.push(Shape::Line { a: *a0, b: *b0 });
            out.push(Shape::Line { a: *a1, b: *b1 });
        }
        out.push(polyline(ipts, ibulges, closed));
    }
    out
}

/// A slab's pieces: its rings, then its edge's.
pub fn pieces(parts: &[Area], look: &Look) -> Vec<Shape> {
    let mut out: Vec<Shape> = parts
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
        .map(|r| polyline(r.pts.clone(), r.bulges.clone(), true))
        .collect();
    if let Some(e) = &look.edge {
        out.extend(edge_pieces(e));
    }
    out
}

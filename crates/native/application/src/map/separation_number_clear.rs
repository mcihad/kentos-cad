//! docs/adr/5080: a separation number written outside its part on a leader (docs/adr/4680) kept clear of what the
//! sheet's map shows. The places are tried nearest first: the leader's line leaves the part from its label place
//! in a direction and runs one, two, then three text heights past the part's boundary; the directions east, west,
//! north, south, then the four between. A place is clear when its line outside the part, its landing and its note
//! touch none of the obstacles: the drawing's lines and areas' boundaries (not their insides), texts, leaders (their
//! lines and notes) and points on the layers shown, and this sheet's other numbers. None clear: the place touching the
//! fewest obstacles, the first among equals (as a label engine that cannot avoid every barrier moves its label where it
//! crosses the fewest), said in the label.

use std::collections::BTreeSet;

use kentos_commands::ScopePart;
use kentos_domain::Document;
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{Shape, TextPlace, entity_bounds_in};
use kentos_geometry_core::geom::intersect::{Edge, intersect_edges};
use kentos_geometry_core::geom::leader;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::text::Font;

use crate::readjustment_sheets::crossings_at;

/// The directions a leader leaves its part in, in the order they are tried: east (docs/adr/4680's), west, north,
/// south, then north-east, north-west, south-east, south-west.
const DIRECTIONS: [[f64; 2]; 8] = [
    [1.0, 0.0],
    [-1.0, 0.0],
    [0.0, 1.0],
    [0.0, -1.0],
    [std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2],
    [-std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2],
    [std::f64::consts::FRAC_1_SQRT_2, -std::f64::consts::FRAC_1_SQRT_2],
    [-std::f64::consts::FRAC_1_SQRT_2, -std::f64::consts::FRAC_1_SQRT_2],
];
/// How far past the part's boundary the line runs, in text heights, nearest first.
const REACH: [f64; 3] = [1.0, 2.0, 3.0];
/// The farthest a place's line, landing and note reach from the part, in text heights beside the note's width: the
/// reach, the landing (2) and the gap before the note (0.5) a leader lays out (docs/adr/0146 §2), and one more.
const SPAN: f64 = 3.0 + 2.0 + 0.5 + 1.0;
/// A hit this near the start of the line's stretch outside the part, a share of its length, is where it leaves the
/// part (a boundary there, its own or a neighbour's on it, is crossed by any place).
const LEAVING: f64 = 1e-9;

fn v(p: [f64; 2]) -> Vec2 {
    Vec2::new(p[0], p[1])
}

/// What a number outside must not touch, each with its owner: an object's persistent id or a number's part name.
#[derive(Default)]
pub struct Obstacles {
    edges: Vec<(Edge, String)>,
    boxes: Vec<(Vec<Vec2>, String)>,
    points: Vec<(Vec2, String)>,
}

/// The box a place's line, landing and note may reach around a part (its outer ring's corners and arcs' ends widened by
/// `SPAN` heights and the note's width): no object outside it can touch any place.
pub fn reach_of(parts: &[&ScopePart], height: f64, width: f64) -> Bounds {
    let mut b = Bounds { min_x: f64::INFINITY, min_y: f64::INFINITY, max_x: f64::NEG_INFINITY, max_y: f64::NEG_INFINITY };
    for part in parts {
        for r in &part.rings {
            for p in &r.pts {
                b.min_x = b.min_x.min(p[0]);
                b.min_y = b.min_y.min(p[1]);
                b.max_x = b.max_x.max(p[0]);
                b.max_y = b.max_y.max(p[1]);
            }
            // An arc bulges at most its chord's half-length times its bulge's size past its chord: widen by the
            // longest chord to be sure.
            if r.bulges.as_ref().is_some_and(|b| b.iter().any(|x| *x != 0.0)) {
                let n = r.pts.len();
                let most = (0..n)
                    .map(|i| {
                        let (a, c) = (r.pts[i], r.pts[(i + 1) % n]);
                        (c[0] - a[0]).hypot(c[1] - a[1])
                    })
                    .fold(0.0, f64::max);
                b.min_x -= most;
                b.min_y -= most;
                b.max_x += most;
                b.max_y += most;
            }
        }
    }
    let grow = SPAN * height + width;
    Bounds { min_x: b.min_x - grow, min_y: b.min_y - grow, max_x: b.max_x + grow, max_y: b.max_y + grow }
}

fn meets(a: &Bounds, b: &Bounds) -> bool {
    a.min_x <= b.max_x && b.min_x <= a.max_x && a.min_y <= b.max_y && b.min_y <= a.max_y
}

impl Obstacles {
    /// The drawing's objects the sheet's map shows within `area`: on a shown layer, not on `skip` (the separation
    /// sheets' own point and piece layers, each map freezing the other parcels').
    pub fn of_drawing(doc: &Document, skip: &BTreeSet<String>, area: &Bounds, font: Font) -> Obstacles {
        let mut out = Obstacles::default();
        for (uid, e) in doc.entities_with_uids() {
            let layer = &e.base().layer_id;
            if skip.contains(layer) || !doc.layers().is_visible(layer) {
                continue;
            }
            let shape = crate::geometry::shape(e);
            if !meets(&entity_bounds_in(&shape, font), area) {
                continue;
            }
            let owner = uid.to_string();
            out.edges.extend(entity_edges(&shape).into_iter().map(|edge| (edge, owner.clone())));
            if let Some(t) = TextPlace::of(&shape) {
                out.boxes.push((t.outline(font), owner.clone()));
            }
            if let Some(t) = leader::note_place(&shape) {
                out.boxes.push((t.outline(font), owner.clone()));
            }
            if let Shape::Point { p, .. } = shape {
                out.points.push((p, owner));
            }
        }
        out
    }

    /// A number's text inside its part (its box).
    pub fn add_box(&mut self, ring: Vec<Vec2>, owner: &str) {
        self.boxes.push((ring, owner.to_owned()));
    }

    /// A number placed on a leader: its whole line (from its tip through its landing) and its note's box.
    pub fn add_leader(&mut self, placed: &Placed, owner: &str) {
        for w in placed.line.windows(2) {
            self.edges.push((Edge::Seg { a: w[0], b: w[1] }, owner.to_owned()));
        }
        self.boxes.push((placed.note.clone(), owner.to_owned()));
    }
}

/// A place a number outside took: where its landing starts (the leader's last vertex), its line from the tip through
/// the landing's end, its note's box, and what it touches (sorted; empty when clear).
pub struct Placed {
    pub bend: [f64; 2],
    pub line: Vec<Vec2>,
    pub note: Vec<Vec2>,
    pub clashes: Vec<String>,
}

/// Where the ray from `tip` in direction `d` last crosses the part's rings, and the point `k` heights past it on the
/// ray: east, the last of the part's crossings of the horizontal line through the tip (`crossings_at`'s rule, as
/// docs/adr/4680 has it); any other way, the same in the part turned so the ray runs east from the origin.
fn leaving(part: &ScopePart, tip: [f64; 2], d: [f64; 2], past: f64) -> Option<([f64; 2], [f64; 2])> {
    if d == [1.0, 0.0] {
        let east = *crossings_at(part, tip[1]).last()?;
        return Some(([east, tip[1]], [east + past, tip[1]]));
    }
    let mut local = part.clone();
    for r in &mut local.rings {
        for p in &mut r.pts {
            let (x, y) = (p[0] - tip[0], p[1] - tip[1]);
            *p = [x * d[0] + y * d[1], d[0] * y - d[1] * x];
        }
    }
    let s = *crossings_at(&local, 0.0).last()?;
    let at = |s: f64| [tip[0] + s * d[0], tip[1] + s * d[1]];
    Some((at(s), at(s + past)))
}

fn seg(a: Vec2, b: Vec2) -> Edge {
    Edge::Seg { a, b }
}

fn sides(ring: &[Vec2]) -> impl Iterator<Item = Edge> + '_ {
    (0..ring.len()).map(move |i| seg(ring[i], ring[(i + 1) % ring.len()]))
}

/// Whether `p` lies in the convex ring (its boundary too).
fn within(ring: &[Vec2], p: Vec2) -> bool {
    let n = ring.len();
    let crosses: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
        })
        .collect();
    n >= 3 && (crosses.iter().all(|c| *c >= 0.0) || crosses.iter().all(|c| *c <= 0.0))
}

fn start_of(e: &Edge) -> Vec2 {
    match *e {
        Edge::Seg { a, .. } => a,
        Edge::Arc { c, r, a0, .. } => Vec2::new(c.x + r * a0.cos(), c.y + r * a0.sin()),
    }
}

/// Whether an edge touches a convex box: crosses or touches a side, or lies inside it.
fn edge_meets_box(e: &Edge, ring: &[Vec2]) -> bool {
    within(ring, start_of(e)) || sides(ring).any(|s| !intersect_edges(&s, e).is_empty())
}

/// Whether two convex boxes touch.
fn boxes_meet(a: &[Vec2], b: &[Vec2]) -> bool {
    a.iter().any(|p| within(b, *p)) || b.iter().any(|p| within(a, *p)) || sides(a).any(|s| sides(b).any(|t| !intersect_edges(&s, &t).is_empty()))
}

/// What a place touches: its line's stretch outside the part (hits where it leaves the part not counted), its landing
/// and its note's box, against every obstacle; sorted, each owner once.
fn clashes(obstacles: &Obstacles, out: [Vec2; 2], landing: [Vec2; 2], note: &[Vec2]) -> Vec<String> {
    let (outside, land) = (seg(out[0], out[1]), seg(landing[0], landing[1]));
    let mut hit = BTreeSet::new();
    for (e, owner) in &obstacles.edges {
        if hit.contains(owner) {
            continue;
        }
        if intersect_edges(&outside, e).iter().any(|h| h.t > LEAVING)
            || !intersect_edges(&land, e).is_empty()
            || edge_meets_box(e, note)
        {
            hit.insert(owner.clone());
        }
    }
    for (ring, owner) in &obstacles.boxes {
        if hit.contains(owner) {
            continue;
        }
        if edge_meets_box(&outside, ring) || edge_meets_box(&land, ring) || boxes_meet(note, ring) {
            hit.insert(owner.clone());
        }
    }
    for (p, owner) in &obstacles.points {
        if within(note, *p) {
            hit.insert(owner.clone());
        }
    }
    hit.into_iter().collect()
}

/// Where a number `height` tall goes outside its part on a leader from `tip`: the first clear place in the order the
/// places are tried, else the one touching the fewest (the first of equals). None when no ray leaves the part.
pub fn place(part: &ScopePart, tip: [f64; 2], number: &str, height: f64, font: Font, obstacles: &Obstacles) -> Option<Placed> {
    let mut best: Option<Placed> = None;
    for k in REACH {
        for d in DIRECTIONS {
            let Some((out, bend)) = leaving(part, tip, d, k * height) else { continue };
            let pts = [v(tip), v(bend)];
            let layout = leader::layout(&pts, height, 0.0, None, None, true)?;
            let (Some(landing), Some(p), align) = (layout.landing, layout.note_point, layout.note_align) else { continue };
            let note = TextPlace::line(p, number, height, 0.0, align, None).outline(font);
            let found = clashes(obstacles, [v(out), v(bend)], landing, &note);
            let clear = found.is_empty();
            if best.as_ref().is_none_or(|b| found.len() < b.clashes.len()) {
                best = Some(Placed { bend, line: vec![v(tip), v(bend), landing[1]], note, clashes: found });
            }
            if clear {
                return best;
            }
        }
    }
    best
}

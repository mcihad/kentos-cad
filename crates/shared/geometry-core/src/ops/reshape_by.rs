//! Biçim değiştir (docs/adr/0173 §2–§3), one for both platforms (the web
//! through WASM): an area or a path reshaped by a sketched line.
//!
//! - An area part: the sketch cutting it keeps the largest piece; the sketch
//!   closing pockets with its outer ring outside it adds them. Both at once,
//!   neither, a hole met or more than one part met is refused. The pieces
//!   and the pockets come from the overlay engine (arcs stay arcs).
//! - A path (a line or a polyline): its stretch between the sketch's first
//!   and last meeting, along the sketch, takes the sketch's stretch between
//!   them; one meeting redraws the path's shorter side by the sketch's
//!   longer side. An arc the meeting cuts stays on its circle.
//!
//! The independent reference is `scripts/fixtures/reshape_cases.py`.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape, area_parts, replace_part};
use crate::geom::arrangement::{Area, Ring, Source};
use crate::geom::bulge::{
    bulge_at, bulge_of_sweep, bulge_path_edges, bulge_path_length, clean_bulge_path, is_arc_bulge,
};
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges};
use crate::geom::region::{
    FaceIndex, intersect_areas, net_area, ring_edges, split_area, union_areas,
};
use crate::jsmath::{atan, js_hypot, js_max};
use crate::op;
use crate::vec2::Vec2;

/// A sketch's end this near a path is on it (the snap's place, 1 µm).
pub const TOUCH: f64 = 1e-6;

/// Why a reshape is refused.
#[derive(Clone, Debug, PartialEq)]
pub enum ReshapeRefusal {
    /// The object is no area, line or polyline.
    NotShape,
    /// The sketch has fewer than two points.
    TooShort,
    /// An area: the sketch neither cuts the part nor closes a pocket with
    /// it; a path: the sketch neither crosses nor touches it.
    NoCrossing,
    /// The sketch meets more than one part of the area.
    ManyParts,
    /// The sketch meets a hole.
    TouchesHole,
    /// The sketch both cuts the part and closes a pocket.
    BothWays,
    /// The pockets would leave the area in pieces.
    Apart,
    /// A multi-part polyline: which part the line runs along is not known (docs/adr/0174 §3).
    MultiPart,
}

crate::json_tagged!(ReshapeRefusal, "why",
    NotShape => "notShape" {},
    TooShort => "tooShort" {},
    NoCrossing => "noCrossing" {},
    ManyParts => "manyParts" {},
    TouchesHole => "touchesHole" {},
    BothWays => "bothWays" {},
    Apart => "apart" {},
    MultiPart => "multiPart" {},
);

/// The object reshaped by `sketch` (its points in order).
pub fn reshape(e: &Shape, sketch: &[Vec2]) -> Result<Shape, ReshapeRefusal> {
    if sketch.len() < 2 {
        return Err(ReshapeRefusal::TooShort);
    }
    match e {
        Shape::Polygon { .. } => reshape_area(e, sketch),
        Shape::Line { a, b } => reshape_path(&[*a, *b], &[0.0, 0.0], sketch),
        Shape::Polyline { .. } if crate::entity::is_multi_part(e) => Err(ReshapeRefusal::MultiPart),
        Shape::Polyline {
            pts,
            bulges,
            holes: None,
            ..
        } => {
            let b: Vec<f64> = (0..pts.len())
                .map(|i| bulge_at(bulges.as_deref(), i))
                .collect();
            reshape_path(pts, &b, sketch)
        }
        _ => Err(ReshapeRefusal::NotShape),
    }
}

fn segments(pts: &[Vec2]) -> Vec<Edge> {
    pts.windows(2)
        .map(|w| Edge::Seg { a: w[0], b: w[1] })
        .collect()
}

fn line_source(pts: &[Vec2]) -> Source {
    Source {
        edges: segments(pts),
        points: Some(pts.to_vec()),
        cut: None,
    }
}

fn ring_source(r: &Ring) -> Source {
    Source {
        edges: ring_edges(r),
        points: Some(r.pts.clone()),
        cut: None,
    }
}

/// Whether any of the edges cross or touch, or an end of the sketch lies on the edges.
fn meets(edges: &[Edge], sketch: &[Vec2], sketch_edges: &[Edge]) -> bool {
    edges.iter().any(|e| {
        sketch_edges
            .iter()
            .any(|s| !intersect_edges(e, s).is_empty())
    }) || [sketch[0], sketch[sketch.len() - 1]]
        .iter()
        .any(|&p| edges.iter().any(|e| closest_on_edge(e, p).d <= TOUCH))
}

fn reshape_area(e: &Shape, sketch: &[Vec2]) -> Result<Shape, ReshapeRefusal> {
    let sketch_edges = segments(sketch);
    let parts = area_parts(e);
    let mut met = Vec::new();
    for (k, part) in parts.iter().enumerate() {
        let Shape::Polygon {
            pts, bulges, holes, ..
        } = part
        else {
            return Err(ReshapeRefusal::NotShape);
        };
        for h in holes.iter().flatten() {
            if meets(&ring_edges(h), sketch, &sketch_edges) {
                return Err(ReshapeRefusal::TouchesHole);
            }
        }
        let outer = Ring {
            pts: pts.clone(),
            bulges: bulges.clone(),
        };
        if meets(&ring_edges(&outer), sketch, &sketch_edges) {
            met.push((
                k,
                Area {
                    outer,
                    holes: holes.clone().unwrap_or_default(),
                },
            ));
        }
    }
    let (k, area) = match met.len() {
        0 => return Err(ReshapeRefusal::NoCrossing),
        1 => met.remove(0),
        _ => return Err(ReshapeRefusal::ManyParts),
    };
    let pieces = split_area(&area, &line_source(sketch));
    let cuts = pieces.len() >= 2;
    // The faces the sketch closes with the outer ring outside the part.
    let solid = Area {
        outer: area.outer.clone(),
        holes: Vec::new(),
    };
    let pockets: Vec<Area> = FaceIndex::new(&[ring_source(&area.outer), line_source(sketch)])
        .faces(false)
        .into_iter()
        .filter(|f| {
            let overlap: f64 = intersect_areas(&[f.clone(), solid.clone()])
                .iter()
                .map(net_area)
                .sum();
            overlap <= 1e-9 * js_max(net_area(f).abs(), 1.0)
        })
        .collect();
    let result = match (cuts, pockets.is_empty()) {
        (true, true) => {
            let mut best = &pieces[0];
            for p in &pieces[1..] {
                if net_area(p) > net_area(best) {
                    best = p;
                }
            }
            best.clone()
        }
        (false, false) => {
            let mut all = vec![area];
            all.extend(pockets);
            let mut joined = union_areas(&all);
            if joined.len() != 1 {
                return Err(ReshapeRefusal::Apart);
            }
            joined.remove(0)
        }
        (true, false) => return Err(ReshapeRefusal::BothWays),
        (false, true) => return Err(ReshapeRefusal::NoCrossing),
    };
    let part = Shape::Polygon {
        pts: result.outer.pts,
        bulges: result.outer.bulges,
        holes: (!result.holes.is_empty()).then_some(result.holes),
        parts: None,
    };
    replace_part(e, k, part).ok_or(ReshapeRefusal::NotShape)
}

/// Where the sketch meets the path: along the sketch (segment index plus
/// the fraction along it), along the path (edge index plus the fraction),
/// and the point.
#[derive(Clone, Copy, Debug)]
struct Meeting {
    s: f64,
    l: f64,
    p: Vec2,
}

/// The path cut at `l` (edge index plus fraction) through `p`: its head
/// (start to the cut) and its tail (the cut to the end), each its points
/// and bulges (as many as its points, the last 0). A cut on a vertex is that
/// vertex; an arc cut stays on its circle.
type Run = (Vec<Vec2>, Vec<f64>);

fn cut_path(pts: &[Vec2], b: &[f64], l: f64, p: Vec2) -> (Run, Run) {
    let edges = pts.len() - 1;
    let whole = l.floor();
    let mut i = if whole < 0.0 { 0 } else { whole as usize };
    let mut t = l - i as f64;
    if i >= edges {
        i = edges - 1;
        t = 1.0;
    }
    let at_vertex = if t <= 1e-12 {
        Some(i)
    } else if t >= 1.0 - 1e-12 {
        Some(i + 1)
    } else {
        None
    };
    if let Some(v) = at_vertex {
        let mut hb = b[..v].to_vec();
        hb.push(0.0);
        let head = (pts[..=v].to_vec(), hb);
        let tail = (pts[v..].to_vec(), b[v..].to_vec());
        return (head, tail);
    }
    let (b1, b2) = if is_arc_bulge(b[i]) {
        let sweep = 4.0 * atan(b[i]);
        (bulge_of_sweep(sweep * t), bulge_of_sweep(sweep * (1.0 - t)))
    } else {
        (0.0, 0.0)
    };
    let mut hp = pts[..=i].to_vec();
    hp.push(p);
    let mut hb = b[..i].to_vec();
    hb.extend([b1, 0.0]);
    let mut tp = vec![p];
    tp.extend_from_slice(&pts[i + 1..]);
    let mut tb = vec![b2];
    tb.extend_from_slice(&b[i + 1..]);
    ((hp, hb), (tp, tb))
}

fn polyline_length(pts: &[Vec2]) -> f64 {
    pts.windows(2)
        .map(|w| js_hypot(w[1].x - w[0].x, w[1].y - w[0].y))
        .sum()
}

/// The sketch's points from the meeting at `s` outward: backward to its
/// start, or forward to its end; the meeting's point first.
fn sketch_from(sketch: &[Vec2], s: f64, p: Vec2, forward: bool) -> Vec<Vec2> {
    let mut out = vec![p];
    if forward {
        let next = s.floor() as usize + 1;
        out.extend(sketch.iter().skip(next).copied());
    } else {
        let back = if s - s.floor() <= 1e-12 {
            s.floor() as isize - 1
        } else {
            s.floor() as isize
        };
        let mut k = back;
        while k >= 0 {
            out.push(sketch[k as usize]);
            k -= 1;
        }
    }
    out
}

fn reshape_path(pts: &[Vec2], b: &[f64], sketch: &[Vec2]) -> Result<Shape, ReshapeRefusal> {
    let path_edges = bulge_path_edges(pts, Some(b), false);
    let sketch_edges = segments(sketch);
    let mut found: Vec<Meeting> = Vec::new();
    for (j, s) in sketch_edges.iter().enumerate() {
        for (i, e) in path_edges.iter().enumerate() {
            for h in intersect_edges(e, s) {
                found.push(Meeting {
                    s: j as f64 + h.u,
                    l: i as f64 + h.t,
                    p: h.p,
                });
            }
        }
    }
    // The sketch's ends on the path count as meetings (a snapped start).
    for (s, p) in [
        (0.0, sketch[0]),
        ((sketch.len() - 1) as f64, sketch[sketch.len() - 1]),
    ] {
        let near = path_edges
            .iter()
            .enumerate()
            .map(|(i, e)| (i, closest_on_edge(e, p)))
            .filter(|(_, c)| c.d <= TOUCH)
            .min_by(|x, y| x.1.d.total_cmp(&y.1.d));
        if let Some((i, c)) = near {
            found.push(Meeting {
                s,
                l: i as f64 + c.t,
                p: c.p,
            });
        }
    }
    found.sort_by(|x, y| x.s.total_cmp(&y.s).then(x.l.total_cmp(&y.l)));
    let mut meetings: Vec<Meeting> = Vec::new();
    for m in found {
        if let Some(last) = meetings.last()
            && js_hypot(m.p.x - last.p.x, m.p.y - last.p.y) <= 1e-9
        {
            continue;
        }
        meetings.push(m);
    }
    let (Some(&first), Some(&last)) = (meetings.first(), meetings.last()) else {
        return Err(ReshapeRefusal::NoCrossing);
    };
    let (out_p, out_b) = if meetings.len() >= 2 {
        let (a, z) = if first.l <= last.l {
            (first, last)
        } else {
            (last, first)
        };
        // The sketch's vertices strictly between the two meetings, from `a` to `z`.
        let mut mid: Vec<Vec2> = sketch
            .iter()
            .enumerate()
            .filter(|&(k, _)| (k as f64) > first.s && (k as f64) < last.s)
            .map(|(_, &q)| q)
            .collect();
        if first.l > last.l {
            mid.reverse();
        }
        let ((hp, mut hb), _) = cut_path(pts, b, a.l, a.p);
        let (_, (tp, tb)) = cut_path(pts, b, z.l, z.p);
        let mut out_p = hp;
        hb.pop();
        hb.push(0.0);
        for q in mid {
            out_p.push(q);
            hb.push(0.0);
        }
        out_p.extend(tp);
        hb.extend(tb);
        (out_p, hb)
    } else {
        let ((hp, mut hb), (tp, tb)) = cut_path(pts, b, first.l, first.p);
        let head_len = bulge_path_length(&hp, Some(&hb), false);
        let tail_len = bulge_path_length(&tp, Some(&tb), false);
        let before = sketch_from(sketch, first.s, first.p, false);
        let after = sketch_from(sketch, first.s, first.p, true);
        let outward = if polyline_length(&before) > polyline_length(&after) {
            before
        } else {
            after
        };
        if tail_len > head_len {
            // The head goes: the sketch runs into the tail.
            let mut out_p: Vec<Vec2> = outward.iter().rev().copied().collect();
            let mut out_b = vec![0.0; out_p.len() - 1];
            out_p.extend(tp.into_iter().skip(1));
            out_b.extend(tb);
            (out_p, out_b)
        } else {
            let mut out_p = hp;
            hb.pop();
            for q in outward.into_iter().skip(1) {
                out_p.push(q);
                hb.push(0.0);
            }
            hb.push(0.0);
            (out_p, hb)
        }
    };
    let clean = clean_bulge_path(&out_p, Some(&out_b), false, 1e-9);
    if clean.pts.len() < 2 {
        return Err(ReshapeRefusal::NoCrossing);
    }
    Ok(Shape::Polyline {
        pts: clean.pts,
        bulges: clean.bulges,
        holes: None,
        parts: None,
    })
}

/// A reshape's answer for the web: the object, or why not.
pub struct Answer(pub Result<Entity, ReshapeRefusal>);

impl ToJson for Answer {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok(e) => field(out, &mut first, "done", e),
            Err(r) => field(out, &mut first, "refusal", r),
        }
        out.push('}');
    }
}

pub(crate) static OPS: &[Op] = &[op!("reshapeBy", |e: Entity, sketch: Vec<Vec2>| {
    Answer(reshape(&e.shape, &sketch).map(Entity::new))
})];

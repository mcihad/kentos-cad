//! Elevations carried to an edit's new vertices (docs/adr/0142). An edit's
//! geometry comes from the core without elevations; each vertex of the
//! result takes one from the objects the edit started from, by the first of
//! these rules that reaches it:
//!
//! 1. on a source vertex (within [`ON`]): that vertex's elevation;
//! 2. `same`, given for an edit that moves an object's own vertices (a grip,
//!    Esnet, a typed coordinate) and for Ötele's copy, with as many vertices:
//!    the elevation of the vertex in the same place of its order;
//! 3. on a source edge: the edge's, linearly along it (a cut, a new corner,
//!    a fillet's or a chamfer's end), by length on a straight edge and by
//!    angle on an arc;
//! 4. an open result's end on the straight extension of a source's end edge:
//!    that edge's grade carried on (Uzat, Uzat-kısalt);
//! 5. for Ötele, the elevation at the closest point of the sources.
//!
//! A vertex no rule reaches has none (`None`, not 0), and so has one whose
//! edge has an end without an elevation.

use crate::api::Op;
use crate::geom::bulge::bulge_arc;
use crate::geom::intersect::{Edge, closest_on_edge, param_on};
use crate::jsmath::js_hypot;
use crate::op;
use crate::vec2::Vec2;

/// How near a vertex must be to lie on a source vertex or edge (m).
pub const ON: f64 = 1e-6;

/// A path of an object before the edit, with its elevations: a line (two
/// vertices), a polyline, a polygon's outer ring or one of its holes.
#[derive(Clone, Debug, PartialEq)]
pub struct Elevated {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
    pub closed: bool,
    pub zs: Vec<Option<f64>>,
}

crate::json_struct!(Elevated {
    pts,
    bulges,
    closed,
    zs
});

/// Whether rule 5 (Ötele's closest point) applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carry {
    Along,
    Offset,
}

/// An edge of a source with its ends' elevations.
type Rise = (Edge, Option<f64>, Option<f64>);

/// A source's edges, each with its ends' elevations.
fn edges(s: &Elevated) -> Vec<Rise> {
    let n = s.pts.len();
    let count = if s.closed { n } else { n.saturating_sub(1) };
    (0..count)
        .map(|i| {
            let j = (i + 1) % n;
            let (a, b) = (s.pts[i], s.pts[j]);
            let bulge = s.bulges.as_ref().and_then(|v| v.get(i)).copied();
            let edge = match bulge.and_then(|v| bulge_arc(a, b, v)) {
                Some(arc) => Edge::Arc {
                    c: arc.c,
                    r: arc.r,
                    a0: arc.a0,
                    sweep: arc.sweep,
                },
                None => Edge::Seg { a, b },
            };
            let z = |k: usize| s.zs.get(k).copied().flatten();
            (edge, z(i), z(j))
        })
        .collect()
}

fn between(za: Option<f64>, zb: Option<f64>, t: f64) -> Option<f64> {
    Some(za? + (zb? - za?) * t)
}

/// Rule 4: `p` beyond a straight end edge of an open source, on its line.
fn extended(p: Vec2, s: &Elevated) -> Option<f64> {
    let n = s.pts.len();
    if s.closed || n < 2 {
        return None;
    }
    let straight = |i: usize| {
        s.bulges
            .as_ref()
            .and_then(|v| v.get(i))
            .is_none_or(|&b| b == 0.0)
    };
    let z = |k: usize| s.zs.get(k).copied().flatten();
    // The last edge carried on past its end, the first before its start.
    let ends = [(n - 2, n - 1, true), (0, 1, false)];
    for (i, j, forward) in ends {
        if !straight(i) {
            continue;
        }
        let (a, b) = (s.pts[i], s.pts[j]);
        let edge = Edge::Seg { a, b };
        let t = param_on(&edge, p);
        let beyond = if forward { t > 1.0 } else { t < 0.0 };
        let on_line = Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
        if beyond
            && js_hypot(p.x - on_line.x, p.y - on_line.y) <= ON
            && let Some(v) = between(z(i), z(j), t)
        {
            return Some(v);
        }
    }
    None
}

/// Elevations for the vertices `pts` of an edit's result (`closed`: a ring),
/// carried from `sources` by the module's rules; `same` is the object itself
/// before the edit, when the result replaces it.
pub fn carry_elevations(
    pts: &[Vec2],
    closed: bool,
    same: Option<&Elevated>,
    sources: &[Elevated],
    carry: Carry,
) -> Vec<Option<f64>> {
    let all: Vec<Vec<Rise>> = sources.iter().map(edges).collect();
    let last = pts.len().saturating_sub(1);
    pts.iter()
        .enumerate()
        .map(|(i, &p)| {
            // 1. On a source vertex.
            for s in sources {
                for (k, q) in s.pts.iter().enumerate() {
                    if js_hypot(p.x - q.x, p.y - q.y) <= ON
                        && let Some(z) = s.zs.get(k).copied().flatten()
                    {
                        return Some(z);
                    }
                }
            }
            // 2. The same object's vertex in the same place of its order.
            if let Some(s) = same.filter(|s| s.pts.len() == pts.len())
                && let Some(z) = s.zs.get(i).copied().flatten()
            {
                return Some(z);
            }
            // 3. On a source edge.
            for edges in &all {
                for (edge, za, zb) in edges {
                    let hit = closest_on_edge(edge, p);
                    if hit.d <= ON
                        && let Some(z) = between(*za, *zb, hit.t)
                    {
                        return Some(z);
                    }
                }
            }
            // 4. An open result's end on a source's extension.
            if !closed
                && (i == 0 || i == last)
                && let Some(z) = sources.iter().find_map(|s| extended(p, s))
            {
                return Some(z);
            }
            // 5. Ötele: the closest point of the sources.
            if carry == Carry::Offset {
                let mut best: Option<(f64, Option<f64>)> = None;
                for edges in &all {
                    for (edge, za, zb) in edges {
                        let hit = closest_on_edge(edge, p);
                        if best.is_none_or(|(d, _)| hit.d < d) {
                            best = Some((hit.d, between(*za, *zb, hit.t)));
                        }
                    }
                }
                return best.and_then(|(_, z)| z);
            }
            None
        })
        .collect()
}

pub(crate) static OPS: &[Op] = &[op!(
    "carryElevations",
    |pts: Vec<Vec2>, closed: bool, same: Option<Elevated>, sources: Vec<Elevated>, offset: bool| {
        carry_elevations(
            &pts,
            closed,
            same.as_ref(),
            &sources,
            if offset { Carry::Offset } else { Carry::Along },
        )
    }
)];

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn line(a: Vec2, b: Vec2, za: Option<f64>, zb: Option<f64>) -> Elevated {
        Elevated {
            pts: vec![a, b],
            bulges: None,
            closed: false,
            zs: vec![za, zb],
        }
    }

    #[test]
    fn a_cut_on_an_edge_takes_its_elevation_linearly() {
        // (0,0,10)–(10,0,20): the middle is 15, a quarter 12.5, the ends their own.
        let s = [line(v(0.0, 0.0), v(10.0, 0.0), Some(10.0), Some(20.0))];
        let got = carry_elevations(
            &[v(0.0, 0.0), v(5.0, 0.0), v(2.5, 0.0), v(10.0, 0.0)],
            false,
            None,
            &s,
            Carry::Along,
        );
        assert_eq!(got, [Some(10.0), Some(15.0), Some(12.5), Some(20.0)]);
        // Off the line: none, unless it is Ötele's copy.
        assert_eq!(
            carry_elevations(&[v(5.0, 1.0)], false, None, &s, Carry::Along),
            [None]
        );
        assert_eq!(
            carry_elevations(&[v(5.0, 1.0)], false, None, &s, Carry::Offset),
            [Some(15.0)]
        );
    }

    #[test]
    fn an_edge_with_an_end_without_elevation_gives_none() {
        let s = [line(v(0.0, 0.0), v(10.0, 0.0), Some(10.0), None)];
        let got = carry_elevations(&[v(0.0, 0.0), v(5.0, 0.0)], false, None, &s, Carry::Along);
        assert_eq!(got, [Some(10.0), None]);
    }

    #[test]
    fn an_arc_edge_shares_its_elevation_by_angle() {
        // A half circle from (0,0) to (10,0) (bulge 1): its middle is at a
        // quarter of a turn, half of its sweep.
        let s = [Elevated {
            pts: vec![v(0.0, 0.0), v(10.0, 0.0)],
            bulges: Some(vec![1.0, 0.0]),
            closed: false,
            zs: vec![Some(0.0), Some(10.0)],
        }];
        let arc = bulge_arc(v(0.0, 0.0), v(10.0, 0.0), 1.0).unwrap();
        let mid = arc.a0 + arc.sweep / 2.0;
        let m = v(
            arc.c.x + crate::jsmath::cos(mid) * arc.r,
            arc.c.y + crate::jsmath::sin(mid) * arc.r,
        );
        let got = carry_elevations(&[m], false, None, &s, Carry::Along);
        let z = got[0].unwrap();
        assert!((z - 5.0).abs() < 1e-9, "{z}");
    }

    #[test]
    fn an_extended_end_carries_the_grade_on() {
        // Grade 1 in 10 from (0,0,0) to (10,0,1): extended to x = 15 it is 1.5,
        // and before its start, at x = −5, −0.5.
        let s = [line(v(0.0, 0.0), v(10.0, 0.0), Some(0.0), Some(1.0))];
        let got = carry_elevations(&[v(0.0, 0.0), v(15.0, 0.0)], false, None, &s, Carry::Along);
        assert_eq!(got, [Some(0.0), Some(1.5)]);
        let got = carry_elevations(&[v(-5.0, 0.0), v(10.0, 0.0)], false, None, &s, Carry::Along);
        assert_eq!(got, [Some(-0.5), Some(1.0)]);
        // Only an end: a middle vertex out on the extension is not reached.
        let got = carry_elevations(
            &[v(0.0, 0.0), v(15.0, 0.0), v(15.0, 5.0)],
            false,
            None,
            &s,
            Carry::Along,
        );
        assert_eq!(got, [Some(0.0), None, None]);
    }

    #[test]
    fn a_moved_vertex_keeps_its_own_by_its_place() {
        // A grip moves the second vertex off everything: the same object's order answers.
        let before = Elevated {
            pts: vec![v(0.0, 0.0), v(5.0, 0.0), v(10.0, 0.0)],
            bulges: None,
            closed: false,
            zs: vec![Some(1.0), Some(2.0), Some(3.0)],
        };
        let after = [v(0.0, 0.0), v(5.0, 7.0), v(10.0, 0.0)];
        let got = carry_elevations(
            &after,
            false,
            Some(&before),
            std::slice::from_ref(&before),
            Carry::Along,
        );
        assert_eq!(got, [Some(1.0), Some(2.0), Some(3.0)]);
        // Reversed, the vertices find their own by place in the plane, not by order.
        let reversed = [v(10.0, 0.0), v(5.0, 0.0), v(0.0, 0.0)];
        let got = carry_elevations(
            &reversed,
            false,
            Some(&before),
            std::slice::from_ref(&before),
            Carry::Along,
        );
        assert_eq!(got, [Some(3.0), Some(2.0), Some(1.0)]);
    }

    #[test]
    fn a_ring_s_closing_edge_counts() {
        // A 10 m square at 0, 10, 20, 30 m: the closing edge's middle is 15.
        let s = [Elevated {
            pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0), v(0.0, 10.0)],
            bulges: None,
            closed: true,
            zs: vec![Some(0.0), Some(10.0), Some(20.0), Some(30.0)],
        }];
        let got = carry_elevations(&[v(0.0, 5.0)], true, None, &s, Carry::Along);
        assert_eq!(got, [Some(15.0)]);
    }
}

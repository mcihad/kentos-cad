//! A route's pieces inside an area (docs/adr/0224 §4, Rota boyunca konumla
//! for areas): each edge cut where it crosses the area's rings, a stretch
//! between two cuts inside when its middle is; touching inside stretches of
//! one part are one piece, with the measures at its ends.

use super::{ON, Walk};
use crate::geom::arrangement::{Area, winding};
use crate::geom::intersect::{Edge, intersect_edges};
use crate::geom::region::ring_edges;

/// A piece of a route inside an area: the measures at its ends (none on an
/// edge with an end without one) and the lengths walked to them.
#[derive(Clone, Debug, PartialEq)]
pub struct Inside {
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub start: f64,
    pub end: f64,
}

crate::json_struct!(Inside {
    from,
    to,
    start,
    end
});

/// An area's ring edges, made once for the many middles asked.
struct Rings {
    outer: Vec<Edge>,
    holes: Vec<Vec<Edge>>,
}

impl Rings {
    fn of(a: &Area) -> Rings {
        Rings {
            outer: ring_edges(&a.outer),
            holes: a.holes.iter().map(ring_edges).collect(),
        }
    }

    fn inside(&self, p: crate::vec2::Vec2) -> bool {
        winding(&self.outer, p) != 0.0 && !self.holes.iter().any(|h| winding(h, p) != 0.0)
    }

    fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.outer.iter().chain(self.holes.iter().flatten())
    }
}

/// The route's pieces inside any of `areas` (an object's parts), in walked
/// order. A stretch of [`ON`] or less neither opens nor closes a piece.
pub fn locate_area(walk: &Walk, areas: &[Area]) -> Vec<Inside> {
    let rings: Vec<Rings> = areas.iter().map(Rings::of).collect();
    let mut out: Vec<Inside> = Vec::new();
    let mut open = false;
    let mut cuts: Vec<f64> = Vec::new();
    for (k, s) in walk.steps.iter().enumerate() {
        let (_, index, along) = walk.at[k];
        if index == 0 {
            open = false;
        }
        cuts.clear();
        cuts.extend([0.0, 1.0]);
        for r in &rings {
            for e in r.edges() {
                for hit in intersect_edges(&s.edge, e) {
                    if hit.t > 0.0 && hit.t < 1.0 {
                        cuts.push(hit.t);
                    }
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        for w in cuts.windows(2) {
            let (t0, t1) = (w[0], w[1]);
            if (t1 - t0) * s.len <= ON {
                continue;
            }
            let mid = s.at((t0 + t1) / 2.0);
            if rings.iter().any(|r| r.inside(mid)) {
                let (m1, end) = (s.m(t1), along + s.len * t1);
                match out.last_mut() {
                    Some(q) if open => {
                        q.to = m1;
                        q.end = end;
                    }
                    _ => {
                        out.push(Inside {
                            from: s.m(t0),
                            to: m1,
                            start: along + s.len * t0,
                            end,
                        });
                        open = true;
                    }
                }
            } else {
                open = false;
            }
        }
    }
    out
}

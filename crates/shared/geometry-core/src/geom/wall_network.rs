//! Analytic room boundaries noded once, with a single path per shared span.
//! Pure geometry: layer/style/reference policy belongs to the native command.
use super::{
    arrangement::{self, Ring, Source, TOL, edge_len, reverse_edge},
    bulge::{bulge_of_sweep, bulge_path_edges},
    intersect::{Edge, closest_on_edge, intersect_edges, point_at},
    overlay::{localize, origin_of},
    region::ring_edges,
};
use crate::{Vec2, geometry::Bounds, store::rtree::PackedTree};

/// Sparse result; `source` is the first input using each shared boundary.
#[derive(Clone, Debug)]
pub struct Run {
    pub source: usize,
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
    pub closed: bool,
}
pub struct Network {
    pub runs: Vec<Run>,
    pub affected: Vec<bool>,
    pub shared_length: f64,
    pub shared_pieces: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    Budget,
    TinyEdge,
    Crossing,
    Contact,
    OverlappingRooms,
    AmbiguousCurve,
}

/// A positive shared interval, not a crossing or a single touching point.
/// Uses the existing arrangement's metre tolerance without tessellation.
pub fn overlap(a: &Edge, b: &Edge) -> bool {
    match (a, b) {
        (Edge::Seg { .. }, Edge::Seg { .. }) => {}
        (Edge::Arc { c: ca, r: ra, .. }, Edge::Arc { c: cb, r: rb, .. })
            if (ca.x - cb.x).abs() <= TOL
                && (ca.y - cb.y).abs() <= TOL
                && (ra - rb).abs() <= TOL => {}
        _ => return false,
    }
    let mut ts = Vec::new();
    for t in [0., 1.] {
        if closest_on_edge(b, point_at(a, t)).d <= TOL {
            ts.push(t);
        }
        let found = closest_on_edge(a, point_at(b, t));
        if found.d <= TOL {
            ts.push(found.t);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.windows(2).any(|w| {
        (w[1] - w[0]) * edge_len(a) > TOL
            && closest_on_edge(b, point_at(a, (w[0] + w[1]) / 2.)).d <= TOL
    })
}

/// None means the original closed rings can stay bit-for-bit unchanged.
/// Intersections and curve splitting are owned by the established arrangement.
pub fn build(rings: &[Ring]) -> Result<Option<Network>, Problem> {
    if rings.len() < 2 {
        return Ok(None);
    }
    let abs: Vec<_> = rings
        .iter()
        .map(|r| Source {
            edges: ring_edges(r),
            points: Some(r.pts.clone()),
            cut: None,
        })
        .collect();
    let edges = abs.iter().map(|s| s.edges.len()).sum::<usize>();
    // arrangement::Piece carries one f64 per source. Bound its initial dense
    // ownership table to 64 MiB; refuse, never truncate or approximate inputs.
    if edges.checked_mul(rings.len()).is_none_or(|n| n > 8_000_000) {
        return Err(Problem::Budget);
    }
    let o = origin_of(&abs);
    let local = localize(&abs, o);
    let flat: Vec<_> = local
        .iter()
        .enumerate()
        .flat_map(|(source, s)| s.edges.iter().map(move |e| (source, *e)))
        .collect();
    let entries: Vec<_> = flat
        .iter()
        .enumerate()
        .map(|(i, (_, e))| (i as u32, arrangement::edge_box(e)))
        .collect();
    let tree = PackedTree::build(&entries);
    let mut nearby = Vec::new();
    for (i, (source, ea)) in flat.iter().enumerate() {
        let (_, bounds) = entries[i];
        nearby.clear();
        tree.search(&bounds, &mut nearby);
        for &j in &nearby {
            let j = j as usize;
            if j <= i || flat[j].0 == *source {
                continue;
            }
            let eb = &flat[j].1;
            if intersect_edges(ea, eb).iter().any(|h| {
                h.t * edge_len(ea) > TOL
                    && (1. - h.t) * edge_len(ea) > TOL
                    && h.u * edge_len(eb) > TOL
                    && (1. - h.u) * edge_len(eb) > TOL
            }) {
                return Err(Problem::Crossing);
            }
        }
    }
    let graph = arrangement::build(&local, &abs, o);
    let mut affected = vec![false; rings.len()];
    let mut owners = Vec::with_capacity(graph.pieces.len());
    let mut shared_length = 0.;
    let mut shared_pieces = 0;
    let mut shared_nodes = vec![false; graph.verts.pos.len()];
    let mut adjacent = vec![Vec::new(); graph.verts.pos.len()];
    for (i, p) in graph.pieces.iter().enumerate() {
        let sources: Vec<_> = p
            .delta
            .iter()
            .enumerate()
            .filter(|(_, d)| **d != 0.)
            .collect();
        let Some(&(first, sign)) = sources.first() else {
            return Err(Problem::AmbiguousCurve);
        };
        owners.push((first, *sign > 0.));
        if sources.len() > 1 {
            if sources.len() != 2
                || sources[0].1 * sources[1].1 >= 0.
                || sources.iter().any(|(_, d)| d.abs() != 1.)
            {
                return Err(Problem::OverlappingRooms);
            }
            // The arrangement also admits near midpoint matches. A shared wall
            // needs the same analytic carrier, checked at the normal tolerance.
            let mid = point_at(&p.edge, 0.5);
            nearby.clear();
            tree.search(
                &Bounds {
                    min_x: mid.x - TOL,
                    min_y: mid.y - TOL,
                    max_x: mid.x + TOL,
                    max_y: mid.y + TOL,
                },
                &mut nearby,
            );
            if !nearby
                .iter()
                .copied()
                .filter(|&j| flat[j as usize].0 == sources[0].0)
                .any(|i| {
                    nearby
                        .iter()
                        .copied()
                        .filter(|&j| flat[j as usize].0 == sources[1].0)
                        .any(|j| {
                            overlap(&flat[i as usize].1, &flat[j as usize].1)
                                && closest_on_edge(&flat[i as usize].1, mid).d <= TOL
                                && closest_on_edge(&flat[j as usize].1, mid).d <= TOL
                        })
                })
            {
                return Err(Problem::AmbiguousCurve);
            }
            for (s, _) in sources {
                affected[s] = true;
            }
            shared_length += edge_len(&p.edge);
            shared_pieces += 1;
            shared_nodes[p.from] = true;
            shared_nodes[p.to] = true;
        }
        adjacent[p.from].push(i);
        adjacent[p.to].push(i);
    }
    for (v, at) in adjacent.iter().enumerate() {
        if !shared_nodes[v]
            && at
                .first()
                .is_some_and(|&p| at.iter().any(|&q| owners[q].0 != owners[p].0))
        {
            return Err(Problem::Contact);
        }
    }
    if shared_pieces == 0 {
        return Ok(None);
    }
    if abs
        .iter()
        .any(|s| s.edges.iter().any(|e| edge_len(e) <= TOL))
    {
        return Err(Problem::TinyEdge);
    }
    let point = |v: usize| {
        graph.verts.input[v].unwrap_or_else(|| {
            let p = graph.verts.pos[v];
            Vec2::new(p.x + o.x, p.y + o.y)
        })
    };
    let next = |v: usize, old: usize, source: usize| -> Option<usize> {
        if adjacent[v].len() != 2 {
            return None;
        }
        adjacent[v]
            .iter()
            .copied()
            .find(|&p| p != old && owners[p].0 == source)
    };
    let mut used = vec![false; graph.pieces.len()];
    let mut runs = Vec::new();
    for seed in 0..graph.pieces.len() {
        let (source, forward) = owners[seed];
        if used[seed] || !affected[source] {
            continue;
        }
        let first = &graph.pieces[seed];
        let (mut start, mut at) = if forward {
            (first.from, first.to)
        } else {
            (first.to, first.from)
        };
        let mut chain = std::collections::VecDeque::from([(seed, forward)]);
        used[seed] = true;
        let mut prior = seed;
        while let Some(p) = next(start, prior, source) {
            if used[p] {
                break;
            }
            let e = &graph.pieces[p];
            let f = e.to == start;
            start = if f { e.from } else { e.to };
            chain.push_front((p, f));
            used[p] = true;
            prior = p;
        }
        let mut prior = seed;
        while let Some(p) = next(at, prior, source) {
            if used[p] {
                break;
            }
            let e = &graph.pieces[p];
            let f = e.from == at;
            at = if f { e.to } else { e.from };
            chain.push_back((p, f));
            used[p] = true;
            prior = p;
        }
        let closed = start == at;
        let mut pts = Vec::new();
        let mut bulges = Vec::new();
        for (p, fwd) in chain {
            let e = &graph.pieces[p];
            pts.push(point(if fwd { e.from } else { e.to }));
            let edge = if fwd { e.edge } else { reverse_edge(&e.edge) };
            bulges.push(match edge {
                Edge::Seg { .. } => 0.,
                Edge::Arc { sweep, .. } => bulge_of_sweep(sweep),
            });
        }
        if !closed {
            pts.push(point(at));
        }
        runs.push(Run {
            source,
            pts,
            bulges: bulges.iter().any(|b| *b != 0.).then_some(bulges),
            closed,
        });
    }
    runs.sort_by_key(|r| r.source);
    Ok(Some(Network {
        runs,
        affected,
        shared_length,
        shared_pieces,
    }))
}

/// Useful when detecting a repeated conversion after shared rings were split
/// into ordinary open wall runs.
pub fn paths_overlap(a: &Ring, b_pts: &[Vec2], b_bulges: Option<&[f64]>, closed: bool) -> bool {
    let origin = a.pts.first().copied().unwrap_or_default();
    let local = |p: &Vec2| Vec2::new(p.x - origin.x, p.y - origin.y);
    let a_pts: Vec<_> = a.pts.iter().map(local).collect();
    let b_pts: Vec<_> = b_pts.iter().map(local).collect();
    let aa = bulge_path_edges(&a_pts, a.bulges.as_deref(), true);
    let bb = bulge_path_edges(&b_pts, b_bulges, closed);
    aa.iter().any(|a| bb.iter().any(|b| overlap(a, b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Ring {
        Ring {
            pts: vec![
                Vec2::new(x, y),
                Vec2::new(x + w, y),
                Vec2::new(x + w, y + h),
                Vec2::new(x, y + h),
            ],
            bulges: None,
        }
    }
    #[test]
    fn full_partial_and_collinear_subdivisions_preserve_unique_analytic_length() {
        for (x, y) in [(0., 0.), (500000., 4400000.)] {
            for (by, bh, shared, runs) in [(0., 3., 3., 3), (1., 1., 1., 3), (-1., 5., 3., 3)] {
                let a = rectangle(x, y, 4., 3.);
                let b = rectangle(x + 4., y + by, 4., bh);
                let n = build(&[a, b]).unwrap().unwrap();
                assert!((n.shared_length - shared).abs() < 1e-9);
                assert_eq!(n.runs.len(), runs);
                let length = n
                    .runs
                    .iter()
                    .map(|p| {
                        bulge_path_edges(&p.pts, p.bulges.as_deref(), p.closed)
                            .iter()
                            .map(edge_len)
                            .sum::<f64>()
                    })
                    .sum::<f64>();
                assert!(
                    (length - (14. + 8. + 2. * bh - shared)).abs() < 1e-8,
                    "{length}"
                );
            }
        }
    }
    #[test]
    fn circle_inside_a_hole_remains_one_exact_closed_arc_wall() {
        let a = Ring {
            pts: vec![Vec2::new(3., 0.), Vec2::new(-3., 0.)],
            bulges: Some(vec![-1., -1.]),
        };
        let b = Ring {
            pts: a.pts.clone(),
            bulges: Some(vec![1., 1.]),
        };
        let n = build(&[a, b]).unwrap().unwrap();
        assert_eq!(n.runs.len(), 1);
        assert!(n.runs[0].closed);
        assert!((n.shared_length - 6. * std::f64::consts::PI).abs() < 1e-10);
        assert!(
            n.runs[0]
                .bulges
                .as_ref()
                .unwrap()
                .iter()
                .all(|b| (b.abs() - 1.).abs() < 1e-14)
        );
    }
    #[test]
    fn overlapping_areas_crossings_and_corner_only_contact_are_not_a_shared_wall() {
        let a = rectangle(0., 0., 4., 3.);
        for (b, wanted) in [
            (a.clone(), Problem::OverlappingRooms),
            (rectangle(3., 1., 4., 3.), Problem::Crossing),
            (rectangle(4., 3., 4., 3.), Problem::Contact),
        ] {
            assert_eq!(build(&[a.clone(), b]).err(), Some(wanted));
        }
        assert!(build(&[a, rectangle(10., 0., 4., 3.)]).unwrap().is_none());
    }
}

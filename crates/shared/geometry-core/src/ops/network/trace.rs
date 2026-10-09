//! Şebeke izleme (docs/adr/0209 §8): what a place on the network reaches
//! without a cost. Bağlı reaches everything along any piece; Akış aşağı
//! follows each piece the way its edge goes (a two-way piece both ways), Akış
//! yukarı against it; Yalıtım goes any way and stops at open valves: those are
//! the valves to close and what it reached is cut off. With sources in the
//! network, what no source reaches once those valves are closed (and did
//! before) is beslemesiz kalan. Closed edges are never followed; closed
//! junctions and barriers are reached but not passed.

use std::collections::{HashSet, VecDeque};

use super::graph::{Graph, Location};
use super::search::{Places, Span};

/// Which trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceKind {
    Connected,
    Downstream,
    Upstream,
    Isolation,
}

/// A trace's answer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Traced {
    /// The stretches reached, each once, in the order reached.
    pub spans: Vec<Span>,
    /// The edges (indices into the graph's edges) with a stretch reached, ascending.
    pub edges: Vec<u32>,
    /// Yalıtım: the valves to close (indices into the graph's junctions), ascending.
    pub valves: Vec<u32>,
    /// Yalıtım with sources: the pieces no source reaches once the valves are closed, and their edges.
    pub unfed: Vec<u32>,
    pub unfed_edges: Vec<u32>,
    pub length: f64,
    pub unfed_length: f64,
}

/// Whether piece `p` is followed `forward` (the way it is drawn) in a trace of `kind`.
fn follows(g: &Graph, p: u32, forward: bool, kind: TraceKind) -> bool {
    let dir = g.edges[g.pieces[p as usize].edge as usize].dir;
    match kind {
        TraceKind::Connected | TraceKind::Isolation => dir != super::rules::Dir::Closed,
        TraceKind::Downstream => dir.allows(forward),
        TraceKind::Upstream => dir.allows(!forward),
    }
}

/// The pieces reached from `starts` by any way, not passing closed junctions, `places`' barriers or `stop` nodes.
fn reach(g: &Graph, places: &Places, starts: &[u32], stop: &HashSet<u32>) -> HashSet<u32> {
    let mut seen = vec![false; places.count()];
    let mut queue: VecDeque<u32> = VecDeque::new();
    for &n in starts {
        if !seen[n as usize] {
            seen[n as usize] = true;
            queue.push_back(n);
        }
    }
    let mut pieces = HashSet::new();
    while let Some(n) = queue.pop_front() {
        places.steps(g, n, |next, piece, forward, _| {
            if !follows(g, piece, forward, TraceKind::Connected) {
                return;
            }
            pieces.insert(piece);
            if places.blocked(g, next) || stop.contains(&next) || seen[next as usize] {
                return;
            }
            seen[next as usize] = true;
            queue.push_back(next);
        });
    }
    pieces
}

/// The trace of `kind` from `starts`, not passing `barriers`.
pub fn trace(g: &Graph, starts: &[Location], barriers: &[Location], kind: TraceKind) -> Traced {
    let places = Places::new(g, starts, &[], barriers);
    let start_nodes: Vec<u32> = (0..starts.len()).map(|k| places.node_of[k]).collect();
    let mut seen = vec![false; places.count()];
    let mut queue: VecDeque<u32> = VecDeque::new();
    for &n in &start_nodes {
        if !seen[n as usize] {
            seen[n as usize] = true;
            queue.push_back(n);
        }
    }
    let mut out = Traced::default();
    // What is taken: a whole piece by a flag, a stretch of one (between a start or a barrier inside it) by its ends.
    let mut whole = vec![false; g.pieces.len()];
    let mut taken: HashSet<(u32, u64, u64)> = HashSet::new();
    let mut valves: HashSet<u32> = HashSet::new();
    let mut edge_seen = vec![false; g.edges.len()];
    let mut next_nodes: Vec<(u32, u32, bool)> = Vec::new();
    while let Some(n) = queue.pop_front() {
        next_nodes.clear();
        places.steps(g, n, |next, piece, forward, _| {
            if follows(g, piece, forward, kind) {
                next_nodes.push((next, piece, forward));
            }
        });
        for &(next, piece, forward) in &next_nodes {
            let a = places.offset(g, n, piece, forward);
            let b = places.offset(g, next, piece, !forward);
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let fresh = if lo == 0.0 && hi == g.pieces[piece as usize].len {
                !std::mem::replace(&mut whole[piece as usize], true)
            } else {
                taken.insert((piece, lo.to_bits(), hi.to_bits()))
            };
            if fresh {
                out.spans.push(Span { piece, a, b });
                out.length += hi - lo;
                edge_seen[g.pieces[piece as usize].edge as usize] = true;
            }
            if places.blocked(g, next) || seen[next as usize] {
                continue;
            }
            seen[next as usize] = true;
            if kind == TraceKind::Isolation
                && (next as usize) < g.nodes.len()
                && g.kinds[next as usize].valve
                && !start_nodes.contains(&next)
            {
                valves.insert(next);
                continue;
            }
            queue.push_back(next);
        }
    }
    out.edges = (0..g.edges.len() as u32)
        .filter(|&e| edge_seen[e as usize])
        .collect();
    if kind == TraceKind::Isolation {
        // The valves' junctions: every open valve at a stopping node.
        out.valves = g
            .junctions
            .iter()
            .enumerate()
            .filter(|(_, j)| {
                j.role == super::graph::Role::Valve
                    && !j.closed
                    && j.node.is_some_and(|n| valves.contains(&n))
            })
            .map(|(k, _)| k as u32)
            .collect();
        let sources: Vec<u32> = (0..g.nodes.len() as u32)
            .filter(|&n| g.kinds[n as usize].source)
            .collect();
        if !sources.is_empty() {
            let open = Places::new(g, &[], &[], barriers);
            let before = reach(g, &open, &sources, &HashSet::new());
            let after = reach(g, &open, &sources, &valves);
            let cut: HashSet<u32> = out.spans.iter().map(|s| s.piece).collect();
            let mut unfed: Vec<u32> = before
                .difference(&after)
                .filter(|p| !cut.contains(p))
                .copied()
                .collect();
            unfed.sort_unstable();
            let mut unfed_edges: Vec<u32> =
                unfed.iter().map(|&p| g.pieces[p as usize].edge).collect();
            unfed_edges.sort_unstable();
            unfed_edges.dedup();
            out.unfed_length = unfed.iter().map(|&p| g.pieces[p as usize].len).sum();
            out.unfed = unfed;
            out.unfed_edges = unfed_edges;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::graph::{EdgeIn, JunctionIn, Role};
    use super::super::rules::Rules;
    use super::*;
    use crate::api::json::Json;
    use crate::geom::intersect::Edge;
    use crate::vec2::Vec2;

    fn pipe(id: f64, a: (f64, f64), b: (f64, f64)) -> EdgeIn {
        EdgeIn {
            id,
            part: 0,
            path: vec![Edge::Seg {
                a: Vec2::new(a.0, a.1),
                b: Vec2::new(b.0, b.1),
            }],
            direction: None,
            costs: Vec::new(),
            closed: false,
        }
    }

    /// A main from a tank at (0,0) east to (300,0) with valves at 100 and 200, and a branch north at 150.
    fn water() -> Graph {
        let rules = Rules::from_json(
            &Json::parse(r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"digitized"}}"#)
                .expect("JSON"),
        )
        .expect("rules");
        let edges = [
            pipe(1.0, (0.0, 0.0), (300.0, 0.0)),
            pipe(2.0, (150.0, 0.0), (150.0, 80.0)),
        ];
        let j = |id: f64, x: f64, y: f64, role: Role| JunctionIn {
            id,
            p: Vec2::new(x, y),
            role,
            closed: false,
        };
        let junctions = [
            j(10.0, 0.0, 0.0, Role::Source),
            j(11.0, 100.0, 0.0, Role::Valve),
            j(12.0, 200.0, 0.0, Role::Valve),
        ];
        Graph::build(rules, &edges, &junctions).0
    }

    #[test]
    fn isolating_a_break_closes_the_valves_around_it_and_cuts_off_what_lies_beyond() {
        let g = water();
        let at = |x: f64, y: f64| g.locate(Vec2::new(x, y), 1.0).expect("on a pipe");
        let t = trace(&g, &[at(150.0, 40.0)], &[], TraceKind::Isolation);
        // The branch and the main between the valves: 100 + 80 m; the two valves; beyond 200 nothing is fed.
        assert!((t.length - 180.0).abs() < 1e-9, "{t:?}");
        assert_eq!(t.valves, vec![1, 2]);
        assert!((t.unfed_length - 100.0).abs() < 1e-9);
        assert_eq!(t.unfed_edges, vec![0]);
        // Downstream from the tank: the whole main and the branch (pipes drawn with the flow).
        let down = trace(&g, &[at(1.0, 0.0)], &[], TraceKind::Downstream);
        assert!((down.length - 379.0).abs() < 1e-9, "{down:?}");
        // Upstream from the branch's end: back along it and the main to the tank.
        let up = trace(&g, &[at(150.0, 80.0)], &[], TraceKind::Upstream);
        assert!((up.length - 230.0).abs() < 1e-9, "{up:?}");
    }
}

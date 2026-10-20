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
use super::terminals::NO_TERMINAL;

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
    /// The junctions at a node the trace reached, a start's too (indices into the graph's junctions), ascending
    /// (docs/adr/0525 §1).
    pub junctions: Vec<u32>,
    /// Yalıtım with sources: the pieces no source reaches once the valves are closed, and their edges.
    pub unfed: Vec<u32>,
    pub unfed_edges: Vec<u32>,
    pub length: f64,
    pub unfed_length: f64,
}

/// Whether piece `p` is followed `forward` (the way it is drawn) in a trace of `kind`.
pub(super) fn follows(g: &Graph, p: u32, forward: bool, kind: TraceKind) -> bool {
    let dir = g.edges[g.pieces[p as usize].edge as usize].dir;
    match kind {
        TraceKind::Connected | TraceKind::Isolation => dir != super::rules::Dir::Closed,
        TraceKind::Downstream => dir.allows(forward),
        TraceKind::Upstream => dir.allows(!forward),
    }
}

/// The pieces reached from `starts` by any way, not passing closed junctions, `places`' barriers or `stop` nodes.
fn reach(g: &Graph, places: &Places, starts: &[u32], stop: &HashSet<u32>) -> HashSet<u32> {
    let mut seen = Seen::new(g, places);
    let mut queue: VecDeque<(u32, u8)> = VecDeque::new();
    for &n in starts {
        if seen.mark(n, NO_TERMINAL) {
            queue.push_back((n, NO_TERMINAL));
        }
    }
    let mut pieces = HashSet::new();
    while let Some((n, from)) = queue.pop_front() {
        places.steps(g, n, |next, piece, forward, _| {
            if !follows(g, piece, forward, TraceKind::Connected)
                || !leaves(g, places, n, from, piece, forward)
            {
                return;
            }
            pieces.insert(piece);
            let by = arrival(g, places, next, piece, forward);
            if places.blocked(g, next) || stop.contains(&next) || !seen.mark(next, by) {
                return;
            }
            queue.push_back((next, by));
        });
    }
    pieces
}

/// The nodes a trace has been at, and by which terminal when a node is a device (docs/adr/0557 §1).
pub(super) struct Seen<'a> {
    g: &'a Graph,
    base: u32,
    seen: Vec<bool>,
}

impl<'a> Seen<'a> {
    pub(super) fn new(g: &'a Graph, places: &Places) -> Seen<'a> {
        let base = g.nodes.len() as u32;
        let n = match &g.terms {
            None => places.count(),
            Some(t) => t.states as usize + places.count() - base as usize,
        };
        Seen {
            g,
            base,
            seen: vec![false; n],
        }
    }

    fn at(&self, n: u32, t: u8) -> usize {
        match &self.g.terms {
            None => n as usize,
            Some(ts) if n < self.base => ts.state(n, t),
            Some(ts) => ts.states as usize + (n - self.base) as usize,
        }
    }

    /// Marks node `n` reached by terminal `t`; false when it already was.
    pub(super) fn mark(&mut self, n: u32, t: u8) -> bool {
        let i = self.at(n, t);
        !std::mem::replace(&mut self.seen[i], true)
    }
}

/// Whether a way that reached node `n` by terminal `from` may leave along `piece` (`forward`: the way it is drawn):
/// at a device only along a path (docs/adr/0557 §1); a virtual node is no device.
pub(super) fn leaves(
    g: &Graph,
    places: &Places,
    n: u32,
    from: u8,
    piece: u32,
    forward: bool,
) -> bool {
    n >= places.base() || g.passes(n, from, piece * 2 + u32::from(!forward))
}

/// The terminal by which a step along `piece` arrives at `next`.
pub(super) fn arrival(g: &Graph, places: &Places, next: u32, piece: u32, forward: bool) -> u8 {
    if next >= places.base() {
        NO_TERMINAL
    } else {
        g.arrives(piece * 2 + u32::from(!forward))
    }
}

/// The trace of `kind` from `starts`, not passing `barriers`.
pub fn trace(g: &Graph, starts: &[Location], barriers: &[Location], kind: TraceKind) -> Traced {
    let places = Places::new(g, starts, &[], barriers);
    let start_nodes: Vec<u32> = (0..starts.len()).map(|k| places.node_of[k]).collect();
    let mut seen = Seen::new(g, &places);
    let mut queue: VecDeque<(u32, u8)> = VecDeque::new();
    for &n in &start_nodes {
        if seen.mark(n, NO_TERMINAL) {
            queue.push_back((n, NO_TERMINAL));
        }
    }
    let mut out = Traced::default();
    // What is taken: a whole piece by a flag, a stretch of one (between a start or a barrier inside it) by its ends.
    let mut whole = vec![false; g.pieces.len()];
    let mut taken: HashSet<(u32, u64, u64)> = HashSet::new();
    let mut valves: HashSet<u32> = HashSet::new();
    let mut edge_seen = vec![false; g.edges.len()];
    let mut next_nodes: Vec<(u32, u32, bool, u8)> = Vec::new();
    // Every node a stretch reached (a closed junction and a barrier too, reached but not passed).
    let mut touched: HashSet<u32> = start_nodes.iter().copied().collect();
    while let Some((n, from)) = queue.pop_front() {
        next_nodes.clear();
        places.steps(g, n, |next, piece, forward, _| {
            if follows(g, piece, forward, kind) && leaves(g, &places, n, from, piece, forward) {
                let by = arrival(g, &places, next, piece, forward);
                next_nodes.push((next, piece, forward, by));
            }
        });
        for &(next, piece, forward, by) in &next_nodes {
            let a = places.offset(g, n, piece, forward);
            let b = places.offset(g, next, piece, !forward);
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            let fresh = if lo == 0.0 && hi == g.pieces[piece as usize].len {
                !std::mem::replace(&mut whole[piece as usize], true)
            } else {
                taken.insert((piece, lo.to_bits(), hi.to_bits()))
            };
            touched.insert(next);
            if fresh {
                out.spans.push(Span { piece, a, b });
                out.length += hi - lo;
                edge_seen[g.pieces[piece as usize].edge as usize] = true;
            }
            if places.blocked(g, next) || !seen.mark(next, by) {
                continue;
            }
            if kind == TraceKind::Isolation
                && (next as usize) < g.nodes.len()
                && g.kinds[next as usize].valve
                && !start_nodes.contains(&next)
            {
                valves.insert(next);
                continue;
            }
            queue.push_back((next, by));
        }
    }
    out.edges = (0..g.edges.len() as u32)
        .filter(|&e| edge_seen[e as usize])
        .collect();
    out.junctions = g
        .junctions
        .iter()
        .enumerate()
        .filter(|(_, j)| j.node.is_some_and(|n| touched.contains(&n)))
        .map(|(k, _)| k as u32)
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
            levels: [None, None, None],
            periods: Vec::new(),
            terminals: [None, None],
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
            level: None,
            device: None,
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
        // The junctions reached (docs/adr/0525 §1): Yalıtım stops at the two valves, they are reached; downstream
        // from just past the tank the valves, not the tank behind; all that is connected every one.
        assert_eq!(t.junctions, vec![1, 2]);
        assert_eq!(down.junctions, vec![1, 2]);
        let all = trace(&g, &[at(1.0, 0.0)], &[], TraceKind::Connected);
        assert_eq!(all.junctions, vec![0, 1, 2]);
    }

    /// A trace over a near miss says it (docs/adr/0525 §2): a service pipe ends 0.5 m short of the main.
    #[test]
    fn a_trace_through_a_connection_not_made_is_not_one_to_rely_on() {
        let rules = Rules::from_json(
            &Json::parse(r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"digitized"}}"#)
                .expect("JSON"),
        )
        .expect("rules");
        let edges = [
            pipe(1.0, (0.0, 0.0), (300.0, 0.0)),
            pipe(2.0, (100.0, 0.5), (100.0, 60.0)),
            pipe(3.0, (500.0, 0.0), (600.0, 0.0)),
        ];
        let (g, report) = Graph::build(rules, &edges, &[]);
        let s = super::super::session::NetworkSession::new(g, report, Vec::new());
        let main = s
            .trace_of(&[Vec2::new(50.0, 0.0)], &[], 1.0, TraceKind::Connected)
            .expect("traced");
        assert_eq!(main.objects, vec![1.0]);
        assert_eq!(main.unverified.len(), 1, "{:?}", main.unverified);
        assert_eq!(main.unverified[0].kind, "nearMiss");
        // A pipe far from any problem: nothing unverified.
        let far = s
            .trace_of(&[Vec2::new(550.0, 0.0)], &[], 1.0, TraceKind::Connected)
            .expect("traced");
        assert!(far.unverified.is_empty(), "{:?}", far.unverified);
    }

    /// A device with three terminals, a and b on one path and c on none (docs/adr/0557 §1); with `unnamed`, a fourth
    /// edge that names no terminal.
    fn device(terminal_rules: bool, unnamed: bool) -> (Graph, super::super::graph::BuildReport) {
        let rules = Rules::from_json(
            &Json::parse(if terminal_rules {
                r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"both"},
                "terminals":{"start":"s","end":"e","devices":[{"layer":"d",
                "terminals":[{"id":"a","name":"A"},{"id":"b","name":"B"},{"id":"c","name":"C"}],
                "paths":[["a","b"]]}]}}"#
            } else {
                r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"both"}}"#
            })
            .expect("JSON"),
        )
        .expect("rules");
        let mut edges = vec![
            pipe(1.0, (0.0, 0.0), (100.0, 0.0)),
            pipe(2.0, (100.0, 0.0), (200.0, 0.0)),
            pipe(3.0, (100.0, 0.0), (100.0, 80.0)),
        ];
        if unnamed {
            edges.push(pipe(4.0, (100.0, 0.0), (100.0, -80.0)));
        }
        edges[0].terminals[1] = Some("A".into());
        edges[1].terminals[0] = Some("b".into());
        edges[2].terminals[0] = Some("c".into());
        let junctions = [JunctionIn {
            id: 10.0,
            p: Vec2::new(100.0, 0.0),
            role: Role::Junction,
            closed: false,
            level: None,
            device: terminal_rules.then_some(0),
        }];
        Graph::build(rules, &edges, &junctions)
    }

    fn ids(g: &Graph, t: &Traced) -> Vec<f64> {
        t.edges.iter().map(|&e| g.edges[e as usize].id).collect()
    }

    #[test]
    fn a_trace_goes_through_a_device_only_along_its_paths() {
        let (g, report) = device(true, false);
        let at = |x: f64, y: f64| g.locate(Vec2::new(x, y), 1.0).expect("on an edge");
        // From the edge on terminal a: the edge on b (a path), not c's.
        let from_a = trace(&g, &[at(50.0, 0.0)], &[], TraceKind::Connected);
        assert_eq!(ids(&g, &from_a), vec![1.0, 2.0]);
        assert_eq!(from_a.junctions, vec![0]);
        // From b's edge: back to a; from c's edge: c alone (a terminal connects to itself).
        let from_b = trace(&g, &[at(150.0, 0.0)], &[], TraceKind::Connected);
        assert_eq!(ids(&g, &from_b), vec![1.0, 2.0]);
        let from_c = trace(&g, &[at(100.0, 40.0)], &[], TraceKind::Connected);
        assert_eq!(ids(&g, &from_c), vec![3.0]);
        // Starting on the device itself reaches every edge.
        let on = trace(&g, &[at(100.0, 0.0)], &[], TraceKind::Connected);
        assert_eq!(ids(&g, &on), vec![1.0, 2.0, 3.0]);
        assert!(report.unterminated.is_empty());
        // Without terminals in the definition every edge connects.
        let (plain, _) = device(false, false);
        let loc = plain.locate(Vec2::new(100.0, 40.0), 1.0).expect("on an edge");
        assert_eq!(trace(&plain, &[loc], &[], TraceKind::Connected).edges.len(), 3);
    }

    /// An end that names no terminal passes to every terminal, both ways: it is said, and a trace over it is not one
    /// to rely on.
    #[test]
    fn an_end_naming_no_terminal_passes_to_all_and_is_said() {
        let (g, report) = device(true, true);
        assert_eq!(report.unterminated, vec![(4.0, Vec2::new(100.0, 0.0))]);
        let s = super::super::session::NetworkSession::new(g, report, Vec::new());
        let t = s
            .trace_of(&[Vec2::new(50.0, 0.0)], &[], 1.0, TraceKind::Connected)
            .expect("traced");
        assert_eq!(t.objects, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(t.unverified.len(), 1);
        assert_eq!(t.unverified[0].kind, "terminal");
    }

    #[test]
    fn a_group_of_controllers_reaches_what_its_device_passes() {
        use super::super::subnetworks::{Reach, subnetworks};
        let (g, _) = device(true, false);
        // The device itself as the controller starts at no terminal: every edge.
        let all = subnetworks(&g, &[vec![0]], Reach::Connected);
        assert_eq!(all[0].edges, vec![0, 1, 2]);
        assert_eq!(all[0].junctions, vec![0]);
        assert!((all[0].length - 280.0).abs() < 1e-9);
    }
}

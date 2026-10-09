//! Denetle (docs/adr/0209 §9): a network's counts and what may be wrong with
//! it, each with its place: parts not connected to the largest, an end near
//! another edge but not on it (within 1 m), edges crossing without a node,
//! junctions near no edge, short pieces dropped, values not read. A dead end
//! is counted, not a problem: road networks have them.

use std::collections::{HashMap, HashSet};

use super::graph::{BuildReport, Graph};
use crate::geom::arrangement::edge_box;
use crate::geom::intersect::{closest_on_edge, intersect_edges};
use crate::jsmath::{js_hypot, js_round};
use crate::vec2::Vec2;

/// How far an end may be from another edge to be called a near miss (metres).
pub const NEAR_MISS: f64 = 1.0;
/// The most problems listed (the counts go on).
pub const PROBLEM_LIMIT: usize = 1000;

/// What a problem is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProblemKind {
    /// A part not connected to the largest.
    Detached,
    /// An end within 1 m of another edge, not on it.
    NearMiss,
    /// Two edges crossing without a node.
    Crossing,
    /// A junction near no edge.
    OffNetwork,
    /// A piece dropped as no longer than the tolerance.
    Short,
    /// A cost field's value not read.
    Unread,
}

/// A problem: its kind, where, the edges' or junctions' ids and its measure (a gap, a part's length).
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub kind: ProblemKind,
    pub at: Vec2,
    pub ids: Vec<f64>,
    pub value: Option<f64>,
    /// For `Unread`: the cost (1 the first besides the length).
    pub cost: Option<usize>,
}

/// Denetle's answer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Checked {
    pub nodes: usize,
    pub pieces: usize,
    pub length: f64,
    /// The connected parts' piece counts and lengths, the longest first.
    pub parts: Vec<(usize, f64)>,
    pub dead_ends: usize,
    /// Every kind's count, problems listed or not.
    pub counts: Vec<(ProblemKind, usize)>,
    pub problems: Vec<Problem>,
}

fn root(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let up = parent[parent[x as usize] as usize];
        parent[x as usize] = up;
        x = up;
    }
    x
}

/// Checks `g`, built with `report`.
pub fn check(g: &Graph, report: &BuildReport) -> Checked {
    let tol = g.rules.tolerance;
    let mut out = Checked {
        nodes: g.nodes.len(),
        pieces: g.pieces.len(),
        length: g.length(),
        ..Checked::default()
    };
    let mut problems: Vec<Problem> = Vec::new();
    // Parts: the pieces joined by their nodes, any way.
    let mut parent: Vec<u32> = (0..g.nodes.len() as u32).collect();
    for p in &g.pieces {
        let (a, b) = (root(&mut parent, p.from), root(&mut parent, p.to));
        if a != b {
            parent[a.max(b) as usize] = a.min(b);
        }
    }
    let mut parts: HashMap<u32, (usize, f64, u32)> = HashMap::new();
    for (k, p) in g.pieces.iter().enumerate() {
        let r = root(&mut parent, p.from);
        let e = parts.entry(r).or_insert((0, 0.0, k as u32));
        e.0 += 1;
        e.1 += p.len;
    }
    let mut list: Vec<(usize, f64, u32)> = parts.into_values().collect();
    list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.2.cmp(&b.2)));
    for &(_, len, first) in list.iter().skip(1) {
        let p = &g.pieces[first as usize];
        problems.push(Problem {
            kind: ProblemKind::Detached,
            at: g.point_on(first, p.len / 2.0),
            ids: vec![g.edges[p.edge as usize].id],
            value: Some(len),
            cost: None,
        });
    }
    out.parts = list.iter().map(|&(n, len, _)| (n, len)).collect();
    // Dead ends and near misses: an end whose node has one piece, near another piece not at its node.
    let mut degree = vec![0usize; g.nodes.len()];
    for p in &g.pieces {
        degree[p.from as usize] += 1;
        degree[p.to as usize] += 1;
    }
    out.dead_ends = degree.iter().filter(|&&d| d == 1).count();
    let mut pairs: HashSet<(u32, u32)> = HashSet::new();
    for (n, &d) in degree.iter().enumerate() {
        if d != 1 {
            continue;
        }
        let p = g.nodes[n];
        let Some(own) = g.at_node(n as u32).first().map(|e| e / 2) else {
            continue;
        };
        let mut best: Option<(f64, u32, Vec2)> = None;
        let mut found = Vec::new();
        g.near_geom(p, NEAR_MISS, &mut found);
        for gi in found {
            let piece = g.piece_of_geom(gi);
            let pc = &g.pieces[piece as usize];
            if piece == own || pc.from == n as u32 || pc.to == n as u32 {
                continue;
            }
            let c = closest_on_edge(&g.geom[gi as usize], p);
            if c.d > tol
                && c.d <= NEAR_MISS
                && best.is_none_or(|b| c.d < b.0 || (c.d == b.0 && piece < b.1))
            {
                best = Some((c.d, piece, c.p));
            }
        }
        if let Some((d, piece, q)) = best {
            let key = (own.min(piece), own.max(piece));
            if pairs.insert(key) {
                let ids = vec![
                    g.edges[g.pieces[own as usize].edge as usize].id,
                    g.edges[g.pieces[piece as usize].edge as usize].id,
                ];
                problems.push(Problem {
                    kind: ProblemKind::NearMiss,
                    at: Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0),
                    ids,
                    value: Some(d),
                    cost: None,
                });
            }
        }
    }
    // Crossings: primitives of two pieces meeting farther than the tolerance from both pieces' ends.
    let mut seen: HashSet<(u64, u64)> = HashSet::new();
    let mut found = Vec::new();
    for (gi, e) in g.geom.iter().enumerate() {
        let pa = g.piece_of_geom(gi as u32);
        found.clear();
        g.near_box(&edge_box(e), &mut found);
        for &gj in &found {
            if (gj as usize) <= gi {
                continue;
            }
            let pb = g.piece_of_geom(gj);
            if pa == pb {
                continue;
            }
            for h in intersect_edges(e, &g.geom[gj as usize]) {
                let ends = [
                    g.pieces[pa as usize].from,
                    g.pieces[pa as usize].to,
                    g.pieces[pb as usize].from,
                    g.pieces[pb as usize].to,
                ];
                if ends.iter().any(|&n| {
                    let q = g.nodes[n as usize];
                    js_hypot(q.x - h.p.x, q.y - h.p.y) <= tol
                }) {
                    continue;
                }
                let key = (
                    js_round(h.p.x / tol) as i64 as u64,
                    js_round(h.p.y / tol) as i64 as u64,
                );
                if seen.insert(key) {
                    problems.push(Problem {
                        kind: ProblemKind::Crossing,
                        at: h.p,
                        ids: vec![
                            g.edges[g.pieces[pa as usize].edge as usize].id,
                            g.edges[g.pieces[pb as usize].edge as usize].id,
                        ],
                        value: None,
                        cost: None,
                    });
                }
            }
        }
    }
    for &(id, at) in &report.off_network {
        problems.push(Problem {
            kind: ProblemKind::OffNetwork,
            at,
            ids: vec![id],
            value: None,
            cost: None,
        });
    }
    for &(id, at) in &report.short {
        problems.push(Problem {
            kind: ProblemKind::Short,
            at,
            ids: vec![id],
            value: None,
            cost: None,
        });
    }
    for &(c, id) in &report.unread {
        let at = g
            .edges
            .iter()
            .find(|e| e.id == id)
            .map_or(Vec2::new(0.0, 0.0), |e| {
                Vec2::new((e.a.x + e.b.x) / 2.0, (e.a.y + e.b.y) / 2.0)
            });
        problems.push(Problem {
            kind: ProblemKind::Unread,
            at,
            ids: vec![id],
            value: None,
            cost: Some(c),
        });
    }
    let mut counts: Vec<(ProblemKind, usize)> = Vec::new();
    for p in &problems {
        match counts.iter_mut().find(|(k, _)| *k == p.kind) {
            Some((_, n)) => *n += 1,
            None => counts.push((p.kind, 1)),
        }
    }
    counts.sort_by_key(|(k, _)| *k);
    problems.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.at.x.total_cmp(&b.at.x))
            .then(a.at.y.total_cmp(&b.at.y))
    });
    problems.truncate(PROBLEM_LIMIT);
    out.counts = counts;
    out.problems = problems;
    out
}

#[cfg(test)]
mod tests {
    use super::super::graph::{EdgeIn, JunctionIn, Role};
    use super::super::rules::Rules;
    use super::*;
    use crate::api::json::Json;
    use crate::geom::intersect::Edge;

    fn line(id: f64, a: (f64, f64), b: (f64, f64)) -> EdgeIn {
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

    #[test]
    fn denetle_finds_a_gap_a_bridge_a_detached_part_and_a_valve_off_the_network() {
        let rules = Rules::from_json(
            &Json::parse(r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"both"}}"#)
                .expect("JSON"),
        )
        .expect("rules");
        let edges = [
            line(1.0, (0.0, 0.0), (100.0, 0.0)),
            // Stops 0.4 m short of the first street.
            line(2.0, (50.0, 50.0), (50.0, 0.4)),
            // Crosses the first street without a node: a bridge.
            line(3.0, (80.0, -20.0), (80.0, 20.0)),
            // Far away, on its own.
            line(4.0, (500.0, 500.0), (520.0, 500.0)),
        ];
        let valve = [JunctionIn {
            id: 9.0,
            p: Vec2::new(300.0, 300.0),
            role: Role::Valve,
            closed: false,
        }];
        let (g, report) = Graph::build(rules, &edges, &valve);
        let c = check(&g, &report);
        let kinds: Vec<ProblemKind> = c.problems.iter().map(|p| p.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ProblemKind::Detached,
                ProblemKind::Detached,
                ProblemKind::Detached,
                ProblemKind::NearMiss,
                ProblemKind::Crossing,
                ProblemKind::OffNetwork
            ],
            "{c:?}"
        );
        let gap = &c.problems[3];
        assert!((gap.value.expect("a gap") - 0.4).abs() < 1e-9);
        assert_eq!(gap.ids, vec![2.0, 1.0]);
        assert_eq!(c.problems[4].at, Vec2::new(80.0, 0.0));
        assert_eq!(c.parts.len(), 4);
        assert_eq!(c.dead_ends, 8);
    }
}

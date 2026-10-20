//! Alt ağlar (docs/adr/0557 §2): what each group of subnetwork controllers (source junctions that carry one name) of a
//! utility network reaches. From the controllers' nodes the walk follows the pieces the way `reach` says (Akış aşağı:
//! the way the edges' directions lead; Bağlı: any way), through devices only along their terminals' paths, never over
//! a closed edge. It stops at a closed junction and at a node of a controller of another group: those are reached, not
//! passed, so a radial system splits cleanly between its sources and a node two groups both reach belongs to both.
//! The apps name the groups, and find what no group reaches and what two reach.

use super::graph::Graph;
use super::terminals::NO_TERMINAL;
use super::trace::{TraceKind, follows};

/// Which way a subnetwork reaches from its controllers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// Akış aşağı.
    Downstream,
    /// Bağlı.
    Connected,
}

/// What one group reaches.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reached {
    /// The edges (indices into the graph's edges) with a piece reached, ascending.
    pub edges: Vec<u32>,
    /// The junctions (indices into the graph's junctions) at a node reached, the controllers' included, ascending.
    pub junctions: Vec<u32>,
    /// The reached pieces' length, each piece once.
    pub length: f64,
}

/// What each of `groups` (each the indices into the graph's junctions of controllers that are one subnetwork) reaches
/// by `reach`, in their order. A controller that is on no edge reaches only itself.
pub fn subnetworks(g: &Graph, groups: &[Vec<u32>], reach: Reach) -> Vec<Reached> {
    let kind = match reach {
        Reach::Downstream => TraceKind::Downstream,
        Reach::Connected => TraceKind::Connected,
    };
    let nodes = g.nodes.len();
    let states = g.terms.as_ref().map_or(nodes, |t| t.states as usize);
    let state = |n: u32, t: u8| match &g.terms {
        None => n as usize,
        Some(ts) => ts.state(n, t),
    };
    // The groups with a controller at each node (the first two; a node of three groups stops all that are not its own).
    const NONE: u32 = u32::MAX;
    let mut at_node: Vec<[u32; 2]> = vec![[NONE; 2]; nodes];
    for (k, group) in groups.iter().enumerate() {
        for &c in group {
            if let Some(n) = g.junctions[c as usize].node {
                let slot = &mut at_node[n as usize];
                let k = k as u32;
                if slot[0] != k && slot[1] != k {
                    if slot[0] == NONE {
                        slot[0] = k;
                    } else {
                        slot[1] = k;
                    }
                }
            }
        }
    }
    let mut junction_at: Vec<Vec<u32>> = vec![Vec::new(); nodes];
    for (k, j) in g.junctions.iter().enumerate() {
        if let Some(n) = j.node {
            junction_at[n as usize].push(k as u32);
        }
    }
    // Stamps instead of cleared flags: each group's walk is its own epoch.
    let mut seen = vec![0u32; states];
    let mut piece_seen = vec![0u32; g.pieces.len()];
    let mut node_seen = vec![0u32; nodes];
    let mut edge_seen = vec![0u32; g.edges.len()];
    let mut out = Vec::with_capacity(groups.len());
    for (k, group) in groups.iter().enumerate() {
        let (epoch, mine) = (k as u32 + 1, k as u32);
        let mut got = Reached::default();
        let mut queue = std::collections::VecDeque::new();
        let mut touched: Vec<u32> = Vec::new();
        for &c in group {
            match g.junctions[c as usize].node {
                None => got.junctions.push(c),
                Some(start) => {
                    if node_seen[start as usize] != epoch {
                        node_seen[start as usize] = epoch;
                        touched.push(start);
                    }
                    let first = state(start, NO_TERMINAL);
                    if seen[first] != epoch {
                        seen[first] = epoch;
                        queue.push_back((start, NO_TERMINAL));
                    }
                }
            }
        }
        while let Some((n, from)) = queue.pop_front() {
            for &arc in g.at_node(n) {
                let (piece, forward) = (arc / 2, arc % 2 == 0);
                if !follows(g, piece, forward, kind) || !g.passes(n, from, arc) {
                    continue;
                }
                if piece_seen[piece as usize] != epoch {
                    piece_seen[piece as usize] = epoch;
                    got.length += g.pieces[piece as usize].len;
                    let e = g.pieces[piece as usize].edge as usize;
                    if edge_seen[e] != epoch {
                        edge_seen[e] = epoch;
                        got.edges.push(e as u32);
                    }
                }
                let next = g.arc_head[arc as usize];
                if node_seen[next as usize] != epoch {
                    node_seen[next as usize] = epoch;
                    touched.push(next);
                }
                let by = g.arrives(arc);
                let owners = at_node[next as usize];
                let others = owners.iter().any(|&o| o != NONE && o != mine);
                let slot = state(next, by);
                if g.kinds[next as usize].closed || others || seen[slot] == epoch {
                    continue;
                }
                seen[slot] = epoch;
                queue.push_back((next, by));
            }
        }
        got.edges.sort_unstable();
        for &n in &touched {
            got.junctions
                .extend(junction_at[n as usize].iter().copied());
        }
        got.junctions.sort_unstable();
        got.junctions.dedup();
        out.push(got);
    }
    out
}

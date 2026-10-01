//! Uçlar and Köşeler (docs/adr/0148 §4): vertices within the tolerance
//! join on a vertex that is there. Vertices within `TOUCH` are one node;
//! the nodes are taken by priority (holding a point, then fixed, then the
//! most objects meeting, then the drawing's order); every fixed node and
//! every mobile one with none in reach becomes a representative; each other
//! mobile node goes to the nearest representative it may join (no path
//! loses its order, no object's two paths meet, no path drops below two
//! distinct vertices, three for a ring or a path that closes). A grid of
//! cells as wide as the reach finds what is near.

use std::collections::{BTreeMap, HashMap};

use super::{Kind, Out, TOUCH, TopoWorks, Work};
use crate::jsmath::js_hypot;
use crate::vec2::Vec2;

/// A vertex: its object, path and index.
type V = (usize, usize, usize);

/// Points by the cell of a grid `size` wide they fall in.
struct Grid {
    size: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl Grid {
    fn new(size: f64) -> Grid {
        Grid {
            size,
            cells: HashMap::new(),
        }
    }

    fn cell(&self, p: Vec2) -> (i64, i64) {
        (
            (p.x / self.size).floor() as i64,
            (p.y / self.size).floor() as i64,
        )
    }

    fn put(&mut self, p: Vec2, item: usize) {
        let c = self.cell(p);
        self.cells.entry(c).or_default().push(item);
    }

    /// The items of the cells around p (what lies within one cell's width of it, and more).
    fn near(&self, p: Vec2, out: &mut Vec<usize>) {
        out.clear();
        let (cx, cy) = self.cell(p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(items) = self.cells.get(&(cx + dx, cy + dy)) {
                    out.extend_from_slice(items);
                }
            }
        }
    }
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(b.x - a.x, b.y - a.y)
}

struct Node {
    members: Vec<V>,
    at: Vec2,
    fixed: bool,
    point: bool,
    degree: usize,
}

impl Node {
    fn priority(&self) -> (u8, isize, V) {
        let class = if self.point {
            0
        } else if self.fixed {
            1
        } else {
            2
        };
        (class, -(self.degree as isize), self.members[0])
    }
}

/// Whether `s` (sorted, without repeats) is a run of a path of `n`
/// vertices: in order for an open path, round the ring for a closed one
/// or for an open path that closes.
fn run_ok(s: &[usize], n: usize, cyclic: bool) -> bool {
    if s.len() < 2 {
        return true;
    }
    if !cyclic {
        return s[s.len() - 1] - s[0] == s.len() - 1;
    }
    let gaps = s.windows(2).filter(|w| w[1] - w[0] > 1).count()
        + usize::from(s[0] + n - s[s.len() - 1] > 1);
    gaps <= 1
}

/// Joins the vertices within `tol` (docs/adr/0148 §4).
pub(super) fn join(objs: &mut [Work], tol: f64, works: TopoWorks, out: &mut Out) {
    let pos = |objs: &[Work], v: V| objs[v.0].paths[v.1].pts[v.2];
    let is_end = |objs: &[Work], v: V| {
        let p = &objs[v.0].paths[v.1];
        objs[v.0].kind.open() && !p.closed && (v.2 == 0 || v.2 + 1 == p.pts.len())
    };
    let mobile = |objs: &[Work], v: V| {
        let o = &objs[v.0];
        if o.fixed || matches!(o.kind, Kind::Point | Kind::Edges) {
            return false;
        }
        (works.ends && is_end(objs, v)) || works.vertices
    };
    let mut verts: Vec<V> = Vec::new();
    for (o, w) in objs.iter().enumerate() {
        if w.kind == Kind::Edges {
            continue;
        }
        for (k, p) in w.paths.iter().enumerate() {
            for i in 0..p.pts.len() {
                verts.push((o, k, i));
            }
        }
    }
    // 1. Nodes: vertices within TOUCH, joined transitively, the first one's place.
    let mut parent: Vec<usize> = (0..verts.len()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let mut grid = Grid::new(TOUCH);
    for (i, &v) in verts.iter().enumerate() {
        grid.put(pos(objs, v), i);
    }
    let mut near = Vec::new();
    for (i, &v) in verts.iter().enumerate() {
        let p = pos(objs, v);
        grid.near(p, &mut near);
        for &j in &near {
            if j > i && dist(p, pos(objs, verts[j])) <= TOUCH {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[a.max(b)] = a.min(b);
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<V>> = BTreeMap::new();
    for i in 0..verts.len() {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(verts[i]);
    }
    let mut nodes: Vec<Node> = groups
        .into_values()
        .map(|members| {
            let fixed = members.iter().any(|&v| !mobile(objs, v));
            let point = members.iter().any(|&v| objs[v.0].kind == Kind::Point);
            let mut seen: Vec<usize> = members.iter().map(|v| v.0).collect();
            seen.dedup();
            let at = pos(objs, members[0]);
            Node {
                degree: seen.len(),
                members,
                at,
                fixed,
                point,
            }
        })
        .collect();
    nodes.sort_by_key(Node::priority);
    // 3. Representatives.
    let mut is_rep = vec![false; nodes.len()];
    let mut reps = Grid::new(tol);
    for (i, n) in nodes.iter().enumerate() {
        if n.fixed {
            is_rep[i] = true;
            reps.put(n.at, i);
        }
    }
    for i in 0..nodes.len() {
        if nodes[i].fixed {
            continue;
        }
        reps.near(nodes[i].at, &mut near);
        if !near.iter().any(|&r| dist(nodes[i].at, nodes[r].at) <= tol) {
            is_rep[i] = true;
            reps.put(nodes[i].at, i);
        }
    }
    // Clusters, one per node to begin with; per path, each cluster's vertex indices.
    let mut members: Vec<Vec<V>> = nodes.iter().map(|n| n.members.clone()).collect();
    let mut sets: HashMap<(usize, usize), BTreeMap<usize, Vec<usize>>> = HashMap::new();
    for (c, n) in nodes.iter().enumerate() {
        for &(o, k, i) in &n.members {
            let entry = sets.entry((o, k)).or_default().entry(c).or_default();
            entry.push(i);
        }
    }
    for by_cluster in sets.values_mut() {
        for s in by_cluster.values_mut() {
            s.sort_unstable();
            s.dedup();
        }
    }
    // 4. Every other mobile node to the nearest representative it may join.
    let mut target: Vec<Option<usize>> = vec![None; nodes.len()];
    for i in 0..nodes.len() {
        if is_rep[i] || nodes[i].fixed {
            continue;
        }
        reps.near(nodes[i].at, &mut near);
        let mut cands: Vec<(f64, usize)> = near
            .iter()
            .map(|&r| (dist(nodes[i].at, nodes[r].at), r))
            .filter(|&(d, _)| d <= tol)
            .collect();
        // Nearest first; equal distance, the higher priority (its place in the order).
        cands.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for &(_, r) in &cands {
            if may_join(objs, &members, &sets, i, r) {
                let moving = std::mem::take(&mut members[i]);
                let touched: Vec<(usize, usize)> = moving.iter().map(|v| (v.0, v.1)).collect();
                members[r].extend(moving);
                for key in touched {
                    if let Some(by_cluster) = sets.get_mut(&key)
                        && let Some(s) = by_cluster.remove(&i)
                    {
                        let joined = by_cluster.entry(r).or_default();
                        joined.extend(s);
                        joined.sort_unstable();
                        joined.dedup();
                    }
                }
                target[i] = Some(r);
                break;
            }
        }
    }
    // 5. Moves, vertex by vertex in the drawing's order.
    let mut moves: Vec<(V, Vec2, Option<f64>)> = Vec::new();
    for (i, n) in nodes.iter().enumerate() {
        let Some(r) = target[i] else { continue };
        let rep = &nodes[r];
        let rz = rep
            .members
            .iter()
            .find_map(|&(o, k, j)| objs[o].paths[k].zs[j]);
        for &v in &n.members {
            moves.push((v, rep.at, rz));
        }
    }
    moves.sort_by_key(|m| m.0);
    for &(v, to, rz) in &moves {
        let (o, k, i) = v;
        let end = is_end(objs, v);
        let old = objs[o].paths[k].pts[i];
        if old != to {
            out.note(if end { "end" } else { "vertex" }, old, to, o);
            if end {
                out.counts.ends += 1;
            } else {
                out.counts.vertices += 1;
            }
        }
        let p = &mut objs[o].paths[k];
        p.pts[i] = to;
        if rz.is_some() {
            p.zs[i] = rz;
        }
        objs[o].changed = true;
    }
    // A changed path drops a vertex that falls on the one before it (a ring's last on its first).
    for w in objs.iter_mut().filter(|w| w.changed) {
        for p in &mut w.paths {
            collapse(p);
        }
    }
}

/// Drops the vertices that fall on the one before them, the next edge's bulge kept.
fn collapse(p: &mut super::TopoPath) {
    let n = p.pts.len();
    if n == 0 {
        return;
    }
    let mut keep: Vec<usize> = vec![0];
    for i in 1..n {
        let last = keep[keep.len() - 1];
        if p.pts[i] == p.pts[last] {
            if let Some(b) = p.bulges.as_mut() {
                b[last] = b[i];
            }
            continue;
        }
        keep.push(i);
    }
    if p.closed && keep.len() > 1 && p.pts[keep[keep.len() - 1]] == p.pts[keep[0]] {
        keep.pop();
    }
    if keep.len() == n {
        return;
    }
    p.pts = keep.iter().map(|&i| p.pts[i]).collect();
    p.zs = keep.iter().map(|&i| p.zs[i]).collect();
    if let Some(b) = p.bulges.as_ref() {
        p.bulges = Some(keep.iter().map(|&i| b[i]).collect());
    }
}

/// Whether node `i` may join cluster `r` (docs/adr/0148 §4.5).
fn may_join(
    objs: &[Work],
    members: &[Vec<V>],
    sets: &HashMap<(usize, usize), BTreeMap<usize, Vec<usize>>>,
    i: usize,
    r: usize,
) -> bool {
    // Two paths of one object never meet.
    let mut paths_of: BTreeMap<usize, usize> = BTreeMap::new();
    for &(o, k, _) in members[r].iter().chain(&members[i]) {
        match paths_of.get(&o) {
            Some(&kk) if kk != k => return false,
            _ => {
                paths_of.insert(o, k);
            }
        }
    }
    let mut touched: Vec<(usize, usize)> = members[i].iter().map(|v| (v.0, v.1)).collect();
    touched.sort_unstable();
    touched.dedup();
    for key in touched {
        let p = &objs[key.0].paths[key.1];
        let n = p.pts.len();
        let Some(by_cluster) = sets.get(&key) else {
            continue;
        };
        let mut joined: Vec<usize> = by_cluster
            .get(&r)
            .into_iter()
            .chain(by_cluster.get(&i))
            .flatten()
            .copied()
            .collect();
        joined.sort_unstable();
        joined.dedup();
        let closes = |s: &[usize]| !p.closed && s.contains(&0) && s.contains(&(n - 1));
        if !run_ok(&joined, n, p.closed || closes(&joined)) {
            return false;
        }
        let others = by_cluster
            .iter()
            .filter(|(c, _)| **c != r && **c != i)
            .map(|(_, s)| s.as_slice());
        let mut merged = joined.len() - 1;
        let mut closing = closes(&joined);
        for s in others {
            merged += s.len() - 1;
            closing |= closes(s);
        }
        let least = if p.closed || closing { 3 } else { 2 };
        if n - merged < least {
            return false;
        }
    }
    true
}

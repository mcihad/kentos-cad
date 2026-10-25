//! Rebuilding a network's graph where its edges changed (docs/adr/0641 §1). A graph built with `build_kept` keeps what
//! its build made: every edge's cuts, every point's node, each edge's unread cost fields, the digests of its edges and
//! junctions, and the cell indexes of the primitives and junction points it was built over (the base).
//! `Graph::rebuild` takes the new edges and junctions whole and gives the graph `build_with` would give them, bit for
//! bit:
//!
//! 1. The new edges and junctions are matched to the kept ones, in order: by their digests (every value they have),
//!    or as the host says (`rebuild_known`: the desktop knows which objects it read again). Those matched are kept,
//!    the others are new; the kept ones not matched are gone.
//! 2. The changed points are the ends of the gone and new edges and the gone and new junction points: only they can
//!    cut an edge differently. A kept edge within the tolerance of one is cut again with the new ones, by the build's
//!    own rule, every end and junction point near them tried in the build's order; every other kept edge keeps its
//!    cuts (its inside level renumbered with it).
//! 3. A node that held a point gone or cut again comes apart; its points and every new point are joined again with
//!    the points within the tolerance of them, the other nodes stay whole (their pairs of points have not changed), and
//!    the nodes are numbered again in the build's order (each where its first point is). The level gaps between two
//!    whole nodes are the kept ones.
//! 4. The graph is put together as a build puts it; an edge that kept its cuts keeps its pieces, their primitives and
//!    their costs as the definition gives them (not as a question's access varied them).
//!
//! A rebuild needs the same rules; with others, or when more than a quarter of the edges or junctions changed since
//! the base was built, it builds anew (and keeps a new base). The base's indexes stay valid for the edges that kept
//! their geometry; the edges changed since are in a small index of their own, made again each rebuild.

use super::build::{
    CellIndex, Cut, Joined, Made, Prep, Reuse, end_of, lay_out, nodes_of, own_cuts, reach,
    start_of, touch, unite,
};
use super::graph::{BuildReport, EdgeIn, Graph, JunctionIn};
use super::rules::Rules;
use super::turns::{Level, TurnIn};
use crate::geom::arrangement::edge_box;
use crate::geom::intersect::Edge;
use crate::geometry::Bounds;
use crate::jsmath::js_hypot;
use crate::vec2::Vec2;

const NONE: u32 = u32::MAX;

/// An edge's or junction's digest: two 64-bit hashes of every value it has.
type Key = (u64, u64);

struct Digest(u64, u64);

impl Digest {
    fn new() -> Digest {
        Digest(0x243f_6a88_85a3_08d3, 0x1319_8a2e_0370_7344)
    }

    fn word(&mut self, w: u64) {
        self.0 = (self.0 ^ w)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .rotate_left(29);
        self.1 = (self.1.rotate_left(17) ^ w).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    }

    fn num(&mut self, v: f64) {
        self.word(v.to_bits());
    }

    fn text(&mut self, t: Option<&str>) {
        let Some(t) = t else {
            self.word(u64::MAX);
            return;
        };
        self.word(t.len() as u64);
        for chunk in t.as_bytes().chunks(8) {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            self.word(u64::from_le_bytes(w));
        }
    }

    fn texts(&mut self, list: &[Option<String>]) {
        self.word(list.len() as u64);
        for t in list {
            self.text(t.as_deref());
        }
    }

    fn key(self) -> Key {
        let mix = |mut h: u64| {
            h ^= h >> 33;
            h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
            h ^= h >> 33;
            h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
            h ^ (h >> 33)
        };
        (mix(self.0), mix(self.1))
    }
}

fn edge_key(e: &EdgeIn) -> Key {
    let mut d = Digest::new();
    d.num(e.id);
    d.word(u64::from(e.part));
    d.word(e.path.len() as u64);
    for p in &e.path {
        match *p {
            Edge::Seg { a, b } => {
                d.word(1);
                for v in [a.x, a.y, b.x, b.y] {
                    d.num(v);
                }
            }
            Edge::Arc { c, r, a0, sweep } => {
                d.word(2);
                for v in [c.x, c.y, r, a0, sweep] {
                    d.num(v);
                }
            }
        }
    }
    d.text(e.direction.as_deref());
    d.texts(&e.costs);
    d.word(u64::from(e.closed));
    d.texts(&e.levels);
    d.word(e.periods.len() as u64);
    for p in &e.periods {
        d.texts(p);
    }
    d.texts(&e.terminals);
    d.texts(&e.facts);
    d.key()
}

fn junction_key(j: &JunctionIn) -> Key {
    let mut d = Digest::new();
    d.num(j.id);
    d.num(j.p.x);
    d.num(j.p.y);
    d.word(j.role as u64);
    d.word(u64::from(j.closed));
    d.text(j.level.as_deref());
    d.word(j.device.map_or(u64::MAX, u64::from));
    d.key()
}

/// What a build keeps for a rebuild (see the module).
pub(crate) struct Kept {
    rules: Rules,
    /// The edges' and junctions' digests; none when the host matched them itself (`rebuild_known`).
    edge_keys: Option<Vec<Key>>,
    junction_keys: Option<Vec<Key>>,
    /// Each edge's unread cost fields, `(edge, cost)` in the build's order.
    unread: Vec<(u32, u32)>,
    /// Each edge's cuts, `cuts[cut_start[i]..cut_start[i + 1]]`, and every point's node (the cuts, then the junctions).
    cut_start: Vec<u32>,
    cuts: Vec<Cut>,
    node_of: Vec<u32>,
    junction_p: Vec<Vec2>,
    /// The base: the primitives and junction points of the last whole build, in cell indexes, and which edge and
    /// junction of now each base one is (none: changed since).
    base_owner: Vec<(u32, u32)>,
    base_near: CellIndex,
    base_edge_now: Vec<u32>,
    base_junctions: CellIndex,
    base_junction_now: Vec<u32>,
}

impl Kept {
    /// What a whole build keeps: everything is the base.
    pub(super) fn new(
        rules: &Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        made: Made,
        owner: Vec<(u32, u32)>,
        near: CellIndex,
        unread: Vec<(u32, u32)>,
    ) -> Kept {
        let junction_boxes: Vec<(u32, Bounds)> = junctions
            .iter()
            .enumerate()
            .map(|(j, jn)| (j as u32, point_box(jn.p)))
            .collect();
        Kept {
            rules: rules.clone(),
            edge_keys: Some(edges.iter().map(edge_key).collect()),
            junction_keys: Some(junctions.iter().map(junction_key).collect()),
            unread,
            cut_start: made.cut_start,
            cuts: made.cuts,
            node_of: made.joined.node_of,
            junction_p: junctions.iter().map(|j| j.p).collect(),
            base_owner: owner,
            base_near: near,
            base_edge_now: (0..edges.len() as u32).collect(),
            base_junctions: CellIndex::new(&junction_boxes, reach(rules.tolerance)),
            base_junction_now: (0..junctions.len() as u32).collect(),
        }
    }

    /// The bytes it holds, about.
    pub fn bytes(&self) -> usize {
        self.edge_keys.as_ref().map_or(0, |k| k.len() * 16)
            + self.junction_keys.as_ref().map_or(0, |k| k.len() * 16)
            + self.unread.len() * 8
            + self.cut_start.len() * 4
            + self.cuts.len() * std::mem::size_of::<Cut>()
            + self.node_of.len() * 4
            + self.junction_p.len() * 16
            + self.base_owner.len() * 8
            + self.base_near.bytes()
            + self.base_edge_now.len() * 4
            + self.base_junctions.bytes()
            + self.base_junction_now.len() * 4
    }

    /// Edge `o`'s own ends, kept: its first and last own cuts.
    fn ends_of(&self, o: usize) -> Vec<Vec2> {
        let cs = &self.cuts[self.cut_start[o] as usize..self.cut_start[o + 1] as usize];
        let mut own = cs.iter().filter(|c| c.own);
        match (own.next(), own.next_back()) {
            (Some(a), Some(b)) => vec![a.p, b.p],
            (Some(a), None) => vec![a.p],
            _ => Vec::new(),
        }
    }
}

fn point_box(p: Vec2) -> Bounds {
    Bounds {
        min_x: p.x,
        min_y: p.y,
        max_x: p.x,
        max_y: p.y,
    }
}

/// What a rebuild did: built anew, or how many edges it cut again and points it joined again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rebuilt {
    pub anew: bool,
    pub recut: usize,
    pub rejoined: usize,
}

/// The new list's items matched to the kept keys, in order: each new one's kept index (none: new). Equal keys side by
/// side are matched; past a difference the next 64 of either side are looked at for the other's key (items gone or
/// new), else both count as changed.
fn matched(kept: &[Key], new: &[Key]) -> Vec<u32> {
    const LOOK: usize = 64;
    let mut out = vec![NONE; new.len()];
    let (mut i, mut o) = (0, 0);
    while i < new.len() && o < kept.len() {
        if new[i] == kept[o] {
            out[i] = o as u32;
            i += 1;
            o += 1;
        } else if let Some(d) = (1..LOOK).find(|d| kept.get(o + d) == Some(&new[i])) {
            o += d;
        } else if let Some(d) = (1..LOOK).find(|d| new.get(i + d) == Some(&kept[o])) {
            i += d;
        } else {
            i += 1;
            o += 1;
        }
    }
    out
}

/// The kept items' new places from the new items' kept ones (none: gone); a kept index out of order or twice counts
/// as new.
fn in_order(was: &mut [u32], kept: usize) -> Vec<u32> {
    let mut now = vec![NONE; kept];
    let mut last: i64 = -1;
    for (i, w) in was.iter_mut().enumerate() {
        if *w == NONE {
            continue;
        }
        if i64::from(*w) <= last || *w as usize >= kept {
            *w = NONE;
            continue;
        }
        last = i64::from(*w);
        now[*w as usize] = i as u32;
    }
    now
}

impl Graph {
    /// The graph of the new `edges`, `junctions` and `turns` by `rules`: the same as `build_with` gives, rebuilt from
    /// this graph's kept build where it can, the edges and junctions matched by their digests (see the module). The
    /// new graph keeps its own for the next rebuild; this one gives its up (it is the old one, let go after).
    pub fn rebuild(
        &mut self,
        rules: Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        turns: &[TurnIn],
    ) -> (Graph, BuildReport, Rebuilt) {
        let keys = self
            .kept
            .as_deref()
            .and_then(|k| Some((k.edge_keys.as_deref()?, k.junction_keys.as_deref()?)));
        let Some((old_edges, old_junctions)) = keys else {
            return Self::anew(rules, edges, junctions, turns);
        };
        let edge_keys: Vec<Key> = edges.iter().map(edge_key).collect();
        let junction_keys: Vec<Key> = junctions.iter().map(junction_key).collect();
        let edge_was = matched(old_edges, &edge_keys);
        let junction_was = matched(old_junctions, &junction_keys);
        self.rebuild_matched(
            rules,
            edges,
            junctions,
            turns,
            edge_was,
            junction_was,
            Some((edge_keys, junction_keys)),
        )
    }

    /// As `rebuild`, the host saying which kept edge and junction each new one is (`edge_was[i]`: the kept edge's
    /// index, `u32::MAX`: new or changed): a host that keeps its input between builds and reads again only what
    /// changed knows it (the desktop's network thread, docs/adr/0641 §3). One said out of order counts as new.
    pub fn rebuild_known(
        &mut self,
        rules: Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        turns: &[TurnIn],
        edge_was: Vec<u32>,
        junction_was: Vec<u32>,
    ) -> (Graph, BuildReport, Rebuilt) {
        if edge_was.len() != edges.len() || junction_was.len() != junctions.len() {
            return Self::anew(rules, edges, junctions, turns);
        }
        self.rebuild_matched(rules, edges, junctions, turns, edge_was, junction_was, None)
    }

    fn anew(
        rules: Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        turns: &[TurnIn],
    ) -> (Graph, BuildReport, Rebuilt) {
        let (g, r) = Self::build_kept(rules, edges, junctions, turns);
        (
            g,
            r,
            Rebuilt {
                anew: true,
                ..Rebuilt::default()
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn rebuild_matched(
        &mut self,
        rules: Rules,
        edges: &[EdgeIn],
        junctions: &[JunctionIn],
        turns: &[TurnIn],
        mut old_edge: Vec<u32>,
        mut old_junction: Vec<u32>,
        keys: Option<(Vec<Key>, Vec<Key>)>,
    ) -> (Graph, BuildReport, Rebuilt) {
        let Some(k) = self.kept.take() else {
            return Self::anew(rules, edges, junctions, turns);
        };
        let k = *k;
        if k.rules != rules {
            return Self::anew(rules, edges, junctions, turns);
        }
        let tol = rules.tolerance;
        let kept_edges = k.cut_start.len() - 1;
        let new_edge = in_order(&mut old_edge, kept_edges);
        let new_junction = in_order(&mut old_junction, k.junction_p.len());
        // The edges and junctions of now not in the base.
        let base_edge_now: Vec<u32> = k
            .base_edge_now
            .iter()
            .map(|&o| {
                if o == NONE {
                    NONE
                } else {
                    new_edge[o as usize]
                }
            })
            .collect();
        let base_junction_now: Vec<u32> = k
            .base_junction_now
            .iter()
            .map(|&o| {
                if o == NONE {
                    NONE
                } else {
                    new_junction[o as usize]
                }
            })
            .collect();
        let mut in_base = vec![false; edges.len()];
        for &i in &base_edge_now {
            if i != NONE {
                in_base[i as usize] = true;
            }
        }
        let extra_edges: Vec<u32> = (0..edges.len() as u32)
            .filter(|&i| !in_base[i as usize])
            .collect();
        let mut j_in_base = vec![false; junctions.len()];
        for &j in &base_junction_now {
            if j != NONE {
                j_in_base[j as usize] = true;
            }
        }
        let extra_junctions: Vec<u32> = (0..junctions.len() as u32)
            .filter(|&j| !j_in_base[j as usize])
            .collect();
        if extra_edges.len() * 4 > edges.len().max(16)
            || extra_junctions.len() * 4 > junctions.len().max(16)
        {
            return Self::anew(rules, edges, junctions, turns);
        }
        let mut report = BuildReport::default();
        let prep = Prep::new(&rules, edges, junctions, &mut report);
        // 2. The changed points: the ends of the gone and new edges, the gone and new junction points.
        let mut changed: Vec<Vec2> = Vec::new();
        for (o, &i) in new_edge.iter().enumerate() {
            if i == NONE {
                changed.extend(k.ends_of(o));
            }
        }
        for (i, e) in edges.iter().enumerate() {
            if old_edge[i] == NONE {
                changed.extend(start_of(e));
                changed.extend(end_of(e));
            }
        }
        for (o, &j) in new_junction.iter().enumerate() {
            if j == NONE {
                changed.push(k.junction_p[o]);
            }
        }
        for (j, jn) in junctions.iter().enumerate() {
            if old_junction[j] == NONE {
                changed.push(jn.p);
            }
        }
        // The extra edges' primitives and junction points in indexes of their own.
        let mut extra_owner: Vec<(u32, u32)> = Vec::new();
        let mut boxes: Vec<(u32, Bounds)> = Vec::new();
        for &i in &extra_edges {
            for (kk, p) in edges[i as usize].path.iter().enumerate() {
                boxes.push((extra_owner.len() as u32, edge_box(p)));
                extra_owner.push((i, kk as u32));
            }
        }
        let extra_near = CellIndex::new(&boxes, reach(tol));
        let junction_boxes: Vec<(u32, Bounds)> = extra_junctions
            .iter()
            .map(|&j| (j, point_box(junctions[j as usize].p)))
            .collect();
        let extra_junction_near = CellIndex::new(&junction_boxes, reach(tol));
        // The edges near a place or a box, now: the base's that kept their geometry and the extra ones.
        let mut found: Vec<u32> = Vec::new();
        let mut near_edges = |at: Result<Vec2, &Bounds>, out: &mut Vec<u32>| {
            found.clear();
            match at {
                Ok(p) => k.base_near.at(p, &mut found),
                Err(b) => k.base_near.within(b, &mut found),
            }
            for &q in &found {
                let i = base_edge_now[k.base_owner[q as usize].0 as usize];
                if i != NONE {
                    out.push(i);
                }
            }
            found.clear();
            match at {
                Ok(p) => extra_near.at(p, &mut found),
                Err(b) => extra_near.within(b, &mut found),
            }
            out.extend(found.iter().map(|&q| extra_owner[q as usize].0));
        };
        let mut jfound: Vec<u32> = Vec::new();
        let mut near_junctions = |at: Result<Vec2, &Bounds>, out: &mut Vec<u32>| {
            jfound.clear();
            match at {
                Ok(p) => k.base_junctions.at(p, &mut jfound),
                Err(b) => k.base_junctions.within(b, &mut jfound),
            }
            for &o in &jfound {
                let j = base_junction_now[o as usize];
                if j != NONE {
                    out.push(j);
                }
            }
            jfound.clear();
            match at {
                Ok(p) => extra_junction_near.at(p, &mut jfound),
                Err(b) => extra_junction_near.within(b, &mut jfound),
            }
            out.extend_from_slice(&jfound);
        };
        let mut recut = vec![false; edges.len()];
        for (i, &o) in old_edge.iter().enumerate() {
            if o == NONE {
                recut[i] = true;
            }
        }
        let mut nearby: Vec<u32> = Vec::new();
        for &c in &changed {
            nearby.clear();
            near_edges(Ok(c), &mut nearby);
            for &i in &nearby {
                recut[i as usize] = true;
            }
        }
        let recut_list: Vec<u32> = (0..edges.len() as u32)
            .filter(|&i| recut[i as usize])
            .collect();
        // 2. The cuts of the edges cut again, by the build's rule: their own, then every end and junction point near
        // them tried in the build's order.
        let mut pushed: Vec<(u32, Cut)> = Vec::new();
        for &i in &recut_list {
            own_cuts(&rules, edges, &prep, i as usize, &mut pushed);
        }
        let mut recut_owner: Vec<(u32, u32)> = Vec::new();
        let mut boxes: Vec<(u32, Bounds)> = Vec::new();
        for &i in &recut_list {
            for (kk, p) in edges[i as usize].path.iter().enumerate() {
                boxes.push((recut_owner.len() as u32, edge_box(p)));
                recut_owner.push((i, kk as u32));
            }
        }
        let grow = reach(tol);
        let mut toucher_edges: Vec<u32> = Vec::new();
        let mut toucher_junctions: Vec<u32> = Vec::new();
        for (_, b) in &boxes {
            let b = Bounds {
                min_x: b.min_x - grow,
                min_y: b.min_y - grow,
                max_x: b.max_x + grow,
                max_y: b.max_y + grow,
            };
            near_edges(Err(&b), &mut toucher_edges);
            near_junctions(Err(&b), &mut toucher_junctions);
        }
        toucher_edges.sort_unstable();
        toucher_edges.dedup();
        toucher_junctions.sort_unstable();
        toucher_junctions.dedup();
        let recut_near = CellIndex::new(&boxes, grow);
        let mut hits: Vec<u32> = Vec::new();
        for &i in &toucher_edges {
            let (i, e) = (i as usize, &edges[i as usize]);
            for (s, end, level) in [
                (0.0, start_of(e), prep.levels[i][0]),
                (prep.lengths[i], end_of(e), prep.levels[i][1]),
            ] {
                if let Some(p) = end {
                    recut_near.at(p, &mut hits);
                    if hits.is_empty() {
                        continue;
                    }
                    hits.sort_unstable();
                    touch(
                        edges,
                        &prep,
                        tol,
                        &recut_owner,
                        &hits,
                        p,
                        level,
                        Some((i, s)),
                        &mut pushed,
                    );
                }
            }
        }
        for &j in &toucher_junctions {
            let jn = &junctions[j as usize];
            recut_near.at(jn.p, &mut hits);
            if hits.is_empty() {
                continue;
            }
            hits.sort_unstable();
            touch(
                edges,
                &prep,
                tol,
                &recut_owner,
                &hits,
                jn.p,
                prep.junction_levels[j as usize],
                None,
                &mut pushed,
            );
        }
        let (fresh_start, fresh) = lay_out(edges.len(), pushed);
        // Every edge's cuts: the kept ones (its own inside level renumbered with it) or the new ones; each point's
        // kept point, none for a new one.
        let mut cut_start: Vec<u32> = Vec::with_capacity(edges.len() + 1);
        let mut cuts: Vec<Cut> = Vec::with_capacity(k.cuts.len() + fresh.len());
        let mut was: Vec<u32> = Vec::with_capacity(k.cuts.len() + junctions.len());
        for i in 0..edges.len() {
            cut_start.push(cuts.len() as u32);
            if recut[i] {
                cuts.extend_from_slice(
                    &fresh[fresh_start[i] as usize..fresh_start[i + 1] as usize],
                );
                was.resize(cuts.len(), NONE);
                continue;
            }
            let o = old_edge[i] as usize;
            let (a, b) = (k.cut_start[o] as usize, k.cut_start[o + 1] as usize);
            for (w, c) in k.cuts[a..b].iter().enumerate() {
                let mut c = *c;
                if let Level::Own(_) = c.level {
                    c.level = Level::Own(i as u32);
                }
                cuts.push(c);
                was.push((a + w) as u32);
            }
        }
        cut_start.push(cuts.len() as u32);
        let kept_cuts = k.cuts.len() as u32;
        for &o in &old_junction {
            was.push(if o == NONE { NONE } else { kept_cuts + o });
        }
        let mut points: Vec<Vec2> = Vec::with_capacity(was.len());
        let mut point_levels: Vec<Level> = Vec::with_capacity(was.len());
        points.extend(cuts.iter().map(|c| c.p));
        point_levels.extend(cuts.iter().map(|c| c.level));
        points.extend(junctions.iter().map(|j| j.p));
        point_levels.extend(prep.junction_levels.iter().copied());
        // 3. The nodes that come apart: those of a kept point not carried over (its edge gone or cut again, its
        // junction gone).
        let old_nodes = self.nodes.len();
        let mut apart = vec![false; old_nodes];
        let mut carried = vec![false; k.node_of.len()];
        for &w in &was {
            if w != NONE {
                carried[w as usize] = true;
            }
        }
        for (w, &c) in carried.iter().enumerate() {
            if !c {
                apart[k.node_of[w] as usize] = true;
            }
        }
        let mut parent: Vec<u32> = (0..points.len() as u32).collect();
        let mut first = vec![NONE; old_nodes];
        let mut again: Vec<u32> = Vec::new();
        for (i, &w) in was.iter().enumerate() {
            if w != NONE && !apart[k.node_of[w as usize] as usize] {
                let n = k.node_of[w as usize] as usize;
                if first[n] == NONE {
                    first[n] = i as u32;
                }
                parent[i] = first[n];
            } else {
                again.push(i as u32);
            }
        }
        // Each point joined again with the points within the tolerance of it: the cuts of the edges near it and the
        // junction points near it.
        let first_junction = cuts.len() as u32;
        let mut level_pairs: Vec<(u32, u32)> = Vec::new();
        let mut candidates: Vec<u32> = Vec::new();
        let mut near_j: Vec<u32> = Vec::new();
        for &a in &again {
            let p = points[a as usize];
            nearby.clear();
            near_edges(Ok(p), &mut nearby);
            nearby.sort_unstable();
            nearby.dedup();
            candidates.clear();
            for &i in &nearby {
                candidates.extend(cut_start[i as usize]..cut_start[i as usize + 1]);
            }
            near_j.clear();
            near_junctions(Ok(p), &mut near_j);
            candidates.extend(near_j.iter().map(|&j| first_junction + j));
            for &b in &candidates {
                if b == a {
                    continue;
                }
                let q = points[b as usize];
                if js_hypot(q.x - p.x, q.y - p.y) > tol {
                    continue;
                }
                if !point_levels[a as usize].meets(point_levels[b as usize]) {
                    level_pairs.push((a.min(b), a.max(b)));
                    continue;
                }
                unite(&mut parent, a, b);
            }
        }
        // The level gaps between two whole nodes are the kept ones, by their first points now.
        for n in 0..old_nodes {
            if apart[n] || first[n] == NONE {
                continue;
            }
            let (s, e) = match (self.gap_start.get(n), self.gap_start.get(n + 1)) {
                (Some(&s), Some(&e)) => (s as usize, e as usize),
                _ => continue,
            };
            for &m in &self.gap_to[s..e] {
                let m = m as usize;
                if !apart[m] && first[m] != NONE {
                    level_pairs.push((first[n], first[m]));
                }
            }
        }
        let rejoined = again.len();
        let joined: Joined = nodes_of(&points, &mut parent, &level_pairs);
        let made = Made {
            prep,
            cut_start,
            cuts,
            joined,
        };
        // 4. Put together, the edges that kept their cuts keeping their pieces.
        let mut piece_start = vec![0u32; kept_edges + 1];
        for p in &self.pieces {
            piece_start[p.edge as usize + 1] += 1;
        }
        for o in 0..kept_edges {
            piece_start[o + 1] += piece_start[o];
        }
        let mut unread_start = vec![0u32; kept_edges + 1];
        for &(o, _) in &k.unread {
            unread_start[o as usize + 1] += 1;
        }
        for o in 0..kept_edges {
            unread_start[o + 1] += unread_start[o];
        }
        let carry: Vec<u32> = (0..edges.len())
            .map(|i| if recut[i] { NONE } else { old_edge[i] })
            .collect();
        let reuse = Reuse {
            old: self,
            piece_start,
            carry,
            unread: &k.unread,
            unread_start,
        };
        let (mut g, report, unread) = Self::put_together(
            rules.clone(),
            edges,
            junctions,
            turns,
            &made,
            report,
            Some(&reuse),
        );
        let (edge_keys, junction_keys) = match keys {
            Some((e, j)) => (Some(e), Some(j)),
            None => (None, None),
        };
        g.kept = Some(Box::new(Kept {
            rules,
            edge_keys,
            junction_keys,
            unread,
            cut_start: made.cut_start,
            cuts: made.cuts,
            node_of: made.joined.node_of,
            junction_p: junctions.iter().map(|j| j.p).collect(),
            base_owner: k.base_owner,
            base_near: k.base_near,
            base_edge_now,
            base_junctions: k.base_junctions,
            base_junction_now,
        }));
        (
            g,
            report,
            Rebuilt {
                anew: false,
                recut: recut_list.len(),
                rejoined,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::Json;
    use crate::geom::bulge::bulge_path_edges;
    use crate::ops::network::graph::Role;

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> f64 {
            // xorshift64*: the same edits every run.
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
        }

        fn below(&mut self, n: usize) -> usize {
            ((self.next() * n as f64) as usize).min(n.saturating_sub(1))
        }

        fn pick<'a>(&mut self, list: &[&'a str]) -> &'a str {
            list[self.below(list.len())]
        }
    }

    fn rules(text: &str) -> Rules {
        Rules::from_json(&Json::parse(text).expect("JSON")).expect("rules")
    }

    /// The three definitions: ends at a hair, vertices with levels, ends at two metres (long chains of joined points).
    fn definitions() -> Vec<Rules> {
        vec![
            rules(
                r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"field","field":"yon","forward":["FT"],"backward":["TF"],"closed":["N"]},
                    "costs":[{"name":"Süre","kind":"speed","field":"hiz","speed":50}],
                    "closures":{"from":"kapanis_bas","to":"kapanis_son"}}"#,
            ),
            rules(
                r#"{"connect":"vertices","tolerance":0.3,"direction":{"kind":"both"},"levels":{"start":"a","end":"b","inside":"c"},
                    "costs":[{"name":"Süre","kind":"speed","field":"hiz","speed":50}]}"#,
            ),
            rules(
                r#"{"connect":"ends","tolerance":2.0,"direction":{"kind":"both"},"levels":{"start":"a","end":"b"},
                    "costs":[{"name":"Bedel","kind":"field","field":"bedel","unit":"TL"}]}"#,
            ),
        ]
    }

    fn value(r: &mut Rng, list: &[&str]) -> Option<String> {
        let v = r.pick(list);
        (!v.is_empty()).then(|| v.to_owned())
    }

    fn edge(r: &mut Rng, id: f64, pts: &[Vec2]) -> EdgeIn {
        let bulges: Vec<f64> = (0..pts.len())
            .map(|_| if r.next() < 0.15 { r.next() - 0.5 } else { 0.0 })
            .collect();
        EdgeIn {
            id,
            part: 0,
            path: bulge_path_edges(pts, Some(&bulges), false),
            direction: value(r, &["", "", "FT", "TF", "N"]),
            costs: vec![value(r, &["30", "50", "70", "", "abc"])],
            closed: r.next() < 0.05,
            levels: [
                value(r, &["", "0", "1", "1", "x"]),
                value(r, &["", "0", "1", "1"]),
                value(r, &["", "", "0", "1", "2"]),
            ],
            periods: Vec::new(),
            terminals: [None, None],
            facts: vec![
                value(r, &["", "", "2026-11-01", "2026-11-03"]),
                value(r, &["", "2026-11-05", "2026-11-02"]),
            ],
        }
    }

    /// A place: a grid crossing nudged within (or a little past) the tolerance, a place along a street, or anywhere.
    fn place(r: &mut Rng, tol: f64) -> Vec2 {
        let (i, j) = (r.below(7) as f64, r.below(7) as f64);
        let nudge = |r: &mut Rng| (r.next() - 0.5) * 2.4 * tol;
        match r.below(4) {
            0 | 1 => Vec2::new(i * 10.0 + nudge(r), j * 10.0 + nudge(r)),
            2 => Vec2::new(i * 10.0 + r.next() * 10.0, j * 10.0),
            _ => Vec2::new(r.next() * 60.0, r.next() * 60.0),
        }
    }

    fn street(r: &mut Rng, id: f64, tol: f64) -> EdgeIn {
        let a = place(r, tol);
        let b = if r.next() < 0.5 {
            Vec2::new(a.x + 10.0, a.y)
        } else {
            place(r, tol)
        };
        let pts = if r.next() < 0.3 {
            vec![
                a,
                Vec2::new((a.x + b.x) / 2.0 + 1.0, (a.y + b.y) / 2.0 - 1.0),
                b,
            ]
        } else {
            vec![a, b]
        };
        edge(r, id, &pts)
    }

    fn junction(r: &mut Rng, id: f64, tol: f64) -> JunctionIn {
        JunctionIn {
            id,
            p: place(r, tol),
            role: [Role::Junction, Role::Valve, Role::Source][r.below(3)],
            closed: r.next() < 0.2,
            level: value(r, &["", "", "0", "1", "y"]),
            device: None,
        }
    }

    /// The city: a 7 × 7 grid of streets (some bent, some arcs) and a few more, and junction points.
    fn city(r: &mut Rng, tol: f64) -> (Vec<EdgeIn>, Vec<JunctionIn>) {
        let mut edges = Vec::new();
        for i in 0..7 {
            for j in 0..7 {
                let at = |i: usize, j: usize| Vec2::new(i as f64 * 10.0, j as f64 * 10.0);
                if i + 1 < 7 {
                    let e = edge(r, edges.len() as f64 + 1.0, &[at(i, j), at(i + 1, j)]);
                    edges.push(e);
                }
                if j + 1 < 7 {
                    let e = edge(r, edges.len() as f64 + 1.0, &[at(i, j), at(i, j + 1)]);
                    edges.push(e);
                }
            }
        }
        for _ in 0..12 {
            let e = street(r, edges.len() as f64 + 1.0, tol);
            edges.push(e);
        }
        let junctions = (0..10)
            .map(|k| junction(r, 1000.0 + k as f64, tol))
            .collect();
        (edges, junctions)
    }

    /// One edit of the drawing, as a user's write gives it.
    /// The drawing as the test keeps it: each edge and junction with a tag that changes when it is edited (what the
    /// desktop knows of the objects it reads again).
    struct Drawing {
        edges: Vec<EdgeIn>,
        junctions: Vec<JunctionIn>,
        edge_tags: Vec<u64>,
        junction_tags: Vec<u64>,
        next_tag: u64,
    }

    impl Drawing {
        fn tag(&mut self) -> u64 {
            self.next_tag += 1;
            self.next_tag
        }

        /// Each item's place in `before` by its tag (none: edited or new).
        fn was(now: &[u64], before: &[u64]) -> Vec<u32> {
            let at: std::collections::HashMap<u64, u32> = before
                .iter()
                .enumerate()
                .map(|(i, t)| (*t, i as u32))
                .collect();
            now.iter()
                .map(|t| at.get(t).copied().unwrap_or(NONE))
                .collect()
        }
    }

    fn edit_tagged(r: &mut Rng, d: &mut Drawing, next_id: &mut f64, tol: f64) {
        let (edges_before, junctions_before) = (d.edges.clone(), d.junctions.clone());
        let (tags, jtags) = (d.edge_tags.clone(), d.junction_tags.clone());
        edit(r, &mut d.edges, &mut d.junctions, next_id, tol);
        // The tags follow the items: an item equal to the one at a place keeps its tag, else a new one.
        let retag =
            |now: &[EdgeIn], before: &[EdgeIn], tags: &[u64], d: &mut Drawing| -> Vec<u64> {
                let mut out = Vec::with_capacity(now.len());
                for e in now {
                    match before.iter().position(|b| b == e) {
                        Some(k) => out.push(tags[k]),
                        None => out.push(d.tag()),
                    }
                }
                out
            };
        let now = d.edges.clone();
        let edge_tags = retag(&now, &edges_before, &tags, d);
        let mut junction_tags = Vec::with_capacity(d.junctions.len());
        for j in d.junctions.clone() {
            match junctions_before.iter().position(|b| *b == j) {
                Some(k) => junction_tags.push(jtags[k]),
                None => junction_tags.push(d.tag()),
            }
        }
        d.edge_tags = edge_tags;
        d.junction_tags = junction_tags;
    }

    fn edit(
        r: &mut Rng,
        edges: &mut Vec<EdgeIn>,
        junctions: &mut Vec<JunctionIn>,
        next_id: &mut f64,
        tol: f64,
    ) {
        match r.below(10) {
            // An edge's end moved: near a crossing, onto a street, onto another edge's end exactly.
            0 | 1 if !edges.is_empty() => {
                let i = r.below(edges.len());
                let mut pts: Vec<Vec2> = vec![
                    crate::geom::intersect::point_at(&edges[i].path[0], 0.0),
                    crate::geom::intersect::point_at(edges[i].path.last().expect("path"), 1.0),
                ];
                let to = if r.next() < 0.3 && edges.len() > 1 {
                    let o = &edges[r.below(edges.len())];
                    crate::geom::intersect::point_at(&o.path[0], 0.0)
                } else {
                    place(r, tol)
                };
                let k = r.below(2);
                pts[k] = to;
                let id = edges[i].id;
                edges[i] = edge(r, id, &pts);
            }
            // An edge's values changed.
            2 if !edges.is_empty() => {
                let i = r.below(edges.len());
                let pts: Vec<Vec2> = edges[i]
                    .path
                    .iter()
                    .map(|p| crate::geom::intersect::point_at(p, 0.0))
                    .chain(std::iter::once(crate::geom::intersect::point_at(
                        edges[i].path.last().expect("path"),
                        1.0,
                    )))
                    .collect();
                let mut e = edge(r, edges[i].id, &pts);
                e.path = edges[i].path.clone();
                edges[i] = e;
            }
            3 if !edges.is_empty() => {
                let i = r.below(edges.len());
                edges.remove(i);
            }
            4 | 5 => {
                let e = street(r, *next_id, tol);
                *next_id += 1.0;
                let at = r.below(edges.len() + 1);
                edges.insert(at, e);
            }
            // Two edges swap places in the order.
            6 if edges.len() > 2 => {
                let (a, b) = (r.below(edges.len()), r.below(edges.len()));
                edges.swap(a, b);
            }
            7 => {
                let j = junction(r, *next_id, tol);
                *next_id += 1.0;
                let at = r.below(junctions.len() + 1);
                junctions.insert(at, j);
            }
            8 if !junctions.is_empty() => {
                let k = r.below(junctions.len());
                junctions[k].p = place(r, tol);
            }
            _ if !junctions.is_empty() => {
                let k = r.below(junctions.len());
                junctions.remove(k);
            }
            _ => {}
        }
    }

    /// Where two graphs differ, or none.
    fn differs(a: &Graph, b: &Graph) -> Option<&'static str> {
        let mut found = Vec::new();
        let all = Bounds {
            min_x: -1e9,
            min_y: -1e9,
            max_x: 1e9,
            max_y: 1e9,
        };
        a.tree.search(&all, &mut found);
        let mut found_b = Vec::new();
        b.tree.search(&all, &mut found_b);
        let checks: [(&'static str, bool); 25] = [
            ("nodes", a.nodes == b.nodes),
            ("kinds", a.kinds == b.kinds),
            ("pieces", a.pieces == b.pieces),
            ("geom", a.geom == b.geom),
            ("geom_at", a.geom_at == b.geom_at),
            ("geom_piece", a.geom_piece == b.geom_piece),
            ("edges", a.edges == b.edges),
            ("junctions", a.junctions == b.junctions),
            ("costs", a.costs == b.costs),
            ("fw", a.fw == b.fw),
            ("bw", a.bw == b.bw),
            ("adj_start", a.adj_start == b.adj_start),
            ("adj", a.adj == b.adj),
            ("arc_head", a.arc_head == b.arc_head),
            ("tree", found == found_b),
            ("turns", a.turns == b.turns),
            ("clocks", a.clocks == b.clocks),
            ("terms", a.terms == b.terms),
            ("facts", a.facts == b.facts),
            ("gap_start", a.gap_start == b.gap_start),
            ("gap_to", a.gap_to == b.gap_to),
            ("part", a.part == b.part),
            ("part_edges", a.part_edges == b.part_edges),
            ("closing", a.closing == b.closing),
            ("rules", a.rules == b.rules),
        ];
        checks.iter().find(|(_, same)| !same).map(|(what, _)| *what)
    }

    /// Where two graphs' kept builds differ in what a whole build would keep (the base aside), or none.
    fn kept_differs(a: &Graph, b: &Graph, keys: bool) -> Option<&'static str> {
        let (Some(a), Some(b)) = (a.kept.as_deref(), b.kept.as_deref()) else {
            return Some("kept");
        };
        let checks: [(&'static str, bool); 7] = [
            ("kept.edge_keys", !keys || a.edge_keys == b.edge_keys),
            (
                "kept.junction_keys",
                !keys || a.junction_keys == b.junction_keys,
            ),
            ("kept.unread", a.unread == b.unread),
            ("kept.cut_start", a.cut_start == b.cut_start),
            ("kept.cuts", a.cuts == b.cuts),
            ("kept.node_of", a.node_of == b.node_of),
            ("kept.junction_p", a.junction_p == b.junction_p),
        ];
        checks.iter().find(|(_, same)| !same).map(|(what, _)| *what)
    }

    /// A rebuild after each of many random edits is the graph a whole build gives the edited drawing, field for
    /// field, and says the same (docs/adr/0641 §1).
    #[test]
    fn a_rebuild_is_the_whole_build_after_random_edits() {
        let (mut anew, mut partly, mut recut) = (0, 0, 0);
        for (n, rules) in definitions().into_iter().enumerate() {
            let tol = rules.tolerance;
            for seed in 0..8u64 {
                // Half the seeds rebuild as the desktop does, saying which items it read again.
                let known = seed % 2 == 1;
                let mut r = Rng(0x9e37_79b9_7f4a_7c15 ^ (seed * 7919 + n as u64 * 104_729 + 1));
                let (edges, junctions) = city(&mut r, tol);
                let mut d = Drawing {
                    edge_tags: (1..=edges.len() as u64).collect(),
                    junction_tags: (10_001..=10_000 + junctions.len() as u64).collect(),
                    next_tag: 100_000,
                    edges,
                    junctions,
                };
                let mut next_id = 5000.0;
                let (mut g, _) = Graph::build_kept(rules.clone(), &d.edges, &d.junctions, &[]);
                for step in 0..40 {
                    let (tags, jtags) = (d.edge_tags.clone(), d.junction_tags.clone());
                    let count = if r.next() < 0.1 {
                        6 + r.below(30)
                    } else {
                        1 + r.below(3)
                    };
                    for _ in 0..count {
                        edit_tagged(&mut r, &mut d, &mut next_id, tol);
                    }
                    // The last graph as a question left it: varied for a day's closures now and then.
                    if r.next() < 0.3 {
                        let day = crate::ops::network::day_of("2026-11-03");
                        g.vary(crate::ops::network::Access {
                            profile: None,
                            day,
                            time: None,
                        });
                    }
                    let (whole, whole_report) =
                        Graph::build_kept(rules.clone(), &d.edges, &d.junctions, &[]);
                    let (again, report, how) = if known {
                        g.rebuild_known(
                            rules.clone(),
                            &d.edges,
                            &d.junctions,
                            &[],
                            Drawing::was(&d.edge_tags, &tags),
                            Drawing::was(&d.junction_tags, &jtags),
                        )
                    } else {
                        g.rebuild(rules.clone(), &d.edges, &d.junctions, &[])
                    };
                    if let Some(what) =
                        differs(&whole, &again).or_else(|| kept_differs(&whole, &again, !known))
                    {
                        panic!("tanım {n}, tohum {seed}, adım {step} ({how:?}): {what} farklı");
                    }
                    assert_eq!(
                        whole_report, report,
                        "tanım {n}, tohum {seed}, adım {step}: rapor"
                    );
                    if how.anew {
                        anew += 1;
                    } else {
                        partly += 1;
                        recut += how.recut;
                    }
                    g = again;
                }
            }
        }
        // Both ways were taken many times.
        assert!(partly > 500 && anew > 50, "artımlı {partly}, baştan {anew}");
        assert!(recut > partly, "yeniden kesilen {recut}");
    }

    /// A graph built without keeping, or with other rules, is built anew.
    #[test]
    fn a_rebuild_without_a_kept_build_or_with_other_rules_starts_anew() {
        let defs = definitions();
        let mut r = Rng(7);
        let (edges, junctions) = city(&mut r, 0.01);
        let (mut g, _) = Graph::build_with(defs[0].clone(), &edges, &junctions, &[]);
        assert_eq!(g.kept_bytes(), 0);
        let (mut g2, _, how) = g.rebuild(defs[0].clone(), &edges, &junctions, &[]);
        assert!(how.anew);
        assert!(g2.kept_bytes() > 0);
        let (_, _, how) = g2.rebuild(defs[1].clone(), &edges, &junctions, &[]);
        assert!(how.anew);
        // The same drawing again: nothing cut again.
        let (mut g3, _) = Graph::build_kept(defs[0].clone(), &edges, &junctions, &[]);
        let (_, _, how) = g3.rebuild(defs[0].clone(), &edges, &junctions, &[]);
        assert_eq!(
            how,
            Rebuilt {
                anew: false,
                recut: 0,
                rejoined: 0
            }
        );
    }
}

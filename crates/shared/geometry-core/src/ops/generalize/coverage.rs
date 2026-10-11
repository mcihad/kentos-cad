//! The coverage's edges (docs/adr/0221 §8): the areas' rings cut at their
//! nodes into edges that are kept once, so that what is done to an edge
//! two neighbours share is done once and both get it. A segment is known by
//! its two ends' exact coordinates and its bulge (turned to one direction);
//! the rings that use it are its ring set. A vertex is a node where other
//! than two segments meet or where the ring set changes. A ring without a
//! node is a closed edge of its own. An edge keeps the direction of the
//! first ring that uses it.

use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use super::Track;
use crate::store::IdHasher;
use crate::vec2::Vec2;

type Fast = BuildHasherDefault<IdHasher>;

/// A vertex's key: its exact coordinates (+0.0 turns −0.0 into 0.0: one key for one place).
fn key(p: Vec2) -> (u64, u64) {
    ((p.x + 0.0).to_bits(), (p.y + 0.0).to_bits())
}

/// The coverage: its edges, how many rings use each, and each ring as its
/// edges in order with the way it goes along them.
#[derive(Clone, Debug)]
pub struct Coverage {
    pub edges: Vec<Track>,
    pub uses: Vec<usize>,
    pub rings: Vec<Vec<(usize, bool)>>,
}

/// Where a segment sits: its edge, its place in it and whether the edge goes along it in the key's direction.
#[derive(Clone, Copy)]
struct Seat {
    edge: usize,
    at: usize,
    along: bool,
}

/// The rings that use a segment, in order: the first two kept inline, more (overlapping areas) aside.
#[derive(Clone, Copy, PartialEq, Eq)]
struct RingSet {
    count: u32,
    first: u32,
    second: u32,
}

/// The coverage of `rings` (closed tracks).
pub fn build(rings: &[&Track]) -> Coverage {
    let total: usize = rings.iter().map(|r| r.pts.len()).sum();
    let mut vertex: HashMap<(u64, u64), u32, Fast> =
        HashMap::with_capacity_and_hasher(total, Fast::default());
    let mut segment: HashMap<(u32, u32, u64), u32, Fast> =
        HashMap::with_capacity_and_hasher(total, Fast::default());
    let mut degree: Vec<u32> = Vec::new();
    let mut sets: Vec<RingSet> = Vec::new();
    let mut more: HashMap<u32, Vec<u32>, Fast> = HashMap::default();
    // Each ring's segments: the segment and whether the ring goes along it in the key's direction.
    let mut segs: Vec<Vec<(u32, bool)>> = Vec::with_capacity(rings.len());
    let mut vertices: Vec<Vec<u32>> = Vec::with_capacity(rings.len());
    for (ri, r) in rings.iter().enumerate() {
        let ri = ri as u32;
        let ids: Vec<u32> = r
            .pts
            .iter()
            .map(|&p| {
                let next = vertex.len() as u32;
                *vertex.entry(key(p)).or_insert_with(|| {
                    degree.push(0);
                    next
                })
            })
            .collect();
        let n = ids.len();
        let mut list = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b, bulge) = (ids[i], ids[(i + 1) % n], r.bulges[i]);
            let (k, along) = if a <= b {
                ((a, b, (bulge + 0.0).to_bits()), true)
            } else {
                ((b, a, turned(bulge).to_bits()), false)
            };
            let next = sets.len() as u32;
            let sid = *segment.entry(k).or_insert_with(|| {
                sets.push(RingSet {
                    count: 0,
                    first: u32::MAX,
                    second: u32::MAX,
                });
                degree[a as usize] += 1;
                if b != a {
                    degree[b as usize] += 1;
                }
                next
            });
            let set = &mut sets[sid as usize];
            let last = match set.count {
                0 => None,
                1 => Some(set.first),
                2 => Some(set.second),
                _ => more.get(&sid).and_then(|m| m.last().copied()),
            };
            if last != Some(ri) {
                match set.count {
                    0 => set.first = ri,
                    1 => set.second = ri,
                    _ => more.entry(sid).or_default().push(ri),
                }
                set.count += 1;
            }
            list.push((sid, along));
        }
        segs.push(list);
        vertices.push(ids);
    }
    let same_set = |a: u32, b: u32| {
        let (x, y) = (sets[a as usize], sets[b as usize]);
        x == y && (x.count <= 2 || more.get(&a) == more.get(&b))
    };
    let mut edges: Vec<Track> = Vec::new();
    let mut uses: Vec<usize> = Vec::new();
    let mut seats: Vec<Option<Seat>> = vec![None; sets.len()];
    let mut out_rings = Vec::with_capacity(rings.len());
    for (ri, r) in rings.iter().enumerate() {
        let n = r.pts.len();
        let list = &segs[ri];
        let ids = &vertices[ri];
        let node = |i: usize| {
            degree[ids[i] as usize] != 2 || !same_set(list[(i + n - 1) % n].0, list[i].0)
        };
        let nodes: Vec<usize> = (0..n).filter(|&i| node(i)).collect();
        let mut refs = Vec::new();
        if nodes.is_empty() {
            // A ring of its own: one closed edge, shared with a ring that is the same one.
            let (s, along) = list[0];
            match seats[s as usize] {
                Some(seat) if edges[seat.edge].closed => {
                    uses[seat.edge] += 1;
                    refs.push((seat.edge, along == seat.along));
                }
                _ => {
                    let id = edges.len();
                    for (i, &(s, along)) in list.iter().enumerate() {
                        seats[s as usize] = Some(Seat {
                            edge: id,
                            at: i,
                            along,
                        });
                    }
                    edges.push((*r).clone());
                    uses.push(1);
                    refs.push((id, true));
                }
            }
            out_rings.push(refs);
            continue;
        }
        for (k, &start) in nodes.iter().enumerate() {
            let end = nodes[(k + 1) % nodes.len()];
            let count = if end > start {
                end - start
            } else {
                end + n - start
            };
            let (s, along) = list[start];
            let found = seats[s as usize].filter(|seat| {
                let e = &edges[seat.edge];
                !e.closed
                    && e.edges() == count
                    && if along == seat.along {
                        seat.at == 0
                    } else {
                        seat.at + 1 == count
                    }
            });
            match found {
                Some(seat) => {
                    uses[seat.edge] += 1;
                    refs.push((seat.edge, along == seat.along));
                }
                None => {
                    let id = edges.len();
                    let mut t = Track::empty(false);
                    for j in 0..=count {
                        let i = (start + j) % n;
                        let bulge = if j < count { r.bulges[i] } else { 0.0 };
                        t.pts.push(r.pts[i]);
                        t.bulges.push(bulge);
                        t.zs.push(r.zs[i]);
                        if j < count {
                            let (s, along) = list[i];
                            seats[s as usize].get_or_insert(Seat {
                                edge: id,
                                at: j,
                                along,
                            });
                        }
                    }
                    edges.push(t);
                    uses.push(1);
                    refs.push((id, true));
                }
            }
        }
        out_rings.push(refs);
    }
    Coverage {
        edges,
        uses,
        rings: out_rings,
    }
}

/// A bulge the other way round (never −0).
fn turned(b: f64) -> f64 {
    0.0 - b
}

/// An edge the other way round: a closed one from the same first vertex.
pub fn reversed(t: &Track) -> Track {
    let n = t.pts.len();
    let mut out = Track::empty(t.closed);
    if n == 0 {
        return out;
    }
    if t.closed {
        for j in 0..n {
            let i = (n - j) % n;
            let from = (n - j - 1) % n;
            out.pts.push(t.pts[i]);
            out.zs.push(t.zs[i]);
            out.bulges.push(turned(t.bulges[from]));
        }
    } else {
        for j in 0..n {
            let i = n - 1 - j;
            out.pts.push(t.pts[i]);
            out.zs.push(t.zs[i]);
            out.bulges
                .push(if i > 0 { turned(t.bulges[i - 1]) } else { 0.0 });
        }
    }
    out
}

/// Ring `k` put back together from `edges` (the coverage's, changed or not).
pub fn ring(c: &Coverage, edges: &[Track], k: usize) -> Track {
    let mut out = Track::empty(true);
    for &(e, forward) in &c.rings[k] {
        let t = if forward {
            edges[e].clone()
        } else {
            reversed(&edges[e])
        };
        if t.closed {
            return t;
        }
        // Each edge but its last vertex: the next edge starts there.
        for i in 0..t.pts.len().saturating_sub(1) {
            out.pts.push(t.pts[i]);
            out.bulges.push(t.bulges[i]);
            out.zs.push(t.zs[i]);
        }
    }
    out
}

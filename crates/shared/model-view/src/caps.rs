//! A section's caps (docs/adr/8131): where a plane of the section cuts a closed body, the cut is filled, as a drawn
//! section fills what its line cuts, so the body reads as solid and not as a hollow shell. Each plane meets each body's
//! triangles along segments; the segments are chained into closed loops by their points (a body's faces may each
//! carry their own corners), a box's loops clipped by its other planes, and nested loops filled even-odd (a loop
//! within one loop is a hole): the shared ear-clipper triangulates them. A cap lies a hair on the cut-away side of its
//! plane ([`LIFT`]), in front of everything the plane cuts (the renderer does not cut a cap's part: its style says so),
//! its faces turned to the cut-away side, where it is seen from. A see-through body (a space, an opening, glass) and a
//! body not closed where it is cut have no cap.

use std::collections::HashMap;
use std::sync::Arc;

use kentos_geometry_core::Vec2;

use crate::view::Source;

/// How far on the cut-away side a cap lies, metres: under any drawing's precision, so that a body's face lying in the
/// plane itself (a wall's end where the cut falls on it) does not fight it for the depth.
pub const LIFT: f64 = 0.000_5;

/// Points closer than this are one where loops are chained, metres: a model's bodies are float32 triangles, and corners
/// that should meet may lie a few hundredths of a millimetre apart (see [`MEET`]); a chain broken by such a hair left
/// its body uncapped, hollow in the section (docs/adr/8171 §1 B3). Far under any drawing's precision.
const JOIN: f64 = 5e-5;

/// How near a loop's edge another loop's corner lies to be taken into it, metres: a model's bodies are float32
/// triangles (their corners a few hundredths of a millimetre apart where they should meet), far under any drawing's
/// precision.
const MEET: f64 = 1e-4;

/// A mesh's vertices and triangles.
type Mesh = (Vec<[f64; 3]>, Vec<[u32; 3]>);

/// The caps of the bodies `sources` where `planes` (each keeping a·x + b·y + c·z + d ≥ 0, its normal of unit length)
/// cut them, each its body's object and name, of part `part`; none for a see-through source (`through`) or one whose
/// part is see-through (`opaque` says which are not). The caps on a plane meet without cracks: a loop's edge takes the
/// corners of the other loops that lie on it (a layer's cap beside the next one's, a wall's beside a column's), so
/// neighbouring triangles share their edges' ends.
pub fn caps(sources: &[Source], planes: &[[f64; 4]], part: u32, opaque: impl Fn(u32) -> bool) -> Vec<Source> {
    let kept: Vec<&Source> = sources.iter().filter(|s| !s.through && opaque(s.part)).collect();
    let mut meshes: Vec<Mesh> = vec![(Vec::new(), Vec::new()); kept.len()];
    for (i, p) in planes.iter().enumerate() {
        let others: Vec<[f64; 4]> = planes.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, q)| *q).collect();
        let mut loops: Vec<(usize, Vec<[f64; 3]>)> = Vec::new();
        for (k, s) in kept.iter().enumerate() {
            loops.extend(loops_of(&s.vertices, &s.triangles, *p, &others).into_iter().map(|r| (k, r)));
        }
        if loops.is_empty() {
            continue;
        }
        let (u, v, origin) = basis(*p, loops[0].1[0]);
        let flat = |q: [f64; 3]| {
            let d = sub(q, origin);
            Vec2::new(dot(d, u), dot(d, v))
        };
        joined(&mut loops, &flat);
        for (k, (vertices, triangles)) in meshes.iter_mut().enumerate() {
            let mine: Vec<Vec<[f64; 3]>> = loops.iter().filter(|(o, _)| *o == k).map(|(_, r)| r.clone()).collect();
            if !mine.is_empty() {
                fill(&mine, *p, &flat, vertices, triangles);
            }
        }
    }
    kept.iter()
        .zip(meshes)
        .filter(|(_, (_, t))| !t.is_empty())
        .map(|(s, (vertices, triangles))| Source {
            part,
            name: s.name.clone(),
            object: s.object,
            vertices: Arc::new(vertices),
            triangles,
            through: false,
        })
        .collect()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn signed(p: &[f64; 4], v: [f64; 3]) -> f64 {
    p[0] * v[0] + p[1] * v[1] + p[2] * v[2] + p[3]
}

/// Where the edge `a`–`b` meets the plane: from its endpoints in one order whichever triangle it comes from, so the
/// two triangles of an edge give the same point.
fn meet(p: &[f64; 4], a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    let (da, db) = (signed(p, a), signed(p, b));
    let t = da / (da - db);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn key(v: [f64; 3]) -> (i64, i64, i64) {
    let q = |x: f64| (x / JOIN).floor() as i64;
    (q(v[0]), q(v[1]), q(v[2]))
}

/// The segments' ends made nodes: an end within [`JOIN`] of a node made before is that node (the cells about its own
/// searched), so two ends a hair apart are one wherever the grid's lines fall between them.
#[derive(Default)]
struct Nodes {
    cells: HashMap<(i64, i64, i64), Vec<usize>>,
    at: Vec<[f64; 3]>,
}

impl Nodes {
    fn id(&mut self, v: [f64; 3]) -> usize {
        let c = key(v);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for &n in self.cells.get(&(c.0 + dx, c.1 + dy, c.2 + dz)).into_iter().flatten() {
                        let d = sub(self.at[n], v);
                        if dot(d, d) <= JOIN * JOIN {
                            return n;
                        }
                    }
                }
            }
        }
        let n = self.at.len();
        self.at.push(v);
        self.cells.entry(c).or_default().push(n);
        n
    }
}

/// Two directions across the plane `p` and a point of it near the model (`near`), so its flat coordinates stay small.
fn basis(p: [f64; 4], near: [f64; 3]) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let n = [p[0], p[1], p[2]];
    let helper = if n[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let u = {
        let c = cross(helper, n);
        let l = dot(c, c).sqrt();
        [c[0] / l, c[1] / l, c[2] / l]
    };
    (u, cross(n, u), near)
}

/// One body's closed loops where the plane `p` cuts it, clipped by `others`.
fn loops_of(vs: &[[f64; 3]], ts: &[[u32; 3]], p: [f64; 4], others: &[[f64; 4]]) -> Vec<Vec<[f64; 3]>> {
    // Each triangle crossing the plane gives a segment between its two edges that cross it (a corner on the plane
    // counts as kept); its ends are nodes, a segment whose ends are one node none.
    let mut nodes = Nodes::default();
    let mut segments: Vec<[usize; 2]> = Vec::new();
    for t in ts {
        let (Some(&a), Some(&b), Some(&c)) = (vs.get(t[0] as usize), vs.get(t[1] as usize), vs.get(t[2] as usize))
        else {
            continue;
        };
        let corners = [a, b, c];
        let side: Vec<bool> = corners.iter().map(|v| signed(&p, *v) >= 0.0).collect();
        let crossing: Vec<[f64; 3]> = (0..3)
            .filter(|&k| side[k] != side[(k + 1) % 3])
            .map(|k| meet(&p, corners[k], corners[(k + 1) % 3]))
            .collect();
        if let [x, y] = crossing.as_slice() {
            let (i, j) = (nodes.id(*x), nodes.id(*y));
            if i != j {
                segments.push([i, j]);
            }
        }
    }
    // Chained by their nodes: a closed body's cut is closed loops; a chain that does not close is left.
    let mut at: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
    for (k, s) in segments.iter().enumerate() {
        at.entry(s[0]).or_default().push((k, 0));
        at.entry(s[1]).or_default().push((k, 1));
    }
    let mut used = vec![false; segments.len()];
    let mut loops: Vec<Vec<[f64; 3]>> = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let first = segments[start][0];
        let mut ring = vec![nodes.at[first]];
        let mut end = segments[start][1];
        let mut closed = false;
        for _ in 0..segments.len() {
            if end == first {
                closed = true;
                break;
            }
            ring.push(nodes.at[end]);
            let next = at.get(&end).and_then(|list| list.iter().copied().find(|(k, _)| !used[*k]));
            let Some((k, e)) = next else { break };
            used[k] = true;
            end = segments[k][1 - e];
        }
        if closed && ring.len() >= 3 {
            loops.push(ring);
        }
    }
    // A box's other planes clip each loop (each keeps a convex region).
    loops
        .into_iter()
        .map(|mut ring| {
            for q in others {
                ring = clip(&ring, q);
            }
            ring
        })
        .filter(|r| r.len() >= 3)
        .collect()
}

/// The loops on one plane made to meet without T-junctions: each edge takes, in order along it, the other loops'
/// corners that lie on it (within [`MEET`]), found through a grid of their flat places.
fn joined(loops: &mut [(usize, Vec<[f64; 3]>)], flat: &dyn Fn([f64; 3]) -> Vec2) {
    let corners: Vec<([f64; 3], Vec2)> = loops.iter().flat_map(|(_, r)| r.iter().map(|q| (*q, flat(*q)))).collect();
    if corners.is_empty() {
        return;
    }
    // A grid of a cell about the corners' spread over their count's square root.
    let (mut lo, mut hi) = (Vec2::new(f64::INFINITY, f64::INFINITY), Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY));
    for (_, f) in &corners {
        lo = Vec2::new(lo.x.min(f.x), lo.y.min(f.y));
        hi = Vec2::new(hi.x.max(f.x), hi.y.max(f.y));
    }
    let side = ((hi.x - lo.x).max(hi.y - lo.y) / (corners.len() as f64).sqrt()).max(1e-3);
    let cell = |f: Vec2| (((f.x - lo.x) / side).floor() as i64, ((f.y - lo.y) / side).floor() as i64);
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (k, (_, f)) in corners.iter().enumerate() {
        grid.entry(cell(*f)).or_default().push(k);
    }
    for (_, ring) in loops.iter_mut() {
        let n = ring.len();
        let mut out: Vec<[f64; 3]> = Vec::with_capacity(n);
        for e in 0..n {
            let (a, b) = (ring[e], ring[(e + 1) % n]);
            out.push(a);
            let (fa, fb) = (flat(a), flat(b));
            let d = Vec2::new(fb.x - fa.x, fb.y - fa.y);
            let len2 = d.x * d.x + d.y * d.y;
            if len2 <= JOIN * JOIN {
                continue;
            }
            let (c0, c1) = (cell(Vec2::new(fa.x.min(fb.x), fa.y.min(fb.y))), cell(Vec2::new(fa.x.max(fb.x), fa.y.max(fb.y))));
            let mut on: Vec<(f64, [f64; 3])> = Vec::new();
            for cx in c0.0..=c1.0 {
                for cy in c0.1..=c1.1 {
                    for &k in grid.get(&(cx, cy)).into_iter().flatten() {
                        let (q, f) = corners[k];
                        let t = ((f.x - fa.x) * d.x + (f.y - fa.y) * d.y) / len2;
                        if t <= 1e-9 || t >= 1.0 - 1e-9 {
                            continue;
                        }
                        let off = (f.x - (fa.x + d.x * t)).hypot(f.y - (fa.y + d.y * t));
                        let apart = |g: Vec2| (f.x - g.x).hypot(f.y - g.y) > MEET;
                        if off <= MEET && apart(fa) && apart(fb) {
                            on.push((t, q));
                        }
                    }
                }
            }
            on.sort_by(|x, y| x.0.total_cmp(&y.0));
            on.dedup_by(|x, y| (flat(x.1).x - flat(y.1).x).hypot(flat(x.1).y - flat(y.1).y) <= MEET);
            out.extend(on.into_iter().map(|(_, q)| q));
        }
        *ring = out;
    }
}

/// A point just inside a ring: off the middle of its longest edge towards its inside.
fn probe(ring: &[Vec2]) -> Vec2 {
    let n = ring.len();
    let twice: f64 = (0..n).map(|k| ring[k].x * ring[(k + 1) % n].y - ring[(k + 1) % n].x * ring[k].y).sum();
    let (a, b) = (0..n)
        .map(|k| (ring[k], ring[(k + 1) % n]))
        .max_by(|x, y| (x.1.x - x.0.x).hypot(x.1.y - x.0.y).total_cmp(&(y.1.x - y.0.x).hypot(y.1.y - y.0.y)))
        .unwrap_or((ring[0], ring[0]));
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l = dx.hypot(dy).max(f64::MIN_POSITIVE);
    // The inside is on the left of a counter-clockwise ring, on the right of a clockwise one.
    let s = if twice >= 0.0 { 1.0 } else { -1.0 };
    let step = (l * 1e-4).min(1e-4);
    Vec2::new((a.x + b.x) / 2.0 - dy / l * step * s, (a.y + b.y) / 2.0 + dx / l * step * s)
}

/// One body's loops on the plane `p` filled even-odd (a loop inside an odd number of the others a hole of the
/// smallest that holds it), each triangle turned to the cut-away side, lying [`LIFT`] on that side.
fn fill(
    loops: &[Vec<[f64; 3]>],
    p: [f64; 4],
    flat: &dyn Fn([f64; 3]) -> Vec2,
    vertices: &mut Vec<[f64; 3]>,
    triangles: &mut Vec<[u32; 3]>,
) {
    let n = [p[0], p[1], p[2]];
    let rings: Vec<Vec<Vec2>> = loops.iter().map(|r| r.iter().map(|q| flat(*q)).collect()).collect();
    let probes: Vec<Vec2> = rings.iter().map(|r| probe(r)).collect();
    let depth: Vec<usize> = (0..rings.len())
        .map(|i| (0..rings.len()).filter(|&j| j != i && inside(probes[i], &rings[j])).count())
        .collect();
    let area = |r: &[Vec2]| {
        let n = r.len();
        (0..n).map(|k| r[k].x * r[(k + 1) % n].y - r[(k + 1) % n].x * r[k].y).sum::<f64>().abs() / 2.0
    };
    for (o, outer) in rings.iter().enumerate().filter(|(i, _)| depth[*i].is_multiple_of(2)) {
        let holes: Vec<usize> = (0..rings.len())
            .filter(|&h| depth[h] == depth[o] + 1 && inside(probes[h], outer))
            .filter(|&h| {
                // The smallest outer loop that holds it is its.
                !rings.iter().enumerate().any(|(k, r)| {
                    k != o && depth[k].is_multiple_of(2) && depth[k] > depth[o] && inside(probes[h], r) && area(r) < area(outer)
                })
            })
            .collect();
        // The rings' corners alone are triangulated: a run of points along a straight edge (a neighbour's corner taken
        // in, a face's split) would leave the ear-clipper's diagonals through them, T-junctions that open at the
        // samples. Each triangle on such an edge is then fanned from its middle through the run's points, so the cap is
        // whole inside and its neighbours still share every point of its edges.
        let mut pts: Vec<Vec2> = Vec::new();
        let mut source: Vec<[f64; 3]> = Vec::new();
        let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
        // The points dropped between two corners, by the pair of the corners' indices into `pts`.
        let mut runs: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        let mut extra: Vec<([f64; 3], Vec2)> = Vec::new();
        for r in std::iter::once(o).chain(holes) {
            let (ring2, ring3) = (&rings[r], &loops[r]);
            let m = ring2.len();
            let corner: Vec<bool> = (0..m)
                .map(|k| {
                    let (a, b, c) = (ring2[(k + m - 1) % m], ring2[k], ring2[(k + 1) % m]);
                    let (dx, dy) = (c.x - a.x, c.y - a.y);
                    let l = dx.hypot(dy);
                    l <= 0.0 || ((b.x - a.x) * dy - (b.y - a.y) * dx).abs() / l > MEET * 0.1
                })
                .collect();
            let Some(first) = (0..m).find(|&k| corner[k]) else { continue };
            let from = pts.len();
            let mut last: Option<usize> = None;
            let mut pending: Vec<usize> = Vec::new();
            for step in 0..m {
                let k = (first + step) % m;
                if corner[k] {
                    let at = pts.len();
                    pts.push(ring2[k]);
                    source.push(ring3[k]);
                    if let Some(prev) = last
                        && !pending.is_empty()
                    {
                        runs.insert((prev, at), std::mem::take(&mut pending));
                    }
                    last = Some(at);
                } else {
                    pending.push(extra.len());
                    extra.push((ring3[k], ring2[k]));
                }
            }
            if let Some(prev) = last
                && !pending.is_empty()
            {
                runs.insert((prev, from), std::mem::take(&mut pending));
            }
            if pts.len() - from >= 3 {
                ranges.push(from..pts.len());
            } else {
                pts.truncate(from);
                source.truncate(from);
            }
        }
        if ranges.is_empty() {
            continue;
        }
        let mut idx: Vec<u32> = Vec::new();
        kentos_geometry_core::triangulate::triangulate_into(&pts, &ranges, &mut idx);
        let base = vertices.len() as u32;
        let lifted = |q: [f64; 3]| [q[0] - n[0] * LIFT, q[1] - n[1] * LIFT, q[2] - n[2] * LIFT];
        vertices.extend(source.iter().map(|q| lifted(*q)));
        // An extra point's index, given once.
        let mut placed: HashMap<usize, u32> = HashMap::new();
        let push = |tri: [u32; 3], vertices: &Vec<[f64; 3]>, triangles: &mut Vec<[u32; 3]>| {
            let (a, b, c) = (vertices[tri[0] as usize], vertices[tri[1] as usize], vertices[tri[2] as usize]);
            let facing = dot(cross(sub(b, a), sub(c, a)), n);
            // Turned to the cut-away side.
            triangles.push(if facing <= 0.0 { tri } else { [tri[0], tri[2], tri[1]] });
        };
        for t in idx.as_chunks::<3>().0.iter() {
            let ids = [t[0] as usize, t[1] as usize, t[2] as usize];
            // Its edges' runs, either way round.
            let run_of = |u: usize, v: usize| -> Option<Vec<usize>> {
                runs.get(&(u, v)).cloned().or_else(|| runs.get(&(v, u)).map(|r| r.iter().rev().copied().collect()))
            };
            let edge_runs: Vec<Option<Vec<usize>>> = (0..3).map(|e| run_of(ids[e], ids[(e + 1) % 3])).collect();
            if edge_runs.iter().all(Option::is_none) {
                push([base + t[0], base + t[1], base + t[2]], vertices, triangles);
                continue;
            }
            // Its boundary with the runs' points, fanned from its middle.
            let mut ring: Vec<u32> = Vec::new();
            for e in 0..3 {
                ring.push(base + ids[e] as u32);
                for &x in edge_runs[e].iter().flatten() {
                    let at = *placed.entry(x).or_insert_with(|| {
                        vertices.push(lifted(extra[x].0));
                        (vertices.len() - 1) as u32
                    });
                    ring.push(at);
                }
            }
            let (a, b, c) = (source[ids[0]], source[ids[1]], source[ids[2]]);
            vertices.push(lifted([(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0]));
            let middle = (vertices.len() - 1) as u32;
            for k in 0..ring.len() {
                push([middle, ring[k], ring[(k + 1) % ring.len()]], vertices, triangles);
            }
        }
    }
}

/// `ring` clipped to the half-space `q` keeps (Sutherland–Hodgman).
fn clip(ring: &[[f64; 3]], q: &[f64; 4]) -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    let n = ring.len();
    for k in 0..n {
        let (a, b) = (ring[k], ring[(k + 1) % n]);
        let (da, db) = (signed(q, a), signed(q, b));
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]);
        }
    }
    out
}

/// Whether `p` is inside the ring (even-odd crossings).
fn inside(p: Vec2, ring: &[Vec2]) -> bool {
    let n = ring.len();
    let mut odd = false;
    for k in 0..n {
        let (a, b) = (ring[k], ring[(k + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (b.x - a.x) * (p.y - a.y) / (b.y - a.y) {
            odd = !odd;
        }
    }
    odd
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A closed box from `lo` to `hi`, its faces each with their own corners (as a model's flat faces are).
    fn cuboid(lo: [f64; 3], hi: [f64; 3], object: usize) -> Source {
        let c = |i: usize| [if i & 1 == 0 { lo[0] } else { hi[0] }, if i & 2 == 0 { lo[1] } else { hi[1] }, if i & 4 == 0 { lo[2] } else { hi[2] }];
        let faces = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        for f in faces {
            let base = vertices.len() as u32;
            vertices.extend(f.iter().map(|&i| c(i)));
            triangles.push([base, base + 1, base + 2]);
            triangles.push([base, base + 2, base + 3]);
        }
        Source { part: 0, name: "kutu".into(), object, vertices: Arc::new(vertices), triangles, through: false }
    }

    fn area(s: &Source) -> f64 {
        s.triangles
            .iter()
            .map(|t| {
                let (a, b, c) = (s.vertices[t[0] as usize], s.vertices[t[1] as usize], s.vertices[t[2] as usize]);
                let n = cross(sub(b, a), sub(c, a));
                dot(n, n).sqrt() / 2.0
            })
            .sum()
    }

    /// A block whose faces' corners lie a few hundredths of a millimetre apart, as a float32 body's that should meet
    /// do (docs/adr/8171 §1 B3): its cut still closes and it is capped whole. With the chain's points one only within
    /// 1e-7 m the loop broke and the block was left hollow.
    #[test]
    fn a_body_whose_corners_lie_a_hair_apart_is_capped_whole() {
        let mut block = cuboid([10.0, 20.0, 0.0], [14.0, 22.0, 3.0], 3);
        // Each face's own corners moved by up to 0.02 mm, differently for each face.
        let moved: Vec<[f64; 3]> = block
            .vertices
            .iter()
            .enumerate()
            .map(|(k, v)| {
                let face = (k / 4) as f64;
                [v[0] + 2e-5 * (face - 2.5) / 2.5, v[1] - 1.5e-5 * (face - 2.5) / 2.5, v[2]]
            })
            .collect();
        block.vertices = Arc::new(moved);
        let caps = super::caps(&[block], &[[0.0, 0.0, -1.0, 1.25]], 99, |_| true);
        assert_eq!(caps.len(), 1, "capped");
        assert!((area(&caps[0]) - 8.0).abs() < 1e-3, "{}", area(&caps[0]));
    }

    /// A 4 × 2 × 3 m block cut level at 1.25 m: its cap is the 4 × 2 rectangle (8 m²), a hair above 1.25 (the cut-away
    /// side), facing up; boxed (x 1 to 3, under 1.25), its caps are the box's top, 2 × 2, and its two sides at x 1 and
    /// 3, each 2 × 1.25.
    #[test]
    fn a_level_cut_caps_a_block_and_a_box_clips_its_caps() {
        let block = cuboid([0.0, 0.0, 0.0], [4.0, 2.0, 3.0], 7);
        let below = [0.0, 0.0, -1.0, 1.25];
        let level_caps = super::caps(std::slice::from_ref(&block), &[below], 99, |_| true);
        assert_eq!(level_caps.len(), 1);
        let c = &level_caps[0];
        assert_eq!((c.part, c.object), (99, 7));
        assert!((area(c) - 8.0).abs() < 1e-9, "{}", area(c));
        assert!(c.vertices.iter().all(|v| (v[2] - (1.25 + LIFT)).abs() < 1e-12), "a hair on the cut-away side, above");
        for t in &c.triangles {
            let (a, b, d) = (c.vertices[t[0] as usize], c.vertices[t[1] as usize], c.vertices[t[2] as usize]);
            assert!(cross(sub(b, a), sub(d, a))[2] > 0.0, "faces up, the cut-away side");
        }
        // A box x 1–3, y −1–3, z −1–1.25: its top cuts 2 × 2; its sides x = 1 and x = 3 cut 2 × 1.25 each (the block from
        // z 0 to the box's top, y 0–2); its bottom and its y sides miss the block.
        let boxed = [
            [1.0, 0.0, 0.0, -1.0],
            [-1.0, 0.0, 0.0, 3.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, -1.0, 0.0, 3.0],
            [0.0, 0.0, 1.0, 1.0],
            [0.0, 0.0, -1.0, 1.25],
        ];
        let boxed_caps = super::caps(&[block], &boxed, 99, |_| true);
        assert_eq!(boxed_caps.len(), 1);
        assert!((area(&boxed_caps[0]) - (4.0 + 2.0 * 2.0 * 1.25)).abs() < 1e-6, "{}", area(&boxed_caps[0]));
    }

    /// A 4 m block beside two 2 m ones along its north face, cut level: the long block's cap takes the corner the two
    /// short ones meet at (x 2, y 1) on its edge, so the caps share it and no crack opens between them.
    #[test]
    fn neighbouring_caps_meet_without_t_junctions() {
        let long = cuboid([0.0, 0.0, 0.0], [4.0, 1.0, 3.0], 1);
        let west = cuboid([0.0, 1.0, 0.0], [2.0, 1.2, 3.0], 2);
        let east = cuboid([2.0, 1.0, 0.0], [4.0, 1.2, 3.0], 3);
        let level = [0.0, 0.0, -1.0, 1.5];
        let caps = super::caps(&[long, west, east], &[level], 99, |_| true);
        assert_eq!(caps.len(), 3);
        let at = |s: &Source, x: f64, y: f64| s.vertices.iter().any(|v| (v[0] - x).abs() < 1e-9 && (v[1] - y).abs() < 1e-9);
        assert!(at(&caps[0], 2.0, 1.0), "the long cap's north edge takes the short ones' corner");
        assert!((area(&caps[0]) - 4.0).abs() < 1e-9 && (area(&caps[1]) - 0.4).abs() < 1e-9, "areas kept");
    }

    /// A shaft's walls as one closed ring-shaped body (outer 3 × 3, inner 2 × 2) cut level: its cap is the ring, 9 − 4 =
    /// 5 m², the inner loop a hole; a see-through body and an open shell have none.
    #[test]
    fn a_ring_s_cap_has_its_hole_and_see_through_or_open_bodies_none() {
        // The ring: outer box faces outward, inner box faces inward (its triangles turned).
        let outer = cuboid([0.0, 0.0, 0.0], [3.0, 3.0, 3.0], 1);
        let mut inner = cuboid([0.5, 0.5, 0.0], [2.5, 2.5, 3.0], 1);
        inner.triangles.iter_mut().for_each(|t| t.swap(1, 2));
        let mut vertices = outer.vertices.to_vec();
        let base = vertices.len() as u32;
        vertices.extend(inner.vertices.iter().copied());
        let mut triangles = outer.triangles.clone();
        triangles.extend(inner.triangles.iter().map(|t| [t[0] + base, t[1] + base, t[2] + base]));
        let ring = Source { vertices: Arc::new(vertices), triangles, ..outer };
        let level = [0.0, 0.0, -1.0, 1.5];
        let ring_caps = super::caps(std::slice::from_ref(&ring), &[level], 99, |_| true);
        assert!((area(&ring_caps[0]) - 5.0).abs() < 1e-9, "{}", area(&ring_caps[0]));
        let glass = Source { through: true, ..ring.clone() };
        assert!(super::caps(&[glass], &[level], 99, |_| true).is_empty());
        assert!(super::caps(std::slice::from_ref(&ring), &[level], 99, |_| false).is_empty(), "a see-through part");
        // An open shell, the block without its east face: its cut does not close, so it has no cap.
        let mut open = cuboid([0.0, 0.0, 0.0], [4.0, 2.0, 3.0], 2);
        open.triangles.truncate(10);
        assert!(super::caps(&[open], &[level], 99, |_| true).is_empty());
    }

    /// A measurement (docs/adr/8131 §4): six storeys of 40 × 40 wall pieces (9 600 bodies, 115 200 triangles) cut
    /// level through the third storey (1 600 bodies cut), north–south through the middle (240) and by a box about a
    /// quarter of the plan.
    ///
    /// ```text
    /// cargo test -p kentos-model-view --release --lib caps::tests::timing -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "a measurement, run by hand in release"]
    fn timing() {
        let mut bodies = Vec::new();
        for storey in 0..6 {
            for i in 0..40 {
                for j in 0..40 {
                    let (x, y, z) = (f64::from(i) * 3.0, f64::from(j) * 3.0, f64::from(storey) * 3.0);
                    bodies.push(cuboid([x, y, z], [x + 2.75, y + 0.25, z + 2.8], bodies.len()));
                }
            }
        }
        let triangles: usize = bodies.iter().map(|b| b.triangles.len()).sum();
        let boxed = [
            [1.0, 0.0, 0.0, -0.1],
            [-1.0, 0.0, 0.0, 60.0],
            [0.0, 1.0, 0.0, -0.1],
            [0.0, -1.0, 0.0, 60.0],
            [0.0, 0.0, 1.0, -0.1],
            [0.0, 0.0, -1.0, 7.5],
        ];
        for (name, planes) in [
            ("level", vec![[0.0, 0.0, -1.0, 7.5]]),
            ("north-south", vec![[-1.0, 0.0, 0.0, 61.4]]),
            ("box", boxed.to_vec()),
        ] {
            let start = std::time::Instant::now();
            let caps = super::caps(&bodies, &planes, 99, |_| true);
            let took = start.elapsed();
            let cap_triangles: usize = caps.iter().map(|c| c.triangles.len()).sum();
            println!(
                "{name}: {} bodies, {triangles} triangles: {} caps, {cap_triangles} triangles in {:.1} ms",
                bodies.len(),
                caps.len(),
                took.as_secs_f64() * 1000.0
            );
        }
    }
}

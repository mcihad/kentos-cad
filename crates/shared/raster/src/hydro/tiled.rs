//! Çukur doldur on the threads (docs/adr/0235 §12; Barnes 2016, “Parallel
//! priority-flood depression filling for trillion cell digital elevation
//! models”): the raster in strips of rows, each strip flooded on its own with
//! its edge rows' cells as outlets of their own, every cell labelled by the
//! outlet it was reached from; the labels' meeting heights, inside a strip
//! and across the strips' seams, make a graph whose least spill heights from
//! the true outlets (one label: the ocean) a small priority flood finds; a
//! cell then rises to its label's spill height. The least fill is unique, so
//! this gives the one-thread flood's surface cell for cell.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

use super::heap::{RadixHeap, key_of, value_of};
use super::surface::{N, Surface};
use crate::par;

/// The true outlets' label.
const OCEAN: u32 = 1;

/// A strip's share of the arrays and what its flood met.
struct Strip<'a> {
    r0: usize,
    r1: usize,
    z: &'a mut [f64],
    labels: &'a mut [u32],
    raised: &'a mut [u8],
    /// The strip's first own label.
    first: u32,
    /// Labels that met: (a, b, height) with a < b.
    edges: Vec<(u32, u32, f64)>,
}

/// Fills `s` (ε = 0) on `threads`; the cells raised.
pub fn fill(s: &mut Surface, threads: usize) -> u64 {
    let w = s.width as usize;
    let h = s.height as usize;
    let n = s.len();
    // The outlets, before any height changes.
    let mut outlet = vec![0u8; n];
    {
        let surface = &*s;
        par::rows(threads, &mut outlet, w, &|first, chunk: &mut [u8]| {
            for (r, row) in chunk.chunks_mut(w).enumerate() {
                let j = first + r;
                for (i, o) in row.iter_mut().enumerate() {
                    if surface.valid(j * w + i) && surface.outlet(i, j) {
                        *o = 1;
                    }
                }
            }
        });
    }
    let pieces = (threads * 4).clamp(1, h);
    let per = h.div_ceil(pieces);
    let starts: Vec<usize> = (0..h).step_by(per).collect();
    // Each strip's labels: its edge rows' cells that are not true outlets.
    let mut first = Vec::with_capacity(starts.len());
    let mut next = OCEAN + 1;
    for &r0 in &starts {
        let r1 = (r0 + per).min(h);
        first.push(next);
        for j in r0..r1 {
            if !((j == r0 && r0 > 0) || (j == r1 - 1 && r1 < h)) {
                continue;
            }
            for i in 0..w {
                let k = j * w + i;
                if s.valid(k) && outlet[k] == 0 {
                    next += 1;
                }
            }
        }
    }
    let count = next as usize;
    let mut labels = vec![0u32; n];
    let mut raised = vec![0u8; n];
    {
        let mut strips: Vec<Strip> = Vec::with_capacity(starts.len());
        let (mut zs, mut ls, mut rs) = (&mut s.z[..], &mut labels[..], &mut raised[..]);
        for (x, &r0) in starts.iter().enumerate() {
            let r1 = (r0 + per).min(h);
            let take = (r1 - r0) * w;
            let (z, zt) = zs.split_at_mut(take);
            let (l, lt) = ls.split_at_mut(take);
            let (r, rt) = rs.split_at_mut(take);
            zs = zt;
            ls = lt;
            rs = rt;
            strips.push(Strip {
                r0,
                r1,
                z,
                labels: l,
                raised: r,
                first: first[x],
                edges: Vec::new(),
            });
        }
        let outlet = &outlet;
        par::each_mut(threads, &mut strips, &|_, st: &mut Strip| {
            flood(st, w, h, outlet)
        });
        // The strips' own meetings, then their seams': each cell of a strip's last row with the next's first.
        let mut edges: Vec<(u32, u32, f64)> = Vec::new();
        for st in &mut strips {
            edges.append(&mut st.edges);
        }
        for pair in strips.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            let ra = (a.r1 - 1 - a.r0) * w;
            for i in 0..w {
                let za = a.z[ra + i];
                if za.is_nan() {
                    continue;
                }
                let la = a.labels[ra + i];
                for di in [-1i64, 0, 1] {
                    let ib = i as i64 + di;
                    if ib < 0 || ib >= w as i64 {
                        continue;
                    }
                    let zb = b.z[ib as usize];
                    if zb.is_nan() {
                        continue;
                    }
                    let lb = b.labels[ib as usize];
                    if la != lb {
                        edges.push((la.min(lb), la.max(lb), if za > zb { za } else { zb }));
                    }
                }
            }
        }
        // The labels' spill heights: the least, over paths to the ocean, of the highest meeting.
        let spill = spills(count, &edges);
        par::each_mut(threads, &mut strips, &|_, st: &mut Strip| {
            for k in 0..st.z.len() {
                let z = st.z[k];
                if z.is_nan() {
                    continue;
                }
                let up = spill[st.labels[k] as usize];
                if up > z {
                    st.z[k] = up;
                    st.raised[k] = 1;
                }
            }
        });
    }
    raised.iter().map(|&r| u64::from(r)).sum()
}

/// The least spill height of every label from the ocean (−∞ for the ocean).
fn spills(count: usize, edges: &[(u32, u32, f64)]) -> Vec<f64> {
    // Both ways, as a compressed adjacency.
    let mut degree = vec![0u32; count + 1];
    for &(a, b, _) in edges {
        degree[a as usize + 1] += 1;
        degree[b as usize + 1] += 1;
    }
    for x in 1..degree.len() {
        degree[x] += degree[x - 1];
    }
    let mut at = degree.clone();
    let mut to = vec![(0u32, 0.0f64); edges.len() * 2];
    for &(a, b, z) in edges {
        to[at[a as usize] as usize] = (b, z);
        at[a as usize] += 1;
        to[at[b as usize] as usize] = (a, z);
        at[b as usize] += 1;
    }
    let mut spill = vec![f64::INFINITY; count];
    spill[OCEAN as usize] = f64::NEG_INFINITY;
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((key_of(f64::NEG_INFINITY), OCEAN)));
    while let Some(Reverse((key, u))) = heap.pop() {
        let su = value_of(key);
        if su != spill[u as usize] {
            continue;
        }
        for &(v, z) in &to[degree[u as usize] as usize..degree[u as usize + 1] as usize] {
            let cand = if su > z { su } else { z };
            if cand < spill[v as usize] {
                spill[v as usize] = cand;
                heap.push(Reverse((key_of(cand), v)));
            }
        }
    }
    spill
}

/// A strip's own flood: its true outlets the ocean's, its edge rows' other cells outlets of their own.
fn flood(st: &mut Strip, w: usize, h: usize, outlet: &[u8]) {
    let (r0, r1) = (st.r0, st.r1);
    let rows = r1 - r0;
    let size = rows * w;
    let mut closed = vec![0u8; size];
    let mut heap = RadixHeap::new();
    let mut pit: VecDeque<u32> = VecDeque::new();
    let mut trace: Vec<u32> = Vec::new();
    let mut label = st.first;
    for (l, done) in closed.iter_mut().enumerate() {
        let (i, j) = (l % w, r0 + l / w);
        let z = st.z[l];
        if z.is_nan() {
            continue;
        }
        let edge = (j == r0 && r0 > 0) || (j == r1 - 1 && r1 < h);
        if outlet[j * w + i] != 0 {
            st.labels[l] = OCEAN;
        } else if edge {
            st.labels[l] = label;
            label += 1;
        } else {
            continue;
        }
        *done = 1;
        heap.push(key_of(z), l as u32);
    }
    // The strip's neighbours of a cell (local index), in order.
    let each = |l: usize, f: &mut dyn FnMut(usize)| {
        let (i, j) = (l % w, l / w);
        for &(di, dj) in &N {
            let (a, b) = (i as i64 + di, j as i64 + dj);
            if a >= 0 && b >= 0 && a < w as i64 && b < rows as i64 {
                f(b as usize * w + a as usize);
            }
        }
    };
    loop {
        let c = if let Some(c) = pit.pop_front() {
            c
        } else if let Some(c) = trace.pop() {
            c
        } else if let Some((_, c)) = heap.pop() {
            c
        } else {
            break;
        };
        let c = c as usize;
        let level = st.z[c];
        let lc = st.labels[c];
        let mut nb = [0usize; 8];
        let mut count = 0;
        each(c, &mut |m| {
            nb[count] = m;
            count += 1;
        });
        for &m in &nb[..count] {
            let zm = st.z[m];
            if zm.is_nan() {
                continue;
            }
            if closed[m] != 0 {
                let lm = st.labels[m];
                if lm != lc {
                    st.edges
                        .push((lc.min(lm), lc.max(lm), if level > zm { level } else { zm }));
                }
                continue;
            }
            closed[m] = 1;
            st.labels[m] = lc;
            if zm <= level {
                if zm < level {
                    st.raised[m] = 1;
                }
                st.z[m] = level;
                pit.push_back(m as u32);
            } else {
                // A slope cell: into the heap while an unreached neighbour is not higher, else traced.
                let mut spills = false;
                each(m, &mut |q| {
                    let zq = st.z[q];
                    if !zq.is_nan() && closed[q] == 0 && zq <= zm {
                        spills = true;
                    }
                });
                if spills {
                    heap.push(key_of(zm), m as u32);
                } else {
                    trace.push(m as u32);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fill::Filling;
    use super::*;

    /// A noisy surface with holes, its pits deep and shallow.
    fn surface(w: u32, h: u32, seed: u32) -> Surface {
        let mut s = Surface::new(w, h, [0.0, 1.0, 0.0, 0.0, 0.0, -1.0], false);
        for j in 0..h {
            for i in 0..w {
                let mut x = i.wrapping_mul(0x9e37_79b9)
                    ^ j.wrapping_mul(0x85eb_ca6b)
                    ^ seed.wrapping_mul(0x2c1b_3c6d);
                x ^= x >> 15;
                x = x.wrapping_mul(0x2c1b_3c6d);
                x ^= x >> 12;
                let noise = f64::from(x % 1000) / 100.0;
                let k = (j * w + i) as usize;
                s.z[k] = if x.is_multiple_of(97) {
                    f64::NAN
                } else {
                    50.0 + 0.05 * f64::from(i) + 8.0 * libm::sin(f64::from(j) / 9.0) + noise
                };
            }
        }
        s
    }

    #[test]
    fn the_strips_fill_as_the_one_flood_does() {
        for (w, h, seed) in [(180, 140, 1), (97, 211, 2), (64, 5, 3), (301, 33, 4)] {
            let mut one = surface(w, h, seed);
            let mut f = Filling::new(&one, 0.0);
            while !f.step(&mut one) {}
            for threads in [2, 3, 8] {
                let mut many = surface(w, h, seed);
                let raised = fill(&mut many, threads);
                assert_eq!(raised, f.raised, "{w} × {h}, {threads}");
                for k in 0..one.len() {
                    assert!(
                        one.z[k].to_bits() == many.z[k].to_bits(),
                        "{w} × {h}, {threads}: cell {k}: {} ≠ {}",
                        one.z[k],
                        many.z[k]
                    );
                }
            }
        }
    }
}

//! Akış yönü (docs/adr/0235 §4): each valid cell flows to the neighbour of
//! steepest descent (f − f')/d, the first in the neighbour order on ties; an
//! outlet with no lower neighbour flows out; the cells of flats take their
//! directions by Barnes, Lehman and Mulla (2014b): a mask 2·l + (U − u) from
//! the breadth-first distances to the flat's lower edges (l) and from its
//! higher edges (u, U the flat's largest), each cell to its same-flat
//! neighbour of least lower mask, the first in order on ties.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::surface::{NOFLOW, NONE, Surface};
use crate::par;

/// D8 directions (0–7, [`NOFLOW`], [`NONE`]) of every cell, rows on `threads`.
pub fn d8(s: &Surface, threads: usize) -> Vec<u8> {
    let w = s.width as usize;
    let mut dirs = vec![NONE; s.len()];
    par::rows(threads, &mut dirs, w, &|first, chunk: &mut [u8]| {
        for (r, row) in chunk.chunks_mut(w).enumerate() {
            let j = first + r;
            let step = &s.row(j).step;
            for (i, out) in row.iter_mut().enumerate() {
                let k = j * w + i;
                let zc = s.z[k];
                if zc.is_nan() {
                    continue;
                }
                let (mut best, mut at, mut open) = (0.0, NOFLOW, NOFLOW);
                for (q, &len) in step.iter().enumerate() {
                    match s.at(i, j, k, q) {
                        Some(m) if !s.z[m].is_nan() => {
                            let d = (zc - s.z[m]) / len;
                            if d > best {
                                best = d;
                                at = q as u8;
                            }
                        }
                        _ => {
                            if open == NOFLOW {
                                open = q as u8;
                            }
                        }
                    }
                }
                *out = if at != NOFLOW { at } else { open };
            }
        }
    });
    dirs
}

/// The cell a direction leads to: none out of the raster, into a cell without a value, or without a direction.
#[inline]
pub fn receiver(s: &Surface, dirs: &[u8], k: usize) -> Option<usize> {
    let d = dirs[k];
    if d >= NOFLOW {
        return None;
    }
    let w = s.width as usize;
    s.neighbour(k % w, k / w, d as usize)
}

/// Gives the drainable flats' cells their directions; the cells resolved.
pub fn resolve_flats(s: &Surface, dirs: &mut [u8], threads: usize) -> u64 {
    let w = s.width as usize;
    let h = s.height as usize;
    let n = s.len();
    // Step 1: the edges, row by row.
    let rows: Vec<usize> = (0..h).collect();
    let edges = par::map(threads, &rows, &|&j| {
        let (mut low, mut high) = (Vec::new(), Vec::new());
        for i in 0..w {
            let k = j * w + i;
            let d = dirs[k];
            if d == NONE {
                continue;
            }
            let zc = s.z[k];
            let mut hit = false;
            if d != NOFLOW {
                s.each(i, j, k, |_, m| hit |= dirs[m] == NOFLOW && s.z[m] == zc);
                if hit {
                    low.push(k as u32);
                }
            } else {
                s.each(i, j, k, |_, m| hit |= s.z[m] > zc);
                if hit {
                    high.push(k as u32);
                }
            }
        }
        (low, high)
    });
    let mut low = Vec::new();
    let mut high = Vec::new();
    for (l, hi) in edges {
        low.extend(l);
        high.extend(hi);
    }
    if low.is_empty() {
        return 0;
    }
    // The flats: from each lower edge, the cells of its height joined to it.
    let mut label = vec![0u32; n];
    let mut count = 0u32;
    let mut queue: Vec<u32> = Vec::new();
    for &seed in &low {
        if label[seed as usize] != 0 {
            continue;
        }
        count += 1;
        let zs = s.z[seed as usize];
        label[seed as usize] = count;
        queue.clear();
        queue.push(seed);
        while let Some(k) = queue.pop() {
            let k = k as usize;
            s.each(k % w, k / w, k, |_, m| {
                if label[m] == 0 && s.z[m] == zs {
                    label[m] = count;
                    queue.push(m as u32);
                }
            });
        }
    }
    high.retain(|&k| label[k as usize] != 0);
    // The edges by their flats (counting sort): each flat's distances are its own, so the flats go on the threads.
    let group = |edges: &[u32]| -> (Vec<u32>, Vec<u32>) {
        let mut start = vec![0u32; count as usize + 2];
        for &k in edges {
            start[label[k as usize] as usize + 1] += 1;
        }
        for x in 1..start.len() {
            start[x] += start[x - 1];
        }
        let mut at = start.clone();
        let mut out = vec![0u32; edges.len()];
        for &k in edges {
            let l = label[k as usize] as usize;
            out[at[l] as usize] = k;
            at[l] += 1;
        }
        (start, out)
    };
    let (low_at, low_by) = group(&low);
    let (high_at, high_by) = group(&high);
    let away: Vec<AtomicU32> = (0..n).map(|_| AtomicU32::new(0)).collect();
    let mask: Vec<AtomicU32> = (0..n).map(|_| AtomicU32::new(0)).collect();
    // Steps 2 and 3, flat by flat: the distances from the higher edges (each flat's largest), then from the lower
    // edges and the mask 2·l + (U − u).
    let flat = |l: usize, front: &mut Vec<u32>, next: &mut Vec<u32>| {
        let inner = |m: usize| label[m] as usize == l && dirs[m] == NOFLOW;
        front.clear();
        for &k in &high_by[high_at[l] as usize..high_at[l + 1] as usize] {
            if away[k as usize].load(Ordering::Relaxed) == 0 {
                away[k as usize].store(1, Ordering::Relaxed);
                front.push(k);
            }
        }
        let (mut level, mut top) = (1u32, 0u32);
        while !front.is_empty() {
            top = level;
            next.clear();
            for &k in front.iter() {
                let k = k as usize;
                s.each(k % w, k / w, k, |_, m| {
                    if inner(m) && away[m].load(Ordering::Relaxed) == 0 {
                        away[m].store(level + 1, Ordering::Relaxed);
                        next.push(m as u32);
                    }
                });
            }
            std::mem::swap(front, next);
            level += 1;
        }
        let pay = |m: usize| match away[m].load(Ordering::Relaxed) {
            0 => 0,
            u => top - u,
        };
        front.clear();
        for &k in &low_by[low_at[l] as usize..low_at[l + 1] as usize] {
            let k = k as usize;
            if mask[k].load(Ordering::Relaxed) == 0 {
                mask[k].store(2 + pay(k), Ordering::Relaxed);
                front.push(k as u32);
            }
        }
        let mut level = 1u32;
        while !front.is_empty() {
            next.clear();
            for &k in front.iter() {
                let k = k as usize;
                s.each(k % w, k / w, k, |_, m| {
                    if inner(m) && mask[m].load(Ordering::Relaxed) == 0 {
                        mask[m].store(2 * (level + 1) + pay(m), Ordering::Relaxed);
                        next.push(m as u32);
                    }
                });
            }
            std::mem::swap(front, next);
            level += 1;
        }
    };
    // The flats in runs of about equal edges, a run a piece of work.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let per = (low.len() + high.len()).div_ceil(threads.max(1) * 8).max(1);
    let (mut from, mut held) = (1usize, 0usize);
    for l in 1..=count as usize {
        held += (low_at[l + 1] - low_at[l] + high_at[l + 1] - high_at[l]) as usize;
        if held >= per || l == count as usize {
            runs.push((from, l + 1));
            from = l + 1;
            held = 0;
        }
    }
    par::map(threads, &runs, &|&(a, b)| {
        let (mut front, mut next) = (Vec::new(), Vec::new());
        for l in a..b {
            flat(l, &mut front, &mut next);
        }
    });
    drop(away);
    // Step 4: each flat cell to its same-flat neighbour of least lower mask, rows on the threads.
    let resolved = AtomicU64::new(0);
    let mask_of = |m: usize| mask[m].load(Ordering::Relaxed);
    par::rows(threads, dirs, w, &|first, chunk: &mut [u8]| {
        let mut got = 0u64;
        for (r, row) in chunk.chunks_mut(w).enumerate() {
            let j = first + r;
            for (i, d) in row.iter_mut().enumerate() {
                let k = j * w + i;
                if *d != NOFLOW || label[k] == 0 || mask_of(k) == 0 {
                    continue;
                }
                let (mut best, mut at) = (mask_of(k), NOFLOW);
                s.each(i, j, k, |q, m| {
                    let mm = mask_of(m);
                    if label[m] == label[k] && mm != 0 && mm < best {
                        best = mm;
                        at = q as u8;
                    }
                });
                if at != NOFLOW {
                    *d = at;
                    got += 1;
                }
            }
        }
        resolved.fetch_add(got, Ordering::Relaxed);
    });
    resolved.into_inner()
}

//! Döküm noktası, Noktadan havza and Havzalar (docs/adr/0235 §7–§9): pour
//! points snapped to the largest D8 accumulation near them, each cell's
//! basin by the first labelled cell its D8 path meets (labels spread
//! upstream from the pour cells, the terminal cells or the streams), the
//! route crossings' whole basins, and every basin's area and rings.

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::vec2::Vec2;

use super::flow::receiver;
use super::surface::{N, Surface, back, length};
use crate::inputs::place_in;
use crate::rasterize::{chord_paths, segment_cells};
use crate::vector::label::Regions;
use crate::vector::rings::{Ring, region_rings};

/// A pour cell and its distance from the point (m); none when no cell takes the point.
pub fn snap(s: &Surface, cells: &[f64], (x, y): (f64, f64), r: f64) -> Option<(usize, f64)> {
    let (u, v) = place_in(&s.affine, x, y);
    let j0 = v.floor();
    let axes = s.axes_at(j0 as i64);
    let w = s.width as usize;
    let dist = |i: usize, j: usize| length(axes, i as f64 + 0.5 - u, j as f64 + 0.5 - v);
    if r == 0.0 {
        let i0 = u.floor();
        if i0 < 0.0 || j0 < 0.0 || i0 >= f64::from(s.width) || j0 >= f64::from(s.height) {
            return None;
        }
        let (i, j) = (i0 as usize, j0 as usize);
        let k = j * w + i;
        return s.valid(k).then(|| (k, dist(i, j)));
    }
    // The cells within reach: the cell-space box of the metric ball, a cell more each way.
    let [a, b, c, d] = axes;
    let det = a * d - b * c;
    let reach_u = r * (d * d + b * b).sqrt() / det.abs() + 1.0;
    let reach_v = r * (c * c + a * a).sqrt() / det.abs() + 1.0;
    let (i_lo, i_hi) = ((u - reach_u).floor(), (u + reach_u).floor());
    let (j_lo, j_hi) = ((v - reach_v).floor(), (v + reach_v).floor());
    let clamp = |x: f64, n: u32| x.max(0.0).min(f64::from(n) - 1.0) as usize;
    if i_hi < 0.0 || j_hi < 0.0 || i_lo >= f64::from(s.width) || j_lo >= f64::from(s.height) {
        return None;
    }
    let mut best: Option<(f64, f64, usize)> = None;
    for j in clamp(j_lo, s.height)..=clamp(j_hi, s.height) {
        for i in clamp(i_lo, s.width)..=clamp(i_hi, s.width) {
            let k = j * w + i;
            if !s.valid(k) {
                continue;
            }
            let dd = dist(i, j);
            if dd > r {
                continue;
            }
            let better = match best {
                None => true,
                Some((bc, bd, _)) => cells[k] > bc || (cells[k] == bc && dd < bd),
            };
            if better {
                best = Some((cells[k], dd, k));
            }
        }
    }
    best.map(|(_, dd, k)| (k, dd))
}

/// Spreads labels upstream from the labelled cells: every unlabelled cell
/// whose D8 direction leads to a labelled one takes its label.
pub fn label_upstream(s: &Surface, dirs: &[u8], labels: &mut [u32], seeds: &[u32]) {
    let w = s.width as usize;
    let mut stack: Vec<u32> = seeds.to_vec();
    while let Some(k) = stack.pop() {
        let k = k as usize;
        let (i, j) = (k % w, k / w);
        let l = labels[k];
        for q in 0..8 {
            if let Some(m) = s.neighbour(i, j, q)
                && labels[m] == 0
                && dirs[m] as usize == back(q)
            {
                labels[m] = l;
                stack.push(m as u32);
            }
        }
    }
}

/// The cells whose D8 path passes through `start` (it included).
pub fn upstream(s: &Surface, dirs: &[u8], start: usize, out: &mut Vec<u32>) {
    let w = s.width as usize;
    out.clear();
    out.push(start as u32);
    let mut at = 0;
    while at < out.len() {
        let k = out[at] as usize;
        at += 1;
        let (i, j) = (k % w, k / w);
        for q in 0..8 {
            if let Some(m) = s.neighbour(i, j, q)
                && dirs[m] as usize == back(q)
            {
                out.push(m as u32);
            }
        }
    }
}

/// Each label's area (m²): its cells' areas added row by row.
pub fn areas(s: &Surface, labels: &[u32], count: usize) -> Vec<f64> {
    let w = s.width as usize;
    let mut out = vec![0.0; count];
    for (k, &l) in labels.iter().enumerate() {
        if l != 0 {
            out[l as usize - 1] += s.row(k / w).area;
        }
    }
    out
}

/// The area of a set of cells, row by row.
pub fn area_of(s: &Surface, cells: &mut [u32]) -> f64 {
    let w = s.width as usize;
    cells.sort_unstable();
    let mut a = 0.0;
    for &k in cells.iter() {
        a += s.row(k as usize / w).area;
    }
    a
}

/// The rings of every label 1..=count of `labels` (`width` wide).
pub fn rings_of(
    labels: Vec<u32>,
    width: u32,
    height: u32,
    count: usize,
) -> Result<Vec<Vec<Ring>>, String> {
    region_rings(&Regions {
        width,
        height,
        labels,
        values: vec![0.0; count],
    })
}

/// A set of cells' rings: a label raster over their box, the rings moved back.
pub fn cells_rings(s: &Surface, cells: &[u32]) -> Result<Vec<Ring>, String> {
    let w = s.width as usize;
    let (mut i0, mut j0, mut i1, mut j1) = (usize::MAX, usize::MAX, 0, 0);
    for &k in cells {
        let (i, j) = (k as usize % w, k as usize / w);
        i0 = i0.min(i);
        j0 = j0.min(j);
        i1 = i1.max(i);
        j1 = j1.max(j);
    }
    let (bw, bh) = (i1 - i0 + 1, j1 - j0 + 1);
    let mut labels = vec![0u32; bw * bh];
    for &k in cells {
        let (i, j) = (k as usize % w, k as usize / w);
        labels[(j - j0) * bw + (i - i0)] = 1;
    }
    let mut rings = rings_of(labels, bw as u32, bh as u32, 1)?;
    let mut out = rings.pop().unwrap_or_default();
    for r in &mut out {
        for p in r.iter_mut() {
            p[0] += i0 as i64;
            p[1] += j0 as i64;
        }
    }
    Ok(out)
}

/// The cells a route's 0.1 mm chords pass through (docs/adr/0234 §3), sorted.
pub fn crossed_cells(s: &Surface, route: &Shape) -> Vec<u32> {
    let mut paths = Vec::new();
    chord_paths(route, &mut paths);
    let w = s.width as usize;
    let mut out: Vec<u32> = Vec::new();
    for path in &paths {
        let cs: Vec<Vec2> = path
            .iter()
            .map(|p| {
                let (u, v) = place_in(&s.affine, p.x, p.y);
                Vec2::new(u, v)
            })
            .collect();
        for pair in cs.windows(2) {
            segment_cells(pair[0], pair[1], (s.width, s.height), &mut |i, j| {
                out.push((j as usize * w + i as usize) as u32);
            });
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Terminal cells (no D8 receiver) in raster order.
pub fn terminals(s: &Surface, dirs: &[u8]) -> Vec<u32> {
    (0..s.len())
        .filter(|&k| s.valid(k) && receiver(s, dirs, k).is_none())
        .map(|k| k as u32)
        .collect()
}

/// The direction table, for the tests.
#[allow(dead_code)]
pub(crate) fn neighbours() -> [(i64, i64); 8] {
    N
}

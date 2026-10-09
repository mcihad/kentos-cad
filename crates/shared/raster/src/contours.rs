//! Eş yükselti eğrileri (docs/adr/0231 §9): marching squares over the cell
//! centres, row after row. A square's corners are four neighbouring cell
//! centres; a corner at or above the level is inside; each piece is drawn
//! with the high side on its left; a saddle's centre (the corners' mean)
//! decides whether its inside corners join. A crossing on an edge is worked
//! out once, from the edge's left (or top) corner, so the two squares
//! sharing it give the same point; pieces join by their edges' ids. A
//! square with a nodata corner is skipped: the line ends there.

use std::collections::HashMap;

use crate::frame::Frame;

/// The settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub interval: f64,
    pub base: f64,
    /// Ana eğri: k a multiple of this (1: every line).
    pub index_every: u32,
    /// Douglas-Peucker's tolerance (metres); 0 off.
    pub simplify: f64,
}

/// The most levels a run may cross, and the most vertices it may write (docs/adr/0231 §2).
pub const MOST_LEVELS: i64 = 10_000;
pub const MOST_VERTICES: usize = 5_000_000;

/// A line: its level's index k (L = base + k·interval), its value and points.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub k: i64,
    pub value: f64,
    /// Ana eğri.
    pub index: bool,
    pub pts: Vec<[f64; 2]>,
}

#[derive(Clone, Copy, Debug)]
struct Seg {
    k: i64,
    start: u64,
    end: u64,
    sp: [f64; 2],
    ep: [f64; 2],
}

/// The pass's state: the row above and the pieces so far.
#[derive(Debug)]
pub struct Contourer {
    spec: Spec,
    frame: Frame,
    /// The pixel frame is the drawing's mirror image (det(J) > 0): every piece turned.
    flip: bool,
    width: usize,
    above: Option<(u32, Vec<f64>)>,
    segs: Vec<Seg>,
    kmin: i64,
    kmax: i64,
}

impl Spec {
    /// The level k's value.
    #[inline]
    pub fn level(&self, k: i64) -> f64 {
        self.base + k as f64 * self.interval
    }

    /// The fewest decimals (at most 6) that write each level whole: the
    /// interval's and the base's (docs/adr/0231 §9).
    pub fn decimals(&self) -> usize {
        fn of(x: f64) -> usize {
            let mut scale = 1.0;
            for d in 0..=6 {
                let v = x * scale;
                if libm::fabs(v - libm::round(v)) <= 1e-9 * libm::fabs(v).max(1.0) {
                    return d;
                }
                scale *= 10.0;
            }
            6
        }
        of(self.interval).max(of(self.base))
    }

    /// A level's Kot as the drawing writes it: the display rule (docs/adr/0149) with [`Spec::decimals`].
    pub fn level_text(&self, value: f64) -> String {
        kentos_geometry_core::display::fixed(value, self.decimals())
    }

    /// The settings' problem, if any.
    pub fn problem(&self) -> Option<String> {
        if !(self.interval > 0.0 && self.interval.is_finite()) {
            return Some("Eğri aralığı sıfırdan büyük olmalı.".into());
        }
        if !self.base.is_finite() {
            return Some("Taban kotu bir sayı olmalı.".into());
        }
        if self.index_every == 0 {
            return Some("Ana eğri sıklığı en az 1 olmalı.".into());
        }
        if !(self.simplify >= 0.0 && self.simplify.is_finite()) {
            return Some("Sadeleştirme toleransı sıfır ya da artı olmalı.".into());
        }
        None
    }
}

impl Contourer {
    /// Its settings.
    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    pub fn new(spec: Spec, frame: Frame) -> Contourer {
        let [_, a, b, _, c, d] = frame.affine;
        Contourer {
            spec,
            flip: a * d - b * c > 0.0,
            width: frame.width as usize,
            frame,
            above: None,
            segs: Vec::new(),
            kmin: i64::MAX,
            kmax: i64::MIN,
        }
    }

    /// Takes row `j` (the raster's width, NaN nodata), the next after the last.
    pub fn push(&mut self, j: u32, row: &[f64]) -> Result<(), String> {
        if let Some((ja, above)) = self.above.take()
            && ja + 1 == j
        {
            self.squares(ja, &above, row)?;
        }
        self.above = Some((j, row.to_vec()));
        Ok(())
    }

    /// The edge ids: horizontal edge from cell (i, j) to (i + 1, j), vertical from (i, j) to (i, j + 1).
    #[inline]
    fn h_edge(&self, i: usize, j: u32) -> u64 {
        2 * (u64::from(j) * self.width as u64 + i as u64)
    }

    #[inline]
    fn v_edge(&self, i: usize, j: u32) -> u64 {
        2 * (u64::from(j) * self.width as u64 + i as u64) + 1
    }

    /// The drawing point of pixel coordinates.
    #[inline]
    fn point(&self, px: f64, py: f64) -> [f64; 2] {
        let (x, y) = self.frame.point(px, py);
        [x, y]
    }

    fn squares(&mut self, j: u32, top: &[f64], bottom: &[f64]) -> Result<(), String> {
        let s = self.spec;
        for i in 0..self.width.saturating_sub(1) {
            let z = [top[i], top[i + 1], bottom[i + 1], bottom[i]];
            if z.iter().any(|v| v.is_nan()) {
                continue;
            }
            let mut lo = z[0];
            let mut hi = z[0];
            for &v in &z[1..] {
                if v < lo {
                    lo = v;
                }
                if v > hi {
                    hi = v;
                }
            }
            // The levels lo < L ≤ hi: from just below lo upward; a square
            // spanning more levels than a run may have refuses at once.
            let below = libm::floor((lo - s.base) / s.interval);
            if libm::floor((hi - s.base) / s.interval) - below > MOST_LEVELS as f64 {
                return Err(format!(
                    "Bu aralıkla {MOST_LEVELS}'den çok düzey çıkıyor; eğri aralığını büyütün (ya da rasterin nodata değerini Raster stili'nde verin)."
                ));
            }
            let mut k = below as i64 - 1;
            while s.level(k) <= hi {
                let l = s.level(k);
                if l > lo {
                    self.square(i, j, &z, k, l)?;
                }
                k += 1;
            }
        }
        Ok(())
    }

    /// The crossing on the square's edge `e` (0 top, 1 right, 2 bottom, 3 left) of level `l`.
    fn crossing(&self, i: usize, j: u32, z: &[f64; 4], e: usize, l: f64) -> (u64, [f64; 2]) {
        let (fi, fj) = (i as f64 + 0.5, f64::from(j) + 0.5);
        match e {
            // Top: from the left corner (TL) to the right (TR).
            0 => {
                let t = (l - z[0]) / (z[1] - z[0]);
                (self.h_edge(i, j), self.point(fi + t, fj))
            }
            // Right: from the top corner (TR) down to BR.
            1 => {
                let t = (l - z[1]) / (z[2] - z[1]);
                (self.v_edge(i + 1, j), self.point(fi + 1.0, fj + t))
            }
            // Bottom: from the left corner (BL) to the right (BR).
            2 => {
                let t = (l - z[3]) / (z[2] - z[3]);
                (self.h_edge(i, j + 1), self.point(fi + t, fj + 1.0))
            }
            // Left: from the top corner (TL) down to BL.
            _ => {
                let t = (l - z[0]) / (z[3] - z[0]);
                (self.v_edge(i, j), self.point(fi, fj + t))
            }
        }
    }

    fn square(&mut self, i: usize, j: u32, z: &[f64; 4], k: i64, l: f64) -> Result<(), String> {
        let inside = [z[0] >= l, z[1] >= l, z[2] >= l, z[3] >= l];
        // Pieces as (start edge, end edge), in the order of their start edges.
        let mut pieces: [(usize, usize); 2] = [(0, 0); 2];
        let mut n = 0;
        if inside[0] == inside[2] && inside[1] == inside[3] && inside[0] != inside[1] {
            // A saddle: the centre (the corners' mean) decides.
            let joined = (z[0] + z[1] + z[2] + z[3]) / 4.0 >= l;
            let p = match (inside[0], joined) {
                // TL and BR inside.
                (true, true) => [(1, 0), (3, 2)],
                (true, false) => [(1, 2), (3, 0)],
                // TR and BL inside.
                (false, true) => [(0, 3), (2, 1)],
                (false, false) => [(0, 1), (2, 3)],
            };
            pieces = p;
            n = 2;
        } else {
            let mut start = None;
            let mut end = None;
            for e in 0..4 {
                if inside[e] != inside[(e + 1) % 4] {
                    // The start is the edge whose clockwise-next corner is inside.
                    if inside[(e + 1) % 4] {
                        start = Some(e);
                    } else {
                        end = Some(e);
                    }
                }
            }
            if let (Some(a), Some(b)) = (start, end) {
                pieces[0] = (a, b);
                n = 1;
            }
        }
        if self.flip {
            for p in pieces.iter_mut().take(n) {
                *p = (p.1, p.0);
            }
            if n == 2 && pieces[1].0 < pieces[0].0 {
                pieces.swap(0, 1);
            }
        }
        for &(a, b) in pieces.iter().take(n) {
            let (start, sp) = self.crossing(i, j, z, a, l);
            let (end, ep) = self.crossing(i, j, z, b, l);
            self.segs.push(Seg {
                k,
                start,
                end,
                sp,
                ep,
            });
        }
        if k < self.kmin {
            self.kmin = k;
        }
        if k > self.kmax {
            self.kmax = k;
        }
        if self.kmax - self.kmin + 1 > MOST_LEVELS {
            return Err(format!(
                "Bu aralıkla {MOST_LEVELS}'den çok düzey çıkıyor; eğri aralığını büyütün."
            ));
        }
        if self.segs.len() > MOST_VERTICES {
            return Err(format!(
                "Eğriler {} milyondan çok köşe tutuyor; eğri aralığını büyütün ya da Sadeleştir'i açın.",
                MOST_VERTICES / 1_000_000
            ));
        }
        Ok(())
    }

    /// The lines: levels ascending; in a level, open lines first by their
    /// first piece's place, then closed ones.
    pub fn finish(self) -> Result<Vec<Line>, String> {
        let spec = self.spec;
        // The pieces of each level, in the order they were made.
        let mut by_level: Vec<(i64, Vec<usize>)> = Vec::new();
        {
            let mut index: HashMap<i64, usize> = HashMap::new();
            for (n, s) in self.segs.iter().enumerate() {
                let at = *index.entry(s.k).or_insert_with(|| {
                    by_level.push((s.k, Vec::new()));
                    by_level.len() - 1
                });
                by_level[at].1.push(n);
            }
        }
        by_level.sort_by_key(|(k, _)| *k);
        let mut out = Vec::new();
        let mut vertices = 0usize;
        for (k, list) in by_level {
            let value = spec.level(k);
            let starts: HashMap<u64, usize> =
                list.iter().map(|&n| (self.segs[n].start, n)).collect();
            let ends: std::collections::HashSet<u64> =
                list.iter().map(|&n| self.segs[n].end).collect();
            let mut used = vec![false; self.segs.len()];
            let mut chains: Vec<Vec<[f64; 2]>> = Vec::new();
            // Open lines: from pieces whose start ends no piece.
            for &n in &list {
                if used[n] || ends.contains(&self.segs[n].start) {
                    continue;
                }
                chains.push(chain(&self.segs, &starts, &mut used, n));
            }
            // Closed lines: the rest, from the first left.
            for &n in &list {
                if !used[n] {
                    chains.push(chain(&self.segs, &starts, &mut used, n));
                }
            }
            for pts in chains {
                let mut pts = dedup(pts);
                if pts.len() < 2 {
                    continue;
                }
                if spec.simplify > 0.0 {
                    pts = simplify(&pts, spec.simplify);
                }
                vertices += pts.len();
                if vertices > MOST_VERTICES {
                    return Err(format!(
                        "Eğriler {} milyondan çok köşe tutuyor; eğri aralığını büyütün ya da Sadeleştir'i açın.",
                        MOST_VERTICES / 1_000_000
                    ));
                }
                out.push(Line {
                    k,
                    value,
                    index: k.rem_euclid(i64::from(spec.index_every)) == 0,
                    pts,
                });
            }
        }
        Ok(out)
    }
}

/// The chain from piece `first`: its start, then each piece's end, until
/// no piece starts there or the chain comes back to `first`.
fn chain(
    segs: &[Seg],
    starts: &HashMap<u64, usize>,
    used: &mut [bool],
    first: usize,
) -> Vec<[f64; 2]> {
    let mut pts = vec![segs[first].sp];
    let mut cur = first;
    used[cur] = true;
    loop {
        pts.push(segs[cur].ep);
        match starts.get(&segs[cur].end) {
            Some(&next) if next == first => break,
            Some(&next) if !used[next] => {
                used[next] = true;
                cur = next;
            }
            _ => break,
        }
    }
    pts
}

/// The points without one equal to the one before it.
fn dedup(pts: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    out
}

/// The distance from `p` to the segment `a`–`b` (to `a` when they meet).
fn to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let (qx, qy) = if len2 == 0.0 {
        (a[0], a[1])
    } else {
        let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0);
        (a[0] + t * dx, a[1] + t * dy)
    };
    libm::sqrt((p[0] - qx) * (p[0] - qx) + (p[1] - qy) * (p[1] - qy))
}

/// Douglas-Peucker: the ends kept; a run split at its farthest point (the
/// first of equals) while that point is farther than `tol`.
pub fn simplify(pts: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((a, b)) = stack.pop() {
        let mut far = 0.0;
        let mut at = 0;
        for k in a + 1..b {
            let d = to_segment(pts[k], pts[a], pts[b]);
            if d > far {
                far = d;
                at = k;
            }
        }
        if at != 0 && far > tol {
            keep[at] = true;
            stack.push((at, b));
            stack.push((a, at));
        }
    }
    pts.iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

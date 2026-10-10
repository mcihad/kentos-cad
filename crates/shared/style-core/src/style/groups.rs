//! Kümeleme's and Yayma's groups (docs/adr/0213 §2.7, §2.8): points in
//! their order join the group whose centre is nearest within `d` (the
//! first made of equals), else start one; a group's centre is the mean of
//! its points (QGIS's point distance renderer, with a round distance). The
//! centres are looked for in a grid of `d`-sided cells. And where Yayma puts
//! a group's points around its centre.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use kentos_geometry_core::Vec2;
use kentos_geometry_core::jsmath::{PI, cos, js_max, js_min, sin};

use super::place::positive;

/// A group: its points (indices into the input), their sums and their mean, kept as points join.
#[derive(Clone, Debug)]
pub struct Group {
    pub members: Vec<usize>,
    sum: Vec2,
    centre: Vec2,
}

impl Group {
    fn of(i: usize, p: Vec2) -> Group {
        Group {
            members: vec![i],
            sum: p,
            centre: p,
        }
    }

    /// Its points' mean.
    pub fn centre(&self) -> Vec2 {
        self.centre
    }

    fn join(&mut self, i: usize, p: Vec2) {
        self.members.push(i);
        self.sum = Vec2::new(self.sum.x + p.x, self.sum.y + p.y);
        let n = self.members.len() as f64;
        self.centre = Vec2::new(self.sum.x / n, self.sum.y / n);
    }
}

fn cell(p: Vec2, d: f64) -> (i64, i64) {
    ((p.x / d).floor() as i64, (p.y / d).floor() as i64)
}

/// The grid's cells hashed fast and the same everywhere (rustc's FxHash): every point
/// asks nine cells, and the map's order is never read.
#[derive(Default)]
struct CellHasher(u64);

impl Hasher for CellHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.add(u64::from(b));
        }
    }

    fn write_i64(&mut self, i: i64) {
        self.add(i as u64);
    }
}

impl CellHasher {
    fn add(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

type Cells = HashMap<(i64, i64), Vec<usize>, BuildHasherDefault<CellHasher>>;

/// The groups of `points` within `d` (world), in the order they were made.
pub fn group(points: &[Vec2], d: f64) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    if !positive(d) || !d.is_finite() {
        return points
            .iter()
            .enumerate()
            .map(|(i, p)| Group::of(i, *p))
            .collect();
    }
    let mut grid = Cells::default();
    let d2 = d * d;
    for (i, &p) in points.iter().enumerate() {
        let (cx, cy) = cell(p, d);
        let mut best: Option<(f64, usize)> = None;
        for gx in cx - 1..=cx + 1 {
            for gy in cy - 1..=cy + 1 {
                let Some(list) = grid.get(&(gx, gy)) else {
                    continue;
                };
                for &g in list {
                    let c = groups[g].centre();
                    let dist = (c.x - p.x) * (c.x - p.x) + (c.y - p.y) * (c.y - p.y);
                    if dist <= d2 && best.is_none_or(|(bd, bg)| dist < bd || (dist == bd && g < bg))
                    {
                        best = Some((dist, g));
                    }
                }
            }
        }
        match best {
            Some((_, g)) => {
                let before = cell(groups[g].centre(), d);
                groups[g].join(i, p);
                let after = cell(groups[g].centre(), d);
                if after != before {
                    if let Some(list) = grid.get_mut(&before) {
                        list.retain(|&k| k != g);
                    }
                    grid.entry(after).or_default().push(g);
                }
            }
            None => {
                groups.push(Group::of(i, p));
                grid.entry((cx, cy)).or_default().push(groups.len() - 1);
            }
        }
    }
    groups
}

/// Where Yayma puts a group's `n` points, in screen pixels from its centre
/// (x right, y up), from the north clockwise. `s`: the largest symbol's
/// diagonal; `c`: the centre symbol's (0 without one); `spacing`: the gap
/// added. Also the radii of the rings drawn round them.
pub fn displaced(
    placement: super::model::Placement,
    n: usize,
    s: f64,
    c: f64,
    spacing: f64,
) -> (Vec<Vec2>, Vec<f64>) {
    use super::model::Placement;
    let bearing = |r: f64, a: f64| Vec2::new(r * sin(a), r * cos(a));
    let mut out = Vec::with_capacity(n);
    let mut rings = Vec::new();
    match placement {
        Placement::Ring => {
            let r = js_max(s / 2.0, n as f64 * s / (2.0 * PI)) + spacing;
            for k in 0..n {
                out.push(bearing(r, 2.0 * PI * k as f64 / n as f64));
            }
            rings.push(r);
        }
        Placement::Rings => {
            let mut r = c / 2.0 + s / 2.0 + spacing;
            let mut left = n;
            while left > 0 {
                let fits = ((2.0 * PI * r / s).floor() as usize).max(1).min(left);
                for k in 0..fits {
                    out.push(bearing(r, 2.0 * PI * k as f64 / fits as f64));
                }
                rings.push(r);
                left -= fits;
                r += s + spacing;
            }
        }
        Placement::Grid => {
            let cols = js_max((n as f64).sqrt().ceil(), 1.0) as usize;
            let rows = n.div_ceil(cols);
            let a = (c / 2.0 + s / 2.0 + s) / 2.0 + spacing;
            let (w, h) = ((cols - 1) as f64 * a, (rows - 1) as f64 * a);
            for k in 0..n {
                let (col, row) = (k % cols, k / cols);
                out.push(Vec2::new(
                    col as f64 * a - w / 2.0,
                    h / 2.0 - row as f64 * a,
                ));
            }
        }
    }
    (out, rings)
}

/// Kümeleme's symbol grows with its count `n` when asked: `min(2, 1 + ln n / ln 1000)`.
pub fn growth(n: usize) -> f64 {
    let n = n.max(1) as f64;
    js_min(
        1.0 + kentos_geometry_core::jsmath::log(n) / kentos_geometry_core::jsmath::log(1000.0),
        2.0,
    )
}

//! Seyrelt (docs/adr/0207 §7): Hücreyle (in each 3D cell of the grid of
//! `size` metres from the origin, the point nearest the cell's centre; on a
//! tie the one met first), Yarıçapla (in the file's order, a point nearer
//! than `size` to one kept is dropped; 3D distances) and Her n'inci (the
//! first point and every n-th after it). Hücreyle decides in a first pass
//! and keeps in a second; the others keep as they read.

use std::collections::HashMap;

use super::ground::world;
use crate::record::Layout;

/// How Seyrelt keeps points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Method {
    Cell(f64),
    Radius(f64),
    Nth(u64),
}

/// Hücreyle's first pass: each cell's nearest point so far, by its number in the file.
#[derive(Clone, Debug, Default)]
pub struct Cells {
    size: f64,
    best: HashMap<[i64; 3], (u64, f64)>,
}

impl Cells {
    pub fn new(size: f64) -> Option<Cells> {
        (size > 0.0 && size.is_finite()).then(|| Cells {
            size,
            best: HashMap::new(),
        })
    }

    /// Records numbered from `first` in the file's order.
    pub fn feed(
        &mut self,
        layout: &Layout,
        records: &[u8],
        scale: [f64; 3],
        offset: [f64; 3],
        first: u64,
    ) {
        let s = self.size;
        for (i, r) in records.chunks_exact(layout.len).enumerate() {
            let p = world(layout, r, scale, offset);
            let k = [
                (p[0] / s).floor() as i64,
                (p[1] / s).floor() as i64,
                (p[2] / s).floor() as i64,
            ];
            let mut d = 0.0;
            for a in 0..3 {
                let c = (k[a] as f64 + 0.5) * s;
                d += (p[a] - c) * (p[a] - c);
            }
            let n = first + i as u64;
            match self.best.get_mut(&k) {
                Some(b) if d < b.1 => *b = (n, d),
                Some(_) => {}
                None => {
                    self.best.insert(k, (n, d));
                }
            }
        }
    }

    /// The numbers of the points kept, in the file's order.
    pub fn kept(&self) -> Vec<u64> {
        let mut v: Vec<u64> = self.best.values().map(|b| b.0).collect();
        v.sort_unstable();
        v
    }
}

/// Yarıçapla: the points kept so far, in a 3D grid of `r` metres.
#[derive(Clone, Debug, Default)]
pub struct Radius {
    r: f64,
    kept: HashMap<[i64; 3], Vec<[f64; 3]>>,
}

impl Radius {
    pub fn new(r: f64) -> Option<Radius> {
        (r > 0.0 && r.is_finite()).then(|| Radius {
            r,
            kept: HashMap::new(),
        })
    }

    /// Whether each record is kept, in order, into `out` (cleared).
    pub fn feed(
        &mut self,
        layout: &Layout,
        records: &[u8],
        scale: [f64; 3],
        offset: [f64; 3],
        out: &mut Vec<bool>,
    ) {
        out.clear();
        let (r, r2) = (self.r, self.r * self.r);
        for rec in records.chunks_exact(layout.len) {
            let p = world(layout, rec, scale, offset);
            let k = [
                (p[0] / r).floor() as i64,
                (p[1] / r).floor() as i64,
                (p[2] / r).floor() as i64,
            ];
            let mut near = false;
            'look: for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(list) = self.kept.get(&[k[0] + dx, k[1] + dy, k[2] + dz]) {
                            for q in list {
                                let d = (p[0] - q[0]) * (p[0] - q[0])
                                    + (p[1] - q[1]) * (p[1] - q[1])
                                    + (p[2] - q[2]) * (p[2] - q[2]);
                                if d < r2 {
                                    near = true;
                                    break 'look;
                                }
                            }
                        }
                    }
                }
            }
            if !near {
                self.kept.entry(k).or_default().push(p);
            }
            out.push(!near);
        }
    }
}

/// Her n'inci: whether the record numbered `i` is kept.
#[inline]
pub fn nth(i: u64, n: u64) -> bool {
    n <= 1 || i.is_multiple_of(n)
}

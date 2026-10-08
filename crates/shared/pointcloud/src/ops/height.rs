//! Yüksekliğe göre sınıfla (docs/adr/0207 §7): the ground's surface from the
//! ground points (class 2): the least height in each cell, the empty cells
//! filled; each point's height above it, bilinear between the cell centres.
//! A point of class 0 or 1 (or, asked, any but ground) whose height is from
//! the base to the low limit becomes low vegetation (3), to the middle limit
//! medium (4), to the high limit high (5); others stay. Two passes.

use super::grid::Grid;
use super::ground::world;
use crate::record::Layout;

/// Yüksekliğe göre sınıfla's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub cell: f64,
    pub base: f64,
    pub low: f64,
    pub middle: f64,
    pub high: f64,
    /// Every class but ground is classed, not only 0 and 1.
    pub all: bool,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            cell: 1.0,
            base: 0.15,
            low: 0.5,
            middle: 2.0,
            high: 50.0,
            all: false,
        }
    }
}

/// Classing by height over a cloud's plan.
#[derive(Clone, Debug)]
pub struct Height {
    p: Params,
    grid: Grid,
}

impl Height {
    /// For a cloud whose plan is `[x₁, y₁, x₂, y₂]`; none when the values are wrong or the grid too large.
    pub fn new(plan: [f64; 4], p: Params) -> Option<Height> {
        if !(p.cell > 0.0 && p.base <= p.low && p.low <= p.middle && p.middle <= p.high) {
            return None;
        }
        Some(Height {
            p,
            grid: Grid::over(plan, p.cell)?,
        })
    }

    /// Pass 1: the ground points' heights into the cells.
    pub fn feed(&mut self, layout: &Layout, records: &[u8], scale: [f64; 3], offset: [f64; 3]) {
        for r in records.chunks_exact(layout.len) {
            if layout.class(r) != 2 {
                continue;
            }
            let [x, y, z] = world(layout, r, scale, offset);
            self.grid.put_min(x, y, z);
        }
    }

    /// Between the passes: the surface filled; false without ground points.
    pub fn surface(&mut self) -> bool {
        if self.grid.is_empty() {
            return false;
        }
        self.grid.fill();
        true
    }

    /// Pass 2: the classes set in place; how many points each of 3, 4 and 5 took.
    pub fn classify(
        &self,
        layout: &Layout,
        records: &mut [u8],
        scale: [f64; 3],
        offset: [f64; 3],
    ) -> [u64; 3] {
        let mut n = [0u64; 3];
        for r in records.chunks_exact_mut(layout.len) {
            let class = layout.class(r);
            let wanted = if self.p.all { class != 2 } else { class <= 1 };
            if !wanted {
                continue;
            }
            let [x, y, z] = world(layout, r, scale, offset);
            let h = z - self.grid.bilinear(x, y);
            let to = if h < self.p.base || h > self.p.high {
                continue;
            } else if h <= self.p.low {
                3
            } else if h <= self.p.middle {
                4
            } else {
                5
            };
            layout.set_class(r, to);
            n[usize::from(to - 3)] += 1;
        }
        n
    }
}

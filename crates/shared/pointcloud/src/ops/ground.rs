//! Zemin süzgeci (docs/adr/0207 §7): Pingel, Clarke and McBride's simple
//! morphological filter (SMRF, 2013) as the ADR defines it, step for step:
//! the least height of the last returns in each cell; the empty cells
//! filled; the low outliers (a progressive opening of the negated surface,
//! 5 m and slope 1) and then the objects (the progressive opening of the
//! surface, the window and the slope) found; the cells of both emptied and
//! filled again: the ground's surface. A point is ground when it lies within
//! the threshold and the scaled slope of that surface: class 2; a 2 that
//! is not becomes 1; every other class stays. Two passes: the cells, then
//! the classes.

use super::grid::{Grid, fill, progressive};
use crate::record::Layout;

/// Zemin süzgeci's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub cell: f64,
    pub slope: f64,
    pub window: f64,
    pub threshold: f64,
    pub scalar: f64,
    /// Only last (and single) returns make the cells.
    pub last_only: bool,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            cell: 1.0,
            slope: 0.15,
            window: 18.0,
            threshold: 0.5,
            scalar: 1.25,
            last_only: true,
        }
    }
}

/// The low outliers' opening: its window (m) and slope.
const OUTLIER_WINDOW: f64 = 5.0;
const OUTLIER_SLOPE: f64 = 1.0;

/// The filter over a cloud's plan.
#[derive(Clone, Debug)]
pub struct Ground {
    p: Params,
    grid: Grid,
    surface: Option<Grid>,
}

/// A record's world place.
#[inline]
pub(crate) fn world(layout: &Layout, r: &[u8], scale: [f64; 3], offset: [f64; 3]) -> [f64; 3] {
    [
        f64::from(layout.x(r)) * scale[0] + offset[0],
        f64::from(layout.y(r)) * scale[1] + offset[1],
        f64::from(layout.z(r)) * scale[2] + offset[2],
    ]
}

impl Ground {
    /// The filter for a cloud whose plan is `[x₁, y₁, x₂, y₂]`; none when its grid would be too large or the values wrong.
    pub fn new(plan: [f64; 4], p: Params) -> Option<Ground> {
        if !(p.cell > 0.0
            && p.window >= 0.0
            && p.slope >= 0.0
            && p.threshold >= 0.0
            && p.scalar >= 0.0)
        {
            return None;
        }
        Some(Ground {
            p,
            grid: Grid::over(plan, p.cell)?,
            surface: None,
        })
    }

    /// Pass 1: records' heights into the cells.
    pub fn feed(&mut self, layout: &Layout, records: &[u8], scale: [f64; 3], offset: [f64; 3]) {
        for r in records.chunks_exact(layout.len) {
            if self.p.last_only {
                let (ret, n) = layout.returns(r);
                if ret < n {
                    continue;
                }
            }
            let [x, y, z] = world(layout, r, scale, offset);
            self.grid.put_min(x, y, z);
        }
    }

    /// Between the passes: the ground's surface; false when no cell had a point.
    pub fn surface(&mut self) -> bool {
        let g = &self.grid;
        let (cols, rows, cell) = (g.cols, g.rows, g.cell);
        let mut filled = g.v.clone();
        fill(&mut filled, cols, rows);
        if filled.iter().all(|v| v.is_nan()) {
            return false;
        }
        // The low outliers: the negated surface's objects.
        let negated: Vec<f64> = filled.iter().map(|v| -v).collect();
        let outliers = progressive(&negated, cols, rows, cell, OUTLIER_WINDOW, OUTLIER_SLOPE);
        let mut cleared = g.v.clone();
        for (k, &o) in outliers.iter().enumerate() {
            if o {
                cleared[k] = f64::NAN;
            }
        }
        let mut again = cleared.clone();
        fill(&mut again, cols, rows);
        // The objects.
        let objects = progressive(&again, cols, rows, cell, self.p.window, self.p.slope);
        let mut ground = cleared;
        for (k, &o) in objects.iter().enumerate() {
            if o {
                ground[k] = f64::NAN;
            }
        }
        fill(&mut ground, cols, rows);
        if ground.iter().all(|v| v.is_nan()) {
            return false;
        }
        let mut s = self.grid.clone();
        s.v = ground;
        self.surface = Some(s);
        true
    }

    /// Pass 2: the records' classes set in place (`records` of `layout`); the number of ground points.
    pub fn classify(
        &self,
        layout: &Layout,
        records: &mut [u8],
        scale: [f64; 3],
        offset: [f64; 3],
    ) -> u64 {
        let Some(s) = &self.surface else {
            return 0;
        };
        let mut ground = 0;
        for r in records.chunks_exact_mut(layout.len) {
            let [x, y, z] = world(layout, r, scale, offset);
            let surface = s.bilinear(x, y);
            let slope = s.slope(x, y);
            let is_ground = (z - surface).abs() <= self.p.threshold + self.p.scalar * slope;
            if is_ground {
                layout.set_class(r, 2);
                ground += 1;
            } else if layout.class(r) == 2 {
                layout.set_class(r, 1);
            }
        }
        ground
    }
}

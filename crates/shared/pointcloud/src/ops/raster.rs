//! Rasterleştir and Sınır çıkar's grid (docs/adr/0207 §7): cells of `cell`
//! metres on the cell's multiples (the left edge ⌊x₁ / cell⌋ · cell, the top
//! ⌈y₂ / cell⌉ · cell), row 0 at the top, as a GeoTIFF's; a point goes to the
//! cell it falls in (one on an inner edge to the right and the lower row's
//! … the cell below its top edge), clamped into the grid. Rasterleştir's
//! values: the least, the most, the mean, the count, or IDW (the points
//! within the radius of a cell's centre weighted by the inverse square of
//! their distance; a point on the centre is the value). Empty cells are
//! nodata (−9999).

use super::ground::world;
use crate::record::Layout;

/// The value of an empty cell.
pub const NODATA: f64 = -9999.0;
/// The most cells a grid may have.
pub const MAX_CELLS: u64 = 400_000_000;

/// A grid on the cell's multiples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x0: f64,
    pub top: f64,
    pub cell: f64,
    pub cols: usize,
    pub rows: usize,
}

impl Frame {
    /// The grid over the plan `[x₁, y₁, x₂, y₂]`; none when it would be too large or the cell wrong.
    pub fn over(plan: [f64; 4], cell: f64) -> Option<Frame> {
        if !(cell > 0.0 && cell.is_finite() && plan.iter().all(|v| v.is_finite())) {
            return None;
        }
        let x0 = (plan[0] / cell).floor() * cell;
        let top = (plan[3] / cell).ceil() * cell;
        let cols = ((plan[2] - x0) / cell).ceil().max(1.0);
        let rows = ((top - plan[1]) / cell).ceil().max(1.0);
        if cols * rows > MAX_CELLS as f64 {
            return None;
        }
        Some(Frame {
            x0,
            top,
            cell,
            cols: cols as usize,
            rows: rows as usize,
        })
    }

    /// The cell of (`x`, `y`), clamped into the grid.
    #[inline]
    pub fn cell_of(&self, x: f64, y: f64) -> (usize, usize) {
        let c = ((x - self.x0) / self.cell).floor();
        let r = ((self.top - y) / self.cell).floor();
        let c = if c >= 0.0 {
            (c as usize).min(self.cols - 1)
        } else {
            0
        };
        let r = if r >= 0.0 {
            (r as usize).min(self.rows - 1)
        } else {
            0
        };
        (c, r)
    }

    /// A cell's centre.
    #[inline]
    pub fn centre(&self, c: usize, r: usize) -> (f64, f64) {
        (
            self.x0 + (c as f64 + 0.5) * self.cell,
            self.top - (r as f64 + 0.5) * self.cell,
        )
    }

    /// The GeoTIFF's affine: `[x₀, cell, 0, top, 0, −cell]`.
    pub fn affine(&self) -> [f64; 6] {
        [self.x0, self.cell, 0.0, self.top, 0.0, -self.cell]
    }
}

/// Rasterleştir's value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Min,
    Max,
    Mean,
    Count,
    /// Inverse distance weighting within this radius (m).
    Idw(f64),
}

/// The cells being filled.
#[derive(Clone, Debug)]
pub struct Rasterize {
    pub frame: Frame,
    value: Value,
    /// Classes taken (by class), all when none.
    classes: Option<[bool; 256]>,
    a: Vec<f64>,
    b: Vec<f64>,
    /// IDW: the points right on a centre, summed and counted.
    on: Vec<f64>,
    n: Vec<u32>,
}

impl Rasterize {
    pub fn new(frame: Frame, value: Value, classes: Option<&[u8]>) -> Rasterize {
        let cells = frame.cols * frame.rows;
        let classes = classes.map(|list| {
            let mut t = [false; 256];
            for &c in list {
                t[usize::from(c)] = true;
            }
            t
        });
        let (a0, b0) = match value {
            Value::Min => (f64::INFINITY, 0.0),
            Value::Max => (f64::NEG_INFINITY, 0.0),
            _ => (0.0, 0.0),
        };
        let idw = matches!(value, Value::Idw(_));
        Rasterize {
            frame,
            value,
            classes,
            a: vec![a0; cells],
            b: if idw { vec![b0; cells] } else { Vec::new() },
            on: if idw { vec![0.0; cells] } else { Vec::new() },
            n: vec![0; cells],
        }
    }

    pub fn feed(&mut self, layout: &Layout, records: &[u8], scale: [f64; 3], offset: [f64; 3]) {
        let f = self.frame;
        for r in records.chunks_exact(layout.len) {
            if let Some(t) = &self.classes
                && !t[usize::from(layout.class(r))]
            {
                continue;
            }
            let [x, y, z] = world(layout, r, scale, offset);
            match self.value {
                Value::Idw(radius) => {
                    let lo_c = ((x - radius - f.x0) / f.cell - 0.5).ceil().max(0.0);
                    let hi_c = ((x + radius - f.x0) / f.cell - 0.5).floor();
                    let lo_r = ((f.top - y - radius) / f.cell - 0.5).ceil().max(0.0);
                    let hi_r = ((f.top - y + radius) / f.cell - 0.5).floor();
                    if !(hi_c >= lo_c && hi_r >= lo_r) {
                        continue;
                    }
                    let (c0, c1) = (lo_c as usize, (hi_c as usize).min(f.cols - 1));
                    let (r0, r1) = (lo_r as usize, (hi_r as usize).min(f.rows - 1));
                    for rr in r0..=r1 {
                        for cc in c0..=c1 {
                            let (cx, cy) = f.centre(cc, rr);
                            let d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                            if d2 > radius * radius {
                                continue;
                            }
                            let k = rr * f.cols + cc;
                            if d2 == 0.0 {
                                self.on[k] += z;
                                self.n[k] += 1;
                            } else {
                                let w = 1.0 / d2;
                                self.a[k] += w * z;
                                self.b[k] += w;
                            }
                        }
                    }
                }
                _ => {
                    let (c, rr) = f.cell_of(x, y);
                    let k = rr * f.cols + c;
                    match self.value {
                        Value::Min => self.a[k] = self.a[k].min(z),
                        Value::Max => self.a[k] = self.a[k].max(z),
                        Value::Mean => self.a[k] += z,
                        Value::Count | Value::Idw(_) => {}
                    }
                    self.n[k] += 1;
                }
            }
        }
    }

    /// The cells' values, row by row from the top; [`NODATA`] where empty.
    pub fn values(&self) -> Vec<f64> {
        (0..self.n.len())
            .map(|k| {
                let n = self.n[k];
                match self.value {
                    Value::Idw(_) => {
                        if n > 0 {
                            self.on[k] / f64::from(n)
                        } else if self.b[k] > 0.0 {
                            self.a[k] / self.b[k]
                        } else {
                            NODATA
                        }
                    }
                    _ if n == 0 => NODATA,
                    Value::Min | Value::Max => self.a[k],
                    Value::Mean => self.a[k] / f64::from(n),
                    Value::Count => f64::from(n),
                }
            })
            .collect()
    }
}

/// Sınır çıkar's cells: how many points each holds.
#[derive(Clone, Debug)]
pub struct Occupancy {
    pub frame: Frame,
    pub n: Vec<u32>,
}

impl Occupancy {
    pub fn new(frame: Frame) -> Occupancy {
        Occupancy {
            frame,
            n: vec![0; frame.cols * frame.rows],
        }
    }

    pub fn feed(&mut self, layout: &Layout, records: &[u8], scale: [f64; 3], offset: [f64; 3]) {
        for r in records.chunks_exact(layout.len) {
            let x = f64::from(layout.x(r)) * scale[0] + offset[0];
            let y = f64::from(layout.y(r)) * scale[1] + offset[1];
            let (c, rr) = self.frame.cell_of(x, y);
            let k = rr * self.frame.cols + c;
            self.n[k] = self.n[k].saturating_add(1);
        }
    }

    /// The cells with at least `least` points.
    pub fn filled(&self, least: u32) -> Vec<bool> {
        self.n.iter().map(|&n| n >= least.max(1)).collect()
    }
}

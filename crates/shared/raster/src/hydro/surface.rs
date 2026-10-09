//! The DEM as the hydrology tools see it (docs/adr/0235 §2): its values in
//! memory (NaN: none), the neighbours in their order, and each row's lengths
//! and areas in metres from the row's pixel axes (ADR 0231 §2: the affine on
//! a projected system, a geographic row's GRS80 metres).

use libm::sqrt;

use crate::frame::degree_metres;

/// The neighbours in their order (§2): east, south-east, south, south-west,
/// west, north-west, north, north-east (cell space, v down).
pub const N: [(i64, i64); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

/// A direction's code when the cell has none (a pit, an undrainable flat).
pub const NOFLOW: u8 = 8;
/// A cell without a value.
pub const NONE: u8 = 9;

/// The direction from a neighbour back to the cell it lies at `q` of.
#[inline]
pub fn back(q: usize) -> usize {
    (q + 4) % 8
}

/// A row's metres: its axes `[a, b, c, d]`, each neighbour's step length,
/// a cell's area and width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowGeom {
    pub axes: [f64; 4],
    pub step: [f64; 8],
    pub area: f64,
    pub width: f64,
}

impl RowGeom {
    fn of(axes: [f64; 4]) -> RowGeom {
        let mut step = [0.0; 8];
        for (s, &(di, dj)) in step.iter_mut().zip(&N) {
            *s = length(axes, di as f64, dj as f64);
        }
        let [a, b, c, d] = axes;
        let area = (a * d - b * c).abs();
        RowGeom {
            axes,
            step,
            area,
            width: sqrt(area),
        }
    }
}

/// The length in metres of a cell-space step (`du`, `dv`) along axes `[a, b, c, d]`.
#[inline]
pub fn length([a, b, c, d]: [f64; 4], du: f64, dv: f64) -> f64 {
    let x = a * du + b * dv;
    let y = c * du + d * dv;
    sqrt(x * x + y * y)
}

/// The DEM in memory.
#[derive(Debug)]
pub struct Surface {
    pub width: u32,
    pub height: u32,
    pub affine: [f64; 6],
    pub geographic: bool,
    /// Heights row by row, NaN without a value.
    pub z: Vec<f64>,
    rows: Vec<RowGeom>,
    /// The neighbours' offsets in `z` (an inner cell's).
    off: [isize; 8],
}

impl Surface {
    /// A surface of `width` × `height` cells, every height NaN until read.
    pub fn new(width: u32, height: u32, affine: [f64; 6], geographic: bool) -> Surface {
        let mut s = Surface {
            width,
            height,
            affine,
            geographic,
            z: vec![f64::NAN; width as usize * height as usize],
            rows: Vec::new(),
            off: {
                let w = width as isize;
                [1, w + 1, w, w - 1, -1, -w - 1, -w, -w + 1]
            },
        };
        s.rows = if geographic {
            (0..i64::from(height))
                .map(|j| RowGeom::of(s.axes_at(j)))
                .collect()
        } else {
            vec![RowGeom::of(s.axes_at(0))]
        };
        s
    }

    /// Row `j`'s pixel axes in metres (any row, the grid's or beyond it).
    pub fn axes_at(&self, j: i64) -> [f64; 4] {
        let [x0, a, b, y0, c, d] = self.affine;
        let _ = x0;
        if self.geographic {
            // ADR 0231 §2: the latitude of the row's middle column's cell centre.
            let lat = y0 + c * (f64::from(self.width) / 2.0) + d * (j as f64 + 0.5);
            let (ke, kn) = degree_metres(lat);
            [a * ke, b * ke, c * kn, d * kn]
        } else {
            [a, b, c, d]
        }
    }

    /// Row `j`'s metres.
    #[inline]
    pub fn row(&self, j: usize) -> &RowGeom {
        if self.geographic {
            &self.rows[j]
        } else {
            &self.rows[0]
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.z.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.z.is_empty()
    }

    /// Whether cell `k` has a value.
    #[inline]
    pub fn valid(&self, k: usize) -> bool {
        !self.z[k].is_nan()
    }

    /// Neighbour `q` of cell (`i`, `j`), when it lies in the raster and has a value.
    #[inline]
    pub fn neighbour(&self, i: usize, j: usize, q: usize) -> Option<usize> {
        let (di, dj) = N[q];
        let (a, b) = (i as i64 + di, j as i64 + dj);
        if a < 0 || b < 0 || a >= i64::from(self.width) || b >= i64::from(self.height) {
            return None;
        }
        let m = b as usize * self.width as usize + a as usize;
        (!self.z[m].is_nan()).then_some(m)
    }

    /// Whether a valid cell is an outlet: on the edge or beside a cell without a value (§2).
    #[inline]
    pub fn outlet(&self, i: usize, j: usize) -> bool {
        (0..8).any(|q| self.neighbour(i, j, q).is_none())
    }

    /// Whether cell (`i`, `j`) has all eight neighbours inside the raster.
    #[inline(always)]
    pub fn inner(&self, i: usize, j: usize) -> bool {
        i > 0 && j > 0 && i + 1 < self.width as usize && j + 1 < self.height as usize
    }

    /// `f(q, m)` for each valid neighbour `m` of cell `k` at (`i`, `j`), in the neighbours' order.
    #[inline(always)]
    pub fn each(&self, i: usize, j: usize, k: usize, mut f: impl FnMut(usize, usize)) {
        if self.inner(i, j) {
            for (q, &o) in self.off.iter().enumerate() {
                let m = (k as isize + o) as usize;
                if !self.z[m].is_nan() {
                    f(q, m);
                }
            }
        } else {
            for q in 0..8 {
                if let Some(m) = self.neighbour(i, j, q) {
                    f(q, m);
                }
            }
        }
    }

    /// Neighbour `q` of cell `k` at (`i`, `j`), valid or not, when inside the raster.
    #[inline(always)]
    pub fn at(&self, i: usize, j: usize, k: usize, q: usize) -> Option<usize> {
        if self.inner(i, j) {
            Some((k as isize + self.off[q]) as usize)
        } else {
            let (di, dj) = N[q];
            let (a, b) = (i as i64 + di, j as i64 + dj);
            if a < 0 || b < 0 || a >= i64::from(self.width) || b >= i64::from(self.height) {
                None
            } else {
                Some(b as usize * self.width as usize + a as usize)
            }
        }
    }
}

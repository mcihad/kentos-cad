//! A surface's point density grid (plan/road.md Y19.3, docs/adr/0920 §6; ROAD-SURVEY-08): the surface's corners
//! counted in square cells of the user's size, the USGS Lidar Base Specification's density raster (a cell of twice the
//! nominal spacing, each cell its count). Cells are whole multiples of the size from the world's origin, rows by y then
//! columns by x; a point on a cell's edge is the cell above and to the right's. A cell counts when its centre is inside
//! the boundary (outer ring less holes) or, without one, on the surface. Every limit is the user's.

use super::Surface;
use super::coverage::{Region, chain};

/// The density grid read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Density {
    pub cell: f64,
    /// The lower left corner of column 0, row 0.
    pub origin: [f64; 2],
    pub columns: usize,
    pub rows: usize,
    /// Each cell's count, row by row; none for a cell that does not count.
    pub counts: Vec<Option<u32>>,
    /// The cells that count.
    pub cells: usize,
    /// Of them, those with a point at least.
    pub filled: usize,
    /// Of them, those with fewer than `least` points; none without `least`.
    pub sparse: Option<usize>,
    /// The points in the cells that count.
    pub points: usize,
    /// The most points in one cell.
    pub most: u32,
}

/// Whether `p` is inside `ring` (even-odd).
pub fn ring_holds(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let n = ring.len();
    let mut hit = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) {
            hit = !hit;
        }
    }
    hit
}

impl Region {
    /// Whether `p` is inside the outer ring and in no hole.
    pub fn holds(&self, p: [f64; 2]) -> bool {
        ring_holds(&self.outer, p) && !self.holes.iter().any(|h| ring_holds(h, p))
    }
}

/// How many cells of `cell` cover [`lo`, `hi`] in one direction, and the first one's index.
fn span(lo: f64, hi: f64, cell: f64) -> (i64, usize) {
    let first = (lo / cell).floor() as i64;
    let last = (hi / cell).floor() as i64;
    (first, usize::try_from(last - first + 1).unwrap_or(0))
}

impl Surface {
    /// The cells a density grid of `cell` would have over the boundary's or the surface's extent.
    pub fn density_cells(&self, boundary: Option<&Region>, cell: f64) -> usize {
        let Some([lo, hi]) = self.grid_bounds(boundary) else { return 0 };
        let (_, columns) = span(lo[0], hi[0], cell);
        let (_, rows) = span(lo[1], hi[1], cell);
        columns.saturating_mul(rows)
    }

    fn grid_bounds(&self, boundary: Option<&Region>) -> Option<[[f64; 2]; 2]> {
        let points: Box<dyn Iterator<Item = [f64; 2]>> = match boundary {
            Some(r) => Box::new(r.outer.iter().copied()),
            None => Box::new(self.vertices.iter().map(|v| [v[0], v[1]])),
        };
        let b = points.fold([[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]], |b, p| {
            [[b[0][0].min(p[0]), b[0][1].min(p[1])], [b[1][0].max(p[0]), b[1][1].max(p[1])]]
        });
        (b[0][0] <= b[1][0] && b[0][1] <= b[1][1]).then_some(b)
    }

    /// The density grid of square cells of `cell` (positive; the caller bounds the cell count with
    /// [`Surface::density_cells`]); `least` counts the cells with fewer points.
    pub fn density(&self, boundary: Option<&Region>, cell: f64, least: Option<u32>) -> Density {
        let Some([lo, hi]) = self.grid_bounds(boundary) else { return Density::default() };
        let (fx, columns) = span(lo[0], hi[0], cell);
        let (fy, rows) = span(lo[1], hi[1], cell);
        let mut counts = vec![0u32; columns * rows];
        for v in &self.vertices {
            let (i, j) = ((v[0] / cell).floor() as i64 - fx, (v[1] / cell).floor() as i64 - fy);
            if (0..columns as i64).contains(&i) && (0..rows as i64).contains(&j) {
                counts[j as usize * columns + i as usize] += 1;
            }
        }
        let mut out = Density {
            cell,
            origin: [fx as f64 * cell, fy as f64 * cell],
            columns,
            rows,
            sparse: least.map(|_| 0),
            ..Density::default()
        };
        let mut start = 0u32;
        out.counts = counts
            .iter()
            .enumerate()
            .map(|(k, &n)| {
                let (i, j) = (k % columns, k / columns);
                let c = [(fx + i as i64) as f64 * cell + cell / 2.0, (fy + j as i64) as f64 * cell + cell / 2.0];
                let counts = match boundary {
                    Some(r) => r.holds(c),
                    None => self.height_walking(c[0], c[1], &mut start).z.is_some(),
                };
                counts.then(|| {
                    out.cells += 1;
                    out.points += n as usize;
                    out.most = out.most.max(n);
                    if n > 0 {
                        out.filled += 1;
                    }
                    if let (Some(least), Some(sparse)) = (least, out.sparse.as_mut())
                        && n < least
                    {
                        *sparse += 1;
                    }
                    n
                })
            })
            .collect();
        out
    }
}

impl Density {
    /// The outlines of the counting cells whose count `chosen` takes (docs/adr/0920 §7): the cells' edges not shared
    /// with another chosen cell, each with its cell on the left, chained like a surface's outlines (the rightmost turn
    /// where two touch at a corner, straight-on corners dropped, each from its least corner, in that order).
    pub fn outlines(&self, chosen: impl Fn(u32) -> bool) -> Vec<Vec<[f64; 2]>> {
        use std::collections::{BTreeMap, BTreeSet};
        let take = |i: usize, j: usize| self.counts[j * self.columns + i].is_some_and(&chosen);
        let corner = |i: usize, j: usize| u32::try_from(j * (self.columns + 1) + i).unwrap_or(u32::MAX);
        let mut edges: BTreeSet<(u32, u32)> = BTreeSet::new();
        for j in 0..self.rows {
            for i in 0..self.columns {
                if take(i, j) {
                    let c = [corner(i, j), corner(i + 1, j), corner(i + 1, j + 1), corner(i, j + 1)];
                    for k in 0..4 {
                        edges.insert((c[k], c[(k + 1) % 4]));
                    }
                }
            }
        }
        let mut next: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &(a, b) in &edges {
            if !edges.contains(&(b, a)) {
                next.entry(a).or_default().push(b);
            }
        }
        let width = self.columns + 1;
        chain(
            |v| {
                let (i, j) = (v as usize % width, v as usize / width);
                [self.origin[0] + i as f64 * self.cell, self.origin[1] + j as f64 * self.cell]
            },
            next,
        )
    }
}

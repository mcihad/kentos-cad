//! Alan sorgusu (docs/adr/0207 §7): for each of the selected areas, the
//! points in it (a point on a straight edge is in): how many, how dense (a
//! square metre's), their heights' least, most, mean and sample standard
//! deviation, and each class's count. The heights are summed as the files'
//! whole numbers (exact, file by file), combined at the end in a fixed
//! order, so the reference gives the same bits.

use super::region::Region;
use crate::record::Layout;
use kentos_geometry_core::vec2::Vec2;

/// A file's heights in an area, as whole numbers of its scale.
#[derive(Clone, Copy, Debug, Default)]
struct Sums {
    n: u64,
    s1: i128,
    s2: i128,
    scale: f64,
    offset: f64,
}

/// One area's figures.
#[derive(Clone, Debug, Default)]
pub struct Acc {
    sums: Vec<Sums>,
    min: f64,
    max: f64,
    classes: Vec<u64>,
}

/// What Alan sorgusu says of an area.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub count: u64,
    /// Points a square metre; none for an area of no size.
    pub density: Option<f64>,
    pub z_min: Option<f64>,
    pub z_max: Option<f64>,
    pub z_mean: Option<f64>,
    /// Sample standard deviation (n − 1); none under two points.
    pub z_std: Option<f64>,
    /// Each class with points, ascending, and its count.
    pub classes: Vec<(u8, u64)>,
    pub area: f64,
}

/// The areas' grid of a side, at most.
const GRID: usize = 128;

/// The query over the areas.
pub struct AreaQuery {
    regions: Vec<Region>,
    acc: Vec<Acc>,
    /// The areas whose box meets each cell of a grid over their boxes.
    cells: Vec<Vec<u32>>,
    bbox: [f64; 4],
    side: usize,
    cell: [f64; 2],
    file: usize,
}

impl AreaQuery {
    pub fn new(regions: Vec<Region>) -> AreaQuery {
        let mut bbox = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for r in &regions {
            let b = r.bbox();
            bbox = [
                bbox[0].min(b[0]),
                bbox[1].min(b[1]),
                bbox[2].max(b[2]),
                bbox[3].max(b[3]),
            ];
        }
        let side = GRID.min(regions.len().max(1)).max(1);
        let cell = [
            ((bbox[2] - bbox[0]) / side as f64).max(1e-9),
            ((bbox[3] - bbox[1]) / side as f64).max(1e-9),
        ];
        let mut cells = vec![Vec::new(); side * side];
        if bbox.iter().all(|v| v.is_finite()) {
            for (i, r) in regions.iter().enumerate() {
                let b = r.bbox();
                let c0 = Self::at(bbox[0], cell[0], side, b[0]);
                let c1 = Self::at(bbox[0], cell[0], side, b[2]);
                let r0 = Self::at(bbox[1], cell[1], side, b[1]);
                let r1 = Self::at(bbox[1], cell[1], side, b[3]);
                for rr in r0..=r1 {
                    for cc in c0..=c1 {
                        cells[rr * side + cc].push(i as u32);
                    }
                }
            }
        }
        let acc = regions
            .iter()
            .map(|_| Acc {
                sums: Vec::new(),
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
                classes: vec![0; 256],
            })
            .collect();
        AreaQuery {
            regions,
            acc,
            cells,
            bbox,
            side,
            cell,
            file: 0,
        }
    }

    #[inline]
    fn at(low: f64, cell: f64, n: usize, v: f64) -> usize {
        let t = ((v - low) / cell).floor();
        if t.is_nan() || t < 0.0 {
            0
        } else {
            (t as usize).min(n - 1)
        }
    }

    /// The next file's records follow: their scale and offset.
    pub fn file(&mut self, scale: [f64; 3], offset: [f64; 3]) {
        self.file = self.acc.first().map_or(0, |a| a.sums.len());
        for a in &mut self.acc {
            a.sums.push(Sums {
                scale: scale[2],
                offset: offset[2],
                ..Sums::default()
            });
        }
    }

    /// Records of the current file.
    pub fn feed(&mut self, layout: &Layout, records: &[u8], scale: [f64; 3], offset: [f64; 3]) {
        if self.acc.first().is_none_or(|a| a.sums.is_empty()) {
            self.file(scale, offset);
        }
        let indexes: Vec<_> = self.regions.iter().map(Region::index).collect();
        let [x1, y1, x2, y2] = self.bbox;
        for r in records.chunks_exact(layout.len) {
            let x = f64::from(layout.x(r)) * scale[0] + offset[0];
            let y = f64::from(layout.y(r)) * scale[1] + offset[1];
            if !(x >= x1 && x <= x2 && y >= y1 && y <= y2) {
                continue;
            }
            let c = Self::at(x1, self.cell[0], self.side, x);
            let rr = Self::at(y1, self.cell[1], self.side, y);
            let p = Vec2::new(x, y);
            for &i in &self.cells[rr * self.side + c] {
                let i = i as usize;
                if !self.regions[i].contains(&indexes[i], p) {
                    continue;
                }
                let zi = layout.z(r);
                let z = f64::from(zi) * scale[2] + offset[2];
                let a = &mut self.acc[i];
                let s = &mut a.sums[self.file];
                s.n += 1;
                s.s1 += i128::from(zi);
                s.s2 += i128::from(zi) * i128::from(zi);
                a.min = a.min.min(z);
                a.max = a.max.max(z);
                a.classes[usize::from(layout.class(r))] += 1;
            }
        }
    }

    /// Each area's figures, in the areas' order.
    pub fn rows(&self) -> Vec<Row> {
        self.acc
            .iter()
            .zip(&self.regions)
            .map(|(a, region)| {
                let count: u64 = a.sums.iter().map(|s| s.n).sum();
                let (mean, std) = moments(&a.sums);
                let area = region.area();
                Row {
                    count,
                    density: (area > 0.0).then(|| count as f64 / area),
                    z_min: (count > 0).then_some(a.min),
                    z_max: (count > 0).then_some(a.max),
                    z_mean: mean,
                    z_std: std,
                    classes: a
                        .classes
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| **n > 0)
                        .map(|(c, n)| (c as u8, *n))
                        .collect(),
                    area,
                }
            })
            .collect()
    }
}

/// The mean and the sample standard deviation over files' sums: each file's
/// mean and squared deviations from it in its whole numbers, combined file
/// by file (Chan, Golub and LeVeque's pairwise rule) in double precision.
fn moments(sums: &[Sums]) -> (Option<f64>, Option<f64>) {
    let mut n = 0.0f64;
    let mut mean = 0.0f64;
    let mut m2 = 0.0f64;
    for s in sums.iter().filter(|s| s.n > 0) {
        let k = s.n as f64;
        // The file's own mean and squared deviations, exact up to the last division.
        let mean_i = s.s1 as f64 / k;
        let dev = match (s.n as i128)
            .checked_mul(s.s2)
            .zip(s.s1.checked_mul(s.s1))
            .and_then(|(a, b)| a.checked_sub(b))
        {
            Some(d) => d as f64 / k,
            // Past 2¹²⁷: in double precision.
            None => s.s2 as f64 - s.s1 as f64 * mean_i,
        };
        let file_mean = mean_i * s.scale + s.offset;
        let file_m2 = dev * s.scale * s.scale;
        let total = n + k;
        let delta = file_mean - mean;
        mean += delta * k / total;
        m2 += file_m2 + delta * delta * n * k / total;
        n = total;
    }
    if n == 0.0 {
        return (None, None);
    }
    let std = (n >= 2.0).then(|| (m2 / (n - 1.0)).sqrt());
    (Some(mean), std)
}

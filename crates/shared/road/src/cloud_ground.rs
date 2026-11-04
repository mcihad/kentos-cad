//! A terrain's points from a point cloud (plan/road.md Y15, docs/adr/0928; ROAD-PC-04, 10, 11): the ground points a
//! caller has chosen by class and return, thinned on a plan grid so that a surface is made of fewer points without
//! losing what it needs, with what the thinning cost said: the density and coverage of the ground points read and
//! the vertical error of the points left out against the surface made of the kept ones.
//!
//! The thinning, decided whatever order the points come in (a tie goes to the point read first):
//! - **Cell:** in each cell of a square grid of `cell` metres from (0, 0), the point nearest the cell's centre in plan
//!   (Civil 3D's distance between points, a grid's representative).
//! - **Steep cell:** with a `z_tolerance`, a cell whose points' heights spread more than it keeps its lowest and its
//!   highest point too, so a sharp slope or a curb keeps its foot and its top (ROAD-PC-11).
//! - **Near a breakline:** with a reach and breaklines in plan, every point within the reach of one is kept
//!   (ROAD-PC-11: the detail beside a breakline is not a display level of detail).
//!
//! The coverage is the share of the grid's cells, in the area or else the points' cell box, that hold a ground
//! point; the density is the points read per square metre of that area. The error ([`residuals`]) is each point left
//! out against the surface made of the kept ones: the largest and the root mean square of the height differences
//! where the surface has a height, and how many points are outside it.

use std::collections::{HashMap, HashSet};

use crate::surface::Surface;

/// How a cloud's ground points are thinned.
#[derive(Clone, Debug, PartialEq)]
pub struct Thinning {
    /// The grid's cell, m.
    pub cell: f64,
    /// A cell whose heights spread more than this keeps its lowest and highest point too, m; none: never.
    pub z_tolerance: Option<f64>,
    /// Every point within this of a breakline in plan is kept, m; none: none.
    pub near: Option<f64>,
}

/// Why a thinning does not run.
#[derive(Clone, Debug, PartialEq)]
pub enum ThinningError {
    Cell,
    ZTolerance,
    Near,
}

impl std::fmt::Display for ThinningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ThinningError::Cell => "seyreltme hücresi sıfırdan büyük, sonlu bir uzunluk olmalı",
            ThinningError::ZTolerance => "kot toleransı sıfırdan büyük, sonlu olmalı",
            ThinningError::Near => "kırık hat erimi sıfırdan büyük, sonlu olmalı",
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct Pick {
    order: u64,
    p: [f64; 3],
}

#[derive(Clone, Copy, Debug)]
struct Cell {
    best: Pick,
    distance: f64,
    low: Pick,
    high: Pick,
}

/// The breaklines' segments in plan by the cells of a coarse grid, for the reach's test.
#[derive(Clone, Debug)]
struct Lines {
    reach: f64,
    size: f64,
    segments: Vec<[[f64; 2]; 2]>,
    cells: HashMap<(i64, i64), Vec<u32>>,
}

impl Lines {
    fn new(reach: f64, lines: &[Vec<[f64; 2]>]) -> Lines {
        let size = (reach * 4.0).max(1.0);
        let mut segments = Vec::new();
        let mut cells: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for line in lines {
            for w in line.windows(2) {
                let k = segments.len() as u32;
                segments.push([w[0], w[1]]);
                let (x0, x1) = (w[0][0].min(w[1][0]) - reach, w[0][0].max(w[1][0]) + reach);
                let (y0, y1) = (w[0][1].min(w[1][1]) - reach, w[0][1].max(w[1][1]) + reach);
                for i in (x0 / size).floor() as i64..=(x1 / size).floor() as i64 {
                    for j in (y0 / size).floor() as i64..=(y1 / size).floor() as i64 {
                        cells.entry((i, j)).or_default().push(k);
                    }
                }
            }
        }
        Lines { reach, size, segments, cells }
    }

    fn near(&self, x: f64, y: f64) -> bool {
        let key = ((x / self.size).floor() as i64, (y / self.size).floor() as i64);
        let Some(list) = self.cells.get(&key) else { return false };
        let r2 = self.reach * self.reach;
        list.iter().any(|&k| {
            let [a, b] = self.segments[k as usize];
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 { (((x - a[0]) * dx + (y - a[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
            let (ex, ey) = (x - (a[0] + t * dx), y - (a[1] + t * dy));
            ex * ex + ey * ey <= r2
        })
    }
}

/// The thinning as points are read.
#[derive(Clone, Debug)]
pub struct Thinner {
    rule: Thinning,
    lines: Option<Lines>,
    cells: HashMap<(i64, i64), Cell>,
    near: Vec<Pick>,
    read: u64,
    bounds: Option<[f64; 4]>,
}

/// What a thinning kept.
#[derive(Clone, Debug, PartialEq)]
pub struct Thinned {
    /// The kept points in the order they were read, each with its number in that order.
    pub points: Vec<(u64, [f64; 3])>,
    /// The ground points read.
    pub read: u64,
    /// The cells that hold a point.
    pub cells: u64,
    /// The points kept as a cell's nearest to its centre, as a steep cell's lowest or highest, and near a breakline
    /// (a point kept for two reasons counts in the first).
    pub by_cell: u64,
    pub by_slope: u64,
    pub by_line: u64,
    /// The occupied cells' keys, for the coverage.
    pub occupied: HashSet<(i64, i64)>,
    /// The points' box in plan: min x, min y, max x, max y; none with no point.
    pub bounds: Option<[f64; 4]>,
}

impl Thinner {
    /// A thinning by `rule`; the breaklines (each a path in plan) count only with a reach.
    pub fn new(rule: Thinning, breaklines: &[Vec<[f64; 2]>]) -> Result<Thinner, ThinningError> {
        if !(rule.cell > 0.0 && rule.cell.is_finite()) {
            return Err(ThinningError::Cell);
        }
        if rule.z_tolerance.is_some_and(|z| !(z > 0.0 && z.is_finite())) {
            return Err(ThinningError::ZTolerance);
        }
        if rule.near.is_some_and(|r| !(r > 0.0 && r.is_finite())) {
            return Err(ThinningError::Near);
        }
        let lines = rule.near.map(|r| Lines::new(r, breaklines));
        Ok(Thinner { rule, lines, cells: HashMap::new(), near: Vec::new(), read: 0, bounds: None })
    }

    /// The next ground point read.
    pub fn feed(&mut self, p: [f64; 3]) {
        let order = self.read;
        self.read += 1;
        if !(p[0].is_finite() && p[1].is_finite() && p[2].is_finite()) {
            return;
        }
        let b = self.bounds.get_or_insert([p[0], p[1], p[0], p[1]]);
        b[0] = b[0].min(p[0]);
        b[1] = b[1].min(p[1]);
        b[2] = b[2].max(p[0]);
        b[3] = b[3].max(p[1]);
        let c = self.rule.cell;
        let key = ((p[0] / c).floor() as i64, (p[1] / c).floor() as i64);
        let (cx, cy) = ((key.0 as f64 + 0.5) * c, (key.1 as f64 + 0.5) * c);
        let d = (p[0] - cx).powi(2) + (p[1] - cy).powi(2);
        let pick = Pick { order, p };
        match self.cells.get_mut(&key) {
            None => {
                self.cells.insert(key, Cell { best: pick, distance: d, low: pick, high: pick });
            }
            Some(cell) => {
                if d < cell.distance {
                    cell.best = pick;
                    cell.distance = d;
                }
                if p[2] < cell.low.p[2] {
                    cell.low = pick;
                }
                if p[2] > cell.high.p[2] {
                    cell.high = pick;
                }
            }
        }
        if let Some(lines) = &self.lines
            && lines.near(p[0], p[1])
        {
            self.near.push(pick);
        }
    }

    /// What was kept, in the order read.
    pub fn finish(self) -> Thinned {
        let mut kept: HashMap<u64, ([f64; 3], u8)> = HashMap::new();
        for cell in self.cells.values() {
            kept.entry(cell.best.order).or_insert((cell.best.p, 0));
            if let Some(z) = self.rule.z_tolerance
                && cell.high.p[2] - cell.low.p[2] > z
            {
                kept.entry(cell.low.order).or_insert((cell.low.p, 1));
                kept.entry(cell.high.order).or_insert((cell.high.p, 1));
            }
        }
        for pick in &self.near {
            kept.entry(pick.order).or_insert((pick.p, 2));
        }
        let mut points: Vec<(u64, [f64; 3], u8)> = kept.into_iter().map(|(o, (p, why))| (o, p, why)).collect();
        points.sort_by_key(|x| x.0);
        let count = |why: u8| points.iter().filter(|x| x.2 == why).count() as u64;
        Thinned {
            by_cell: count(0),
            by_slope: count(1),
            by_line: count(2),
            points: points.iter().map(|x| (x.0, x.1)).collect(),
            read: self.read,
            cells: self.cells.len() as u64,
            occupied: self.cells.keys().copied().collect(),
            bounds: self.bounds,
        }
    }
}

/// The ground points' coverage of an area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coverage {
    /// The area's plan area, m²: the ring's, or the cell box's.
    pub area: f64,
    /// The cells in it (their centres inside the ring, or the box's) and those holding a point.
    pub cells: u64,
    pub occupied: u64,
    /// `occupied / cells`, 0–1.
    pub share: f64,
    /// The ground points read per m² of the area.
    pub density: f64,
}

fn inside(ring: &[[f64; 2]], x: f64, y: f64) -> bool {
    let mut odd = false;
    let n = ring.len();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a[1] > y) != (b[1] > y) && x < a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]) {
            odd = !odd;
        }
    }
    odd
}

fn ring_area(ring: &[[f64; 2]]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1]).sum::<f64>().abs() / 2.0
}

/// The coverage of `ring` (an area's outer ring), or with none of the points' cell box, by what `thinned` read on a
/// grid of `cell`; none with no point and no ring.
pub fn coverage(thinned: &Thinned, cell: f64, ring: Option<&[[f64; 2]]>) -> Option<Coverage> {
    let (cells, occupied, area) = match ring {
        Some(r) if r.len() >= 3 => {
            let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            for p in r {
                x0 = x0.min(p[0]);
                y0 = y0.min(p[1]);
                x1 = x1.max(p[0]);
                y1 = y1.max(p[1]);
            }
            let (mut cells, mut occupied) = (0u64, 0u64);
            for i in (x0 / cell).floor() as i64..=(x1 / cell).floor() as i64 {
                for j in (y0 / cell).floor() as i64..=(y1 / cell).floor() as i64 {
                    if inside(r, (i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell) {
                        cells += 1;
                        occupied += u64::from(thinned.occupied.contains(&(i, j)));
                    }
                }
            }
            (cells, occupied, ring_area(r))
        }
        _ => {
            let keys = &thinned.occupied;
            if keys.is_empty() {
                return None;
            }
            let (mut i0, mut j0, mut i1, mut j1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
            for k in keys {
                i0 = i0.min(k.0);
                j0 = j0.min(k.1);
                i1 = i1.max(k.0);
                j1 = j1.max(k.1);
            }
            let cells = ((i1 - i0 + 1) * (j1 - j0 + 1)) as u64;
            (cells, keys.len() as u64, cells as f64 * cell * cell)
        }
    };
    Some(Coverage {
        area,
        cells,
        occupied,
        share: if cells == 0 { 0.0 } else { occupied as f64 / cells as f64 },
        density: if area > 0.0 { thinned.read as f64 / area } else { 0.0 },
    })
}

/// The points left out against a surface.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Residuals {
    /// The points that have a height on the surface, and those that do not (outside it or in a void).
    pub count: u64,
    pub outside: u64,
    /// The largest |z − surface| and the root mean square, m; 0 with no point measured.
    pub max: f64,
    pub rms: f64,
}

/// The vertical error of `points` (the ones left out) against `surface`.
pub fn residuals(surface: &Surface, points: impl IntoIterator<Item = [f64; 3]>) -> Residuals {
    let mut r = Residuals::default();
    let mut sum = 0.0;
    let mut start = 0;
    for p in points {
        match surface.height_walking(p[0], p[1], &mut start).z {
            Some(z) => {
                let d = (p[2] - z).abs();
                r.count += 1;
                r.max = r.max.max(d);
                sum += d * d;
            }
            None => r.outside += 1,
        }
    }
    if r.count > 0 {
        r.rms = (sum / r.count as f64).sqrt();
    }
    r
}

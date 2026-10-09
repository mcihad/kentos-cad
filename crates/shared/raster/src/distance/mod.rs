//! Uzaklık ve maliyet (docs/adr/0236): Uzaklık yüzeyi (the exact distance
//! transform, `edt`), Birikimli maliyet, En düşük maliyetli yol and Maliyet
//! koridoru (the cost network and its searches, `network`). The raster is
//! read whole into memory; the sources and destinations are objects burnt
//! onto its cells by Rasterleştir's rule (docs/adr/0234 §3), each with its
//! number; a raster result is written strip by strip, the paths as objects.

pub mod edt;
pub mod network;

use kentos_contracts::RasterSample;
use kentos_formats::raster::samples::Samples;
use kentos_geometry_core::entity::Shape;

use crate::frame::Frame;
use crate::grid::Grid;
use crate::inputs::{Sampling, View, mapping, place_in, sample_row};
use crate::par;
use crate::rasterize::{Burn, BurnSample, Objects, Overlap};
use crate::vector::simplify::douglas_peucker;
use crate::vector::{FeatureKind, Features};
use network::{
    MOVES, Neighbours, Network, STEP_CELLS, Search, path_to, predecessor, predecessors, sources_of,
};

/// The most cells in memory (§2; hydrology's, docs/adr/0235 §2).
pub const MOST_CELLS: u64 = crate::hydro::MOST_CELLS;

/// Maliyet koridoru's threshold (§6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Threshold {
    None,
    /// Per cent above the least sum.
    Percent(f64),
    /// A sum.
    Value(f64),
}

/// A tool and its settings (§3–§6), checked by [`DistanceWork::new`].
#[derive(Clone, Debug, PartialEq)]
pub enum DistanceTool {
    /// Uzaklık yüzeyi from a raster's cells with values.
    Euclid { max: f64, allocation: bool },
    Cost {
        neighbours: Neighbours,
        surface_length: bool,
        slope: f64,
        max: f64,
        allocation: bool,
    },
    Path {
        neighbours: Neighbours,
        surface_length: bool,
        slope: f64,
        simplify: f64,
        /// The objects before this are the sources, the rest the destinations.
        first: usize,
    },
    Corridor {
        neighbours: Neighbours,
        surface_length: bool,
        slope: f64,
        /// The objects before this are the first ends, the rest the second.
        first: usize,
        threshold: Threshold,
    },
}

impl DistanceTool {
    /// Whether the result is a raster (else the paths).
    pub fn raster(&self) -> bool {
        !matches!(self, DistanceTool::Path { .. })
    }
}

/// What a run met, for the host's summary and warnings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DistanceNotes {
    /// Source cells (both ends' for a corridor).
    pub sources: u64,
    /// Objects without a cell on the grid (sources, ends, destinations).
    pub outside: u64,
    /// Destinations no path reaches or without a cell, by number.
    pub unreached: Vec<u32>,
    /// The result's least and largest value (a corridor's least: the least sum before the threshold, its cheapest path's cost).
    pub least: f64,
    pub most: f64,
    /// Cells of the result with a value.
    pub cells: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Read,
    Prepare,
    Search,
    Finish,
    Emit,
    Done,
}

/// The step lengths of a north-up or turned grid with square or oblong cells
/// (§3): the column's and the row's; refused on a skewed or geographic grid.
pub fn plane_steps(frame: &Frame) -> Result<(f64, f64), String> {
    if frame.geographic {
        return Err("Düz uzaklık coğrafi ızgarada metre değildir: rasteri ya da projeyi projeksiyonlu bir sisteme alın.".into());
    }
    let [_, a, b, _, c, d] = frame.affine;
    let dot = a * b + c * d;
    if dot.abs() > 1e-12 * (a * a + b * b + c * c + d * d) {
        return Err("Rasterin pikselleri eğik (eksenleri dik değil): önce Yeniden örnekle ile kuzeyi yukarıda bir ızgaraya alın.".into());
    }
    Ok((libm::sqrt(a * a + c * c), libm::sqrt(b * b + d * d)))
}

/// Uzaklık yüzeyi over cells whose sources' numbers or values are `own` (NaN:
/// none): each cell's distance (or its nearest source's number) within `max`.
pub fn euclid(
    frame: &Frame,
    own: &[f64],
    max: f64,
    allocation: bool,
    threads: usize,
) -> Result<(Vec<f64>, u64), String> {
    let steps = plane_steps(frame)?;
    let source: Vec<bool> = own.iter().map(|v| !v.is_nan()).collect();
    let n = source.iter().filter(|s| **s).count() as u64;
    if n == 0 {
        return Err("Kaynak hücre yok: kaynaklar rasterin hiçbir hücresine düşmüyor.".into());
    }
    let near = edt::nearest(frame.width, frame.height, &source, steps, threads);
    let w = frame.width as usize;
    let mut out = vec![f64::NAN; own.len()];
    par::rows(threads, &mut out, w, &|first, chunk: &mut [f64]| {
        for (r, row) in chunk.chunks_mut(w).enumerate() {
            let j = first + r;
            for (i, o) in row.iter_mut().enumerate() {
                let to = near[j * w + i];
                if to == edt::NONE {
                    continue;
                }
                let d = edt::distance(i, j, to, w, steps);
                if max > 0.0 && d > max {
                    continue;
                }
                *o = if allocation { own[to as usize] } else { d };
            }
        }
    });
    Ok((out, n))
}

/// Objects burnt onto `grid`, each cell the first object's number (NaN: none),
/// and how many burnt no cell.
pub fn burnt(shapes: Vec<Shape>, grid: &Grid, threads: usize) -> Result<(Vec<f64>, u64), String> {
    burnt_objects(Objects::numbered(shapes), grid, threads)
}

/// [`burnt`] of objects already read.
pub fn burnt_objects(
    objects: Objects,
    grid: &Grid,
    threads: usize,
) -> Result<(Vec<f64>, u64), String> {
    let (w, h) = (grid.width as usize, grid.height);
    let mut out = vec![f64::NAN; w * h as usize];
    if objects.is_empty() {
        return Ok((out, 0));
    }
    let mut burn = Burn::new(objects, grid, Overlap::First, BurnSample::F64)?;
    let mut y0 = 0;
    while y0 < h {
        let n = kentos_formats::raster::TILE.min(h - y0);
        let (samples, _) = burn.strip(y0, n, threads)?;
        let rows = &mut out[y0 as usize * w..(y0 + n) as usize * w];
        for (k, v) in rows.iter_mut().enumerate() {
            *v = samples.get(k);
        }
        y0 += n;
    }
    Ok((out, burn.outside() as u64))
}

/// Uzaklık yüzeyi from objects (§3): the objects burnt onto the grid and the
/// transform done once, then the rows.
pub struct Near {
    objects: Option<Objects>,
    grid: Grid,
    frame: Frame,
    max: f64,
    allocation: bool,
    values: Vec<f64>,
    /// Source cells, and objects without a cell.
    pub sources: u64,
    pub outside: u64,
}

impl Near {
    /// The run over `grid` (its axes checked here).
    pub fn new(
        objects: Objects,
        grid: Grid,
        geographic: bool,
        max: f64,
        allocation: bool,
    ) -> Result<Near, String> {
        let (w, h) = (grid.width, grid.height);
        if u64::from(w) * u64::from(h) > MOST_CELLS {
            return Err(format!(
                "Izgara uzaklık için çok büyük ({w} × {h}): en çok 2²⁵ hücre (5792 × 5792). Hücre boyunu büyütün."
            ));
        }
        let frame = Frame::new(grid.affine, w, h, geographic)?;
        plane_steps(&frame)?;
        Ok(Near {
            objects: Some(objects),
            grid,
            frame,
            max,
            allocation,
            values: Vec::new(),
            sources: 0,
            outside: 0,
        })
    }

    /// Whether the result is the nearest sources' numbers.
    pub fn allocation(&self) -> bool {
        self.allocation
    }

    /// The objects burnt and the transform done (the first time).
    pub fn compute(&mut self, threads: usize) -> Result<(), String> {
        let Some(objects) = self.objects.take() else {
            return Ok(());
        };
        let (own, outside) = burnt_objects(objects, &self.grid, threads)?;
        self.outside = outside;
        let (values, n) = euclid(&self.frame, &own, self.max, self.allocation, threads)?;
        self.sources = n;
        self.values = values;
        Ok(())
    }

    /// Rows `y0 .. y0 + n` of the result.
    pub fn rows(&self, y0: u32, n: u32) -> &[f64] {
        let w = self.grid.width as usize;
        &self.values[y0 as usize * w..(y0 + n) as usize * w]
    }
}

/// One object's cells on `grid`: its box's rows burnt on the whole grid (so
/// that a cell is the same as among all objects).
fn cells_of(shape: Shape, grid: &Grid, threads: usize) -> Result<Vec<u32>, String> {
    let objects = Objects::numbered(vec![shape]);
    let Some([x0, y0, x1, y1]) = objects.bounds() else {
        return Ok(Vec::new());
    };
    let (mut jmin, mut jmax) = (f64::INFINITY, f64::NEG_INFINITY);
    for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
        let (_, v) = place_in(&grid.affine, x, y);
        jmin = jmin.min(v);
        jmax = jmax.max(v);
    }
    if !(jmin.is_finite() && jmax.is_finite()) {
        return Ok(Vec::new());
    }
    let h = i64::from(grid.height);
    let a = (jmin.floor() as i64 - 1).clamp(0, h);
    let b = (jmax.floor() as i64 + 2).clamp(0, h);
    if a >= b {
        return Ok(Vec::new());
    }
    let mut burn = Burn::new(objects, grid, Overlap::First, BurnSample::F64)?;
    let (samples, _) = burn.strip(a as u32, (b - a) as u32, threads)?;
    let w = grid.width as usize;
    let base = a as usize * w;
    Ok((0..samples.len())
        .filter(|&k| !samples.get(k).is_nan())
        .map(|k| (base + k) as u32)
        .collect())
}

/// The burnt cells that can be entered; refused when there are none.
fn cells_in(own: &[f64], net: &Network) -> Result<Vec<u32>, String> {
    let cells: Vec<u32> = (0..own.len())
        .filter(|&k| !own[k].is_nan() && !net.cost[k].is_nan())
        .map(|k| k as u32)
        .collect();
    if cells.is_empty() {
        return Err(
            "Kaynak hücre yok: kaynak nesneleri maliyet rasterinin değerli hücrelerine düşmüyor."
                .into(),
        );
    }
    Ok(cells)
}

/// The run's state.
pub struct DistanceWork {
    tool: DistanceTool,
    band: usize,
    grid: Grid,
    frame: Frame,
    /// The surface's place, when one is read (the second input).
    surface: Option<[f64; 6]>,
    stage: Stage,
    net: Option<Network>,
    /// Uzaklık yüzeyi: the band's values; then the result's.
    values: Vec<f64>,
    /// The least cost read that is not above 0 or not a number.
    bad: Option<f64>,
    shapes: Vec<Shape>,
    searches: Vec<Search>,
    /// The sources' numbers by cell (NaN: none).
    own: Vec<f64>,
    /// The destinations' cells.
    targets: Vec<Vec<u32>>,
    features: Option<Features>,
    emitted: u32,
    threads: usize,
    pub notes: DistanceNotes,
}

fn within(v: f64, least: f64, most: f64, what: &str) -> Result<f64, String> {
    if v.is_finite() && v >= least && v <= most {
        Ok(v)
    } else {
        Err(format!("{what} {least} ile {most} arasında olmalı."))
    }
}

impl DistanceWork {
    /// A run of `tool` over band `band` of a raster on `grid` (and a surface on
    /// `surface` when given), the objects `shapes`, on `threads`.
    pub fn new(
        tool: DistanceTool,
        band: usize,
        grid: Grid,
        geographic: bool,
        surface: Option<[f64; 6]>,
        shapes: Vec<Shape>,
        threads: usize,
    ) -> Result<DistanceWork, String> {
        let (w, h) = (grid.width, grid.height);
        if u64::from(w) * u64::from(h) > MOST_CELLS {
            return Err(format!(
                "Raster uzaklık ve maliyet için çok büyük ({w} × {h}): en çok 2²⁵ hücre (5792 × 5792). Yeniden örnekle ya da Maskeyle kırp ile küçültün."
            ));
        }
        let frame = Frame::new(grid.affine, w, h, geographic)?;
        let n = w as usize * h as usize;
        let ends = |first: usize, what: (&str, &str)| -> Result<(), String> {
            if first == 0 {
                return Err(format!("{} seçin.", what.0));
            }
            if first >= shapes.len() {
                return Err(format!("{} seçin.", what.1));
            }
            Ok(())
        };
        let net = |neighbours, surface_length, slope: f64| -> Result<Network, String> {
            within(slope, 0.0, 1000.0, "En büyük boyuna eğim (yüzde)")?;
            Ok(Network::new(
                &frame,
                neighbours,
                surface.is_some(),
                surface_length,
                slope,
            ))
        };
        let net = match &tool {
            DistanceTool::Euclid { max, .. } => {
                within(*max, 0.0, f64::MAX, "En büyük uzaklık")?;
                plane_steps(&frame)?;
                None
            }
            DistanceTool::Cost {
                neighbours,
                surface_length,
                slope,
                max,
                ..
            } => {
                within(*max, 0.0, f64::MAX, "En büyük maliyet")?;
                if shapes.is_empty() {
                    return Err("Kaynakları seçin: nokta, çizgi ya da alan.".into());
                }
                Some(net(*neighbours, *surface_length, *slope)?)
            }
            DistanceTool::Path {
                neighbours,
                surface_length,
                slope,
                simplify,
                first,
            } => {
                within(*simplify, 0.0, 1e6, "Sadeleştirme")?;
                ends(*first, ("Başlangıç nesnelerini", "Varış nesnelerini"))?;
                Some(net(*neighbours, *surface_length, *slope)?)
            }
            DistanceTool::Corridor {
                neighbours,
                surface_length,
                slope,
                first,
                threshold,
            } => {
                match threshold {
                    Threshold::Percent(p) => {
                        within(*p, 0.0, 1e6, "Eşiğin yüzdesi")?;
                    }
                    Threshold::Value(v) => {
                        within(*v, 0.0, f64::MAX, "Eşiğin değeri")?;
                    }
                    Threshold::None => {}
                }
                ends(*first, ("Birinci uçları", "İkinci uçları"))?;
                Some(net(*neighbours, *surface_length, *slope)?)
            }
        };
        Ok(DistanceWork {
            values: if net.is_none() {
                vec![f64::NAN; n]
            } else {
                Vec::new()
            },
            tool,
            band,
            grid,
            frame,
            surface,
            stage: Stage::Read,
            net,
            bad: None,
            shapes,
            searches: Vec::new(),
            own: Vec::new(),
            targets: Vec::new(),
            features: None,
            emitted: 0,
            threads: threads.max(1),
            notes: DistanceNotes::default(),
        })
    }

    pub fn tool(&self) -> &DistanceTool {
        &self.tool
    }

    /// Whether the raster is still being read.
    pub fn reading(&self) -> bool {
        self.stage == Stage::Read
    }

    /// Whether the run writes a raster's strips now.
    pub fn emitting(&self) -> bool {
        self.stage == Stage::Emit
    }

    pub fn done(&self) -> bool {
        self.stage == Stage::Done
    }

    /// Whether the surface is read too (the second input).
    pub fn reads_surface(&self) -> bool {
        self.surface.is_some()
    }

    /// A block of rows (`c0..c1`, `y0..y1`): the band from the raster's view, the heights from the surface's.
    pub fn block(
        &mut self,
        (c0, c1, y0, y1): (u32, u32, u32, u32),
        raster: Option<&View>,
        surface: Option<&View>,
    ) {
        let w = self.grid.width as usize;
        let (rs, re) = (y0 as usize * w, y1 as usize * w);
        let band = self.band;
        let empty = View::default();
        let raster = raster.unwrap_or(&empty);
        let cols = c0 as usize..c1 as usize;
        let read_row = |j: usize, row: &mut [f64]| {
            raster.row_into(band, c0 as i64, j as i64, &mut row[cols.clone()]);
        };
        match &mut self.net {
            None => {
                par::rows(
                    self.threads,
                    &mut self.values[rs..re],
                    w,
                    &|first, chunk: &mut [f64]| {
                        for (k, row) in chunk.chunks_mut(w).enumerate() {
                            read_row(y0 as usize + first + k, row);
                        }
                    },
                );
            }
            Some(net) => {
                par::rows(
                    self.threads,
                    &mut net.cost[rs..re],
                    w,
                    &|first, chunk: &mut [f64]| {
                        for (k, row) in chunk.chunks_mut(w).enumerate() {
                            read_row(y0 as usize + first + k, row);
                        }
                    },
                );
                for &v in &net.cost[rs..re] {
                    // A value that is neither empty nor a positive number.
                    if !(v.is_nan() || v > 0.0 && v.is_finite()) {
                        self.bad = Some(self.bad.map_or(v, |b| b.min(v)));
                    }
                }
                if let (Some(z), Some(affine), Some(view)) = (&mut net.z, self.surface, surface) {
                    let grid = self.grid;
                    let map = mapping(&grid, &affine);
                    par::rows(
                        self.threads,
                        &mut z[rs..re],
                        w,
                        &|first, chunk: &mut [f64]| {
                            for (k, row) in chunk.chunks_mut(w).enumerate() {
                                let j = y0 + (first + k) as u32;
                                sample_row(
                                    (&grid, &affine),
                                    map,
                                    view,
                                    0,
                                    (c0, j),
                                    Sampling::Bilinear,
                                    &mut row[cols.clone()],
                                );
                            }
                        },
                    );
                }
            }
        }
    }

    /// The raster read: its values checked, the run goes on.
    pub fn read_done(&mut self) -> Result<(), String> {
        if let Some(v) = self.bad {
            return Err(format!(
                "Maliyet rasterinde 0 ya da eksi değer var (en küçüğü {v}): maliyet 0'dan büyük olmalı. Geçilmeyecek yerleri değersiz yapın, çok ucuz yerlere küçük bir artı değer verin."
            ));
        }
        if let Some(net) = &mut self.net
            && let Some(z) = &net.z
        {
            // A cell without a height cannot be entered.
            for (c, &h) in net.cost.iter_mut().zip(z) {
                if h.is_nan() {
                    *c = f64::NAN;
                }
            }
        }
        self.stage = Stage::Prepare;
        Ok(())
    }

    /// The share done, 0..1, the reading's share `read`.
    pub fn share(&self, read: f64) -> f64 {
        match self.stage {
            Stage::Read => 0.2 * read,
            Stage::Prepare => 0.2,
            Stage::Search => {
                let n = self.net.as_ref().map_or(1, |n| n.len()).max(1) as f64;
                let k = self.searches.len().max(1) as f64;
                let got: f64 = self.searches.iter().map(|s| s.order.len() as f64).sum();
                0.25 + 0.6 * (got / (n * k)).min(1.0)
            }
            Stage::Finish => 0.85,
            Stage::Emit => 0.9 + 0.1 * f64::from(self.emitted) / f64::from(self.grid.height.max(1)),
            Stage::Done => 1.0,
        }
    }

    /// The next stage's piece of work.
    pub fn step(&mut self) -> Result<(), String> {
        match self.stage {
            Stage::Read | Stage::Emit | Stage::Done => {
                Err("Uzaklık işi bu aşamada adım almaz.".into())
            }
            Stage::Prepare => self.prepare(),
            Stage::Search => {
                let net = self.net.as_ref().ok_or("Maliyet ağı yok.")?;
                let mut searches = std::mem::take(&mut self.searches);
                par::each_mut(
                    self.threads.min(searches.len().max(1)),
                    &mut searches,
                    &|_, s: &mut Search| {
                        s.advance(net, STEP_CELLS);
                    },
                );
                let done = searches.iter().all(|s| s.done);
                let lost = searches.iter().any(|s| s.lost);
                self.searches = searches;
                if lost {
                    return Err("Bir adımın maliyeti birikimli maliyetin yanında sayı duyarlığının altında kaldı: maliyetlerin en küçüğü ile en büyüğü arasındaki oran çok büyük. Maliyetleri ölçekleyin.".into());
                }
                if let DistanceTool::Path { .. } = self.tool {
                    self.watch_targets();
                }
                if done {
                    self.stage = Stage::Finish;
                }
                Ok(())
            }
            Stage::Finish => self.finish_results(),
        }
    }

    /// The sources burnt, the destinations' cells found, the searches begun
    /// (Uzaklık yüzeyi: the transform done).
    fn prepare(&mut self) -> Result<(), String> {
        let threads = self.threads;
        let shapes = std::mem::take(&mut self.shapes);
        match self.tool.clone() {
            DistanceTool::Euclid { max, allocation } => {
                let own = std::mem::take(&mut self.values);
                let (out, n) = euclid(&self.frame, &own, max, allocation, threads)?;
                self.notes.sources = n;
                self.values = out;
                self.stage = Stage::Emit;
                self.measure();
                return Ok(());
            }
            DistanceTool::Cost { max, .. } => {
                let (own, outside) = burnt(shapes, &self.grid, threads)?;
                self.notes.outside = outside;
                let net = self.net.as_ref().ok_or("Maliyet ağı yok.")?;
                let cells = cells_in(&own, net)?;
                self.notes.sources = cells.len() as u64;
                self.searches = vec![Search::new(net, &cells, max)];
                self.own = own;
            }
            DistanceTool::Path { first, .. } => {
                let mut shapes = shapes;
                let destinations = shapes.split_off(first);
                let (own, outside) = burnt(shapes, &self.grid, threads)?;
                let net = self.net.as_ref().ok_or("Maliyet ağı yok.")?;
                let cells = cells_in(&own, net)?;
                self.notes.sources = cells.len() as u64;
                let mut targets = Vec::with_capacity(destinations.len());
                for (x, d) in destinations.into_iter().enumerate() {
                    let t = cells_of(d, &self.grid, threads)?;
                    let t: Vec<u32> = t
                        .into_iter()
                        .filter(|&k| !net.cost[k as usize].is_nan())
                        .collect();
                    if t.is_empty() {
                        self.notes.unreached.push(x as u32 + 1);
                    }
                    targets.push(t);
                }
                self.notes.outside = outside;
                let mut s = Search::new(net, &cells, 0.0);
                if targets.iter().all(|t| t.is_empty()) {
                    s.done = true;
                }
                self.searches = vec![s];
                self.own = own;
                self.targets = targets;
            }
            DistanceTool::Corridor { first, .. } => {
                let mut shapes = shapes;
                let second = shapes.split_off(first);
                let (a, out_a) = burnt(shapes, &self.grid, threads)?;
                let (b, out_b) = burnt(second, &self.grid, threads)?;
                self.notes.outside = out_a + out_b;
                let net = self.net.as_ref().ok_or("Maliyet ağı yok.")?;
                let ca = cells_in(&a, net)?;
                let cb = cells_in(&b, net)?;
                self.notes.sources = (ca.len() + cb.len()) as u64;
                self.searches = vec![Search::new(net, &ca, 0.0), Search::new(net, &cb, 0.0)];
            }
        }
        self.stage = Stage::Search;
        Ok(())
    }

    /// The search stops once every destination is settled and the sums tied with the last are.
    fn watch_targets(&mut self) {
        let Some(s) = self.searches.first_mut() else {
            return;
        };
        if s.stop.is_finite() || s.done {
            return;
        }
        let mut most = f64::NEG_INFINITY;
        for t in &self.targets {
            if t.is_empty() {
                continue;
            }
            let least = t
                .iter()
                .map(|&k| s.acc[k as usize])
                .fold(f64::INFINITY, f64::min);
            // Not yet settled: a cell's sum is final once it is not above the last settled.
            let last = s
                .order
                .last()
                .map_or(f64::NEG_INFINITY, |&k| s.acc[k as usize]);
            if !(least <= last) {
                return;
            }
            most = most.max(least);
        }
        s.stop = most;
    }

    /// The raster's values or the paths.
    fn finish_results(&mut self) -> Result<(), String> {
        let threads = self.threads;
        let net = self.net.as_ref().ok_or("Maliyet ağı yok.")?;
        let mut cheapest = None;
        match self.tool.clone() {
            DistanceTool::Euclid { .. } => {}
            DistanceTool::Cost {
                max, allocation, ..
            } => {
                let s = &self.searches[0];
                let limit = if max > 0.0 { max } else { f64::INFINITY };
                let mut values: Vec<f64> = s
                    .acc
                    .iter()
                    .map(|&a| {
                        if a.is_finite() && a <= limit {
                            a
                        } else {
                            f64::NAN
                        }
                    })
                    .collect();
                if allocation {
                    let pred = predecessors(net, &values, threads);
                    let mut own = std::mem::take(&mut self.own);
                    sources_of(net, &s.order, &pred, &mut own);
                    for (v, o) in values.iter_mut().zip(&own) {
                        if !v.is_nan() {
                            *v = *o;
                        }
                    }
                }
                self.values = values;
            }
            DistanceTool::Path {
                simplify,
                surface_length: _,
                ..
            } => {
                self.features = Some(self.paths(simplify));
                self.stage = Stage::Done;
                return Ok(());
            }
            DistanceTool::Corridor { threshold, .. } => {
                let (a, b) = (&self.searches[0].acc, &self.searches[1].acc);
                let mut sums: Vec<f64> = a
                    .iter()
                    .zip(b)
                    .map(|(&x, &y)| {
                        if x.is_finite() && y.is_finite() {
                            x + y
                        } else {
                            f64::NAN
                        }
                    })
                    .collect();
                let least = sums
                    .iter()
                    .copied()
                    .filter(|v| !v.is_nan())
                    .fold(f64::INFINITY, f64::min);
                if !least.is_finite() {
                    return Err("Uçlar birbirine erişemiyor: aralarında maliyet rasterinin değerli hücrelerinden bir yol yok.".into());
                }
                let limit = match threshold {
                    Threshold::None => f64::INFINITY,
                    Threshold::Percent(p) => least * (1.0 + p / 100.0),
                    Threshold::Value(v) => v,
                };
                for s in &mut sums {
                    if *s > limit {
                        *s = f64::NAN;
                    }
                }
                self.values = sums;
                cheapest = Some(least);
            }
        }
        self.measure();
        if let Some(least) = cheapest {
            self.notes.least = least;
        }
        self.stage = Stage::Emit;
        Ok(())
    }

    /// The result's least and largest values and its cells with a value.
    fn measure(&mut self) {
        let (mut lo, mut hi, mut n) = (f64::INFINITY, f64::NEG_INFINITY, 0u64);
        for &v in &self.values {
            if !v.is_nan() {
                lo = lo.min(v);
                hi = hi.max(v);
                n += 1;
            }
        }
        self.notes.least = if n > 0 { lo } else { f64::NAN };
        self.notes.most = if n > 0 { hi } else { f64::NAN };
        self.notes.cells = n;
    }

    /// Each reached destination's path from its cheapest source (§5).
    fn paths(&mut self, simplify: f64) -> Features {
        let net = self.net.as_ref().expect("the network");
        let s = &self.searches[0];
        let acc = &s.acc;
        let surface = net.z.is_some();
        let fields: &[&'static str] = if surface {
            &[
                "Yol",
                "Kaynak",
                "Maliyet",
                "Uzunluk",
                "Yüzey uzunluğu",
                "En büyük eğim",
            ]
        } else {
            &["Yol", "Kaynak", "Maliyet", "Uzunluk"]
        };
        let mut out = Features::with_fields(FeatureKind::Lines, fields);
        let w = net.width as usize;
        let stop = s.stop;
        for (x, t) in self.targets.iter().enumerate() {
            // The destination's cell: its least settled sum, then its least cell number.
            let end = t
                .iter()
                .map(|&k| k as usize)
                .filter(|&k| acc[k].is_finite() && acc[k] <= stop)
                .min_by(|&p, &q| acc[p].total_cmp(&acc[q]).then(p.cmp(&q)));
            let Some(end) = end else {
                if !t.is_empty() {
                    self.notes.unreached.push(x as u32 + 1);
                }
                continue;
            };
            let cells = path_to(net, end, &|c| predecessor(net, acc, c));
            let (mut plan, mut surf, mut grade) = (0.0, 0.0, 0.0f64);
            for pair in cells.windows(2) {
                let (from, to) = (pair[0], pair[1]);
                let step = (
                    (to % w) as i64 - (from % w) as i64,
                    (to / w) as i64 - (from / w) as i64,
                );
                let Some(q) = MOVES.iter().position(|&m| m == step) else {
                    continue;
                };
                let j = from / w;
                let len = net.length(j, q);
                plan += len;
                if let Some(z) = &net.z {
                    let dz = z[to] - z[from];
                    surf += network::surface_length(net, j, q, dz);
                    grade = grade.max(dz.abs() / len * 100.0);
                }
            }
            let pts: Vec<[f64; 2]> = cells
                .iter()
                .map(|&k| [(k % w) as f64 + 0.5, (k / w) as f64 + 0.5])
                .collect();
            let kept = douglas_peucker(&pts, simplify);
            out.push_line(&kept, &self.grid.affine, f64::NAN, String::new());
            let start = cells[0];
            out.numbers
                .extend([x as f64 + 1.0, self.own[start], acc[end], plan]);
            if surface {
                out.numbers.extend([surf, grade]);
            }
        }
        self.notes.unreached.sort_unstable();
        self.notes.unreached.dedup();
        out
    }

    /// The result's next strip: rows `n` in `sample`.
    pub fn strip(&mut self, n: u32, sample: RasterSample) -> Samples {
        let w = self.grid.width as usize;
        let (a, b) = (self.emitted as usize * w, (self.emitted + n) as usize * w);
        let vals = &self.values[a..b];
        self.emitted += n;
        if self.emitted >= self.grid.height {
            self.stage = Stage::Done;
        }
        match sample {
            RasterSample::F64 => Samples::F64(vals.to_vec()),
            _ => Samples::F32(vals.iter().map(|&v| v as f32).collect()),
        }
    }

    /// The paths.
    pub fn finish(&mut self) -> Result<Features, String> {
        self.features
            .take()
            .ok_or_else(|| "Uzaklık işinin nesneleri yok.".into())
    }
}

/// The cell a cell comes from, by the predecessors' rule (tests).
#[cfg(test)]
fn predecessor_cell(net: &Network, acc: &[f64], c: usize) -> Option<usize> {
    let q = predecessor(net, acc, c);
    (q != network::NO_MOVE).then(|| network::from_cell(net, c, q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::vec2::Vec2;

    fn grid(w: u32, h: u32) -> Grid {
        Grid::of([0.0, 10.0, 0.0, 0.0, 0.0, -10.0], w, h).expect("a grid")
    }

    fn point(x: f64, y: f64) -> Shape {
        Shape::Point {
            p: Vec2::new(x, y),
            z: None,
            parts: None,
        }
    }

    #[test]
    fn the_path_runs_from_its_source_to_its_destination() {
        let g = grid(8, 6);
        let tool = DistanceTool::Path {
            neighbours: Neighbours::Sixteen,
            surface_length: false,
            slope: 0.0,
            simplify: 0.0,
            first: 1,
        };
        let mut work = DistanceWork::new(
            tool,
            0,
            g,
            false,
            None,
            vec![point(5.0, -5.0), point(75.0, -55.0)],
            2,
        )
        .expect("a run");
        let cost = View {
            x: 0,
            y: 0,
            w: 8,
            h: 6,
            bands: vec![vec![1.0; 48]],
        };
        work.block((0, 8, 0, 6), Some(&cost), None);
        work.read_done().expect("read");
        while !work.done() {
            work.step().expect("a step");
        }
        let f = work.finish().expect("the paths");
        assert_eq!(f.fields, vec!["Yol", "Kaynak", "Maliyet", "Uzunluk"]);
        assert_eq!(f.sizes.len(), 1);
        // From the source's centre to the destination's.
        assert_eq!(&f.xy[..2], &[5.0, -5.0]);
        assert_eq!(&f.xy[f.xy.len() - 2..], &[75.0, -55.0]);
        let net = work.net.as_ref().expect("net");
        let end = 5 * 8 + 7;
        assert!(predecessor_cell(net, &work.searches[0].acc, end).is_some());
    }
}

//! Hidroloji (docs/adr/0235): a DEM's depressions filled, its D8 flow
//! directions with the flats resolved, its accumulation by D8, D∞ or
//! multiple directions, the wetness index, pour points, watersheds, basins
//! and the stream network. The operation job (`crate::ops`) reads the DEM
//! strip by strip into memory; the work then runs in steps (the fill and the
//! accumulation in budgets of cells) and, for a raster result, the job
//! writes its strips from here.
//!
//! - [`surface`]: the DEM in memory, the neighbours, each row's metres.
//! - [`heap`]: the priority flood's radix heap.
//! - [`fill`]: Çukur doldur (§3).
//! - [`flow`]: D8 and the flats (§4).
//! - [`accum`]: Akış birikimi (§5).
//! - [`basins`]: pour points, watersheds, basins, route crossings (§7–§9).
//! - [`streams`]: the stream network (§10).

pub mod accum;
pub mod basins;
pub mod fill;
pub mod flow;
pub mod heap;
pub mod streams;
pub mod surface;
pub mod tiled;

use kentos_contracts::RasterSample;
use kentos_formats::raster::samples::Samples;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::tools::point_calc::reading;
use kentos_geometry_core::vec2::Vec2;
use libm::{log, sqrt};

use crate::frame::Gradient;
use crate::inputs::{Raw, point_of};
use crate::par;
use crate::terrain::{Method as Gradients, derivatives};
use crate::vector::simplify::douglas_peucker;
use crate::vector::work::world_ring;
use crate::vector::{FeatureKind, Features};
use accum::{Accumulating, Method, Own, Routing};
use basins::{
    area_of, areas, cells_rings, crossed_cells, label_upstream, rings_of, snap, terminals, upstream,
};
use fill::Filling;
use flow::{d8, receiver, resolve_flats};
use streams::{network, path_length};
use surface::{NOFLOW, NONE, Surface};

/// Cells a hydrology run takes (2²⁵: 5792 × 5792).
pub const MOST_CELLS: u64 = 1 << 25;

/// ESRI's codes and TauDEM's of the neighbours' directions (§4).
const ESRI: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
const TAUDEM: [u8; 8] = [1, 8, 7, 6, 5, 4, 3, 2];

/// Akış yönü's coding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coding {
    Esri,
    Taudem,
}

/// Akış birikimi's unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Cells,
    Area,
    Sca,
}

/// Havzalar' kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasinMode {
    Main,
    Sub,
    Route,
}

/// What a run makes.
#[derive(Clone, Debug, PartialEq)]
pub enum HydroTool {
    Fill {
        slope: f64,
        depth: bool,
    },
    Direction {
        fill: bool,
        coding: Coding,
    },
    Accumulation {
        fill: bool,
        method: Method,
        unit: Unit,
    },
    Wetness {
        fill: bool,
        method: Method,
        slope: f64,
    },
    PourPoints {
        fill: bool,
        snap: f64,
    },
    Watershed {
        fill: bool,
        snap: f64,
    },
    Basins {
        fill: bool,
        mode: BasinMode,
        threshold: f64,
        least: f64,
    },
    Streams {
        fill: bool,
        threshold: f64,
        simplify: f64,
    },
}

impl HydroTool {
    fn fills(&self) -> bool {
        match *self {
            HydroTool::Fill { .. } => true,
            HydroTool::Direction { fill, .. }
            | HydroTool::Accumulation { fill, .. }
            | HydroTool::Wetness { fill, .. }
            | HydroTool::PourPoints { fill, .. }
            | HydroTool::Watershed { fill, .. }
            | HydroTool::Basins { fill, .. }
            | HydroTool::Streams { fill, .. } => fill,
        }
    }

    /// The accumulations the tool reads, in order.
    fn sums(&self) -> Vec<(Method, Own)> {
        match *self {
            HydroTool::Fill { .. } | HydroTool::Direction { .. } => Vec::new(),
            HydroTool::Accumulation { method, unit, .. } => vec![(
                method,
                if unit == Unit::Cells {
                    Own::Cells
                } else {
                    Own::Area
                },
            )],
            HydroTool::Wetness { method, .. } => vec![(method, Own::Area)],
            HydroTool::PourPoints { .. } => vec![(Method::D8, Own::Cells), (Method::D8, Own::Area)],
            HydroTool::Watershed { snap, .. } => {
                if snap > 0.0 {
                    vec![(Method::D8, Own::Cells)]
                } else {
                    Vec::new()
                }
            }
            HydroTool::Basins { mode, .. } => {
                if mode == BasinMode::Main {
                    Vec::new()
                } else {
                    vec![(Method::D8, Own::Area)]
                }
            }
            HydroTool::Streams { .. } => vec![(Method::D8, Own::Area)],
        }
    }

    /// The raster the tool writes (bands, sample, nodata), none for objects.
    pub fn raster(&self, input: RasterSample) -> Option<(RasterSample, f64)> {
        match *self {
            HydroTool::Fill { depth: false, .. } => Some((
                if input == RasterSample::F64 {
                    RasterSample::F64
                } else {
                    RasterSample::F32
                },
                f64::NAN,
            )),
            HydroTool::Fill { depth: true, .. }
            | HydroTool::Accumulation { .. }
            | HydroTool::Wetness { .. } => Some((RasterSample::F32, f64::NAN)),
            HydroTool::Direction { .. } => Some((RasterSample::U8, 255.0)),
            _ => None,
        }
    }
}

/// What a run met, for the host's summary (§12 and the tools' notes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HydroNotes {
    /// Çukur doldur: cells raised; Akış yönü: flat cells resolved; TWI: cells whose slope met the floor.
    pub cells: u64,
    /// Akış yönü: cells left without a direction.
    pub empty: u64,
    /// Points no cell took (their places in the input).
    pub skipped: Vec<u32>,
    /// Noktadan havza: points whose cell a later point took.
    pub empty_points: Vec<u32>,
    /// Havzalar: basins under the least area.
    pub dropped: u64,
    /// The stream threshold used (m²).
    pub threshold: f64,
    /// Dere ağı: links.
    pub links: u64,
    /// Çukur doldur: the deepest fill; Akış birikimi: the largest value; Dere ağı: the highest Strahler order.
    pub most: f64,
}

/// The run's stage.
enum Stage {
    Read,
    Fill(Box<Filling>),
    /// The flat fill (ε = 0) on the threads, in one step.
    FillTiled,
    Directions,
    Sum {
        at: usize,
        routing: Box<Routing>,
        work: Box<Accumulating>,
    },
    Result,
    Emit,
    Done,
}

/// A hydrology run.
pub struct HydroWork {
    tool: HydroTool,
    band: usize,
    sample: RasterSample,
    pub surface: Surface,
    /// The heights before the fill (Dolgu derinliği).
    original: Option<Vec<f64>>,
    stage: Stage,
    dirs: Vec<u8>,
    sums: Vec<Vec<f64>>,
    /// Points and routes (the run's objects).
    points: Vec<(f64, f64)>,
    routes: Vec<Shape>,
    /// A raster result's values row by row (NaN: none).
    values: Vec<f64>,
    emitted: u32,
    features: Option<Features>,
    pub notes: HydroNotes,
    threads: usize,
}

impl std::fmt::Debug for HydroWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HydroWork")
            .field("tool", &self.tool)
            .finish()
    }
}

impl HydroWork {
    /// A run over a `width` × `height` DEM (its band `band`, samples `sample`)
    /// placed by `affine`; `shapes` the points or the routes.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tool: HydroTool,
        band: usize,
        sample: RasterSample,
        (width, height): (u32, u32),
        affine: [f64; 6],
        geographic: bool,
        shapes: &[Shape],
        threads: usize,
    ) -> Result<HydroWork, String> {
        if u64::from(width) * u64::from(height) > MOST_CELLS {
            return Err(format!(
                "Hidroloji için raster çok büyük ({width} × {height}): en çok 2²⁵ hücre (5792 × 5792). Önce Maskeyle kırp ile bölün ya da Yeniden örnekle ile hücreleri büyütün."
            ));
        }
        let mut points = Vec::new();
        let mut routes = Vec::new();
        for s in shapes {
            match s {
                Shape::Point { p, parts, .. } => {
                    points.push((p.x, p.y));
                    for q in parts.iter().flatten() {
                        points.push((q.p.x, q.p.y));
                    }
                }
                Shape::Line { .. } | Shape::Polyline { .. } | Shape::Arc { .. } => {
                    routes.push(s.clone())
                }
                _ => {}
            }
        }
        match tool {
            HydroTool::PourPoints { .. } | HydroTool::Watershed { .. } if points.is_empty() => {
                return Err("Döküm noktalarını seçin: nokta nesneleri.".into());
            }
            HydroTool::Basins {
                mode: BasinMode::Route,
                ..
            } if routes.is_empty() => {
                return Err("Güzergâhı seçin: çizgi, çoklu çizgi ya da yay.".into());
            }
            _ => {}
        }
        Ok(HydroWork {
            tool,
            band,
            sample,
            surface: Surface::new(width, height, affine, geographic),
            original: None,
            stage: Stage::Read,
            dirs: Vec::new(),
            sums: Vec::new(),
            points,
            routes,
            values: Vec::new(),
            emitted: 0,
            features: None,
            notes: HydroNotes::default(),
            threads: threads.max(1),
        })
    }

    pub fn tool(&self) -> &HydroTool {
        &self.tool
    }

    /// Whether the job still reads the DEM.
    pub fn reading(&self) -> bool {
        matches!(self.stage, Stage::Read)
    }

    /// Whether the run writes a raster's strips now.
    pub fn emitting(&self) -> bool {
        matches!(self.stage, Stage::Emit)
    }

    pub fn done(&self) -> bool {
        matches!(self.stage, Stage::Done)
    }

    /// A block of the DEM's rows (`c0..c1`, `y0..y1`) from the input's samples.
    pub fn block(&mut self, (c0, c1, y0, y1): (u32, u32, u32, u32), raw: &Raw) {
        let w = self.surface.width as usize;
        let band = [self.band];
        let (rs, re) = (y0 as usize * w, y1 as usize * w);
        par::rows(
            self.threads,
            &mut self.surface.z[rs..re],
            w,
            &|first, chunk: &mut [f64]| {
                for (k, row) in chunk.chunks_mut(w).enumerate() {
                    let j = i64::from(y0) + (first + k) as i64;
                    raw.cells(j, i64::from(c0), &band, &mut row[c0 as usize..c1 as usize]);
                }
            },
        );
    }

    /// The DEM is read: the work begins.
    pub fn read_done(&mut self) {
        let slope = match self.tool {
            HydroTool::Fill { slope, depth } => {
                if depth {
                    self.original = Some(self.surface.z.clone());
                }
                slope / 100.0
            }
            _ => 0.0,
        };
        self.stage = if !self.tool.fills() {
            Stage::Directions
        } else if slope == 0.0 && self.threads > 1 {
            Stage::FillTiled
        } else {
            Stage::Fill(Box::new(Filling::new(&self.surface, slope)))
        };
    }

    /// The share done: reading `f` of the DEM, then the stages.
    pub fn share(&self, read: f64) -> f64 {
        match &self.stage {
            Stage::Read => 0.2 * read,
            Stage::Fill(f) => 0.2 + 0.35 * f.share(),
            Stage::FillTiled => 0.2,
            Stage::Directions => 0.55,
            Stage::Sum { at, work, .. } => {
                let n = self.tool.sums().len().max(1) as f64;
                0.6 + 0.3 * (*at as f64 + work.share()) / n
            }
            Stage::Result => 0.9,
            Stage::Emit => {
                0.92 + 0.08 * f64::from(self.emitted) / f64::from(self.surface.height.max(1))
            }
            Stage::Done => 1.0,
        }
    }

    /// One step of the work.
    pub fn step(&mut self) -> Result<(), String> {
        let threads = self.threads;
        match std::mem::replace(&mut self.stage, Stage::Done) {
            Stage::Read | Stage::Emit | Stage::Done => {
                return Err("Hidroloji işi bu aşamada adım almaz.".into());
            }
            Stage::FillTiled => {
                self.notes.cells = tiled::fill(&mut self.surface, threads);
                self.stage = if let HydroTool::Fill { .. } = self.tool {
                    Stage::Result
                } else {
                    Stage::Directions
                };
            }
            Stage::Fill(mut f) => {
                if f.step(&mut self.surface) {
                    self.notes.cells = f.raised;
                    self.stage = if let HydroTool::Fill { .. } = self.tool {
                        Stage::Result
                    } else {
                        Stage::Directions
                    };
                } else {
                    self.stage = Stage::Fill(f);
                }
            }
            Stage::Directions => {
                let mut dirs = d8(&self.surface, threads);
                let resolved = resolve_flats(&self.surface, &mut dirs, threads);
                if let HydroTool::Direction { .. } = self.tool {
                    self.notes.cells = resolved;
                    self.notes.empty = dirs.iter().filter(|&&d| d == NOFLOW).count() as u64;
                }
                self.dirs = dirs;
                self.stage = self.next_sum(0);
            }
            Stage::Sum {
                at,
                routing,
                mut work,
            } => {
                if work.step(&self.surface, &self.dirs, &routing, threads) {
                    self.sums.push(std::mem::take(&mut work.acc));
                    self.stage = self.next_sum(at + 1);
                } else {
                    self.stage = Stage::Sum { at, routing, work };
                }
            }
            Stage::Result => {
                self.result()?;
                self.stage = if self.tool.raster(self.sample).is_some() {
                    Stage::Emit
                } else {
                    Stage::Done
                };
            }
        }
        Ok(())
    }

    /// The accumulation `at` of the tool's, or the result when none is left.
    fn next_sum(&self, at: usize) -> Stage {
        match self.tool.sums().get(at) {
            Some(&(method, own)) => {
                let routing = Routing::new(&self.surface, method, self.threads);
                let work =
                    Accumulating::new(&self.surface, &self.dirs, &routing, own, self.threads);
                Stage::Sum {
                    at,
                    routing: Box::new(routing),
                    work: Box::new(work),
                }
            }
            None => Stage::Result,
        }
    }

    /// The tool's result: a raster's values or the objects.
    fn result(&mut self) -> Result<(), String> {
        let s = &self.surface;
        let w = s.width as usize;
        let n = s.len();
        match self.tool {
            HydroTool::Fill { .. } => match self.original.take() {
                Some(z0) => {
                    self.values = s.z.iter().zip(&z0).map(|(f, z)| f - z).collect();
                    self.notes.most = self
                        .values
                        .iter()
                        .fold(0.0, |m, &v| if v > m { v } else { m });
                }
                None => self.values = s.z.clone(),
            },
            HydroTool::Direction { coding, .. } => {
                let codes = if coding == Coding::Esri { ESRI } else { TAUDEM };
                self.values = self
                    .dirs
                    .iter()
                    .map(|&d| match d {
                        NONE => f64::NAN,
                        NOFLOW => 0.0,
                        q => f64::from(codes[q as usize]),
                    })
                    .collect();
            }
            HydroTool::Accumulation { unit, .. } => {
                let mut acc = self.sums.pop().unwrap_or_default();
                if unit == Unit::Sca {
                    for (k, a) in acc.iter_mut().enumerate() {
                        *a /= s.row(k / w).width;
                    }
                }
                self.notes.most = acc
                    .iter()
                    .filter(|v| !v.is_nan())
                    .fold(0.0, |m, &v| if v > m { v } else { m });
                self.values = acc;
            }
            HydroTool::Wetness { slope, .. } => {
                let area = self.sums.pop().unwrap_or_default();
                let floor = slope / 100.0;
                let mut out = vec![f64::NAN; n];
                let floored = std::sync::atomic::AtomicU64::new(0);
                par::rows(self.threads, &mut out, w, &|first, chunk: &mut [f64]| {
                    let mut low = 0u64;
                    for (r, row) in chunk.chunks_mut(w).enumerate() {
                        let j = first + r;
                        let geom = s.row(j);
                        let grad = Gradient::new(geom.axes, 1.0);
                        for (i, o) in row.iter_mut().enumerate() {
                            let k = j * w + i;
                            if !s.valid(k) {
                                continue;
                            }
                            let mut t = horn(s, &grad, i, j);
                            if t < floor {
                                t = floor;
                                low += 1;
                            }
                            *o = log(area[k] / geom.width / t);
                        }
                    }
                    floored.fetch_add(low, std::sync::atomic::Ordering::Relaxed);
                });
                self.notes.cells = floored.into_inner();
                self.values = out;
            }
            HydroTool::PourPoints { snap: r, .. } => {
                let area = self.sums.pop().unwrap_or_default();
                let cells = self.sums.pop().unwrap_or_default();
                let mut out = Features::with_fields(
                    FeatureKind::Points,
                    &["Nokta", "Birikim", "Alan", "Uzaklık"],
                );
                for (x, &pt) in self.points.iter().enumerate() {
                    match snap(s, &cells, pt, r) {
                        None => self.notes.skipped.push(x as u32),
                        Some((k, dist)) => {
                            let (i, j) = (k % w, k / w);
                            out.push_point(
                                [i as f64 + 0.5, j as f64 + 0.5],
                                &s.affine,
                                f64::NAN,
                                String::new(),
                                0,
                            );
                            out.numbers
                                .extend([x as f64 + 1.0, cells[k], area[k], dist]);
                        }
                    }
                }
                self.features = Some(out);
            }
            HydroTool::Watershed { snap: r, .. } => {
                let cells = self.sums.pop().unwrap_or_default();
                // The pour cells; a later point takes a cell an earlier one took (ArcGIS's rule).
                let mut owner: Vec<(usize, u32)> = Vec::new();
                for (x, &pt) in self.points.iter().enumerate() {
                    match snap(s, &cells, pt, r) {
                        None => self.notes.skipped.push(x as u32),
                        Some((k, _)) => {
                            owner.retain(|&(c, _)| c != k);
                            owner.push((k, x as u32));
                        }
                    }
                }
                let mut by_point: Vec<Option<usize>> = vec![None; self.points.len()];
                for &(k, x) in &owner {
                    by_point[x as usize] = Some(k);
                }
                let mut labels = vec![0u32; n];
                let mut order: Vec<u32> = Vec::new();
                let mut seeds = Vec::new();
                for (x, cell) in by_point.iter().enumerate() {
                    if self.notes.skipped.contains(&(x as u32)) {
                        continue;
                    }
                    match cell {
                        None => self.notes.empty_points.push(x as u32),
                        Some(k) => {
                            order.push(x as u32);
                            labels[*k] = order.len() as u32;
                            seeds.push(*k as u32);
                        }
                    }
                }
                label_upstream(s, &self.dirs, &mut labels, &seeds);
                let count = order.len();
                let sizes = areas(s, &labels, count);
                let rings = rings_of(labels, s.width, s.height, count)?;
                let mut out = Features::with_fields(FeatureKind::Polygons, &["Havza", "Alan"]);
                for (l, x) in order.iter().enumerate() {
                    push_rings(&mut out, &rings[l], &s.affine);
                    out.numbers.extend([f64::from(*x) + 1.0, sizes[l]]);
                }
                self.features = Some(out);
            }
            HydroTool::Basins {
                mode,
                threshold,
                least,
                ..
            } => {
                self.features = Some(self.basins(mode, threshold, least)?);
            }
            HydroTool::Streams {
                threshold,
                simplify,
                ..
            } => {
                let area = self.sums.pop().unwrap_or_default();
                let net = network(s, &self.dirs, &area, threshold);
                self.notes.threshold = net.threshold;
                self.notes.links = net.links.len() as u64;
                let mut out = Features::with_fields(
                    FeatureKind::Lines,
                    &[
                        "Bağ", "Sıra", "Shreve", "Uzunluk", "Düşü", "Eğim", "Alan", "Aşağı",
                    ],
                );
                let mut top = 0u32;
                for (x, l) in net.links.iter().enumerate() {
                    let path = l.path();
                    let length = path_length(s, &path);
                    let first = path[0] as usize;
                    let last = path[path.len() - 1] as usize;
                    let drop = s.z[first] - s.z[last];
                    let slope = if length > 0.0 { drop / length } else { 0.0 };
                    let pts: Vec<[f64; 2]> = path
                        .iter()
                        .map(|&k| [(k as usize % w) as f64 + 0.5, (k as usize / w) as f64 + 0.5])
                        .collect();
                    let kept = douglas_peucker(&pts, simplify);
                    out.push_line(&kept, &s.affine, f64::NAN, String::new());
                    let end = l.cells[l.cells.len() - 1] as usize;
                    out.numbers.extend([
                        x as f64 + 1.0,
                        f64::from(l.strahler),
                        f64::from(l.shreve),
                        length,
                        drop,
                        slope,
                        area[end],
                        f64::from(l.down),
                    ]);
                    top = top.max(l.strahler);
                }
                self.notes.most = f64::from(top);
                self.features = Some(out);
            }
        }
        Ok(())
    }

    fn basins(&mut self, mode: BasinMode, threshold: f64, least: f64) -> Result<Features, String> {
        let s = &self.surface;
        let n = s.len();
        let w = s.width as usize;
        match mode {
            BasinMode::Main => {
                let ends = terminals(s, &self.dirs);
                let mut labels = vec![0u32; n];
                for (x, &k) in ends.iter().enumerate() {
                    labels[k as usize] = x as u32 + 1;
                }
                label_upstream(s, &self.dirs, &mut labels, &ends);
                let sizes = areas(s, &labels, ends.len());
                // Kept basins renumbered 1… in order; the small ones gone.
                let mut renumber = vec![0u32; ends.len() + 1];
                let mut kept: Vec<f64> = Vec::new();
                for (x, &a) in sizes.iter().enumerate() {
                    if a < least {
                        self.notes.dropped += 1;
                    } else {
                        kept.push(a);
                        renumber[x + 1] = kept.len() as u32;
                    }
                }
                for l in labels.iter_mut() {
                    *l = renumber[*l as usize];
                }
                let rings = rings_of(labels, s.width, s.height, kept.len())?;
                let mut out = Features::with_fields(FeatureKind::Polygons, &["Havza", "Alan"]);
                for (x, a) in kept.iter().enumerate() {
                    push_rings(&mut out, &rings[x], &s.affine);
                    out.numbers.extend([x as f64 + 1.0, *a]);
                }
                Ok(out)
            }
            BasinMode::Sub => {
                let area = self.sums.pop().unwrap_or_default();
                let net = network(s, &self.dirs, &area, threshold);
                self.notes.threshold = net.threshold;
                let mut labels = net.link_of.clone();
                let seeds: Vec<u32> = (0..n)
                    .filter(|&k| labels[k] != 0)
                    .map(|k| k as u32)
                    .collect();
                label_upstream(s, &self.dirs, &mut labels, &seeds);
                let sizes = areas(s, &labels, net.links.len());
                let mut renumber = vec![0u32; net.links.len() + 1];
                let mut kept: Vec<usize> = Vec::new();
                for (x, &a) in sizes.iter().enumerate() {
                    if a < least {
                        self.notes.dropped += 1;
                    } else {
                        kept.push(x);
                        renumber[x + 1] = kept.len() as u32;
                    }
                }
                for l in labels.iter_mut() {
                    *l = renumber[*l as usize];
                }
                let rings = rings_of(labels, s.width, s.height, kept.len())?;
                let mut out =
                    Features::with_fields(FeatureKind::Polygons, &["Bağ", "Sıra", "Alan"]);
                for (r, &x) in kept.iter().enumerate() {
                    push_rings(&mut out, &rings[r], &s.affine);
                    out.numbers.extend([
                        x as f64 + 1.0,
                        f64::from(net.links[x].strahler),
                        sizes[x],
                    ]);
                }
                Ok(out)
            }
            BasinMode::Route => {
                let area = self.sums.pop().unwrap_or_default();
                let net = network(s, &self.dirs, &area, threshold);
                self.notes.threshold = net.threshold;
                let mut out =
                    Features::with_fields(FeatureKind::Polygons, &["Havza", "Km", "Sıra", "Alan"]);
                let mut count = 0u32;
                let mut cells = Vec::new();
                for route in &self.routes {
                    let crossed: Vec<u32> = crossed_cells(s, route)
                        .into_iter()
                        .filter(|&k| net.link_of[k as usize] != 0)
                        .collect();
                    let mut here: Vec<(f64, usize)> = Vec::new();
                    for &k in &crossed {
                        let k = k as usize;
                        let ends = match receiver(s, &self.dirs, k) {
                            None => true,
                            Some(to) => crossed.binary_search(&(to as u32)).is_err(),
                        };
                        if ends {
                            let (x, y) =
                                point_of(&s.affine, (k % w) as f64 + 0.5, (k / w) as f64 + 0.5);
                            let km = reading(route, false, Vec2::new(x, y)).map_or(0.0, |r| r.s);
                            here.push((km, k));
                        }
                    }
                    // By km, then row by row.
                    here.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                    for (km, k) in here {
                        upstream(s, &self.dirs, k, &mut cells);
                        let a = area_of(s, &mut cells);
                        if a < least {
                            self.notes.dropped += 1;
                            continue;
                        }
                        count += 1;
                        let rings = cells_rings(s, &cells)?;
                        push_rings(&mut out, &rings, &s.affine);
                        out.numbers.extend([
                            f64::from(count),
                            km,
                            f64::from(net.links[net.link_of[k] as usize - 1].strahler),
                            a,
                        ]);
                    }
                }
                Ok(out)
            }
        }
    }

    /// A raster result's strip: rows `y0..y0 + n` in the result's sample.
    pub fn strip(&mut self, n: u32, sample: RasterSample) -> Samples {
        let w = self.surface.width as usize;
        let (a, b) = (self.emitted as usize * w, (self.emitted + n) as usize * w);
        let vals = &self.values[a..b];
        self.emitted += n;
        if self.emitted >= self.surface.height {
            self.stage = Stage::Done;
        }
        match sample {
            RasterSample::U8 => Samples::U8(
                vals.iter()
                    .map(|&v| if v.is_nan() { 255 } else { v as u8 })
                    .collect(),
            ),
            RasterSample::F64 => Samples::F64(vals.to_vec()),
            _ => Samples::F32(vals.iter().map(|&v| v as f32).collect()),
        }
    }

    /// The objects.
    pub fn finish(&mut self) -> Result<Features, String> {
        self.features
            .take()
            .ok_or_else(|| "Hidroloji işinin nesneleri yok.".into())
    }
}

/// An area's rings (cell space) into `out`, turned when the affine mirrors.
fn push_rings(out: &mut Features, rings: &[crate::vector::rings::Ring], affine: &[f64; 6]) {
    let [_, a, b, _, c, d] = *affine;
    let flip = a * d - b * c > 0.0;
    let world: Vec<Vec<[f64; 2]>> = rings.iter().map(|r| world_ring(r, flip)).collect();
    out.push_area(&world, affine, (f64::NAN, String::new()), 0);
}

/// Horn's gradient size at a cell (ADR 0231 §3): the missing neighbours the centre's.
fn horn(s: &Surface, grad: &Gradient, i: usize, j: usize) -> f64 {
    let w = s.width as usize;
    let c = s.z[j * w + i];
    let mut win = [c; 9];
    let mut x = 0;
    for dj in -1i64..=1 {
        for di in -1i64..=1 {
            let (a, b) = (i as i64 + di, j as i64 + dj);
            if a >= 0 && b >= 0 && a < i64::from(s.width) && b < i64::from(s.height) {
                let v = s.z[b as usize * w + a as usize];
                if !v.is_nan() {
                    win[x] = v;
                }
            }
            x += 1;
        }
    }
    let (pi, pj) = derivatives(&win, Gradients::Horn);
    let (gx, gy) = grad.of(pi, pj);
    sqrt(gx * gx + gy * gy)
}

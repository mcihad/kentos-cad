//! The vectorizing tools' work inside the operation job (docs/adr/0234 §4–§8):
//! what a block of rows gives each tool, what a whole strip and a whole pass
//! do, and the features at the end. The job reads the blocks; this decides.

use kentos_contracts::RasterSample;

use super::capture::{self, Colour, FIRST_WINDOW, MOST_LINE_CELLS, Window};
use super::label::{Labeler, Regions};
use super::rings::{Ring, region_rings, twice_area};
use super::simplify::{douglas_peucker, douglas_peucker_closed};
use super::thin::{Mask, clean_paths, thin};
use super::{FeatureKind, Features, MOST_CELLS, MOST_FEATURES, MOST_POINTS, value_text};
use crate::grid::Grid;
use crate::inputs::{Input, Mapping, Raw, mapping, place_in};
use crate::par;

/// Which cells are a line's (Rasterden çizgi, §5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Select {
    NonZero,
    Range { min: f64, max: f64 },
    Colour { colour: Colour, tol: f64 },
}

/// Rasterden nokta's cells (§6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointMode {
    All,
    Step(u32),
    Extrema(u32),
}

/// A vectorizing tool's settings, read by the job from the host's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorTool {
    Polygons {
        band: usize,
        eight: bool,
    },
    Lines {
        band: usize,
        select: Select,
        spur: u32,
        eps: f64,
    },
    Points {
        band: usize,
        mode: PointMode,
    },
    CaptureLine {
        at: (f64, f64),
        tol: f64,
        spur: u32,
        eps: f64,
    },
    CloseArea {
        at: (f64, f64),
        tol: f64,
        keep_holes: bool,
        eps: f64,
    },
}

enum Phase {
    /// The 7 × 7 cells round the click: their colours.
    Seed(Vec<Option<Colour>>),
    /// A window: its cells near the target colour (1).
    Window(Vec<u8>),
}

struct Capture {
    close: bool,
    click: (i64, i64),
    tol: f64,
    spur: u32,
    eps: f64,
    keep_holes: bool,
    rgb: bool,
    phase: Phase,
    seed: (i64, i64),
    target: Colour,
    side: u64,
    win: Window,
    passes: u32,
    /// The flood's cells once the window holds it.
    found: Option<Vec<u32>>,
}

enum Task {
    Polygons {
        band: usize,
        labeler: Option<Labeler>,
        strip: Vec<f64>,
        regions: Option<Regions>,
    },
    Lines {
        band: usize,
        select: Select,
        mask: Mask,
        spur: u32,
        eps: f64,
    },
    Points {
        band: usize,
        mode: PointMode,
        out: Features,
        strip: Vec<(u32, u32, f64, u8)>,
        too_many: bool,
    },
    Capture(Box<Capture>),
}

/// Whether a cell (value `v`) is a peak (1) or a pit (2) among the cells
/// with a value in its (2r + 1)² window (`value(di, dj)` its neighbours,
/// NaN off the raster); none without such a cell.
fn extremum(value: &dyn Fn(i64, i64) -> f64, v: f64, r: i64) -> Option<u8> {
    let (mut peak, mut pit, mut any) = (true, true, false);
    for dj in -r..=r {
        for di in -r..=r {
            if di == 0 && dj == 0 {
                continue;
            }
            let u = value(di, dj);
            if u.is_nan() {
                continue;
            }
            any = true;
            peak &= u < v;
            pit &= u > v;
            if !peak && !pit {
                return None;
            }
        }
    }
    match (any, peak, pit) {
        (true, true, _) => Some(1),
        (true, _, true) => Some(2),
        _ => None,
    }
}

/// A vectorizing run's state.
pub struct VectorWork {
    affine: [f64; 6],
    size: (u32, u32),
    sample: RasterSample,
    /// The grid the job reads now (the raster's, or a capture's window) and its place on the raster.
    grid: Grid,
    map: Mapping,
    task: Task,
    threads: usize,
}

fn sub_grid(affine: &[f64; 6], win: Window) -> Result<Grid, String> {
    let [x0, a, b, y0, c, d] = *affine;
    let (u, v) = (f64::from(win.i0), f64::from(win.j0));
    Grid::of(
        [x0 + a * u + b * v, a, b, y0 + c * u + d * v, c, d],
        win.w,
        win.h,
    )
}

impl VectorWork {
    /// The work over `input`, and the grid of its first pass.
    pub fn new(
        tool: VectorTool,
        input: &Input,
        threads: usize,
    ) -> Result<(VectorWork, Grid), String> {
        let size = (input.width, input.height);
        let cells = u64::from(size.0) * u64::from(size.1);
        let rgb = input.values() >= 3;
        let band_ok = |b: usize| -> Result<usize, String> {
            if b < input.values() as usize {
                Ok(b)
            } else {
                Err(format!(
                    "Rasterin {} bandı var; {}. bant yok.",
                    input.values(),
                    b + 1
                ))
            }
        };
        let whole = || -> Result<(), String> {
            if cells > MOST_CELLS {
                Err(format!(
                    "Raster vektörleştirme için çok büyük ({} × {}): en çok 2²⁶ hücre (8192 × 8192). Önce Maskeyle kırp ile bölün ya da Yeniden örnekle ile hücreleri büyütün.",
                    size.0, size.1
                ))
            } else {
                Ok(())
            }
        };
        let grid = input.grid();
        let task = match tool {
            VectorTool::Polygons { band, eight } => {
                whole()?;
                Task::Polygons {
                    band: band_ok(band)?,
                    labeler: Some(Labeler::new(size.0, size.1, eight)?),
                    strip: Vec::new(),
                    regions: None,
                }
            }
            VectorTool::Lines {
                band,
                select,
                spur,
                eps,
            } => {
                whole()?;
                if matches!(select, Select::Colour { .. }) && !rgb {
                    return Err("Renkle seçmek üç bantlı (RGB) bir raster ister; bu rasterde Değer aralığı'nı kullanın.".into());
                }
                Task::Lines {
                    band: band_ok(band)?,
                    select,
                    mask: Mask::new(size.0 as usize, size.1 as usize),
                    spur,
                    eps,
                }
            }
            VectorTool::Points { band, mode } => Task::Points {
                band: band_ok(band)?,
                mode,
                out: Features::new(FeatureKind::Points),
                strip: Vec::new(),
                too_many: false,
            },
            VectorTool::CaptureLine { at, tol, spur, eps } => Task::Capture(Box::new(
                Capture::new(input, at, tol, false, (spur, eps, false), rgb)?,
            )),
            VectorTool::CloseArea {
                at,
                tol,
                keep_holes,
                eps,
            } => Task::Capture(Box::new(Capture::new(
                input,
                at,
                tol,
                true,
                (0, eps, keep_holes),
                rgb,
            )?)),
        };
        let first = match &task {
            Task::Capture(c) => sub_grid(&input.affine, c.win)?,
            _ => grid,
        };
        let work = VectorWork {
            affine: input.affine,
            size,
            sample: input.sample,
            grid: first,
            map: mapping(&first, &input.affine),
            task,
            threads: threads.max(1),
        };
        Ok((work, first))
    }

    /// Rows round a block a tool reads besides (Tepeler ve çukurlar' window).
    pub fn margin(&self) -> i64 {
        match &self.task {
            Task::Points {
                mode: PointMode::Extrema(r),
                ..
            } => i64::from(*r),
            _ => 0,
        }
    }

    /// A block of the current grid's rows (`c0..c1`, `y0..y1`), its cells
    /// read from `raw` (the input's samples) row by row on the threads.
    pub fn block(&mut self, rect: (u32, u32, u32, u32), raw: &Raw) -> Result<(), String> {
        let (c0, c1, y0, y1) = rect;
        let Mapping::Offset(ko, lo) = self.map else {
            return Err("Vektörleştirmenin ızgarası rasterin kendisininki olmalı.".into());
        };
        let bw = (c1 - c0) as usize;
        let gw = self.grid.width as usize;
        let threads = self.threads;
        // Grid row j's input row, grid column c0's input column.
        let (row_of, col0) = (move |j: usize| j as i64 + lo, i64::from(c0) + ko);
        let (rs, re) = (y0 as usize * gw, y1 as usize * gw);
        match &mut self.task {
            Task::Polygons { band, strip, .. } => {
                let n = (y1 - y0) as usize;
                if strip.len() != n * gw {
                    *strip = vec![f64::NAN; n * gw];
                }
                let b = [*band];
                par::rows(threads, strip, gw, &|first, chunk: &mut [f64]| {
                    for (k, row) in chunk.chunks_mut(gw).enumerate() {
                        let j = row_of(y0 as usize + first + k);
                        raw.cells(j, col0, &b, &mut row[c0 as usize..c1 as usize]);
                    }
                });
            }
            Task::Lines {
                band, select, mask, ..
            } => {
                let select = *select;
                let bands: Vec<usize> = match select {
                    Select::Colour { .. } => vec![0, 1, 2],
                    _ => vec![*band],
                };
                let k = bands.len();
                par::rows(
                    threads,
                    &mut mask.cells[rs..re],
                    gw,
                    &|first, chunk: &mut [u8]| {
                        let mut buf = vec![0.0; bw * k];
                        for (q, row) in chunk.chunks_mut(gw).enumerate() {
                            raw.cells(row_of(y0 as usize + first + q), col0, &bands, &mut buf);
                            for (c, px) in buf.chunks_exact(k).enumerate() {
                                let on = match select {
                                    Select::NonZero => !px[0].is_nan() && px[0] != 0.0,
                                    Select::Range { min, max } => {
                                        !px[0].is_nan() && min <= px[0] && px[0] <= max
                                    }
                                    Select::Colour { colour, tol } => {
                                        !px.iter().any(|v| v.is_nan())
                                            && capture::near(
                                                &[px[0], px[1], px[2]],
                                                &colour,
                                                tol,
                                                true,
                                            )
                                    }
                                };
                                if on {
                                    row[c0 as usize + c] = 1;
                                }
                            }
                        }
                    },
                );
            }
            Task::Points {
                band, mode, strip, ..
            } => {
                let (band, mode) = ([*band], *mode);
                let r = match mode {
                    PointMode::Extrema(r) => i64::from(r),
                    _ => 0,
                };
                let on = |n: u32| match mode {
                    PointMode::Step(k) => n % k == k / 2,
                    _ => true,
                };
                let rows: Vec<u32> = (y0..y1).filter(|&j| on(j)).collect();
                // Each row with r rows round it and r cells either side (NaN off the raster).
                let width = bw + 2 * r as usize;
                let found = par::map(threads, &rows, &|&j| {
                    let mut win = vec![f64::NAN; (2 * r as usize + 1) * width];
                    for (q, dj) in (-r..=r).enumerate() {
                        let jj = row_of(j as usize) + dj;
                        raw.cells(jj, col0 - r, &band, &mut win[q * width..(q + 1) * width]);
                    }
                    let at = |di: i64, dj: i64, c: usize| {
                        win[((dj + r) as usize) * width + (c as i64 + r + di) as usize]
                    };
                    let mut out = Vec::new();
                    for (c, i) in (c0..c1).enumerate().filter(|&(_, i)| on(i)) {
                        let v = at(0, 0, c);
                        if v.is_nan() {
                            continue;
                        }
                        let tag = match mode {
                            PointMode::All | PointMode::Step(_) => Some(0),
                            PointMode::Extrema(_) => extremum(&|di, dj| at(di, dj, c), v, r),
                        };
                        if let Some(tag) = tag {
                            out.push((j, i, v, tag));
                        }
                    }
                    out
                });
                strip.extend(found.into_iter().flatten());
            }
            Task::Capture(c) => {
                let bands: Vec<usize> = if c.rgb { vec![0, 1, 2] } else { vec![0] };
                let k = bands.len();
                let (target, tol, rgb) = (c.target, c.tol, c.rgb);
                let colour = |px: &[f64]| -> Option<Colour> {
                    let col = if rgb {
                        [px[0], px[1], px[2]]
                    } else {
                        [px[0], 0.0, 0.0]
                    };
                    (!col.iter().any(|v| v.is_nan())).then_some(col)
                };
                match &mut c.phase {
                    Phase::Seed(colours) => {
                        let mut buf = vec![0.0; bw * k];
                        for j in y0..y1 {
                            raw.cells(row_of(j as usize), col0, &bands, &mut buf);
                            for (q, px) in buf.chunks_exact(k).enumerate() {
                                colours[j as usize * gw + c0 as usize + q] = colour(px);
                            }
                        }
                    }
                    Phase::Window(mask) => {
                        par::rows(
                            threads,
                            &mut mask[rs..re],
                            gw,
                            &|first, chunk: &mut [u8]| {
                                let mut buf = vec![0.0; bw * k];
                                for (q, row) in chunk.chunks_mut(gw).enumerate() {
                                    raw.cells(
                                        row_of(y0 as usize + first + q),
                                        col0,
                                        &bands,
                                        &mut buf,
                                    );
                                    for (c, px) in buf.chunks_exact(k).enumerate() {
                                        if colour(px).is_some_and(|col| {
                                            capture::near(&col, &target, tol, rgb)
                                        }) {
                                            row[c0 as usize + c] = 1;
                                        }
                                    }
                                }
                            },
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// A whole strip (`n` rows from `y0`) is in.
    pub fn strip_done(&mut self, _y0: u32, n: u32) -> Result<(), String> {
        let gw = self.grid.width as usize;
        match &mut self.task {
            Task::Polygons { labeler, strip, .. } => {
                let l = labeler.as_mut().ok_or("Bölgeler zaten kuruldu.")?;
                for k in 0..n as usize {
                    l.row(&strip[k * gw..(k + 1) * gw])?;
                }
            }
            Task::Points {
                out,
                strip,
                too_many,
                ..
            } => {
                strip.sort_by_key(|p| (p.0, p.1));
                for &(j, i, v, tag) in strip.iter() {
                    if out.len() >= MOST_POINTS {
                        *too_many = true;
                        break;
                    }
                    let text = value_text(v, self.sample);
                    out.push_point(
                        [f64::from(i) + 0.5, f64::from(j) + 0.5],
                        &self.affine,
                        v,
                        text,
                        tag,
                    );
                }
                strip.clear();
            }
            _ => {}
        }
        Ok(())
    }

    /// A pass is over: the grid of the next one, if the work reads again.
    pub fn pass_done(&mut self) -> Result<Option<Grid>, String> {
        let size = self.size;
        let affine = self.affine;
        match &mut self.task {
            Task::Polygons {
                labeler, regions, ..
            } => {
                let l = labeler.take().ok_or("Bölgeler zaten kuruldu.")?;
                *regions = Some(l.finish(MOST_FEATURES)?);
                Ok(None)
            }
            Task::Points { too_many: true, .. } => Err(format!(
                "Rasterden {MOST_POINTS} noktadan fazlası çıkıyor: Adımla'yı seçin ya da Adım'ı büyütün."
            )),
            Task::Capture(c) => {
                let next = c.pass_done(size)?;
                Ok(match next {
                    Some(win) => {
                        let g = sub_grid(&affine, win)?;
                        self.grid = g;
                        self.map = mapping(&g, &affine);
                        Some(g)
                    }
                    None => None,
                })
            }
            _ => Ok(None),
        }
    }

    /// The share done, `f` of the current pass done: a capture's passes
    /// are not known ahead, so its p-th pass runs from p / (p + 1) towards
    /// (p + 1) / (p + 2); every other tool reads once.
    pub fn share(&self, f: f64) -> f64 {
        match &self.task {
            Task::Capture(c) => {
                let p = f64::from(c.passes);
                (p + f) / (p + f + 1.0)
            }
            _ => f,
        }
    }

    /// The features.
    pub fn finish(self) -> Result<Features, String> {
        let threads = self.threads;
        let (affine, sample) = (self.affine, self.sample);
        let flip = {
            let [_, a, b, _, c, d] = affine;
            a * d - b * c > 0.0
        };
        match self.task {
            Task::Polygons { regions, .. } => {
                let regions = regions.ok_or("Rasterin satırları eksik geldi.")?;
                let all = region_rings(&regions)?;
                let mut out = Features::new(FeatureKind::Polygons);
                for (k, rings) in all.iter().enumerate() {
                    let v = regions.values[k];
                    let rings: Vec<Vec<[f64; 2]>> =
                        rings.iter().map(|r| world_ring(r, flip)).collect();
                    out.push_area(&rings, &affine, (v, value_text(v, sample)), 0);
                }
                Ok(out)
            }
            Task::Lines {
                mut mask,
                spur,
                eps,
                ..
            } => {
                thin(&mut mask, threads);
                let paths = clean_paths(&mut mask, spur, threads);
                if paths.len() > MOST_FEATURES {
                    return Err(format!(
                        "Rasterden {MOST_FEATURES} çizgiden fazlası çıkıyor: çizgi hücrelerini daraltın ya da Kısa parçaları at'ı büyütün."
                    ));
                }
                Ok(lines_of(&paths, (0, 0), eps, &affine))
            }
            Task::Points { out, .. } => Ok(out),
            Task::Capture(c) => c.finish(&affine, flip, threads),
        }
    }
}

/// A ring's corners as float64, turned when the affine mirrors the cell
/// space (an outline then still runs counter-clockwise in the world).
pub(crate) fn world_ring(r: &Ring, flip: bool) -> Vec<[f64; 2]> {
    let mut pts: Vec<[f64; 2]> = r.iter().map(|&[u, v]| [u as f64, v as f64]).collect();
    if flip && pts.len() > 1 {
        pts[1..].reverse();
    }
    pts
}

/// Polylines through the paths' pixel centres (offset by `off`), simplified.
fn lines_of(paths: &[super::thin::Path], off: (u32, u32), eps: f64, affine: &[f64; 6]) -> Features {
    let mut out = Features::new(FeatureKind::Lines);
    for p in paths {
        let pts: Vec<[f64; 2]> = p
            .iter()
            .map(|&(i, j)| [f64::from(i + off.0) + 0.5, f64::from(j + off.1) + 0.5])
            .collect();
        let closed = pts.len() > 2 && pts.first() == pts.last();
        let kept = if closed {
            douglas_peucker_closed(&pts, eps)
        } else {
            douglas_peucker(&pts, eps)
        };
        out.push_line(&kept, affine, f64::NAN, String::new());
    }
    out
}

impl Capture {
    fn new(
        input: &Input,
        at: (f64, f64),
        tol: f64,
        close: bool,
        (spur, eps, keep_holes): (u32, f64, bool),
        rgb: bool,
    ) -> Result<Capture, String> {
        let (u, v) = place_in(&input.affine, at.0, at.1);
        let (i, j) = (u.floor(), v.floor());
        if !(i >= 0.0 && j >= 0.0 && i < f64::from(input.width) && j < f64::from(input.height)) {
            return Err("Seçilen nokta rasterin dışında.".into());
        }
        if !(tol >= 0.0) {
            return Err("Renk toleransı 0 ya da büyük olmalı.".into());
        }
        let click = (i as i64, j as i64);
        let reach = if close { 0 } else { capture::SEED_REACH };
        let win = capture::window(click, (2 * reach + 1) as u64, (input.width, input.height));
        Ok(Capture {
            close,
            click,
            tol,
            spur,
            eps,
            keep_holes,
            rgb,
            phase: Phase::Seed(vec![None; (win.w * win.h) as usize]),
            seed: click,
            target: [0.0; 3],
            side: FIRST_WINDOW / 2,
            win,
            passes: 0,
            found: None,
        })
    }

    /// The seed chosen, or the flood tried: the next window, if any.
    fn pass_done(&mut self, size: (u32, u32)) -> Result<Option<Window>, String> {
        match &mut self.phase {
            Phase::Seed(colours) => {
                let win = self.win;
                let colour = |i: i64, j: i64| -> Option<Colour> {
                    let (wi, wj) = (i - i64::from(win.i0), j - i64::from(win.j0));
                    if wi < 0 || wj < 0 || wi >= i64::from(win.w) || wj >= i64::from(win.h) {
                        return None;
                    }
                    colours[(wj * i64::from(win.w) + wi) as usize]
                };
                let found = if self.close {
                    colour(self.click.0, self.click.1).map(|c| (self.click, c))
                } else {
                    capture::seed(self.click, self.rgb, &colour)
                };
                let Some((seed, target)) = found else {
                    return Err(if self.close {
                        "Seçilen noktanın hücresinin değeri yok.".into()
                    } else {
                        "Noktanın çevresinde değeri olan hücre yok.".into()
                    });
                };
                self.seed = seed;
                self.target = target;
            }
            Phase::Window(mask) => {
                let start = (
                    (self.seed.0 - i64::from(self.win.i0)) as u32,
                    (self.seed.1 - i64::from(self.win.j0)) as u32,
                );
                let most = if self.close {
                    usize::MAX
                } else {
                    MOST_LINE_CELLS
                };
                let f = capture::flood(mask, self.win, size, start, !self.close, most);
                if !self.close && f.cells.len() > MOST_LINE_CELLS {
                    return Err(format!(
                        "Tıklanan yer bir çizgi değil gibi: {MOST_LINE_CELLS} hücreden büyük bir bölge. Çizginin üstüne tıklayın ya da Renk toleransı'nı küçültün."
                    ));
                }
                if self.close && f.edge {
                    return Err("Alan kapanmıyor: dolgu rasterin kenarına ulaştı. Boşluğu kapatın ya da Renk toleransı'nı küçültün.".into());
                }
                if !f.inner {
                    self.found = Some(f.cells);
                    return Ok(None);
                }
            }
        }
        // A window, or a window twice as wide.
        self.side *= 2;
        let win = capture::window(self.seed, self.side, size);
        if u64::from(win.w) * u64::from(win.h) > MOST_CELLS && self.passes > 0 {
            return Err(if self.close {
                "Alan çok büyük: 8192 × 8192 hücreyi aşıyor. Rasteri Maskeyle kırp ile bölün."
                    .into()
            } else {
                "Çizgi çok uzun: 8192 × 8192 hücreyi aşıyor. Rasteri Maskeyle kırp ile bölün."
                    .into()
            });
        }
        self.win = win;
        self.phase = Phase::Window(vec![0; (win.w * win.h) as usize]);
        self.passes += 1;
        Ok(Some(win))
    }

    fn finish(self, affine: &[f64; 6], flip: bool, threads: usize) -> Result<Features, String> {
        let cells = self.found.ok_or("Yakalama bitmedi.")?;
        let win = self.win;
        if self.close {
            let mut labels = vec![0u32; (win.w * win.h) as usize];
            for &k in &cells {
                labels[k as usize] = 1;
            }
            let regions = Regions {
                width: win.w,
                height: win.h,
                labels,
                values: vec![f64::NAN],
            };
            let mut rings = region_rings(&regions)?.swap_remove(0);
            if !self.keep_holes {
                rings.truncate(1);
            }
            let off = |r: &Ring| -> Ring {
                r.iter()
                    .map(|&[u, v]| [u + i64::from(win.i0), v + i64::from(win.j0)])
                    .collect()
            };
            let rings: Vec<Ring> = rings.iter().map(off).collect();
            // Tag 1: the simplified rings crossed, the rings as they are.
            let (used, tag) = if self.eps > 0.0 {
                match simplify_rings(&rings, self.eps) {
                    Some(s) => (s, 0),
                    None => (rings, 1),
                }
            } else {
                (rings, 0)
            };
            let mut out = Features::new(FeatureKind::Polygons);
            let world: Vec<Vec<[f64; 2]>> = used.iter().map(|r| world_ring(r, flip)).collect();
            out.push_area(&world, affine, (f64::NAN, String::new()), tag);
            return Ok(out);
        }
        let mut mask = Mask::new(win.w as usize, win.h as usize);
        for &k in &cells {
            mask.cells[k as usize] = 1;
        }
        thin(&mut mask, threads);
        let paths = clean_paths(&mut mask, self.spur, threads);
        Ok(lines_of(&paths, (win.i0, win.j0), self.eps, affine))
    }
}

/// Rings simplified by the closed rule (§8); a ring that would fall under
/// three corners keeps its own. None when the rings then cross themselves
/// or each other: they are written as they are.
fn simplify_rings(rings: &[Ring], eps: f64) -> Option<Vec<Ring>> {
    let mut out: Vec<Ring> = Vec::with_capacity(rings.len());
    for r in rings {
        let mut pts: Vec<[f64; 2]> = r.iter().map(|&[u, v]| [u as f64, v as f64]).collect();
        pts.push(pts[0]);
        let mut kept = douglas_peucker_closed(&pts, eps);
        kept.pop();
        if kept.len() < 3 {
            out.push(r.clone());
            continue;
        }
        out.push(kept.iter().map(|&[u, v]| [u as i64, v as i64]).collect());
    }
    if out.iter().any(|r| twice_area(r) == 0) || crossing(&out) {
        return None;
    }
    Some(out)
}

/// Whether any two edges meet anywhere but where a ring's neighbouring
/// edges share their corner (and do not fold back over each other). Edges
/// go into the cells of a grid their boxes cover; pairs within a cell are tested.
fn crossing(rings: &[Ring]) -> bool {
    let edges: Vec<(usize, usize, [i64; 2], [i64; 2])> = rings
        .iter()
        .enumerate()
        .flat_map(|(ri, r)| (0..r.len()).map(move |k| (ri, k, r[k], r[(k + 1) % r.len()])))
        .collect();
    let orient = |a: [i64; 2], b: [i64; 2], c: [i64; 2]| -> i128 {
        i128::from(b[0] - a[0]) * i128::from(c[1] - a[1])
            - i128::from(b[1] - a[1]) * i128::from(c[0] - a[0])
    };
    let on = |a: [i64; 2], b: [i64; 2], c: [i64; 2]| -> bool {
        c[0] >= a[0].min(b[0])
            && c[0] <= a[0].max(b[0])
            && c[1] >= a[1].min(b[1])
            && c[1] <= a[1].max(b[1])
    };
    let meet = |a: [i64; 2], b: [i64; 2], c: [i64; 2], d: [i64; 2]| -> bool {
        let (d1, d2, d3, d4) = (
            orient(c, d, a),
            orient(c, d, b),
            orient(a, b, c),
            orient(a, b, d),
        );
        if ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0))
        {
            return true;
        }
        (d1 == 0 && on(c, d, a))
            || (d2 == 0 && on(c, d, b))
            || (d3 == 0 && on(a, b, c))
            || (d4 == 0 && on(a, b, d))
    };
    // A neighbour folds back when it turns by half a turn at their corner.
    let folds = |p: [i64; 2], q: [i64; 2], r: [i64; 2]| {
        orient(p, q, r) == 0 && (q[0] - p[0]) * (r[0] - q[0]) + (q[1] - p[1]) * (r[1] - q[1]) < 0
    };
    let test = |x: usize, y: usize| -> bool {
        let (ra, ka, a, b) = edges[x];
        let (rb, kb, c, d) = edges[y];
        if ra == rb {
            let n = rings[ra].len();
            if (ka + 1) % n == kb || (kb + 1) % n == ka {
                return ((ka + 1) % n == kb && folds(a, b, d))
                    || ((kb + 1) % n == ka && folds(c, d, b));
            }
        }
        meet(a, b, c, d)
    };
    let n = edges.len();
    if n < 64 {
        return (0..n).any(|x| (x + 1..n).any(|y| test(x, y)));
    }
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for &(_, _, a, _) in &edges {
        x0 = x0.min(a[0]);
        y0 = y0.min(a[1]);
        x1 = x1.max(a[0]);
        y1 = y1.max(a[1]);
    }
    let g = ((n as f64).sqrt().ceil() as i64).clamp(1, 1024);
    let size = ((x1 - x0).max(y1 - y0) / g + 1).max(1);
    let cols = (x1 - x0) / size + 1;
    let rows = (y1 - y0) / size + 1;
    let mut cells: Vec<Vec<u32>> = vec![Vec::new(); (cols * rows) as usize];
    for (k, &(_, _, a, b)) in edges.iter().enumerate() {
        let (ca, cb) = ((a[0].min(b[0]) - x0) / size, (a[0].max(b[0]) - x0) / size);
        let (ra, rb) = ((a[1].min(b[1]) - y0) / size, (a[1].max(b[1]) - y0) / size);
        for r in ra..=rb {
            for c in ca..=cb {
                cells[(r * cols + c) as usize].push(k as u32);
            }
        }
    }
    cells.iter().any(|list| {
        (0..list.len())
            .any(|p| (p + 1..list.len()).any(|q| test(list[p] as usize, list[q] as usize)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings() {
        let square: Ring = vec![[0, 0], [0, 2], [2, 2], [2, 0]];
        assert!(!crossing(std::slice::from_ref(&square)));
        let bow: Ring = vec![[0, 0], [2, 2], [2, 0], [0, 2]];
        assert!(crossing(&[bow]));
        // A hole touching the outline at a corner counts as a meeting.
        let hole: Ring = vec![[0, 0], [1, 1], [1, 0]];
        assert!(crossing(&[square, hole]));
    }
}

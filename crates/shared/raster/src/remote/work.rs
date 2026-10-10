//! The remote sensing runs on the operation job (docs/adr/0242): each tool's
//! grid, result and look; the blocks its passes read; its cells; its notes.
//!
//! Denetimli sınıflandırma reads the training areas' cells in a first pass
//! (only the blocks under them), Denetimsiz sınıflandırma its sample, and
//! Doğruluk analizi the reference objects' cells (its only pass); the first
//! two then write the class raster in a second pass. Every other tool writes
//! its raster in one pass.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use kentos_contracts::{RasterRender, RasterResampling, RasterSample, RasterStretch, RasterStyle};
use kentos_formats::raster::style::default_style;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::statistics::read_number;
use kentos_geometry_core::text::natural::natural_cmp;
use kentos_geometry_core::tools::point_text::js_trim;
use serde::Deserialize;

use crate::areas::{Areas, Span, union};
use crate::grid::Grid;
use crate::inputs::{Input, Mapping, Sampling, View, empty_of, mapping, place_in, sample_row};
use crate::par;
use crate::suitability::common_grid;

use super::accuracy::{Confusion, reference_of};
use super::classify::{Classifier, Method, Moments};
use super::cluster::{self, Clusters};
use super::spectral::{Index, IndexBands};
use super::{RemoteNotes, Table, cell_area, count, fixed};

/// Değişim tespiti's methods (§9).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChangeMethod {
    Difference,
    Ratio,
    Normalized,
    Classes,
}

/// Görüntü birleştirme's methods (§10).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FuseMethod {
    Brovey,
    Mean,
}

/// Görüntü birleştirme's weights as the host gives them: numbers, or a text
/// of numbers (kentos.statistics/1) apart by semicolons or spaces.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Weights {
    List(Vec<f64>),
    Text(String),
}

impl Weights {
    /// The weights; none when none are given (equal weights).
    pub fn read(&self) -> Result<Option<Vec<f64>>, String> {
        match self {
            Weights::List(v) => Ok(Some(v.clone())),
            Weights::Text(t) => {
                let mut out = Vec::new();
                for tok in t
                    .split(|c: char| c == ';' || c.is_whitespace())
                    .filter(|s| !s.is_empty())
                {
                    let d = read_number(tok).ok_or_else(|| {
                        format!("Ağırlık okunamadı: “{tok}”; sayıları noktalı virgülle ayırın.")
                    })?;
                    out.push(d.to_f64());
                }
                Ok((!out.is_empty()).then_some(out))
            }
        }
    }
}

/// A remote sensing tool's settings, as the operation job's tool names them.
pub enum RemoteTool<'a> {
    Composite {
        sampling: Sampling,
    },
    Band {
        band: u32,
    },
    Index {
        index: Index,
        bands: IndexBands,
    },
    /// Each object's class text (Sınıf alanı).
    Supervised {
        method: Method,
        texts: &'a [Option<String>],
    },
    Unsupervised {
        clusters: u32,
        iterations: u32,
    },
    Accuracy {
        band: u32,
        reference: &'a [Option<String>],
    },
    Change {
        band: u32,
        method: ChangeMethod,
    },
    Pansharpen {
        method: FuseMethod,
        weights: Option<Vec<f64>>,
        sampling: Sampling,
    },
}

/// A run as it starts: its grid, its work, its result raster (sample,
/// value bands, alpha, nodata; none for a table) and the result's look.
pub struct Started {
    pub grid: Grid,
    pub work: RemoteWork,
    pub kind: Option<(RasterSample, u32, bool, Option<f64>)>,
    pub style: Option<RasterStyle>,
}

/// What a block of a result pass met.
#[derive(Default)]
pub struct Tally {
    zero: u64,
    range: Option<(f64, f64)>,
    counts: Vec<u64>,
    inc: u64,
    dec: u64,
    same: u64,
    pairs: BTreeMap<(i64, i64), u64>,
}

struct Supervised {
    method: Method,
    names: Vec<String>,
    /// Each object's class (1…k; 0 none).
    of: Vec<u32>,
    areas: Areas,
    /// The training cells' box (columns, rows): the blocks the first pass reads.
    inside: Option<(u32, u32, u32, u32)>,
    bands: usize,
    /// Each row's moments by class, (row, first column, moments).
    parts: Vec<(u32, u32, Vec<Moments>)>,
    trained: Vec<u64>,
    model: Option<Classifier>,
    counts: Vec<u64>,
}

struct Unsupervised {
    k: usize,
    iterations: u32,
    step: u32,
    bands: usize,
    /// Each row's sample cells, (row, first column, values).
    parts: Vec<(u32, u32, Vec<f64>)>,
    fitted: Option<Clusters>,
    counts: Vec<u64>,
}

struct Accuracy {
    band: usize,
    areas: Option<Areas>,
    /// Each area object's reference value.
    area_refs: Vec<i64>,
    /// The points' cells (row, column) and reference values, sorted.
    points: Vec<(u32, u32, i64)>,
    inside: Option<(u32, u32, u32, u32)>,
    confusion: Confusion,
    notes: Option<RemoteNotes>,
}

enum Run {
    /// Bant birleştir and Bantlara ayır: each result band an input's band.
    Bands {
        sampling: Sampling,
        maps: Vec<Mapping>,
        bands: Vec<(usize, usize)>,
    },
    Index {
        index: Index,
        bands: Vec<usize>,
        zero: u64,
        range: Option<(f64, f64)>,
    },
    Supervised(Box<Supervised>),
    Unsupervised(Box<Unsupervised>),
    Accuracy(Box<Accuracy>),
    Change {
        method: ChangeMethod,
        band: usize,
        maps: [Mapping; 2],
        inc: u64,
        dec: u64,
        same: u64,
        pairs: BTreeMap<(i64, i64), u64>,
        /// The result's least and largest value (the look's stretch).
        range: Option<(f64, f64)>,
    },
    Pansharpen {
        brovey: bool,
        weights: Vec<f64>,
        sampling: Sampling,
        maps: [Mapping; 2],
    },
}

/// A remote sensing run's state.
pub struct RemoteWork {
    run: Run,
    /// The grid's cell area (m²).
    area: f64,
    /// Bant birleştir's table, the tails known at the start.
    notes: RemoteNotes,
}

fn band_of(input: &Input, band: u32) -> Result<usize, String> {
    if band == 0 || band > input.values() {
        return Err(format!(
            "Rasterin {} bandı var; {band}. bant yok.",
            input.values()
        ));
    }
    Ok(band as usize - 1)
}

fn one_raster(inputs: &[Input], tool: &str) -> Result<(), String> {
    if inputs.len() > 1 {
        return Err(format!(
            "{} raster seçili; {tool} tek raster ister.",
            inputs.len()
        ));
    }
    Ok(())
}

fn same_nodata(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => x == y || (x.is_nan() && y.is_nan()),
        _ => false,
    }
}

/// A look over a single band: `ramp` (none: grey), stretched 2–98 (8 bit as it is).
fn single_style(sample: RasterSample, ramp: Option<&str>, invert: bool) -> RasterStyle {
    RasterStyle {
        render: if ramp.is_some() {
            RasterRender::Ramp
        } else {
            RasterRender::Gray
        },
        bands: vec![1],
        stretch: if sample == RasterSample::U8 && ramp.is_none() {
            RasterStretch::None
        } else {
            RasterStretch::Percent
        },
        ramp: ramp.map(str::to_owned),
        invert,
        ..default_style(1, sample, false)
    }
}

/// A class raster's look (§2): Spektral over 0.5 … k + 0.5 (or the values' range), read by the nearest cell.
fn class_style(range: Option<(f64, f64)>) -> RasterStyle {
    let mut s = single_style(RasterSample::F32, Some("Spektral"), false);
    match range {
        Some((lo, hi)) => {
            s.stretch = RasterStretch::Manual;
            s.min = Some(lo);
            s.max = Some(hi);
        }
        None => s.stretch = RasterStretch::MinMax,
    }
    s.resampling = RasterResampling::Nearest;
    s
}

/// Bant birleştir's look (§3): colour from bands 1, 2, 3 when there are
/// three, else grey; 8 bit as it is, else stretched 2–98.
fn composite_style(sample: RasterSample, values: u32, alpha: bool) -> RasterStyle {
    let stretch = if sample == RasterSample::U8 {
        RasterStretch::None
    } else {
        RasterStretch::Percent
    };
    if values >= 3 {
        let mut bands = vec![1, 2, 3];
        if alpha {
            bands.push(values + 1);
        }
        RasterStyle {
            render: RasterRender::Rgb,
            bands,
            stretch,
            ..default_style(values, sample, false)
        }
    } else {
        RasterStyle {
            render: RasterRender::Gray,
            bands: vec![1],
            stretch,
            ..default_style(1, sample, false)
        }
    }
}

/// The classes of the objects' texts (§6): the trimmed, non-empty ones in
/// their natural order, valued 1, 2, …; each object's value (0: none).
pub fn classes_of(texts: &[Option<String>]) -> (Vec<String>, Vec<u32>) {
    let trimmed: Vec<Option<&str>> = texts
        .iter()
        .map(|t| t.as_deref().map(js_trim).filter(|t| !t.is_empty()))
        .collect();
    let mut names: Vec<String> = trimmed.iter().flatten().map(|t| (*t).to_owned()).collect();
    names.sort_by(|a, b| natural_cmp(a, b));
    names.dedup();
    let of = trimmed
        .iter()
        .map(|t| {
            t.and_then(|t| names.binary_search_by(|n| natural_cmp(n, t)).ok())
                .map_or(0, |k| k as u32 + 1)
        })
        .collect();
    (names, of)
}

/// The class raster's samples for `k` classes (§2).
fn class_sample(k: usize) -> RasterSample {
    if k <= 255 {
        RasterSample::U8
    } else {
        RasterSample::U16
    }
}

/// The least and largest value with one (none when no value has one).
fn range_of(vals: &[f64]) -> Option<(f64, f64)> {
    let mut out: Option<(f64, f64)> = None;
    for &x in vals {
        if !x.is_nan() {
            out = Some(match out {
                None => (x, x),
                Some((lo, hi)) => (lo.min(x), hi.max(x)),
            });
        }
    }
    out
}

/// Whether two column and row ranges meet.
fn meets((c0, c1, y0, y1): (u32, u32, u32, u32), inside: Option<(u32, u32, u32, u32)>) -> bool {
    inside.is_some_and(|(i0, i1, j0, j1)| c0 < i1 && i0 < c1 && y0 < j1 && j0 < y1)
}

impl RemoteWork {
    /// The run of `tool` over `inputs` (named `names`, the first's look
    /// `first_style`) and `shapes`.
    pub fn start(
        tool: RemoteTool<'_>,
        inputs: &[Input],
        names: &[String],
        first_style: Option<&RasterStyle>,
        shapes: &[Shape],
    ) -> Result<Started, String> {
        let first = &inputs[0];
        let mut notes = RemoteNotes::default();
        let (grid, run, kind, style) = match tool {
            RemoteTool::Composite { sampling } => {
                if inputs.len() < 2 {
                    return Err("Bant birleştir en az iki raster ister.".into());
                }
                let grid = common_grid(inputs)?;
                let same = inputs
                    .iter()
                    .all(|i| i.sample == first.sample && same_nodata(i.nodata, first.nodata));
                let (sample, nodata) = if same {
                    (first.sample, first.nodata)
                } else {
                    (RasterSample::F32, None)
                };
                let values: u32 = inputs.iter().map(Input::values).sum();
                let alpha_in = inputs.iter().any(|i| i.alpha);
                let (nodata, alpha) = empty_of(sample, values, alpha_in, nodata);
                let mut bands = Vec::new();
                let mut table = Table::new(&["Bant", "Kaynak", "Kaynağın bandı"]);
                for (q, input) in inputs.iter().enumerate() {
                    for b in 0..input.values() as usize {
                        bands.push((q, b));
                        table.rows.push(vec![
                            bands.len().to_string(),
                            names.get(q).cloned().unwrap_or_default(),
                            (b + 1).to_string(),
                        ]);
                    }
                }
                notes.table = Some(table);
                notes.tail = format!("{values} bant.");
                let maps = inputs.iter().map(|i| mapping(&grid, &i.affine)).collect();
                (
                    grid,
                    Run::Bands {
                        sampling,
                        maps,
                        bands,
                    },
                    Some((sample, values, alpha, nodata)),
                    Some(composite_style(sample, values, alpha)),
                )
            }
            RemoteTool::Band { band } => {
                one_raster(inputs, "Bantlara ayır")?;
                let b = band_of(first, band)?;
                let grid = first.grid();
                let (nodata, alpha) = empty_of(first.sample, 1, first.alpha, first.nodata);
                notes.tail = format!("{band}. bant.");
                (
                    grid,
                    Run::Bands {
                        sampling: Sampling::Nearest,
                        maps: vec![Mapping::Offset(0, 0)],
                        bands: vec![(0, b)],
                    },
                    Some((first.sample, 1, alpha, nodata)),
                    Some(single_style(first.sample, None, false)),
                )
            }
            RemoteTool::Index { index, bands } => {
                one_raster(inputs, "Spektral indis")?;
                let numbers = [
                    index.scale,
                    index.offset,
                    index.savi_l,
                    index.g,
                    index.c1,
                    index.c2,
                    index.evi_l,
                ];
                if numbers.iter().any(|v| !v.is_finite()) {
                    return Err("İndisin ölçeği, ötelemesi ve sabitleri sayı olmalı.".into());
                }
                let used = index
                    .kind
                    .uses()
                    .iter()
                    .map(|n| band_of(first, n.of(&bands)))
                    .collect::<Result<Vec<_>, _>>()?;
                let style = if index.kind.vegetation() {
                    single_style(RasterSample::F32, Some("Arazi"), true)
                } else {
                    single_style(RasterSample::F32, Some("Mavi-kırmızı"), index.kind.water())
                };
                (
                    first.grid(),
                    Run::Index {
                        index,
                        bands: used,
                        zero: 0,
                        range: None,
                    },
                    Some((RasterSample::F32, 1, false, Some(f64::NAN))),
                    Some(style),
                )
            }
            RemoteTool::Supervised { method, texts } => {
                one_raster(inputs, "Denetimli sınıflandırma")?;
                if texts.len() != shapes.len() {
                    return Err("Eğitim alanlarının sınıfları eksik verildi.".into());
                }
                let (classes, of) = classes_of(texts);
                let k = classes.len();
                if k == 0 {
                    return Err("Eğitim alanı yok: Sınıf alanı dolu en az bir alan seçin.".into());
                }
                if k > 65_535 {
                    return Err(format!(
                        "En çok 65.535 sınıf olur; {} sınıf var.",
                        count(k as u64)
                    ));
                }
                let grid = first.grid();
                let areas = Areas::new(&grid, shapes);
                let inside = areas.inside_box();
                (
                    grid,
                    Run::Supervised(Box::new(Supervised {
                        method,
                        names: classes,
                        of,
                        areas,
                        inside,
                        bands: first.values() as usize,
                        parts: Vec::new(),
                        trained: vec![0; k],
                        model: None,
                        counts: vec![0; k + 1],
                    })),
                    Some((class_sample(k), 1, false, Some(0.0))),
                    Some(class_style(Some((0.5, k as f64 + 0.5)))),
                )
            }
            RemoteTool::Unsupervised {
                clusters,
                iterations,
            } => {
                one_raster(inputs, "Denetimsiz sınıflandırma")?;
                if !(2..=50).contains(&clusters) {
                    return Err("Küme sayısı 2 ile 50 arasında olmalı.".into());
                }
                if !(1..=100).contains(&iterations) {
                    return Err("En çok yineleme 1 ile 100 arasında olmalı.".into());
                }
                let grid = first.grid();
                let k = clusters as usize;
                (
                    grid,
                    Run::Unsupervised(Box::new(Unsupervised {
                        k,
                        iterations,
                        step: cluster::step(u64::from(grid.width) * u64::from(grid.height)),
                        bands: first.values() as usize,
                        parts: Vec::new(),
                        fitted: None,
                        counts: vec![0; k + 1],
                    })),
                    Some((RasterSample::U8, 1, false, Some(0.0))),
                    Some(class_style(Some((0.5, k as f64 + 0.5)))),
                )
            }
            RemoteTool::Accuracy { band, reference } => {
                one_raster(inputs, "Doğruluk analizi")?;
                let b = band_of(first, band)?;
                if reference.len() != shapes.len() {
                    return Err("Referans değerleri eksik verildi.".into());
                }
                let grid = first.grid();
                let mut confusion = Confusion::default();
                let mut points = Vec::new();
                let mut area_shapes = Vec::new();
                let mut area_refs = Vec::new();
                for (s, text) in shapes.iter().zip(reference) {
                    let Some(r) = text.as_deref().and_then(reference_of) else {
                        confusion.unread += 1;
                        continue;
                    };
                    match s {
                        Shape::Point { p, parts, .. } => {
                            let places =
                                std::iter::once(*p).chain(parts.iter().flatten().map(|q| q.p));
                            for p in places {
                                let (u, v) = place_in(&grid.affine, p.x, p.y);
                                let (i, j) = (u.floor(), v.floor());
                                if i >= 0.0
                                    && j >= 0.0
                                    && i < f64::from(grid.width)
                                    && j < f64::from(grid.height)
                                {
                                    points.push((j as u32, i as u32, r));
                                } else {
                                    confusion.off += 1;
                                }
                            }
                        }
                        _ => {
                            area_shapes.push(s.clone());
                            area_refs.push(r);
                        }
                    }
                }
                points.sort_unstable();
                let areas = (!area_shapes.is_empty()).then(|| Areas::new(&grid, &area_shapes));
                let mut inside = areas.as_ref().and_then(Areas::inside_box);
                for &(j, i, _) in &points {
                    inside = Some(match inside {
                        None => (i, i + 1, j, j + 1),
                        Some((i0, i1, j0, j1)) => {
                            (i0.min(i), i1.max(i + 1), j0.min(j), j1.max(j + 1))
                        }
                    });
                }
                (
                    grid,
                    Run::Accuracy(Box::new(Accuracy {
                        band: b,
                        areas,
                        area_refs,
                        points,
                        inside,
                        confusion,
                        notes: None,
                    })),
                    None,
                    None,
                )
            }
            RemoteTool::Change { band, method } => {
                if inputs.len() != 2 {
                    return Err("Değişim tespiti iki raster ister: önceki ve sonraki.".into());
                }
                let b = band_of(&inputs[0], band)?;
                band_of(&inputs[1], band)?;
                let grid = common_grid(inputs)?;
                let maps = [
                    mapping(&grid, &inputs[0].affine),
                    mapping(&grid, &inputs[1].affine),
                ];
                let classes = method == ChangeMethod::Classes;
                (
                    grid,
                    Run::Change {
                        method,
                        band: b,
                        maps,
                        inc: 0,
                        dec: 0,
                        same: 0,
                        pairs: BTreeMap::new(),
                        range: None,
                    },
                    Some(if classes {
                        (RasterSample::I32, 1, false, Some(0.0))
                    } else {
                        (RasterSample::F32, 1, false, Some(f64::NAN))
                    }),
                    Some(if classes {
                        class_style(None)
                    } else {
                        single_style(RasterSample::F32, Some("Mavi-kırmızı"), false)
                    }),
                )
            }
            RemoteTool::Pansharpen {
                method,
                weights,
                sampling,
            } => {
                if inputs.len() != 2 {
                    return Err(
                        "Görüntü birleştirme iki raster ister: çok bantlı ve pankromatik.".into(),
                    );
                }
                let (ms, pan) = (&inputs[0], &inputs[1]);
                let n = ms.values();
                let weights = match weights.as_deref() {
                    Some(w) if w.len() != n as usize => {
                        return Err(format!(
                            "Ağırlıkların sayısı ({}) çok bantlının bant sayısı ({n}) değil.",
                            w.len()
                        ));
                    }
                    Some(w) if w.iter().any(|v| !v.is_finite()) => {
                        return Err("Ağırlıklar sayı olmalı.".into());
                    }
                    Some(w) => w.to_vec(),
                    None => vec![1.0 / f64::from(n); n as usize],
                };
                let grid = pan.grid();
                let maps = [mapping(&grid, &ms.affine), mapping(&grid, &pan.affine)];
                let (nodata, alpha) = empty_of(ms.sample, n, ms.alpha, ms.nodata);
                let mut style = first_style
                    .cloned()
                    .unwrap_or_else(|| default_style(n, ms.sample, false));
                if alpha && style.render == RasterRender::Rgb && style.bands.len() == 3 {
                    style.bands.push(n + 1);
                }
                notes.tail = format!(
                    "{}, {n} bant.",
                    if method == FuseMethod::Brovey {
                        "Brovey"
                    } else {
                        "Basit ortalama"
                    }
                );
                (
                    grid,
                    Run::Pansharpen {
                        brovey: method == FuseMethod::Brovey,
                        weights,
                        sampling,
                        maps,
                    },
                    Some((ms.sample, n, alpha, nodata)),
                    Some(style),
                )
            }
        };
        Ok(Started {
            grid,
            work: RemoteWork {
                run,
                area: cell_area(&grid.affine),
                notes,
            },
            kind,
            style,
        })
    }

    /// Whether the pass under way reads without writing (training, the sample, the reference cells).
    pub fn reading(&self) -> bool {
        match &self.run {
            Run::Supervised(s) => s.model.is_none(),
            Run::Unsupervised(u) => u.fitted.is_none(),
            Run::Accuracy(_) => true,
            _ => false,
        }
    }

    /// A table's run (Doğruluk analizi): no raster.
    pub fn report(&self) -> bool {
        matches!(self.run, Run::Accuracy(_))
    }

    /// The inputs a block (columns `c0..c1`, rows `y0..y1`) reads and how
    /// many cells round it; `planning`: the most any pass reads.
    pub fn reads(&self, rect: (u32, u32, u32, u32), planning: bool) -> Vec<(usize, i64)> {
        match &self.run {
            Run::Bands { sampling, maps, .. } => {
                (0..maps.len()).map(|k| (k, sampling.margin())).collect()
            }
            Run::Index { .. } => vec![(0, 0)],
            Run::Supervised(s) => {
                if planning || s.model.is_some() || meets(rect, s.inside) {
                    vec![(0, 0)]
                } else {
                    Vec::new()
                }
            }
            Run::Unsupervised(u) => {
                let (_, _, y0, y1) = rect;
                let sampled = (y0..y1).any(|j| j % u.step == 0);
                if planning || u.fitted.is_some() || sampled {
                    vec![(0, 0)]
                } else {
                    Vec::new()
                }
            }
            Run::Accuracy(a) => {
                if planning || meets(rect, a.inside) {
                    vec![(0, 0)]
                } else {
                    Vec::new()
                }
            }
            Run::Change { .. } => vec![(0, 0), (1, 0)],
            Run::Pansharpen { sampling, .. } => vec![(0, sampling.margin()), (1, 0)],
        }
    }

    /// A reading pass's block.
    pub fn read(
        &mut self,
        views: &[Option<View>],
        (c0, c1, y0, y1): (u32, u32, u32, u32),
        threads: usize,
    ) {
        let Some(view) = views.first().and_then(Option::as_ref) else {
            return;
        };
        let rows: Vec<u32> = (y0..y1).collect();
        match &mut self.run {
            Run::Supervised(s) => {
                let k = s.names.len();
                let bands = s.bands;
                let near = s.areas.strip(y0, y1);
                let (areas, of) = (&s.areas, &s.of);
                let made: Vec<Option<Vec<Moments>>> = par::map(threads, &rows, &|&j| {
                    let (mut spans, mut cuts) = (Vec::<Span>::new(), Vec::new());
                    areas.row(j, &near, &mut spans, &mut cuts);
                    if spans.is_empty() {
                        return None;
                    }
                    let mut ms = vec![Moments::new(bands); k];
                    let (mut x, mut d) = (vec![0.0; bands], vec![0.0; bands]);
                    let (mut mine, mut merged) = (Vec::new(), Vec::new());
                    for (c, m) in ms.iter_mut().enumerate() {
                        mine.clear();
                        mine.extend(
                            spans
                                .iter()
                                .filter(|sp| of.get(sp.object as usize) == Some(&(c as u32 + 1)))
                                .copied(),
                        );
                        union(&mine, &mut merged);
                        for &(a, z) in &merged {
                            for i in a.max(c0)..z.min(c1) {
                                let mut empty = false;
                                for (b, xb) in x.iter_mut().enumerate() {
                                    *xb = view.at(b, i64::from(i), i64::from(j));
                                    empty |= xb.is_nan();
                                }
                                if !empty {
                                    m.push(&x, &mut d);
                                }
                            }
                        }
                    }
                    Some(ms)
                });
                for (j, ms) in rows.iter().zip(made) {
                    if let Some(ms) = ms {
                        s.parts.push((*j, c0, ms));
                    }
                }
            }
            Run::Unsupervised(u) => {
                let (bands, st) = (u.bands, u.step);
                for &j in &rows {
                    if j % st != 0 {
                        continue;
                    }
                    let mut vals = Vec::new();
                    let mut i = c0.div_ceil(st) * st;
                    let mut x = vec![0.0; bands];
                    while i < c1 {
                        let mut empty = false;
                        for (b, xb) in x.iter_mut().enumerate() {
                            *xb = view.at(b, i64::from(i), i64::from(j));
                            empty |= xb.is_nan();
                        }
                        if !empty {
                            vals.extend_from_slice(&x);
                        }
                        i += st;
                    }
                    if !vals.is_empty() {
                        u.parts.push((j, c0, vals));
                    }
                }
            }
            Run::Accuracy(a) => {
                let band = a.band;
                let near = a.areas.as_ref().map(|ar| ar.strip(y0, y1));
                let (areas, refs, points) = (&a.areas, &a.area_refs, &a.points);
                let made: Vec<Confusion> = par::map(threads, &rows, &|&j| {
                    let mut conf = Confusion::default();
                    let take = |i: u32, r: i64, conf: &mut Confusion| {
                        let g = view.at(band, i64::from(i), i64::from(j));
                        if g.is_nan() {
                            conf.off += 1;
                        } else {
                            conf.add(g.trunc() as i64, r, 1);
                        }
                    };
                    if let (Some(ar), Some(near)) = (areas, &near) {
                        let (mut spans, mut cuts) = (Vec::<Span>::new(), Vec::new());
                        ar.row(j, near, &mut spans, &mut cuts);
                        for sp in &spans {
                            let r = refs[sp.object as usize];
                            for i in sp.i0.max(c0)..sp.i1.min(c1) {
                                take(i, r, &mut conf);
                            }
                        }
                    }
                    let from = points.partition_point(|&(pj, pi, _)| (pj, pi) < (j, c0));
                    for &(_, i, r) in points[from..]
                        .iter()
                        .take_while(|&&(pj, pi, _)| pj == j && pi < c1)
                    {
                        take(i, r, &mut conf);
                    }
                    conf
                });
                for c in &made {
                    a.confusion.join(c);
                }
            }
            _ => {}
        }
    }

    /// A reading pass ended: the model, the clusters or the report; whether a result pass follows.
    pub fn read_done(&mut self, threads: usize) -> Result<bool, String> {
        match &mut self.run {
            Run::Supervised(s) => {
                s.parts.sort_by_key(|p| (p.0, p.1));
                let mut ms = vec![Moments::new(s.bands); s.names.len()];
                for (_, _, row) in &s.parts {
                    for (m, r) in ms.iter_mut().zip(row) {
                        m.join(r);
                    }
                }
                s.parts = Vec::new();
                s.trained = ms.iter().map(|m| m.n).collect();
                s.model = Some(Classifier::new(s.method, &s.names, &ms)?);
                Ok(true)
            }
            Run::Unsupervised(u) => {
                u.parts.sort_by_key(|p| (p.0, p.1));
                let mut sample = Vec::new();
                for (_, _, v) in &u.parts {
                    sample.extend_from_slice(v);
                }
                u.parts = Vec::new();
                u.fitted = Some(cluster::fit(&sample, u.bands, u.k, u.iterations, threads)?);
                Ok(true)
            }
            Run::Accuracy(a) => {
                a.notes = Some(a.confusion.notes()?);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// The share of the run done when the pass under way is `f` done.
    pub fn share(&self, f: f64) -> f64 {
        match &self.run {
            Run::Supervised(_) | Run::Unsupervised(_) if self.reading() => 0.5 * f,
            Run::Supervised(_) | Run::Unsupervised(_) => 0.5 + 0.5 * f,
            _ => f,
        }
    }

    /// A result pass's block: its rows on the threads, each cell's `b`
    /// bands (alpha last) as float64s into `vals`; what it met.
    #[allow(clippy::too_many_arguments)]
    pub fn block(
        &self,
        grid: &Grid,
        inputs: &[Input],
        views: &[Option<View>],
        (c0, c1, y0, _): (u32, u32, u32, u32),
        b: usize,
        threads: usize,
        vals: &mut [f64],
    ) -> Tally {
        let bw = (c1 - c0) as usize;
        let row_len = bw * b;
        let empty = View::default();
        let view = |k: usize| views.get(k).and_then(Option::as_ref).unwrap_or(&empty);
        // A band of input `k` along result row `j`.
        let read = |k: usize, map: Mapping, band: usize, j: u32, how: Sampling, out: &mut [f64]| {
            match views.get(k).and_then(Option::as_ref) {
                Some(v) => sample_row((grid, &inputs[k].affine), map, v, band, (c0, j), how, out),
                None => out.fill(f64::NAN),
            }
        };
        let mut tally = Tally::default();
        match &self.run {
            Run::Bands {
                sampling,
                maps,
                bands,
            } => {
                let values = bands.len();
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut buf = vec![0.0; bw];
                    for (q, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + q) as u32;
                        for (o, &(k, band)) in bands.iter().enumerate() {
                            read(k, maps[k], band, j, *sampling, &mut buf);
                            for i in 0..bw {
                                row[i * b + o] = buf[i];
                            }
                        }
                        if b > values {
                            for i in 0..bw {
                                let any = (0..values).any(|o| !row[i * b + o].is_nan());
                                row[i * b + values] = if any { 255.0 } else { 0.0 };
                            }
                        }
                    }
                });
            }
            Run::Index { index, bands, .. } => {
                let zero = AtomicU64::new(0);
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut bufs = vec![vec![0.0; bw]; bands.len()];
                    let mut v = vec![0.0; bands.len()];
                    let mut zeros = 0u64;
                    for (q, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + q) as u32;
                        for (buf, &band) in bufs.iter_mut().zip(bands) {
                            read(0, Mapping::Offset(0, 0), band, j, Sampling::Nearest, buf);
                        }
                        for (i, o) in row.iter_mut().enumerate() {
                            let mut empty = false;
                            for (vb, buf) in v.iter_mut().zip(&bufs) {
                                let dn = buf[i];
                                empty |= dn.is_nan();
                                *vb = index.reflectance(dn);
                            }
                            *o = if empty {
                                f64::NAN
                            } else {
                                match index.value(&v) {
                                    Some(x) => f64::from(x as f32),
                                    None => {
                                        zeros += 1;
                                        f64::NAN
                                    }
                                }
                            };
                        }
                    }
                    zero.fetch_add(zeros, Ordering::Relaxed);
                });
                tally.zero = zero.into_inner();
                tally.range = range_of(vals);
            }
            Run::Supervised(s) => {
                let Some(model) = &s.model else {
                    return tally;
                };
                let counts: Vec<AtomicU64> =
                    (0..=s.names.len()).map(|_| AtomicU64::new(0)).collect();
                let bands = s.bands;
                let src = view(0);
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut lines = vec![vec![0.0; bw]; bands];
                    let (mut x, mut z) = (vec![0.0; bands], vec![0.0; bands]);
                    let mut mine = vec![0u64; counts.len()];
                    for (q, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + q) as u32;
                        for (band, line) in lines.iter_mut().enumerate() {
                            src.row_into(band, i64::from(c0), i64::from(j), line);
                        }
                        for (i, o) in row.iter_mut().enumerate() {
                            let mut empty = false;
                            for (xb, line) in x.iter_mut().zip(&lines) {
                                *xb = line[i];
                                empty |= xb.is_nan();
                            }
                            *o = if empty {
                                f64::NAN
                            } else {
                                let c = model.assign(&x, &mut z) + 1;
                                mine[c] += 1;
                                c as f64
                            };
                        }
                    }
                    for (t, m) in counts.iter().zip(mine) {
                        t.fetch_add(m, Ordering::Relaxed);
                    }
                });
                tally.counts = counts.into_iter().map(AtomicU64::into_inner).collect();
            }
            Run::Unsupervised(u) => {
                let Some(fitted) = &u.fitted else {
                    return tally;
                };
                let counts: Vec<AtomicU64> = (0..=u.k).map(|_| AtomicU64::new(0)).collect();
                let bands = u.bands;
                let src = view(0);
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut lines = vec![vec![0.0; bw]; bands];
                    let mut x = vec![0.0; bands];
                    let mut mine = vec![0u64; counts.len()];
                    for (q, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + q) as u32;
                        for (band, line) in lines.iter_mut().enumerate() {
                            src.row_into(band, i64::from(c0), i64::from(j), line);
                        }
                        for (i, o) in row.iter_mut().enumerate() {
                            let mut empty = false;
                            for (xb, line) in x.iter_mut().zip(&lines) {
                                *xb = line[i];
                                empty |= xb.is_nan();
                            }
                            *o = if empty {
                                f64::NAN
                            } else {
                                let c = cluster::nearest(&x, &fitted.centers, bands) + 1;
                                mine[c] += 1;
                                c as f64
                            };
                        }
                    }
                    for (t, m) in counts.iter().zip(mine) {
                        t.fetch_add(m, Ordering::Relaxed);
                    }
                });
                tally.counts = counts.into_iter().map(AtomicU64::into_inner).collect();
            }
            Run::Change {
                method, band, maps, ..
            } => {
                let met: [AtomicU64; 3] = Default::default();
                let pairs = Mutex::new(BTreeMap::<(i64, i64), u64>::new());
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let (mut before, mut after) = (vec![0.0; bw], vec![0.0; bw]);
                    let mut counted = [0u64; 3];
                    let mut mine = BTreeMap::<(i64, i64), u64>::new();
                    for (q, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + q) as u32;
                        read(0, maps[0], *band, j, Sampling::Nearest, &mut before);
                        read(1, maps[1], *band, j, Sampling::Nearest, &mut after);
                        for (i, o) in row.iter_mut().enumerate() {
                            let (a, c) = (before[i], after[i]);
                            if *method == ChangeMethod::Classes {
                                *o = if a.is_nan() || c.is_nan() || a == 0.0 || c == 0.0 {
                                    f64::NAN
                                } else {
                                    let (p, r) = (a.trunc() as i64, c.trunc() as i64);
                                    *mine.entry((p, r)).or_insert(0) += 1;
                                    (p * 1000 + r) as f64
                                };
                                continue;
                            }
                            if a.is_nan() || c.is_nan() {
                                *o = f64::NAN;
                                continue;
                            }
                            counted[if c > a {
                                0
                            } else if c < a {
                                1
                            } else {
                                2
                            }] += 1;
                            let x = match method {
                                ChangeMethod::Difference => Some(c - a),
                                ChangeMethod::Ratio => (a != 0.0).then(|| c / a),
                                _ => {
                                    let den = c + a;
                                    (den != 0.0).then(|| (c - a) / den)
                                }
                            };
                            *o = x.map_or(f64::NAN, |x| f64::from(x as f32));
                        }
                    }
                    for (t, m) in met.iter().zip(counted) {
                        t.fetch_add(m, Ordering::Relaxed);
                    }
                    if !mine.is_empty() {
                        let mut all = pairs.lock().unwrap_or_else(|e| e.into_inner());
                        for (key, n) in mine {
                            *all.entry(key).or_insert(0) += n;
                        }
                    }
                });
                let [inc, dec, same] = met.map(AtomicU64::into_inner);
                (tally.inc, tally.dec, tally.same) = (inc, dec, same);
                tally.pairs = pairs.into_inner().unwrap_or_else(|e| e.into_inner());
                if *method != ChangeMethod::Classes {
                    tally.range = range_of(vals);
                }
            }
            Run::Pansharpen {
                brovey,
                weights,
                sampling,
                maps,
            } => {
                let n = weights.len();
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut ms = vec![vec![0.0; bw]; n];
                    let mut pan = vec![0.0; bw];
                    for (q, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + q) as u32;
                        read(1, maps[1], 0, j, Sampling::Nearest, &mut pan);
                        for (band, line) in ms.iter_mut().enumerate() {
                            read(0, maps[0], band, j, *sampling, line);
                        }
                        for i in 0..bw {
                            let p = pan[i];
                            let cell = &mut row[i * b..(i + 1) * b];
                            if p.is_nan() || ms.iter().any(|l| l[i].is_nan()) {
                                cell.fill(f64::NAN);
                                continue;
                            }
                            if *brovey {
                                let mut s = 0.0;
                                for (w, l) in weights.iter().zip(&ms) {
                                    s += w * l[i];
                                }
                                for (o, l) in cell.iter_mut().zip(&ms) {
                                    *o = if s <= 0.0 { 0.0 } else { l[i] * p / s };
                                }
                            } else {
                                for (o, l) in cell.iter_mut().zip(&ms) {
                                    *o = (l[i] + p) / 2.0;
                                }
                            }
                            if b > n {
                                cell[n] = 255.0;
                            }
                        }
                    }
                });
            }
            Run::Accuracy(_) => {}
        }
        tally
    }

    /// A result pass's block met `t`.
    pub fn add(&mut self, t: Tally) {
        let join = |into: &mut Vec<u64>, from: &[u64]| {
            for (a, b) in into.iter_mut().zip(from) {
                *a += b;
            }
        };
        match &mut self.run {
            Run::Index { zero, range, .. } => {
                *zero += t.zero;
                if let Some((lo, hi)) = t.range {
                    *range = Some(match *range {
                        None => (lo, hi),
                        Some((a, b)) => (a.min(lo), b.max(hi)),
                    });
                }
            }
            Run::Supervised(s) => join(&mut s.counts, &t.counts),
            Run::Unsupervised(u) => join(&mut u.counts, &t.counts),
            Run::Change {
                inc,
                dec,
                same,
                pairs,
                range,
                ..
            } => {
                *inc += t.inc;
                *dec += t.dec;
                *same += t.same;
                for (key, n) in t.pairs {
                    *pairs.entry(key).or_insert(0) += n;
                }
                if let Some((lo, hi)) = t.range {
                    *range = Some(match *range {
                        None => (lo, hi),
                        Some((a, b)) => (a.min(lo), b.max(hi)),
                    });
                }
            }
            _ => {}
        }
    }

    /// The result's look once its values are known, when it is not the
    /// start's: Değişim tespiti's (§9) Mavi-kırmızı stretched evenly round
    /// no change (0; a ratio's 1) as far as the largest change, which is white.
    pub fn style(&self) -> Option<RasterStyle> {
        let Run::Change {
            method,
            range: Some((lo, hi)),
            ..
        } = &self.run
        else {
            return None;
        };
        let centre = if *method == ChangeMethod::Ratio {
            1.0
        } else {
            0.0
        };
        let reach = (hi - centre).max(centre - lo);
        if !(reach > 0.0 && reach.is_finite()) {
            return None;
        }
        let mut s = single_style(RasterSample::F32, Some("Mavi-kırmızı"), false);
        s.stretch = RasterStretch::Manual;
        s.min = Some(centre - reach);
        s.max = Some(centre + reach);
        Some(s)
    }

    /// What the run says once it is done.
    pub fn notes(&self) -> RemoteNotes {
        let area = self.area;
        let mut notes = self.notes.clone();
        match &self.run {
            Run::Index {
                index, zero, range, ..
            } => {
                let name = index.kind.name();
                notes.tail = match range {
                    Some((lo, hi)) => format!("{name} {} … {}.", fixed(*lo, 4), fixed(*hi, 4)),
                    None => format!("{name}: hiçbir hücrede değer yok."),
                };
                if *zero > 0 {
                    notes.warnings.push(format!(
                        "{} hücrede bölen sıfır; boş bırakıldı.",
                        count(*zero)
                    ));
                }
            }
            Run::Supervised(s) => {
                let mut table = Table::new(&[
                    "Sınıf",
                    "Değer",
                    "Eğitim hücresi",
                    "Hücre sayısı",
                    "Alan (m²)",
                ]);
                for (c, name) in s.names.iter().enumerate() {
                    let n = s.counts[c + 1];
                    table.rows.push(vec![
                        name.clone(),
                        (c + 1).to_string(),
                        s.trained[c].to_string(),
                        n.to_string(),
                        fixed(n as f64 * area, 2),
                    ]);
                }
                notes.table = Some(table);
                notes.tail = format!(
                    "{} sınıf, {} eğitim hücresi.",
                    s.names.len(),
                    count(s.trained.iter().sum())
                );
            }
            Run::Unsupervised(u) => {
                let Some(fit) = &u.fitted else {
                    return notes;
                };
                let mut columns =
                    vec!["Küme".to_owned(), "Hücre sayısı".into(), "Alan (m²)".into()];
                columns.extend((1..=u.bands).map(|b| format!("Merkez {b}")));
                let mut table = Table {
                    columns,
                    rows: Vec::new(),
                };
                for c in 0..u.k {
                    let n = u.counts[c + 1];
                    let mut row = vec![
                        (c + 1).to_string(),
                        n.to_string(),
                        fixed(n as f64 * area, 2),
                    ];
                    row.extend(
                        fit.centers[c * u.bands..(c + 1) * u.bands]
                            .iter()
                            .map(|v| fixed(*v, 3)),
                    );
                    table.rows.push(row);
                }
                notes.table = Some(table);
                notes.tail = format!(
                    "{} küme, {} yineleme ({} örnek hücre).",
                    u.k,
                    fit.rounds,
                    count(fit.samples as u64)
                );
            }
            Run::Accuracy(a) => {
                if let Some(n) = &a.notes {
                    notes = n.clone();
                }
            }
            Run::Change {
                method,
                inc,
                dec,
                same,
                pairs,
                ..
            } => {
                if *method != ChangeMethod::Classes {
                    notes.tail = format!(
                        "Artan {}, azalan {}, değişmeyen {} hücre.",
                        count(*inc),
                        count(*dec),
                        count(*same)
                    );
                } else {
                    let mut froms: Vec<i64> = pairs.keys().map(|k| k.0).collect();
                    let mut tos: Vec<i64> = pairs.keys().map(|k| k.1).collect();
                    froms.dedup();
                    tos.sort_unstable();
                    tos.dedup();
                    let at = |p: i64, q: i64| pairs.get(&(p, q)).copied().unwrap_or(0);
                    let mut columns = vec!["Önceki \\ Sonraki".to_owned()];
                    columns.extend(tos.iter().map(i64::to_string));
                    columns.push("Toplam".into());
                    let mut table = Table {
                        columns,
                        rows: Vec::new(),
                    };
                    for &p in &froms {
                        let mut row = vec![p.to_string()];
                        row.extend(tos.iter().map(|&q| at(p, q).to_string()));
                        row.push(tos.iter().map(|&q| at(p, q)).sum::<u64>().to_string());
                        table.rows.push(row);
                    }
                    let total: u64 = pairs.values().sum();
                    let mut last = vec!["Toplam".to_owned()];
                    last.extend(
                        tos.iter()
                            .map(|&q| froms.iter().map(|&p| at(p, q)).sum::<u64>().to_string()),
                    );
                    last.push(total.to_string());
                    table.rows.push(last);
                    let changed: u64 = pairs
                        .iter()
                        .filter(|(k, _)| k.0 != k.1)
                        .map(|(_, n)| n)
                        .sum();
                    notes.table = Some(table);
                    notes.tail = if total > 0 {
                        format!(
                            "Değişen hücre {} / {} (%{}).",
                            count(changed),
                            count(total),
                            fixed((changed * 100) as f64 / total as f64, 2)
                        )
                    } else {
                        "Değerli ortak hücre yok.".into()
                    };
                }
            }
            Run::Bands { .. } | Run::Pansharpen { .. } => {}
        }
        notes
    }
}

//! A raster made from points or lines (docs/adr/0232): the host's settings
//! and objects in, the grid worked out a strip of 256 rows at a time on the
//! job's threads and written as ADR 0231's tiled GeoTIFF; then, when asked,
//! every point's cross-validation in pieces. Notes for the run's summary
//! (joined points, unread values, empty cells, the radius and variogram
//! taken) come with the result.

use kentos_contracts::{RasterRender, RasterResampling, RasterSample, RasterStretch, RasterStyle};
use kentos_formats::raster::TILE;
use kentos_formats::raster::style::default_style;
use kentos_formats::raster::write::Geo;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::crs::System;
use kentos_geometry_core::entity::Shape;
use serde::Deserialize;

use crate::density::{Kernel, KernelDensity, LineDensity, Stamps, silverman};
use crate::grid::Grid;
use crate::interp::kriging::{self, Model, Variogram};
use crate::interp::spline::{Basis, Kind};
use crate::interp::{MOST_NEIGHBOURS, Method, Prepared, Scratch};
use crate::out::{Out, OutSpec, Rows};
use crate::par;
use crate::points::{Lines, Source, bounds_of, gather, gather_lines, gather_weighted};
use crate::rasterize::{Burn, BurnSample, Objects, Overlap};

/// What the host asks for.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PointSpec {
    pub tool: PointTool,
    /// The cell size; 0: chosen from the box (§3).
    #[serde(default)]
    pub cell: f64,
    /// Another raster's grid; none: the input's box.
    #[serde(default)]
    pub grid: Option<GridOf>,
    /// The result's EPSG code.
    #[serde(default)]
    pub epsg: Option<u32>,
    /// The project's coordinate system as the geometry core reads it (whether
    /// the result's place is in degrees); none for a local project.
    #[serde(default)]
    pub system: Option<serde_json::Value>,
    /// Cross-validation after the grid (the interpolations).
    #[serde(default)]
    pub cross: bool,
}

/// A raster's grid as the host gives it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridOf {
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SplineName {
    Regularized,
    Tension,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModelName {
    Spherical,
    Exponential,
    Gaussian,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KernelName {
    Quartic,
    Triangular,
    Uniform,
    Epanechnikov,
    Triweight,
}

/// A density's unit: points (or a length) per this area.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DensityUnit {
    SquareKilometre,
    Hectare,
    Decare,
    SquareMetre,
    KilometrePerSquareKilometre,
    MetrePerHectare,
    MetrePerSquareMetre,
}

impl DensityUnit {
    fn scale(self) -> f64 {
        match self {
            DensityUnit::SquareKilometre => 1e6,
            DensityUnit::Hectare => 1e4,
            DensityUnit::Decare => 1e3,
            DensityUnit::SquareMetre => 1.0,
            DensityUnit::KilometrePerSquareKilometre => 1e3,
            DensityUnit::MetrePerHectare => 1e4,
            DensityUnit::MetrePerSquareMetre => 1.0,
        }
    }
}

/// Kriging's variogram: fitted or given.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(
    tag = "fit",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum VariogramSpec {
    Auto { lags: u32 },
    Manual { nugget: f64, sill: f64, range: f64 },
}

/// The tools of İnterpolasyon and Yoğunluk (§1).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PointTool {
    Idw {
        power: f64,
        points: u32,
        #[serde(default)]
        radius: f64,
        #[serde(default = "one")]
        min_points: u32,
    },
    NaturalNeighbor,
    Tin,
    Spline {
        spline: SplineName,
        weight: f64,
        points: u32,
    },
    Kriging {
        model: ModelName,
        variogram: VariogramSpec,
        points: u32,
        #[serde(default)]
        radius: f64,
        #[serde(default)]
        error: bool,
    },
    Kernel {
        #[serde(default)]
        radius: f64,
        kernel: KernelName,
        unit: DensityUnit,
    },
    LineDensity {
        #[serde(default)]
        radius: f64,
        unit: DensityUnit,
    },
    /// Rasterleştir (docs/adr/0234 §3): the objects' constant value, or
    /// their field's when the host gives the texts.
    Rasterize {
        value: f64,
        overlap: Overlap,
        sample: BurnSample,
    },
    /// Uzaklık yüzeyi from objects (docs/adr/0236 §3): the largest distance
    /// (m, 0: none); the margin (m) round the objects' box when the grid is theirs.
    Distance {
        #[serde(default)]
        max: f64,
        result: crate::ops::DistanceResult,
        #[serde(default)]
        margin: f64,
    },
}

fn one() -> u32 {
    1
}

/// The objects: points' sources with each one's value or weight text (none: elevations, weight 1), or lines.
pub enum PointInput {
    Sources {
        sources: Vec<Source>,
        values: Option<Vec<Option<String>>>,
    },
    Lines {
        shapes: Vec<Shape>,
        weights: Option<Vec<Option<String>>>,
    },
}

/// What the run met, for its summary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notes {
    /// The points (distinct places) or edges taken.
    pub taken: usize,
    pub merged: usize,
    pub unread: usize,
    pub no_elevation: usize,
    /// Cells left without a value.
    pub empty: u64,
    /// A density's radius.
    pub radius: Option<f64>,
    /// Kriging's variogram (fitted or given).
    pub variogram: Option<Variogram>,
    /// Rasterleştir: objects without a cell on the grid.
    pub outside: usize,
}

/// One point's cross-validation: its place among the points, the object it came from, its place and values.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossRow {
    pub point: u32,
    pub object: u32,
    pub x: f64,
    pub y: f64,
    pub measured: f64,
    pub predicted: Option<f64>,
    pub error: Option<f64>,
}

/// The cross-validation's sums (§12).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CrossSummary {
    pub count: usize,
    pub missing: usize,
    pub mean: f64,
    pub rmse: f64,
    pub mae: f64,
    /// Kriging: the standardized differences' mean and root mean square.
    pub std_mean: Option<f64>,
    pub std_rmse: Option<f64>,
}

impl CrossSummary {
    pub fn of(rows: &[CrossRow]) -> CrossSummary {
        let (mut n, mut sum, mut sq, mut abs) = (0usize, 0.0, 0.0, 0.0);
        let (mut sn, mut ssum, mut ssq) = (0usize, 0.0, 0.0);
        for r in rows {
            let Some(p) = r.predicted else {
                continue;
            };
            let d = p - r.measured;
            n += 1;
            sum += d;
            sq += d * d;
            abs += d.abs();
            if let Some(e) = r.error
                && e > 0.0
            {
                let z = d / e;
                sn += 1;
                ssum += z;
                ssq += z * z;
            }
        }
        let k = n as f64;
        CrossSummary {
            count: n,
            missing: rows.len() - n,
            mean: if n > 0 { sum / k } else { f64::NAN },
            rmse: if n > 0 { (sq / k).sqrt() } else { f64::NAN },
            mae: if n > 0 { abs / k } else { f64::NAN },
            std_mean: (sn > 0).then(|| ssum / sn as f64),
            std_rmse: (sn > 0).then(|| (ssq / sn as f64).sqrt()),
        }
    }
}

enum Work {
    Interp(Box<Prepared>),
    Kernel(Box<KernelDensity>),
    Lines(Box<LineDensity>),
    /// Rasterleştir's objects until the grid is known, then their burn.
    Objects(Box<(Objects, Overlap, BurnSample)>),
    Burn(Box<Burn>),
    /// Uzaklık yüzeyi's objects until the grid is known (the largest
    /// distance, the nearest source's number asked), then its run.
    NearObjects(Box<(Objects, f64, bool)>),
    Near(Box<crate::distance::Near>),
}

/// The run's state.
pub struct PointJob {
    grid: Grid,
    work: Work,
    out: Out,
    next: u32,
    threads: usize,
    bands: u32,
    cross: bool,
    rows: Vec<CrossRow>,
    cross_next: u32,
    notes: Notes,
    ramp: &'static str,
    zero_clear: bool,
}

impl std::fmt::Debug for PointJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PointJob")
            .field("next", &self.next)
            .finish()
    }
}

/// How the run ends: the GeoTIFF's directories to append and the header to write over its start.
#[derive(Debug)]
pub struct PointFinished {
    pub tail: Vec<u8>,
    pub header: Vec<u8>,
    pub rows: Vec<CrossRow>,
    pub notes: Notes,
}

/// Points of cross-validation a step works out.
const CROSS_STEP: u32 = 4_096;

fn neighbours(n: u32, least: u32, what: &str) -> Result<usize, String> {
    if n < least || n as usize > MOST_NEIGHBOURS {
        return Err(format!(
            "{what} {least} ile {MOST_NEIGHBOURS} arasında olmalı; {n} verildi."
        ));
    }
    Ok(n as usize)
}

fn finite(v: f64, what: &str) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{what} bir sayı olmalı."))
    }
}

impl PointJob {
    /// A run of `spec` over `input`, its rows on `threads`; the header to write first.
    pub fn new(
        input: PointInput,
        spec: &PointSpec,
        threads: usize,
    ) -> Result<(PointJob, Vec<u8>), String> {
        let mut notes = Notes::default();
        let by_field = matches!(
            &input,
            PointInput::Sources {
                values: Some(_),
                ..
            }
        );
        let (work, box_of, ramp, zero_clear, bands) = match (&spec.tool, input) {
            (
                PointTool::Kernel {
                    radius,
                    kernel,
                    unit,
                },
                PointInput::Sources { sources, values },
            ) => {
                let pts = gather_weighted(&sources, values.as_deref())?;
                notes.unread = pts.unread;
                notes.taken = pts.xy.len();
                if pts.xy.is_empty() {
                    return Err("Yoğunluk için nokta yok: Noktalar'da nokta nesnesi seçin.".into());
                }
                let r = if *radius > 0.0 {
                    finite(*radius, "Yarıçap")?
                } else {
                    silverman(&pts).ok_or("Yarıçap kendiliğinden bulunamadı (noktalar tek yerde ya da ağırlıklar 0): Yarıçap'ı yazın.")?
                };
                notes.radius = Some(r);
                let b = bounds_of(pts.xy.iter().copied()).ok_or("Nokta yok.")?;
                let kernel = match kernel {
                    KernelName::Quartic => Kernel::Quartic,
                    KernelName::Triangular => Kernel::Triangular,
                    KernelName::Uniform => Kernel::Uniform,
                    KernelName::Epanechnikov => Kernel::Epanechnikov,
                    KernelName::Triweight => Kernel::Triweight,
                };
                let work = Work::Kernel(Box::new(KernelDensity::new(pts, r, kernel, unit.scale())));
                (
                    work,
                    [b[0] - r, b[1] - r, b[2] + r, b[3] + r],
                    "Sıcaklık",
                    true,
                    1,
                )
            }
            (PointTool::LineDensity { radius, unit }, PointInput::Lines { shapes, weights }) => {
                let lines: Lines = gather_lines(&shapes, weights.as_deref())?;
                notes.unread = lines.unread;
                notes.taken = lines.edges.len();
                if lines.edges.is_empty() {
                    return Err(
                        "Yoğunluk için çizgi yok: Çizgiler'de çizgi ya da alan seçin.".into(),
                    );
                }
                let b = lines
                    .edges
                    .iter()
                    .map(crate::index::edge_box)
                    .fold(None, |acc: Option<[f64; 4]>, e| {
                        Some(match acc {
                            None => e,
                            Some(a) => [
                                a[0].min(e[0]),
                                a[1].min(e[1]),
                                a[2].max(e[2]),
                                a[3].max(e[3]),
                            ],
                        })
                    })
                    .ok_or("Çizgi yok.")?;
                let r = if *radius > 0.0 {
                    finite(*radius, "Yarıçap")?
                } else {
                    let (w, h) = (b[2] - b[0], b[3] - b[1]);
                    let short = if w > 0.0 && h > 0.0 {
                        w.min(h)
                    } else {
                        w.max(h)
                    };
                    if !(short > 0.0) {
                        return Err("Çizgilerin kutusu boş: Yarıçap'ı yazın.".into());
                    }
                    short / 30.0
                };
                notes.radius = Some(r);
                let work = Work::Lines(Box::new(LineDensity::new(lines, r, unit.scale())));
                (
                    work,
                    [b[0] - r, b[1] - r, b[2] + r, b[3] + r],
                    "Sıcaklık",
                    true,
                    1,
                )
            }
            (tool, PointInput::Sources { sources, values }) => {
                let pts = gather(&sources, values.as_deref())?;
                notes.merged = pts.merged;
                notes.unread = pts.unread;
                notes.no_elevation = pts.no_elevation;
                notes.taken = pts.len();
                if pts.is_empty() {
                    return Err(if by_field {
                        "Değeri okunan nokta yok: Değer alanındaki değerler sayı değil.".into()
                    } else {
                        "Kotu olan nokta yok: köşelerin kotu yok. Kotlu nokta seçin ya da Değer alanı'nı seçin.".into()
                    });
                }
                let method = match tool {
                    PointTool::Idw {
                        power,
                        points,
                        radius,
                        min_points,
                    } => {
                        let power = finite(*power, "Üs")?;
                        if !(0.1..=10.0).contains(&power) {
                            return Err("Üs 0,1 ile 10 arasında olmalı.".into());
                        }
                        let k = neighbours(*points, 1, "Nokta sayısı")?;
                        let min = neighbours(*min_points, 1, "En az nokta")?;
                        if min > k {
                            return Err("En az nokta, Nokta sayısı'ndan büyük olamaz.".into());
                        }
                        Method::Idw {
                            power,
                            k,
                            radius: finite(*radius, "Arama yarıçapı")?.max(0.0),
                            min,
                        }
                    }
                    PointTool::NaturalNeighbor => Method::Natural,
                    PointTool::Tin => Method::Tin,
                    PointTool::Spline {
                        spline,
                        weight,
                        points,
                    } => Method::Spline {
                        basis: Basis::new(
                            match spline {
                                SplineName::Regularized => Kind::Regularized,
                                SplineName::Tension => Kind::Tension,
                            },
                            finite(*weight, "Ağırlık")?,
                        )?,
                        k: neighbours(*points, 3, "Nokta sayısı")?,
                    },
                    PointTool::Kriging {
                        model,
                        variogram,
                        points,
                        radius,
                        ..
                    } => {
                        let model = match model {
                            ModelName::Spherical => Model::Spherical,
                            ModelName::Exponential => Model::Exponential,
                            ModelName::Gaussian => Model::Gaussian,
                        };
                        let g = match variogram {
                            VariogramSpec::Auto { lags } => {
                                if !(3..=100).contains(lags) {
                                    return Err("Aralık sayısı 3 ile 100 arasında olmalı.".into());
                                }
                                kriging::fit(&pts.xy, &pts.v, model, *lags as usize)?
                            }
                            VariogramSpec::Manual {
                                nugget,
                                sill,
                                range,
                            } => {
                                let g = Variogram {
                                    model,
                                    nugget: *nugget,
                                    sill: *sill,
                                    range: *range,
                                };
                                if let Some(p) = g.problem() {
                                    return Err(p);
                                }
                                g
                            }
                        };
                        notes.variogram = Some(g);
                        Method::Kriging {
                            variogram: g,
                            k: neighbours(*points, 1, "Nokta sayısı")?,
                            radius: finite(*radius, "Arama yarıçapı")?.max(0.0),
                        }
                    }
                    PointTool::Kernel { .. }
                    | PointTool::LineDensity { .. }
                    | PointTool::Rasterize { .. }
                    | PointTool::Distance { .. } => {
                        return Err("Yoğunluğun girdisi bu araca uymuyor.".into());
                    }
                };
                let b = pts.bounds().ok_or("Nokta yok.")?;
                let bands = match tool {
                    PointTool::Kriging { error: true, .. } => 2,
                    _ => 1,
                };
                let prepared = Prepared::new(pts, method)?;
                (
                    Work::Interp(Box::new(prepared)),
                    b,
                    if by_field { "Viridis" } else { "Arazi" },
                    false,
                    bands,
                )
            }
            (
                PointTool::Rasterize {
                    value,
                    overlap,
                    sample,
                },
                PointInput::Lines { shapes, weights },
            ) => {
                let objects = Objects::new(shapes, weights.as_deref(), *value)?;
                notes.unread = objects.unread;
                notes.taken = objects.len();
                if objects.is_empty() {
                    return Err(if objects.unread > 0 {
                        "Değeri okunan nesne yok: Değer alanındaki değerler sayı değil.".into()
                    } else {
                        "Rasterleştirilecek nesne yok: Nesneler'de alan, çizgi ya da nokta seçin."
                            .into()
                    });
                }
                let b = objects.bounds().ok_or("Nesnelerin yeri okunamadı.")?;
                (
                    Work::Objects(Box::new((objects, *overlap, *sample))),
                    b,
                    "Viridis",
                    false,
                    1,
                )
            }
            (
                PointTool::Distance {
                    max,
                    result,
                    margin,
                },
                PointInput::Lines { shapes, .. },
            ) => {
                let objects = Objects::numbered(shapes);
                notes.taken = objects.len();
                if objects.is_empty() {
                    return Err(
                        "Kaynak nesne yok: Kaynaklar'da nokta, çizgi ya da alan seçin.".into(),
                    );
                }
                if !(margin.is_finite() && *margin >= 0.0) {
                    return Err("Kenar payı 0 ya da artı bir uzunluk olmalı.".into());
                }
                if !(max.is_finite() && *max >= 0.0) {
                    return Err("En büyük uzaklık 0 ya da artı olmalı.".into());
                }
                let b = objects.bounds().ok_or("Kaynakların yeri okunamadı.")?;
                let allocation = *result == crate::ops::DistanceResult::Allocation;
                (
                    Work::NearObjects(Box::new((objects, *max, allocation))),
                    [b[0] - margin, b[1] - margin, b[2] + margin, b[3] + margin],
                    if allocation { "Spektral" } else { "Viridis" },
                    false,
                    1,
                )
            }
            (_, PointInput::Lines { .. }) => {
                return Err("Çizgiler yalnız Çizgi yoğunluğu'na ve Rasterleştir'e girer.".into());
            }
        };
        let grid = match &spec.grid {
            Some(g) => Grid::of(g.affine, g.width, g.height)?,
            None => Grid::of_box(box_of, spec.cell)?,
        };
        let geographic_system = match &spec.system {
            Some(v) => Json::parse(&v.to_string())
                .and_then(|j| System::from_json(&j))
                .map_err(|e| format!("Projenin koordinat sistemi okunamadı: {e}"))?
                .is_geographic(),
            None => false,
        };
        let work = match work {
            Work::Objects(o) => {
                let (objects, overlap, sample) = *o;
                Work::Burn(Box::new(Burn::new(objects, &grid, overlap, sample)?))
            }
            Work::NearObjects(o) => {
                let (objects, max, allocation) = *o;
                Work::Near(Box::new(crate::distance::Near::new(
                    objects,
                    grid,
                    geographic_system,
                    max,
                    allocation,
                )?))
            }
            w => w,
        };
        let (sample, nodata) = match &work {
            Work::Burn(b) => (b.sample(), b.nodata()),
            _ => (RasterSample::F32, f64::NAN),
        };
        let cross = spec.cross && matches!(work, Work::Interp(_));
        let geographic = geographic_system;
        let (out, header) = Out::new(
            OutSpec {
                width: grid.width,
                height: grid.height,
                bands,
                sample,
                alpha: false,
                nodata: Some(nodata),
                geo: Geo {
                    affine: grid.affine,
                    epsg: spec.epsg,
                    geographic,
                },
            },
            threads,
        )?;
        Ok((
            PointJob {
                grid,
                work,
                out,
                next: 0,
                threads: threads.max(1),
                bands,
                cross,
                rows: Vec::new(),
                cross_next: 0,
                notes,
                ramp,
                zero_clear,
            },
            header,
        ))
    }

    /// The grid worked out.
    pub fn grid(&self) -> Grid {
        self.grid
    }

    /// The result's bands (2: Kriging's prediction and standard error).
    pub fn bands(&self) -> u32 {
        self.bands
    }

    /// The result's samples: 32-bit floats, Rasterleştir's its own.
    pub fn sample(&self) -> RasterSample {
        match &self.work {
            Work::Burn(b) => b.sample(),
            _ => RasterSample::F32,
        }
    }

    pub fn notes(&self) -> &Notes {
        &self.notes
    }

    /// The value at `q` as a cell gets it, and Kriging's standard error (NaN where none).
    pub fn at(&self, q: kentos_geometry_core::vec2::Vec2) -> (f64, f64) {
        match &self.work {
            Work::Interp(p) => p.at(q, &mut Scratch::default()),
            Work::Kernel(d) => (d.at(q), f64::NAN),
            Work::Lines(d) => (d.at(q, &mut d.stamps()), f64::NAN),
            // A burn's values and the distances come row by row.
            Work::Objects(_) | Work::Burn(_) | Work::NearObjects(_) | Work::Near(_) => {
                (f64::NAN, f64::NAN)
            }
        }
    }

    /// The interpolation's points (for the cross-validation's names); none for a density.
    pub fn points(&self) -> Option<&crate::points::Points> {
        match &self.work {
            Work::Interp(p) => Some(&p.points),
            _ => None,
        }
    }

    /// Band `band`'s look (docs/adr/0232 §13): one band in the tool's ramp,
    /// least to most; a density's zeros clear; Rasterleştir's values (most
    /// often numbers or classes) shown cell by cell (docs/adr/0234 §3).
    pub fn style(&self, band: u32) -> RasterStyle {
        let base = default_style(1, RasterSample::F32, false);
        RasterStyle {
            render: RasterRender::Ramp,
            bands: vec![band.max(1)],
            stretch: RasterStretch::MinMax,
            ramp: Some(if band == 2 { "Viridis" } else { self.ramp }.to_owned()),
            invert: false,
            nodata: if self.zero_clear {
                Some(0.0)
            } else {
                base.nodata
            },
            resampling: if matches!(&self.work, Work::Burn(_))
                || matches!(&self.work, Work::Near(n) if n.allocation())
            {
                RasterResampling::Nearest
            } else {
                base.resampling
            },
            ..base
        }
    }

    pub fn done(&self) -> bool {
        self.next >= self.grid.height && (!self.cross || self.cross_points() <= self.cross_next)
    }

    fn cross_points(&self) -> u32 {
        match &self.work {
            Work::Interp(p) if self.cross => p.points.len() as u32,
            _ => 0,
        }
    }

    /// The share done, 0..1 (cross-validation the last fifth when asked).
    pub fn share(&self) -> f64 {
        let g = f64::from(self.next.min(self.grid.height)) / f64::from(self.grid.height.max(1));
        if !self.cross {
            return g;
        }
        let n = self.cross_points().max(1);
        0.8 * g + 0.2 * f64::from(self.cross_next.min(n)) / f64::from(n)
    }

    /// Works out the next strip (or the next piece of cross-validation); the bytes to append.
    pub fn step(&mut self) -> Result<Vec<u8>, String> {
        if self.next < self.grid.height {
            return self.strip();
        }
        if self.cross && self.cross_next < self.cross_points() {
            self.cross_piece();
        }
        Ok(Vec::new())
    }

    fn strip(&mut self) -> Result<Vec<u8>, String> {
        let y0 = self.next;
        let n = TILE.min(self.grid.height - y0);
        if let Work::Near(d) = &mut self.work {
            d.compute(self.threads)?;
            let vals = d.rows(y0, n);
            let rows: Vec<f32> = vals.iter().map(|&v| v as f32).collect();
            self.notes.empty += rows.iter().filter(|v| v.is_nan()).count() as u64;
            let bytes = self.out.push(Rows::F32(&rows), n)?;
            self.next += n;
            return Ok(bytes);
        }
        if let Work::Burn(b) = &mut self.work {
            let (samples, empty) = b.strip(y0, n, self.threads)?;
            self.notes.empty += empty;
            let bytes = self.out.push(Rows::Any(&samples), n)?;
            self.next += n;
            return Ok(bytes);
        }
        let width = self.grid.width as usize;
        let bands = self.bands as usize;
        let row = width * bands;
        let grid = self.grid;
        let mut rows = vec![0f32; n as usize * row];
        match &self.work {
            Work::Interp(p) => {
                let p = &**p;
                par::rows(self.threads, &mut rows, row, &|first, chunk: &mut [f32]| {
                    let mut s = Scratch::default();
                    for (k, out) in chunk.chunks_mut(row).enumerate() {
                        let j = y0 + (first + k) as u32;
                        for i in 0..width {
                            let (v, e) = p.at(grid.center(i as u32, j), &mut s);
                            out[i * bands] = v as f32;
                            if bands == 2 {
                                out[i * bands + 1] = e as f32;
                            }
                        }
                    }
                });
            }
            Work::Kernel(d) => {
                let d = &**d;
                par::rows(self.threads, &mut rows, row, &|first, chunk: &mut [f32]| {
                    for (k, out) in chunk.chunks_mut(row).enumerate() {
                        let j = y0 + (first + k) as u32;
                        for (i, o) in out.iter_mut().enumerate() {
                            *o = d.at(grid.center(i as u32, j)) as f32;
                        }
                    }
                });
            }
            Work::Objects(_) | Work::Burn(_) | Work::NearObjects(_) | Work::Near(_) => {
                return Err("Rasterleştirmenin nesneleri ızgaraya yerleşmedi.".into());
            }
            Work::Lines(d) => {
                let d = &**d;
                par::rows(self.threads, &mut rows, row, &|first, chunk: &mut [f32]| {
                    let mut s: Stamps = d.stamps();
                    for (k, out) in chunk.chunks_mut(row).enumerate() {
                        let j = y0 + (first + k) as u32;
                        for (i, o) in out.iter_mut().enumerate() {
                            *o = d.at(grid.center(i as u32, j), &mut s) as f32;
                        }
                    }
                });
            }
        }
        self.notes.empty += rows.iter().step_by(bands).filter(|v| v.is_nan()).count() as u64;
        let bytes = self.out.push(Rows::F32(&rows), n)?;
        self.next += n;
        Ok(bytes)
    }

    fn cross_piece(&mut self) {
        let Work::Interp(p) = &self.work else {
            return;
        };
        let p = &**p;
        let start = self.cross_next;
        let end = (start + CROSS_STEP).min(p.points.len() as u32);
        // A piece a thread, each with its own scratch (its caches are this run's).
        let per = (end - start).div_ceil(self.threads as u32).max(1);
        let pieces: Vec<(u32, u32)> = (start..end)
            .step_by(per as usize)
            .map(|a| (a, (a + per).min(end)))
            .collect();
        let rows = par::map(self.threads, &pieces, &|&(a, b)| {
            let mut s = Scratch::default();
            (a..b)
                .map(|i| {
                    let got = p.leave_out(i, &mut s);
                    let q = p.points.xy[i as usize];
                    CrossRow {
                        point: i,
                        object: p.points.object[i as usize],
                        x: q.x,
                        y: q.y,
                        measured: p.points.v[i as usize],
                        predicted: got.map(|g| g.0),
                        error: got.and_then(|g| (!g.1.is_nan()).then_some(g.1)),
                    }
                })
                .collect::<Vec<CrossRow>>()
        });
        self.rows.extend(rows.into_iter().flatten());
        self.cross_next = end;
    }

    /// The result: the GeoTIFF's last pieces, the cross-validation's rows and the notes.
    pub fn finish(self) -> Result<PointFinished, String> {
        if !self.done() {
            return Err("Çözümleme bitmeden bırakıldı.".into());
        }
        let mut notes = self.notes;
        if let Work::Burn(b) = &self.work {
            notes.outside = b.outside();
        }
        if let Work::Near(d) = &self.work {
            notes.outside = d.outside as usize;
        }
        let (tail, header) = self.out.finish()?;
        Ok(PointFinished {
            tail,
            header,
            rows: self.rows,
            notes,
        })
    }
}

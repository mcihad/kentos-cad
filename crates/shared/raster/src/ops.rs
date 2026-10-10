//! Raster işlemleri's runs (docs/adr/0233): Raster hesaplayıcı, Yeniden
//! sınıflandır, Maskeyle kırp, Mozaik, Yeniden örnekle, Bölgesel istatistik,
//! Histogram, Komşuluk istatistiği and Hücre istatistiği over one or more
//! input rasters (and areas for the mask and the zones); the vectorizing,
//! hydrology, distance (docs/adr/0234–0236) and suitability tools
//! (docs/adr/0237) on the same steps.
//!
//! The host opens each input (the formats core's reader) and, until the
//! run is done, gives the blocks [`OpsJob::needs`] names (each with its
//! input), then appends what [`OpsJob::step`] gives; [`OpsJob::finish`]
//! gives the GeoTIFF's directories and header, or the zones' figures, or
//! the histogram. A step is a block of the result: a 256-row strip's columns,
//! at most 4096 of them (fewer when the inputs are many or turned, so that
//! what a step reads stays within bounds); its rows are worked out on the
//! job's threads and every target gives the same bytes.

use std::collections::BTreeMap;

use kentos_contracts::{RasterRender, RasterResampling, RasterSample, RasterStretch, RasterStyle};
use kentos_formats::raster::TILE;
use kentos_formats::raster::samples::{Samples, stored};
use kentos_formats::raster::source::{BlockNeed, Layout, Put, decode_block};
use kentos_formats::raster::style::default_style;
use kentos_formats::raster::write::Geo;
use kentos_geometry_core::entity::Shape;
use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::areas::{Areas, Span, union};
use crate::calc::{Calc, Empty};
use crate::distance::network::Neighbours;
use crate::distance::{DistanceNotes, DistanceTool, DistanceWork, Threshold};
use crate::focal::{Focal, Region, Window};
use crate::grid::Grid;
use crate::hydro::accum::Method as FlowMethod;
use crate::hydro::{BasinMode, Coding, HydroNotes, HydroTool, HydroWork, Unit};
use crate::inputs::{
    Input, Mapping, Raw, Sampling, View, empty_of, mapping, place_in, point_of, region_for,
    sample_row,
};
use crate::out::{Out, OutSpec, Rows};
use crate::par;
use crate::reclass::{self, Bounds, Rule};
use crate::resample::{self, Method};
use crate::stats::{Moments, Stat, order_stat};
use crate::suitability::pairwise::{Pairwise, pairwise};
use crate::suitability::roc::{Roc, RocWork};
use crate::suitability::{FuzzyOp, Membership, Overlay, Scaled, common_grid, sum_weights};
use crate::vector::Features;
use crate::vector::capture::Colour;
use crate::vector::work::{PointMode, Select, VectorTool, VectorWork};

/// The widest block of columns a step works out.
pub const BLOCK_COLUMNS: u32 = 4096;
/// The most cells a step reads from its inputs (their value bands counted).
const MOST_READ: u64 = 1 << 24;
/// The most input rasters a run takes.
pub const MOST_INPUTS: usize = 64;

/// What the host asks for.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpsSpec {
    pub tool: OpsTool,
    /// The inputs as the host opened them, in the run's order (§2: the
    /// layers from the top of the panel down; on a layer the later first).
    pub inputs: Vec<InputSpec>,
    /// The result's EPSG code.
    #[serde(default)]
    pub epsg: Option<u32>,
    /// The project's coordinates are degrees.
    #[serde(default)]
    pub geographic: bool,
}

impl OpsSpec {
    /// The inputs the run reads, each once (§3): Raster hesaplayıcı's those
    /// its expression names, in the order it names them (the result takes
    /// the first's grid, file name and place); every other tool's all, in
    /// the run's order. Known before a raster is opened, so a host opens
    /// only these (a raster the expression does not name is neither opened
    /// nor counted); a name that is no raster's is refused here, with every
    /// input's name.
    pub fn reads(&self) -> Result<Vec<usize>, String> {
        match &self.tool {
            OpsTool::Calculator { expression, .. } => {
                let names: Vec<&str> = self.inputs.iter().map(|i| i.name.as_str()).collect();
                crate::calc::named(expression, &names)
            }
            _ => Ok((0..self.inputs.len()).collect()),
        }
    }

    /// The settings over only the inputs `keep` (`reads`), in that order.
    pub fn reading(&self, keep: &[usize]) -> OpsSpec {
        OpsSpec {
            tool: self.tool.clone(),
            inputs: keep.iter().map(|&k| self.inputs[k].clone()).collect(),
            epsg: self.epsg,
            geographic: self.geographic,
        }
    }
}

/// An input raster's place, look and name.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputSpec {
    /// The raster object's affine.
    pub affine: [f64; 6],
    /// The look's nodata (it stands for the file's); none: the file's.
    #[serde(default)]
    pub nodata: Option<f64>,
    /// Raster hesaplayıcı's name for it (§3).
    #[serde(default)]
    pub name: String,
    /// Its look (Maskeyle kırp, Mozaik and Yeniden örnekle keep it).
    #[serde(default)]
    pub style: Option<RasterStyle>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EmptyName {
    Propagate,
    Expression,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FloatSample {
    F32,
    F64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClassSample {
    F32,
    I32,
    U8,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BoundsName {
    UpperClosed,
    LowerClosed,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Unmatched {
    Keep,
    Empty,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Overlap {
    Top,
    Bottom,
    Mean,
    Min,
    Max,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MethodName {
    Nearest,
    Bilinear,
    Cubic,
    Mean,
    Mode,
    Min,
    Max,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ShapeName {
    Rect,
    Circle,
    Ring,
}

/// The tools of Raster işlemleri and Raster istatistiği (§1).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OpsTool {
    Calculator {
        expression: String,
        empty: EmptyName,
        sample: FloatSample,
    },
    Reclassify {
        band: u32,
        table: String,
        bounds: BoundsName,
        unmatched: Unmatched,
        sample: ClassSample,
    },
    ClipByMask {
        crop: bool,
    },
    Mosaic {
        overlap: Overlap,
        sampling: Sampling,
    },
    Resample {
        cell: f64,
        method: MethodName,
    },
    ZonalStatistics {
        band: u32,
        stat: Stat,
    },
    Histogram {
        band: u32,
        bins: u32,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    FocalStatistics {
        band: u32,
        shape: ShapeName,
        #[serde(default)]
        width: u32,
        #[serde(default)]
        height: u32,
        #[serde(default)]
        radius: u32,
        #[serde(default)]
        inner: u32,
        stat: Stat,
        ignore: bool,
    },
    CellStatistics {
        band: u32,
        stat: Stat,
        ignore: bool,
    },
    /// Rasterden alan (docs/adr/0234 §4).
    ToPolygons {
        band: u32,
        connect: Connect,
    },
    /// Rasterden çizgi (§5): the line cells, the short pieces dropped, the simplification.
    ToLines {
        band: u32,
        select: LineSelect,
        #[serde(default)]
        min: f64,
        #[serde(default)]
        max: f64,
        /// #RRGGBB.
        #[serde(default)]
        color: String,
        #[serde(default)]
        tolerance: f64,
        spur: u32,
        simplify: f64,
    },
    /// Rasterden nokta (§6).
    ToPoints {
        band: u32,
        mode: PointsMode,
        #[serde(default)]
        step: u32,
        #[serde(default)]
        radius: u32,
    },
    /// Çizgi yakala (§7): the point picked (drawing coordinates).
    CaptureLine {
        x: f64,
        y: f64,
        tolerance: f64,
        spur: u32,
        simplify: f64,
    },
    /// Alan kapat (§8).
    CloseArea {
        x: f64,
        y: f64,
        tolerance: f64,
        holes: HolesName,
        simplify: f64,
    },
    /// Çukur doldur (docs/adr/0235 §3): the least slope (percent), the result.
    Fill {
        band: u32,
        slope: f64,
        result: FillResult,
    },
    /// Akış yönü (§4).
    FlowDirection {
        band: u32,
        fill: bool,
        coding: CodingName,
    },
    /// Akış birikimi (§5): Çoklu yön's exponent (0: adaptive).
    FlowAccumulation {
        band: u32,
        fill: bool,
        method: FlowMethodName,
        exponent: f64,
        unit: UnitName,
    },
    /// Topografik nemlilik indisi (§6): the least slope (percent).
    Wetness {
        band: u32,
        fill: bool,
        method: FlowMethodName,
        exponent: f64,
        slope: f64,
    },
    /// Döküm noktası (§7): the snapping distance (m); the points are the run's objects.
    PourPoint {
        band: u32,
        fill: bool,
        snap: f64,
    },
    /// Noktadan havza (§8).
    Watershed {
        band: u32,
        fill: bool,
        snap: f64,
    },
    /// Havzalar (§9): the stream threshold (m², 0 the largest / 100) and the least area (m²).
    Basins {
        band: u32,
        fill: bool,
        mode: BasinModeName,
        threshold: f64,
        least: f64,
    },
    /// Dere ağı (§10).
    Streams {
        band: u32,
        fill: bool,
        threshold: f64,
        simplify: f64,
    },
    /// Uzaklık yüzeyi from a raster's cells with values (docs/adr/0236 §3):
    /// the largest distance (m, 0: none).
    Distance {
        band: u32,
        max: f64,
        result: DistanceResult,
    },
    /// Birikimli maliyet (docs/adr/0236 §4); a second input is the surface.
    /// The grade limit in per cent (0: none), the largest sum (0: none).
    CostDistance {
        band: u32,
        neighbours: NeighboursName,
        surface_length: bool,
        slope: f64,
        max: f64,
        result: CostResult,
        sample: FloatSample,
    },
    /// En düşük maliyetli yol (docs/adr/0236 §5): the objects before
    /// `first` are the sources, the rest the destinations.
    CostPath {
        band: u32,
        neighbours: NeighboursName,
        surface_length: bool,
        slope: f64,
        simplify: f64,
        first: u32,
    },
    /// Maliyet koridoru (docs/adr/0236 §6): the objects before `first` are
    /// the first ends, the rest the second.
    CostCorridor {
        band: u32,
        neighbours: NeighboursName,
        surface_length: bool,
        slope: f64,
        first: u32,
        threshold: ThresholdName,
        value: f64,
        sample: FloatSample,
    },
    /// Bulanık üyelik (docs/adr/0237 §3): the function and its settings.
    FuzzyMembership {
        band: u32,
        function: String,
        #[serde(default)]
        low: f64,
        #[serde(default)]
        high: f64,
        #[serde(default)]
        exponent: f64,
        #[serde(default)]
        midpoint: f64,
        #[serde(default)]
        spread: f64,
        #[serde(default)]
        steep: f64,
        sample: FloatSample,
    },
    /// Bulanık çakıştırma (§4).
    FuzzyOverlay {
        band: u32,
        op: String,
        #[serde(default)]
        gamma: f64,
        sample: FloatSample,
    },
    /// Ağırlıklı toplam (§5): the weights by the rasters' names (none: 1).
    WeightedSum {
        band: u32,
        #[serde(default)]
        weights: BTreeMap<String, Option<f64>>,
        sample: FloatSample,
    },
    /// Ağırlıklı çakıştırma (§6): the scale's ends, the influences (per
    /// cent) and the class tables by the rasters' names.
    WeightedOverlay {
        band: u32,
        low: f64,
        high: f64,
        #[serde(default)]
        influence: BTreeMap<String, Option<f64>>,
        #[serde(default)]
        classes: BTreeMap<String, Option<String>>,
        bounds: BoundsName,
    },
    /// İkili karşılaştırma (§7): the comparisons by the rasters' names;
    /// `write` the weighted sum's raster.
    Pairwise {
        band: u32,
        #[serde(default)]
        pairs: Vec<(String, String, f64)>,
        write: bool,
        sample: FloatSample,
    },
    /// ROC ile doğrulama (§8): the objects before `first` are the presence,
    /// the rest the absence (the background when `absence`, else every cell).
    Roc {
        band: u32,
        first: u32,
        absence: bool,
        higher: bool,
    },
}

/// Uzaklık yüzeyi's result (docs/adr/0236 §3).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DistanceResult {
    Distance,
    Allocation,
}

/// Birikimli maliyet's result (docs/adr/0236 §4).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CostResult {
    Cost,
    Allocation,
}

/// The cost network's neighbours (docs/adr/0236 §2).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum NeighboursName {
    #[serde(rename = "8")]
    Eight,
    #[serde(rename = "16")]
    Sixteen,
}

/// Maliyet koridoru's threshold (docs/adr/0236 §6).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThresholdName {
    None,
    Percent,
    Value,
}

/// Çukur doldur's result.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FillResult {
    Filled,
    Depth,
}

/// Akış yönü's coding.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodingName {
    Esri,
    Taudem,
}

/// How flow is shared.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FlowMethodName {
    D8,
    Mfd,
    Dinf,
}

/// Akış birikimi's unit.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UnitName {
    Cells,
    Area,
    Sca,
}

/// Havzalar' kind.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BasinModeName {
    Main,
    Sub,
    Route,
}

/// Rasterden alan's neighbours.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Connect {
    Four,
    Eight,
}

/// Rasterden çizgi's line cells.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LineSelect {
    NonZero,
    Range,
    Color,
}

/// Rasterden nokta's cells.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PointsMode {
    Step,
    All,
    Extrema,
}

/// Alan kapat's holes.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HolesName {
    Fill,
    Keep,
}

/// A colour written #RRGGBB.
pub fn colour_of(text: &str) -> Result<Colour, String> {
    let t = text.trim();
    let hex = t.strip_prefix('#').unwrap_or(t);
    let byte = |k: usize| {
        hex.get(k..k + 2)
            .and_then(|h| u8::from_str_radix(h, 16).ok())
    };
    match (hex.len(), byte(0), byte(2), byte(4)) {
        (6, Some(r), Some(g), Some(b)) => Ok([f64::from(r), f64::from(g), f64::from(b)]),
        _ => Err(format!("Renk #RRGGBB biçiminde olmalı; “{t}” okunamadı.")),
    }
}

/// A count or a size the tools take: a number ≥ `least` and ≤ `most`.
fn within(v: f64, least: f64, most: f64, what: &str) -> Result<f64, String> {
    if !(v >= least && v <= most) {
        return Err(format!("{what} {least} ile {most} arasında olmalı."));
    }
    Ok(v)
}

/// A hydrology tool's settings, checked (docs/adr/0235).
fn hydro_tool(tool: &OpsTool) -> Result<Option<(HydroTool, u32)>, String> {
    let method = |m: FlowMethodName, p: f64| -> Result<FlowMethod, String> {
        Ok(match m {
            FlowMethodName::D8 => FlowMethod::D8,
            FlowMethodName::Dinf => FlowMethod::Dinf,
            FlowMethodName::Mfd => {
                if !(p == 0.0 || (0.1..=100.0).contains(&p)) {
                    return Err(
                        "Çoklu yönün üssü 0 (uyarlanan) ya da 0,1 ile 100 arasında olmalı.".into(),
                    );
                }
                FlowMethod::Mfd(p)
            }
        })
    };
    let threshold = |t: f64| -> Result<f64, String> {
        if !(t >= 0.0 && t.is_finite()) {
            return Err("Eşik alanı 0 (kendiliğinden) ya da artı bir alan olmalı.".into());
        }
        Ok(t)
    };
    Ok(Some(match *tool {
        OpsTool::Fill {
            band,
            slope,
            result,
        } => (
            HydroTool::Fill {
                slope: within(slope, 0.0, 100.0, "En küçük eğim (yüzde)")?,
                depth: result == FillResult::Depth,
            },
            band,
        ),
        OpsTool::FlowDirection { band, fill, coding } => (
            HydroTool::Direction {
                fill,
                coding: match coding {
                    CodingName::Esri => Coding::Esri,
                    CodingName::Taudem => Coding::Taudem,
                },
            },
            band,
        ),
        OpsTool::FlowAccumulation {
            band,
            fill,
            method: m,
            exponent,
            unit,
        } => (
            HydroTool::Accumulation {
                fill,
                method: method(m, exponent)?,
                unit: match unit {
                    UnitName::Cells => Unit::Cells,
                    UnitName::Area => Unit::Area,
                    UnitName::Sca => Unit::Sca,
                },
            },
            band,
        ),
        OpsTool::Wetness {
            band,
            fill,
            method: m,
            exponent,
            slope,
        } => {
            if !(slope > 0.0 && slope <= 100.0) {
                return Err("En küçük eğim (yüzde) 0'dan büyük ve en çok 100 olmalı.".into());
            }
            (
                HydroTool::Wetness {
                    fill,
                    method: method(m, exponent)?,
                    slope,
                },
                band,
            )
        }
        OpsTool::PourPoint { band, fill, snap } => (
            HydroTool::PourPoints {
                fill,
                snap: within(snap, 0.0, 1e6, "Yaklaştırma uzaklığı")?,
            },
            band,
        ),
        OpsTool::Watershed { band, fill, snap } => (
            HydroTool::Watershed {
                fill,
                snap: within(snap, 0.0, 1e6, "Yaklaştırma uzaklığı")?,
            },
            band,
        ),
        OpsTool::Basins {
            band,
            fill,
            mode,
            threshold: t,
            least,
        } => (
            HydroTool::Basins {
                fill,
                mode: match mode {
                    BasinModeName::Main => BasinMode::Main,
                    BasinModeName::Sub => BasinMode::Sub,
                    BasinModeName::Route => BasinMode::Route,
                },
                threshold: threshold(t)?,
                least: within(least, 0.0, f64::MAX, "En küçük alan")?,
            },
            band,
        ),
        OpsTool::Streams {
            band,
            fill,
            threshold: t,
            simplify,
        } => (
            HydroTool::Streams {
                fill,
                threshold: threshold(t)?,
                simplify: within(simplify, 0.0, 1e6, "Sadeleştirme")?,
            },
            band,
        ),
        _ => return Ok(None),
    }))
}

/// A hydrology result's look (§3–§6).
fn hydro_style(tool: &HydroTool, source: Option<&RasterStyle>, kind: &Kind) -> RasterStyle {
    let ramp = |name: &str, stretch: RasterStretch| RasterStyle {
        stretch,
        ..ramp_style(name)
    };
    match tool {
        HydroTool::Fill { depth: false, .. } => kept_style(source, kind),
        HydroTool::Fill { depth: true, .. } => ramp("Viridis", RasterStretch::MinMax),
        HydroTool::Direction { coding, .. } => RasterStyle {
            stretch: RasterStretch::Manual,
            min: Some(1.0),
            max: Some(if *coding == Coding::Esri { 128.0 } else { 8.0 }),
            resampling: kentos_contracts::RasterResampling::Nearest,
            ..ramp_style("Spektral")
        },
        HydroTool::Wetness { .. } => RasterStyle {
            invert: true,
            ..ramp("Mavi-kırmızı", RasterStretch::Percent)
        },
        _ => ramp("Viridis", RasterStretch::Percent),
    }
}

/// Uzaklık ve maliyet's tool (docs/adr/0236): its settings, band and the result's samples; none for another tool.
fn distance_tool(tool: &OpsTool) -> Option<(DistanceTool, u32, RasterSample)> {
    let neighbours = |n: NeighboursName| match n {
        NeighboursName::Eight => Neighbours::Eight,
        NeighboursName::Sixteen => Neighbours::Sixteen,
    };
    let float = |s: FloatSample| match s {
        FloatSample::F32 => RasterSample::F32,
        FloatSample::F64 => RasterSample::F64,
    };
    Some(match *tool {
        OpsTool::Distance { band, max, result } => (
            DistanceTool::Euclid {
                max,
                allocation: result == DistanceResult::Allocation,
            },
            band,
            RasterSample::F32,
        ),
        OpsTool::CostDistance {
            band,
            neighbours: n,
            surface_length,
            slope,
            max,
            result,
            sample,
        } => {
            let allocation = result == CostResult::Allocation;
            (
                DistanceTool::Cost {
                    neighbours: neighbours(n),
                    surface_length,
                    slope,
                    max,
                    allocation,
                },
                band,
                if allocation {
                    RasterSample::F32
                } else {
                    float(sample)
                },
            )
        }
        OpsTool::CostPath {
            band,
            neighbours: n,
            surface_length,
            slope,
            simplify,
            first,
        } => (
            DistanceTool::Path {
                neighbours: neighbours(n),
                surface_length,
                slope,
                simplify,
                first: first as usize,
            },
            band,
            RasterSample::F32,
        ),
        OpsTool::CostCorridor {
            band,
            neighbours: n,
            surface_length,
            slope,
            first,
            threshold,
            value,
            sample,
        } => (
            DistanceTool::Corridor {
                neighbours: neighbours(n),
                surface_length,
                slope,
                first: first as usize,
                threshold: match threshold {
                    ThresholdName::None => Threshold::None,
                    ThresholdName::Percent => Threshold::Percent(value),
                    ThresholdName::Value => Threshold::Value(value),
                },
            },
            band,
            float(sample),
        ),
        _ => return None,
    })
}

/// A distance or cost result's look (docs/adr/0236 §3, §4, §6): the
/// distances and sums Viridis, the sources' numbers spectral, cell by cell.
fn distance_style(tool: &DistanceTool) -> RasterStyle {
    let allocation = matches!(
        tool,
        DistanceTool::Euclid {
            allocation: true,
            ..
        } | DistanceTool::Cost {
            allocation: true,
            ..
        }
    );
    if allocation {
        return RasterStyle {
            resampling: kentos_contracts::RasterResampling::Nearest,
            ..ramp_style("Spektral")
        };
    }
    match tool {
        DistanceTool::Cost { .. } => RasterStyle {
            stretch: RasterStretch::Percent,
            ..ramp_style("Viridis")
        },
        _ => ramp_style("Viridis"),
    }
}

/// A zone's figures (Bölgesel istatistik): its sums and the chosen statistic.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneFigures {
    pub moments: Moments,
    /// The chosen statistic; none when the zone has no cell with a value.
    pub value: Option<f64>,
}

/// A histogram (§11).
#[derive(Clone, Debug, PartialEq)]
pub struct Histogram {
    pub lo: f64,
    pub hi: f64,
    pub counts: Vec<u64>,
    /// Values below the least and above the largest given.
    pub below: u64,
    pub above: u64,
    /// Cells with a value and without one.
    pub valid: u64,
    pub empty: u64,
}

/// How a run ends.
#[derive(Debug)]
pub enum OpsFinished {
    /// The GeoTIFF's directories to append and the header to write over its start.
    Raster {
        tail: Vec<u8>,
        header: Vec<u8>,
    },
    /// Each zone's figures, in the zones' order.
    Zones(Vec<ZoneFigures>),
    Histogram(Histogram),
    /// Areas, polylines or points (docs/adr/0234).
    Features(Features),
    /// ROC ile doğrulama's figures (docs/adr/0237 §8).
    Roc(Roc),
    /// İkili karşılaştırma without its raster: the weights are in the notes.
    Weights,
}

/// What a run met, for the host's summary and warnings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notes {
    /// Maskeyle kırp: cells inside the mask; Yeniden sınıflandır: cells no rule held.
    pub cells: u64,
    /// Cells of the result without a value.
    pub empty_cells: u64,
    /// A hydrology run's (docs/adr/0235).
    pub hydro: HydroNotes,
    /// A distance or cost run's (docs/adr/0236).
    pub distance: DistanceNotes,
    /// A suitability run's (docs/adr/0237).
    pub suit: SuitNotes,
}

/// What a suitability run met (docs/adr/0237).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SuitNotes {
    /// Ağırlıklı çakıştırma: cells left empty because a table had no rule
    /// for a value, or a value was off the scale (each by its first input
    /// in the run's order), and the restricted cells.
    pub unmatched: u64,
    pub outside: u64,
    pub restricted: u64,
    /// Bulanık çakıştırma: cells with a membership outside 0–1.
    pub invalid: u64,
    /// İkili karşılaştırma's weights.
    pub pairwise: Option<Pairwise>,
}

struct Hist {
    band: usize,
    bins: u32,
    lo: f64,
    hi: f64,
    /// The first pass works out the band's least and largest.
    bounds_pass: bool,
    /// The bounds were not given: two passes in all.
    two_pass: bool,
    out: Histogram,
}

enum Work {
    Calc {
        calc: Calc,
        maps: Vec<Mapping>,
    },
    Reclass {
        band: usize,
        rules: Vec<Rule>,
        bounds: Bounds,
        keep: bool,
    },
    Clip {
        areas: Areas,
        /// The source cell of the result's cell (0, 0).
        off: (u32, u32),
    },
    Mosaic {
        overlap: Overlap,
        sampling: Sampling,
        maps: Vec<Mapping>,
    },
    Resample {
        method: Method,
        ru: f64,
        rv: f64,
    },
    Zonal {
        areas: Areas,
        band: usize,
        stat: Stat,
        zones: Vec<Moments>,
        values: Option<Vec<Vec<f64>>>,
        cell_area: f64,
    },
    Histogram(Box<Hist>),
    Focal {
        focal: Focal,
        band: usize,
    },
    Cells {
        band: usize,
        stat: Stat,
        ignore: bool,
        maps: Vec<Mapping>,
    },
    Vector(Box<VectorWork>),
    Hydro(Box<HydroWork>),
    Distance(Box<DistanceWork>),
    /// A suitability tool's cells (docs/adr/0237 §3–§7).
    Suit {
        cell: SuitCell,
        band: usize,
        maps: Vec<Mapping>,
    },
    Roc(Box<RocWork>),
    /// İkili karşılaştırma without its raster: nothing to read.
    Weights,
}

/// A suitability cell's rule.
enum SuitCell {
    Membership(Membership),
    Fuzzy(FuzzyOp),
    Sum(Vec<f64>),
    Overlay(Box<Overlay>),
}

/// The result raster's samples: type, bands (alpha included), nodata.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Kind {
    sample: RasterSample,
    /// Value bands.
    values: u32,
    alpha: bool,
    nodata: Option<f64>,
}

impl Kind {
    fn bands(&self) -> u32 {
        self.values + u32::from(self.alpha)
    }

    /// A value band's sample where there is no value.
    fn fill(&self) -> f64 {
        match self.nodata {
            Some(d) => d,
            None if self.sample.float() => f64::NAN,
            None => 0.0,
        }
    }
}

/// The run's state.
pub struct OpsJob {
    inputs: Vec<Input>,
    /// The grid the run steps over: the result's, or the source's for a table.
    grid: Grid,
    work: Work,
    out: Option<Out>,
    kind: Option<Kind>,
    /// The result's look (a raster result).
    style: Option<RasterStyle>,
    /// The next strip's first row and the next block.
    next: u32,
    block: usize,
    /// The column blocks every strip is cut into.
    blocks: Vec<(u32, u32)>,
    /// The strip being put together (a raster result).
    strip: Option<Samples>,
    threads: usize,
    notes: Notes,
}

impl std::fmt::Debug for OpsJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpsJob")
            .field("next", &self.next)
            .field("block", &self.block)
            .finish()
    }
}

fn band_index(input: &Input, band: u32) -> Result<usize, String> {
    if band == 0 || band > input.values() {
        return Err(format!(
            "Rasterin {} bandı var; {band}. bant yok.",
            input.values()
        ));
    }
    Ok(band as usize - 1)
}

/// The finest input's lattice grown to hold every input (§7).
fn union_grid(inputs: &[Input]) -> Result<Grid, String> {
    let area = |k: usize| {
        let [_, a, b, _, c, d] = inputs[k].affine;
        (a * d - b * c).abs()
    };
    let mut base = 0;
    for k in 1..inputs.len() {
        if area(k) < area(base) {
            base = k;
        }
    }
    let g = inputs[base].grid();
    let (mut umin, mut umax, mut vmin, mut vmax) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for input in inputs {
        let (w, h) = (f64::from(input.width), f64::from(input.height));
        for (i, j) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
            let (x, y) = point_of(&input.affine, i, j);
            let (u, v) = place_in(&g.affine, x, y);
            umin = umin.min(u);
            umax = umax.max(u);
            vmin = vmin.min(v);
            vmax = vmax.max(v);
        }
    }
    let (i0, i1) = ((umin + 1e-9).floor(), (umax - 1e-9).ceil());
    let (j0, j1) = ((vmin + 1e-9).floor(), (vmax - 1e-9).ceil());
    if !(i1 > i0 && j1 > j0) || i1 - i0 > f64::from(u32::MAX) || j1 - j0 > f64::from(u32::MAX) {
        return Err("Girdilerin ortak ızgarası kurulamadı.".into());
    }
    let (x0, y0) = point_of(&g.affine, i0, j0);
    let [_, a, b, _, c, d] = g.affine;
    Grid::of([x0, a, b, y0, c, d], (i1 - i0) as u32, (j1 - j0) as u32)
}

/// A ramp's look over a single band (min–max).
fn ramp_style(ramp: &str) -> RasterStyle {
    RasterStyle {
        render: RasterRender::Ramp,
        bands: vec![1],
        stretch: RasterStretch::MinMax,
        ramp: Some(ramp.to_owned()),
        invert: false,
        ..default_style(1, RasterSample::F32, false)
    }
}

/// A source's look kept for its cells (its alpha band drawn when one is added).
fn kept_style(source: Option<&RasterStyle>, kind: &Kind) -> RasterStyle {
    let mut s = source
        .cloned()
        .unwrap_or_else(|| default_style(kind.bands(), kind.sample, false));
    if kind.alpha && s.render == RasterRender::Rgb && s.bands.len() == 3 {
        s.bands.push(kind.bands());
    }
    s
}

/// A vectorizing tool's settings, checked.
fn vector_tool(tool: &OpsTool, first: &Input) -> Result<VectorTool, String> {
    let eps = |v: f64| within(v, 0.0, 1e6, "Sadeleştirme");
    let tol = |v: f64| within(v, 0.0, 1e9, "Renk toleransı");
    let place = |x: f64, y: f64| {
        if x.is_finite() && y.is_finite() {
            Ok((x, y))
        } else {
            Err("Çizimde bir nokta seçin.".to_owned())
        }
    };
    Ok(match tool {
        OpsTool::ToPolygons { band, connect } => VectorTool::Polygons {
            band: band_index(first, *band)?,
            eight: *connect == Connect::Eight,
        },
        OpsTool::ToLines {
            band,
            select,
            min,
            max,
            color,
            tolerance,
            spur,
            simplify,
        } => VectorTool::Lines {
            band: band_index(first, *band)?,
            select: match select {
                LineSelect::NonZero => Select::NonZero,
                LineSelect::Range => {
                    if !(min.is_finite() && max.is_finite()) {
                        return Err("En küçük ve en büyük değer sayı olmalı.".into());
                    }
                    if min > max {
                        return Err("En küçük değer en büyükten büyük olamaz.".into());
                    }
                    Select::Range {
                        min: *min,
                        max: *max,
                    }
                }
                LineSelect::Color => Select::Colour {
                    colour: colour_of(color)?,
                    tol: tol(*tolerance)?,
                },
            },
            spur: *spur,
            eps: eps(*simplify)?,
        },
        OpsTool::ToPoints {
            band,
            mode,
            step,
            radius,
        } => VectorTool::Points {
            band: band_index(first, *band)?,
            mode: match mode {
                PointsMode::All => PointMode::All,
                PointsMode::Step => {
                    PointMode::Step(within(f64::from(*step), 1.0, 100_000.0, "Adım")? as u32)
                }
                PointsMode::Extrema => {
                    PointMode::Extrema(
                        within(f64::from(*radius), 1.0, 50.0, "Pencere yarıçapı")? as u32
                    )
                }
            },
        },
        OpsTool::CaptureLine {
            x,
            y,
            tolerance,
            spur,
            simplify,
        } => VectorTool::CaptureLine {
            at: place(*x, *y)?,
            tol: tol(*tolerance)?,
            spur: *spur,
            eps: eps(*simplify)?,
        },
        OpsTool::CloseArea {
            x,
            y,
            tolerance,
            holes,
            simplify,
        } => VectorTool::CloseArea {
            at: place(*x, *y)?,
            tol: tol(*tolerance)?,
            keep_holes: *holes == HolesName::Keep,
            eps: eps(*simplify)?,
        },
        _ => return Err("Bu araç vektörleştirme değil.".into()),
    })
}

impl OpsJob {
    /// A run of `spec` over the opened inputs (in `spec.inputs`' order) and
    /// `shapes` (the mask's or the zones' objects), its rows on `threads`;
    /// the bytes to write first (none for a table).
    pub fn new(
        inputs: Vec<Input>,
        spec: &OpsSpec,
        shapes: Vec<Shape>,
        threads: usize,
    ) -> Result<(OpsJob, Vec<u8>), String> {
        if inputs.is_empty() {
            return Err("Raster seçin.".into());
        }
        if inputs.len() > MOST_INPUTS {
            return Err(format!(
                "En çok {MOST_INPUTS} raster birlikte işlenir; {} raster seçili.",
                inputs.len()
            ));
        }
        if inputs.len() != spec.inputs.len() {
            return Err("Rasterlerin ayarları eksik verildi.".into());
        }
        for input in &inputs {
            let (w, h) = (input.width, input.height);
            if w > crate::job::MOST_WIDTH || u64::from(w) * u64::from(h) > crate::job::MOST_CELLS {
                return Err(format!(
                    "Raster çözümleme için çok büyük ({w} × {h}): genişlik en çok {}, hücre sayısı en çok 2³¹. Rasteri parçalara bölün.",
                    crate::job::MOST_WIDTH
                ));
            }
        }
        let first = &inputs[0];
        let first_style = spec.inputs[0].style.as_ref();
        let float_of = |s: FloatSample| match s {
            FloatSample::F32 => RasterSample::F32,
            FloatSample::F64 => RasterSample::F64,
        };
        let float_kind = |sample| Kind {
            sample,
            values: 1,
            alpha: false,
            nodata: Some(f64::NAN),
        };
        let wide = |inputs: &[&Input]| {
            if inputs.iter().any(|i| i.sample == RasterSample::F64) {
                RasterSample::F64
            } else {
                RasterSample::F32
            }
        };
        let names: Vec<String> = spec.inputs.iter().map(|i| i.name.clone()).collect();
        let mut suit_notes = SuitNotes::default();
        let (grid, work, kind, style) = match &spec.tool {
            OpsTool::Calculator {
                expression,
                empty,
                sample,
            } => {
                let names: Vec<(String, u32)> = spec
                    .inputs
                    .iter()
                    .zip(&inputs)
                    .map(|(s, i)| (s.name.clone(), i.values()))
                    .collect();
                let empty = match empty {
                    EmptyName::Propagate => Empty::Propagate,
                    EmptyName::Expression => Empty::Expression,
                };
                let calc = Calc::new(expression, &names, empty)?;
                let grid = inputs[calc.grid_input()].grid();
                let maps = inputs.iter().map(|i| mapping(&grid, &i.affine)).collect();
                (
                    grid,
                    Work::Calc { calc, maps },
                    Some(float_kind(float_of(*sample))),
                    Some(ramp_style("Viridis")),
                )
            }
            OpsTool::Reclassify {
                band,
                table,
                bounds,
                unmatched,
                sample,
            } => {
                let b = band_index(first, *band)?;
                let rules = reclass::parse(table)?;
                let (sample, nodata, range) = match sample {
                    ClassSample::F32 => (RasterSample::F32, f64::NAN, None),
                    ClassSample::I32 => (
                        RasterSample::I32,
                        -2_147_483_648.0,
                        Some((-2_147_483_647.0, 2_147_483_647.0)),
                    ),
                    ClassSample::U8 => (RasterSample::U8, 255.0, Some((0.0, 254.0))),
                };
                if let Some((lo, hi)) = range
                    && let Some(v) = reclass::new_values(&rules)
                        .map(|v| v.round())
                        .find(|v| *v < lo || *v > hi)
                {
                    return Err(format!(
                        "Yeni değer {v} seçilen türe sığmıyor ({lo} ile {hi} arası olmalı); Ondalık 32 bit'i seçin."
                    ));
                }
                (
                    first.grid(),
                    Work::Reclass {
                        band: b,
                        rules,
                        bounds: match bounds {
                            BoundsName::UpperClosed => Bounds::UpperClosed,
                            BoundsName::LowerClosed => Bounds::LowerClosed,
                        },
                        keep: *unmatched == Unmatched::Keep,
                    },
                    Some(Kind {
                        sample,
                        values: 1,
                        alpha: false,
                        nodata: Some(nodata),
                    }),
                    Some(ramp_style("Spektral")),
                )
            }
            OpsTool::ClipByMask { crop } => {
                let source = first.grid();
                let areas = Areas::new(&source, &shapes);
                let (i0, i1, j0, j1) = if *crop {
                    areas.inside_box().ok_or(
                        "Maske rasterle kesişmiyor: maskenin içinde rasterin hiçbir hücre merkezi yok.",
                    )?
                } else {
                    (0, source.width, 0, source.height)
                };
                let (x0, y0) = point_of(&source.affine, f64::from(i0), f64::from(j0));
                let [_, a, b, _, c, d] = source.affine;
                let grid = Grid::of([x0, a, b, y0, c, d], i1 - i0, j1 - j0)?;
                let (nodata, alpha) =
                    empty_of(first.sample, first.values(), first.alpha, first.nodata);
                let kind = Kind {
                    sample: first.sample,
                    values: first.values(),
                    alpha,
                    nodata,
                };
                (
                    grid,
                    Work::Clip {
                        areas,
                        off: (i0, j0),
                    },
                    Some(kind),
                    Some(kept_style(first_style, &kind)),
                )
            }
            OpsTool::Mosaic { overlap, sampling } => {
                if inputs.len() < 2 {
                    return Err("Mozaik en az iki raster ister.".into());
                }
                if let Some(other) = inputs
                    .iter()
                    .find(|i| i.values() != first.values() || i.sample != first.sample)
                {
                    return Err(format!(
                        "Mozaiğin rasterleri aynı türde olmalı: ilki {} bantlı {:?}, biri {} bantlı {:?}.",
                        first.values(),
                        first.sample,
                        other.values(),
                        other.sample
                    ));
                }
                let grid = union_grid(&inputs)?;
                let maps = inputs.iter().map(|i| mapping(&grid, &i.affine)).collect();
                let alpha_in = inputs.iter().any(|i| i.alpha);
                let (nodata, alpha) =
                    empty_of(first.sample, first.values(), alpha_in, first.nodata);
                let kind = Kind {
                    sample: first.sample,
                    values: first.values(),
                    alpha,
                    nodata,
                };
                (
                    grid,
                    Work::Mosaic {
                        overlap: *overlap,
                        sampling: *sampling,
                        maps,
                    },
                    Some(kind),
                    Some(kept_style(first_style, &kind)),
                )
            }
            OpsTool::Resample { cell, method } => {
                let (grid, ru, rv) = resample::grid_of(&first.grid(), *cell)?;
                let method = match method {
                    MethodName::Nearest => Method::Point(Sampling::Nearest),
                    MethodName::Bilinear => Method::Point(Sampling::Bilinear),
                    MethodName::Cubic => Method::Point(Sampling::Cubic),
                    MethodName::Mean => Method::Mean,
                    MethodName::Mode => Method::Mode,
                    MethodName::Min => Method::Min,
                    MethodName::Max => Method::Max,
                };
                let (nodata, alpha) =
                    empty_of(first.sample, first.values(), first.alpha, first.nodata);
                let kind = Kind {
                    sample: first.sample,
                    values: first.values(),
                    alpha,
                    nodata,
                };
                (
                    grid,
                    Work::Resample { method, ru, rv },
                    Some(kind),
                    Some(kept_style(first_style, &kind)),
                )
            }
            OpsTool::ZonalStatistics { band, stat } => {
                let b = band_index(first, *band)?;
                let grid = first.grid();
                let areas = Areas::new(&grid, &shapes);
                let [_, a, bb, _, c, d] = grid.affine;
                let zones = vec![Moments::default(); shapes.len()];
                let values = stat.orders().then(|| vec![Vec::new(); shapes.len()]);
                (
                    grid,
                    Work::Zonal {
                        areas,
                        band: b,
                        stat: *stat,
                        zones,
                        values,
                        cell_area: (a * d - bb * c).abs(),
                    },
                    None,
                    None,
                )
            }
            OpsTool::Histogram {
                band,
                bins,
                min,
                max,
            } => {
                let b = band_index(first, *band)?;
                if !(1..=1000).contains(bins) {
                    return Err("Aralık sayısı 1 ile 1000 arasında olmalı.".into());
                }
                let given =
                    match (min, max) {
                        (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() && lo <= hi => {
                            Some((*lo, *hi))
                        }
                        (Some(_), Some(_)) => {
                            return Err("En küçük en büyükten büyük olamaz.".into());
                        }
                        (None, None) => None,
                        _ => return Err(
                            "En küçük ve en büyük birlikte verilmeli ya da ikisi de boş kalmalı."
                                .into(),
                        ),
                    };
                let (lo, hi) = given.unwrap_or((f64::INFINITY, f64::NEG_INFINITY));
                (
                    first.grid(),
                    Work::Histogram(Box::new(Hist {
                        band: b,
                        bins: *bins,
                        lo,
                        hi,
                        bounds_pass: given.is_none(),
                        two_pass: given.is_none(),
                        out: Histogram {
                            lo,
                            hi,
                            counts: vec![0; *bins as usize],
                            below: 0,
                            above: 0,
                            valid: 0,
                            empty: 0,
                        },
                    })),
                    None,
                    None,
                )
            }
            OpsTool::FocalStatistics {
                band,
                shape,
                width,
                height,
                radius,
                inner,
                stat,
                ignore,
            } => {
                let b = band_index(first, *band)?;
                let window = match shape {
                    ShapeName::Rect => Window::rect(*width, *height)?,
                    ShapeName::Circle => Window::circle(*radius)?,
                    ShapeName::Ring => Window::ring(*inner, *radius)?,
                };
                let focal = Focal::new(window, *stat, *ignore)?;
                (
                    first.grid(),
                    Work::Focal { focal, band: b },
                    Some(float_kind(wide(&[first]))),
                    Some(ramp_style("Viridis")),
                )
            }
            OpsTool::ToPolygons { .. }
            | OpsTool::ToLines { .. }
            | OpsTool::ToPoints { .. }
            | OpsTool::CaptureLine { .. }
            | OpsTool::CloseArea { .. } => {
                let tool = vector_tool(&spec.tool, first)?;
                let (work, grid) = VectorWork::new(tool, first, threads)?;
                (grid, Work::Vector(Box::new(work)), None, None)
            }
            OpsTool::Fill { .. }
            | OpsTool::FlowDirection { .. }
            | OpsTool::FlowAccumulation { .. }
            | OpsTool::Wetness { .. }
            | OpsTool::PourPoint { .. }
            | OpsTool::Watershed { .. }
            | OpsTool::Basins { .. }
            | OpsTool::Streams { .. } => {
                let (tool, band) = hydro_tool(&spec.tool)?.ok_or("Hidroloji aracı bilinmiyor.")?;
                let b = band_index(first, band)?;
                let grid = first.grid();
                let kind = tool.raster(first.sample).map(|(sample, nodata)| Kind {
                    sample,
                    values: 1,
                    alpha: false,
                    nodata: Some(nodata),
                });
                let style = kind.map(|k| hydro_style(&tool, first_style, &k));
                let work = HydroWork::new(
                    tool,
                    b,
                    first.sample,
                    (grid.width, grid.height),
                    grid.affine,
                    spec.geographic,
                    &shapes,
                    threads,
                )?;
                (grid, Work::Hydro(Box::new(work)), kind, style)
            }
            OpsTool::Distance { .. }
            | OpsTool::CostDistance { .. }
            | OpsTool::CostPath { .. }
            | OpsTool::CostCorridor { .. } => {
                let (tool, band, sample) =
                    distance_tool(&spec.tool).ok_or("Uzaklık aracı bilinmiyor.")?;
                let b = band_index(first, band)?;
                let surface = match tool {
                    DistanceTool::Euclid { .. } => {
                        if inputs.len() > 1 {
                            return Err(
                                "Uzaklık yüzeyi tek raster ister: kaynakların rasteri.".into()
                            );
                        }
                        None
                    }
                    _ => {
                        if inputs.len() > 2 {
                            return Err(
                                "En çok iki raster: maliyet rasteri ve yükseklik modeli.".into()
                            );
                        }
                        inputs.get(1).map(|i| i.affine)
                    }
                };
                let grid = first.grid();
                let kind = tool.raster().then_some(Kind {
                    sample,
                    values: 1,
                    alpha: false,
                    nodata: Some(f64::NAN),
                });
                let style = kind.map(|_| distance_style(&tool));
                let work = DistanceWork::new(
                    tool,
                    b,
                    grid,
                    spec.geographic,
                    surface,
                    shapes.clone(),
                    threads,
                )?;
                (grid, Work::Distance(Box::new(work)), kind, style)
            }
            OpsTool::CellStatistics { band, stat, ignore } => {
                if inputs.len() < 2 {
                    return Err("Hücre istatistiği en az iki raster ister.".into());
                }
                if *stat == Stat::Area {
                    return Err("Hücre istatistiği alan vermez.".into());
                }
                let b = band_index(first, *band)?;
                for i in &inputs {
                    band_index(i, *band)?;
                }
                let grid = union_grid(&inputs)?;
                let maps = inputs.iter().map(|i| mapping(&grid, &i.affine)).collect();
                let all: Vec<&Input> = inputs.iter().collect();
                (
                    grid,
                    Work::Cells {
                        band: b,
                        stat: *stat,
                        ignore: *ignore,
                        maps,
                    },
                    Some(float_kind(wide(&all))),
                    Some(ramp_style("Viridis")),
                )
            }
            OpsTool::FuzzyMembership {
                band,
                function,
                low,
                high,
                exponent,
                midpoint,
                spread,
                steep,
                sample,
            } => {
                if inputs.len() > 1 {
                    return Err(format!(
                        "{} raster seçili; Bulanık üyelik tek raster ister.",
                        inputs.len()
                    ));
                }
                let b = band_index(first, *band)?;
                let m = Membership::new(
                    function,
                    (*low, *high, *exponent),
                    (*midpoint, *spread, *steep),
                )?;
                let grid = first.grid();
                (
                    grid,
                    Work::Suit {
                        cell: SuitCell::Membership(m),
                        band: b,
                        maps: vec![mapping(&grid, &first.affine)],
                    },
                    Some(float_kind(float_of(*sample))),
                    Some(suit_style(Some((0.0, 1.0)), false)),
                )
            }
            OpsTool::FuzzyOverlay {
                band,
                op,
                gamma,
                sample,
            } => {
                if inputs.len() < 2 {
                    return Err("Bulanık çakıştırma en az iki raster ister.".into());
                }
                let op = FuzzyOp::new(op, *gamma)?;
                let (grid, b, maps) = suit_inputs(&inputs, *band)?;
                (
                    grid,
                    Work::Suit {
                        cell: SuitCell::Fuzzy(op),
                        band: b,
                        maps,
                    },
                    Some(float_kind(float_of(*sample))),
                    Some(suit_style(Some((0.0, 1.0)), false)),
                )
            }
            OpsTool::WeightedSum {
                band,
                weights,
                sample,
            } => {
                let given: BTreeMap<String, f64> = weights
                    .iter()
                    .filter_map(|(k, v)| v.map(|v| (k.clone(), v)))
                    .collect();
                let w = sum_weights(&names, &given)?;
                let (grid, b, maps) = suit_inputs(&inputs, *band)?;
                (
                    grid,
                    Work::Suit {
                        cell: SuitCell::Sum(w),
                        band: b,
                        maps,
                    },
                    Some(float_kind(float_of(*sample))),
                    Some(suit_style(None, false)),
                )
            }
            OpsTool::WeightedOverlay {
                band,
                low,
                high,
                influence,
                classes,
                bounds,
            } => {
                if inputs.len() < 2 {
                    return Err("Ağırlıklı çakıştırma en az iki raster ister.".into());
                }
                let influence: BTreeMap<String, f64> = influence
                    .iter()
                    .filter_map(|(k, v)| v.map(|v| (k.clone(), v)))
                    .collect();
                let classes: BTreeMap<String, String> = classes
                    .iter()
                    .filter_map(|(k, v)| v.clone().map(|v| (k.clone(), v)))
                    .collect();
                let overlay = Overlay::new(
                    &names,
                    (*low, *high),
                    &influence,
                    &classes,
                    match bounds {
                        BoundsName::UpperClosed => Bounds::UpperClosed,
                        BoundsName::LowerClosed => Bounds::LowerClosed,
                    },
                )?;
                let (grid, b, maps) = suit_inputs(&inputs, *band)?;
                let range = ((overlay.lo - 1) as f64, overlay.hi as f64);
                (
                    grid,
                    Work::Suit {
                        cell: SuitCell::Overlay(Box::new(overlay)),
                        band: b,
                        maps,
                    },
                    Some(Kind {
                        sample: RasterSample::I32,
                        values: 1,
                        alpha: false,
                        nodata: Some(-2_147_483_648.0),
                    }),
                    Some(suit_style(Some(range), true)),
                )
            }
            OpsTool::Pairwise {
                band,
                pairs,
                write,
                sample,
            } => {
                let p = pairwise(&names, pairs)?;
                let weights = p.weights.clone();
                suit_notes.pairwise = Some(p);
                if *write {
                    let (grid, b, maps) = suit_inputs(&inputs, *band)?;
                    (
                        grid,
                        Work::Suit {
                            cell: SuitCell::Sum(weights),
                            band: b,
                            maps,
                        },
                        Some(float_kind(float_of(*sample))),
                        Some(suit_style(None, false)),
                    )
                } else {
                    (first.grid(), Work::Weights, None, None)
                }
            }
            OpsTool::Roc {
                band,
                first: split,
                absence,
                higher,
            } => {
                if inputs.len() > 1 {
                    return Err(format!(
                        "{} raster seçili; ROC ile doğrulama tek raster ister.",
                        inputs.len()
                    ));
                }
                let b = band_index(first, *band)?;
                let grid = first.grid();
                let work = RocWork::new(&grid, b, &shapes, *split as usize, *absence, *higher)?;
                (grid, Work::Roc(Box::new(work)), None, None)
            }
        };
        if u64::from(grid.width) * u64::from(grid.height) > crate::job::MOST_CELLS
            || grid.width > crate::job::MOST_WIDTH
        {
            return Err(format!(
                "Sonuç çok büyük ({} × {}): genişlik en çok {}, hücre sayısı en çok 2³¹.",
                grid.width,
                grid.height,
                crate::job::MOST_WIDTH
            ));
        }
        let (out, header) = match kind {
            Some(k) => {
                let (o, h) = Out::new(
                    OutSpec {
                        width: grid.width,
                        height: grid.height,
                        bands: k.bands(),
                        sample: k.sample,
                        alpha: k.alpha,
                        nodata: k.nodata,
                        geo: Geo {
                            affine: grid.affine,
                            epsg: spec.epsg,
                            geographic: spec.geographic,
                        },
                    },
                    threads,
                )?;
                (Some(o), h)
            }
            None => (None, Vec::new()),
        };
        let mut job = OpsJob {
            inputs,
            grid,
            work,
            out,
            kind,
            style,
            next: 0,
            block: 0,
            blocks: Vec::new(),
            strip: None,
            threads: threads.max(1),
            notes: Notes {
                suit: suit_notes,
                ..Notes::default()
            },
        };
        job.blocks = job.plan();
        Ok((job, header))
    }

    /// The column blocks: at most 4096 wide, narrower while a block would
    /// read more than its share from the inputs (many inputs, a turned one).
    fn plan(&self) -> Vec<(u32, u32)> {
        let w = self.grid.width;
        let mut bw = BLOCK_COLUMNS;
        while bw > TILE {
            let rect = (0, bw.min(w), 0, TILE.min(self.grid.height));
            let read: u64 = self
                .regions(rect)
                .iter()
                .map(|(k, r)| u64::from(r.2) * u64::from(r.3) * u64::from(self.inputs[*k].values()))
                .sum();
            if read <= MOST_READ {
                break;
            }
            bw /= 2;
        }
        (0..w.div_ceil(bw))
            .map(|k| (k * bw, ((k + 1) * bw).min(w)))
            .collect()
    }

    /// The inputs a block of the grid (columns `c0..c1`, rows `y0..y1`) reads, and their regions.
    fn regions(
        &self,
        (c0, c1, y0, y1): (u32, u32, u32, u32),
    ) -> Vec<(usize, (i64, i64, u32, u32))> {
        let mut out = Vec::new();
        let mut add = |k: usize, margin: i64| {
            if let Some(r) = region_for(&self.inputs[k], &self.grid, (c0, c1, y0, y1), margin) {
                out.push((k, r));
            }
        };
        match &self.work {
            Work::Calc { calc, .. } => calc.reads().into_iter().for_each(|k| add(k, 0)),
            Work::Reclass { .. } | Work::Clip { .. } | Work::Zonal { .. } | Work::Histogram(_) => {
                add(0, 0)
            }
            Work::Mosaic { sampling, .. } => {
                (0..self.inputs.len()).for_each(|k| add(k, sampling.margin()))
            }
            Work::Cells { .. } | Work::Suit { .. } => {
                (0..self.inputs.len()).for_each(|k| add(k, 0))
            }
            Work::Roc(_) => add(0, 0),
            Work::Weights => {}
            Work::Vector(v) => add(0, v.margin()),
            Work::Hydro(h) => {
                if h.reading() {
                    add(0, 0)
                }
            }
            Work::Distance(d) => {
                if d.reading() {
                    add(0, 0);
                    if d.reads_surface() {
                        add(1, Sampling::Bilinear.margin());
                    }
                }
            }
            Work::Resample { method, ru, rv } => match method {
                Method::Point(s) => add(0, s.margin()),
                _ => {
                    let input = &self.inputs[0];
                    let (xa, xb) = resample::reach(c0, c1, *ru);
                    let (ya, yb) = resample::reach(y0, y1, *rv);
                    let (xa, ya) = (xa.max(0), ya.max(0));
                    let (xb, yb) = (
                        xb.min(i64::from(input.width)),
                        yb.min(i64::from(input.height)),
                    );
                    if xa < xb && ya < yb {
                        out.push((0, (xa, ya, (xb - xa) as u32, (yb - ya) as u32)));
                    }
                }
            },
            Work::Focal { focal, .. } => {
                let (hx, hy) = focal.window.halo();
                let input = &self.inputs[0];
                let xa = (i64::from(c0) - i64::from(hx)).max(0);
                let ya = (i64::from(y0) - i64::from(hy)).max(0);
                let xb = (i64::from(c1) + i64::from(hx)).min(i64::from(input.width));
                let yb = (i64::from(y1) + i64::from(hy)).min(i64::from(input.height));
                if xa < xb && ya < yb {
                    out.push((0, (xa, ya, (xb - xa) as u32, (yb - ya) as u32)));
                }
            }
        }
        out
    }

    /// The current block: columns, first row, rows.
    fn rect(&self) -> (u32, u32, u32, u32) {
        let (c0, c1) = self.blocks[self.block];
        let n = TILE.min(self.grid.height - self.next.min(self.grid.height));
        (c0, c1, self.next, self.next + n)
    }

    /// The blocks the next step reads that the readers do not hold, each with its input.
    pub fn needs(&mut self) -> Vec<(u32, BlockNeed)> {
        if self.done() {
            return Vec::new();
        }
        let regions = self.regions(self.rect());
        let mut out = Vec::new();
        for (k, r) in regions {
            out.extend(self.inputs[k].needs(r).into_iter().map(|n| (k as u32, n)));
        }
        out
    }

    /// The blocks [`OpsJob::needs`] named, with their bytes: decoded on the
    /// job's threads and kept; each JPEG block's stream comes back for the
    /// host's pixels ([`OpsJob::put_pixels`]).
    pub fn put_all(
        &mut self,
        blocks: Vec<(u32, BlockNeed, Vec<u8>)>,
    ) -> Result<Vec<(u32, BlockNeed, Vec<u8>)>, String> {
        let items: Vec<(u32, BlockNeed, Vec<u8>, Option<Layout>)> = blocks
            .into_iter()
            .map(|(k, need, bytes)| {
                let layout = self
                    .inputs
                    .get(k as usize)
                    .and_then(|i| i.reader.layout_for(&need));
                (k, need, bytes, layout)
            })
            .collect();
        let decoded = par::map(
            self.threads,
            &items,
            &|(_, need, bytes, layout)| match layout {
                Some(l) => decode_block(l, need.index, bytes).map_err(|e| e.0),
                None => Err("Bu rasterin böyle bir bloğu yok.".to_owned()),
            },
        );
        let mut jpeg = Vec::new();
        for ((k, need, _, _), d) in items.iter().zip(decoded) {
            let input = self
                .inputs
                .get_mut(*k as usize)
                .ok_or("Böyle bir girdi raster yok.")?;
            if let Put::Jpeg(stream) = input.reader.keep(need, d?) {
                jpeg.push((*k, *need, stream));
            }
        }
        Ok(jpeg)
    }

    /// A JPEG block's pixels (`components` a pixel), decoded by the host.
    pub fn put_pixels(
        &mut self,
        input: u32,
        need: &BlockNeed,
        pixels: Vec<u8>,
        components: u32,
    ) -> Result<(), String> {
        self.inputs
            .get_mut(input as usize)
            .ok_or("Böyle bir girdi raster yok.")?
            .reader
            .put_pixels(need, pixels, components)
            .map_err(|e| e.0)
    }

    /// Whether every block of every strip (and pass) is done.
    pub fn done(&self) -> bool {
        if let Work::Hydro(h) = &self.work {
            return h.done();
        }
        if let Work::Distance(d) = &self.work {
            return d.done();
        }
        match &self.work {
            Work::Weights => return true,
            Work::Roc(r) => return r.done,
            _ => {}
        }
        let bounds = matches!(&self.work, Work::Histogram(h) if h.bounds_pass);
        !bounds && self.next >= self.grid.height
    }

    /// The share done, 0..1 (the histogram's first pass counts as a third).
    pub fn share(&self) -> f64 {
        let h = f64::from(self.grid.height.max(1));
        let blocks = self.blocks.len().max(1) as f64;
        let f = (f64::from(self.next.min(self.grid.height))
            + TILE as f64 * self.block as f64 / blocks)
            / h;
        let f = f.min(1.0);
        match &self.work {
            Work::Vector(v) => v.share(f),
            Work::Hydro(h) => h.share(f),
            Work::Distance(d) => d.share(f),
            Work::Roc(r) => r.share(f),
            Work::Weights => 1.0,
            Work::Histogram(h) if h.bounds_pass => f / 3.0,
            Work::Histogram(h) if h.two_pass => 1.0 / 3.0 + 2.0 * f / 3.0,
            _ => f,
        }
    }

    /// The result raster's bands and samples; none for a table.
    pub fn result(&self) -> Option<(u32, RasterSample)> {
        self.kind.map(|k| (k.bands(), k.sample))
    }

    /// The result's grid (its affine, width and height).
    pub fn grid(&self) -> Grid {
        self.grid
    }

    /// The result raster's look; none for a table.
    pub fn style(&self) -> Option<RasterStyle> {
        self.style.clone()
    }

    pub fn notes(&self) -> &Notes {
        &self.notes
    }

    /// Works out the next block; gives the bytes to append.
    pub fn step(&mut self) -> Result<Vec<u8>, String> {
        if self.done() {
            return Ok(Vec::new());
        }
        if let Work::Hydro(h) = &mut self.work
            && !h.reading()
        {
            // The DEM is in memory: a stage of the work, or the result's next strip.
            if h.emitting() {
                let kind = self.kind.ok_or("Sonuç rasteri yok.")?;
                let n = TILE.min(self.grid.height - self.next.min(self.grid.height));
                let strip = h.strip(n, kind.sample);
                self.next += n;
                let bytes = match self.out.as_mut() {
                    Some(out) => out.push(Rows::Any(&strip), n)?,
                    None => Vec::new(),
                };
                if h.done() {
                    self.notes.hydro = h.notes.clone();
                }
                return Ok(bytes);
            }
            h.step()?;
            if h.emitting() {
                self.next = 0;
            }
            if h.done() {
                self.notes.hydro = h.notes.clone();
            }
            return Ok(Vec::new());
        }
        if let Work::Distance(d) = &mut self.work
            && !d.reading()
        {
            // The raster is in memory: a stage of the work, or the result's next strip.
            if d.emitting() {
                let kind = self.kind.ok_or("Sonuç rasteri yok.")?;
                let n = TILE.min(self.grid.height - self.next.min(self.grid.height));
                let strip = d.strip(n, kind.sample);
                self.next += n;
                let bytes = match self.out.as_mut() {
                    Some(out) => out.push(Rows::Any(&strip), n)?,
                    None => Vec::new(),
                };
                if d.done() {
                    self.notes.distance = d.notes.clone();
                }
                return Ok(bytes);
            }
            d.step()?;
            if d.emitting() {
                self.next = 0;
            }
            if d.done() {
                self.notes.distance = d.notes.clone();
            }
            return Ok(Vec::new());
        }
        let rect = self.rect();
        let (c0, c1, y0, y1) = rect;
        let n = y1 - y0;
        let bw = (c1 - c0) as usize;
        let regions = self.regions(rect);
        if matches!(self.work, Work::Distance(_)) {
            // The cost raster (or the sources') and the surface read onto the grid.
            let mut views: Vec<Option<View>> = vec![None; self.inputs.len()];
            for (k, r) in regions {
                views[k] = Some(self.inputs[k].view(r, self.threads)?);
            }
            if let Work::Distance(d) = &mut self.work {
                d.block(
                    rect,
                    views.first().and_then(Option::as_ref),
                    views.get(1).and_then(Option::as_ref),
                );
            }
        } else if matches!(self.work, Work::Vector(_) | Work::Hydro(_)) {
            // The vectorizing and hydrology tools read the file's samples a cell at a time.
            let raw = match regions.first() {
                Some(&(k, r)) => self.inputs[k].raw(r)?,
                None => Raw::empty(),
            };
            match &mut self.work {
                Work::Vector(v) => v.block(rect, &raw)?,
                Work::Hydro(h) => h.block(rect, &raw),
                _ => unreachable!(),
            }
        } else if let Some(kind) = self.kind {
            let mut views: Vec<Option<View>> = vec![None; self.inputs.len()];
            for (k, r) in regions {
                views[k] = Some(self.inputs[k].view(r, self.threads)?);
            }
            let b = kind.bands() as usize;
            let mut vals = vec![f64::NAN; n as usize * bw * b];
            self.raster_block(rect, &views, &mut vals)?;
            let w = self.grid.width as usize;
            let strip = self.strip.get_or_insert_with(|| {
                Samples::filled(kind.sample, n as usize * w * b, kind.fill())
            });
            self.notes.empty_cells +=
                put_block(strip, &vals, (c0 as usize, bw, w, b), &kind, self.threads);
        } else {
            let mut views: Vec<Option<View>> = vec![None; self.inputs.len()];
            for (k, r) in regions {
                views[k] = Some(self.inputs[k].view(r, self.threads)?);
            }
            self.table_block(rect, &views);
        }
        self.block += 1;
        if self.block < self.blocks.len() {
            return Ok(Vec::new());
        }
        self.block = 0;
        if let Work::Vector(v) = &mut self.work {
            v.strip_done(y0, n)?;
        }
        self.next += n;
        if self.next >= self.grid.height
            && let Work::Hydro(h) = &mut self.work
        {
            // The DEM read: its work steps from here, no strip written yet.
            h.read_done();
            self.strip = None;
            return Ok(Vec::new());
        }
        if self.next >= self.grid.height
            && let Work::Distance(d) = &mut self.work
        {
            d.read_done()?;
            self.strip = None;
            return Ok(Vec::new());
        }
        if self.next >= self.grid.height
            && let Work::Roc(r) = &mut self.work
        {
            // The presence read: every cell is counted against it in a second pass, or the absence is.
            if r.second {
                r.second_done();
            } else if r.first_done()? {
                self.next = 0;
            }
            return Ok(Vec::new());
        }
        if self.next >= self.grid.height
            && let Work::Vector(v) = &mut self.work
            && let Some(g) = v.pass_done()?
        {
            // A capture's next pass: a wider window.
            self.grid = g;
            self.next = 0;
            self.blocks = self.plan();
        }
        let bytes = match (self.strip.take(), self.out.as_mut()) {
            (Some(strip), Some(out)) => out.push(Rows::Any(&strip), n)?,
            _ => Vec::new(),
        };
        if self.next >= self.grid.height
            && let Work::Histogram(h) = &mut self.work
            && h.bounds_pass
        {
            // The bounds known, the counting pass reads the strips again.
            h.bounds_pass = false;
            h.out.lo = h.lo;
            h.out.hi = h.hi;
            h.out.valid = 0;
            h.out.empty = 0;
            self.next = 0;
        }
        Ok(bytes)
    }

    /// A raster result's block: rows on the threads, each cell's bands (alpha last) as float64s.
    fn raster_block(
        &mut self,
        (c0, c1, y0, y1): (u32, u32, u32, u32),
        views: &[Option<View>],
        vals: &mut [f64],
    ) -> Result<(), String> {
        let kind = self.kind.ok_or("Sonuç rasteri yok.")?;
        let b = kind.bands() as usize;
        let bw = (c1 - c0) as usize;
        let grid = self.grid;
        let inputs = &self.inputs;
        let threads = self.threads;
        let row_len = bw * b;
        let empty = View::default();
        let view = |k: usize| views.get(k).and_then(Option::as_ref).unwrap_or(&empty);
        match &self.work {
            Work::Calc { calc, maps } => {
                let area = {
                    let [_, a, bb, _, c, d] = grid.affine;
                    (a * d - bb * c).abs()
                };
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let refs = calc.refs();
                    let mut cols: Vec<Vec<f64>> = vec![vec![0.0; bw]; refs.len()];
                    for (k, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + k) as u32;
                        for (f, &(input, band)) in refs.iter().enumerate() {
                            sample_row(
                                (&grid, &inputs[input].affine),
                                maps[input],
                                view(input),
                                band,
                                (c0, j),
                                Sampling::Nearest,
                                &mut cols[f],
                            );
                        }
                        let slices: Vec<&[f64]> = cols.iter().map(Vec::as_slice).collect();
                        calc.row(&slices, &grid.affine, (c0, j), area, row);
                    }
                });
            }
            Work::Reclass {
                band,
                rules,
                bounds,
                keep,
            } => {
                let map = mapping(&grid, &inputs[0].affine);
                let v = view(0);
                let unmatched = AtomicU64::new(0);
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut missed = 0u64;
                    for (k, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + k) as u32;
                        sample_row(
                            (&grid, &inputs[0].affine),
                            map,
                            v,
                            *band,
                            (c0, j),
                            Sampling::Nearest,
                            row,
                        );
                        for o in row.iter_mut() {
                            *o = match reclass::apply(rules, *bounds, *o) {
                                Some(new) => new.unwrap_or(f64::NAN),
                                None => {
                                    missed += 1;
                                    if *keep { *o } else { f64::NAN }
                                }
                            };
                        }
                    }
                    unmatched.fetch_add(missed, Ordering::Relaxed);
                });
                self.notes.cells += unmatched.into_inner();
            }
            Work::Clip { areas, off } => {
                let (ox, oy) = *off;
                let values = kind.values as usize;
                let near = areas.strip(oy + y0, oy + y1);
                let src = view(0);
                let map = mapping(&grid, &inputs[0].affine);
                let inside = AtomicU64::new(0);
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let (mut spans, mut cuts, mut merged) =
                        (Vec::<Span>::new(), Vec::new(), Vec::new());
                    let mut band_row = vec![0.0; bw];
                    let mut n_in = 0u64;
                    for (k, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + k) as u32;
                        areas.row(oy + j, &near, &mut spans, &mut cuts);
                        union(&spans, &mut merged);
                        row.fill(f64::NAN);
                        for bnd in 0..values {
                            sample_row(
                                (&grid, &inputs[0].affine),
                                map,
                                src,
                                bnd,
                                (c0, j),
                                Sampling::Nearest,
                                &mut band_row,
                            );
                            for &(a, z) in &merged {
                                // Source columns a..z are result columns a − ox .. z − ox.
                                let (lo, hi) = (a.max(ox + c0), z.min(ox + c1));
                                for col in lo..hi.max(lo) {
                                    let q = (col - ox - c0) as usize;
                                    row[q * b + bnd] = band_row[q];
                                    if bnd == 0 {
                                        n_in += 1;
                                    }
                                }
                            }
                        }
                        if kind.alpha {
                            for q in 0..bw {
                                let any = (0..values).any(|bnd| !row[q * b + bnd].is_nan());
                                row[q * b + values] = if any { 255.0 } else { 0.0 };
                            }
                        }
                    }
                    inside.fetch_add(n_in, Ordering::Relaxed);
                });
                self.notes.cells += inside.into_inner();
            }
            Work::Mosaic {
                overlap,
                sampling,
                maps,
            } => {
                let values = kind.values as usize;
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut bufs: Vec<Vec<f64>> = vec![vec![0.0; bw]; inputs.len()];
                    for (k, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + k) as u32;
                        for bnd in 0..values {
                            for (q, buf) in bufs.iter_mut().enumerate() {
                                match views.get(q).and_then(Option::as_ref) {
                                    Some(v) => sample_row(
                                        (&grid, &inputs[q].affine),
                                        maps[q],
                                        v,
                                        bnd,
                                        (c0, j),
                                        *sampling,
                                        buf,
                                    ),
                                    None => buf.fill(f64::NAN),
                                }
                            }
                            for i in 0..bw {
                                row[i * b + bnd] = combine(*overlap, bufs.iter().map(|buf| buf[i]));
                            }
                        }
                        if kind.alpha {
                            for i in 0..bw {
                                let any = (0..values).any(|bnd| !row[i * b + bnd].is_nan());
                                row[i * b + values] = if any { 255.0 } else { 0.0 };
                            }
                        }
                    }
                });
            }
            Work::Resample { method, ru, rv } => {
                let values = kind.values as usize;
                let src = view(0);
                let (ru, rv) = (*ru, *rv);
                let map = Mapping::Affine;
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut buf = vec![0.0; bw];
                    let mut scratch = resample::Scratch::default();
                    for (k, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + k) as u32;
                        for bnd in 0..values {
                            match method {
                                Method::Point(s) => {
                                    sample_row(
                                        (&grid, &inputs[0].affine),
                                        map,
                                        src,
                                        bnd,
                                        (c0, j),
                                        *s,
                                        &mut buf,
                                    );
                                    for i in 0..bw {
                                        row[i * b + bnd] = buf[i];
                                    }
                                }
                                m => {
                                    for i in 0..bw {
                                        row[i * b + bnd] = resample::area_value(
                                            src,
                                            bnd,
                                            (c0 + i as u32, j),
                                            (ru, rv),
                                            *m,
                                            &mut scratch,
                                        );
                                    }
                                }
                            }
                        }
                        if kind.alpha {
                            for i in 0..bw {
                                let any = (0..values).any(|bnd| !row[i * b + bnd].is_nan());
                                row[i * b + values] = if any { 255.0 } else { 0.0 };
                            }
                        }
                    }
                });
            }
            Work::Focal { focal, band } => {
                let src = view(0);
                let region = Region {
                    x0: src.x,
                    y0: src.y,
                    w: src.w as usize,
                    h: src.h as usize,
                    v: src.bands.get(*band).map_or(&[][..], Vec::as_slice),
                };
                let input = &inputs[0];
                focal.block(
                    &region,
                    (input.width, input.height),
                    (c0, c1, y0, y1 - y0),
                    threads,
                    vals,
                );
            }
            Work::Cells {
                band,
                stat,
                ignore,
                maps,
            } => {
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut bufs: Vec<Vec<f64>> = vec![vec![0.0; bw]; inputs.len()];
                    let mut values = Vec::with_capacity(inputs.len());
                    for (k, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + k) as u32;
                        for (q, buf) in bufs.iter_mut().enumerate() {
                            match views.get(q).and_then(Option::as_ref) {
                                Some(v) => sample_row(
                                    (&grid, &inputs[q].affine),
                                    maps[q],
                                    v,
                                    *band,
                                    (c0, j),
                                    Sampling::Nearest,
                                    buf,
                                ),
                                None => buf.fill(f64::NAN),
                            }
                        }
                        for (i, o) in row.iter_mut().enumerate() {
                            values.clear();
                            let mut missing = false;
                            for buf in &bufs {
                                let v = buf[i];
                                if v.is_nan() {
                                    missing = true;
                                } else {
                                    values.push(v);
                                }
                            }
                            *o = if missing && !*ignore {
                                f64::NAN
                            } else {
                                cell_stat(*stat, &mut values)
                            };
                        }
                    }
                });
            }
            Work::Suit { cell, band, maps } => {
                // Ağırlıklı çakıştırma's unmatched, off the scale and restricted cells, Bulanık çakıştırma's invalid.
                let counts: [AtomicU64; 4] = Default::default();
                par::rows(threads, vals, row_len, &|first, chunk: &mut [f64]| {
                    let mut bufs: Vec<Vec<f64>> = vec![vec![0.0; bw]; inputs.len()];
                    let mut mu = Vec::with_capacity(inputs.len());
                    let mut met = [0u64; 4];
                    for (k, row) in chunk.chunks_mut(row_len).enumerate() {
                        let j = y0 + (first + k) as u32;
                        for (q, buf) in bufs.iter_mut().enumerate() {
                            match views.get(q).and_then(Option::as_ref) {
                                Some(v) => sample_row(
                                    (&grid, &inputs[q].affine),
                                    maps[q],
                                    v,
                                    *band,
                                    (c0, j),
                                    Sampling::Nearest,
                                    buf,
                                ),
                                None => buf.fill(f64::NAN),
                            }
                        }
                        for (i, o) in row.iter_mut().enumerate() {
                            *o = suit_cell(cell, &bufs, i, &mut mu, &mut met);
                        }
                    }
                    for (c, m) in counts.iter().zip(met) {
                        c.fetch_add(m, Ordering::Relaxed);
                    }
                });
                let [unmatched, outside, restricted, invalid] = counts.map(AtomicU64::into_inner);
                let n = &mut self.notes.suit;
                n.unmatched += unmatched;
                n.outside += outside;
                n.restricted += restricted;
                n.invalid += invalid;
            }
            Work::Zonal { .. }
            | Work::Histogram(_)
            | Work::Vector(_)
            | Work::Hydro(_)
            | Work::Distance(_)
            | Work::Roc(_)
            | Work::Weights => {}
        }
        Ok(())
    }

    /// A table's block: the zones' sums or the histogram's counts, row by row in order.
    fn table_block(&mut self, (c0, c1, y0, y1): (u32, u32, u32, u32), views: &[Option<View>]) {
        let empty = View::default();
        let src = views.first().and_then(Option::as_ref).unwrap_or(&empty);
        let grid = self.grid;
        let input_affine = self.inputs[0].affine;
        let threads = self.threads;
        let bw = (c1 - c0) as usize;
        let rows: Vec<u32> = (y0..y1).collect();
        match &mut self.work {
            Work::Roc(r) => {
                let map = mapping(&grid, &input_affine);
                let band = r.band;
                r.block((c0, c1), &rows, threads, &|j, out: &mut [f64]| {
                    sample_row(
                        (&grid, &input_affine),
                        map,
                        src,
                        band,
                        (c0, j),
                        Sampling::Nearest,
                        out,
                    )
                });
            }
            Work::Zonal {
                areas,
                band,
                zones,
                values,
                ..
            } => {
                let near = areas.strip(y0, y1);
                let keep = values.is_some();
                let map = mapping(&grid, &input_affine);
                let band = *band;
                let areas = &*areas;
                // Each row's sums by zone, then joined in the rows' order.
                let made: Vec<Vec<(u32, Moments, Vec<f64>)>> = par::map(threads, &rows, &|&j| {
                    let mut spans = Vec::new();
                    let mut cuts = Vec::new();
                    areas.row(j, &near, &mut spans, &mut cuts);
                    let mut line = vec![0.0; bw];
                    sample_row(
                        (&grid, &input_affine),
                        map,
                        src,
                        band,
                        (c0, j),
                        Sampling::Nearest,
                        &mut line,
                    );
                    let mut out: Vec<(u32, Moments, Vec<f64>)> = Vec::new();
                    for s in &spans {
                        let (a, z) = (s.i0.max(c0), s.i1.min(c1));
                        if a >= z {
                            continue;
                        }
                        if out.last().is_none_or(|l| l.0 != s.object) {
                            out.push((s.object, Moments::default(), Vec::new()));
                        }
                        let Some(entry) = out.last_mut() else {
                            continue;
                        };
                        for i in a..z {
                            let v = line[(i - c0) as usize];
                            if !v.is_nan() {
                                entry.1.push(v);
                                if keep {
                                    entry.2.push(v);
                                }
                            }
                        }
                    }
                    out
                });
                for row in made {
                    for (z, m, vs) in row {
                        zones[z as usize].merge(&m);
                        if let Some(all) = values.as_mut() {
                            all[z as usize].extend(vs);
                        }
                    }
                }
            }
            Work::Histogram(h) => {
                let map = mapping(&grid, &input_affine);
                let band = h.band;
                if h.bounds_pass {
                    let made: Vec<(f64, f64, u64, u64)> = par::map(threads, &rows, &|&j| {
                        let mut line = vec![0.0; bw];
                        sample_row(
                            (&grid, &input_affine),
                            map,
                            src,
                            band,
                            (c0, j),
                            Sampling::Nearest,
                            &mut line,
                        );
                        let (mut lo, mut hi, mut valid, mut empty) =
                            (f64::INFINITY, f64::NEG_INFINITY, 0, 0);
                        for &v in &line {
                            if v.is_nan() {
                                empty += 1;
                            } else {
                                valid += 1;
                                lo = lo.min(v);
                                hi = hi.max(v);
                            }
                        }
                        (lo, hi, valid, empty)
                    });
                    for (lo, hi, valid, empty) in made {
                        h.lo = h.lo.min(lo);
                        h.hi = h.hi.max(hi);
                        h.out.valid += valid;
                        h.out.empty += empty;
                    }
                } else {
                    let (lo, hi, bins) = (h.lo, h.hi, h.bins);
                    let made: Vec<(Vec<u64>, u64, u64, u64, u64)> =
                        par::map(threads, &rows, &|&j| {
                            let mut line = vec![0.0; bw];
                            sample_row(
                                (&grid, &input_affine),
                                map,
                                src,
                                band,
                                (c0, j),
                                Sampling::Nearest,
                                &mut line,
                            );
                            let mut counts = vec![0u64; bins as usize];
                            let (mut below, mut above, mut valid, mut empty) = (0, 0, 0, 0);
                            for &v in &line {
                                if v.is_nan() {
                                    empty += 1;
                                    continue;
                                }
                                valid += 1;
                                if v < lo {
                                    below += 1;
                                } else if v > hi {
                                    above += 1;
                                } else {
                                    counts[bin_of(v, lo, hi, bins)] += 1;
                                }
                            }
                            (counts, below, above, valid, empty)
                        });
                    for (counts, below, above, valid, empty) in made {
                        for (t, c) in h.out.counts.iter_mut().zip(counts) {
                            *t += c;
                        }
                        h.out.below += below;
                        h.out.above += above;
                        h.out.valid += valid;
                        h.out.empty += empty;
                    }
                }
            }
            _ => {}
        }
    }

    /// The result: the GeoTIFF's last pieces, the zones' figures or the histogram.
    pub fn finish(self) -> Result<OpsFinished, String> {
        if !self.done() {
            return Err("Çözümleme bitmeden bırakıldı.".into());
        }
        match self.work {
            Work::Zonal {
                zones,
                values,
                stat,
                cell_area,
                ..
            } => {
                let mut values = values;
                Ok(OpsFinished::Zones(
                    zones
                        .into_iter()
                        .enumerate()
                        .map(|(z, m)| {
                            let value = match stat {
                                Stat::Area => Some(m.n as f64 * cell_area),
                                s if s.orders() => {
                                    values.as_mut().and_then(|all| order_stat(&mut all[z], s))
                                }
                                s => m.stat(s),
                            };
                            ZoneFigures { moments: m, value }
                        })
                        .collect(),
                ))
            }
            Work::Histogram(h) => Ok(OpsFinished::Histogram(h.out)),
            Work::Roc(r) => Ok(OpsFinished::Roc(r.finish()?)),
            Work::Weights => Ok(OpsFinished::Weights),
            Work::Vector(v) => Ok(OpsFinished::Features(v.finish()?)),
            Work::Hydro(mut h) if h.tool().raster(RasterSample::F32).is_none() => {
                Ok(OpsFinished::Features(h.finish()?))
            }
            Work::Distance(mut d) if !d.tool().raster() => Ok(OpsFinished::Features(d.finish()?)),
            _ => {
                let out = self.out.ok_or("Sonuç rasteri yok.")?;
                let (tail, header) = out.finish()?;
                Ok(OpsFinished::Raster { tail, header })
            }
        }
    }
}

/// A suitability result's look (docs/adr/0237): Spektral, stretched over
/// `manual`'s range (else the band's least to its most); read by the
/// nearest cell for classes.
fn suit_style(manual: Option<(f64, f64)>, nearest: bool) -> RasterStyle {
    let mut s = ramp_style("Spektral");
    if let Some((lo, hi)) = manual {
        s.stretch = RasterStretch::Manual;
        s.min = Some(lo);
        s.max = Some(hi);
    }
    if nearest {
        s.resampling = RasterResampling::Nearest;
    }
    s
}

/// Several inputs on their common grid (docs/adr/0237 §2): the grid, the band (every input has it), the mappings.
fn suit_inputs(inputs: &[Input], band: u32) -> Result<(Grid, usize, Vec<Mapping>), String> {
    let b = band_index(&inputs[0], band)?;
    for i in inputs {
        band_index(i, band)?;
    }
    let grid = common_grid(inputs)?;
    let maps = inputs.iter().map(|i| mapping(&grid, &i.affine)).collect();
    Ok((grid, b, maps))
}

/// A suitability cell from its inputs' values (`bufs[k][i]`); `met` counts
/// what it met: [unmatched, off the scale, restricted, outside 0–1].
#[inline]
fn suit_cell(
    cell: &SuitCell,
    bufs: &[Vec<f64>],
    i: usize,
    mu: &mut Vec<f64>,
    met: &mut [u64; 4],
) -> f64 {
    match cell {
        SuitCell::Membership(m) => {
            let x = bufs[0][i];
            if x.is_nan() { f64::NAN } else { m.of(x) }
        }
        SuitCell::Fuzzy(op) => {
            mu.clear();
            let (mut empty, mut invalid) = (false, false);
            for buf in bufs {
                let v = buf[i];
                if v.is_nan() {
                    empty = true;
                } else if !(0.0..=1.0).contains(&v) {
                    invalid = true;
                } else {
                    mu.push(v);
                }
            }
            if invalid {
                met[3] += 1;
            }
            if empty || invalid {
                f64::NAN
            } else {
                op.combine(mu)
            }
        }
        SuitCell::Sum(w) => {
            let mut v = 0.0;
            for (buf, wk) in bufs.iter().zip(w) {
                let x = buf[i];
                if x.is_nan() {
                    return f64::NAN;
                }
                v += wk * x;
            }
            v
        }
        SuitCell::Overlay(o) => {
            let (mut sum, mut restricted) = (0i64, false);
            for (k, buf) in bufs.iter().enumerate() {
                match o.scaled(k, buf[i]) {
                    Scaled::Value(s) => sum += s * o.weights[k],
                    Scaled::Restricted => restricted = true,
                    Scaled::Empty => return f64::NAN,
                    Scaled::Unmatched => {
                        met[0] += 1;
                        return f64::NAN;
                    }
                    Scaled::Outside => {
                        met[1] += 1;
                        return f64::NAN;
                    }
                }
            }
            if restricted {
                met[2] += 1;
                (o.lo - 1) as f64
            } else {
                o.class_of(sum) as f64
            }
        }
    }
}

/// The overlapping inputs' values of a cell, in the inputs' order, combined (§7).
fn combine(overlap: Overlap, values: impl Iterator<Item = f64>) -> f64 {
    let mut out = f64::NAN;
    let (mut sum, mut n) = (0.0, 0u32);
    for v in values {
        if v.is_nan() {
            continue;
        }
        match overlap {
            Overlap::Top => return v,
            Overlap::Bottom => out = v,
            Overlap::Mean => {
                sum += v;
                n += 1;
            }
            Overlap::Min => {
                if out.is_nan() || v < out {
                    out = v;
                }
            }
            Overlap::Max => {
                if out.is_nan() || v > out {
                    out = v;
                }
            }
        }
    }
    if overlap == Overlap::Mean {
        return if n > 0 { sum / f64::from(n) } else { f64::NAN };
    }
    out
}

/// A cell's statistic over the inputs' values (§9), each from what it needs.
fn cell_stat(stat: Stat, values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    match stat {
        Stat::Count => values.len() as f64,
        Stat::Sum | Stat::Mean => {
            let s = values
                .iter()
                .fold(crate::dd::Dd::ZERO, |s, &v| s.add_f64(v));
            if stat == Stat::Sum {
                s.value() + 0.0
            } else {
                s.div_f64(values.len() as f64).value() + 0.0
            }
        }
        Stat::Min | Stat::Max | Stat::Range => {
            let (mut lo, mut hi) = (values[0], values[0]);
            for &v in &values[1..] {
                if v < lo {
                    lo = v;
                }
                if v > hi {
                    hi = v;
                }
            }
            match stat {
                Stat::Min => lo + 0.0,
                Stat::Max => hi + 0.0,
                _ => hi - lo,
            }
        }
        s if s.orders() => order_stat(values, s).unwrap_or(f64::NAN),
        s => {
            let mut m = Moments::default();
            for &v in values.iter() {
                m.push(v);
            }
            m.stat(s).unwrap_or(f64::NAN)
        }
    }
}

/// A value's interval: ⌊(x − lo) / (hi − lo) · n⌋ held to 0..n − 1 (§11).
#[inline]
pub fn bin_of(v: f64, lo: f64, hi: f64, bins: u32) -> usize {
    if hi <= lo {
        return 0;
    }
    let k = ((v - lo) / (hi - lo) * f64::from(bins)).floor();
    (k.max(0.0) as usize).min(bins as usize - 1)
}

/// A block's values into the strip: columns `c0..c0 + bw` of every row,
/// each value as the result's type holds it, an empty one as its nodata
/// (an alpha band's as 0); rows on the threads. The cells left empty.
fn put_block(
    strip: &mut Samples,
    vals: &[f64],
    (c0, bw, w, b): (usize, usize, usize, usize),
    kind: &Kind,
    threads: usize,
) -> u64 {
    let fill = kind.fill();
    let alpha = kind.alpha.then_some(kind.values as usize);
    let empty = AtomicU64::new(0);
    macro_rules! write {
        ($dst:expr, $conv:expr) => {{
            let conv = $conv;
            let (fill_t, zero_t) = (conv(fill), conv(0.0));
            par::rows(threads, $dst, w * b, &|first, chunk| {
                let mut lacking = 0u64;
                for (k, row) in chunk.chunks_mut(w * b).enumerate() {
                    let src = &vals[(first + k) * bw * b..(first + k + 1) * bw * b];
                    let dst = &mut row[c0 * b..(c0 + bw) * b];
                    for (s, d) in src.chunks(b).zip(dst.chunks_mut(b)) {
                        let mut any = false;
                        for q in 0..b {
                            let v = s[q];
                            let is_alpha = alpha == Some(q);
                            d[q] = if v.is_nan() {
                                if is_alpha { zero_t } else { fill_t }
                            } else {
                                any |= !is_alpha;
                                conv(v)
                            };
                        }
                        lacking += u64::from(!any);
                    }
                }
                empty.fetch_add(lacking, Ordering::Relaxed);
            });
        }};
    }
    let k = kind.sample;
    match strip {
        Samples::U8(v) => write!(v.as_mut_slice(), |x: f64| stored(k, x) as u8),
        Samples::I8(v) => write!(v.as_mut_slice(), |x: f64| stored(k, x) as i8),
        Samples::U16(v) => write!(v.as_mut_slice(), |x: f64| stored(k, x) as u16),
        Samples::I16(v) => write!(v.as_mut_slice(), |x: f64| stored(k, x) as i16),
        Samples::U32(v) => write!(v.as_mut_slice(), |x: f64| stored(k, x) as u32),
        Samples::I32(v) => write!(v.as_mut_slice(), |x: f64| stored(k, x) as i32),
        Samples::F32(v) => write!(v.as_mut_slice(), |x: f64| x as f32),
        Samples::F64(v) => write!(v.as_mut_slice(), |x: f64| x),
    }
    empty.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bins_and_combinations() {
        assert_eq!(bin_of(0.0, 0.0, 10.0, 5), 0);
        assert_eq!(bin_of(2.0, 0.0, 10.0, 5), 1);
        assert_eq!(bin_of(10.0, 0.0, 10.0, 5), 4);
        assert_eq!(bin_of(3.0, 3.0, 3.0, 5), 0);
        let v = [f64::NAN, 4.0, 2.0, 8.0];
        assert_eq!(combine(Overlap::Top, v.into_iter()), 4.0);
        assert_eq!(combine(Overlap::Bottom, v.into_iter()), 8.0);
        assert_eq!(combine(Overlap::Mean, v.into_iter()), 14.0 / 3.0);
        assert_eq!(combine(Overlap::Min, v.into_iter()), 2.0);
        assert_eq!(combine(Overlap::Max, v.into_iter()), 8.0);
        assert!(combine(Overlap::Top, [f64::NAN].into_iter()).is_nan());
    }
}

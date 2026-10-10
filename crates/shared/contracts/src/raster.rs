//! Raster layers (docs/adr/0204 §2, §4): an orthophoto, a scanned sheet or a
//! digital elevation model placed by an affine transform from its pixels to
//! the drawing, its samples in a GeoTIFF, PNG or JPEG the object links
//! (`file`) or the project embeds (`asset`), shown by its style. The samples
//! are never in the drawing; only where they are and how they are shown.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::EntityBase;

/// The least and most opacity a raster is drawn with.
pub const MIN_RASTER_OPACITY: f64 = 0.1;
pub const MAX_RASTER_OPACITY: f64 = 1.0;
/// The most pixels a raster may have on a side.
pub const MAX_RASTER_SIDE: u32 = 4_000_000;
/// The most bands a raster may have.
pub const MAX_RASTER_BANDS: u32 = 255;
/// The longest a linked file's path may be, in letters.
pub const MAX_RASTER_PATH: usize = 4096;
/// The longest a NetCDF variable's name may be, in letters (docs/adr/0243 §6).
pub const MAX_DATASET_NAME: usize = 256;
/// The most slice dimensions a dataset may have, and values a dimension.
pub const MAX_DATASET_DIMS: usize = 8;
pub const MAX_DIM_VALUES: usize = 100_000;
/// The ramps a single band may be coloured with (docs/adr/0204 §4), in the menu's order.
pub const RASTER_RAMPS: [&str; 6] = [
    "Gri",
    "Arazi",
    "Spektral",
    "Viridis",
    "Mavi-kırmızı",
    "Sıcaklık",
];
/// Gölgeli kabartma's light when the style names none: from the north-west, 45° up, heights as they are.
pub const DEFAULT_AZIMUTH: f64 = 315.0;
pub const DEFAULT_ALTITUDE: f64 = 45.0;
pub const DEFAULT_Z_FACTOR: f64 = 1.0;
/// Yüzde gerdirme's bounds (docs/adr/0204 §4).
pub const PERCENT_LOW: f64 = 0.02;
pub const PERCENT_HIGH: f64 = 0.98;

/// A band's samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RasterSample {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    F64,
}

/// Names as the files and the wire hold them, both ways.
macro_rules! named {
    ($t:ty { $($v:ident => $n:literal),+ $(,)? }) => {
        impl $t {
            /// Every value, in the contract's order.
            pub const ALL: &'static [$t] = &[$(<$t>::$v),+];

            /// The name files and the wire hold.
            pub fn name(self) -> &'static str {
                match self {
                    $(<$t>::$v => $n),+
                }
            }

            pub fn from_name(name: &str) -> Option<$t> {
                match name {
                    $($n => Some(<$t>::$v)),+,
                    _ => None,
                }
            }
        }
    };
}

named!(RasterSample { U8 => "u8", I8 => "i8", U16 => "u16", I16 => "i16", U32 => "u32", I32 => "i32", F32 => "f32", F64 => "f64" });
named!(RasterRender { Rgb => "rgb", Gray => "gray", Palette => "palette", Ramp => "ramp", Hillshade => "hillshade", RampShade => "rampShade" });
named!(RasterStretch { None => "none", MinMax => "minMax", Percent => "percent", Manual => "manual" });
named!(RasterResampling { Bilinear => "bilinear", Nearest => "nearest" });

impl RasterSample {
    /// The name the window shows.
    pub fn label(self) -> &'static str {
        match self {
            RasterSample::U8 => "8 bit",
            RasterSample::I8 => "8 bit işaretli",
            RasterSample::U16 => "16 bit",
            RasterSample::I16 => "16 bit işaretli",
            RasterSample::U32 => "32 bit",
            RasterSample::I32 => "32 bit işaretli",
            RasterSample::F32 => "32 bit kayan nokta",
            RasterSample::F64 => "64 bit kayan nokta",
        }
    }

    /// Bytes a sample takes.
    pub fn bytes(self) -> usize {
        match self {
            RasterSample::U8 | RasterSample::I8 => 1,
            RasterSample::U16 | RasterSample::I16 => 2,
            RasterSample::U32 | RasterSample::I32 | RasterSample::F32 => 4,
            RasterSample::F64 => 8,
        }
    }

    /// Whether samples are floating point.
    pub fn float(self) -> bool {
        matches!(self, RasterSample::F32 | RasterSample::F64)
    }

    /// The least a sample may be (a resampled integer raster's empty value, docs/adr/0204 §6).
    pub fn least(self) -> f64 {
        match self {
            RasterSample::U8 | RasterSample::U16 | RasterSample::U32 => 0.0,
            RasterSample::I8 => -128.0,
            RasterSample::I16 => -32768.0,
            RasterSample::I32 => -2_147_483_648.0,
            RasterSample::F32 | RasterSample::F64 => f64::NAN,
        }
    }
}

/// How the bands are drawn (docs/adr/0204 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RasterRender {
    /// Three bands as red, green and blue, a fourth as alpha.
    Rgb,
    /// One band in grey.
    Gray,
    /// One band through the file's palette.
    Palette,
    /// One band through a colour ramp.
    Ramp,
    /// One band (heights) as a shaded relief.
    Hillshade,
    /// The ramp darkened by the shaded relief.
    RampShade,
}

impl RasterRender {
    /// How many bands the kind draws, and whether a fourth (alpha) may follow.
    pub fn bands(self) -> (usize, bool) {
        match self {
            RasterRender::Rgb => (3, true),
            _ => (1, false),
        }
    }

    /// Whether it colours a single band by a ramp.
    pub fn ramped(self) -> bool {
        matches!(self, RasterRender::Ramp | RasterRender::RampShade)
    }

    /// Whether it shades a single band as heights.
    pub fn shaded(self) -> bool {
        matches!(self, RasterRender::Hillshade | RasterRender::RampShade)
    }
}

/// How values are brought to colours (docs/adr/0204 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RasterStretch {
    /// As they are (8 bit).
    #[default]
    None,
    /// The band's least to its most.
    MinMax,
    /// Its 2nd to its 98th percentile.
    Percent,
    /// The style's `min` to `max`.
    Manual,
}

/// How a pixel between samples is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RasterResampling {
    #[default]
    Bilinear,
    Nearest,
}

/// A raster's look (docs/adr/0204 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RasterStyle {
    pub render: RasterRender,
    /// The bands drawn, from 1: three or four for `rgb`, one otherwise.
    pub bands: Vec<u32>,
    #[serde(default, skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<RasterStretch>", optional))]
    pub stretch: RasterStretch,
    /// `manual`'s least and most value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<f64>,
    /// One of `RASTER_RAMPS`; absent: Gri.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ramp: Option<String>,
    /// The ramp turned round.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub invert: bool,
    /// Gölgeli kabartma's light: degrees from north, clockwise (0–360).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub azimuth: Option<f64>,
    /// Its height above the horizon, degrees (0–90).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub altitude: Option<f64>,
    /// Heights multiplied by it (over 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z_factor: Option<f64>,
    /// The value shown as nothing, in place of the file's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub nodata: Option<f64>,
    #[serde(default, skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<RasterResampling>", optional))]
    pub resampling: RasterResampling,
    /// A mesh's edges drawn over it in this colour (`#RRGGBB`; docs/adr/0243 §5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edges: Option<String>,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

impl RasterStyle {
    /// The look as the contract's JSON text (the geometry core carries it so).
    pub fn to_json_text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// A look from the contract's JSON text; none when it is not one.
    pub fn from_json_text(text: &str) -> Option<RasterStyle> {
        serde_json::from_str(text).ok()
    }

    /// The ramp's name: the style's, else Gri.
    pub fn ramp_name(&self) -> &str {
        self.ramp.as_deref().unwrap_or(RASTER_RAMPS[0])
    }

    /// Gölgeli kabartma's light and scale: the style's, else the defaults.
    pub fn light(&self) -> (f64, f64, f64) {
        (
            self.azimuth.unwrap_or(DEFAULT_AZIMUTH),
            self.altitude.unwrap_or(DEFAULT_ALTITUDE),
            self.z_factor.unwrap_or(DEFAULT_Z_FACTOR),
        )
    }

    /// Why the style does not suit a raster of `bands` bands, in the
    /// commands' words; none when it does.
    pub fn problem(&self, bands: u32) -> Option<String> {
        let (need, alpha) = self.render.bands();
        let n = self.bands.len();
        if !(n == need || (alpha && n == need + 1)) {
            return Some(match self.render {
                RasterRender::Rgb => {
                    "RGB görünüş üç bant ister (dördüncüsü alfa olabilir).".to_owned()
                }
                _ => "Bu görünüş tek bant ister.".to_owned(),
            });
        }
        if let Some(b) = self.bands.iter().find(|&&b| b == 0 || b > bands) {
            return Some(format!(
                "Rasterin {bands} bandı var; {b}. bant gösterilemez."
            ));
        }
        if self.stretch == RasterStretch::Manual {
            match (self.min, self.max) {
                (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() && lo < hi => {}
                _ => {
                    return Some(
                        "Elle gerdirmenin en küçüğü ve en büyüğü sonlu sayılar olmalı, en küçük en büyükten küçük.".to_owned(),
                    );
                }
            }
        }
        if let Some(r) = &self.ramp
            && !RASTER_RAMPS.contains(&r.as_str())
        {
            return Some(format!(
                "“{r}” diye bir renk rampası yok; {} rampalarından biri seçilmeli.",
                RASTER_RAMPS.join(", ")
            ));
        }
        let (az, alt, z) = self.light();
        let lit = az.is_finite()
            && (0.0..=360.0).contains(&az)
            && alt.is_finite()
            && (0.0..=90.0).contains(&alt)
            && z.is_finite()
            && z > 0.0;
        if !lit {
            return Some(
                "Gölgeli kabartmanın ışığı 0–360° doğrultudan, 0–90° yükseklikten gelmeli; yükseklik çarpanı sıfırdan büyük olmalı.".to_owned(),
            );
        }
        if self.nodata.is_some_and(|v| !v.is_finite()) {
            return Some("Nodata değeri sonlu bir sayı olmalı.".to_owned());
        }
        if let Some(c) = &self.edges
            && !(c.len() == 7
                && c.starts_with('#')
                && c[1..].chars().all(|h| h.is_ascii_hexdigit()))
        {
            return Some(format!(
                "Ağ çizgilerinin rengi #RRGGBB olmalı; “{c}” verildi."
            ));
        }
        None
    }
}

/// One of a dataset's slice dimensions (docs/adr/0243 §6): its name, the
/// index shown, its values (coordinates; moments in milliseconds since 1970
/// when `time`) and units.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetDim {
    pub name: String,
    pub index: u32,
    pub values: Vec<f64>,
    /// The values are a CF time axis's moments.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub time: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<String>,
}

/// A NetCDF variable a raster shows (docs/adr/0243 §6): a CF grid's or a
/// UGRID mesh's dataset, one slice of its other dimensions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RasterDataset {
    /// The variable's name in the file (a vector's x component).
    pub variable: String,
    /// A vector's y component: its magnitude is shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vector: Option<String>,
    /// The mesh topology it lies on (a UGRID file); absent: a CF grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mesh: Option<String>,
    /// Its slice dimensions in the file's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DatasetDim>>", optional))]
    pub dims: Vec<DatasetDim>,
    /// The step shown follows the time slider (docs/adr/0243 §7).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub follow_time: bool,
}

impl RasterDataset {
    /// The dataset as the contract's JSON text (the geometry core carries it so).
    pub fn to_json_text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// A dataset from the contract's JSON text; none when it is not one.
    pub fn from_json_text(text: &str) -> Option<RasterDataset> {
        serde_json::from_str(text).ok()
    }

    /// The slice: each dimension's index.
    pub fn slice(&self) -> Vec<u32> {
        self.dims.iter().map(|d| d.index).collect()
    }

    /// Its time dimension, when it has one.
    pub fn time_dim(&self) -> Option<usize> {
        self.dims.iter().position(|d| d.time)
    }

    /// The slice shown at moment `t` of the time slider (§7): the time
    /// dimension's last step at or before `t`; none before its first step.
    /// Without `followTime` or a moment: the slice as it is.
    pub fn slice_at(&self, t: Option<f64>) -> Option<Vec<u32>> {
        let mut out = self.slice();
        let (Some(k), true, Some(t)) = (self.time_dim(), self.follow_time, t) else {
            return Some(out);
        };
        let values = &self.dims[k].values;
        let n = values.partition_point(|&v| v <= t);
        out[k] = u32::try_from(n.checked_sub(1)?).ok()?;
        Some(out)
    }

    /// Why the dataset does not make a raster's, in the commands' words; none when it does.
    pub fn problem(&self) -> Option<String> {
        let name_ok = |s: &str| {
            !s.trim().is_empty()
                && s.chars().count() <= MAX_DATASET_NAME
                && !s.chars().any(char::is_control)
        };
        if !name_ok(&self.variable)
            || self.vector.as_deref().is_some_and(|v| !name_ok(v))
            || self.mesh.as_deref().is_some_and(|v| !name_ok(v))
        {
            return Some(format!(
                "Veri setinin değişken adları 1 ile {MAX_DATASET_NAME} harf arasında olmalı ve denetim karakteri içermemeli."
            ));
        }
        if self.dims.len() > MAX_DATASET_DIMS {
            return Some(format!(
                "Veri setinin en çok {MAX_DATASET_DIMS} dilim boyutu olabilir."
            ));
        }
        for d in &self.dims {
            if !name_ok(&d.name) {
                return Some("Dilim boyutunun adı boş olamaz.".into());
            }
            if d.values.is_empty()
                || d.values.len() > MAX_DIM_VALUES
                || !d.values.iter().all(|v| v.is_finite())
            {
                return Some(format!(
                    "“{}” boyutunun 1 ile {MAX_DIM_VALUES} arasında sonlu değeri olmalı.",
                    d.name
                ));
            }
            if d.index as usize >= d.values.len() {
                return Some(format!(
                    "“{}” boyutunun {} değeri var; gösterilen {}. değer yok.",
                    d.name,
                    d.values.len(),
                    u64::from(d.index) + 1
                ));
            }
            if d.time && d.values.windows(2).any(|w| w[1] < w[0]) {
                return Some(format!(
                    "“{}” zaman boyutunun değerleri azalmamalı.",
                    d.name
                ));
            }
        }
        if self.dims.iter().filter(|d| d.time).count() > 1 {
            return Some("Veri setinin en çok bir zaman boyutu olabilir.".into());
        }
        if self.follow_time && self.time_dim().is_none() {
            return Some("Zaman sürgüsünü izlemek için veri setinin zaman boyutu olmalı.".into());
        }
        None
    }
}

/// A raster (docs/adr/0204 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RasterEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub raster: RasterFields,
}

/// What places and shows a raster: the raster object's own fields, and its
/// geometry in the commands (`EntityGeometry::Raster`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RasterFields {
    /// `[x₀, a, b, y₀, c, d]`: column i and row j (pixel corners, 0, 0 the
    /// upper left) lie at x = x₀ + a·i + b·j, y = y₀ + c·i + d·j (GDAL's
    /// geotransform order). Invertible.
    pub affine: [f64; 6],
    /// Its size in pixels.
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub sample: RasterSample,
    /// The project library's asset its file is (embedded); exactly one of
    /// `asset` and `file` is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// The file it shows (linked): absolute, or relative to the drawing's folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// An HTTP or HTTPS address read by ranges (a COG; docs/adr/0207 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub url: Option<String>,
    /// The file's coordinate system; 0: the file named none and the user took the project's.
    pub srid: u32,
    pub style: RasterStyle,
    /// 0.1 to 1; absent: opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opacity: Option<f64>,
    /// A NetCDF file's variable and slice it shows (docs/adr/0243 §6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dataset: Option<RasterDataset>,
}

impl RasterFields {
    /// The affine's linear part's determinant: area of a pixel, signed.
    pub fn determinant(&self) -> f64 {
        let [_, a, b, _, c, d] = self.affine;
        a * d - b * c
    }

    /// Where pixel corner (i, j) lies.
    pub fn at(&self, i: f64, j: f64) -> (f64, f64) {
        let [x0, a, b, y0, c, d] = self.affine;
        (x0 + a * i + b * j, y0 + c * i + d * j)
    }

    /// The pixel (column, row; fractional) a point lies on, none when the
    /// affine does not invert.
    pub fn pixel_of(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let [x0, a, b, y0, c, d] = self.affine;
        let det = a * d - b * c;
        if !(det.is_finite() && det != 0.0) {
            return None;
        }
        let (dx, dy) = (x - x0, y - y0);
        Some(((d * dx - b * dy) / det, (a * dy - c * dx) / det))
    }

    /// Its four corners, counter-clockwise when the affine keeps the turn:
    /// lower left, lower right, upper right, upper left (pixel corners
    /// (0, h), (w, h), (w, 0), (0, 0)).
    pub fn corners(&self) -> [(f64, f64); 4] {
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        [
            self.at(0.0, h),
            self.at(w, h),
            self.at(w, 0.0),
            self.at(0.0, 0.0),
        ]
    }

    /// Why the fields do not make a raster, in the commands' words; none when
    /// they do. The asset's being in the project's library is the command's to check.
    pub fn problem(&self) -> Option<String> {
        if !self.affine.iter().all(|v| v.is_finite()) {
            return Some("Rasterin dönüşümü sonlu altı sayı olmalı.".into());
        }
        let det = self.determinant();
        if !(det.is_finite() && det != 0.0) {
            return Some(
                "Rasterin dönüşümü tersinmiyor: pikselin iki kenarı aynı doğrultuda ya da sıfır."
                    .into(),
            );
        }
        if !(1..=MAX_RASTER_SIDE).contains(&self.width)
            || !(1..=MAX_RASTER_SIDE).contains(&self.height)
        {
            return Some(format!(
                "Rasterin genişliği ve yüksekliği 1 ile {MAX_RASTER_SIDE} piksel arasında olmalı."
            ));
        }
        if !(1..=MAX_RASTER_BANDS).contains(&self.bands) {
            return Some(format!(
                "Rasterin 1 ile {MAX_RASTER_BANDS} arasında bandı olmalı."
            ));
        }
        let asset = self
            .asset
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let file = self
            .file
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let url = self.url.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let given =
            usize::from(asset.is_some()) + usize::from(file.is_some()) + usize::from(url.is_some());
        let named = usize::from(self.asset.is_some())
            + usize::from(self.file.is_some())
            + usize::from(self.url.is_some());
        if given == 0 {
            return Some("Rasterin kaynağı yok: gömülü varlığın kimliğini (asset), bağlı dosyanın yolunu (file) ya da adresini (url) verin.".into());
        }
        if given != 1 || named != 1 {
            return Some("Rasterin kaynağı ya gömülü varlık (asset), ya bağlı dosya (file), ya adres (url) olmalı; yalnız biri.".into());
        }
        if let Some(p) = url.and_then(crate::url_problem) {
            return Some(p);
        }
        if file
            .is_some_and(|f| f.chars().count() > MAX_RASTER_PATH || f.chars().any(char::is_control))
        {
            return Some(format!(
                "Bağlı dosyanın yolu en çok {MAX_RASTER_PATH} harf olmalı ve denetim karakteri içermemeli."
            ));
        }
        if let Some(p) = self.style.problem(self.bands) {
            return Some(p);
        }
        if let Some(d) = &self.dataset {
            if let Some(p) = d.problem() {
                return Some(p);
            }
            if self.bands != 1 {
                return Some("Veri setini gösteren rasterin tek bandı olur.".into());
            }
        }
        if self.style.edges.is_some() && self.dataset.as_ref().is_none_or(|d| d.mesh.is_none()) {
            return Some("Ağ çizgileri yalnız mesh'te çizilir.".into());
        }
        if let Some(o) = self.opacity
            && !(o.is_finite() && (MIN_RASTER_OPACITY..=MAX_RASTER_OPACITY).contains(&o))
        {
            return Some(format!(
                "Rasterin donukluğu {MIN_RASTER_OPACITY} ile {MAX_RASTER_OPACITY} arasında olmalı; {o} verildi."
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_serdes() {
        fn check<T: Serialize + Copy>(all: &[T], name: impl Fn(T) -> &'static str) {
            for &v in all {
                assert_eq!(
                    serde_json::to_value(v).unwrap(),
                    serde_json::Value::from(name(v))
                );
            }
        }
        check(RasterSample::ALL, RasterSample::name);
        check(RasterRender::ALL, RasterRender::name);
        check(RasterStretch::ALL, RasterStretch::name);
        check(RasterResampling::ALL, RasterResampling::name);
        assert_eq!(
            RasterRender::from_name("rampShade"),
            Some(RasterRender::RampShade)
        );
        assert_eq!(RasterSample::from_name("u9"), None);
    }
}

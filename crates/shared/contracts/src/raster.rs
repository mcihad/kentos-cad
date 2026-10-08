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
    /// The file's coordinate system; 0: the file named none and the user took the project's.
    pub srid: u32,
    pub style: RasterStyle,
    /// 0.1 to 1; absent: opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opacity: Option<f64>,
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
        match (asset, file, &self.asset, &self.file) {
            (Some(_), None, _, None) | (None, Some(_), None, _) => {}
            (Some(_), Some(_), ..) => {
                return Some("Rasterin kaynağı ya gömülü varlık (asset) ya bağlı dosya (file) olmalı, ikisi birden değil.".into());
            }
            _ => {
                return Some("Rasterin kaynağı yok: gömülü varlığın kimliğini (asset) ya da bağlı dosyanın yolunu (file) verin.".into());
            }
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

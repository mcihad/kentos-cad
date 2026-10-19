//! A raster's georeference record and `cad.rasters.georef` v1 (docs/adr/0518; plan/gis.md `CBS-29`): Raster oturt
//! (docs/adr/0204 §6) as a persistent, reproducible operation. The record is kept on the raster (`RasterFields::georef`,
//! `.kcad` schema 88): the method, the control and check points as given (pixels of the source, targets in `srid`), the
//! source the pixels are of (its file, embedded asset or address and its SHA-256), the control points' m0, the check
//! points' error, and for a resampled raster the output grid and resampling. Run again with the same record, the
//! command gives the same transform and the same grid; a resampling never writes over its source.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{MAX_RASTER_PATH, MAX_RASTER_SIDE, RasterFields, RasterResampling};

pub const CAD_RASTERS_GEOREF: &str = "cad.rasters.georef";
pub const CAD_RASTERS_GEOREF_VERSION: u32 = 1;

/// The most points a record keeps (control, check and left out together).
pub const MAX_GEOREF_POINTS: usize = 1000;
/// The longest a point's name may be, in letters.
pub const MAX_GEOREF_NAME: usize = 64;

/// A transform from the source's pixels to the drawing (docs/adr/0204 §6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GeorefMethod {
    Helmert,
    #[default]
    Affine,
    Projective,
    Poly2,
    Poly3,
    ThinPlate,
}

impl GeorefMethod {
    pub const ALL: [GeorefMethod; 6] = [
        GeorefMethod::Helmert,
        GeorefMethod::Affine,
        GeorefMethod::Projective,
        GeorefMethod::Poly2,
        GeorefMethod::Poly3,
        GeorefMethod::ThinPlate,
    ];

    /// Its name in the JSON and the geometry core (`ops::georef::Method::from_name`).
    pub fn name(self) -> &'static str {
        match self {
            GeorefMethod::Helmert => "helmert",
            GeorefMethod::Affine => "affine",
            GeorefMethod::Projective => "projective",
            GeorefMethod::Poly2 => "poly2",
            GeorefMethod::Poly3 => "poly3",
            GeorefMethod::ThinPlate => "thinPlate",
        }
    }

    pub fn from_name(name: &str) -> Option<GeorefMethod> {
        GeorefMethod::ALL.into_iter().find(|m| m.name() == name)
    }

    /// The fewest control points it is fitted to.
    pub fn need(self) -> usize {
        match self {
            GeorefMethod::Helmert => 2,
            GeorefMethod::Affine | GeorefMethod::ThinPlate => 3,
            GeorefMethod::Projective => 4,
            GeorefMethod::Poly2 => 6,
            GeorefMethod::Poly3 => 10,
        }
    }

    /// Whether it places the raster by its affine (Helmert, affine) rather than resampling it.
    pub fn places(self) -> bool {
        matches!(self, GeorefMethod::Helmert | GeorefMethod::Affine)
    }

    /// As the window names it.
    pub fn title(self) -> &'static str {
        match self {
            GeorefMethod::Helmert => "Helmert",
            GeorefMethod::Affine => "Afin",
            GeorefMethod::Projective => "Projektif",
            GeorefMethod::Poly2 => "Polinom 2",
            GeorefMethod::Poly3 => "Polinom 3",
            GeorefMethod::ThinPlate => "İnce plaka",
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_bilinear(r: &RasterResampling) -> bool {
    *r == RasterResampling::Bilinear
}

/// A control or check point: a pixel of the source (column, row; pixel corners, 0, 0 the upper left corner of the
/// upper left pixel, as GDAL's GCPs) and where it is (`target`, in the record's `srid`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeorefPoint {
    /// What the user calls it (a mark on the sheet, a survey point's name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub pixel: [f64; 2],
    /// Its east and north (a geographic system's longitude and latitude, degrees).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub target: [f64; 2],
    /// A check point: kept out of the fit; its error says how good the fit is where no control point is.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub check: bool,
    /// Left out (Kullan off): neither fits nor checks; its residual is still shown.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub off: bool,
}

impl GeorefPoint {
    /// A control point: fitted to.
    pub fn control(&self) -> bool {
        !self.check && !self.off
    }

    /// Why it is not a point; none when it is.
    pub fn problem(&self) -> Option<String> {
        if !self.pixel.iter().chain(&self.target).all(|v| v.is_finite()) {
            return Some("Noktanın pikseli ve hedefi sonlu sayılar olmalı.".into());
        }
        if self.check && self.off {
            return Some(
                "Kullanılmayan nokta denetim noktası olamaz; denetim noktası Kullan'da kalır."
                    .into(),
            );
        }
        if let Some(n) = &self.name
            && (n.trim().is_empty()
                || n.trim() != n
                || n.chars().count() > MAX_GEOREF_NAME
                || n.chars().any(char::is_control))
        {
            return Some(format!(
                "Noktanın adı kırpılmış, boş olmayan ve en çok {MAX_GEOREF_NAME} harf olmalı."
            ));
        }
        None
    }
}

/// The raster the points' pixels are of: what it read and its size.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeorefSource {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub height: u32,
    /// The project library's raster it was (embedded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// The file it was (linked), as kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// The address it was read from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub url: Option<String>,
    /// Its bytes' SHA-256 (64 lowercase hex digits) when they were read: the revision the points belong to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub digest: Option<String>,
}

/// A resampled raster's grid: north-up, its corner on a multiple of its pixel (docs/adr/0204 §6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeorefOutput {
    /// `[x₀, s, 0, y₀, 0, −s]`.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub affine: [f64; 6],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub height: u32,
    /// The pixel's side, s.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pixel: f64,
    #[serde(default, skip_serializing_if = "is_bilinear")]
    #[cfg_attr(feature = "ts", ts(as = "Option<RasterResampling>", optional))]
    pub resampling: RasterResampling,
    /// The written file's SHA-256 when it was written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub digest: Option<String>,
}

/// How a raster was georeferenced (docs/adr/0518 §1): enough to do it again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RasterGeoref {
    pub method: GeorefMethod,
    /// The control, check and left-out points, as given.
    pub points: Vec<GeorefPoint>,
    /// The targets' coordinate system (EPSG) when it is not the project's; they were carried to the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    pub source: GeorefSource,
    /// The control points' m0 (the project's length unit); none for the thin plate or without redundancy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub m0: Option<f64>,
    /// The check points' root mean square error and largest error (the project's length unit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub check_rmse: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub check_max: Option<f64>,
    /// The output grid, for a resampled raster (projective, polynomials, thin plate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub output: Option<GeorefOutput>,
}

fn digest_problem(d: &Option<String>) -> Option<String> {
    d.as_ref()
        .filter(|d| {
            d.len() != 64
                || !d
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .map(|_| "SHA-256 özeti 64 küçük onaltılık hane olmalı.".to_owned())
}

impl RasterGeoref {
    /// The points fitted to, checking and left out.
    pub fn counts(&self) -> (usize, usize) {
        let control = self.points.iter().filter(|p| p.control()).count();
        let checks = self.points.iter().filter(|p| p.check).count();
        (control, checks)
    }

    /// Why the record is not one; none when it is. Its numbers are not solved again here: a record is what the
    /// command wrote.
    pub fn problem(&self) -> Option<String> {
        if self.points.is_empty() || self.points.len() > MAX_GEOREF_POINTS {
            return Some(format!(
                "Kaydın 1 ile {MAX_GEOREF_POINTS} arasında noktası olmalı."
            ));
        }
        if let Some(p) = self.points.iter().find_map(GeorefPoint::problem) {
            return Some(p);
        }
        let (control, checks) = self.counts();
        if control < self.method.need() {
            return Some(format!(
                "{} en az {} kontrol noktası ister; kayıtta {control} var.",
                self.method.title(),
                self.method.need()
            ));
        }
        if self.srid == Some(0) {
            return Some("Hedeflerin sistemi verildiyse bir EPSG kodu olmalı.".into());
        }
        let s = &self.source;
        if !(1..=MAX_RASTER_SIDE).contains(&s.width) || !(1..=MAX_RASTER_SIDE).contains(&s.height) {
            return Some(format!(
                "Kaynağın genişliği ve yüksekliği 1 ile {MAX_RASTER_SIDE} piksel arasında olmalı."
            ));
        }
        let given = usize::from(s.asset.is_some())
            + usize::from(s.file.is_some())
            + usize::from(s.url.is_some());
        if given != 1 {
            return Some("Kaynak ya gömülü varlık (asset), ya bağlı dosya (file), ya adres (url) olmalı; yalnız biri.".into());
        }
        if [&s.asset, &s.file, &s.url].into_iter().flatten().any(|t| {
            t.trim().is_empty()
                || t.chars().count() > MAX_RASTER_PATH
                || t.chars().any(char::is_control)
        }) {
            return Some(format!(
                "Kaynağın adı boş olmamalı, en çok {MAX_RASTER_PATH} harf olmalı."
            ));
        }
        if let Some(p) = digest_problem(&s.digest) {
            return Some(p);
        }
        let fine = |v: Option<f64>| v.is_none_or(|v| v.is_finite() && v >= 0.0);
        if !(fine(self.m0) && fine(self.check_rmse) && fine(self.check_max)) {
            return Some("m0 ve denetim hataları sonlu ve eksi olmayan sayılar olmalı.".into());
        }
        if self.check_rmse.is_some() != self.check_max.is_some()
            || self.check_rmse.is_some() != (checks > 0)
        {
            return Some("Denetim noktası varken denetimin karesel ortalama hatası ve en büyük hatası birlikte yazılır, yokken hiçbiri.".into());
        }
        match (&self.output, self.method.places()) {
            (Some(_), true) => {
                return Some(format!(
                    "{} rasteri yeniden örneklemez; çıktı ızgarası yazılmaz.",
                    self.method.title()
                ));
            }
            (None, false) => {
                return Some(format!(
                    "{} rasteri yeniden örnekler; çıktı ızgarası yazılır.",
                    self.method.title()
                ));
            }
            (Some(o), false) => {
                let [x0, a, b, y0, c, d] = o.affine;
                if !(o.pixel.is_finite() && o.pixel > 0.0 && x0.is_finite() && y0.is_finite())
                    || a != o.pixel
                    || b != 0.0
                    || c != 0.0
                    || d != -o.pixel
                {
                    return Some("Çıktı ızgarası kuzeye bakan ve pikseli kare olmalı: [x₀, s, 0, y₀, 0, −s].".into());
                }
                if !(1..=MAX_RASTER_SIDE).contains(&o.width)
                    || !(1..=MAX_RASTER_SIDE).contains(&o.height)
                {
                    return Some(format!(
                        "Çıktının genişliği ve yüksekliği 1 ile {MAX_RASTER_SIDE} piksel arasında olmalı."
                    ));
                }
                if let Some(p) = digest_problem(&o.digest) {
                    return Some(p);
                }
            }
            (None, true) => {}
        }
        None
    }

    /// Why the record is not the raster's: a resampled raster is its output grid, a placed one its source's size.
    pub fn misfit(&self, r: &RasterFields) -> Option<String> {
        match &self.output {
            Some(o) if o.affine != r.affine || o.width != r.width || o.height != r.height => {
                Some("Kaydın çıktı ızgarası rasterin dönüşümü ve boyu değil.".into())
            }
            None if self.source.width != r.width || self.source.height != r.height => {
                Some("Kaydın kaynağının boyu rasterin boyu değil.".into())
            }
            _ => None,
        }
    }
}

/// The resampled raster the host wrote itself (the desktop, on a thread of its own, by the plan's grid): a file, or
/// an embedded raster of the project's library. The command reads it and refuses one that is not the plan's grid.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeorefMade {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
}

/// Input of `cad.rasters.georef` v1. Refusals, in this order: `invalid_uid`, `entity_not_found`, `not_a_raster`,
/// `layer_locked`, `invalid_control_point`, `unknown_srid`, `target_not_converted`, `too_few_control_points`,
/// `control_points_duplicate`, `control_points_collinear`, `georef_singular`, `invalid_pixel_size`, `georef_grid`,
/// `invalid_file_path`, `file_not_found`, `import_unreadable`, `unknown_asset`, `georef_output_mismatch`,
/// `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersGeoref {
    /// The raster.
    pub uid: String,
    pub method: GeorefMethod,
    /// Control, check and left-out points; the pixels are the source's (a resampled raster's record's source).
    pub points: Vec<GeorefPoint>,
    /// The targets' system (EPSG) when not the project's: they are carried to the project's system first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// A resampled output's pixel side; none: the transform's mean scale at the control points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pixel: Option<f64>,
    #[serde(default, skip_serializing_if = "is_bilinear")]
    #[cfg_attr(feature = "ts", ts(as = "Option<RasterResampling>", optional))]
    pub resampling: RasterResampling,
    /// Where a resampled linked raster is written (absolute, or relative to the drawing's folder); none: beside its
    /// source as `<ad>-oturtulmus.tif`. An embedded source's output is embedded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub output: Option<String>,
    /// The output the host resampled itself by the plan's grid; none: the command resamples.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub made: Option<GeorefMade>,
    /// The open drawing's own file, an absolute path, when it has one: relative paths are read from its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A point's residual (control, left out) or error (check): the transformed pixel less its target, in the project's
/// system; none beyond a projective's horizon.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GeorefResidual {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dx: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub d: Option<f64>,
    /// Its target in the project's system (carried from `srid`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub target: [f64; 2],
}

/// Output of `cad.rasters.georef` v1 (its plan's too: what it would write, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RastersGeoreffed {
    pub uid: String,
    /// Every point's, in the input's order.
    pub residuals: Vec<GeorefResidual>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub control: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub checks: u32,
    /// The raster as it is written: its affine, size, source and record.
    pub raster: RasterFields,
    /// The file a resampling wrote (or, planned, will write).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub written: Option<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

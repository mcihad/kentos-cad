//! `cad.orthophoto.check` v1 (plan/map-cad.md MAP-12, docs/adr/1480): Ortofoto doğruluk denetimi, read only. An
//! orthophoto (a raster of the drawing) checked against points measured on the ground (GNSS, a point layer) and the
//! same points read on the image (another point layer), matched by name: each point's error (image − ground), the
//! check points' KOH (RMSE) in X,Y and Z, their mean (a direction's bias) and largest error, apart from the points the
//! image was fitted to (its georeference's control points: they say how well it fits, not how true it is). The limits
//! come from a rule package, by default BÖHHBÜY's Article 91(2) (KOH_X,Y in cm ≤ −1.665·10⁻⁶·S² + 0.01745·S − 1.166,
//! KOH_Z ≤ 1.33·KOH_X,Y) and Article 54's ground sampling distance by scale, as data
//! (`catalog/v1/rules/ortofoto-bohhbuy-2018.json`), not as constants in code. Nothing is written; the image's
//! coordinates are never moved. The shared cases are `fixtures/commands/v1/cad.orthophoto.check.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ORTHOPHOTO_CHECK: &str = "cad.orthophoto.check";
pub const CAD_ORTHOPHOTO_CHECK_VERSION: u32 = 1;

/// A scale's largest ground sampling distance (metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoGsd {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub gsd: f64,
}

/// A rule package: who issues it, the document, when it holds, its release; KOH_X,Y's limit in centimetres as
/// `xy[0]·S² + xy[1]·S + xy[2]` (S the scale's denominator), KOH_Z's as `zFactor` × that, the largest ground sampling
/// distance by scale and the share it may vary by; and the largest shift of a point between two orthophotos as a
/// multiple of the ground sampling distance (`mosaic`, BÖHHBÜY Article 67(6); none: the package gives no such limit and
/// `cad.orthophoto.compare` refuses it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoRules {
    pub id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub release: u32,
    pub jurisdiction: String,
    pub source: String,
    pub effective: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub xy: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub z_factor: f64,
    pub gsd: Vec<OrthoGsd>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub gsd_tolerance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub mosaic: Option<f64>,
}

/// KentOS's default package: BÖHHBÜY 2018's Articles 54, 67(6) and 91.
pub fn ortho_rules_default() -> Option<OrthoRules> {
    serde_json::from_str(include_str!("../../../../catalog/v1/rules/ortofoto-bohhbuy-2018.json")).ok()
}

/// Input of `cad.orthophoto.check` v1: the orthophoto (a raster's persistent id), the layer of the points measured on
/// the ground and the layer of the same points read on the image, the attribute that names a point (none: `Ad`), the
/// scale's denominator (none: the project's plot scale) and the rule package (none: KentOS's BÖHHBÜY 2018).
///
/// Refusals: `invalid_uid`, `entity_not_found` (`raster`), `not_a_raster` (`raster`), `layer_not_found` (`known`,
/// `observed`), `crs_mismatch` (`raster`: its coordinate system is not the project's), `invalid_input` (`scale` 0;
/// `rules`: empty id or source, a limit not finite).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoCheck {
    pub raster: String,
    pub known: String,
    pub observed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<OrthoRules>,
}

/// A point: its name, its role (`check`: independent; `fit`: one the image's georeference was fitted to), its error
/// image − ground in east, north and (when both have one) height, and its horizontal error (metres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoPoint {
    pub name: String,
    pub role: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dx: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dy: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dz: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub d: f64,
}

/// A set of points' statistics (metres): how many, RMSE in east and north, KOH_X,Y = √(Σ(dx² + dy²) / 2n), the
/// radial RMSE, the mean error in east and north (bias), the largest horizontal error, and KOH_Z over the points with
/// heights on both sides (with their number).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoStats {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub n: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rmse_x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rmse_y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub koh_xy: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub rmse_r: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub mean_x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub mean_y: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_d: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub n_z: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub koh_z: Option<f64>,
}

/// Output of `cad.orthophoto.check` v1: the package (id, source), the scale, the image's ground sampling distance
/// (the square root of a pixel's area), the package's limit for it at that scale and whether it holds (none when the
/// package names no such scale), the matched points, the names on one side only, the check points' and the fit
/// points' statistics, the KOH limits (metres) and whether the check points hold to them (none without check points
/// or heights), whether KOH_X,Y is under half a pixel (finer than reading the image: the result may flatter it), and notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoChecked {
    pub rules: String,
    pub source: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scale: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub gsd: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub gsd_limit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub gsd_ok: Option<bool>,
    pub points: Vec<OrthoPoint>,
    pub unmatched: Vec<String>,
    pub check: OrthoStats,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fit: Option<OrthoStats>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit_xy: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit_z: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed_xy: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed_z: Option<bool>,
    pub below_pixel: bool,
    pub notes: Vec<String>,
}

pub const CAD_ORTHOPHOTO_COMPARE: &str = "cad.orthophoto.compare";
pub const CAD_ORTHOPHOTO_COMPARE_VERSION: u32 = 1;

/// Input of `cad.orthophoto.compare` v1 (plan/map-cad.md MAP-12, docs/adr/1780): two orthophotos (rasters' persistent
/// ids; the first the older, or the one the mosaic's neighbour is set against), the layers of the same points read on
/// each, and the layer of those points measured on the ground (none: the images are compared with each other only);
/// the attribute that names a point (none: `Ad`), the scale's denominator (none: the project's plot scale) and the rule
/// package (none: KentOS's BÖHHBÜY 2018).
///
/// Refusals: `invalid_uid`, `entity_not_found`, `not_a_raster` (`first`, `second`), `crs_mismatch` (a raster's
/// coordinate system is not the project's), `invalid_input` (one raster twice; `scale` 0; `rules`: empty id or source,
/// no positive `mosaic`), `layer_not_found` (`firstPoints`, `secondPoints`, `known`); the seam's (docs/adr/2460):
/// `invalid_input` (`seam.step` not above zero, `seam.window` even or outside 5..=255, `seam.search` outside 1..=64,
/// `seam.correlation` outside (0, 1]; `seam`: the images' pixels not the same size and unturned, no overlap), the file's
/// (`invalid_file_path`, `file_not_found`, `import_unreadable`; at `first` or `second`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoCompare {
    pub first: String,
    pub second: String,
    pub first_points: String,
    pub second_points: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub known: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rules: Option<OrthoRules>,
    /// The shift measured in the images themselves where they overlap (docs/adr/2460): no points read by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub seam: Option<OrthoSeamAsk>,
}

/// How the shift is measured in the overlap (docs/adr/2460): places every `step` metres across the overlap; at each,
/// the first image's patch of `window` × `window` pixels (odd) sought in the second within `search` pixels each way by
/// normalized cross-correlation, its peak refined to a fraction of a pixel by a parabola on its neighbours; a place
/// whose peak correlation is below `correlation` (the user's; none: every place counts) is not counted. `folder`: where
/// a raster's relative file is looked for (the drawing's folder). `line` (docs/adr/2860): a drawn seam line's persistent
/// id (a line or a polyline, docs/adr/3260 its arcs too, distances along them): the places are every `step` metres along
/// it from its start instead, those inside the overlap measured. `subpixel` (docs/adr/3260): how the peak is refined.
/// `resample` (docs/adr/3660): when the two images' pixels differ in size or the second is turned, the second is
/// resampled onto the first's grid by cubic convolution (Keys, a = −1/2: 16 pixel centres round each point) at every
/// pixel centre the patches use; the first, the grid, must face north. Not given, such images are refused.
/// `polygons` (docs/adr/4460): the two images' seamline polygons, as their mosaic keeps them (each the part of its image
/// the mosaic uses; either order): the places are every `step` metres along their shared boundary instead, each of its
/// parts from its start; `line` and `polygons` are not given together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoSeamAsk {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub step: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub window: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub search: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub correlation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub subpixel: Option<OrthoSubpixel>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub resample: bool,
    /// The two images' seamline polygons' persistent ids (docs/adr/4460): areas or closed polylines of straight edges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub polygons: Option<[String; 2]>,
}

/// How a place's peak is refined to a fraction of a pixel (docs/adr/3260): `parabola` (none given, docs/adr/2460) through
/// the peak and its two neighbours each way; `bicubic` by the second image itself resampled with cubic convolution
/// (Keys, a = −1/2), the correlation's highest sought on a grid of 1/8 pixel within half a pixel of the peak, then of
/// 1/64 pixel within 1/8 of it (Debella-Gilo and Kääb 2011: interpolating the image is the more accurate way).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OrthoSubpixel {
    Parabola,
    Bicubic,
}

/// A place of the overlap measured (docs/adr/2460): where (the first image's pixel centre, metres), the shift of the
/// second's match − the place in east and north and its length (metres), the peak correlation, the state (`ok`;
/// `low`: below the correlation asked; `edge`: the peak on the search's edge, the true one may lie beyond; `flat`: the
/// patch has no texture), and whether an `ok` place is over the limit; along a seam line (docs/adr/2860), its distance
/// along the line; along the seamline polygons' shared boundary (docs/adr/4460), its distance along its part and the
/// part's number.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoPatch {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub at: [f64; 2],
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub correlation: Option<f64>,
    pub state: String,
    pub over: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub station: Option<f64>,
    /// The shared boundary's part it is on, 0 the first (docs/adr/4460).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub part: Option<u32>,
}

/// The two images' seamline polygons' shared boundary measured along (docs/adr/4460): the polygons' persistent ids as
/// given; its parts, each the vertices (east, north) of a run of the first polygon's rings lying on the second's within
/// 1 mm (docs/adr/0200 §1), in the first's ring order; their length together, how many places they have every step and
/// how many of them lie outside the overlap (not measured).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoSeamShared {
    pub polygons: [String; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub parts: Vec<Vec<[f64; 2]>>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub places: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
}

/// The seam line measured along (docs/adr/2860): its persistent id and length, how many places it has every step and
/// how many of them lie outside the overlap (not measured).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoSeamLine {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub places: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
}

/// The overlap's measurement (docs/adr/2460): every place in order (north to south, west to east), the counts by state,
/// the shift's statistics over the `ok` places (their means are the systematic shift; none without one) and whether
/// every `ok` place holds to the limit (none without one).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoSeam {
    pub patches: Vec<OrthoPatch>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ok: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub low: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edge: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub flat: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shift: Option<OrthoStats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed: Option<bool>,
    /// The seam line measured along (docs/adr/2860); none: the overlap's grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line: Option<OrthoSeamLine>,
    /// The seamline polygons' shared boundary measured along (docs/adr/4460).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shared: Option<OrthoSeamShared>,
    /// The second image was resampled onto the first's grid (docs/adr/3660): their pixels differ in size or the second
    /// is turned, and `resample` was asked.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub resampled: bool,
    /// The first image, the grid, is turned (docs/adr/4060): the places, its patches and the match are in its own
    /// pixels, its affine taking them to the ground; the shifts are still east and north.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub turned: bool,
}

/// A point read on both images: its name, the second's place − the first's in east and north (metres), the distance,
/// and whether it is over the limit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthoPair {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dx: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dy: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub d: f64,
    pub over: bool,
}

/// Output of `cad.orthophoto.compare` v1: the package (id, source), the two images' ground sampling distances, the
/// limit of a point's shift (the package's `mosaic` × the coarser image's GSD, metres: BÖHHBÜY Article 67(6)'s ±2·GSD
/// between a mosaic's orthophotos, read by KentOS for two images of different pixels), the
/// points read on both, the names on one side only, the shift's statistics (its means are the systematic shift),
/// whether every point holds to the limit (none without points), each image's own check against the ground's points
/// (`cad.orthophoto.check`'s answer, when `known` is given), and notes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OrthophotoCompared {
    pub rules: String,
    pub source: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub gsd: [f64; 2],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub limit: f64,
    pub pairs: Vec<OrthoPair>,
    pub unmatched: Vec<String>,
    pub shift: OrthoStats,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub passed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub first_check: Option<OrthophotoChecked>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub second_check: Option<OrthophotoChecked>,
    /// The overlap measured in the images themselves (docs/adr/2460).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub seam: Option<OrthoSeam>,
    pub notes: Vec<String>,
}

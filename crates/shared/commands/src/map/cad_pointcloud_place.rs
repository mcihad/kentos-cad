//! `cad.pointclouds.place` v1 (plan/map-cad.md MAP-13, docs/adr/5100): Bulutu konumla. A cloud's files are given a
//! place in the drawing without writing them again, as a scan's pose sits beside its points: a rigid map (rotation and
//! translation) taken after the place they have, so a registration's or a network's matrix moves the scan as the
//! drawing shows it. Its points are then drawn, queried and measured there; Dönüşüm uygula alone writes them so. The
//! map is given as 16 numbers, an axis and an angle, or Euler angles about Z, Y′ and X″ in that order (R = Rz·Ry·Rx,
//! degrees); or the place is taken away. One undo step, “Bulutu konumla”. The shared cases are
//! `fixtures/commands/v1/cad.pointclouds.place.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_POINTCLOUDS_PLACE: &str = "cad.pointclouds.place";
pub const CAD_POINTCLOUDS_PLACE_VERSION: u32 = 1;

/// How the map is given (docs/adr/5100). A centre left out is the middle of the cloud's box in the drawing; a
/// translation left out is none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CloudPlaceBy {
    /// Matris: 16 numbers row by row (Tarama kaydı's and Tarama ağı's matrix), a rotation and a translation only.
    Matrix {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        matrix: Vec<f64>,
    },
    /// Eksen ve açı: a turn of `angle` degrees about `axis` (right-handed; its length does not matter) about `center`,
    /// then `translation` (docs/adr/4300).
    #[serde(rename_all = "camelCase")]
    AxisAngle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        axis: [f64; 3],
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        angle: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        center: Option<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        translation: Option<[f64; 3]>,
    },
    /// Euler açıları: `yaw` about Z, then `pitch` about the turned Y, then `roll` about the twice turned X, degrees,
    /// each right-handed (X east, Y north, Z up): R = Rz(yaw)·Ry(pitch)·Rx(roll); about `center`, then `translation`.
    #[serde(rename_all = "camelCase")]
    Euler {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        yaw: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        pitch: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        roll: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        center: Option<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        translation: Option<[f64; 3]>,
    },
    /// Konumu kaldır: the files where they say again.
    Remove,
}

/// Input of `cad.pointclouds.place` v1: the cloud's persistent id and the map.
///
/// Refusals, in this order: `invalid_uid`, `entity_not_found`, `not_a_pointcloud`, `layer_locked` (`uid`);
/// `not_finite`, `invalid_input` (`place`: 16 numbers, an affine map, rigid; an axis of no length); `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsPlace {
    pub uid: String,
    pub place: CloudPlaceBy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `cad.pointclouds.place` wrote: the map taken (16 numbers row by row; Konumu kaldır's is the inverse of the
/// first file's place, the move back), each file's place after it (none: where the file says), the cloud's box in the
/// drawing after it, how far the box's middle went (m) and the map's rotation angle (degrees); `changed` false when
/// nothing was written (taking away the place of a cloud with none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointCloudsPlaced {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub matrix: Vec<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub poses: Vec<Option<Vec<f64>>>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub bounds: [f64; 6],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub shift: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: f64,
    pub changed: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

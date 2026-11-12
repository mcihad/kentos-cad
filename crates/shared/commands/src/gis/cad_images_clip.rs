//! Resmi kırp as a product command (docs/adr/0192 §5, 0622 §9, TODOS.md `UX-14`). The caller gives the picture by its
//! persistent id and the boundary drawn in the drawing, a rectangle by two corners or a polygon by its corners, or
//! asks the clip taken away; the command cuts the boundary to the picture in the picture's own fractions with the
//! shared core (`ops::image::clip_of`) and writes it as `cad.entities.edit`'s `imageClip` writes, one undo step “Resmi
//! kırp”. The tool writes through it; Python and an agent ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Clips a picture.
pub const CAD_IMAGES_CLIP: &str = "cad.images.clip";
pub const CAD_IMAGES_CLIP_VERSION: u32 = 1;

/// The part of the picture shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ImageClipBy {
    /// Dikdörtgen: the rectangle of two opposite corners `a` and `b`, along the drawing's axes.
    Rectangle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
    /// Çokgen: the boundary's corners in the drawing, at least three, in either turn.
    Polygon {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        points: Vec<Vec2>,
    },
    /// Kaldır: the whole picture shown again.
    Remove,
}

/// `cad.images.clip`'s input.
///
/// Refusals, in this order: `not_finite`, `too_few_points`, `invalid_uid`, `entity_not_found`, `not_an_image`,
/// `layer_locked`, `clip_outside`, `invalid_revision`, `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImagesClip {
    /// The picture's persistent id.
    pub uid: String,
    pub clip: ImageClipBy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `cad.images.clip` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImagesClipped {
    pub uid: String,
    /// The part shown in the picture's own fractions (0,0 its lower left, 1,1 its upper right), counter-clockwise from
    /// its lowest corner; none: the whole picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub clip: Option<Vec<Vec2>>,
    /// Whether the picture changed: taking away the clip of a picture with none writes nothing.
    pub changed: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

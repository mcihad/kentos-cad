//! `cad.images.edit` v1 (docs/adr/0350 §1; plan/cad.md `CAD-REF-16`): an image's source changed. Göm reads linked
//! images' files into the project's library and makes them embedded; Yeniden bağla points images at another file. Each
//! is one undo step, whole or not at all; the frame, turn, clip and opacity stay.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_IMAGES_EDIT: &str = "cad.images.edit";
pub const CAD_IMAGES_EDIT_VERSION: u32 = 1;

/// The largest file an image is read from, in bytes (docs/adr/0192 §2).
pub const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

/// What `cad.images.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ImagesOperation {
    /// Göm: the images' files read into the project's library, the images made embedded; none given: every linked
    /// image of the drawing.
    Embed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        uids: Option<Vec<String>>,
    },
    /// Yeniden bağla: the images show `file` (absolute, or relative to the drawing's folder), linked.
    Relink { uids: Vec<String>, file: String },
}

/// Input of `cad.images.edit` v1. Refusals, in this order: `entity_not_found`, `not_an_image`, `layer_locked`,
/// `nothing_to_embed`, `invalid_file_path`, `file_not_found`, `import_unreadable`, `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImagesEdit {
    pub operation: ImagesOperation,
    /// The open drawing's own file, an absolute path, when it has one: relative paths are read from its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// An image's new source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImageSourced {
    pub uid: String,
    /// The library's image it now is (embedded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// The file it now shows (linked), as kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// Where the file was read from.
    pub read: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub width: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "px")))]
    pub height: u32,
}

/// Output of `cad.images.edit` v1 (its plan's too: what it would do, nothing written).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImagesEdited {
    /// The images changed, in the drawing's order.
    pub images: Vec<ImageSourced>,
    /// The library's images added (embedding a picture the library has adds none), by id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

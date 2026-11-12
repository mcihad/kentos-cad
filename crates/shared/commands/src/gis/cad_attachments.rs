//! `cad.attachments.inspect` (docs/adr/0442 §7): what each attachment of some objects is, read from its
//! bytes, writing nothing: its media type, size and SHA-256, and for a photo the moment and place its
//! EXIF says; an embedded photo from the project's library, a linked file from the disk (a relative path
//! from the folder given), a web address by its name only (it is not fetched).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::UID_TEXT;

pub const CAD_ATTACHMENTS_INSPECT: &str = "cad.attachments.inspect";
pub const CAD_ATTACHMENTS_INSPECT_VERSION: u32 = 1;

/// The most objects one call inspects.
pub const MOST_INSPECTED: usize = 1_000;

/// Input of `cad.attachments.inspect` v1.
///
/// Refusals: `invalid_count` (no object or more than 1 000), `invalid_uid`, `entity_not_found`,
/// `invalid_folder` (a folder that is not an absolute path).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AttachmentsInspect {
    /// The objects (lowercase UUID texts with hyphens), in the answer's order.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The drawing's folder, an absolute path: where a relative file path is read from. None: a
    /// relative path is not read (`missing`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub folder: Option<String>,
}

/// Where an attachment's bytes are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AttachmentSource {
    /// An image of the project's library.
    Asset,
    /// A file on the disk.
    File,
    /// A web address.
    Url,
}

/// What an attachment is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AttachmentFacts {
    pub name: String,
    pub source: AttachmentSource,
    /// The media type by the bytes (PNG, JPEG, GIF, TIFF, PDF, ZIP), else by the name's extension, else
    /// `application/octet-stream`; none for a web address whose name says nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mime: Option<String>,
    /// Bytes; none when they were not read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub size: Option<u64>,
    /// The bytes' SHA-256, lowercase hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sha256: Option<String>,
    /// When a photo was taken, as its EXIF says (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub taken: Option<String>,
    /// Where it was taken, WGS 84 degrees, as its EXIF says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub latitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub longitude: Option<f64>,
    /// An embedded photo not in the library, or a file not found or not read (a relative path without
    /// a folder too); written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub missing: bool,
}

/// An object's attachments, in their order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InspectedObject {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub attachments: Vec<AttachmentFacts>,
}

/// Output of `cad.attachments.inspect` v1, and what plan says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AttachmentsInspected {
    pub objects: Vec<InspectedObject>,
}

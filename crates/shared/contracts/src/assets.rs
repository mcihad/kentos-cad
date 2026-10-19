//! `cad.assets.check` v1 (docs/adr/0349 §2; plan/cad.md `CAD-REF-19`): every file the drawing links to — external
//! references, linked images, rasters, point clouds, linked tables, attachments and bound layers' sources — with where
//! it was found and whether it changed since it was read, in one structured answer for the desktop, Python and an AI
//! agent alike. Read-only: nothing is loaded, nothing written.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_ASSETS_CHECK: &str = "cad.assets.check";
pub const CAD_ASSETS_CHECK_VERSION: u32 = 1;

/// What a linked file is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AssetKind {
    /// An external reference's source drawing (docs/adr/0348).
    Xref,
    /// A linked image.
    Image,
    /// A linked raster.
    Raster,
    /// A point cloud's file.
    PointCloud,
    /// A linked table's file.
    Table,
    /// An object's attachment.
    Attachment,
    /// A bound layer's source (docs/adr/0503).
    Source,
}

/// Where a linked file was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AssetVia {
    /// At its path as kept.
    Path,
    /// At its relative path, from the open drawing's folder.
    Relative,
    /// In one of the project's search paths (an external reference only, docs/adr/0349 §1).
    SearchPath,
    /// Nowhere.
    None,
    /// An address: not looked at.
    Address,
    /// A database's table: not looked at.
    Database,
}

/// A linked file's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AssetState {
    /// Found, and the same as when read (or nothing kept to compare with).
    Ok,
    /// Found, and its SHA-256 is not the one kept.
    Changed,
    /// Not found.
    Missing,
    /// An external reference unloaded: its file is not looked at.
    Unloaded,
    /// An address.
    Remote,
    /// A database's table.
    Database,
}

/// Input of `cad.assets.check` v1. No refusals: an unreadable file is `missing`, an unknown kind none listed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AssetsCheck {
    /// The open drawing's file, an absolute path: relative paths and search paths are read from its folder. None: a
    /// relative path is not looked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    /// Only these kinds; none: every kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kinds: Option<Vec<AssetKind>>,
    /// Whether a file whose SHA-256 is kept is read to compare it (default yes); no: `changed` is never said, a large
    /// file is not read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub digests: Option<bool>,
}

/// One linked file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AssetStatus {
    pub kind: AssetKind,
    /// The object it belongs to (an image, raster, point cloud, table or an object with the attachment), by its
    /// lasting id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    /// The block definition it is (an external reference) or in (an object of a block's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<crate::BlockId>,
    /// The bound layer's source id (`kaynak-N`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    /// What it is called: an external reference's name, else the file's name.
    pub name: String,
    /// The path as kept (or the address).
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub relative: Option<String>,
    /// Where it was found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub found: Option<String>,
    pub via: AssetVia,
    pub state: AssetState,
    /// The SHA-256 kept, when one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub digest: Option<String>,
    /// The file's SHA-256 now, when it is not the one kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub current: Option<String>,
    /// The places looked in, in order, when it was not found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tried: Vec<String>,
}

/// How many of each state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AssetCounts {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ok: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub missing: u32,
    /// Unloaded, remote and database ones.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub other: u32,
}

/// Output of `cad.assets.check` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AssetsChecked {
    /// External references in the drawing's block order, then the objects' links in the drawing's order (an object's
    /// own file, then its attachments), then the blocks' objects' by block, then the bound layers' sources.
    pub assets: Vec<AssetStatus>,
    pub counts: AssetCounts,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// The longest list of search paths a project keeps, and the longest path.
pub const MAX_XREF_PATHS: usize = 64;
pub const MAX_XREF_PATH: usize = 1024;

/// A project's search paths as kept: trimmed, blank and repeated ones dropped, at most [`MAX_XREF_PATHS`], each at
/// most [`MAX_XREF_PATH`] letters.
pub fn sanitized_xref_paths(list: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in list {
        let p = p.trim().to_owned();
        if p.is_empty() || p.chars().count() > MAX_XREF_PATH || out.contains(&p) {
            continue;
        }
        out.push(p);
        if out.len() == MAX_XREF_PATHS {
            break;
        }
    }
    out
}

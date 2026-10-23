//! Paketle (docs/adr/0506, plan/gis.md CBS-36): the project with the files it uses, in one folder, so it opens on
//! another computer. The folder holds the project file and `veri/` with a copy of each bound GeoPackage and each
//! linked file (images, rasters with their world files, point clouds, linked tables, attachments); the copies are
//! named once each, the project's links rewritten to them: a linked file's path relative to the folder, a bound
//! layer's source the copy (its relative path `veri/…`, found again wherever the folder goes). A database's table,
//! an address and a missing file are not copied: their links stay and the answer says each (`package_left`). The
//! folder is made whole or not at all: written under a temporary name, every copy read back and its SHA-256
//! compared, the project file verified, then renamed into place. Passwords and tokens are never in the project, so
//! none is packed.
//!
//! The package also carries the external references' sources (docs/adr/0350 §4), byte for byte so their digests
//! hold, and `paket.json` (`kentos.package/1`): the project file, every copy and every link left out, the typefaces
//! drawn; a text style drawn in a substitute face is said as left out (KentOS draws only its own faces, which come with
//! it).
//!
//! The project's own data profiles and mappings (`profiller/<ad>.profil.json`, `<ad>.eslem.json` beside the project
//! file, docs/adr/0558) go too: byte for byte under `profiller/` of the package, which sits beside the project file
//! there as it did here, so the commands find them again (`from` is the project file's folder); a file that cannot be
//! read is left out, said.
//!
//! How the bound sources go is asked (docs/adr/0596): each kind, GeoPackages and database tables, copied, kept as
//! linked or embedded; a database table copied is written as a GeoPackage of the rows its link reads. What a licence
//! or an access rule keeps out is named in `leaveOut`: a link (its layer's objects go out with it), an object's files
//! or an external reference's source.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const FILE_PACKAGE: &str = "file.package";
pub const FILE_PACKAGE_VERSION: u32 = 1;

/// The longest a package's project file name may be, in letters.
pub const MAX_PACKAGE_NAME: usize = 255;

/// Input of `file.package` v1. Refusals, in order: `invalid_file_path` (`path` not absolute, its folder not there,
/// `name` not a file name, `from` not an absolute folder), `file_exists` (`path` is there and not an empty folder),
/// `invalid_input` (a `leaveOut` entry that names nothing), `source_unreachable`, `source_changed` and
/// `input_too_large` (a database table copied, docs/adr/0596 §2), `file_unwritable`. Warning: `package_left` (what
/// was not copied, each said).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PackageProject {
    /// The folder to make, an absolute path; it must not be there, or be an empty folder.
    pub path: String,
    /// The project file's name in it (`.kcad` added when missing); none: the drawing's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// The project file's folder, an absolute path: linked files written relative to it are read from there. None:
    /// a relative link is not found (the drawing was never saved).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    /// How the bound GeoPackages go (docs/adr/0596 §1): copied into `veri/` (none: the default), their links kept as
    /// they are, or embedded (the link removed, the layer keeps its objects as the project's own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geopackages: Option<PackageWay>,
    /// How the bound database tables go (docs/adr/0596 §1): their links kept (none: the default), each table copied
    /// into a GeoPackage in `veri/` (only the rows the link reads), or embedded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub databases: Option<PackageWay>,
    /// What is not to go into the package (docs/adr/0596 §3), each a bound layer's link (`kaynak-N`: the link and the
    /// layer's objects are taken out), an object's lasting id (its files are not copied, its links stay) or an
    /// external reference (`xref:` and its name: its source is not copied). It comes before `geopackages` and
    /// `databases`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub leave_out: Vec<String>,
}

/// How a bound layer's source goes into a package (docs/adr/0596 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PackageWay {
    /// Copied into `veri/`, the link the copy's: a GeoPackage byte for byte, a database table as a GeoPackage.
    Copy,
    /// The link as it is, nothing copied.
    Keep,
    /// The link removed in the package's project, the layer holding its objects as its own.
    Embed,
}

/// What a packed or left link is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PackagedKind {
    /// A bound layer's source.
    Source,
    /// A linked image or raster, or a point cloud's file.
    Picture,
    /// A raster's world file or auxiliary file beside it.
    Sidecar,
    /// A linked table's file.
    Table,
    /// An object's attachment.
    Attachment,
    /// An external reference's source drawing (docs/adr/0350 §4).
    Xref,
    /// A text style's typeface (left out only: KentOS's own faces come with it).
    Font,
    /// A data profile of the project, `profiller/<ad>.profil.json` (docs/adr/0558).
    Profile,
    /// An institution's mapping of the project, `profiller/<ad>.eslem.json` (docs/adr/0558).
    Mapping,
}

/// A file copied into the package.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PackagedFile {
    /// Its path in the package, `/` between its parts (`veri/kadastro.gpkg`).
    pub path: String,
    /// The file it was copied from.
    pub from: String,
    pub kind: PackagedKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// How many links point to it (a profile or a mapping is found by its place, not linked: 0).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub links: u32,
    /// The database table a copy was read from (`path` a GeoPackage made of its rows, `from` the database's address,
    /// docs/adr/0596 §2); none for a file copied byte for byte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
    /// The rows a database table's copy holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: Option<u64>,
}

/// Why a link was not packed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LeftWhy {
    /// The file is not there.
    Missing,
    /// A database's table: the link stays, the computer that opens the package reaches the database itself.
    Database,
    /// An address (http, https): read where the package opens, not copied.
    Address,
    /// A typeface KentOS does not have (an SHX line font, another family): its texts are drawn in a substitute face.
    Substituted,
    /// The file is there and cannot be read (a profile or a mapping the disk does not give): not copied.
    Unreadable,
    /// A bound GeoPackage whose link was kept as asked (`geopackages: keep`, docs/adr/0596 §1).
    Kept,
    /// Left out as asked (`leaveOut`, docs/adr/0596 §3).
    Excluded,
}

/// A link left as it was.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PackageLeft {
    pub kind: PackagedKind,
    /// The path or address as the project keeps it.
    pub path: String,
    pub why: LeftWhy,
    /// The object (its lasting id), bound layer (its id), external reference (`xref:` and its name), text style
    /// (`stil:` and its name) or profile file (`profiller:` and its file name) that keeps it.
    pub at: String,
    /// The objects taken out of the package with it: an excluded link's layer's (docs/adr/0596 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: Option<u32>,
}

/// A bound layer's link embedded in the package (docs/adr/0596 §1): removed, the layer holding its objects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PackageEmbedded {
    /// The link's id (`kaynak-N`).
    pub at: String,
    pub layer: String,
    /// The file's path or the database's address, and the table, as the link kept them.
    pub path: String,
    pub table: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
}

/// What `file.package` made, or with `plan` would make.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectPackaged {
    /// The folder.
    pub path: String,
    /// The project file, its bytes and their SHA-256.
    pub project: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub project_bytes: u64,
    pub project_sha256: String,
    pub files: Vec<PackagedFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PackageLeft>>", optional))]
    pub left: Vec<PackageLeft>,
    /// The links embedded (docs/adr/0596 §1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PackageEmbedded>>", optional))]
    pub embedded: Vec<PackageEmbedded>,
    /// The manifest, `paket.json` in the folder, and its SHA-256.
    pub manifest: String,
    pub manifest_sha256: String,
    /// The faces the drawing's texts are drawn in (KentOS's own, which come with it; nothing copied).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::DrawingFont>>", optional))]
    pub faces: Vec<crate::DrawingFont>,
    /// Every byte written: the project file, the manifest and the copies.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
}

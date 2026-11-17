//! Issues exchanged as BCF (plan/ifc-uyum.md IFC-10, docs/adr/8103): `cad.bcf.write` writes topics as a BCF 3.0 file,
//! each topic's viewpoint selecting its objects by their IFC GlobalIds and looking at them from a camera made from their
//! bodies in the IFC file named; `cad.bcf.read` reads a BCF 3.0 or 2.1 file's topics, and leads each selected object
//! to the IFC file named and to the drawing's object a KentOS GlobalId stands for. The BCF is `kentos-ifc`'s
//! (`kentos_ifc::bcf`); the handlers are `crates/native/application/src/arch/bcf_exchange.rs`; the shared cases are
//! `fixtures/commands/v1/cad.bcf.write.json` and `cad.bcf.read.json` (`scripts/fixtures/bcf_cases.py`: the files read
//! are IfcOpenShell's BCF library's; what is written is read back and checked by the same rules).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::IfcComparedObject;

pub const CAD_BCF_WRITE: &str = "cad.bcf.write";
pub const CAD_BCF_WRITE_VERSION: u32 = 1;
pub const CAD_BCF_READ: &str = "cad.bcf.read";
pub const CAD_BCF_READ_VERSION: u32 = 1;
/// The most topics a file is written with.
pub const MAX_BCF_TOPICS: usize = 10_000;
/// The most objects one topic selects.
pub const MAX_BCF_OBJECTS: usize = 1000;
/// The most characters of a topic's title.
pub const MAX_BCF_TITLE: usize = 300;
/// The most characters of a description or a comment.
pub const MAX_BCF_TEXT: usize = 4000;

/// A comment written with a topic: its author, date (`YYYY-MM-DDThh:mm:ss`, with a zone or UTC; absent: the topic's)
/// and text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfCommentInput {
    pub author: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    pub text: String,
}

/// A topic to write: its GUID (lowercase, hyphenated; absent: made from the file's place and the topic's order), title,
/// description, type (absent: `Issue`), status (absent: `Open`), priority, labels, author, date (absent: now, UTC),
/// whom it is assigned to, the objects it is about (IFC GlobalIds of the `model` file) and its comments.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfTopicInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub guid: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub topic_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub labels: Vec<String>,
    pub author: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub assigned_to: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub objects: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BcfCommentInput>>", optional))]
    pub comments: Vec<BcfCommentInput>,
}

/// Input of `cad.bcf.write` v1: the BCF file (absolute, ending `.bcf`), whether it may be written over, the IFC file
/// the topics' objects are in (absolute, `.ifc`; needed when a topic has objects: its viewpoint's camera looks at their
/// bodies and the topic names the file), the topics in order.
///
/// Refusals, in this order: `invalid_file_path` (at `path` or `model`: empty, relative, another extension, a folder or
/// a model not there), `file_exists` (without `overwrite`), `invalid_bcf_topic` (no topic, more than 10 000; a GUID not
/// lowercase hyphenated or given twice; an empty title, author or type; past a length; a date that does not read; more
/// than 1000 objects; objects without `model`; at the topic's field), `ifc_unreadable` (at `model`),
/// `ifc_object_not_found` (a GlobalId the model does not hold, at the topic's `objects`), `ifc_no_body` (no object of a
/// topic has a body to look at).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfWrite {
    pub path: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub overwrite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub model: Option<String>,
    pub topics: Vec<BcfTopicInput>,
}

/// Output of `cad.bcf.write` v1, and its plan: the file, how many topics and viewpoints it holds, the topics' GUIDs in
/// order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfWritten {
    pub path: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub topics: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub viewpoints: u32,
    pub guids: Vec<String>,
}

/// Input of `cad.bcf.read` v1: the BCF file (absolute, `.bcf` or `.bcfzip`) and, to lead the selected objects to their
/// objects, the IFC file the topics are about (absolute, `.ifc`).
///
/// Refusals, in this order: `invalid_file_path` (at `path` or `model`), `bcf_unreadable` (not a zip, no `bcf.version`,
/// a version other than 3.0, 2.1 or 2.0, an XML that does not read, a viewpoint the topic names and the file lacks;
/// the message says which), `ifc_unreadable` (at `model`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfRead {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub model: Option<String>,
}

/// A viewpoint's camera: `perspective` (its vertical field of view, degrees) or `orthogonal` (the height it shows,
/// metres); its eye, the way it looks and its up, in the IFC file's world, metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfCamera {
    pub kind: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub eye: Vec<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub direction: Vec<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub up: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub field_of_view: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub scale: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub aspect: f64,
}

/// A viewpoint read: its GUID, the objects it selects, whether the rest shows and the objects excepted, its camera
/// (absent only in a 2.1 file), its snapshot's name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfViewpoint {
    pub guid: String,
    pub selection: Vec<String>,
    pub visible: bool,
    pub exceptions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub camera: Option<BcfCamera>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub snapshot: Option<String>,
}

/// A comment read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfComment {
    pub guid: String,
    pub date: String,
    pub author: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub viewpoint: Option<String>,
}

/// A topic read, as the file writes it; its files are the header's IFC file names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfTopic {
    pub guid: String,
    pub topic_type: String,
    pub status: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub priority: Option<String>,
    pub labels: Vec<String>,
    pub created: String,
    pub author: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub assigned_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    pub files: Vec<String>,
    pub comments: Vec<BcfComment>,
    pub viewpoints: Vec<BcfViewpoint>,
}

/// An object some viewpoint selects, by its GlobalId: the `model` file's object it is, and the drawing's object (its
/// persistent id) when the GlobalId is one KentOS wrote for it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfObject {
    pub global_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub model: Option<IfcComparedObject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// Output of `cad.bcf.read` v1, and its plan: the file, its BCF version, its topics (by their folders' names), and the
/// objects their viewpoints select, once each, by GlobalId.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BcfReadOutput {
    pub path: String,
    pub version: String,
    pub topics: Vec<BcfTopic>,
    pub objects: Vec<BcfObject>,
}

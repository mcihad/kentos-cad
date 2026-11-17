//! Gelen paketi incele (docs/adr/8104; plan/bim-plan.md BIM-18, B20): the administration's reviewer opens a package as
//! it came — a publication's folder (its `yayin.json`, PDF/A sheets, IFC model and model quality form) or a lone IFC
//! file another program wrote — and `file.review.package` reads it without the author's drawing: what each file is
//! measured from its own bytes, and the standards package's rule (`dijital_proje.inceleme`) decides, layer by layer:
//! the package (its list, each file's bytes and digest), the sheets (PDF/A, scanned pages, the drawing revision each
//! was made from), the model (read, schema, the regulation's mandatory fields), its place (the system and zone against
//! the administration's own) and the form (the author's answers against what KentOS measures). Each finding names its
//! layer, its file and the objects it is about by their GlobalIds. Nothing is written.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const FILE_REVIEW_PACKAGE: &str = "file.review.package";
pub const FILE_REVIEW_PACKAGE_VERSION: u32 = 1;

/// Input of `file.review.package` v1: the package (absolute: a publication's folder, or a lone `.ifc` file) and the
/// administration's own system, by its EPSG code (`5255`: TUREF / TM33), the model's place is held to; absent: the
/// place layer is not judged.
///
/// Refusals: `invalid_file_path` (empty, relative, not there, a file that is not `.ifc`), `review_rule_unavailable` (the
/// standards package's rule does not answer: no Python; the message says how to give one). The facts are read whatever
/// the files hold: a file that does not read is a finding, not a refusal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewPackage {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub system: Option<u32>,
}

/// A layer of the review: `paket`, `pafta`, `model`, `konum` or `form`; whether it holds (absent: not judged, the
/// words say why) and its words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewLayer {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holds: Option<bool>,
    pub words: String,
}

/// A finding: its layer, its code, whether it is an error (else a warning), what it says (the measured and the asked),
/// how to put it right, the article it rests on, the file it is in, and the objects it is about by their GlobalIds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewFinding {
    pub layer: String,
    pub code: String,
    pub error: bool,
    pub message: String,
    pub repair: String,
    pub cite: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    pub objects: Vec<String>,
}

/// A row of the model quality form (EK-9 Tablo 9.1): its number and title, the author's answer (absent: the package
/// has no form) and KentOS's measured one, and whether they agree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewFormRow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub no: u8,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub author: Option<String>,
    pub kentos: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub agree: Option<bool>,
}

/// A file of the package as the review read it: its name, its kind (`pdf`, `ifc`, `form`, `list`, `other`) and whether
/// the package's list names it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewFile {
    pub name: String,
    pub kind: String,
    pub listed: bool,
}

/// Output of `file.review.package` v1, and its plan: the package, its files, the model's file name, the layers in
/// order, the findings (the layers' order, then the rule's), the form's rows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PackageReviewed {
    pub path: String,
    pub files: Vec<ReviewFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub model: Option<String>,
    pub layers: Vec<ReviewLayer>,
    pub findings: Vec<ReviewFinding>,
    /// The model's EK-9 form: `9.1` for the architectural discipline's model, `9.2` for another discipline's (its file
    /// name's EK-2 discipline; docs/adr/8114); none without a model read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub table: Option<String>,
    pub rows: Vec<ReviewFormRow>,
}

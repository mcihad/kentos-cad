//! A PDF page into the drawing (docs/adr/0352, plan/cad.md CAD-IO-21–26): the page's strokes, fills and texts as the
//! drawing's own objects, scaled from paper and placed, as one undo step named after the file. A PDF holds what was
//! printed, not the drawing it was printed from: a dimension, a leader or a table comes as its lines and texts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Vec2;
use crate::file_import::ImportedLayer;

/// A PDF page into the drawing.
pub const FILE_IMPORT_PDF: &str = "file.import.pdf";
pub const FILE_IMPORT_PDF_VERSION: u32 = 1;

/// What the page's curves become (docs/adr/0352 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PdfCurveOutput {
    /// Points within the tolerance, as polylines.
    #[default]
    Polyline,
    /// Each run of curves exactly, as a cubic NURBS curve; the straight runs between them as lines and polylines.
    Nurbs,
}

/// A rectangle on the page, millimetres from its lower left corner as it shows.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PdfRegion {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub a: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub b: Vec2,
}

/// Input of `file.import.pdf` v1. Refusals (`CommandError.code`), in order: `invalid_file_path`, `invalid_scale`,
/// `invalid_tolerance`, `invalid_pdf_region`, `file_not_found`, `import_unreadable` (not a PDF, encrypted or broken),
/// `pdf_page_not_found`, `layer_not_found` and `not_a_layer` (a target of `layers`), `layer_locked`,
/// `nothing_to_import`. Warnings: `import_losses` (pictures, shadings, letters that read as no character, white fills
/// taken for the paper, a page cut short).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImportPdf {
    /// The file to read, an absolute path.
    pub path: String,
    /// The page, from 1. Absent: the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub page: Option<u32>,
    /// Only what lies inside this rectangle of the page: a part of a path whose every point is in it, a text whose
    /// start is in it. Absent: the whole page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub region: Option<PdfRegion>,
    /// The drawing's scale denominator the page was printed at: a paper metre is this many of the drawing's metres.
    /// Absent: 1 (the page at its paper size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    /// Where the page's lower left corner goes in the drawing (metres on the wire). Absent: the origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// What is taken: strokes, fills, texts. Absent: each of them.
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub strokes: bool,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub fills: bool,
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub texts: bool,
    /// What the curves become; absent: polylines.
    #[serde(default, skip_serializing_if = "is_polyline")]
    #[cfg_attr(feature = "ts", ts(as = "Option<PdfCurveOutput>", optional))]
    pub curves: PdfCurveOutput,
    /// How far a polyline made of a curve may pass from it, millimetres on paper. Absent: 0.01.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub tolerance: Option<f64>,
    /// The page's layers (its optional content groups, by name; “PDF” for what is in none) whose objects are left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub skip_layers: Vec<String>,
    /// A page's layer → the drawing's layer (its id) its objects go onto. A layer not named here goes onto the
    /// drawing's layer of the same name (case and Turkish marks aside), else onto a new layer.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<BTreeMap<String, String>>", optional))]
    pub layers: BTreeMap<String, String>,
    /// The group at the top of the tree the new layers go into, found by its name or made. Absent: one named after
    /// the file; empty: the top of the tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// The drawing's revision the input was prepared against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

fn yes() -> bool {
    true
}

fn is_yes(b: &bool) -> bool {
    *b
}

fn is_polyline(c: &PdfCurveOutput) -> bool {
    *c == PdfCurveOutput::Polyline
}

/// The objects taken, by kind.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PdfTaken {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub polylines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub curves: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub areas: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub texts: u32,
}

/// What the page held that was not taken, by why.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PdfLeftOut {
    /// Pictures and shadings: a PDF import takes vectors and texts, never pixels.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub images: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub shadings: u32,
    /// Letters of a font without a map to characters.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unreadable: u32,
    /// White fills, taken for the paper.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub white: u32,
    /// Parts and texts outside the region.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
    /// The page held more than a page is read for; the rest is left out.
    pub truncated: bool,
}

/// Output (and plan) of `file.import.pdf` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PdfImported {
    /// The file's name, its bytes and their SHA-256 (64 hex digits).
    pub file: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
    /// The page taken and how many the file has.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub page: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pages: u32,
    /// The page's size as it shows, millimetres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub paper: [f64; 2],
    /// The box of what was taken in the drawing, `[minX, minY, maxX, maxY]` (metres on the wire); absent when nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bounds: Option<[f64; 4]>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    pub taken: PdfTaken,
    pub left: PdfLeftOut,
    /// A page's layer → the drawing's layer its objects went onto; the layers left out are not here.
    pub targets: BTreeMap<String, String>,
    /// The layers made, in the tree's order, and the group they went into.
    pub layers: Vec<ImportedLayer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// What a PDF cannot carry back (docs/adr/0352 §5), said once per import.
    pub notes: Vec<String>,
    /// The drawing's revision after the step (before it, in a plan).
    pub revision: String,
}

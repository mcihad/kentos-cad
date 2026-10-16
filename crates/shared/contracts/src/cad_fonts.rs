//! `cad.fonts.report` v1 (docs/adr/0324 §5): which typeface each text style names and which of KentOS's faces it is
//! drawn in, and how; the faces a sheet's PDF embeds. Read only. KentOS draws, measures and embeds only its own seven
//! faces; a style naming another (an SHX line font, Times …) is drawn in a substitute and said here, never silently.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{DrawingFont, REVISION_TEXT};

pub const CAD_FONTS_REPORT: &str = "cad.fonts.report";
pub const CAD_FONTS_REPORT_VERSION: u32 = 1;

/// The longest typeface name it reads.
pub const MAX_FONT_NAME: usize = 80;

/// Input of `cad.fonts.report` v1. Refusals: `empty_name`, `invalid_name`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FontsReport {
    /// Typefaces by a file or a family (`romans.shx`, `Arial`) to resolve by the same rule, as a DXF's would be.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub names: Vec<String>,
}

/// How a named typeface is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FontMatch {
    /// One of KentOS's own faces (a style naming no file is its own face).
    Own,
    /// A metric twin: Arial, Helvetica, Liberation Sans as Arimo, Courier as Courier Prime.
    Equivalent,
    /// An AutoCAD SHX line font, not read: drawn in the style's face.
    Shx,
    /// A face KentOS does not have: drawn in the style's face.
    Missing,
}

/// A text style's typeface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StyleFont {
    pub style: String,
    pub name: String,
    /// The file it named (a DXF's STYLE), when it named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// The face it is drawn in.
    pub face: DrawingFont,
    pub how: FontMatch,
    /// How many texts of the drawing take the style.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub texts: u32,
}

/// A name given, resolved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NamedFont {
    pub name: String,
    /// KentOS's face for it; absent: the project's face takes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub face: Option<DrawingFont>,
    pub how: FontMatch,
}

/// Output of `cad.fonts.report` v1. A style drawn in a substitute is also a `font_substituted` warning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FontsReported {
    /// The project's face: texts without a style or a face of their own.
    pub project: DrawingFont,
    pub styles: Vec<StyleFont>,
    pub names: Vec<NamedFont>,
    /// The faces the drawing's texts, dimensions and leaders are drawn in, in the contract's order: a sheet's PDF
    /// embeds subsets of these.
    pub faces: Vec<DrawingFont>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

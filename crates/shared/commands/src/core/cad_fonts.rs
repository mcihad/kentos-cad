//! `cad.fonts.report` v1 (docs/adr/0324 §5): which typeface each text style names and which face it is drawn in, and
//! how; the faces a sheet's PDF embeds. Read only. KentOS draws, measures and embeds its own faces and the families its
//! fonts folders hold (docs/adr/9313); a style naming another (an SHX line font, Calibri …) is drawn in a substitute
//! and said here, never silently. A style whose SHX font (and Big Font) the project embeds is drawn in that font's
//! strokes instead (ADR 1125). It also lists the fonts folders' files and the typefaces a person can pick.

use serde::{Deserialize, Serialize};

use crate::DrawingFont;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_FONTS_REPORT: &str = "cad.fonts.report";
pub const CAD_FONTS_REPORT_VERSION: u32 = 1;

/// The longest typeface name it reads.
pub const MAX_FONT_NAME: usize = 80;

/// Input of `cad.fonts.report` v1. Refusals: `empty_name`, `invalid_name`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontsReport {
    /// Typefaces by a file or a family (`romans.shx`, `Arial`) to resolve by the same rule, as a DXF's would be.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
}

/// How a named typeface is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum FontMatch {
    /// One of KentOS's own faces (a style naming no file is its own face).
    Own,
    /// A metric twin: Arial, Helvetica, Liberation Sans as Arimo, Courier as Courier Prime.
    Equivalent,
    /// An SHX line font: drawn in the style's face unless the project embeds it (`StyleFont::embedded`).
    Shx,
    /// A face KentOS does not have: drawn in the style's face.
    Missing,
    /// A family a fonts folder gave (docs/adr/9313): drawn in its own faces.
    Folder,
}

/// A text style's typeface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StyleFont {
    pub style: String,
    pub name: String,
    /// The file it named (a DXF's STYLE), when it named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The face it is drawn in.
    pub face: DrawingFont,
    pub how: FontMatch,
    /// Its SHX font, and its Big Font when it names one, are embedded in the project and read (ADR 1125): its
    /// texts are drawn in the font's own strokes, not in `face`, and it is no `font_substituted`. Absent: false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub embedded: bool,
    /// How many texts of the drawing take the style.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub texts: u32,
}

/// A name given, resolved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct NamedFont {
    pub name: String,
    /// KentOS's face for it; absent: the project's face takes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<DrawingFont>,
    pub how: FontMatch,
}

/// Output of `cad.fonts.report` v1. A style drawn in a substitute is also a `font_substituted` warning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct FontsReported {
    /// The project's face: texts without a style or a face of their own.
    pub project: DrawingFont,
    pub styles: Vec<StyleFont>,
    pub names: Vec<NamedFont>,
    /// The faces the drawing's texts, dimensions and leaders are drawn in, in the contract's order: a sheet's PDF
    /// embeds subsets of these.
    pub faces: Vec<DrawingFont>,
    /// The typefaces a person can pick, in the order the lists show them: KentOS's own, then the fonts folders'
    /// families (docs/adr/9313).
    pub choices: Vec<DrawingFont>,
    /// The fonts folders' files as this program read them when it started (docs/adr/9313), and the operating
    /// system's Arial and Times New Roman it uses.
    pub files: Vec<FontFile>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Where a font file was found (docs/adr/9313, CLAUDE.md §26).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum FontFolder {
    /// The install's `fonts` folder (`<kaynak>/fonts`; the checkout's `paket/fonts`).
    Program,
    /// The user's `fonts` folder (`<kullanıcı>/fonts`).
    User,
    /// The operating system's fonts: only Arial and Times New Roman are read there (docs/adr/9311).
    System,
}

/// What KentOS does with a face of a fonts folder (docs/adr/9313).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum FontRole {
    /// One of KentOS's own drawing faces: the program draws its own copy, the same bytes.
    Own,
    /// The bundled metric twin of Arial (Arimo) or Times New Roman (Liberation Serif): it draws where the named face
    /// is not installed.
    Twin,
    /// Arial or Times New Roman itself (a licensed copy, or the operating system's): drawn in place of its twin.
    Given,
    /// One of the interface's faces whose family is one of KentOS's own drawing faces (the drawing takes its own).
    Interface,
    /// A fallback face for the letters a face has not got (Noto Sans Arabic …).
    Fallback,
    /// A family texts can take by its name (`face`).
    Family,
    /// Not used; `note` says why.
    Unused,
}

/// A face of a font file in a fonts folder (docs/adr/9313).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct FontFile {
    /// The file within its folder (`cizim/Barlow-400.ttf`).
    pub file: String,
    pub folder: FontFolder,
    /// Its family as the file names it (`Roboto`); empty when it names none.
    pub family: String,
    /// Its style within the family (`Bold Italic`); empty when the file names none.
    pub style: String,
    /// 100–900 (400 regular, 700 bold).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub weight: u16,
    pub italic: bool,
    pub role: FontRole,
    /// The typeface texts take it as; absent for a fallback face and a file not used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<DrawingFont>,
    /// Whether it has every letter of the Turkish alphabet (ÇçĞğİıÖöŞşÜü).
    pub turkish: bool,
    /// Why it is not used, or how it is (`değişken yazı tipi: varsayılan kalınlığıyla çizilir`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

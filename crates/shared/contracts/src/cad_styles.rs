//! The text and dimension styles' command of the product command catalog
//! (docs/adr/0248 §4): `cad.styles.edit` creates, updates, renames or
//! removes one of the project's named styles (docs/adr/0183). The table is
//! a project setting (an edit, no undo step); the objects that follow a
//! changed or removed style are written in one undo step, “Yazı stili” or
//! “Ölçü stili”, as the Yazı stilleri and Ölçü stilleri windows write them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::annotation::{DimensionStyleDef, TextStyleDef};
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Creates, updates, renames or removes a text or dimension style.
pub const CAD_STYLES_EDIT: &str = "cad.styles.edit";
pub const CAD_STYLES_EDIT_VERSION: u32 = 1;

/// Which table `cad.styles.edit` changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AnnotationStyleKind {
    Text,
    Dimension,
}

/// What `cad.styles.edit` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StyleEditOperation {
    Create,
    Update,
    Rename,
    Remove,
    /// The kind's styles as a `kentos.annotation-styles` file's text (docs/adr/0322 §4).
    Export,
    /// A `kentos.annotation-styles` file's styles taken in by name (docs/adr/0322 §4).
    Import,
}

/// The format name of a file of text or dimension styles (docs/adr/0322 §4): `{"format", "version": 1, "kind",
/// "styles": [definitions]}`, the definitions as the project's table holds them.
pub const ANNOTATION_STYLES_FORMAT: &str = "kentos.annotation-styles";

/// Input of `cad.styles.edit` v1.
///
/// - `create`: the kind's definition (`text` or `dimension`); an empty id
///   gives it a new one (a UUIDv7), a given id must be new. It goes last in
///   the table; nothing follows it yet;
/// - `update`: the kind's definition, whose id names the style it replaces
///   (its place kept). The texts (or dimensions) of the style take its new
///   values where they still held the old ones (`follow_text_style`,
///   `follow_dimension_style`), in one undo step;
/// - `rename`: `style`, `name`; the objects name the style by its id and
///   do not change;
/// - `remove`: `style`; its objects keep their look and lose the link, in
///   one undo step;
/// - `export`: `styles` (absent: every style of the kind), in the table's
///   order, as a `kentos.annotation-styles` file's text in `library`;
///   nothing is written;
/// - `import`: `library`, a `kentos.annotation-styles` file's text of the
///   same kind; each of its styles by name (Turkish case folded, trimmed):
///   a new name is added (its id kept when the table has none such, else a
///   new one), a name the table has is kept as it is (`skipped`), or with
///   `replace` updated in its place (its id and name kept, its objects following as
///   `update` makes them).
///
/// The table is the project's setting: changing it makes the drawing dirty
/// and is no undo step (docs/adr/0183 §5, kept by docs/adr/0248 §4). Objects
/// on locked layers are left as they are and counted in `locked`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_definition`
/// (create, update without the kind's definition), `no_style` (rename,
/// remove without `style`), `empty_name` (rename with a name empty or only
/// white space), `no_library` (import without `library`); then
/// `invalid_revision`, `revision_conflict` (status `conflict`);
/// `style_not_found` (update's id, rename's and remove's `style`, each of
/// export's `styles`), `duplicate_style` (create's given id the table has),
/// `invalid_library` (import's text not such a file, or of the other kind);
/// `invalid_style` (the table it would leave, by `text_styles_problem` or
/// `dimension_styles_problem`: the names, then each style's values).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StylesEdit {
    pub kind: AnnotationStyleKind,
    pub operation: StyleEditOperation,
    /// Rename's and remove's style, by its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    /// Rename's new name, trimmed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Create's and update's text style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<TextStyleDef>,
    /// Create's and update's dimension style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dimension: Option<DimensionStyleDef>,
    /// Export's styles, by id; absent: every style of the kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub styles: Option<Vec<String>>,
    /// Import's `kentos.annotation-styles` file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub library: Option<String>,
    /// Import: true, a style whose name the table has is updated from the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub replace: Option<bool>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.styles.edit` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StylesEdited {
    /// The style's id; export's and import's first (empty when none).
    pub style: String,
    /// Whether the table changed.
    pub changed: bool,
    /// The objects written in the undo step.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub followed: u32,
    /// The objects a locked layer kept as they were.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub locked: u32,
    /// Import: the styles added, by id, in the file's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<String>,
    /// Import with `replace`: the styles updated, by id, in the file's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaced: Vec<String>,
    /// Import: the file's names the table has, kept as they are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    /// Export: the file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub library: Option<String>,
    /// The document's revision afterwards (plan: the one it was made
    /// against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

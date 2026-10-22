//! The sheet book's command of the product command catalog (docs/adr/0252
//! §2): the sheet core's own operations (`kentos.sheet/1`, the ones the
//! sheet designer makes) applied to the project's book, as Python and an
//! agent make a sheet the designer would make.
//!
//! The operations cross as the sheet core writes them (each an object tagged
//! by `op`: `addSheet`, `addItems`, `setItemProps` …); the core reads and
//! checks them, so this contract keeps them opaque, as a snapshot keeps the
//! book (docs/adr/0251). The book is the paper's: changing it is an edit of
//! the project, not an undo step of the drawing (docs/adr/0251 §3). The
//! answer carries the operations that take the change back.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Changes the project's sheet book.
pub const CAD_SHEETS_EDIT: &str = "cad.sheets.edit";
pub const CAD_SHEETS_EDIT_VERSION: u32 = 1;

/// Input of `cad.sheets.edit` v1: the sheet core's operations applied to the
/// project's book in order, as one change. Lengths on the paper are
/// micrometres, angles thousandths of a degree, as the core writes them. A
/// project without a book starts from an empty one; a book left empty is no
/// book. The book is the paper's (docs/adr/0251 §3): the change makes the
/// drawing dirty and is no undo step of it; `inverse` in the answer takes it
/// back. Inside an explicit group/changeset the book and its pictures are part of the model undo step (ADR 0590).
///
/// Refusals (`CommandError.code`), checked in this order: `no_operations`
/// (none given), `invalid_operation` (one the core does not read, at
/// `ops[i]`), `invalid_revision`, `revision_conflict` (status `conflict`),
/// `book_unreadable` (the project's book is one the core cannot read: it is
/// kept as it is), then the core's own refusal of an operation with its code
/// (`unknown_sheet`, `duplicate_id`, `unknown_item` …), at `ops[i]` and the
/// core's path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetsEdit {
    /// The operations, at least one, each an object tagged by `op`
    /// (`kentos.sheet/1`).
    #[cfg_attr(feature = "ts", ts(type = "Array<Record<string, unknown>>"))]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<kentos_sheet::ops::Op>"))]
    pub ops: Vec<serde_json::Value>,
    /// Embedded picture bytes, keyed by SHA-256. Metadata is supplied by addAssets or applyTemplate.
    /// Each picture must match the book's metadata (digest and byte length). Existing bytes are retained.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<crate::SheetPicture>>"))]
    pub pictures: Vec<crate::SheetPicture>,
    /// Replace the entire picture byte set instead of merging it; use with the answer's inversePictures for undo.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub replace_pictures: bool,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A map frame of a sheet as the answer lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetMapInfo {
    /// The item's id.
    pub item: String,
    /// The item's name, unique in its sheet.
    pub name: String,
    /// The map's scale denominator (1:N); none for a map that follows something else (an atlas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<f64>,
}

/// A sheet of the book as the answer lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetInfo {
    pub id: String,
    pub name: String,
    /// The page's width and height, millimetres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub width: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: f64,
    /// Its items, how many.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub items: u32,
    /// Its map frames, in drawing order.
    pub maps: Vec<SheetMapInfo>,
}

/// Output of `cad.sheets.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SheetsEdited {
    /// What was done, as the designer's undo names it (“Pafta ekle: Kat planı”, “3 işlem”).
    pub label: String,
    /// The operations that take the change back, in the order to apply them
    /// (an input of this command again).
    #[cfg_attr(feature = "ts", ts(type = "Array<Record<string, unknown>>"))]
    pub inverse: Vec<serde_json::Value>,
    /// Previous embedded bytes when the operation changed them. Restore together with inverse operations,
    /// passing these as pictures and replacePictures=true. Omitted when bytes did not change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub inverse_pictures: Option<Vec<crate::SheetPicture>>,
    /// The book's sheets after the change, in its order.
    pub sheets: Vec<SheetInfo>,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn operations_cross_as_they_are() {
        let input: SheetsEdit = serde_json::from_value(json!({
            "ops": [{ "op": "removeSheet", "id": "s1" }],
            "expectedRevision": "4"
        }))
        .expect("reads");
        assert_eq!(input.ops[0]["op"], "removeSheet");
        assert_eq!(input.expected_revision.as_deref(), Some("4"));
    }
}

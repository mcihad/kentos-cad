//! The associative array's edit command of the product command catalog
//! (docs/adr/0301; AutoCAD's ARRAYEDIT, Diziyi düzenle): an array named by its
//! id, edited by its layout, its first placement or its items rather than by
//! the inserts' new geometry.
//!
//! The array is the document's record (`assoc_array::AssocArray`, docs/adr/0300);
//! its items are numbered in the layout's order from 0 (a grid row after row:
//! row `r`, column `c` is `r × cols + c`). Lengths are metres on the wire,
//! whatever unit the project shows (docs/adr/0165 §2); angles radians, but a
//! polar layout's `fill` degrees as the array command's.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_array::ArrayLayout;
use crate::entity::Vec2;

/// Edits an associative array.
pub const CAD_ARRAYS_EDIT: &str = "cad.arrays.edit";
pub const CAD_ARRAYS_EDIT_VERSION: u32 = 1;

/// What changes in the array (AutoCAD ARRAYEDIT's properties, Replace, Reset; an item's erase).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArrayEdit {
    /// The rows, columns, count, spacing, fill or path: every item is laid out again. An item keeps its number
    /// (a grid's its row and column); a removed one stays removed, a kept one where it is; the places that are new
    /// get inserts, the ones that are gone take their inserts away. The kind may change too.
    Layout { layout: ArrayLayout },
    /// The first place: where the block's base point goes, its scale and turn, mirrored or not; the items that
    /// are neither removed nor kept follow. Absent fields stay.
    Placement {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        p: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        scale: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        rotation: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        mirror: Option<bool>,
    },
    /// These items insert the block named `block` instead (Değiştir): each is kept from then on.
    Replace {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        items: Vec<u32>,
        block: String,
    },
    /// These items' inserts are deleted; the items stay removed until reset.
    Remove {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        items: Vec<u32>,
    },
    /// These items (none: every item) as the layout makes them: a removed one comes back, a kept one goes to its
    /// place with the array's block (Sıfırla).
    Reset {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        items: Option<Vec<u32>>,
    },
}

/// Input of `cad.arrays.edit` v1: the associative array `array` edited by `edit`, as one undo step “Diziyi
/// düzenle”. The array's layer and every insert the edit touches must be unlocked: all or nothing. The answer is
/// `cad.entities.edit`'s: the item inserts changed, created and removed.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid` (the array's id), the edit's own numbers
/// (`not_finite`, `invalid_count`, `invalid_spacing`, `invalid_fill`, `invalid_uid` for a path; `invalid_scale`;
/// `invalid_item` for an empty or repeated list), `invalid_revision`, `revision_conflict` (status `conflict`),
/// `array_not_found`, `invalid_item` (a number past the places), `layer_locked`, `item_removed` (Değiştir or
/// Sil on a removed item), `unknown_block`, a path layout's `entity_not_found`, `invalid_path` and
/// `invalid_spacing`, `not_finite` (places past the largest float64), `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArraysEdit {
    /// The array's id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub array: String,
    /// What changes.
    pub edit: ArrayEdit,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_edits_read_by_their_kind() {
        let input: ArraysEdit = serde_json::from_value(json!({
            "array": "0192f5a3-3333-7000-8000-000000000001",
            "edit": { "kind": "layout", "layout": { "kind": "grid", "rows": 2, "cols": 3, "dx": 4, "dy": 5 } }
        }))
        .expect("reads");
        assert!(matches!(input.edit, ArrayEdit::Layout { .. }));
        let reset: ArrayEdit = serde_json::from_value(json!({ "kind": "reset" })).expect("reads");
        assert_eq!(reset, ArrayEdit::Reset { items: None });
        let replace: ArrayEdit =
            serde_json::from_value(json!({ "kind": "replace", "items": [2], "block": "Ağaç" }))
                .expect("reads");
        assert!(matches!(replace, ArrayEdit::Replace { .. }));
    }
}

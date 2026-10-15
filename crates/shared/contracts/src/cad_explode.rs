//! Patlat's semantic command of the product command catalog (docs/adr/0302; AutoCAD's EXPLODE): objects named by
//! their persistent ids broken one level into simpler ones, the pieces computed by the command rather than given.
//!
//! The pieces are the shared core's (`ops::explode`, a block's `Blocks::explode`): a path its lines and arcs, an
//! area its edges (holes and parts too), a dimension its lines, arrowheads and text, a patterned hatch its lines, a
//! leader its line, arrowhead and note, a table its lines and words, a block's insert its definition's objects. An
//! item of an associative array (docs/adr/0300) explodes its whole array: every item's insert, and the array is
//! gone (§2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};

/// Explodes objects one level.
pub const CAD_ENTITIES_EXPLODE: &str = "cad.entities.explode";
pub const CAD_ENTITIES_EXPLODE_VERSION: u32 = 1;

/// Input of `cad.entities.explode` v1: the objects `uids` exploded one level, as one undo step “Patlat”. Each object
/// is replaced by its pieces: a piece keeps the object's layer, look and data, a block's object its own layer when
/// the drawing has it, colour, line weight, data and label (the insert's when it has none). An object on a locked
/// layer is left out with a warning (`layer_locked`); one that does not explode (a line, a circle, a point, a text,
/// a solid hatch …) too (`not_explodable`), with the first one's words.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`, `invalid_uid`, `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`, `layer_locked` (every object locked),
/// `not_explodable` (none explodes), `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesExplode {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

//! The semantic break and join commands of the product command catalog
//! (docs/adr/0254 §3, §4; plan/cad.md CAD-CMD-01, 05): an object broken
//! between two points or split at one, and objects meeting end to end
//! joined, named by persistent ids and the points or the tolerance rather
//! than the pieces.
//!
//! The pieces and the chains are the shared geometry core's
//! (`break_entity`, `join_entities`, `chain`: the rules Kır and Birleştir
//! have always used). Their output and plan are `cad.entities.edit`'s
//! (`EntitiesEdited`, `EntitiesEditPlan`). Coordinates and lengths are
//! metres on the wire (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_trim::TrimBoundaries;
use crate::entity::Vec2;

/// Breaks an object between two points, or splits it at one.
pub const CAD_ENTITIES_BREAK: &str = "cad.entities.break";
pub const CAD_ENTITIES_BREAK_VERSION: u32 = 1;
/// Joins objects meeting end to end.
pub const CAD_ENTITIES_JOIN: &str = "cad.entities.join";
pub const CAD_ENTITIES_JOIN_VERSION: u32 = 1;

/// The ends' gap Birleştir takes when none is given, metres (the tool's).
pub const JOIN_TOLERANCE: f64 = 0.001;

/// Input of `cad.entities.break` v1: the part of the object between `first`
/// and `second` goes; without `second` (or with it on `first`) the object
/// splits at `first`. One undo step “Kır”. The object is replaced by its
/// pieces, the first in its place keeping its id (its data kept when one
/// piece is left, not for a closed area), the others new objects from it.
/// Lines, polylines, areas, arcs, circles, ellipses, construction lines and
/// rays break.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid`,
/// `not_finite` (`first`, then `second`), `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`,
/// `not_breakable` (another kind; an area with holes, which would lose
/// them), `layer_locked`, `break_failed` (the
/// core's words: the points off the object, a closed object split at one
/// point); on the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesBreak {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The first break point, on or near the object.
    pub first: Vec2,
    /// The second; absent: the object splits at `first`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub second: Option<Vec2>,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Input of `cad.entities.join` v1: the lines, arcs and open polylines of
/// `uids` meeting end to end within `tolerance` become one object each
/// chain (a chain that closes, a closed area), in the place and with the
/// persistent id and the data of its first object; the others go. One undo
/// step “Birleştir”. Those on a locked layer are left out with a
/// `locked_skipped` warning at `uids[i]`.
///
/// Zincir (`chain`, docs/adr/0161 §2): `uids` is one object, the seed; the
/// objects joined end to end with it among `chain`'s objects (lines, arcs,
/// open polylines) are joined with it, up to a free end, a junction, a
/// locked object (a `chain_stopped` warning) or the start again. The seed
/// keeps its place, its id and its data.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`
/// (no id; with `chain` more than one), `invalid_uid` (at `uids[i]`), then
/// `chain`'s ids, `invalid_distance` (`tolerance` not over 0 or not
/// finite), `not_finite` (`chain`'s rectangle), `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`,
/// `not_joinable` (a seed that is not a line, an arc or an open polyline),
/// `layer_locked` (the seed), `nothing_to_join` (no chain of two),
/// `join_failed` (the core's words); on the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesJoin {
    /// The objects to join; with `chain`, the seed alone.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// How far apart two ends may be and still meet, metres; absent: 0.001.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tolerance: Option<f64>,
    /// Zincir: the objects the seed's chain walks over; absent: the objects of `uids`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub chain: Option<TrimBoundaries>,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
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
    fn a_chain_reads_its_objects_by_their_kind() {
        let input: EntitiesJoin = serde_json::from_value(json!({
            "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"], "chain": { "kind": "all" }
        }))
        .expect("reads");
        assert_eq!(input.chain, Some(TrimBoundaries::All));
        let split: EntitiesBreak = serde_json::from_value(json!({
            "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "first": { "x": 1.0, "y": 0.0 }
        }))
        .expect("reads");
        assert!(split.second.is_none());
    }
}

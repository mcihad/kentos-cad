//! The semantic stretch command of the product command catalog
//! (docs/adr/0254 §2; plan/cad.md CAD-CMD-01, 05): the vertices inside a
//! window moved, the rest kept, named by the window and the displacement
//! rather than the objects' new geometry.
//!
//! The objects' new shapes are the shared geometry core's (`stretch_entity`,
//! the rule Esnet has always used: an arc keeps its bow), so the tool,
//! Python and an agent asking the same get the same shapes. Coordinates are
//! metres on the wire, x east (Y), y north (X), whatever unit the project
//! shows (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};

/// Moves the vertices inside a window.
pub const CAD_ENTITIES_STRETCH: &str = "cad.entities.stretch";
pub const CAD_ENTITIES_STRETCH_VERSION: u32 = 1;

/// The window: the vertices inside it move (its edges count as inside).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StretchWindow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max: Vec2,
}

/// Topolojik düzenleme (docs/adr/0160 §3): the shown objects sharing a
/// moved vertex go with the stretched ones, in the same step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StretchTopology {
    /// Noktalar da: a point is a corner too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub points: bool,
}

/// Input of `cad.entities.stretch` v1: the vertices inside `window` move by
/// (`dx`, `dy`), the others stay, as one undo step “Esnet”.
///
/// The objects: those of `uids` (the selection), each without a vertex in
/// the window left out with an `outside_window` warning at `uids[i]`; or,
/// with no `uids`, every shown object meeting the window that has a vertex
/// in it. Those on a locked layer are left out with a `locked_skipped`
/// warning (at `uids[i]` when listed). With `topology`, the shown objects
/// sharing a moved vertex follow; a locked one does not (`neighbour_locked`
/// warning), nor one it would leave invalid (`neighbour_invalid`). A zero
/// displacement writes nothing.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite` (the
/// window's corners, then `dx`, `dy`), `no_entities` (`uids` given empty),
/// `invalid_uid` (at `uids[i]`), `invalid_revision`, `revision_conflict`
/// (status `conflict`), `entity_not_found` (at `uids[i]`),
/// `nothing_to_stretch` (no object left to stretch); on the desktop also
/// `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesStretch {
    /// The window over the vertices to move, metres.
    pub window: StretchWindow,
    /// How far east (Y) the vertices move, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dx: f64,
    /// How far north (X) the vertices move, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub dy: f64,
    /// Only these objects (the selection); absent: every shown object the window meets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Option<Vec<String>>,
    /// Topolojik düzenleme: the neighbours sharing a moved vertex follow; absent: they stay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub topology: Option<StretchTopology>,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.stretch` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesStretched {
    /// The objects stretched, in the input's (or the drawing's) order.
    pub stretched: Vec<String>,
    /// The neighbours that followed them (Topolojik düzenleme).
    pub followed: Vec<String>,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.stretch` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesStretchPlan {
    /// The objects as they would be, with their slots.
    pub stretched: Vec<Entity>,
    /// The neighbours as they would be.
    pub followed: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_input_reads_with_and_without_its_options() {
        let input: EntitiesStretch = serde_json::from_value(json!({
            "window": { "min": { "x": 0.0, "y": 0.0 }, "max": { "x": 2.0, "y": 2.0 } },
            "dx": 1.0, "dy": 0.0, "topology": {}
        }))
        .expect("reads");
        assert_eq!(input.topology, Some(StretchTopology { points: false }));
        assert!(input.uids.is_none());
    }
}

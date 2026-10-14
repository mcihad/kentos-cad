//! The semantic trim and extend commands of the product command catalog
//! (docs/adr/0250; plan/cad.md CAD-CMD-03): objects cut back to, or grown up
//! to, their boundaries, named by persistent ids and picked where the part
//! to take away, or the end to grow, lies.
//!
//! The input is what the user asked for: which objects, where each was
//! picked, and which objects bound them. The pieces are computed by the
//! shared geometry core (`trim_entity`, `extend_entity`, the rule Buda and
//! Uzat have always used), so the tools, Python and an agent asking the same
//! get the same pieces. Coordinates are metres on the wire, x east (Y), y
//! north (X), whatever unit the project shows (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};

/// Cuts objects back to their boundaries.
pub const CAD_ENTITIES_TRIM: &str = "cad.entities.trim";
pub const CAD_ENTITIES_TRIM_VERSION: u32 = 1;
/// Grows objects' ends up to their boundaries.
pub const CAD_ENTITIES_EXTEND: &str = "cad.entities.extend";
pub const CAD_ENTITIES_EXTEND_VERSION: u32 = 1;

/// Which objects bound the cut or the growth. An object never bounds itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TrimBoundaries {
    /// Every shown object of the drawing: on a visible layer, passing its
    /// layer's filter (docs/adr/0211) and, on a host showing a time slice,
    /// inside it (docs/adr/0210).
    All,
    /// Every shown object whose box meets the rectangle (the tools give their
    /// view: AutoCAD's quick mode, “every visible edge”).
    Area {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        min: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        max: Vec2,
    },
    /// These objects (Sınır seç), shown or not; at least one.
    Objects {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
}

/// AutoCAD's EDGEMODE: whether a boundary bounds only where it is, or along
/// its natural path past its ends too.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TrimEdgeMode {
    /// A boundary bounds where it is (EDGEMODE 0).
    #[default]
    None,
    /// A boundary bounds along its natural path past its ends too (EDGEMODE
    /// 1): a line as the whole infinite line, an arc as its whole circle, an
    /// open polyline's first and last segments on past its ends the same way.
    /// Closed objects, elliptic arcs and curves bound where they are.
    Extend,
}

/// A crossing window (Kesişen): the shown objects meeting the rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TrimWindow {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max: Vec2,
}

/// Where an object was picked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TrimPick {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// A point on or near the object: Buda takes away the part between the
    /// boundaries around it; Uzat grows the end nearest it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
}

/// Input of `cad.entities.trim` v1 and `cad.entities.extend` v1: the picked
/// objects trimmed (or extended) at their picks, as one undo step “Buda”
/// (“Uzat”), AutoCAD's TRIM and EXTEND (docs/adr/0294): the picks are clicks,
/// a fence (Çit) or a crossing window (Kesişen); the boundaries chosen
/// (standard mode) or every shown edge (quick mode); `edgeMode` bounds along
/// the boundaries' extensions; `eraseUncut` is quick mode's erase of what no
/// boundary cuts. A trim replaces the object by the pieces that stay, the first
/// in its place keeping its id, its data kept when one piece is left (not
/// for a closed area); an extend gives the object its new geometry.
///
/// Picks of one object are taken in their order, each on the object as the
/// earlier ones left it: a trim cuts the piece the pick lies on, and takes
/// away whole a piece no boundary cuts between its ends (each of its ends is
/// then an earlier cut or the object's end); an extend grows each end at
/// most once.
///
/// The fence's crossings are picks after `picks`, each object's in the
/// fence's order, the objects in the drawing's order. The window's objects
/// come last: Buda takes away every piece between the boundaries that meets
/// the rectangle, Uzat grows every end inside it. A fence or a window takes
/// only shown objects; those on a locked layer are left out with a
/// `layer_locked` warning counting them, and one that does nothing with a
/// `trim_failed` (`extend_failed`) warning at `fence` (`crossing`). Every object
/// is bounded by the drawing as it was before the command. A pick that can
/// do nothing (no boundary crosses there, the end meets none) is left out
/// with a `trim_failed` (`extend_failed`) warning at `picks[i]`; when no
/// pick does anything nothing is written and that is the answer.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`
/// (no pick, no fence and no window; `boundaries.uids` empty),
/// `too_few_points` (a fence of fewer than two points), `invalid_uid` (the
/// picks' ids, then the boundaries'), `not_finite` (a pick's, the fence's,
/// the window's, then the rectangle's x and y),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (the picks' objects, then the boundaries'),
/// `layer_locked` (a picked object, at `picks[i]`), `not_trimmable` (Buda:
/// an area with holes, which would lose them), then `trim_failed`
/// (`extend_failed`, at `picks`, with the first pick's reason: the core's
/// words for a kind it does not cut or grow, a closed path for Uzat, no
/// boundary there; at `fence` or `crossing` when it picked nothing); on the
/// desktop also `slots_exhausted`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTrim {
    /// The picks; none when a fence or a window picks.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<TrimPick>>"))]
    pub picks: Vec<TrimPick>,
    /// Çit: a drawn open path, at least two points; every shown object it
    /// crosses is picked where it crosses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub fence: Option<Vec<Vec2>>,
    /// Kesişen: a crossing window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crossing: Option<TrimWindow>,
    /// AutoCAD's EDGEMODE; absent: `none`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edge_mode: Option<TrimEdgeMode>,
    /// Buda: an object no boundary cuts goes whole where it is picked
    /// (AutoCAD's quick mode) instead of being left with a `trim_failed`
    /// warning. Uzat ignores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub erase_uncut: Option<bool>,
    /// The boundaries; absent: every shown object (`all`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundaries: Option<TrimBoundaries>,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.trim` v1 and `cad.entities.extend` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTrimmed {
    /// The objects that took new geometry (a trim's first piece, an extended
    /// object), in the picks' order.
    pub changed: Vec<String>,
    /// The new objects (a trim's other pieces), in the order written.
    pub created: Vec<String>,
    /// The objects a trim took away whole.
    pub removed: Vec<String>,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.trim` or `.extend` would write (plan mode); nothing is
/// written. Execute against the plan's revision writes exactly this.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTrimPlan {
    /// The objects that would take new geometry, as written, with their slots.
    pub changed: Vec<Entity>,
    /// The new objects as written, `id` 0.
    pub created: Vec<Entity>,
    /// The ids of the objects that would go.
    pub removed: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_boundaries_read_by_their_kind() {
        let input: EntitiesTrim = serde_json::from_value(json!({
            "picks": [{ "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "at": { "x": 1.0, "y": 2.0 } }],
            "boundaries": { "kind": "area", "min": { "x": 0.0, "y": 0.0 }, "max": { "x": 10.0, "y": 10.0 } }
        }))
        .expect("reads");
        assert!(matches!(
            input.boundaries,
            Some(TrimBoundaries::Area { .. })
        ));
        let all: TrimBoundaries = serde_json::from_value(json!({ "kind": "all" })).expect("reads");
        assert_eq!(all, TrimBoundaries::All);
    }
}

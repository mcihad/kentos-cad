//! The semantic opening commands of the product command catalog (docs/adr/0341 §4, plan/cad.md `CAD-ARCH-15`, `19`):
//! doors, windows and bare openings put in walls (`cad.openings.create`) and changed (`cad.openings.edit`). Each is one
//! undo step.
//!
//! Their input is what the user gave: the wall clicked and where on it, the type and values chosen. The jambs are
//! drawn from the wall, never given. Lengths are metres on the wire, whatever unit the project shows (docs/adr/0165 §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};
use crate::{OpeningForm, OpeningHand, OpeningOperation, OpeningSide, OpeningTypeDef, WallEnd};

pub const CAD_OPENINGS_CREATE: &str = "cad.openings.create";
pub const CAD_OPENINGS_CREATE_VERSION: u32 = 1;
pub const CAD_OPENINGS_EDIT: &str = "cad.openings.edit";
pub const CAD_OPENINGS_EDIT_VERSION: u32 = 1;
/// The most openings one call writes.
pub const MAX_OPENINGS: usize = 10_000;

/// Where along its wall a new opening goes, measured from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningFrom {
    /// The wall's end nearer the point given.
    #[default]
    Near,
    Start,
    End,
}

/// Which part of an opening is the requested distance from the chosen wall end.
/// Omitted on v1 requests: the nearer jamb, as before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningDistanceReference {
    #[default]
    Jamb,
    Center,
    /// The jamb named by `hand`, independently of the wall end named by `from`.
    Hinge,
}

/// One opening to put in a wall.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewOpening {
    /// The wall's persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub host: String,
    /// Its centre's distance along the wall's base line from its start. Absent: from `point`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<f64>,
    /// A point by the wall (the click): its centre goes where the base line passes nearest it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub point: Option<Vec2>,
    /// With `point`: its near jamb this far from the wall's end `from` (köşeden mesafe).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<OpeningFrom>,
    /// With `distance`: near jamb (default), centre, or the hinge jamb.
    /// This chooses placement only; the stored opening keeps its centre anchor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub distance_reference: Option<OpeningDistanceReference>,
    /// Two jamb points projected onto the named wall's base line. Their arc-length
    /// separation is the width, their midpoint along the path is the centre.
    /// Both must fit on one edge. Cannot be combined with at/distance/middle/width.
    /// `point` may still choose the opening side. Lengths are metres on the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub span: Option<[Vec2; 2]>,
    /// With `point`: centred on the edge of the base line the point is by (duvar ortası).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub middle: bool,
    /// An opening type by its id or name (KentOS's own). Absent: the form's first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Over the type's: a door, a window or a bare opening.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub form: Option<OpeningForm>,
    /// Over the type's: the rough opening, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub leaves: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operation: Option<OpeningOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand: Option<OpeningHand>,
    /// The side it opens to. Absent: the side `point` is on, else the left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<OpeningSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub swing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
}

/// Input of `cad.openings.create` v1: doors, windows and bare openings in their walls, written as one undo step
/// (“Kapı”, “Pencere” or “Boşluk”, the first's form). The tool gives the wall clicked and the point, the type and the
/// values chosen (TODOS.md CMD-07).
///
/// Where it goes: `at`; else `point` with `distance` (its near jamb that far from the wall's end `from`); else `point`
/// with `middle` (centred on the base line's edge the point is by); else `point` (centred where the base line passes
/// nearest it). It lies on one edge of its wall's base line, within its ends, clear of the wall's other openings.
/// Additive precision options: `distanceReference` selects the near jamb (default), centre or hinge;
/// `span` projects two jamb points onto one host edge and derives centre and width along its analytic path.
///
/// Refusals, in this order: `no_objects`, `not_finite`, `invalid_uid`, `entity_not_found`, `not_a_wall`,
/// `style_not_found`, `invalid_opening`, `opening_off_wall`, `openings_overlap`, `invalid_revision`, `revision_conflict`,
/// `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also `slots_exhausted`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsCreate {
    /// The layer they go on: a layer's id, not a group's.
    pub layer_id: String,
    pub openings: Vec<NewOpening>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.openings.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.openings.create` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsPlan {
    /// The openings as they would be stored, their `id` 0.
    pub openings: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// An opening's own fields to change; absent ones stay. A nullable field's null takes it away.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningChange {
    /// The rough opening, metres; it grows about its anchor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub form: Option<OpeningForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub leaves: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operation: Option<OpeningOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hand: Option<OpeningHand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<OpeningSide>,
    /// Degrees; null: 90.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub swing: Option<Option<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub show_swing: Option<bool>,
    /// Metres; null: no frame.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub frame: Option<Option<f64>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub sill: Option<Option<f64>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub head: Option<Option<f64>>,
    /// Its mark; null: none.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub number: Option<Option<String>>,
}

/// What `cad.openings.flip` turns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningFlip {
    /// The side it opens to.
    #[default]
    Side,
    /// Its hinge's end.
    Hand,
    Both,
}

/// Along the analytic host path: the clear jamb gap or the centre pitch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningArraySpacing {
    #[default]
    Clear,
    Centers,
}

/// Placement of NEW copies; the source remains unchanged and is not counted.
/// All distances are metres along the whole host's analytic path. Every copy
/// must still fit on one edge; crossing a corner refuses the whole operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningArrayLayout {
    /// Copies successively from the source; start goes backwards, end forwards.
    Spacing {
        #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 10000), extend("x-unit" = "count")))]
        count: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
        #[serde(default)]
        reference: OpeningArraySpacing,
        direction: WallEnd,
    },
    /// Fit the copies between the two jamb limits: startMargin from the host
    /// start, endMargin from its end. The first/last jamb touches each limit.
    /// With one copy, centre it in the available interval.
    Fit {
        #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 10000), extend("x-unit" = "count")))]
        count: u32,
        #[serde(rename = "startMargin")]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        start_margin: f64,
        #[serde(rename = "endMargin")]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        end_margin: f64,
    },
}

/// Marks only the NEW copies. Positive step; exact marks must be unique among
/// existing openings of the same form (door/window/bare opening) in the drawing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningArrayNumbering {
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(length(max = 128)))]
    pub prefix: String,
    #[serde(default = "first_number")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub start: u32,
    #[serde(default = "first_number")]
    #[cfg_attr(feature = "schema", schemars(range(min = 1), extend("x-unit" = "count")))]
    pub step: u32,
}

/// What `cad.openings.edit` does to the openings (docs/adr/0341 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningsOperation {
    /// Değiştir: the fields given, on every opening named.
    Set(OpeningChange),
    /// Taşı: along its wall by `by` (toward its end positive), or its anchor to `at`.
    Move {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        by: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Option<f64>,
    },
    /// Çevir: the side it opens to, its hinge's end, or both.
    Flip {
        #[serde(default)]
        what: OpeningFlip,
    },
    /// Başka duvara: each opening into the wall `to` (its persistent id), centred where the wall's base line passes
    /// nearest `point`, else nearest its jambs' middle.
    Rehost {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        to: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        point: Option<Vec2>,
    },
    /// Türü değiştir: the type `to` (its id or name) for the openings named, or with no ids every opening of type
    /// `from`.
    ReplaceType {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        from: Option<String>,
        to: String,
    },
    /// Numarala: marks `prefix` + `start`, `start` + 1, … in their walls' order and along each wall; with no ids every
    /// opening of the form `form` (all forms: absent).
    Renumber {
        #[serde(default)]
        prefix: String,
        #[serde(default = "first_number")]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        start: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        form: Option<OpeningForm>,
    },
    /// Adet/aralıkla çoğalt: exactly one source; count NEW independent copies,
    /// source unchanged. Optional unique numbering belongs to these copies only.
    Array {
        layout: OpeningArrayLayout,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        numbering: Option<OpeningArrayNumbering>,
    },
    /// Kopyala: a copy of each opening along its wall at each offset (toward its end positive).
    Copy {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        offsets: Vec<f64>,
    },
}

fn first_number() -> u32 {
    1
}

/// Input of `cad.openings.edit` v1: openings named by their persistent ids changed as one undo step “Açıklığı
/// düzenle”, all or none. An opening that would run off its wall or into another: nothing is written.
///
/// Refusals: `no_entities`, `invalid_uid`, `entity_not_found`, `not_an_opening`, `nothing_to_set`, `not_finite`,
/// `invalid_opening`, `style_not_found`, `style_not_used`, `not_a_wall`, `opening_host_gone`, `opening_off_wall`,
/// `openings_overlap`, `invalid_revision`, `revision_conflict`, `layer_locked`; on the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsEdit {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub operation: OpeningsOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.openings.edit` v1 (its plan's too: the openings as they would be, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsEdited {
    /// The openings as written, in their order.
    pub openings: Vec<Entity>,
    /// The new openings (Kopyala).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Entity>>", optional))]
    pub added: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// KentOS's opening types (for the tool's menu and the catalog's examples).
pub fn opening_types() -> Vec<OpeningTypeDef> {
    crate::builtin_opening_types()
}

/// The wall end `from` names for a point at `s` along a wall of length `l`.
pub fn wall_end_of(from: OpeningFrom, s: f64, l: f64) -> WallEnd {
    match from {
        OpeningFrom::Start => WallEnd::Start,
        OpeningFrom::End => WallEnd::End,
        OpeningFrom::Near => {
            if s <= l / 2.0 {
                WallEnd::Start
            } else {
                WallEnd::End
            }
        }
    }
}

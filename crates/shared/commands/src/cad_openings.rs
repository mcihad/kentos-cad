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
pub const CAD_OPENINGS_DISTANCES: &str = "cad.openings.distances";
pub const CAD_OPENINGS_DISTANCES_VERSION: u32 = 1;
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
    /// An opening type by its id or name: the project's (docs/adr/0772), which the opening then follows, else KentOS's
    /// own, whose values it copies. Absent: the form's first KentOS type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Over the type's: a door, a window, a bare opening, a niche or a flue (docs/adr/0719).
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
    /// The side it opens to (a niche's or flue's face). Absent: the side `point` is on, else the left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub side: Option<OpeningSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub swing: Option<f64>,
    /// Over the type's: its sill above the floor, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub sill: Option<f64>,
    /// Over the type's: its head above the floor, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub head: Option<f64>,
    /// Over the type's: a niche's depth from its face, a flue's size across its wall, metres (docs/adr/0719).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
    /// A flue's distance from its face to its cavity, metres; absent: centred in its wall.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub inset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// Over the type's: its construction's fields given (docs/adr/0772 §3). On a project type's opening, each value
    /// given over the type's is its own (örnek istisnası).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub build: Option<crate::OpeningBuild>,
}

/// Input of `cad.openings.create` v1: doors, windows, bare openings, niches and flues in their walls, written as one
/// undo step (“Kapı”, “Pencere”, “Boşluk”, “Niş” or “Baca”, the first's form). The tool gives the wall clicked and the
/// point, the type and the values chosen (TODOS.md CMD-07). A niche is cut from its face (`side`, absent: the side
/// `point` is on) `depth` into the wall, short of its other face; a flue is a cavity `inset` from its face (absent:
/// centred) and `depth` across, clear of both faces (docs/adr/0719).
///
/// Where it goes: `at`; else `point` with `distance` (its near jamb that far from the wall's end `from`); else `point`
/// with `middle` (centred on the base line's edge the point is by); else `point` (centred where the base line passes
/// nearest it). It lies on one edge of its wall's base line, within its ends, clear of the wall's other openings.
/// Additive precision options: `distanceReference` selects the near jamb (default), centre or hinge;
/// `span` projects two jamb points onto one host edge and derives centre and width along its analytic path.
///
/// Refusals, in this order: `no_objects`, `not_finite`, `invalid_uid`, `entity_not_found`, `not_a_wall`,
/// `style_not_found`, `invalid_opening`, `opening_off_wall`, `recess_too_deep`, `openings_overlap`, `invalid_revision`,
/// `revision_conflict`, `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also `slots_exhausted`.
/// Warning: `layer_hidden`.
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
    /// Derived source-following preview changes; absent when none. New identities are provisional.
    #[serde(default, skip_serializing_if = "crate::DerivedEntitiesPlan::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<crate::DerivedEntitiesPlan>", optional)
    )]
    pub followers: crate::DerivedEntitiesPlan,

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
    /// A niche's or flue's depth, metres (docs/adr/0719); null: none (a form that goes through).
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<Option<f64>>,
    /// A flue's distance from its face to its cavity, metres; null: centred.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub inset: Option<Option<f64>>,
    /// Its mark; null: none.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub number: Option<Option<String>>,
    /// Its construction's fields given (docs/adr/0772 §3); each set, the others stay. Boxed: the operations' enum
    /// stays small.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<crate::OpeningBuild>", optional))]
    pub build: Option<Box<crate::OpeningBuild>>,
    /// Its construction's fields taken away.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::OpeningTypeField>>", optional))]
    pub unset: Vec<crate::OpeningTypeField>,
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
    /// `from` (Aynı tiptekileri değiştir), with `level` only those on that level (null: on none). A project type's
    /// openings follow it (docs/adr/0772): they take all its values, their exceptions gone.
    ReplaceType {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        from: Option<String>,
        to: String,
        #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
        level: Option<Option<String>>,
    },
    /// Tipten ayır (docs/adr/0772 §1): the openings keep their values and follow no type.
    Detach,
    /// Tipe uygula: the one opening's values of `fields` (none: the fields it keeps as its own) become its type's; its
    /// type's version goes up and every other opening following it takes them, as `cad.openingtypes.edit`'s Update.
    Push {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::OpeningTypeField>>", optional))]
        fields: Vec<crate::OpeningTypeField>,
    },
    /// Tipi yeniden uygula: the openings take their type's latest version again, their exceptions kept (one left at an
    /// older version on a locked layer, docs/adr/0772 §4).
    Refresh,
    /// Tipe döndür: the openings give up their own values of `fields` (none: all) and take their type's again, its
    /// latest version.
    Reset {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::OpeningTypeField>>", optional))]
        fields: Vec<crate::OpeningTypeField>,
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
    /// Mesafeyle taşı (docs/adr/0701): each opening along its wall until its jamb on the wall's `face` is `distance`
    /// from the nearest reference before it (`side` start) or after it (end) on that face, the reference fixed; it
    /// stays on its edge. With `to: grid` (docs/adr/0728 §1) the distance is from the opening's middle to the nearest
    /// grid crossing the face on that side, which stays.
    Distance {
        side: WallEnd,
        #[serde(default)]
        face: crate::DimensionSetFace,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
        #[serde(default, skip_serializing_if = "crate::DistanceTo::is_face")]
        #[cfg_attr(feature = "ts", ts(as = "Option<crate::DistanceTo>", optional))]
        to: crate::DistanceTo,
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
/// `recess_too_deep`, `openings_overlap`, `invalid_revision`, `revision_conflict`, `layer_locked`, and since
/// docs/adr/0772 `opening_type_not_found`, `opening_not_typed`, `opening_type_form`, `opening_type_refused`,
/// `level_not_found`; on the desktop also `slots_exhausted`. Warnings (docs/adr/0772): `opening_type_override` when a
/// following opening's field set becomes its own, `opening_type_kept`, `opening_type_skipped`. A form changed to a niche or flue takes the depth of its form's first type when it has none, and
/// one changed to a form that goes through drops its depth and inset (docs/adr/0719).
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
    /// Derived source-following preview changes; absent when none. New identities are provisional.
    #[serde(default, skip_serializing_if = "crate::DerivedEntitiesPlan::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<crate::DerivedEntitiesPlan>", optional)
    )]
    pub followers: crate::DerivedEntitiesPlan,

    /// The openings as written, in their order.
    pub openings: Vec<Entity>,
    /// The new openings (Kopyala).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Entity>>", optional))]
    pub added: Vec<Entity>,
    /// Tipe uygula's type change: every other opening's fate (docs/adr/0772 §4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub report: Option<crate::OpeningTypeReport>,
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

/// Input of `cad.openings.distances` v1 (docs/adr/0701): the openings named, measured on their walls' `face` (the
/// path's left, the default, or right). Read-only.
///
/// Refusals: `no_entities`, `invalid_uid`, `entity_not_found`, `not_an_opening`, `opening_host_gone`,
/// `invalid_opening` (an arc edge, a bevelled face).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsDistancesInput {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    #[serde(default)]
    pub face: crate::DimensionSetFace,
}

/// What a distance is measured to: the face's corner or end, another opening's jamb, another wall's face.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum OpeningReference {
    End,
    Opening { uid: String },
    Wall { uid: String },
    /// A grid crossing the face, measured from the opening's middle (docs/adr/0728 §1).
    Grid { uid: String },
}

/// One side's distance: net along the face, from the opening's jamb to the reference's point on the face.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningDistance {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    pub reference: OpeningReference,
}

/// An opening's two sides on a face of its wall's edge `edge`; a side without a reference is absent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningDistances {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edge: u32,
    pub face: crate::DimensionSetFace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub start: Option<OpeningDistance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end: Option<OpeningDistance>,
    /// The nearest grid crossing the face before (after) the opening's middle, from that middle (docs/adr/0728 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub start_grid: Option<OpeningDistance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub end_grid: Option<OpeningDistance>,
}

/// Output of `cad.openings.distances` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningsDistances {
    pub openings: Vec<OpeningDistances>,
    pub revision: String,
}

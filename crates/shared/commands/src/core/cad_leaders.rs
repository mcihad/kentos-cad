//! `cad.leaders.edit` v1 (docs/adr/0326 §5): a leader's arms added and taken away, leaders' landings aligned
//! (AutoCAD's MLEADERALIGN), block content leaders collected into one (MLEADERCOLLECT), a leader's note and content
//! blocks, its look and style, and what its tips follow.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};

pub const CAD_LEADERS_EDIT: &str = "cad.leaders.edit";
pub const CAD_LEADERS_EDIT_VERSION: u32 = 1;

/// One edit of `cad.leaders.edit`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LeaderEdit {
    /// Kol ekle: an arm whose tip is `pts[0]` (its bends after it) to the leader's last vertex; `tip` what the tip
    /// follows.
    #[serde(rename_all = "camelCase")]
    AddArm {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        pts: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        tip: Option<crate::SubRef>,
    },
    /// Kol sil: arm `arm` (0 the first) taken away; without its first arm the next is first. The last arm stays.
    RemoveArm {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        arm: u32,
    },
    /// Hizala (MLEADERALIGN): the first leader stays; the others' last vertices go onto the line through its last
    /// vertex at `angle` (degrees counter-clockwise from east), where they fall on it, or `spacing` metres apart from
    /// it in that order.
    Align {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        angle: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        spacing: Option<f64>,
    },
    /// Topla (MLEADERCOLLECT): block content leaders into the first: the others' arms its arms, their blocks after its
    /// own; the others taken away, their blocks kept.
    Collect {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// The note written (null: taken away).
    Content {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        text: Option<String>,
    },
    /// The content blocks (inserts, by persistent id; empty: none): they stand in a row at the landing's end.
    Blocks {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        blocks: Vec<String>,
    },
    /// The leaders' look, each field given set apart from their style: the arrowhead (`filled` or a `LeaderArrow`
    /// name), its length in the note's height, the note's height in metres, the mask.
    #[serde(rename_all = "camelCase")]
    Look {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        arrow: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        arrow_size: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        height: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        mask: Option<bool>,
    },
    /// The leaders follow the leader style `style` (its values written into them; null: they keep their look and
    /// follow none).
    Style {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        style: Option<String>,
    },
    /// Arm `arm`'s tip bound to `tip` (it goes to the place's point) or (null) freed.
    Bind {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        arm: u32,
        tip: Option<crate::SubRef>,
    },
}

/// Input of `cad.leaders.edit` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_leader`, `layer_locked`,
/// `no_entities`, `too_few_objects`, `arm_not_found`, `invalid_leader`, `not_an_insert`, `style_not_found`,
/// `no_such_place`, `not_finite`, `empty_text`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LeadersEdit {
    pub edit: LeaderEdit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.leaders.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LeadersEdited {
    /// The leaders that changed, by their persistent ids.
    pub changed: Vec<String>,
    /// The leaders Topla took away.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.leaders.edit` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LeadersPlan {
    /// The leaders as execute would write them.
    pub leaders: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

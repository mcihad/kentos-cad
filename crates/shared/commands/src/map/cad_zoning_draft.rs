//! `cad.zoning.draft` v1 (plan/map-cad.md MAP-09, docs/adr/5040): Plan değişikliği taslağı. A draft of a change to a
//! plan of the profile `imar-plani`, kept in the project (`.kcad` schema 250, `settings.zoningPlan.drafts`): the plan,
//! the drawing's revision it is made from, its reason (Mekânsal Planlar Yapım Yönetmeliği m. 9(4): a plan change's
//! report explains its justification) and, for each decision area it changes, a field's value before and after. The
//! decision areas in force are never written. The answer compares the plan's areas by use before and after and says
//! what m. 26 asks of such a change (a social or technical infrastructure area made smaller, a density or a height
//! raised); it judges neither equivalence nor conformity, and a draft is never an approved change. The shared cases
//! are `fixtures/commands/v1/cad.zoning.draft.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_ZONING_DRAFT: &str = "cad.zoning.draft";
pub const CAD_ZONING_DRAFT_VERSION: u32 = 1;

/// The uses that are social or technical infrastructure areas (Mekânsal Planlar Yapım Yönetmeliği m. 4: education,
/// health, religious, cultural, social and administrative buildings, sports, parks, green areas and squares; technical
/// infrastructure and car parks), roads left out as m. 26(3)(b) leaves them out.
pub const INFRASTRUCTURE_USES: [&str; 12] =
    ["egitim", "saglik", "dini", "sosyal", "kultur", "spor", "resmi", "teknik", "otopark", "park", "yesil", "meydan"];

/// A change the draft makes: a decision area, its field and the value it takes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningDraftChange {
    pub uid: String,
    pub field: String,
    pub value: String,
}

/// Input of `cad.zoning.draft` v1: the draft's id; the plan (`PlanKimlik`), the reason and the changes it keeps (a draft
/// of the id is replaced); or `remove`, which takes it out of the project.
///
/// Refusals, in this order: `invalid_revision`, `revision_conflict`; `invalid_input` (`id` empty, with a space at its
/// ends or longer than 64 characters); with `remove`, `unknown_draft` (no draft of the id); `layer_not_found` (the
/// drawing has no plan boundary or decision layer); `invalid_input` (`plan` empty), `unknown_plan` (no plan boundary
/// of the id); `invalid_input` (`reason` empty or longer than 4000 characters; `changes` empty); for `changes[i]`:
/// `invalid_input` (the area's field given twice), `invalid_uid`, `entity_not_found`, `not_a_decision`, `invalid_input`
/// (an area of another plan, a field a draft does not change, a value empty, not in the list or not a number of 0 or
/// more, the area's value already).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningDraft {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub plan: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ZoningDraftChange>>"))]
    pub changes: Vec<ZoningDraftChange>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub remove: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// A use's area in the plan before and after the draft (square metres); the use empty for areas without one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningDraftUse {
    #[serde(rename = "use")]
    #[cfg_attr(feature = "ts", ts(rename = "use"))]
    pub usage: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub before: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub after: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub difference: f64,
}

/// What Mekânsal Planlar Yapım Yönetmeliği m. 26 asks of the draft: `infrastructureLess` (a social or technical
/// infrastructure use's area gets smaller: m. 26(3); `area` how much), `densityUp` (an area's KAKS or KatAdedi rises:
/// m. 26(5)(a)), `heightUp` (an area's KatAdedi or Hmax rises: m. 26(4), (5)(b)); the area's id where it is one area's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningDraftFinding {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[serde(rename = "use")]
    #[cfg_attr(feature = "ts", ts(rename = "use"))]
    pub usage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: Option<f64>,
    pub message: String,
}

/// Output of `cad.zoning.draft` v1: the draft as kept (none when removed), its plan's level, the plan's areas by use
/// (in the list's order, then areas without a use), what m. 26 asks, the project's drafts after it and the revision.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ZoningDrafted {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub draft: Option<crate::PlanDraft>,
    pub level: String,
    pub uses: Vec<ZoningDraftUse>,
    pub findings: Vec<ZoningDraftFinding>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub drafts: u32,
    pub revision: String,
}

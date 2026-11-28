//! `cad.zoning.parcel` v1 (“Parselin plan kararları”, docs/adr/8072 §5; plan/bim-plan.md BIM-11), read only: the
//! zoning plan's decisions that fall on a parcel (the building's by default; KentOS Mimari's, for İmar durumu), as the drawing's zoning plan layers (the profile
//! `imar-plani`, from the municipality's CBS) give them: each decision area's share of the parcel, its values as
//! written (use, building order, TAKS, KAKS, storeys, Hmax, setbacks, the values the plan does not give), the plan it
//! belongs to (its id, name, level, status, version, approval date, basis) and its plan notes; the parcel's area under
//! no decision; two decisions over the same piece of the parcel saying different things (a conflict, both shown); the
//! project's typed İmar durumu where it differs from the plan. A drawn intersection is no legal ruling: KentOS shows the
//! decisions with their source and writes nothing; İmar durum belgesi (PAİY 4/1-ll) stays the administration's.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use crate::cad::UID_TEXT;

/// The command's name.
pub const CAD_ZONING_PARCEL: &str = "cad.zoning.parcel";
/// Its version.
pub const CAD_ZONING_PARCEL_VERSION: u32 = 1;

/// Input of `cad.zoning.parcel` v1: the parcel by its persistent id (a closed line or a polygon; absent: the site
/// plan's, docs/adr/8032) and the width of the band along its boundary an overlap must pass to count, metres (absent
/// 0.01, as İmar planı denetimi's tolerance).
///
/// Refusals: `parcel_missing`, `entity_not_found`, `not_a_parcel`, `invalid_input`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ZoningParcel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub parcel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// A decision's value as the plan writes it (its field in the profile `imar-plani`: Kullanim, Nizam, TAKS, KAKS,
/// KatAdedi, Hmax, OnCekme, YanCekme, ArkaCekme, Belirtilmeyen); the text kept, never read through a number.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlanValue {
    pub field: String,
    pub value: String,
}

/// The plan a decision belongs to: its boundary's persistent id and its fields as written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlanSource {
    pub uid: String,
    /// PlanKimlik.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Seviye (msp, cdp, nip, uip).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// Durum (taslak, askida, onayli, yururlukte, iptal).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// OnayTarihi as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<String>,
    /// Dayanak: the document the plan rests on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis: Option<String>,
}

/// A plan note a decision names (or a note of its whole plan): its number, text and scope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlanNoteText {
    pub number: String,
    pub text: String,
    /// Kapsam: genel (the whole plan) or alan (the decisions that name it).
    pub scope: String,
}

/// A decision area on the parcel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ParcelDecision {
    pub uid: String,
    /// KararKimlik.
    pub id: String,
    /// The parcel's area under it, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub share: f64,
    /// Its values as written, in the profile's order; a field it leaves empty is not here.
    pub values: Vec<PlanValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<PlanSource>,
    /// The notes it names, then its plan's general notes, by number.
    pub notes: Vec<PlanNoteText>,
}

/// Two decisions saying different things over the same piece of the parcel: the field, the piece's area (m²), each
/// decision's id and value as written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlanConflict {
    pub field: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    pub decisions: Vec<String>,
    pub values: Vec<String>,
}

/// The project's typed İmar durumu where the plan's decisions on the parcel say otherwise: KAKS (emsal) or TAKS, the
/// typed value and the plan's as written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlanAgainst {
    pub field: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub typed: f64,
    pub plan: Vec<String>,
}

/// What `cad.zoning.parcel` answers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ZoningParcelDecided {
    pub parcel: String,
    /// The parcel's area, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    /// In the drawing's order.
    pub decisions: Vec<ParcelDecision>,
    /// The parcel's area under no decision, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub uncovered: f64,
    pub conflicts: Vec<PlanConflict>,
    pub against: Vec<PlanAgainst>,
    /// What else is to be said, a sentence each: a plan not in force or without an approval date, a note named but not
    /// drawn, a parcel under several decisions (a kademe hattı, PAİY 4/1-mm), no zoning plan layer.
    pub notes: Vec<String>,
}

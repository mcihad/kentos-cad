//! `cad.parcels.check` v1 (plan/map-cad.md MAP-08, docs/adr/1400): Parsel alan denetimi, read only. Each parcel's
//! geometric area (computed from its coordinates, arcs and holes included) beside the area its record gives
//! (`Tapu alanı (m²)`, kept as written), the difference and the tolerance of the 2022 regulation's Article 8(2) for
//! archive-approved geometries (Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik, RG 27.09.2022 /
//! 31966): within, outside, no record, or a record that cannot be read. The identity is the parcel's place, İl, İlçe,
//! Mahalle, Ada and Parsel together, never the parcel number alone: two parcels of one identity and a parcel without
//! Ada or Parsel are reported. The check writes nothing and claims no legal conformity: the geometric area never
//! overwrites the recorded one. The shared cases are `fixtures/commands/v1/cad.parcels.check.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_PARCELS_CHECK: &str = "cad.parcels.check";
pub const CAD_PARCELS_CHECK_VERSION: u32 = 1;

/// The rule the tolerance comes from, named in the answer (CLAUDE.md §23.4: the result says what made it).
pub const PARCEL_AREA_RULE: &str = "tkgm-2022-m8-arsiv";

/// Input of `cad.parcels.check` v1: the parcels (none: every area on the parcel layer `parsel`) and the attribute
/// that holds the recorded area (none: `Tapu alanı (m²)`).
///
/// Refusals: `invalid_uid` and `entity_not_found` (`uids[i]`), `not_a_parcel` (`uids[i]`: not an area).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
}

/// A parcel as checked: its identity's parts as written (empty when missing), its geometric area (square metres),
/// its recorded area's text as written and its value when it reads, the difference (geometric − recorded), the
/// tolerance from the geometric area, and the status: `within`, `outside`, `noRecord` or `invalidRecord`;
/// `duplicate` when another parcel has the same identity, `noIdentity` when Ada or Parsel is missing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelAreaCheck {
    pub uid: String,
    pub il: String,
    pub ilce: String,
    pub mahalle: String,
    pub ada: String,
    pub parsel: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recorded_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub recorded: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub difference: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub tolerance: f64,
    pub status: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub duplicate: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub no_identity: bool,
}

/// Output of `cad.parcels.check` v1: the rule, the recorded area's attribute, every parcel in the drawing's order,
/// the counts by status, the identities held by more than one parcel (`İl/İlçe/Mahalle/Ada/Parsel`), and the parcels
/// without Ada or Parsel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParcelsChecked {
    pub rule: String,
    pub field: String,
    pub parcels: Vec<ParcelAreaCheck>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub within: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub no_record: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub invalid_record: u32,
    pub duplicates: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub no_identity: u32,
}

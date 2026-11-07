//! `cad.readjustment.scope` v1 (plan/map-cad.md MAP-11, docs/adr/2060): Düzenleme kapsamı, read only. The parcels a
//! land readjustment's boundary (düzenleme sınırı, a closed area) takes in, with the numbers of the separation sheet
//! (ayırma çapı) and the area the distribution rests on, by Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik
//! (RG 22.02.2020 / 31047) Articles 13 and 31:
//!
//! - every parcel's registered area as written and its area computed from its coordinates, their difference and the
//!   tolerance (yanılma sınırı; the 2022 regulation's Article 8(2) table, as `cad.parcels.check` gives it, Article
//!   31(3) referring to that regulation);
//! - the part inside the boundary and the part outside, computed exactly (arcs stay arcs), and the outside pieces;
//! - the basis: a whole parcel within the tolerance takes its registered area (31(1)); a partial parcel takes the
//!   computed area of its part inside at the given places and rounding (31(2)), the area error staying with the part
//!   outside (registered − basis);
//! - no basis where it needs a person: a difference beyond the tolerance (31(1): the corrected area, which is the
//!   cadastre's work), no or unreadable record, a registered area finer than the places, an outside part smaller than
//!   the error, or a piece narrower than the tolerance (the boundary passes a parcel's edge closely: drawn off it).
//!
//! The geometric area never overwrites the registered one (CLAUDE.md §7, §23). The shared cases are
//! `fixtures/commands/v1/cad.readjustment.scope.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_READJUSTMENT_SCOPE: &str = "cad.readjustment.scope";
pub const CAD_READJUSTMENT_SCOPE_VERSION: u32 = 1;

/// Input of `cad.readjustment.scope` v1: the boundary (a closed area's uid), the parcels (none: every area on the
/// parcel layer `parsel` that has a part inside it), the attribute that holds the registered area (none:
/// `Tapu alanı (m²)`), the places and rounding of a computed basis (the rule package's numeric policy: `half_even`,
/// `half_away_from_zero`, `half_toward_zero`, `away_from_zero`, `toward_zero`, `toward_positive`, `toward_negative`),
/// and the narrowest piece counted (metres, mean width 2A/P; none: 0.01).
///
/// Refusals, in this order: `invalid_input` (`scale` over 6, `rounding` unknown, `tolerance` not finite or under
/// 1e-6), `invalid_uid`, `entity_not_found` and `not_an_area` (`boundary`), then `invalid_uid`, `entity_not_found` and
/// `not_a_parcel` (`uids[i]`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentScope {
    pub boundary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scale: u32,
    pub rounding: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// A parcel in the scope: its uid and its records' name (`Ada/Parsel`), its identity's parts as written, how the
/// boundary takes it (`whole`, `partial`, `outside`), its registered area's text as written, its computed area, the
/// difference (computed − registered) and the tolerance (from the computed area) and whether it is within; the parts
/// inside and outside (square metres), the outside pieces counted (each gets its own parcel number, Article 13(3)) and
/// the pieces narrower than the tolerance; the basis and the area the part outside keeps (decimal text, square
/// metres); the status: `registered`, `computed`, `needsCorrection`, `noRecord`, `invalidRecord`, `finerThanPolicy`,
/// `outsideTooSmall`, `sliver` or `outside`. `duplicate` when another parcel in the answer has the same `Ada/Parsel`,
/// `noIdentity` when Ada or Parsel is missing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentScopeParcel {
    pub uid: String,
    pub parcel: String,
    pub il: String,
    pub ilce: String,
    pub mahalle: String,
    pub ada: String,
    pub parsel: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub recorded_text: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub calculated: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub difference: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub limit: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub within: Option<bool>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub inside: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub outside: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub pieces: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub slivers: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub basis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub outside_recorded: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub duplicate: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub no_identity: bool,
}

/// Output of `cad.readjustment.scope` v1: the tolerance's rule (`cad.parcels.check`'s), the registered area's
/// attribute, the places, rounding and piece tolerance used; the boundary's area, the parcels' parts inside it and
/// what of the boundary no parcel in the answer covers (roads, water … when the parcels do not overlap; square
/// metres); the bases' exact sum (decimal text); the counts (whole, partial, with a basis, needing review); the parcels
/// in the drawing's order (or the uids').
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReadjustmentScoped {
    pub rule: String,
    pub field: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scale: u32,
    pub rounding: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub boundary_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub inside: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub uncovered: f64,
    pub basis: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub whole: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub partial: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub ready: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub review: u32,
    pub parcels: Vec<ReadjustmentScopeParcel>,
}

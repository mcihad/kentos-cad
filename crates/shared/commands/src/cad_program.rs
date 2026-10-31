//! The building program's commands (docs/adr/0773; plan/architecture-advanced.md ADV-04): the program written —
//! buildings, wings, units, common and outdoor areas and the design brief — with the spaces it groups
//! (`cad.program.edit`), and the program read against the drawing (`cad.program.check`, read-only): each group's totals
//! without a space summed twice, each space's measures against its brief, and the findings. A brief is the designer's
//! target, never a legal limit. Lengths are metres and areas square metres on the wire (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_arch_audit::ArchAuditFinding;
use crate::entity::Vec2;
use crate::{BuildingProgram, ProgramShares, ProgramUnitShare, SpaceMethod, UnitKind};

pub const CAD_PROGRAM_EDIT: &str = "cad.program.edit";
pub const CAD_PROGRAM_EDIT_VERSION: u32 = 1;
pub const CAD_PROGRAM_CHECK: &str = "cad.program.check";
pub const CAD_PROGRAM_CHECK_VERSION: u32 = 1;

/// Spaces given to a group, or taken out of theirs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramAssign {
    /// The spaces, by their persistent ids.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// The group's id; null: out of any group.
    #[serde(default)]
    pub unit: Option<String>,
}

/// Input of `cad.program.edit` v1 (“Bina programı”): the program as it is to be (absent: as it is) and the spaces
/// given to its groups, written as one undo step. A group spaces still name is not taken away unless they are given
/// elsewhere in the same call.
///
/// Refusals, in this order: `nothing_to_set`, `building_in_use` (a building or a wing taken away that groups still
/// name: the words list their numbers; docs/adr/0784 §1), `invalid_program` (the program's own problem, at its path),
/// `level_not_found` (a building's or a wing's level the project lacks, at its path), `no_entities`, `invalid_uid`,
/// `entity_not_found`, `not_a_space` (`assign[i].uids[k]`), `unit_not_found` (`assign[i].unit`), `unit_in_use` (a group
/// taken away that spaces still name: their ids), `invalid_revision`, `revision_conflict`, `layer_locked`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramEdit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub program: Option<BuildingProgram>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProgramAssign>>", optional))]
    pub assign: Vec<ProgramAssign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.program.edit` v1 (its plan's too): the program as written and the spaces whose group changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramEdited {
    pub program: BuildingProgram,
    /// The spaces whose group changed, in the drawing's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub assigned: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.program.check` v1 (read-only): the groups and levels read (each absent: all), the findings asked.
/// A long check: run as a job it says how far it has come and stops when asked.
///
/// Refusals: `unit_not_found` (`units[i]`), `level_not_found` (`levels[i]`), `cancelled` (stopped).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub levels: Option<Vec<String>>,
    /// Absent or true: the findings too; false: the numbers only (the passage scan skipped).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub findings: Option<bool>,
}

/// A group's totals, each space summed once: its rooms', its outdoor spaces' (briefs marked outdoor: a balcony) and its
/// voids', m².
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramUnitRow {
    pub id: String,
    pub number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    pub kind: UnitKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub building: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wing: Option<String>,
    /// The levels its spaces lie on, in the project's order (a duplex: two); a space without a level gives none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub levels: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub spaces: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub rooms: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub outdoor: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub voids: f64,
    /// The methods its spaces are measured by, in their order of first use: one name tells a reader how its area was
    /// taken; two or more, that it mixes them.
    pub methods: Vec<SpaceMethod>,
    /// The recorded area the program keeps for it, as given; never written by a measurement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub recorded_area: Option<f64>,
    /// Its rooms less its recorded area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub recorded_difference: Option<f64>,
    /// An independent unit's part of the common areas when the program shares them (docs/adr/0784 §2): its own rooms,
    /// its share and its gross, m² as decimals, always of all its spaces whatever the check reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub share: Option<ProgramUnitShare>,
}

/// The drawing's totals, each space once: by kind of group, and the spaces in none.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramTotals {
    /// The independent units' rooms.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub units: f64,
    /// The independent units' outdoor spaces.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub unit_outdoor: f64,
    /// The common areas' spaces (rooms and outdoor).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub common: f64,
    /// The outdoor areas' spaces.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub outdoor: f64,
    /// The voids, whatever their group.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub voids: f64,
    /// The spaces in no group, or in one the program lacks.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub unassigned: f64,
    /// All the spaces' areas but the voids: the sum of the others.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub all: f64,
}

/// A brief's neighbour or reach: whether a space of the usage is one, and which.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramLink {
    pub usage: String,
    /// The spaces of the usage that are, in the drawing's order; empty: none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub by: Vec<String>,
}

/// A space read against its brief.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramSpaceRow {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub usage: Option<String>,
    /// Its group's id as the space names it (one the program lacks too).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    /// How its area was taken: its rings' method (net: the walls' finished inner faces).
    pub method: SpaceMethod,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub void: bool,
    /// Summed as its group's outdoor area (its brief says outdoor).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub outdoor: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    /// Its net width: the largest circle inside it, its diameter (within 1 mm).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Its doors and openings into other spaces, counted.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub doors: u32,
    /// Its net passage (a brief with a least passage, two doors or more): the widest a body may be and still go
    /// between all its doors (within 1 mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub passage: Option<f64>,
    /// Where it is narrowest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub passage_at: Option<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub target_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub target_max: Option<f64>,
    /// How far its area lies outside its brief's range (below minus, above plus, inside 0); none without a range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub target_difference: Option<f64>,
    /// Its passage less its brief's least passage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub passage_difference: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProgramLink>>", optional))]
    pub adjacent: Vec<ProgramLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ProgramLink>>", optional))]
    pub reach: Vec<ProgramLink>,
}

/// Output of `cad.program.check` v1 (its plan's too).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProgramChecked {
    /// The rule the numbers come from and the brief's version (`kentos.program/1 · …`).
    pub rule: String,
    /// The program's groups, in its order, with their totals.
    pub units: Vec<ProgramUnitRow>,
    pub totals: ProgramTotals,
    /// The spaces read, in the drawing's order.
    pub spaces: Vec<ProgramSpaceRow>,
    /// The findings (the audit's group `program`), by code then along x and y.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ArchAuditFinding>>", optional))]
    pub findings: Vec<ArchAuditFinding>,
    /// The common areas shared out by the program's rule (docs/adr/0784 §2), of the whole program whatever the check
    /// reads; none when the program shares nothing. A pool not shared is said as a warning too (`share_unshared`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shares: Option<ProgramShares>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

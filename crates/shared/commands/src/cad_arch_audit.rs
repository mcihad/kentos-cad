//! Mimari denetim's query of the product command catalog (docs/adr/0703; plan/architecture.md ARCH-DOC-03): the
//! drawing's architectural problems in one list — walls that do not join or stop short of another, openings, spaces,
//! dimensions, columns and view marks whose links broke, numbers given twice, objects without a level — each with what
//! resolves it and, where one is plain, the command call that does (`fix`). It writes nothing; a list without findings
//! is no compliance with any regulation or standard.
//!
//! `cad.arch.exceptions` marks findings as left on purpose (docs/adr/0732 §3), kept in the project's settings and bound
//! to the findings as they were then.
//!
//! Lengths are metres and areas square metres on the wire, whatever unit the project shows (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_changeset::ChangesetStep;
use crate::entity::Vec2;

pub const CAD_ARCH_AUDIT: &str = "cad.arch.audit";
pub const CAD_ARCH_AUDIT_VERSION: u32 = 1;
/// Marks findings as left on purpose, takes the mark away, or drops those whose finding is gone (docs/adr/0732 §3).
pub const CAD_ARCH_EXCEPTIONS: &str = "cad.arch.exceptions";
pub const CAD_ARCH_EXCEPTIONS_VERSION: u32 = 1;
/// An open wall shorter than this is said: a leftover piece; metres (docs/adr/0732 §4).
pub const ARCH_AUDIT_MIN_LENGTH: f64 = 0.05;
/// A wall end this close to another wall's body, or to another free end, is said; metres (docs/adr/0703 §1).
pub const ARCH_AUDIT_MAX_GAP: f64 = 0.10;
/// A space smaller than this is said; square metres.
pub const ARCH_AUDIT_MIN_AREA: f64 = 0.10;
/// A space whose mean width (twice its area over its perimeter) is less than this is said; metres.
pub const ARCH_AUDIT_MIN_WIDTH: f64 = 0.15;

/// A group of the audit's checks, in the order its findings come.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArchAuditCheck {
    /// Joins that are not made, ends that stop short of another wall or of another end.
    Walls,
    /// Openings whose wall link broke, openings without a type.
    Openings,
    /// Spaces whose boundary broke, very small or very narrow spaces.
    Spaces,
    /// A number on more than one opening or space.
    Numbers,
    /// Tags whose object is gone.
    Tags,
    /// Associative dimensions and dimension sets whose link broke.
    Dimensions,
    /// A grid label given twice on one level (docs/adr/0725 §6).
    Grids,
    /// Columns whose grids are gone.
    Columns,
    /// View marks whose target is gone.
    Viewmarks,
    /// Walls, spaces, columns, stairs, ramps and room separators without a level while the project has levels.
    Levels,
    /// Ramps measured against an accessibility profile (docs/adr/0723 §4); nothing unless one is given.
    Accessibility,
    /// Stairs measured against a stair profile (docs/adr/0724 §2), and their own checks (docs/adr/0732 §4).
    Stairs,
    /// The ramps' own checks: no sloped run, a given slope its rise and run do not make (docs/adr/0732 §4).
    Ramps,
    /// Kitchen runs whose wall is gone or that can no longer be laid (docs/adr/0732 §4).
    Casework,
    /// The building program's findings (docs/adr/0773 §5): spaces against their design brief, doors, passages,
    /// fixtures' clearances; nothing unless the project has a program.
    Program,
}

impl ArchAuditCheck {
    pub const ALL: [ArchAuditCheck; 15] = [
        ArchAuditCheck::Walls,
        ArchAuditCheck::Openings,
        ArchAuditCheck::Spaces,
        ArchAuditCheck::Numbers,
        ArchAuditCheck::Tags,
        ArchAuditCheck::Dimensions,
        ArchAuditCheck::Grids,
        ArchAuditCheck::Columns,
        ArchAuditCheck::Viewmarks,
        ArchAuditCheck::Levels,
        ArchAuditCheck::Accessibility,
        ArchAuditCheck::Stairs,
        ArchAuditCheck::Ramps,
        ArchAuditCheck::Casework,
        ArchAuditCheck::Program,
    ];

    /// Its name on the wire.
    pub fn name(self) -> &'static str {
        match self {
            ArchAuditCheck::Walls => "walls",
            ArchAuditCheck::Openings => "openings",
            ArchAuditCheck::Spaces => "spaces",
            ArchAuditCheck::Numbers => "numbers",
            ArchAuditCheck::Tags => "tags",
            ArchAuditCheck::Dimensions => "dimensions",
            ArchAuditCheck::Grids => "grids",
            ArchAuditCheck::Columns => "columns",
            ArchAuditCheck::Viewmarks => "viewmarks",
            ArchAuditCheck::Levels => "levels",
            ArchAuditCheck::Accessibility => "accessibility",
            ArchAuditCheck::Stairs => "stairs",
            ArchAuditCheck::Ramps => "ramps",
            ArchAuditCheck::Casework => "casework",
            ArchAuditCheck::Program => "program",
        }
    }

    /// Its name in a panel.
    pub fn label(self) -> &'static str {
        match self {
            ArchAuditCheck::Walls => "Duvarlar",
            ArchAuditCheck::Openings => "Açıklıklar",
            ArchAuditCheck::Spaces => "Mahaller",
            ArchAuditCheck::Numbers => "Numaralar",
            ArchAuditCheck::Tags => "Etiketler",
            ArchAuditCheck::Dimensions => "Ölçüler",
            ArchAuditCheck::Grids => "Akslar",
            ArchAuditCheck::Columns => "Kolonlar",
            ArchAuditCheck::Viewmarks => "Görünüm işaretleri",
            ArchAuditCheck::Levels => "Katlar",
            ArchAuditCheck::Accessibility => "Erişilebilirlik",
            ArchAuditCheck::Stairs => "Merdivenler",
            ArchAuditCheck::Ramps => "Rampalar",
            ArchAuditCheck::Casework => "Mutfak dizileri",
            ArchAuditCheck::Program => "Bina programı",
        }
    }
}

/// Which findings marked as exceptions `cad.arch.audit` lists (docs/adr/0732 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArchAuditExceptionView {
    /// Left out and counted (`excepted`).
    #[default]
    Hide,
    /// Listed with the open ones, marked `exception`.
    Show,
    /// Only they are listed.
    Only,
}

/// The part of the drawing `cad.arch.audit` reports on (docs/adr/0732 §2): a finding is kept when one of its objects
/// lies on one of `levels` and one of them is among `uids`; either absent: no bound.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchAuditWithin {
    /// Levels by their ids; null, the objects on no level. An object's level is the level view's (docs/adr/0715): an
    /// opening's its wall's, a tag's its object's, a dimension's its bonds'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub levels: Option<Vec<Option<String>>>,
    /// Objects by their persistent ids (Revit's Show Related Warnings).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Option<Vec<String>>,
}

/// Input of `cad.arch.audit` v1 (“Mimari denetim”, read-only, docs/adr/0703, 0732): the checks asked (none: every one),
/// the thresholds (absent: [`ARCH_AUDIT_MAX_GAP`], [`ARCH_AUDIT_MIN_AREA`], [`ARCH_AUDIT_MIN_WIDTH`],
/// [`ARCH_AUDIT_MIN_LENGTH`]), the part of the drawing (`within`) and the exceptions listed.
///
/// Refusals, in this order: `not_finite` (`maxGap`, `minArea`, `minWidth`, `minLength`), `invalid_arch_audit` (a
/// threshold below zero, in the same order; a profile the catalog lacks, two with ramp or with stair rules),
/// `level_not_found` (`level`, then `within.levels[i]`), `entity_not_found` (`within.uids[i]`). Warning:
/// `exception_lapsed` (an exception whose finding changed: listed open again).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchAudit {
    /// The groups to check; absent or empty: every group. The findings keep the groups' own order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ArchAuditCheck>>", optional))]
    pub checks: Vec<ArchAuditCheck>,
    /// A free wall end this near another wall's body, or another free end, is said; metres, at least 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max_gap: Option<f64>,
    /// A space smaller than this is said; square metres, at least 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub min_area: Option<f64>,
    /// A space whose mean width (2 × area / perimeter) is less than this is said; metres, at least 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min_width: Option<f64>,
    /// The level a level finding's fix gives (docs/adr/0717: a host's Etkin kat), by its id among the project's;
    /// absent: the project's ground level (the one at elevation 0, else its first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    /// The profiles the ramps and the stairs are measured against (docs/adr/0723 §4, 0724 §2), by their ids
    /// ([`ARCH_PROFILES`]): at most one with ramp rules and one with stair rules; none: those measures are not taken.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub profiles: Vec<String>,
    /// An open wall shorter than this is said; metres, at least 0 (docs/adr/0732 §4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min_length: Option<f64>,
    /// The part of the drawing reported on; absent: all of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub within: Option<ArchAuditWithin>,
    /// The exceptions listed; absent: hidden and counted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub exceptions: Option<ArchAuditExceptionView>,
}

/// What resolves a finding: in words, and as the command call that does it (`cad.changeset.apply`'s step, run as it
/// is; one undo step).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchAuditFix {
    /// What the call does, as a button says it (“Bitiş ucunu öbür duvara uzat”).
    pub label: String,
    pub step: ChangesetStep,
}

/// One thing `cad.arch.audit` found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchAuditFinding {
    /// Its lasting key (docs/adr/0732 §1): `code:uids`, a wall end's `:start` or `:end`, `#2` for a second alike.
    pub key: String,
    pub check: ArchAuditCheck,
    /// `wall_join_overlap`, `wall_join_crowded`, `wall_join_acute`, `wall_join_unresolved`, `arch_wall_end_gap`,
    /// `arch_wall_ends_apart`, `arch_opening_broken`, `tag_missing_type`, `arch_space_broken`, `arch_space_small`,
    /// `arch_space_narrow`, `tag_duplicate_number`, `tag_target_gone`, `arch_dimension_set_broken`,
    /// `arch_dimension_broken`, `arch_grid_label_duplicate`, `arch_column_grid_gone`, `arch_viewmark_target_gone`,
    /// `arch_level_missing`,
    /// `ramp_slope_over_profile`, `ramp_long_run_slope`, `ramp_landing_short`, `ramp_turn_narrow`,
    /// `stair_riser_over_profile`, `stair_tread_short`, `stair_step_over_profile`, `stair_width_narrow`,
    /// `stair_landing_short`, `stair_winder_narrow`, `stair_railing_missing`; `arch_wall_short`,
    /// `arch_wall_overlap`, `arch_separator_overlap`, `tag_empty`, `stair_short`, `stair_long`, `stair_rise_level`,
    /// `ramp_no_run`, `ramp_slope_mismatch` or `arch_casework_broken` (docs/adr/0732 §4); the program group's
    /// `program_unit_missing`, `program_area_below`, `program_area_above`, `program_width_narrow`,
    /// `program_passage_narrow`, `program_adjacent_missing`, `program_reach_missing`, `program_no_door`,
    /// `program_fixture_clearance` (docs/adr/0773 §5).
    pub code: String,
    /// What is wrong.
    pub message: String,
    /// What resolves it.
    pub repair: String,
    /// The objects, by their persistent ids.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// Where to look.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The call that resolves it, when one plainly does and the object it writes is on an unlocked layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fix: Option<ArchAuditFix>,
    /// Marked as left on purpose, and the finding is as it was then (docs/adr/0732 §3).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub exception: bool,
    /// Marked as left on purpose, but the finding changed since: open again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub lapsed: bool,
    /// The exception's note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

/// Output of `cad.arch.audit` v1 (its plan's too).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchAudited {
    /// By group, then by code within it, then along x and y.
    pub findings: Vec<ArchAuditFinding>,
    /// The findings `within` left out.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub outside: u32,
    /// The findings in the part reported on that are exceptions, listed or not.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub excepted: u32,
    /// The keys of the project's exceptions whose finding the audit no longer finds (`cad.arch.exceptions`' prune).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub stale: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.arch.exceptions` does (docs/adr/0732 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArchExceptionOperation {
    /// İstisna yap: the findings of `keys` left on purpose, bound to the findings as they are now (a lapsed exception is
    /// marked again; a note given replaces the old one).
    Mark,
    /// İstisnayı kaldır: the exceptions of `keys` taken away.
    Unmark,
    /// İstisnaları temizle: the exceptions whose finding the audit no longer finds taken away.
    Prune,
}

/// Input of `cad.arch.exceptions` v1: Mimari denetim's findings marked as left on purpose, or the marks taken away, as
/// one undo step (“İstisna”, “İstisnayı kaldır”, “İstisnaları temizle”). The findings are found by `audit` (absent: every
/// group with the default thresholds and no profile); its `within` and `exceptions` are not read.
///
/// Refusals, in this order: `invalid_input` (mark or unmark without a key, a key twice, a note of more than 1000
/// characters), the audit's own (its paths under `audit.`), `finding_not_found` (`keys[i]`: no finding of the drawing as
/// it is now has it, for mark; no exception has it, for unmark). Prune with nothing to drop, or marks already as asked,
/// change nothing and write no step. `expectedRevision` given and the drawing not at it: nothing is written, a conflict.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchExceptions {
    pub operation: ArchExceptionOperation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub keys: Vec<String>,
    /// Why the findings are left, for mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub audit: Option<ArchAudit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.arch.exceptions` v1, and its plan: the keys marked, unmarked or dropped (those already so are not
/// counted), how many exceptions the project keeps after it, and the revision after it (a plan: before it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchExceptionsEdited {
    pub changed: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub exceptions: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// An architectural profile (docs/adr/0723 §4, 0724 §2): versioned values ramps and stairs are measured against, with
/// their sources and what they are not (a regulation's approval).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArchProfile {
    /// `kentos.arch-profile`.
    pub format: String,
    pub version: u32,
    /// Its id with its version (`erisilebilirlik-rampa/1`).
    pub id: String,
    pub name: String,
    pub note: String,
    pub sources: Vec<ProfileReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ramp: Option<RampRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stair: Option<StairRules>,
}

/// Where a profile's values come from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileReference {
    pub title: String,
    pub url: String,
}

/// A ramp's rules in a profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RampRules {
    /// The steepest slope by the rise's size, in order: the first whose `upTo` (metres) the rise does not pass, the
    /// last having none.
    pub slopes: Vec<SlopeBand>,
    /// A sloped run longer than `over` metres is at most `max` per cent steep.
    pub long_run: LongRun,
    /// A landing's least length, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub landing: f64,
    /// A ramp that turns is at least this wide at its turns, metres (a landing that turns is this square).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub turn: f64,
}

/// A stair's rules in a profile (docs/adr/0724 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StairRules {
    /// The highest riser, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub riser: f64,
    /// The shortest tread, metres (a winding stair's on its walking line).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tread: f64,
    /// Two risers and a tread (2a+b), metres: the least and the most.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub step: [f64; 2],
    /// The narrowest flight, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// A landing at least as long as the flight is wide.
    pub landing_width: bool,
    /// A winding (curved) stair's tread at its narrow end, at least; metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub winder: f64,
    /// A railing on both sides.
    pub both_railings: bool,
}

/// One band of [`RampRules::slopes`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlopeBand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub up_to: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub max: f64,
}

/// [`RampRules::long_run`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LongRun {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub over: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub max: f64,
}

/// The built-in architectural profiles: their ids and their data (`catalog/v1/arch-profiles/`), ramps' first.
pub const ARCH_PROFILES: &[(&str, &str)] = &[
    (
        "erisilebilirlik-rampa/1",
        include_str!("../../../../catalog/v1/arch-profiles/erisilebilirlik-rampa-1.json"),
    ),
    (
        "imar-merdiven-konut/1",
        include_str!("../../../../catalog/v1/arch-profiles/imar-merdiven-konut-1.json"),
    ),
    (
        "imar-merdiven-konut-asansorsuz/1",
        include_str!("../../../../catalog/v1/arch-profiles/imar-merdiven-konut-asansorsuz-1.json"),
    ),
    (
        "imar-merdiven-diger/1",
        include_str!("../../../../catalog/v1/arch-profiles/imar-merdiven-diger-1.json"),
    ),
    (
        "imar-merdiven-daire-ici/1",
        include_str!("../../../../catalog/v1/arch-profiles/imar-merdiven-daire-ici-1.json"),
    ),
];

/// A built-in architectural profile by its id.
pub fn arch_profile(id: &str) -> Option<ArchProfile> {
    let (_, text) = ARCH_PROFILES.iter().find(|(i, _)| *i == id)?;
    serde_json::from_str(text).ok()
}

impl RampRules {
    /// The steepest slope a ramp of rise `rise` (its size) may have, per cent.
    pub fn slope_for(&self, rise: f64) -> Option<f64> {
        let r = rise.abs();
        self.slopes.iter().find(|b| b.up_to.is_none_or(|u| r <= u)).map(|b| b.max)
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn every_built_in_profile_reads_and_is_named_by_its_id() {
        for (id, _) in ARCH_PROFILES {
            let p = arch_profile(id).expect("reads");
            assert_eq!(p.id, *id);
            assert_eq!(p.format, "kentos.arch-profile");
            assert!(p.ramp.is_some() || p.stair.is_some(), "{id}: rules");
            if let Some(r) = &p.ramp {
                assert!(r.slopes.last().is_some_and(|b| b.up_to.is_none()), "the last band has no bound");
                assert!(r.slopes.windows(2).all(|w| w[0].up_to < w[1].up_to || w[1].up_to.is_none()));
            }
            if let Some(s) = &p.stair {
                assert!(s.step[0] < s.step[1] && s.riser > 0.0 && s.tread > 0.0 && s.width > 0.0);
            }
        }
        let r = arch_profile("erisilebilirlik-rampa/1").unwrap().ramp.unwrap();
        assert_eq!(r.slope_for(0.15), Some(8.0));
        assert_eq!(r.slope_for(-0.16), Some(7.0));
        assert_eq!(r.slope_for(1.0), Some(6.0));
        assert_eq!(r.slope_for(1.01), Some(5.0));
        let s = arch_profile("imar-merdiven-konut-asansorsuz/1").unwrap().stair.unwrap();
        assert_eq!((s.riser, s.width), (0.16, 1.2));
    }
}

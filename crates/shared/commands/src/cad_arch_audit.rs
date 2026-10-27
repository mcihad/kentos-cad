//! Mimari denetim's query of the product command catalog (docs/adr/0703; plan/architecture.md ARCH-DOC-03): the
//! drawing's architectural problems in one list — walls that do not join or stop short of another, openings, spaces,
//! dimensions, columns and view marks whose links broke, numbers given twice, objects without a level — each with what
//! resolves it and, where one is plain, the command call that does (`fix`). It writes nothing; a list without findings
//! is no compliance with any regulation or standard.
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
    /// Columns whose grids are gone.
    Columns,
    /// View marks whose target is gone.
    Viewmarks,
    /// Walls, spaces, columns, stairs and ramps without a level while the project has levels.
    Levels,
}

impl ArchAuditCheck {
    pub const ALL: [ArchAuditCheck; 9] = [
        ArchAuditCheck::Walls,
        ArchAuditCheck::Openings,
        ArchAuditCheck::Spaces,
        ArchAuditCheck::Numbers,
        ArchAuditCheck::Tags,
        ArchAuditCheck::Dimensions,
        ArchAuditCheck::Columns,
        ArchAuditCheck::Viewmarks,
        ArchAuditCheck::Levels,
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
            ArchAuditCheck::Columns => "columns",
            ArchAuditCheck::Viewmarks => "viewmarks",
            ArchAuditCheck::Levels => "levels",
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
            ArchAuditCheck::Columns => "Kolonlar",
            ArchAuditCheck::Viewmarks => "Görünüm işaretleri",
            ArchAuditCheck::Levels => "Katlar",
        }
    }
}

/// Input of `cad.arch.audit` v1 (“Mimari denetim”, read-only, docs/adr/0703): the checks asked (none: every one) and
/// the thresholds (absent: [`ARCH_AUDIT_MAX_GAP`], [`ARCH_AUDIT_MIN_AREA`], [`ARCH_AUDIT_MIN_WIDTH`]).
///
/// Refusals, in this order: `not_finite` (`maxGap`, `minArea`, `minWidth`), `invalid_arch_audit` (a threshold below
/// zero, in the same order).
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
    pub check: ArchAuditCheck,
    /// `wall_join_overlap`, `wall_join_crowded`, `wall_join_acute`, `wall_join_unresolved`, `arch_wall_end_gap`,
    /// `arch_wall_ends_apart`, `arch_opening_broken`, `tag_missing_type`, `arch_space_broken`, `arch_space_small`,
    /// `arch_space_narrow`, `tag_duplicate_number`, `tag_target_gone`, `arch_dimension_set_broken`,
    /// `arch_dimension_broken`, `arch_column_grid_gone`, `arch_viewmark_target_gone` or `arch_level_missing`.
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
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

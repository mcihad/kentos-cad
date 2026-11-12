//! `cad.standards.exceptions` v1 (docs/adr/1129): standards findings accepted with a reason, or the acceptance
//! withdrawn. A finding is named as `cad.standards.check` gives it for the drawing as it is now (its kind, subject and
//! layer) against the standard the check is given with its source; the acceptance keeps the source's revision and what
//! the drawing had, so it lapses once either changes. The accepted findings are the project's settings
//! (`standardExceptions`): writing them makes the drawing dirty and is no undo step, as topology exceptions
//! (docs/adr/0589 §3, 1260 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{DrawingStandard, StandardIssueKind, StandardOrigin, StandardResources};

/// Accepts standards findings with a reason, or withdraws the acceptance.
pub const CAD_STANDARDS_EXCEPTIONS: &str = "cad.standards.exceptions";
pub const CAD_STANDARDS_EXCEPTIONS_VERSION: u32 = 1;

/// What `cad.standards.exceptions` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StandardExceptionOperation {
    /// The findings are kept on purpose, with `reason`.
    Accept,
    /// Their acceptance is taken away: they are open again.
    Withdraw,
}

/// A finding as `cad.standards.check` names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardFindingKey {
    pub kind: StandardIssueKind,
    pub subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
}

/// Input of `cad.standards.exceptions` v1.
///
/// - `accept`: each finding, as today's check against the standard gives it, is accepted with `reason` (and `by` and
///   `at` when given); a lapsed one is accepted again, on the standard's revision and the drawing as they are now; one
///   accepted with the same reason, by and at is left;
/// - `withdraw`: each finding's acceptance against the standard's source is taken away (the finding itself may be gone).
///
/// Refusals (`CommandError.code`), in this order: `invalid_input` (no finding, one twice, `reason`, `by` or `at`
/// with `withdraw`, a reason over 500 characters, an empty `by` or `at`); `reason_required` (`accept` without a reason
/// or with one only of white space); the standard's (`standard_not_found`, `invalid_standard`, as the check's);
/// `invalid_revision`, `revision_conflict` (status `conflict`); `finding_not_found` (`findings[i]`: today's check has no
/// such finding, or `withdraw`: none such is accepted against the source).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsExceptions {
    pub operation: StandardExceptionOperation,
    /// The findings, at least one.
    pub findings: Vec<StandardFindingKey>,
    /// The standard, as `cad.standards.check` takes it: a built-in one by its id, or the standard itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub standard: Option<DrawingStandard>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub resources: Option<StandardResources>,
    /// The standard's source (a bound package's id and revision); absent: the standard's name, no revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub origin: Option<StandardOrigin>,
    /// `accept`: why the findings are kept (not empty, at most 500 characters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
    /// `accept`: who accepts them and when (ISO 8601), recorded as given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.standards.exceptions` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsExceptionsEdited {
    /// The findings whose acceptance changed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    /// The project's accepted findings afterwards, against every source.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub exceptions: u32,
    /// The document's revision afterwards (plan: the one it was made against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

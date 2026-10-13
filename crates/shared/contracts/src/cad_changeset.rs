//! The change set (docs/adr/0253 §8): several commands written as one
//! change. Objects, blocks, layers, styles, the project's settings and the
//! sheet book changing together come and go together: a step refused takes
//! every step back, the parts outside the undo history too.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::command::CommandWarning;

/// Writes several commands as one change.
pub const CAD_CHANGESET_APPLY: &str = "cad.changeset.apply";
pub const CAD_CHANGESET_APPLY_VERSION: u32 = 1;

/// The undo step's name when the input gives none.
pub const CHANGESET_LABEL: &str = "Değişiklik kümesi";

/// Input of `cad.changeset.apply` v1: the steps, in order, each a command of
/// this host with its input as its catalog says. A later step names an
/// earlier one's output by a text that is the whole value: `"$ad"` its
/// `uid`, `"$ad[2]"` its `created[2]`, `"$ad.layer"` its `layer`,
/// `"$ad.created[0]"` the same as `"$ad[0]"`, where `ad` is the earlier
/// step's `as`. A text that names a step not before it is refused
/// (`unknown_reference`); any other text is the text.
///
/// Every step runs in the one change: a step refused refuses the set (its
/// code, at `steps[i].input.…`), and nothing of any step is written. The
/// linked texts, hatches and dimensions follow once, after the last step;
/// the document's integrity is checked then. Objects, layers and blocks are
/// one undo step named `label`; the project's settings, its style library
/// and its sheet book are no undo step (docs/adr/0248, 0251), the answer's
/// summary says which changed.
///
/// Refusals, in this order: `no_steps`; per step, in order, `nested_changeset`
/// (a change set inside), `side_effect_in_changeset` (a `file.*` command),
/// `unknown_command`, `unknown_version`; `invalid_revision`,
/// `revision_conflict`; then the steps' own, and `unknown_reference`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChangesetApply {
    /// At least one.
    pub steps: Vec<ChangesetStep>,
    /// The undo step's name; “Değişiklik kümesi” when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A step of a change set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChangesetStep {
    /// The command's id (`cad.polygon.create`).
    pub command: String,
    /// Its version; the host's when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub version: Option<u32>,
    /// Its input, its catalog's schema, with references to earlier steps.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
    /// The name later steps refer to its output by: a letter or `_`, then
    /// letters, digits and `_`; each once.
    #[serde(default, rename = "as", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, rename = "as"))]
    pub name: Option<String>,
}

/// A step written: its command and its output, as the command answers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChangesetStepDone {
    pub command: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub output: serde_json::Value,
    /// The step's warnings (the set's answer has them too, at `steps[i]…`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CommandWarning>>", optional))]
    pub warnings: Vec<CommandWarning>,
}

/// Output of `cad.changeset.apply` v1, and what its plan says it would be.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ChangesetApplied {
    /// The undo step's name.
    pub label: String,
    pub steps: Vec<ChangesetStepDone>,
    /// After execute, the document's revision after the write; after plan,
    /// the revision the input should expect for exactly this.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_step_names_itself_with_as() {
        let input: ChangesetApply = serde_json::from_value(json!({
            "steps": [
                { "command": "cad.layers.edit", "input": {}, "as": "katman" },
                { "command": "cad.line.create", "version": 1, "input": { "layerId": "$katman.layer" } }
            ],
            "label": "Yol ekseni"
        }))
        .expect("reads");
        assert_eq!(input.steps[0].name.as_deref(), Some("katman"));
        assert_eq!(input.steps[1].version, Some(1));
        let back = serde_json::to_value(&input).expect("writes");
        assert_eq!(back["steps"][0]["as"], "katman");
    }
}

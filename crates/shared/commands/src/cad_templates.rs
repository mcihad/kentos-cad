//! Work templates (docs/adr/0346 §1; plan/cad.md `CAD-ARCH-41`): a product package that starts a kind of project — its
//! unit, scale, levels, drawing standard, block library, wall styles and sheets — as data (`catalog/v1/templates/`), and
//! `cad.templates.apply`, which applies one to a drawing, keeping what the drawing already has.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{ArchLevel, DrawingUnit, SpaceMethod};

pub const CAD_TEMPLATES_APPLY: &str = "cad.templates.apply";
pub const CAD_TEMPLATES_APPLY_VERSION: u32 = 1;

/// A sheet a template brings: A3 landscape, one map frame filling its margins at `scale`, its output 300 dpi.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateSheet {
    pub id: String,
    pub name: String,
    /// The map frame's name.
    pub frame: String,
    /// The map frame's scale, 1:N.
    pub scale: u32,
}

/// A work template as `catalog/v1/templates/*.json` keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkTemplate {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub title: String,
    pub description: String,
    pub unit: DrawingUnit,
    pub plot_scale: f64,
    pub levels: Vec<ArchLevel>,
    pub space_method: SpaceMethod,
    /// The drawing standard by its built-in name (`cad.standards.apply`).
    pub standard: String,
    /// The block library by its built-in name (`cad.blocks.edit`'s import).
    pub blocks: String,
    pub sheets: Vec<TemplateSheet>,
}

/// KentOS's templates, by name.
pub const BUILTIN_TEMPLATES: &[(&str, &str)] = &[(
    "mimari",
    include_str!("../../../../catalog/v1/templates/mimari.json"),
)];

/// A built-in template by its name (case aside); none for a name KentOS has not.
pub fn builtin_template(name: &str) -> Option<WorkTemplate> {
    let key = name.trim().to_lowercase();
    BUILTIN_TEMPLATES
        .iter()
        .find(|(n, _)| *n == key)
        .and_then(|(_, text)| serde_json::from_str(text).ok())
}

/// Input of `cad.templates.apply` v1. Explicit settingsMode opts into safe whole-package
/// undo and preservation (ADR 0659); absent retains legacy automation behaviour (ADR 0346).
/// Name, policy/unit conflict and expected revision are checked before package preflight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplatesApply {
    /// The template's name: `mimari`.
    pub name: String,
    /// Legacy absent settingsMode: mm/cm, only empty drawings. Explicit template mode:
    /// any supported local display unit; a coordinate system requires metres.
    /// Absent uses the package's unit (cm), rejected on a projected drawing.
    /// Rejected with keep. Geometry coordinates never rescale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<DrawingUnit>,
    /// Explicit safe application policy (ADR 0659). `keep` preserves unit/plot scale;
    /// `template` applies the shown package settings without rescaling geometry.
    /// Both preserve existing definitions and make the whole package one undo step.
    /// Absent: the original ADR 0346 v1 behaviour, for existing automation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub settings_mode: Option<TemplateSettingsMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.templates.apply` v1 (its plan's too: what it would add, nothing written).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplatesApplied {
    /// The drawing's unit afterwards.
    pub unit: DrawingUnit,
    /// Explicit policy's before/after settings and the existing definitions kept.
    /// Absent for legacy calls, preserving their output contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub review: Option<TemplateReview>,
    /// The layers added, by name.
    pub layers: Vec<String>,
    /// The text and dimension styles added, by name.
    pub styles: Vec<String>,
    /// The blocks added, by name.
    pub blocks: Vec<String>,
    /// The wall styles added, by name.
    pub wall_styles: Vec<String>,
    /// The levels added, by name.
    pub levels: Vec<String>,
    /// The sheets added, by name.
    pub sheets: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// A deliberate template settings policy, separate from the legacy absent field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TemplateSettingsMode {
    Keep,
    Template,
}

/// What the explicit safe template plan will preserve and change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplateReview {
    pub mode: TemplateSettingsMode,
    pub previous_unit: DrawingUnit,
    /// Previous 1:N denominator.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub previous_plot_scale: f64,
    /// Resulting 1:N denominator.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub plot_scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub previous_space_method: Option<SpaceMethod>,
    pub space_method: SpaceMethod,
    pub preserved: Vec<TemplatePreserved>,
}

/// A package part kept because the drawing already defines its identity or name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplatePreserved {
    /// layer, textStyle, dimensionStyle, block, wallStyle, level or sheet.
    pub kind: String,
    /// The existing definition's identifier.
    pub id: String,
    pub name: String,
    /// The matching key: id, name, or idAndName. Its existing value wins.
    pub matched_by: String,
}

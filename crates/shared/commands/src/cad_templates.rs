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

/// Input of `cad.templates.apply` v1 (“Mimari şablonu”): the template's parts the drawing lacks, as one undo step for its
/// layers, blocks and styles; the settings and the sheet book are no undo step (docs/adr/0346 §1).
///
/// Refusals, in this order: `unknown_template`, `invalid_template_unit`, `invalid_revision`, `revision_conflict`. Warning:
/// `template_unit_kept`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TemplatesApply {
    /// The template's name: `mimari`.
    pub name: String,
    /// The drawing's unit, `mm` or `cm`; absent: the template's. Written only to a drawing without objects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<DrawingUnit>,
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

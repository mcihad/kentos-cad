//! `cad.standards.check` and `cad.standards.apply` (docs/adr/0312 §1–§3): a
//! drawing standard (AutoCAD's CAD Standards, its `.dws`) as data: the
//! layers an office's drawings have, each with its colour, line type, line
//! weight, Yazdır and the names of the layers that belong to it; the names
//! other layers may take; the fonts, text and dimension styles; the line
//! types; whether objects take their look from their layers only. The check
//! reports what a drawing breaks, writing nothing; the apply is the separate,
//! planned fix. The built-in standards are data under
//! `catalog/v1/standards/` (the architectural office's `mimari-ofis.json`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::{DrawingFont, LayerTranslation, LineType, fold_name, layer_color_ok, pattern_problem};

pub const CAD_STANDARDS_CHECK: &str = "cad.standards.check";
pub const CAD_STANDARDS_CHECK_VERSION: u32 = 1;
pub const CAD_STANDARDS_APPLY: &str = "cad.standards.apply";
pub const CAD_STANDARDS_APPLY_VERSION: u32 = 1;
pub const CAD_STANDARDS_CONFLICTS: &str = "cad.standards.conflicts";
pub const CAD_STANDARDS_CONFLICTS_VERSION: u32 = 1;
pub const CAD_STANDARDS_RESOLVE: &str = "cad.standards.resolve";
pub const CAD_STANDARDS_RESOLVE_VERSION: u32 = 1;

/// Builds a prospective standard from the project's ordered bindings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsResolve {}

/// Read-only effective standard. Global rules and duplicate named definitions
/// come from the first document; other named definitions are added in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsResolved {
    pub standard: DrawingStandard,
    /// First definition of each named resource, in binding order. Read-only;
    /// `cad.standards.apply` does not install or replace these resources.
    pub resources: StandardResources,
    pub sources: Vec<String>,
    pub provenance: StandardProvenance,
    pub revision: String,
}

/// Source identity and office revision of an effective definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardOrigin {
    pub id: String,
    pub revision: String,
}

/// Provenance of the global rules and each effective named definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardProvenance {
    pub rules: StandardOrigin,
    pub layers: std::collections::BTreeMap<String, StandardOrigin>,
    pub text_styles: std::collections::BTreeMap<String, StandardOrigin>,
    pub dimension_styles: std::collections::BTreeMap<String, StandardOrigin>,
    pub line_types: std::collections::BTreeMap<String, StandardOrigin>,
    pub hatch_patterns: std::collections::BTreeMap<String, StandardOrigin>,
    pub blocks: std::collections::BTreeMap<String, StandardOrigin>,
    pub sheet_templates: std::collections::BTreeMap<String, StandardOrigin>,
    pub leader_styles: std::collections::BTreeMap<String, StandardOrigin>,
    pub text_style_definitions: std::collections::BTreeMap<String, StandardOrigin>,
    pub dimension_style_definitions: std::collections::BTreeMap<String, StandardOrigin>,
    /// Office tool palettes by id (docs/adr/1014).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<std::collections::BTreeMap<String, StandardOrigin>>"))]
    pub tool_palettes: std::collections::BTreeMap<String, StandardOrigin>,
    /// Table styles by name (docs/adr/1126 §6).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<std::collections::BTreeMap<String, StandardOrigin>>"))]
    pub table_style_definitions: std::collections::BTreeMap<String, StandardOrigin>,
}

/// Inspect global rules and named definitions in ordered standard documents.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsConflicts {}

/// A later document differs from the first on named definitions or global rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardConflict {
    pub kind: String,
    pub name: String,
    pub first_id: String,
    pub later_id: String,
    pub fields: Vec<String>,
    // Values for exactly the fields that differ; missing optional values are null.
    pub first_values: std::collections::BTreeMap<String, serde_json::Value>,
    pub later_values: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Compares global rules and named layer, text and dimension style definitions;
/// it does not combine standards or decide which rules should be applied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsConflictsReport {
    pub conflicts: Vec<StandardConflict>,
    pub revision: String,
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A layer the standard defines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardLayer {
    /// Its name; the drawing's layer of this name (letters folded as Turkish writes them) is it.
    pub name: String,
    /// What it holds, for people.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub description: String,
    /// `#rrggbb`, or a theme colour (`fg`, `fg-dim`, `ink`, `paper`).
    pub color: String,
    pub line_type: LineType,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: f64,
    /// Yazdır: whether it is printed.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub plot: bool,
    /// The names of the layers that belong to it (`cad.layers.find`'s
    /// pattern, comma separated alternatives): Standardı uygula merges them
    /// into it as Katmanları çevir does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
}

/// A text style the standard asks for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardTextStyle {
    pub name: String,
    pub font: DrawingFont,
    /// The texts' height on paper; absent: the tool's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
}

/// A dimension style the standard asks for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardDimensionStyle {
    pub name: String,
    /// The value's height on paper.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: f64,
}

/// A drawing standard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingStandard {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub description: String,
    /// Its layers, in the order they are added.
    #[serde(default)]
    pub layers: Vec<StandardLayer>,
    /// The names other layers may take (patterns, `A-*`); none: only its layers'.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub naming: Vec<String>,
    /// The fonts texts may be written in; none: any. The first is the one a fix gives.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<DrawingFont>>"))]
    pub fonts: Vec<DrawingFont>,
    /// The text styles the project has; when there are some, it has no others.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<StandardTextStyle>>"))]
    pub text_styles: Vec<StandardTextStyle>,
    /// As the text styles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(optional, as = "Option<Vec<StandardDimensionStyle>>")
    )]
    pub dimension_styles: Vec<StandardDimensionStyle>,
    /// The line types layers and objects may take; none: any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<LineType>>"))]
    pub line_types: Vec<LineType>,
    /// Whether objects take colour, line type and line weight from their layers only.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub by_layer: bool,
}

/// Portable resource definitions carried by a version 2 office standard.
/// Association alone never overwrites the project's definitions.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardResources {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<crate::LineTypeDef>>"))]
    pub line_types: Vec<crate::LineTypeDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(optional, as = "Option<Vec<crate::HatchPatternDef>>")
    )]
    pub hatch_patterns: Vec<crate::HatchPatternDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(optional, as = "Option<Vec<StandardBlockReference>>")
    )]
    pub blocks: Vec<StandardBlockReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(optional, as = "Option<Vec<StandardSheetTemplateReference>>")
    )]
    pub sheet_templates: Vec<StandardSheetTemplateReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(optional, as = "Option<Vec<crate::LeaderStyleDef>>")
    )]
    pub leader_styles: Vec<crate::LeaderStyleDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<crate::TextStyleDef>>"))]
    pub text_style_definitions: Vec<crate::TextStyleDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(optional, as = "Option<Vec<crate::DimensionStyleDef>>")
    )]
    pub dimension_style_definitions: Vec<crate::DimensionStyleDef>,
    /// Published office tool palettes (docs/adr/1014): each tool a catalog command with its input. Read from the
    /// package where it is bound; never installed in the drawing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<ToolPalette>>"))]
    pub tool_palettes: Vec<ToolPalette>,
    /// Named table styles (docs/adr/1126 §6): installed by name as the other styles are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<crate::TableStyleDef>>"))]
    pub table_style_definitions: Vec<crate::TableStyleDef>,
}

/// An office tool palette (docs/adr/1014).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ToolPalette {
    /// 1–128 bytes, no surrounding spaces, unique in the package regardless of case.
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// 1 to 200 tools.
    pub tools: Vec<PaletteTool>,
}

/// One tool of a palette: a catalog command run with this input (the tool recipe, docs/adr/1014 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PaletteTool {
    /// 1–128 bytes, unique in its palette regardless of case.
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub category: Option<String>,
    /// The catalog command and its version.
    pub command: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub command_version: u32,
    /// The command's whole input: an object, at most 64 KiB as JSON.
    pub input: serde_json::Value,
}

/// What is wrong with a package's tool palettes, if anything (docs/adr/1014 §2): the shape only; whether each tool's
/// command runs here is `cad.standards.palettes`'s answer.
pub fn tool_palettes_problem(palettes: &[ToolPalette]) -> Option<String> {
    let good = |s: &str| !s.trim().is_empty() && s.trim() == s && s.len() <= 128;
    if palettes.len() > 50 {
        return Some("bir pakette en çok 50 araç paleti olur".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for p in palettes {
        if !good(&p.id) || !good(&p.name) {
            return Some("paletin kimliği ve adı 1–128 bayt olmalı ve kenar boşluğu içermemeli".into());
        }
        if !ids.insert(p.id.to_lowercase()) {
            return Some(format!("“{}” palet kimliği yineleniyor", p.id));
        }
        if p.tools.is_empty() || p.tools.len() > 200 {
            return Some(format!("“{}” paletinde 1 ile 200 araç olmalı", p.id));
        }
        let mut tools = std::collections::BTreeSet::new();
        for t in &p.tools {
            if !good(&t.id) || !good(&t.title) || !good(&t.command) {
                return Some(format!("“{}” paletinin aracının kimliği, başlığı ve komutu 1–128 bayt olmalı", p.id));
            }
            if !tools.insert(t.id.to_lowercase()) {
                return Some(format!("“{}” paletinde “{}” araç kimliği yineleniyor", p.id, t.id));
            }
            if t.command_version == 0 {
                return Some(format!("“{}” aracının komut sürümü 1 ya da büyük olmalı", t.id));
            }
            if !t.input.is_object() || serde_json::to_string(&t.input).map_or(true, |s| s.len() > 65_536) {
                return Some(format!("“{}” aracının girdisi en çok 64 KiB'lık bir nesne olmalı", t.id));
            }
        }
    }
    None
}

/// Input of `cad.standards.palettes` v1 (docs/adr/1014): the bound standards' office tool palettes, read only.
/// Refusal: `invalid_standard` (a bound package does not read).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsPalettes {}

/// Why a palette tool does not run here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PaletteToolProblem {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
}

/// A palette tool as this drawing sees it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PaletteToolState {
    /// `standart:<package id>:<palette id>:<tool id>`: what a personal favourite keeps.
    pub key: String,
    /// SHA-256 of the command, its version and its input as canonical JSON: a favourite whose digest changed points to
    /// a changed tool and is not run in its place.
    pub digest: String,
    #[serde(flatten)]
    pub tool: PaletteTool,
    /// The command checked its input against this drawing (`validate`) and would run.
    pub ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<PaletteToolProblem>,
}

/// One palette, from the package that gives it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PaletteState {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// The package's id and revision.
    pub standard: String,
    pub revision: String,
    pub tools: Vec<PaletteToolState>,
}

/// Output of `cad.standards.palettes` v1: the palettes in the standards' order, a palette id once (the first
/// standard's).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsPalettesRead {
    pub palettes: Vec<PaletteState>,
}

pub const CAD_STANDARDS_PALETTES: &str = "cad.standards.palettes";
pub const CAD_STANDARDS_PALETTES_VERSION: u32 = 1;

/// Approved sheet-template origin. It identifies a template revision, not a
/// copy of its contents or permission to modify an existing sheet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardSheetTemplateReference {
    pub template_id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
}

pub fn standard_sheet_templates_problem(
    items: &[StandardSheetTemplateReference],
) -> Option<String> {
    let mut ids = std::collections::BTreeSet::new();
    for item in items {
        if item.template_id.trim().is_empty()
            || item.template_id.trim() != item.template_id
            || item.template_id.len() > 128
            || item.template_id.chars().any(char::is_control)
            || item.revision == 0
        {
            return Some("Pafta şablonu kimliği 1–128 bayt ve revizyonu en az 1 olmalı".into());
        }
        if !ids.insert(item.template_id.to_lowercase()) {
            return Some(format!("“{}” pafta şablonu yineleniyor", item.template_id));
        }
    }
    None
}

/// A named block library release required by an office standard. The
/// publication identifies a reusable type; geometry stays in its library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardBlockReference {
    pub name: String,
    pub publication: crate::BlockPublication,
    /// Optional approved authored unit. Checking it never rescales a local block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<crate::BlockUnits>,
}

pub fn standard_blocks_problem(blocks: &[StandardBlockReference]) -> Option<String> {
    let mut names = std::collections::BTreeSet::new();
    for block in blocks {
        if block.name.trim().is_empty()
            || block.name.trim() != block.name
            || block.name.chars().count() > 128
            || block.name.chars().any(char::is_control)
        {
            return Some("Blok adı 1–128 karakter olmalı ve kenar boşluğu içermemeli".into());
        }
        if !names.insert(crate::blocks::name_key(&block.name)) {
            return Some(format!("“{}” blok adı yineleniyor", block.name));
        }
        let card = crate::BlockCard {
            publication: Some(block.publication.clone()),
            version: 1,
            ..Default::default()
        };
        if let Some(problem) = crate::blocks::card_problem(&card) {
            return Some(format!("“{}” blok yayını geçersiz: {problem}", block.name));
        }
    }
    None
}

/// A portable, versioned envelope for a drawing standard. Version 1 carries
/// rules only; version 2 additionally carries named line and hatch definitions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StandardPackage {
    pub format: String,
    pub version: u32,
    /// Stable office-assigned identity and revision, independent of a filename.
    pub id: String,
    pub revision: String,
    pub standard: DrawingStandard,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<StandardResources>,
}

pub const STANDARD_PACKAGE_FORMAT: &str = "kentos.drawing-standard";
pub const STANDARD_PACKAGE_VERSION: u32 = 1;
pub const STANDARD_PACKAGE_RESOURCES_VERSION: u32 = 2;

/// Parse either the original bare standard or the versioned envelope. Unknown
/// fields are refused at every supported level: silently discarding a rule
/// would make an incomplete standard appear to pass inspection.
pub fn parse_standard_document(
    bytes: &[u8],
) -> Result<(DrawingStandard, Option<StandardPackage>), String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Standart okunamadı: {e}"))?;
    let object = value
        .as_object()
        .ok_or("Standart bir JSON nesnesi olmalı.")?;
    let (standard_value, package) = if object.contains_key("format")
        || object.contains_key("version")
    {
        let package: StandardPackage = serde_json::from_slice(bytes).map_err(|e| {
            format!(
                "Standart paketi okunamadı: {}",
                e.to_string().replace("duplicate field", "yinelenen alan")
            )
        })?;
        if package.format != STANDARD_PACKAGE_FORMAT
            || ![STANDARD_PACKAGE_VERSION, STANDARD_PACKAGE_RESOURCES_VERSION]
                .contains(&package.version)
        {
            return Err("Standart paketinin biçimi veya sürümü desteklenmiyor.".into());
        }
        match package.version {
            STANDARD_PACKAGE_VERSION if object.contains_key("resources") => {
                return Err("Sürüm 1 standart paketi kaynak tanımı içeremez.".into());
            }
            STANDARD_PACKAGE_RESOURCES_VERSION => {
                let Some(resources) = object.get("resources") else {
                    return Err("Sürüm 2 standart paketinin kaynaklar bölümü eksik.".into());
                };
                check_resource_fields(resources)?;
                let resources = package
                    .resources
                    .as_ref()
                    .ok_or("Kaynaklar bölümü geçersiz.")?;
                if let Some(problem) = crate::line_types_problem(&resources.line_types) {
                    return Err(format!("Çizgi tipi kaynağı geçersiz: {problem}."));
                }
                if let Some(problem) = crate::hatch_patterns_problem(&resources.hatch_patterns) {
                    return Err(format!("Tarama kaynağı geçersiz: {problem}."));
                }
                if let Some(problem) = standard_blocks_problem(&resources.blocks) {
                    return Err(format!("Blok kaynağı geçersiz: {problem}."));
                }
                if let Some(problem) = standard_sheet_templates_problem(&resources.sheet_templates)
                {
                    return Err(format!("Pafta şablonu kaynağı geçersiz: {problem}."));
                }
                if let Some(problem) = crate::leader_styles_problem(&resources.leader_styles) {
                    return Err(format!("Kılavuz stili kaynağı geçersiz: {problem}."));
                }
                if let Some(problem) = crate::text_styles_problem(&resources.text_style_definitions)
                {
                    return Err(format!("Yazı stili kaynağı geçersiz: {problem}."));
                }
                if let Some(problem) =
                    crate::dimension_styles_problem(&resources.dimension_style_definitions)
                {
                    return Err(format!("Ölçü stili kaynağı geçersiz: {problem}."));
                }
                if let Some(problem) = tool_palettes_problem(&resources.tool_palettes) {
                    return Err(format!("Araç paleti geçersiz: {problem}."));
                }
                if let Some(problem) = crate::table_style_defs_problem(&resources.table_style_definitions) {
                    return Err(format!("Tablo stili kaynağı geçersiz: {problem}."));
                }
            }
            _ => {}
        }
        if package.id.trim().is_empty()
            || package.revision.trim().is_empty()
            || package.id.len() > 128
            || package.revision.len() > 128
        {
            return Err("Paket kimliği ve revizyonu boş olmamalı ve 128 baytı aşmamalı.".into());
        }
        (
            object
                .get("standard")
                .ok_or("Paketin standart tanımı yok.")?,
            Some(package),
        )
    } else {
        (&value, None)
    };
    let standard_object = standard_value
        .as_object()
        .ok_or("Standart bir JSON nesnesi olmalı.")?;
    const FIELDS: &[&str] = &[
        "name",
        "description",
        "layers",
        "naming",
        "fonts",
        "textStyles",
        "dimensionStyles",
        "lineTypes",
        "byLayer",
    ];
    if let Some(key) = standard_object
        .keys()
        .find(|key| !FIELDS.contains(&key.as_str()))
    {
        return Err(format!("Standartta bilinmeyen alan: {key}."));
    }
    for (section, allowed) in [
        (
            "layers",
            &[
                "name",
                "description",
                "color",
                "lineType",
                "lineWeight",
                "plot",
                "from",
            ][..],
        ),
        ("textStyles", &["name", "font", "height"][..]),
        ("dimensionStyles", &["name", "height"][..]),
    ] {
        if let Some(items) = standard_object.get(section).and_then(|v| v.as_array()) {
            for item in items {
                if let Some(key) = item
                    .as_object()
                    .and_then(|o| o.keys().find(|k| !allowed.contains(&k.as_str())))
                {
                    return Err(format!("{section} içinde bilinmeyen alan: {key}."));
                }
            }
        }
    }
    // Deserialize the original bytes, not the intermediate JSON value: a
    // Value silently keeps only the last occurrence of a duplicate key.
    let standard: DrawingStandard = match &package {
        Some(package) => package.standard.clone(),
        None => serde_json::from_slice(bytes).map_err(|e| {
            format!(
                "Standart okunamadı: {}",
                e.to_string().replace("duplicate field", "yinelenen alan")
            )
        })?,
    };
    if let Some(problem) = standard_problem(&standard) {
        return Err(format!("Standart geçersiz: {problem}."));
    }
    Ok((standard, package))
}

/// Resource types in the drawing model are lenient when reading old drawing
/// files. Office packages must reject unknown rules even inside a definition.
pub fn check_resource_fields(resources: &serde_json::Value) -> Result<(), String> {
    fn object<'a>(
        value: &'a serde_json::Value,
        path: &str,
    ) -> Result<&'a serde_json::Map<String, serde_json::Value>, String> {
        value
            .as_object()
            .ok_or_else(|| format!("{path} bir nesne olmalı."))
    }
    fn keys(value: &serde_json::Value, path: &str, allowed: &[&str]) -> Result<(), String> {
        if let Some(key) = object(value, path)?
            .keys()
            .find(|key| !allowed.contains(&key.as_str()))
        {
            return Err(format!("{path} içinde bilinmeyen alan: {key}."));
        }
        Ok(())
    }
    keys(
        resources,
        "resources",
        &[
            "lineTypes",
            "hatchPatterns",
            "blocks",
            "sheetTemplates",
            "leaderStyles",
            "textStyleDefinitions",
            "dimensionStyleDefinitions",
            "toolPalettes",
            "tableStyleDefinitions",
        ],
    )?;
    // Table styles (docs/adr/1126 §6): a style's and each cell style's fields, nothing dropped unseen.
    if let Some(value) = object(resources, "resources")?.get("tableStyleDefinitions") {
        let styles = value
            .as_array()
            .ok_or("resources.tableStyleDefinitions bir dizi olmalı.")?;
        for (i, item) in styles.iter().enumerate() {
            let path = format!("resources.tableStyleDefinitions[{i}]");
            keys(item, &path, &["id", "name", "title", "header", "data"])?;
            for cell in ["title", "header", "data"] {
                if let Some(c) = object(item, &path)?.get(cell) {
                    keys(
                        c,
                        &format!("{path}.{cell}"),
                        &[
                            "textStyle",
                            "height",
                            "align",
                            "color",
                            "fill",
                            "horizontalMargin",
                            "verticalMargin",
                            "borderWeight",
                            "borderColor",
                        ],
                    )?;
                }
            }
        }
    }
    let root = object(resources, "resources")?;
    if let Some(value) = root.get("blocks") {
        let blocks = value
            .as_array()
            .ok_or("resources.blocks bir dizi olmalı.")?;
        for (i, block) in blocks.iter().enumerate() {
            let path = format!("resources.blocks[{i}]");
            keys(block, &path, &["name", "publication", "units"])?;
            if let Some(publication) = object(block, &path)?.get("publication") {
                keys(
                    publication,
                    &format!("{path}.publication"),
                    &["familyId", "typeId", "typeName", "release"],
                )?;
            }
        }
    }
    if let Some(value) = root.get("sheetTemplates") {
        let templates = value
            .as_array()
            .ok_or("resources.sheetTemplates bir dizi olmalı.")?;
        for (i, item) in templates.iter().enumerate() {
            keys(
                item,
                &format!("resources.sheetTemplates[{i}]"),
                &["templateId", "revision"],
            )?;
        }
    }
    if let Some(value) = root.get("leaderStyles") {
        let styles = value
            .as_array()
            .ok_or("resources.leaderStyles bir dizi olmalı.")?;
        for (i, item) in styles.iter().enumerate() {
            keys(
                item,
                &format!("resources.leaderStyles[{i}]"),
                // docs/adr/1025: the note's text style, by name.
                &["id", "name", "height", "arrow", "arrowSize", "mask", "textStyle"],
            )?;
        }
    }
    if let Some(value) = root.get("textStyleDefinitions") {
        let styles = value
            .as_array()
            .ok_or("resources.textStyleDefinitions bir dizi olmalı.")?;
        for (i, item) in styles.iter().enumerate() {
            keys(
                item,
                &format!("resources.textStyleDefinitions[{i}]"),
                &[
                    "id",
                    "name",
                    "font",
                    "bold",
                    "italic",
                    "oblique",
                    "height",
                    "widthFactor",
                    "fontFile",
                    // ADR 1120 §4: the Big Font beside an SHX font.
                    "bigFontFile",
                ],
            )?;
        }
    }
    if let Some(value) = root.get("dimensionStyleDefinitions") {
        let styles = value
            .as_array()
            .ok_or("resources.dimensionStyleDefinitions bir dizi olmalı.")?;
        for (i, item) in styles.iter().enumerate() {
            keys(
                item,
                &format!("resources.dimensionStyleDefinitions[{i}]"),
                &[
                    "id",
                    "name",
                    "height",
                    "arrow",
                    "arrowSize",
                    "extOffset",
                    "extBeyond",
                    "textGap",
                    "textPlace",
                    "decimals",
                    "unit",
                    "prefix",
                    "suffix",
                    "font",
                    // docs/adr/1023: the text style its value is written in, by name.
                    "textStyle",
                    "dimLineColor",
                    "dimLineWeight",
                    "dimLineType",
                    "extColor",
                    "extWeight",
                    "extLineType",
                    "textColor",
                    "options",
                ],
            )?;
        }
    }
    for (section, fields) in [
        (
            "lineTypes",
            &["name", "description", "elements", "unit", "source"][..],
        ),
        (
            "hatchPatterns",
            &["name", "description", "unit", "lines", "source"][..],
        ),
    ] {
        let Some(value) = root.get(section) else {
            continue;
        };
        let items = value
            .as_array()
            .ok_or_else(|| format!("resources.{section} bir dizi olmalı."))?;
        for (i, item) in items.iter().enumerate() {
            let path = format!("resources.{section}[{i}]");
            keys(item, &path, fields)?;
            let nested = if section == "lineTypes" {
                "elements"
            } else {
                "lines"
            };
            if let Some(value) = object(item, &path)?.get(nested) {
                let elements = value
                    .as_array()
                    .ok_or_else(|| format!("{path}.{nested} bir dizi olmalı."))?;
                for (j, element) in elements.iter().enumerate() {
                    let child = format!("{path}.{nested}[{j}]");
                    if section == "lineTypes" {
                        keys(element, &child, &["length", "text", "shape"])?;
                        for (kind, fields) in [
                            (
                                "text",
                                &[
                                    "text", "style", "scale", "rotation", "absolute", "upright",
                                    "x", "y",
                                ][..],
                            ),
                            (
                                "shape",
                                &[
                                    "name", "file", "scale", "rotation", "absolute", "upright",
                                    "x", "y",
                                ][..],
                            ),
                        ] {
                            if let Some(value) = object(element, &child)?.get(kind) {
                                keys(value, &format!("{child}.{kind}"), fields)?;
                            }
                        }
                    } else {
                        keys(element, &child, &["angle", "origin", "offset", "dashes"])?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// A font's name in the contract and the file (`architects-daughter`).
pub fn font_id(f: DrawingFont) -> &'static str {
    match f {
        DrawingFont::Barlow => "barlow",
        DrawingFont::Arimo => "arimo",
        DrawingFont::Overpass => "overpass",
        DrawingFont::Quicksand => "quicksand",
        DrawingFont::ArchitectsDaughter => "architects-daughter",
        DrawingFont::CourierPrime => "courier-prime",
        DrawingFont::PlexMono => "plex-mono",
    }
}

/// A font's name as people read it.
pub fn font_label(f: DrawingFont) -> &'static str {
    match f {
        DrawingFont::Barlow => "Barlow",
        DrawingFont::Arimo => "Arimo",
        DrawingFont::Overpass => "Overpass",
        DrawingFont::Quicksand => "Quicksand",
        DrawingFont::ArchitectsDaughter => "Architects Daughter",
        DrawingFont::CourierPrime => "Courier Prime",
        DrawingFont::PlexMono => "IBM Plex Mono",
    }
}

/// The built-in standards: their ids and their data.
const BUILT_IN: &[(&str, &str)] = &[(
    "mimari-ofis",
    include_str!("../../../../catalog/v1/standards/mimari-ofis.json"),
)];

/// The ids of the built-in standards, in their order.
pub fn built_in_standards() -> Vec<&'static str> {
    BUILT_IN.iter().map(|(id, _)| *id).collect()
}

/// A built-in standard by its id (`mimari-ofis`).
pub fn built_in_standard(id: &str) -> Option<DrawingStandard> {
    let (_, text) = BUILT_IN.iter().find(|(i, _)| *i == id)?;
    serde_json::from_str(text).ok()
}

/// What is wrong with a standard, said so its writer can fix it; none: it holds.
pub fn standard_problem(s: &DrawingStandard) -> Option<String> {
    if s.name.trim().is_empty() {
        return Some("Standardın adı boş".into());
    }
    let mut seen: Vec<String> = Vec::new();
    for l in &s.layers {
        let name = l.name.trim();
        if name.is_empty() {
            return Some("Standardın bir katmanının adı boş".into());
        }
        let folded = fold_name(name);
        if seen.contains(&folded) {
            return Some(format!("“{name}” katmanı standartta iki kez var"));
        }
        seen.push(folded);
        if !layer_color_ok(&l.color) {
            return Some(format!(
                "“{name}” katmanının rengi (“{}”) bir katman rengi değil; #rrggbb ya da fg, fg-dim, ink, paper yazın",
                l.color
            ));
        }
        if !(l.line_weight.is_finite() && (0.0..=crate::MAX_LINE_WEIGHT).contains(&l.line_weight)) {
            return Some(format!(
                "“{name}” katmanının kalınlığı 0 ile 100 mm arasında olmalı"
            ));
        }
        if let Some(from) = &l.from
            && let Some(p) = pattern_problem(from)
        {
            return Some(format!(
                "“{name}” katmanının topladığı adlar (“{from}”) bir örüntü değil: {p}"
            ));
        }
    }
    for n in &s.naming {
        if let Some(p) = pattern_problem(n) {
            return Some(format!("Katman adı örüntüsü “{n}” geçersiz: {p}"));
        }
    }
    let mut names: Vec<String> = Vec::new();
    for t in &s.text_styles {
        let name = t.name.trim();
        if name.is_empty() || names.contains(&fold_name(name)) {
            return Some(format!("Yazı stili “{name}” boş ya da iki kez var"));
        }
        names.push(fold_name(name));
        if t.height.is_some_and(|h| !(h.is_finite() && h > 0.0)) {
            return Some(format!(
                "“{name}” yazı stilinin yüksekliği 0'dan büyük olmalı"
            ));
        }
    }
    names.clear();
    for d in &s.dimension_styles {
        let name = d.name.trim();
        if name.is_empty() || names.contains(&fold_name(name)) {
            return Some(format!("Ölçü stili “{name}” boş ya da iki kez var"));
        }
        names.push(fold_name(name));
        if !(d.height.is_finite() && d.height > 0.0) {
            return Some(format!(
                "“{name}” ölçü stilinin yüksekliği 0'dan büyük olmalı"
            ));
        }
    }
    None
}

/// `cad.standards.check` v1: what the drawing breaks of a standard. It writes nothing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsCheck {
    /// A built-in standard by its id (`mimari-ofis`); or
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// the standard itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub standard: Option<DrawingStandard>,
    /// Optional named definitions from a version 2 standard package. Checking
    /// them never imports or replaces a drawing resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub resources: Option<StandardResources>,
    /// The standard's source, which accepted findings are bound to (docs/adr/1129): a bound package's id and revision,
    /// or `cad.standards.resolve`'s `provenance.rules`. Absent: the standard's name, no revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub origin: Option<StandardOrigin>,
}

/// Whether a finding is the project's to look at (docs/adr/1129 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StandardIssueState {
    /// Not accepted (the field is then absent).
    #[default]
    Open,
    /// Accepted with a reason, against this standard's revision and with what the drawing has now.
    Accepted,
    /// Accepted once, but the standard's revision or what the drawing has changed since: to be looked at again.
    Lapsed,
}

impl StandardIssueState {
    pub fn is_open(&self) -> bool {
        *self == StandardIssueState::Open
    }
}

/// What a finding is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StandardIssueKind {
    /// A standard layer the drawing lacks.
    MissingLayer,
    /// A layer whose name the standard does not have nor allow.
    LayerName,
    LayerColor,
    LayerLineType,
    LayerWeight,
    LayerPlot,
    /// The project's drawing font is not one of the standard's.
    ProjectFont,
    /// A text style written in a font the standard does not have.
    TextFont,
    MissingTextStyle,
    /// A text style the standard does not have.
    TextStyleName,
    /// A text style's paper height differs from the standard.
    TextStyleHeight,
    MissingDimensionStyle,
    DimensionStyleName,
    /// A dimension style's paper text height differs from the standard.
    DimensionStyleHeight,
    MissingLineTypeDefinition,
    LineTypeDefinition,
    MissingHatchPatternDefinition,
    HatchPatternDefinition,
    MissingBlockReference,
    BlockPublication,
    BlockUnit,
    SheetTemplateOrigin,
    MissingLeaderStyleDefinition,
    LeaderStyleDefinition,
    MissingTextStyleDefinition,
    TextStyleDefinition,
    MissingDimensionStyleDefinition,
    DimensionStyleDefinition,
    /// A table style the standard has and the drawing does not (docs/adr/1126 §6).
    MissingTableStyleDefinition,
    /// A table style whose cell styles differ from the standard's.
    TableStyleDefinition,
    /// Objects with a line type of their own the standard does not have.
    ObjectLineType,
    /// Objects with a look of their own when the standard has them take their layers'.
    OwnLook,
}

/// A finding.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardIssue {
    pub kind: StandardIssueKind,
    /// What it is about: a layer's, a style's or a line type's name.
    pub subject: String,
    /// The layer's id, when it is about one of the drawing's layers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// What the standard asks for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected: Option<String>,
    /// What the drawing has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub found: Option<String>,
    /// Direct objects this finding is about (ObjectLineType, OwnLook, named
    /// line type or hatch definition), or direct placements of a mismatched
    /// published block (BlockPublication or BlockUnit). Layer and nested-block
    /// uses are described separately in the finding's message.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// Whether `cad.standards.apply` fixes it.
    pub fixable: bool,
    /// For people: what is wrong and what fixes it.
    pub message: String,
    /// Accepted or lapsed (docs/adr/1129 §2); absent: open.
    #[serde(default, skip_serializing_if = "StandardIssueState::is_open")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<StandardIssueState>"))]
    pub state: StandardIssueState,
    /// The accepted finding's reason, when it is accepted or lapsed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub reason: Option<String>,
}

/// `cad.standards.check`'s answer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsReport {
    /// The standard's name.
    pub standard: String,
    /// In the order: the standard's layers, the drawing's other layers, the
    /// fonts and styles, the objects.
    pub issues: Vec<StandardIssue>,
    /// How many of them `cad.standards.apply` fixes.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub fixable: u32,
    /// How many are accepted, and how many lapsed (docs/adr/1129 §2): still listed, counted apart.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub accepted: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub lapsed: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// `cad.standards.apply` v1: the drawing brought to a standard. The layers
/// in one undo step “Standardı uygula”; the styles and the project's font
/// are settings (docs/adr/0183), written with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsApply {
    /// As the check's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub standard: Option<DrawingStandard>,
    /// Add the standard layers the drawing lacks.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub layers: bool,
    /// Give the standard layers the drawing has their colour, line type, weight and Yazdır.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub restyle: bool,
    /// Merge the layers that belong to a standard layer into it.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub rename: bool,
    /// The project's font, the text styles' fonts, the missing text and dimension styles.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub styles: bool,
    /// The group new layers go into; absent: the top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

impl Default for StandardsApply {
    fn default() -> Self {
        Self {
            name: None,
            standard: None,
            layers: true,
            restyle: true,
            rename: true,
            styles: true,
            parent: None,
            expected_revision: None,
        }
    }
}

/// What `cad.standards.apply` did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsApplied {
    pub changed: bool,
    /// The layers added, by id, in the standard's order.
    pub created: Vec<String>,
    /// The standard layers whose look or Yazdır changed, by id.
    pub restyled: Vec<String>,
    /// The layers merged into a standard layer.
    pub translations: Vec<LayerTranslation>,
    /// The objects moved.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The text styles added or given the standard's font, by name.
    pub text_styles: Vec<String>,
    /// The dimension styles added, by name.
    pub dimension_styles: Vec<String>,
    /// Whether the project's drawing font changed.
    pub font: bool,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_standards_hold() {
        for id in built_in_standards() {
            let s = built_in_standard(id).expect(id);
            assert_eq!(standard_problem(&s), None, "{id}");
        }
        let s = built_in_standard("mimari-ofis").unwrap();
        assert_eq!(s.layers.len(), 10);
        assert!(s.by_layer);
        assert!(!s.layers[9].plot);
        assert_eq!(built_in_standard("yok"), None);
    }

    #[test]
    fn standard_documents_preserve_legacy_and_reject_unsupported_package_rules() {
        let original = built_in_standard("mimari-ofis").unwrap();
        let raw = serde_json::to_vec(&original).unwrap();
        let (standard, package) = parse_standard_document(&raw).unwrap();
        assert_eq!(standard, original);
        assert!(package.is_none());

        let package = StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_VERSION,
            id: "belediye-mimari".into(),
            revision: "2026.1".into(),
            standard: original.clone(),
            resources: None,
        };
        let mut value = serde_json::to_value(&package).unwrap();
        let (standard, parsed) =
            parse_standard_document(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(standard, original);
        assert_eq!(parsed, Some(package));
        value["version"] = 3.into();
        assert!(parse_standard_document(&serde_json::to_vec(&value).unwrap()).is_err());
        value["version"] = 1.into();
        value["standard"]["layers"][0]["unsupportedRule"] = true.into();
        assert!(parse_standard_document(&serde_json::to_vec(&value).unwrap()).is_err());
        value["standard"]["layers"][0]
            .as_object_mut()
            .unwrap()
            .remove("unsupportedRule");
        value["id"] = " ".into();
        assert!(parse_standard_document(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn standard_documents_reject_duplicate_rule_keys() {
        let legacy = br#"{"name":"A","layers":[],"byLayer":true,"byLayer":false}"#;
        let envelope = br#"{"format":"kentos.drawing-standard","version":1,"id":"a","id":"b","revision":"1","standard":{"name":"A","layers":[]}}"#;
        let nested = br#"{"format":"kentos.drawing-standard","version":1,"id":"a","revision":"1","standard":{"name":"A","layers":[],"byLayer":true,"byLayer":false}}"#;
        let layer = br##"{"name":"A","layers":[{"name":"A-WALL","color":"ink","color":"#ff0000","lineType":"continuous","lineWeight":0.2}]}"##;
        for raw in [&legacy[..], &envelope[..], &nested[..], &layer[..]] {
            let error = parse_standard_document(raw).unwrap_err();
            assert!(error.contains("yinelenen alan"), "{error}");
        }
    }

    #[test]
    fn version_two_resources_are_strict_and_version_one_cannot_carry_them() {
        let line: crate::LineTypeDef = serde_json::from_value(serde_json::json!({
            "name":"OFFICE-DASH", "elements":[{"length":2.0},{"length":-1.0}]
        }))
        .unwrap();
        let hatch: crate::HatchPatternDef = serde_json::from_value(serde_json::json!({
            "name":"OFFICE-HATCH", "lines":[{"angle":45.0,"origin":[0.0,0.0],"offset":[0.0,2.0]}]
        }))
        .unwrap();
        let package = StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: "office-a".into(),
            revision: "2".into(),
            standard: built_in_standard("mimari-ofis").unwrap(),
            resources: Some(StandardResources {
                line_types: vec![line],
                hatch_patterns: vec![hatch],
                blocks: vec![],
                sheet_templates: vec![],
                leader_styles: vec![],
                ..Default::default()
            }),
        };
        let mut value = serde_json::to_value(&package).unwrap();
        assert_eq!(
            parse_standard_document(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .1,
            Some(package)
        );
        value["resources"]["lineTypes"][0]["elements"][0]["unknown"] = true.into();
        assert!(
            parse_standard_document(&serde_json::to_vec(&value).unwrap())
                .unwrap_err()
                .contains("bilinmeyen alan")
        );
        value["resources"]["lineTypes"][0]["elements"][0]
            .as_object_mut()
            .unwrap()
            .remove("unknown");
        value["version"] = 1.into();
        assert!(parse_standard_document(&serde_json::to_vec(&value).unwrap()).is_err());
        value["version"] = 2.into();
        value.as_object_mut().unwrap().remove("resources");
        assert!(parse_standard_document(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn version_two_block_references_validate_publication_and_names() {
        let block: StandardBlockReference = serde_json::from_value(serde_json::json!({
            "name":"A-KAPI",
            "publication": {
                "familyId":"22222222-3333-4444-8555-666666666666",
                "typeId":"33333333-4444-4555-8666-777777777777",
                "typeName":"Standart tek kanat", "release":2
            }
        }))
        .unwrap();
        let package = StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: "office-a".into(),
            revision: "2".into(),
            standard: built_in_standard("mimari-ofis").unwrap(),
            resources: Some(StandardResources {
                blocks: vec![block],
                ..Default::default()
            }),
        };
        let mut value = serde_json::to_value(package).unwrap();
        let parse =
            |v: &serde_json::Value| parse_standard_document(&serde_json::to_vec(v).unwrap());
        assert!(parse(&value).is_ok());
        value["resources"]["blocks"][0]["units"] = "mm".into();
        assert_eq!(
            parse(&value).unwrap().1.unwrap().resources.unwrap().blocks[0].units,
            Some(crate::BlockUnits::Mm)
        );
        value["resources"]["blocks"][0]["units"] = "mile".into();
        assert!(parse(&value).is_err());
        value["resources"]["blocks"][0]
            .as_object_mut()
            .unwrap()
            .remove("units");
        value["resources"]["blocks"][0]["publication"]["unsupported"] = true.into();
        assert!(parse(&value).is_err());
        value["resources"]["blocks"][0]["publication"]
            .as_object_mut()
            .unwrap()
            .remove("unsupported");
        value["resources"]["blocks"][0]["publication"]["release"] = 0.into();
        assert!(parse(&value).unwrap_err().contains("yayın"));
        value["resources"]["blocks"][0]["publication"]["release"] = 2.into();
        let duplicate = value["resources"]["blocks"][0].clone();
        value["resources"]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        value["resources"]["blocks"][1]["name"] = "a-kapı".into();
        assert!(parse(&value).unwrap_err().contains("yineleniyor"));
    }

    #[test]
    fn version_two_sheet_template_references_validate_revision_and_duplicates() {
        let package = StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: "office-a".into(),
            revision: "2".into(),
            standard: built_in_standard("mimari-ofis").unwrap(),
            resources: Some(StandardResources {
                sheet_templates: vec![StandardSheetTemplateReference {
                    template_id: "office-a3".into(),
                    revision: 2,
                }],
                ..Default::default()
            }),
        };
        let mut value = serde_json::to_value(package).unwrap();
        let parse =
            |v: &serde_json::Value| parse_standard_document(&serde_json::to_vec(v).unwrap());
        assert!(parse(&value).is_ok());
        value["resources"]["sheetTemplates"][0]["unknown"] = true.into();
        assert!(parse(&value).is_err());
        value["resources"]["sheetTemplates"][0]
            .as_object_mut()
            .unwrap()
            .remove("unknown");
        value["resources"]["sheetTemplates"][0]["revision"] = 0.into();
        assert!(parse(&value).unwrap_err().contains("revizyonu"));
        value["resources"]["sheetTemplates"][0]["revision"] = 2.into();
        value["resources"]["sheetTemplates"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"templateId":"OFFICE-A3","revision":3}));
        assert!(parse(&value).unwrap_err().contains("yineleniyor"));
    }

    #[test]
    fn version_two_leader_styles_validate_content_and_unknown_fields() {
        let package = StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: "office-a".into(),
            revision: "2".into(),
            standard: built_in_standard("mimari-ofis").unwrap(),
            resources: Some(StandardResources {
                leader_styles: vec![
                    serde_json::from_value(serde_json::json!({
                        "id":"office-leader-1", "name":"A-NOT", "height":2.5,
                        "arrow":"open", "arrowSize":1.5
                    }))
                    .unwrap(),
                ],
                ..Default::default()
            }),
        };
        let mut value = serde_json::to_value(package).unwrap();
        let parse =
            |v: &serde_json::Value| parse_standard_document(&serde_json::to_vec(v).unwrap());
        assert!(parse(&value).is_ok());
        value["resources"]["leaderStyles"][0]["unexpected"] = true.into();
        assert!(parse(&value).is_err());
        value["resources"]["leaderStyles"][0]
            .as_object_mut()
            .unwrap()
            .remove("unexpected");
        value["resources"]["leaderStyles"][0]["height"] = 0.into();
        assert!(parse(&value).unwrap_err().contains("Kılavuz stili"));
        value["resources"]["leaderStyles"][0]["height"] = 2.5.into();
        value["resources"]["leaderStyles"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id":"office-leader-2", "name":"a-not", "height":2.5}));
        assert!(parse(&value).unwrap_err().contains("iki kez"));
    }

    #[test]
    fn version_two_complete_annotation_styles_reject_unknown_and_invalid_content() {
        let package = StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: "office-a".into(),
            revision: "2".into(),
            standard: built_in_standard("mimari-ofis").unwrap(),
            resources: Some(StandardResources {
                text_style_definitions: vec![
                    serde_json::from_value(serde_json::json!({
                        "id":"office-text", "name":"A-NOT", "font":"arimo",
                        "height":2.5, "widthFactor":0.9,
                    }))
                    .unwrap(),
                ],
                dimension_style_definitions: vec![
                    serde_json::from_value(serde_json::json!({
                        "id":"office-dim", "name":"A-DIM", "height":2.5,
                        "dimLineColor":"#ff0000", "options":{"round":0.001},
                    }))
                    .unwrap(),
                ],
                ..Default::default()
            }),
        };
        let mut value = serde_json::to_value(package).unwrap();
        let parse =
            |v: &serde_json::Value| parse_standard_document(&serde_json::to_vec(v).unwrap());
        assert!(parse(&value).is_ok());
        value["resources"]["textStyleDefinitions"][0]["unknown"] = true.into();
        assert!(parse(&value).unwrap_err().contains("bilinmeyen alan"));
        value["resources"]["textStyleDefinitions"][0]
            .as_object_mut()
            .unwrap()
            .remove("unknown");
        value["resources"]["dimensionStyleDefinitions"][0]["options"]["unknown"] = true.into();
        assert!(parse(&value).is_err());
        value["resources"]["dimensionStyleDefinitions"][0]["options"]
            .as_object_mut()
            .unwrap()
            .remove("unknown");
        value["resources"]["textStyleDefinitions"][0]["height"] = 0.into();
        assert!(parse(&value).unwrap_err().contains("Yazı stili kaynağı"));
        value["resources"]["textStyleDefinitions"][0]["height"] = 2.5.into();
        value["resources"]["dimensionStyleDefinitions"][0]["height"] = 0.into();
        assert!(parse(&value).unwrap_err().contains("Ölçü stili kaynağı"));
        value["resources"]["dimensionStyleDefinitions"][0]["height"] = 2.5.into();
        value["resources"]["textStyleDefinitions"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "id":"other-text", "name":"a-not", "font":"arimo"
            }));
        assert!(parse(&value).unwrap_err().contains("iki kez"));
    }

    #[test]
    fn direct_command_input_does_not_ignore_unknown_standard_rules() {
        let standard = built_in_standard("mimari-ofis").unwrap();
        let valid = serde_json::to_value(standard).unwrap();
        for path in ["root", "layer", "text", "dimension"] {
            let mut value = valid.clone();
            let target = match path {
                "root" => &mut value,
                "layer" => &mut value["layers"][0],
                "text" => &mut value["textStyles"][0],
                _ => &mut value["dimensionStyles"][0],
            };
            target["unknownRule"] = serde_json::json!(true);
            assert!(
                serde_json::from_value::<DrawingStandard>(value).is_err(),
                "{path}"
            );
        }
    }
}

pub const CAD_STANDARDS_INSTALL: &str = "cad.standards.install";
pub const CAD_STANDARDS_INSTALL_VERSION: u32 = 1;
/// The undo step's name.
pub const STANDARDS_INSTALL_STEP: &str = "Standart kaynaklarını kur";

/// A kind of named resource a bound office standard carries (docs/adr/1002 §1); a block is a library publication's
/// reference and a sheet template an approved template revision (docs/adr/1100 §2–§3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StandardResourceKind {
    LineType,
    HatchPattern,
    TextStyle,
    DimensionStyle,
    LeaderStyle,
    /// A named table style (docs/adr/1126 §6).
    TableStyle,
    Block,
    SheetTemplate,
}

/// What happens to one resource: added when the drawing lacks it, the drawing's replaced in place (its name and
/// local id kept, so what names it follows), the drawing's kept, or the standard's added under another name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StandardInstallAction {
    Add,
    Replace,
    KeepLocal,
    RenameIncoming,
}

/// The user's choice for one resource; a resource without one takes its default (add a missing one, keep a
/// differing or equal one).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardInstallChoice {
    pub kind: StandardResourceKind,
    /// The resource's name in the standard.
    pub name: String,
    pub action: StandardInstallAction,
    /// For `renameIncoming`: the name the standard's resource takes in the drawing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub new_name: Option<String>,
}

/// Input of `cad.standards.install` v1: the bound standards' named resources installed into the drawing by the
/// choices; a block reference is read from the first block library that holds its publication, a sheet template
/// revision is recorded as approved (docs/adr/1100). Refusals, in order: `invalid_file_path`, `file_not_found`,
/// `block_library_unreadable` (a `libraries` path, docs/adr/1019), `no_standard`, `invalid_standard`,
/// `standard_resource_not_found`, `invalid_install_choice`, `duplicate_name`, `invalid_revision`,
/// `revision_conflict`, `block_refused`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsInstall {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<StandardInstallChoice>>"))]
    pub choices: Vec<StandardInstallChoice>,
    /// A project's or office's block libraries (`kentos.blocks`, absolute paths ending `.kblok`, at most 16 MiB
    /// each), searched in this order before KentOS's own; each is named by its file stem in a row's `library`. Read
    /// at each call, never written to the drawing (docs/adr/1019).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    #[cfg_attr(feature = "schema", schemars(length(max = 16)))]
    pub libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// One resource of the bound standards and what the install does with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardInstallRow {
    pub kind: StandardResourceKind,
    pub name: String,
    /// `missing`, `same` or `differs`, against the drawing's resource of that name; `unavailable` when no library
    /// KentOS reads holds the block publication or template revision the standard names (kept, reported).
    pub status: String,
    pub action: StandardInstallAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub new_name: Option<String>,
    /// The differing appearance fields (`differs`); for `unavailable`, what the nearest library entry differs in
    /// (`release`, `units`, `revision`), none when no library has the type or template at all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub fields: Vec<String>,
    /// Blocks and sheet templates: the library the definition is read from (`kentos-ofis`, `sistem`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub library: Option<String>,
    /// Blocks: the drawing's inserts of its definition of that name, which follow a replaced definition; sheet
    /// templates: the sheets made from that template, which are never changed; table styles: the tables of the drawing
    /// and of its block definitions that follow the drawing's style of that name (docs/adr/1127 §1).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub uses: u32,
    /// The standard the resource comes from and its office revision.
    pub source: StandardOrigin,
}

/// Output of `cad.standards.install` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardsInstalled {
    /// Every resource of the bound standards, by kind then by the standards' order.
    pub rows: Vec<StandardInstallRow>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub replaced: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub renamed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u32,
    /// The references no readable library holds (each also counted in `kept`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unavailable: u32,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}


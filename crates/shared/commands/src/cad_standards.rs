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
fn check_resource_fields(resources: &serde_json::Value) -> Result<(), String> {
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
    keys(resources, "resources", &["lineTypes", "hatchPatterns"])?;
    let root = object(resources, "resources")?;
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
    /// The objects it is about (ObjectLineType, OwnLook).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// Whether `cad.standards.apply` fixes it.
    pub fixable: bool,
    /// For people: what is wrong and what fixes it.
    pub message: String,
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

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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
    MissingDimensionStyle,
    DimensionStyleName,
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
}

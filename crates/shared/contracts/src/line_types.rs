//! Line types with patterns (docs/adr/0313 §1; AutoCAD's LIN and LTYPE): a
//! named pattern of dashes, gaps, dots and complex elements (a text or a
//! shape placed where the pattern has come to), its unit (paper millimetres
//! that follow the drawing's scale, or the drawing's own lengths) and the
//! file it came from. A project keeps them in its settings (`lineTypes`); a
//! layer and an object name one (`linePattern`) and keep the nearest built-in
//! type in `lineType`, what a reader without patterns draws.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::LineType;

/// The most elements a pattern has (AutoCAD's LIN).
pub const MAX_PATTERN_ELEMENTS: usize = 12;
/// The longest element, in its unit.
pub const MAX_PATTERN_LENGTH: f64 = 1.0e6;

fn one() -> f64 {
    1.0
}

fn is_one(x: &f64) -> bool {
    *x == 1.0
}

fn is_zero(x: &f64) -> bool {
    *x == 0.0
}

fn not(b: &bool) -> bool {
    !*b
}

/// Where a pattern's lengths are measured.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PatternUnit {
    /// Millimetres on paper: the same on every sheet, the drawing's scale over 1000 metres in it (KentOS's own types).
    #[default]
    Paper,
    /// The drawing's own lengths, in the project's unit (a LIN file's).
    Drawing,
}

/// A complex element's placing: its size, its turn and its shift from where the pattern has come to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PatternPlace {
    /// S: its height (a text) or its scale (a shape), in the pattern's unit.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    /// R (along the line), A (absolute) or U (along the line, kept readable): degrees.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub absolute: bool,
    #[serde(default, skip_serializing_if = "not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub upright: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub x: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub y: f64,
}

/// A text in a pattern (`["GAZ",STANDARD,S=.1]`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PatternText {
    pub text: String,
    /// The text style it names, as written.
    pub style: String,
    #[serde(flatten)]
    pub place: PatternPlace,
}

/// A shape of a shape file in a pattern (`[CIRC1,ltypeshp.shx]`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PatternShape {
    pub name: String,
    /// The SHX file it is in, as written.
    pub file: String,
    #[serde(flatten)]
    pub place: PatternPlace,
}

/// An element of a pattern: a dash (a positive length), a gap (negative), a dot (0), or a text or a shape (0: it
/// takes no length of its own).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PatternElement {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<PatternText>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<PatternShape>,
}

impl PatternElement {
    /// Whether it is a text or a shape.
    pub fn is_complex(&self) -> bool {
        self.text.is_some() || self.shape.is_some()
    }
}

/// A project's line type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineTypeDef {
    /// One of its kind among the project's, letters' case aside.
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub description: String,
    pub elements: Vec<PatternElement>,
    #[serde(default)]
    pub unit: PatternUnit,
    /// The file it came from (`kentos.lin`, a DXF's name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
}

impl LineTypeDef {
    /// The built-in type nearest to it: continuous without a dash, dotted when its dashes are dots, dash-dot with a
    /// dot among dashes, else dashed.
    pub fn nearest(&self) -> LineType {
        let drawn: Vec<f64> = self
            .elements
            .iter()
            .filter(|e| !e.is_complex() && e.length >= 0.0)
            .map(|e| e.length)
            .collect();
        let gaps = self
            .elements
            .iter()
            .any(|e| !e.is_complex() && e.length < 0.0);
        if !gaps {
            LineType::Continuous
        } else if drawn.iter().all(|l| *l == 0.0) {
            LineType::Dotted
        } else if drawn.contains(&0.0) {
            LineType::Dashdot
        } else {
            LineType::Dashed
        }
    }

    /// The sum of its lengths' sizes, in its unit.
    pub fn period(&self) -> f64 {
        self.elements.iter().map(|e| e.length.abs()).sum()
    }
}

/// An element's length in the drawing, metres (docs/adr/0313 §3): a paper pattern's millimetres times the drawing's
/// scale over 1000, a drawing pattern's lengths in the project's unit (`unit`: its ratio to the metre, `(mul, div)`);
/// times the project's line type scale (LTSCALE) and the object's (CELTSCALE).
pub fn pattern_length_m(
    length: f64,
    unit: PatternUnit,
    plot_scale: f64,
    project_unit: (f64, f64),
    ltscale: f64,
    object_scale: f64,
) -> f64 {
    let base = match unit {
        PatternUnit::Paper => length * plot_scale / 1000.0,
        PatternUnit::Drawing => length * project_unit.0 / project_unit.1,
    };
    base * ltscale * object_scale
}

/// Whether two line type names are the same one (AutoCAD's: letters' case aside).
pub fn same_line_type_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// What is wrong with a project's line types, said so a reader can fix it; none: they hold.
pub fn line_types_problem(types: &[LineTypeDef]) -> Option<String> {
    for (i, t) in types.iter().enumerate() {
        let name = t.name.trim();
        if name.is_empty() {
            return Some("bir çizgi tipinin adı boş".into());
        }
        if name != t.name {
            return Some(format!(
                "“{}” çizgi tipinin adının başında ya da sonunda boşluk var",
                t.name
            ));
        }
        if types[..i]
            .iter()
            .any(|o| same_line_type_name(&o.name, name))
        {
            return Some(format!("“{name}” çizgi tipi iki kez var"));
        }
        if t.elements.len() > MAX_PATTERN_ELEMENTS {
            return Some(format!(
                "“{name}” çizgi tipinin deseni en çok {MAX_PATTERN_ELEMENTS} öğe olur, {} var",
                t.elements.len()
            ));
        }
        if t.elements
            .iter()
            .any(|e| !(e.length.is_finite() && e.length.abs() <= MAX_PATTERN_LENGTH))
        {
            return Some(format!("“{name}” çizgi tipinin bir uzunluğu geçersiz"));
        }
        if t.elements.iter().any(|e| e.is_complex() && e.length != 0.0) {
            return Some(format!(
                "“{name}” çizgi tipinde yazı ya da şeklin uzunluğu 0 olmalı"
            ));
        }
        if t.elements.iter().any(|e| {
            let place = e
                .text
                .as_ref()
                .map(|x| &x.place)
                .or(e.shape.as_ref().map(|s| &s.place));
            place.is_some_and(|p| {
                !(p.scale.is_finite()
                    && p.scale > 0.0
                    && p.rotation.is_finite()
                    && p.x.is_finite()
                    && p.y.is_finite())
            })
        }) {
            return Some(format!(
                "“{name}” çizgi tipinde bir yazının ya da şeklin yerleşimi geçersiz"
            ));
        }
        if let Some(first) = t.elements.iter().find(|e| !e.is_complex())
            && first.length < 0.0
        {
            return Some(format!(
                "“{name}” çizgi tipinin deseni bir çizgi ya da noktayla başlamalı"
            ));
        }
    }
    None
}

// ── cad.linetypes.edit ──────────────────────────────────────────────────────

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_LINE_TYPES_EDIT: &str = "cad.linetypes.edit";
pub const CAD_LINE_TYPES_EDIT_VERSION: u32 = 1;

/// What `cad.linetypes.edit` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LineTypesOperation {
    /// The types of a LIN file's text taken in.
    Load,
    /// One type added, or one of its name replaced.
    Add,
    /// One type taken away.
    Remove,
    /// The project's line type scale (LTSCALE) set.
    Scale,
    /// The project's types as a LIN file's text; writes nothing.
    Export,
}

/// `cad.linetypes.edit` v1 (docs/adr/0313 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineTypesEdit {
    pub operation: LineTypesOperation,
    /// load: a LIN file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lin: Option<String>,
    /// load: the file's name, each type's source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
    /// load: the types to take, by name; export: those to write. Absent: all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub names: Option<Vec<String>>,
    /// load, add: a type of a name the project has replaces it; else load skips it and add refuses.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub replace: bool,
    /// add: the type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<LineTypeDef>,
    /// remove: its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// scale: LTSCALE, above 0 and at most 1000; 1 is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A LIN file's line that did not read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LinLineProblem {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub line: u32,
    pub message: String,
}

/// What `cad.linetypes.edit` did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineTypesEdited {
    pub changed: bool,
    /// The types added, by name, in their order.
    pub added: Vec<String>,
    /// The project's types a load or an add replaced.
    pub replaced: Vec<String>,
    /// load: the file's types of a name the project has, kept as they are.
    pub skipped: Vec<String>,
    pub removed: Vec<String>,
    /// load: the file's lines that did not read.
    pub problems: Vec<LinLineProblem>,
    /// export: the LIN file's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lin: Option<String>,
    /// The project's line type scale after; absent: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

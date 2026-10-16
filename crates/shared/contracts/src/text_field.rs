//! A text's fields (docs/adr/0325 §1): a range of its letters bound to a source, an object's area, length, layer,
//! label, attribute, text or place, or the project's name, scale, unit, coordinate system or variable. The letters are the
//! field's last written value (its cache); the document writes them again before a step is recorded whenever the
//! source changed (kentos_domain's text_fields.rs). The source is typed, not an expression: the document recomputes
//! it at every step without the expression engine.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::identity::EntityId;

/// What a broken field's letters are.
pub const BROKEN_FIELD: &str = "####";

/// The most fields a text has.
pub const MAX_FIELDS: usize = 256;

/// The most decimals a field writes.
pub const MAX_FIELD_DECIMALS: u8 = 10;

/// A range of a text's letters bound to a source (docs/adr/0325 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextField {
    /// Its first letter and the one past its last, counting Unicode scalar values (as a run's); its letters are the
    /// value last written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub start: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub end: u32,
    pub source: FieldSource,
    /// How a number is written; absent: the project's decimals and unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub format: Option<FieldFormat>,
    /// Why its value cannot be read; its letters are then `####`. Absent: it reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<FieldBroken>,
}

/// Where a field's value comes from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldSource {
    /// An object, by its persistent id.
    #[serde(rename_all = "camelCase")]
    Object {
        uid: EntityId,
        value: ObjectValue,
        /// The attribute's name, with `attribute`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
    },
    /// The project.
    #[serde(rename_all = "camelCase")]
    Project {
        value: ProjectValue,
        /// The variable's name, with `variable`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
    },
}

/// An object's value a field writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ObjectValue {
    /// A closed object's area, its holes taken off.
    Area,
    /// A closed object's perimeter.
    Perimeter,
    /// A line's, an arc's, a path's or a curve's length.
    Length,
    /// Its layer's name.
    Layer,
    /// Its label.
    Label,
    /// The attribute named: a block insert's attribute value, else the object's attribute.
    Attribute,
    /// A text's letters (a cross reference).
    Text,
    /// Its east and north: a point's place, a text's, a block's or a table's insertion point, else where its label goes.
    X,
    Y,
}

/// The project's value a field writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProjectValue {
    /// The project's name.
    Name,
    /// The drawing's scale, `1:500`.
    Scale,
    /// The drawing unit, `m`.
    Unit,
    /// The coordinate system, `EPSG:5254`, or `Yerel`.
    Crs,
    /// The project's variable named (docs/adr/0214 §4).
    Variable,
}

/// How a field writes a number.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldFormat {
    /// Absent: the project's length or area decimals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(max = 10)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u8>,
    /// Absent: the drawing unit for a length or a place, the project's area unit for an area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<FieldUnit>,
    /// A decimal comma (`24,35`) instead of a point.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub comma: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub prefix: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub suffix: String,
}

/// The unit a field writes a number in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldUnit {
    Mm,
    Cm,
    M,
    Km,
    M2,
    Donum,
    Ha,
}

impl FieldUnit {
    /// Its written name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::M => "m",
            Self::Km => "km",
            Self::M2 => "m²",
            Self::Donum => "dönüm",
            Self::Ha => "ha",
        }
    }

    /// Whether it is an area's.
    pub fn is_area(self) -> bool {
        matches!(self, Self::M2 | Self::Donum | Self::Ha)
    }

    /// Metres (or square metres) in one of it.
    pub fn metres(self) -> f64 {
        match self {
            Self::Mm => 0.001,
            Self::Cm => 0.01,
            Self::M | Self::M2 => 1.0,
            Self::Km | Self::Donum => 1000.0,
            Self::Ha => 10_000.0,
        }
    }
}

/// Why a field's value cannot be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldBroken {
    /// The object is gone.
    Removed,
    /// The object has no such value (an open object's area, a line's text).
    NoValue,
    /// The attribute or the variable named is not there.
    NotNamed,
    /// Texts whose fields read each other in a ring.
    Cycle,
}

impl FieldBroken {
    /// Its words for the user.
    pub fn words(self) -> &'static str {
        match self {
            Self::Removed => "nesne silindi",
            Self::NoValue => "nesnenin bu değeri yok",
            Self::NotNamed => "adı verilen öznitelik ya da değişken yok",
            Self::Cycle => "yazılar birbirini okuyor (döngü)",
        }
    }
}

impl ObjectValue {
    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            Self::Area => "Alan",
            Self::Perimeter => "Çevre",
            Self::Length => "Uzunluk",
            Self::Layer => "Katman",
            Self::Label => "Etiket",
            Self::Attribute => "Öznitelik",
            Self::Text => "Yazı",
            Self::X => "Doğu (X)",
            Self::Y => "Kuzey (Y)",
        }
    }

    /// Whether it is a number.
    pub fn is_number(self) -> bool {
        matches!(
            self,
            Self::Area | Self::Perimeter | Self::Length | Self::X | Self::Y
        )
    }
}

impl ProjectValue {
    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Proje adı",
            Self::Scale => "Çizim ölçeği",
            Self::Unit => "Çizim birimi",
            Self::Crs => "Koordinat sistemi",
            Self::Variable => "Değişken",
        }
    }
}

impl FieldSource {
    /// The object it reads, when it reads one.
    pub fn object(&self) -> Option<EntityId> {
        match self {
            Self::Object { uid, .. } => Some(*uid),
            Self::Project { .. } => None,
        }
    }

    /// What is wrong with it: the refusal's words.
    pub fn problem(&self) -> Option<String> {
        let named = |n: &Option<String>| n.as_deref().is_some_and(|n| !n.trim().is_empty());
        match self {
            Self::Object { value, name, .. } => match (value, named(name)) {
                (ObjectValue::Attribute, false) => {
                    Some("Öznitelik alanı özniteliğin adını ister (`name`).".to_owned())
                }
                (ObjectValue::Attribute, true) | (_, false) => None,
                (_, true) => Some("Ad yalnız öznitelik alanında verilir.".to_owned()),
            },
            Self::Project { value, name } => match (value, named(name)) {
                (ProjectValue::Variable, false) => {
                    Some("Değişken alanı değişkenin adını ister (`name`).".to_owned())
                }
                (ProjectValue::Variable, true) | (_, false) => None,
                (_, true) => Some("Ad yalnız değişken alanında verilir.".to_owned()),
            },
        }
    }
}

impl FieldFormat {
    /// Nothing set.
    pub fn is_plain(&self) -> bool {
        *self == Self::default()
    }

    /// What is wrong with it: the refusal's words.
    pub fn problem(&self) -> Option<String> {
        if self.decimals.is_some_and(|d| d > MAX_FIELD_DECIMALS) {
            return Some(format!("Basamak en çok {MAX_FIELD_DECIMALS} olur."));
        }
        if self.is_plain() {
            return Some("Boş biçim yazılmaz; alanı kaldırın.".to_owned());
        }
        None
    }
}

/// What is wrong with a text's fields for its letters `text` (and its runs' rule of ranges): in order, not
/// overlapping, inside the text, each source and format right, a broken field's letters `####`. None when they may be
/// written.
pub fn text_fields_problem(fields: &[TextField], text: &str) -> Option<String> {
    if fields.len() > MAX_FIELDS {
        return Some(format!(
            "Bir yazıda en çok {MAX_FIELDS} alan olur; {} verildi.",
            fields.len()
        ));
    }
    let letters: Vec<char> = text.chars().collect();
    let mut last = 0u32;
    for (i, f) in fields.iter().enumerate() {
        let n = i + 1;
        if f.start >= f.end || f.end as usize > letters.len() || (i > 0 && f.start < last) {
            return Some(format!(
                "{n}. alan {}–{}: başı sonundan önce, öncekinden sonra olmalı, sonu yazının harf sayısını ({}) aşmamalı.",
                f.start,
                f.end,
                letters.len()
            ));
        }
        last = f.end;
        if let Some(words) = f.source.problem() {
            return Some(format!("{n}. alan: {words}"));
        }
        if let Some(words) = f.format.as_ref().and_then(FieldFormat::problem) {
            return Some(format!("{n}. alan: {words}"));
        }
        if let (Some(unit), FieldSource::Object { value, .. }) =
            (f.format.as_ref().and_then(|f| f.unit), &f.source)
            && (!value.is_number() || unit.is_area() != (*value == ObjectValue::Area))
        {
            return Some(format!(
                "{n}. alan: {} değeri {} biriminde yazılamaz.",
                value.label(),
                unit.name()
            ));
        }
        if matches!(f.source, FieldSource::Project { .. })
            && f.format.as_ref().is_some_and(|f| f.unit.is_some())
        {
            return Some(format!("{n}. alan: projenin değeri birimle yazılmaz."));
        }
        let own: String = letters[f.start as usize..f.end as usize].iter().collect();
        if f.broken.is_some() && own != BROKEN_FIELD {
            return Some(format!("{n}. alan kopuk: harfleri {BROKEN_FIELD} olmalı."));
        }
    }
    None
}

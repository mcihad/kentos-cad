//! Veri çıkarımı (docs/adr/1030, plan/cad-advanced.md `CAD-ADV-13`): a reusable recipe that reads drawings (the open
//! one and, read only, others), takes their objects by kind, block and layer within a declared counting scope (nested
//! blocks, associative arrays, hidden parts, external references), writes each occurrence's fields (its words, or a
//! quantity with its unit), groups, sorts, totals and joins a description table by a key. The recipe is kept on the
//! table it wrote (`TableSource::Extraction`, `.kcad` schema 182) with what that table was written from: the sources'
//! digests, each data row's key and the cells last extracted, so Tabloyu güncelle tells a cell changed by hand from
//! a source that changed.
//!
//! A quantity is a number with a unit: an occurrence count (adet), a length, an area, a dimension's measured value,
//! or another quantity column times a declared factor. A dimension's shown text is words: it is never summed, even
//! when it reads as a number (an override). Sums are worked out from the unrounded values; the cells round them at
//! the end (`display::fixed`, half away from zero after the noise is rounded away at 7 decimals).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::table_cells::SourceOverrides;

/// The most columns a recipe writes (its joined ones too).
pub const MAX_EXTRACT_COLUMNS: usize = 30;
/// The most other drawings a recipe reads.
pub const MAX_EXTRACT_DRAWINGS: usize = 16;
/// The most kinds, blocks or layers a recipe names.
pub const MAX_EXTRACT_NAMES: usize = 500;
/// The most sort keys.
pub const MAX_EXTRACT_SORT: usize = 5;
/// The most decimals a quantity column writes.
pub const MAX_EXTRACT_DECIMALS: u32 = 10;
/// The largest factor (in magnitude) a computed column takes.
pub const MAX_EXTRACT_FACTOR: f64 = 1.0e9;
/// The most occurrences one extraction reads (every drawing's); past it the extraction is refused, not cut.
pub const MAX_EXTRACT_OCCURRENCES: usize = 1_000_000;
/// The deepest a nested block is looked into (as the documents keep blocks, `kentos_contracts::blocks`).
pub const MAX_EXTRACT_DEPTH: usize = 16;

/// The kinds of objects a recipe may take, by their contract names.
pub const EXTRACT_KINDS: [&str; 13] = [
    "insert",
    "line",
    "polyline",
    "polygon",
    "circle",
    "arc",
    "ellipse",
    "spline",
    "hatch",
    "dimension",
    "text",
    "point",
    "leader",
];

/// A kind's Turkish name, as a Tür column writes it; none for a kind a recipe does not take.
pub fn extract_kind_label(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "insert" => "Blok",
        "line" => "Çizgi",
        "polyline" => "Çoklu çizgi",
        "polygon" => "Alan",
        "circle" => "Daire",
        "arc" => "Yay",
        "ellipse" => "Elips",
        "spline" => "Eğri",
        "hatch" => "Tarama",
        "dimension" => "Ölçü",
        "text" => "Yazı",
        "point" => "Nokta",
        "leader" => "Kılavuz",
        "array" => "Dizi",
        _ => return None,
    })
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

/// A data extraction recipe (docs/adr/1030 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractionRecipe {
    /// Its name (Tarif adı); not empty, one line, at most 100 letters.
    pub name: String,
    /// The open drawing is read (default yes).
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub current: bool,
    /// Other drawings read (read only: never saved nor changed): `.kcad` or KentOS JSON paths, absolute or from the
    /// drawing's folder; each once, at most [`MAX_EXTRACT_DRAWINGS`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub drawings: Vec<String>,
    /// The kinds taken ([`EXTRACT_KINDS`]); none named: inserts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub kinds: Vec<String>,
    /// The blocks whose inserts are taken, by name (Turkish case folded as block names are); none named: every block.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub blocks: Vec<String>,
    /// The layers whose objects are taken, by name (an object's effective layer: a block's piece on the block's layer
    /// is on its insert's); none named: every layer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub layers: Vec<String>,
    /// What is counted (docs/adr/1030 §3).
    #[serde(default, skip_serializing_if = "CountScope::is_default")]
    #[cfg_attr(feature = "ts", ts(as = "Option<CountScope>", optional))]
    pub scope: CountScope,
    /// The columns, in order; at least one, at most [`MAX_EXTRACT_COLUMNS`].
    pub columns: Vec<ExtractColumn>,
    /// Alike rows one (Benzer satırları birleştir): rows whose word columns read the same, their quantities summed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub group: bool,
    /// The rows' order: by these columns in turn; ties keep the reading order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ExtractSort>>", optional))]
    pub sort: Vec<ExtractSort>,
    /// A last row, Toplam, with every quantity column's sum.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub total: bool,
    /// A description table joined by a key (Dış veriyle eşle).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub join: Option<ExtractJoin>,
}

/// What a recipe counts (docs/adr/1030 §3): each choice open, none implied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CountScope {
    /// İç içe bloklar: a block's objects are looked into, its nested inserts counted (each by its path), its pieces'
    /// lengths and areas measured as placed. Off: the drawing's own objects alone.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub nested: bool,
    /// How an associative array counts.
    #[serde(default, skip_serializing_if = "ArrayCount::is_items")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ArrayCount>", optional))]
    pub arrays: ArrayCount,
    /// Görünmeyen parçalar: a dynamic block's pieces its visibility state hides and the objects on a hidden or frozen
    /// layer are counted too. Off: left out (the report says how many).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub hidden: bool,
    /// Dış referanslar: an external reference's content is looked into as a nested block's (its objects by their
    /// source ids). Off: an external reference is one insert. A reference to a drawing the recipe reads itself is
    /// never looked into: that drawing is counted once, as its own.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub xrefs: bool,
}

impl CountScope {
    pub fn is_default(&self) -> bool {
        *self == CountScope::default()
    }
}

/// How an associative array counts (docs/adr/1030 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArrayCount {
    /// Each item an insert of its own (an item removed by hand is not there).
    #[default]
    Items,
    /// The array one occurrence (Dizi) of its block; its items not counted apart.
    Whole,
}

impl ArrayCount {
    pub fn is_items(&self) -> bool {
        *self == ArrayCount::Items
    }
}

/// One column of a recipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractColumn {
    /// Its heading; a quantity's unit is written after it in brackets (`Uzunluk (m)`). Not empty, one line.
    pub label: String,
    pub value: ExtractValue,
    /// A quantity's decimals; absent: the project's (lengths' for a length or a dimension, areas' for an area, 0 for
    /// a count, the column's it multiplies for a computed one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(max = 10)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u32>,
}

/// What a column writes of an occurrence (docs/adr/1030 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "of", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExtractValue {
    /// The drawing it is in: its file's name, Bu çizim for the open one.
    Drawing,
    /// Its kind's Turkish name (Blok, Çizgi …; Dizi for an array counted whole).
    Kind,
    /// Its block's name; empty for another kind.
    Block,
    /// Its effective layer's name.
    Layer,
    /// A dynamic block's variant: its parameter values that differ from the defaults, `name=value` by name, `; `
    /// between; empty for the defaults and for another kind.
    Variant,
    /// An attribute's effective value (a constant's, the insert's, the definition's default) or the object's datum.
    Attribute { tag: String },
    /// Adet: one an occurrence.
    Count,
    /// Uzunluk: a curve's length (a closed one's perimeter, an area's with its holes), placed as it is drawn.
    Length {
        #[serde(default, skip_serializing_if = "ExtractLengthUnit::is_metre")]
        #[cfg_attr(feature = "ts", ts(as = "Option<ExtractLengthUnit>", optional))]
        unit: ExtractLengthUnit,
    },
    /// Alan: a closed object's area net of its holes, placed as it is drawn.
    Area {
        #[serde(default, skip_serializing_if = "ExtractAreaUnit::is_square_metre")]
        #[cfg_attr(feature = "ts", ts(as = "Option<ExtractAreaUnit>", optional))]
        unit: ExtractAreaUnit,
    },
    /// Gerçek ölçü: a length dimension's measured value from its geometry (never its written text).
    Measured {
        #[serde(default, skip_serializing_if = "ExtractLengthUnit::is_metre")]
        #[cfg_attr(feature = "ts", ts(as = "Option<ExtractLengthUnit>", optional))]
        unit: ExtractLengthUnit,
    },
    /// Yazılan: a dimension's text as shown (words, never summed).
    Shown,
    /// Elle yazılmış: whether a dimension's text overrides its measure (evet, hayır).
    Overridden,
    /// Hesaplanan: an earlier quantity column (by index) times `factor` (a decimal number, `.` before its fraction),
    /// in `unit` (one line; may be empty for a pure number).
    Computed {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        column: u32,
        factor: String,
        #[serde(default)]
        unit: String,
    },
    /// A joined table's column by its heading (the join's row whose key is this row's key column's words).
    Joined { field: String },
}

impl ExtractValue {
    /// Whether it is a quantity (a number with a unit, summed).
    pub fn is_quantity(&self) -> bool {
        matches!(
            self,
            ExtractValue::Count
                | ExtractValue::Length { .. }
                | ExtractValue::Area { .. }
                | ExtractValue::Measured { .. }
                | ExtractValue::Computed { .. }
        )
    }
}

/// A length's unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExtractLengthUnit {
    #[default]
    M,
    Cm,
    Mm,
    Km,
}

impl ExtractLengthUnit {
    pub fn is_metre(&self) -> bool {
        *self == ExtractLengthUnit::M
    }

    /// Its symbol.
    pub fn symbol(self) -> &'static str {
        match self {
            ExtractLengthUnit::M => "m",
            ExtractLengthUnit::Cm => "cm",
            ExtractLengthUnit::Mm => "mm",
            ExtractLengthUnit::Km => "km",
        }
    }

    /// A metre's worth: `metres × per_metre` is the value in it (exact powers of ten).
    pub fn per_metre(self) -> f64 {
        match self {
            ExtractLengthUnit::M => 1.0,
            ExtractLengthUnit::Cm => 100.0,
            ExtractLengthUnit::Mm => 1000.0,
            ExtractLengthUnit::Km => 0.001,
        }
    }
}

/// An area's unit (CLAUDE.md §5: a dönüm 1000 m², a hectare 10000 m²).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExtractAreaUnit {
    #[default]
    M2,
    Donum,
    Ha,
}

impl ExtractAreaUnit {
    pub fn is_square_metre(&self) -> bool {
        *self == ExtractAreaUnit::M2
    }

    pub fn symbol(self) -> &'static str {
        match self {
            ExtractAreaUnit::M2 => "m²",
            ExtractAreaUnit::Donum => "dönüm",
            ExtractAreaUnit::Ha => "ha",
        }
    }

    /// The square metres one of it is: `m² / square_metres` is the value in it.
    pub fn square_metres(self) -> f64 {
        match self {
            ExtractAreaUnit::M2 => 1.0,
            ExtractAreaUnit::Donum => 1000.0,
            ExtractAreaUnit::Ha => 10_000.0,
        }
    }
}

/// A sort key: a column by index, up or down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractSort {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub column: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub descending: bool,
}

/// A description table joined to the rows (docs/adr/1030 §5): a CSV, XLSX or ODS file's sheet whose first row names
/// its columns; its row whose `key` column reads as a row's `on` column is that row's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractJoin {
    /// Its path, absolute or from the drawing's folder.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheet: Option<String>,
    /// The file's key column's heading: its words unique and not empty on every row.
    pub key: String,
    /// The recipe's column (by index, a word column not joined) whose words are looked up.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub on: u32,
}

/// A source an extraction table was written from, as last read: a drawing or the joined file by its path as the
/// recipe gives it and the SHA-256 of its bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractRead {
    pub path: String,
    pub digest: String,
}

/// What an extraction table keeps (`TableSource::Extraction`, docs/adr/1030 §6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExtractionSource {
    pub recipe: ExtractionRecipe,
    /// The other drawings and the joined file as last read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ExtractRead>>", optional))]
    pub read: Vec<ExtractRead>,
    /// Each data row's key as last extracted, in the rows' order: a grouped row's word cells, an ungrouped row's
    /// occurrence path; unique.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub keys: Vec<String>,
    /// The extracted cells as last written (heading, data rows, total): a cell of them that the table reads otherwise
    /// was changed by hand. Only the recipe's columns: the table's further columns are the user's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Vec<String>>>", optional))]
    pub copy: Vec<Vec<String>>,
    /// What Tabloyu güncelle does with a cell changed by hand whose source changed too.
    #[serde(default, skip_serializing_if = "SourceOverrides::is_keep")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SourceOverrides>", optional))]
    pub overrides: SourceOverrides,
}

/// A recipe column's heading as a table writes it: the label, a quantity's unit after it in brackets (not when the
/// label is the unit's word itself: Adet, not “Adet (adet)”).
pub fn extract_heading(c: &ExtractColumn) -> String {
    let unit = match &c.value {
        ExtractValue::Count => "adet",
        ExtractValue::Length { unit } | ExtractValue::Measured { unit } => unit.symbol(),
        ExtractValue::Area { unit } => unit.symbol(),
        ExtractValue::Computed { unit, .. } => unit.as_str(),
        _ => "",
    };
    if unit.is_empty() || crate::blocks::name_key(c.label.trim()) == unit {
        c.label.clone()
    } else {
        format!("{} ({unit})", c.label)
    }
}

/// `factor` as a computed column reads it: a decimal number (an optional `-`, digits, an optional `.` and digits),
/// finite and at most [`MAX_EXTRACT_FACTOR`] in magnitude.
pub fn extract_factor(factor: &str) -> Option<f64> {
    let body = factor.strip_prefix('-').unwrap_or(factor);
    let (whole, fraction) = body.split_once('.').unwrap_or((body, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty()
        || !digits(whole)
        || !digits(fraction)
        || (body.contains('.') && fraction.is_empty())
    {
        return None;
    }
    let x: f64 = factor.parse().ok()?;
    (x.is_finite() && x.abs() <= MAX_EXTRACT_FACTOR).then_some(x)
}

fn one_line(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(char::is_control)
}

/// What is wrong with a recipe, if anything: the field and the words (docs/adr/1030 §2).
pub fn extraction_recipe_problem(r: &ExtractionRecipe) -> Option<(String, String)> {
    let fail = |at: String, words: String| Some((at, words));
    if !one_line(&r.name) || r.name.chars().count() > 100 {
        return fail(
            "name".into(),
            "Tarifin adı boş olamaz, tek satır ve en çok 100 harf olur.".into(),
        );
    }
    if !r.current && r.drawings.is_empty() {
        return fail(
            "drawings".into(),
            "Tarif hiçbir çizimi okumuyor: açık çizimi ya da en az bir çizim dosyasını seçin."
                .into(),
        );
    }
    if r.drawings.len() > MAX_EXTRACT_DRAWINGS {
        return fail(
            "drawings".into(),
            format!(
                "{} çizim verildi; bir tarif en çok {MAX_EXTRACT_DRAWINGS} çizim okur.",
                r.drawings.len()
            ),
        );
    }
    for (i, d) in r.drawings.iter().enumerate() {
        if !one_line(d) {
            return fail(
                format!("drawings[{i}]"),
                "Çizim dosyasının yolu boş olamaz.".into(),
            );
        }
        if r.drawings[..i].contains(d) {
            return fail(
                format!("drawings[{i}]"),
                format!("“{d}” çizimi iki kez verildi; her çizim bir kez sayılır."),
            );
        }
    }
    for (field, names) in [
        ("kinds", &r.kinds),
        ("blocks", &r.blocks),
        ("layers", &r.layers),
    ] {
        if names.len() > MAX_EXTRACT_NAMES {
            return fail(
                field.into(),
                format!("En çok {MAX_EXTRACT_NAMES} ad verilir."),
            );
        }
        if let Some(i) = names.iter().position(|n| !one_line(n)) {
            return fail(format!("{field}[{i}]"), "Ad boş olamaz.".into());
        }
    }
    if let Some(i) = r
        .kinds
        .iter()
        .position(|k| !EXTRACT_KINDS.contains(&k.as_str()))
    {
        return fail(
            format!("kinds[{i}]"),
            format!(
                "“{}” bir nesne türü değil; şunlardan biri olmalı: {}.",
                r.kinds[i],
                EXTRACT_KINDS.join(", ")
            ),
        );
    }
    if r.columns.is_empty() || r.columns.len() > MAX_EXTRACT_COLUMNS {
        return fail(
            "columns".into(),
            format!(
                "Tarifin {} sütunu var; en az 1, en çok {MAX_EXTRACT_COLUMNS} olmalı.",
                r.columns.len()
            ),
        );
    }
    for (i, c) in r.columns.iter().enumerate() {
        let at = |f: &str| format!("columns[{i}].{f}");
        if !one_line(&c.label) || c.label.chars().count() > 100 {
            return fail(
                at("label"),
                "Sütun başlığı boş olamaz, tek satır ve en çok 100 harf olur.".into(),
            );
        }
        if c.decimals.is_some_and(|d| d > MAX_EXTRACT_DECIMALS) {
            return fail(
                at("decimals"),
                format!("En çok {MAX_EXTRACT_DECIMALS} ondalık yazılır."),
            );
        }
        if c.decimals.is_some() && !c.value.is_quantity() {
            return fail(
                at("decimals"),
                "Ondalık yalnız miktar sütunlarına (adet, uzunluk, alan, ölçü, hesaplanan) verilir.".into(),
            );
        }
        match &c.value {
            ExtractValue::Attribute { tag } if !one_line(tag) => {
                return fail(at("value.tag"), "Özniteliğin adı boş olamaz.".into());
            }
            ExtractValue::Computed {
                column,
                factor,
                unit,
            } => {
                let source = *column as usize;
                if source >= i {
                    return fail(
                        at("value.column"),
                        "Hesaplanan sütun kendinden önceki bir miktar sütununu çarpar.".into(),
                    );
                }
                if !r.columns[source].value.is_quantity() {
                    let words = if matches!(r.columns[source].value, ExtractValue::Shown) {
                        "Ölçünün yazılan metni sayı olarak hesaplanmaz; gerçek ölçü için Gerçek ölçü sütununu kullanın."
                    } else {
                        "Hesaplanan sütun yalnız bir miktar sütununu (adet, uzunluk, alan, ölçü) çarpar."
                    };
                    return fail(at("value.column"), words.into());
                }
                if extract_factor(factor).is_none() {
                    return fail(
                        at("value.factor"),
                        format!(
                            "“{factor}” bir katsayı değil: noktalı ondalık bir sayı yazın (1.05 gibi), büyüklüğü en çok {MAX_EXTRACT_FACTOR:e}."
                        ),
                    );
                }
                if unit.chars().any(char::is_control) || unit.chars().count() > 20 {
                    return fail(
                        at("value.unit"),
                        "Birim tek satır ve en çok 20 harf olur.".into(),
                    );
                }
            }
            ExtractValue::Joined { field } => {
                if !one_line(field) {
                    return fail(
                        at("value.field"),
                        "Eşlenen sütunun başlığı boş olamaz.".into(),
                    );
                }
                if r.join.is_none() {
                    return fail(
                        at("value"),
                        "Eşlenen sütun bir dış tablo ister: Dış veriyle eşle seçimini yapın."
                            .into(),
                    );
                }
            }
            _ => {}
        }
    }
    if r.sort.len() > MAX_EXTRACT_SORT {
        return fail(
            "sort".into(),
            format!("En çok {MAX_EXTRACT_SORT} sıralama anahtarı verilir."),
        );
    }
    for (k, s) in r.sort.iter().enumerate() {
        if s.column as usize >= r.columns.len() {
            return fail(
                format!("sort[{k}].column"),
                "Sıralama tarifin bir sütununu göstermeli.".into(),
            );
        }
        if r.sort[..k].iter().any(|o| o.column == s.column) {
            return fail(
                format!("sort[{k}].column"),
                "Bir sütuna göre bir kez sıralanır.".into(),
            );
        }
    }
    if r.group && !r.columns.iter().any(|c| c.value.is_quantity()) {
        return fail(
            "group".into(),
            "Benzer satırları birleştirmek için tarifte bir miktar sütunu (adet gibi) olmalı; yoksa birleşen satırların sayısı kaybolur.".into(),
        );
    }
    if r.total && !r.columns.iter().any(|c| c.value.is_quantity()) {
        return fail(
            "total".into(),
            "Toplam satırı için tarifte bir miktar sütunu olmalı.".into(),
        );
    }
    if r.total && r.columns.iter().all(|c| c.value.is_quantity()) {
        return fail(
            "total".into(),
            "Toplam satırının adı bir yazı sütununa yazılır: tarife blok, tür ya da katman gibi bir yazı sütunu ekleyin.".into(),
        );
    }
    if let Some(j) = &r.join {
        if !one_line(&j.path) {
            return fail("join.path".into(), "Dış tablonun yolu boş olamaz.".into());
        }
        if j.sheet.as_deref().is_some_and(|s| !one_line(s)) {
            return fail(
                "join.sheet".into(),
                "Dış tablonun sayfasının adı boş olamaz.".into(),
            );
        }
        if !one_line(&j.key) {
            return fail(
                "join.key".into(),
                "Dış tablonun anahtar sütununun başlığı boş olamaz.".into(),
            );
        }
        let on = r.columns.get(j.on as usize);
        if on
            .is_none_or(|c| c.value.is_quantity() || matches!(c.value, ExtractValue::Joined { .. }))
        {
            return fail(
                "join.on".into(),
                "Eşleme anahtarı tarifin bir yazı sütunu (blok, öznitelik gibi) olmalı; miktar ya da eşlenen sütun olamaz.".into(),
            );
        }
        if !r
            .columns
            .iter()
            .any(|c| matches!(c.value, ExtractValue::Joined { .. }))
        {
            return fail(
                "join".into(),
                "Dış tablodan alınacak bir sütun (Eşlenen) verilmedi.".into(),
            );
        }
    }
    None
}

/// What is wrong with an extraction table's kept source, if anything (its recipe's, then its reads', keys' and
/// copy's shape): the field and the words.
pub fn extraction_source_problem(s: &ExtractionSource, columns: usize) -> Option<(String, String)> {
    if let Some((at, words)) = extraction_recipe_problem(&s.recipe) {
        return Some((format!("recipe.{at}"), words));
    }
    let digest_ok = |d: &str| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit());
    if s.read
        .iter()
        .any(|r| !one_line(&r.path) || !digest_ok(&r.digest))
    {
        return Some((
            "read".into(),
            "Çıkarımın okuduğu kaynakların yolu boş olamaz, özeti 64 onaltılık rakamlık bir SHA-256 olur.".into(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    if s.keys
        .iter()
        .any(|k| k.is_empty() || !seen.insert(k.as_str()))
    {
        return Some((
            "keys".into(),
            "Çıkarımın satır anahtarları boş olamaz ve her biri bir kez geçer.".into(),
        ));
    }
    let width = s.recipe.columns.len();
    if width > columns {
        return Some((
            "copy".into(),
            "Tablonun sütunu tarifin sütunlarından az; çıkarım tablosunun ilk sütunları tarifinkilerdir.".into(),
        ));
    }
    if !s.copy.is_empty() {
        let rows = 1 + s.keys.len() + usize::from(s.recipe.total);
        if s.copy.len() != rows
            || s.copy.iter().any(|r| {
                r.len() != width || r.iter().any(|w| crate::table::line_problem(w).is_some())
            })
        {
            return Some((
                "copy".into(),
                "Çıkarımın son kopyası başlık, her anahtar için bir satır ve toplamdan oluşur; her satırda tarifin sütunları kadar hücre olur.".into(),
            ));
        }
    }
    None
}

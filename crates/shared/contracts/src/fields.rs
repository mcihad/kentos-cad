//! A layer's fields (docs/adr/0199 §1, §2): the schema a layer may give its
//! objects' attributes. Values stay text; a field says which text it takes
//! and in what one form (its canonical text), which the product commands
//! write, the attribute table sorts by and the forms edit.
//!
//! The rules are written once here; the web keeps the same in
//! `apps/web/src/model/layerFields.ts`. Both pass the independent
//! reference's cases (`scripts/fixtures/layer_field_cases.py`,
//! `fixtures/layer-fields/v1/cases.json`) word for word.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The largest whole number a field takes, in magnitude: 2⁵³ − 1, the
/// largest a JavaScript number keeps exactly.
pub const MAX_INTEGER: u64 = 9_007_199_254_740_991;
/// The most digits a decimal value has, whole and fraction together.
pub const MAX_DIGITS: usize = 30;
/// The most characters of a field's name and of its alias.
pub const MAX_NAME: usize = 64;
/// A text field's longest length.
pub const MAX_LENGTH: u32 = 10_000;
/// A decimal field's most fraction digits.
pub const MAX_SCALE: u32 = 15;
/// A unit's most characters (docs/adr/0438 §4).
pub const MAX_UNIT: usize = 16;
/// A JSON value's most characters (docs/adr/0629 §2).
pub const MAX_JSON: usize = 1_000_000;
/// A JSON value's deepest nesting (docs/adr/0629 §2).
pub const MAX_JSON_DEPTH: usize = 64;
/// A binary value's most bytes (docs/adr/0629 §2): 1 MiB.
pub const MAX_BINARY: usize = 1 << 20;
/// A listed value's description's most characters (docs/adr/0438 §5).
pub const MAX_DESCRIPTION: usize = 500;

/// What a field holds (docs/adr/0199 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerFieldKind {
    Text,
    Integer,
    Decimal,
    Date,
    Boolean,
    /// A date and a time of day, local (docs/adr/0412 §1): `YYYY-AA-GGTSS:DD:SS`.
    #[serde(rename = "datetime")]
    DateTime,
    /// A time of day (docs/adr/0412 §1): `SS:DD:SS`.
    Time,
    /// A 64-bit whole number (docs/adr/0629 §2), past 2⁵³ as well.
    #[serde(rename = "bigint")]
    BigInt,
    /// An instant: a date and time with its offset from UTC (docs/adr/0629 §2), `…TSS:DD:SS[.k]Z|±SS:DD`.
    Instant,
    /// A UUID, lowercase with hyphens (docs/adr/0629 §2).
    Uuid,
    /// A JSON value without its insignificant white space (docs/adr/0629 §2).
    Json,
    /// Bytes as base64 with padding, at most 1 MiB (docs/adr/0629 §2).
    Binary,
}

impl LayerFieldKind {
    /// Every kind, in the order the Alanlar window lists them.
    pub const ALL: [LayerFieldKind; 12] = [
        LayerFieldKind::Text,
        LayerFieldKind::Integer,
        LayerFieldKind::BigInt,
        LayerFieldKind::Decimal,
        LayerFieldKind::Date,
        LayerFieldKind::DateTime,
        LayerFieldKind::Instant,
        LayerFieldKind::Time,
        LayerFieldKind::Boolean,
        LayerFieldKind::Uuid,
        LayerFieldKind::Json,
        LayerFieldKind::Binary,
    ];

    /// Its name in the contract.
    pub fn name(self) -> &'static str {
        match self {
            LayerFieldKind::Text => "text",
            LayerFieldKind::Integer => "integer",
            LayerFieldKind::Decimal => "decimal",
            LayerFieldKind::Date => "date",
            LayerFieldKind::Boolean => "boolean",
            LayerFieldKind::DateTime => "datetime",
            LayerFieldKind::Time => "time",
            LayerFieldKind::BigInt => "bigint",
            LayerFieldKind::Instant => "instant",
            LayerFieldKind::Uuid => "uuid",
            LayerFieldKind::Json => "json",
            LayerFieldKind::Binary => "binary",
        }
    }

    /// The kind named `name`, if one is.
    pub fn from_name(name: &str) -> Option<LayerFieldKind> {
        LayerFieldKind::ALL.into_iter().find(|k| k.name() == name)
    }

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            LayerFieldKind::Text => "Metin",
            LayerFieldKind::Integer => "Tam sayı",
            LayerFieldKind::Decimal => "Ondalık sayı",
            LayerFieldKind::Date => "Tarih",
            LayerFieldKind::Boolean => "Evet/hayır",
            LayerFieldKind::DateTime => "Tarih ve saat",
            LayerFieldKind::Time => "Saat",
            LayerFieldKind::BigInt => "Büyük tam sayı",
            LayerFieldKind::Instant => "Zaman damgası",
            LayerFieldKind::Uuid => "UUID",
            LayerFieldKind::Json => "JSON",
            LayerFieldKind::Binary => "İkili",
        }
    }

    /// Whether it is one of docs/adr/0629's kinds, a file's of schema 104.
    pub fn is_wide(self) -> bool {
        matches!(
            self,
            LayerFieldKind::BigInt
                | LayerFieldKind::Instant
                | LayerFieldKind::Uuid
                | LayerFieldKind::Json
                | LayerFieldKind::Binary
        )
    }

    /// Whether its values are numbers (a range and a value list take them).
    pub fn is_number(self) -> bool {
        matches!(
            self,
            LayerFieldKind::Integer | LayerFieldKind::Decimal | LayerFieldKind::BigInt
        )
    }

    /// Whether its values are a date, a date and a time or a time of day (docs/adr/0412).
    pub fn is_temporal(self) -> bool {
        matches!(
            self,
            LayerFieldKind::Date
                | LayerFieldKind::DateTime
                | LayerFieldKind::Time
                | LayerFieldKind::Instant
        )
    }
}

/// One value of a field's value list: the code written to the attribute
/// and the label shown for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldChoice {
    pub code: String,
    pub label: String,
    /// Kullanımdan kalktı (docs/adr/0418): a record that holds it keeps it and shows it, a new
    /// choice does not offer it, a form does not write it anew. Written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub retired: bool,
    /// What the value means, said beside it in a list (docs/adr/0438 §5): 1–500 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// The first day the value holds (`YYYY-AA-GG`, docs/adr/0438 §5); before it a new choice does not
    /// offer it, as a retired one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub from: Option<String>,
    /// The last day the value holds (`YYYY-AA-GG`); after it a new choice does not offer it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub until: Option<String>,
}

impl FieldChoice {
    /// A value offered for new choices.
    pub fn new(code: impl Into<String>, label: impl Into<String>) -> FieldChoice {
        FieldChoice {
            code: code.into(),
            label: label.into(),
            retired: false,
            description: None,
            from: None,
            until: None,
        }
    }

    /// Whether the value holds on `today` (`YYYY-AA-GG`, docs/adr/0438 §5): its validity's first and
    /// last days included; none: every day.
    pub fn holds_on(&self, today: Option<&str>) -> bool {
        let Some(day) = today.map(|t| t.get(..10).unwrap_or(t)) else {
            return true;
        };
        self.from.as_deref().is_none_or(|f| f <= day)
            && self.until.as_deref().is_none_or(|u| day <= u)
    }

    /// Whether a new choice on `today` does not offer it: retired, or outside its validity.
    pub fn withdrawn_on(&self, today: Option<&str>) -> bool {
        self.retired || !self.holds_on(today)
    }
}

impl LayerField {
    /// Whether `code` is a retired value of the field's list (docs/adr/0418).
    pub fn is_retired(&self, code: &str) -> bool {
        self.is_withdrawn_on(code, None)
    }

    /// Whether `code` is a value a new choice on `today` does not offer: retired, or outside its
    /// validity (docs/adr/0418, 0438 §5).
    pub fn is_withdrawn_on(&self, code: &str, today: Option<&str>) -> bool {
        self.values
            .as_ref()
            .is_some_and(|v| v.iter().any(|c| c.code == code && c.withdrawn_on(today)))
    }

    /// The list's values a choice offers to a record holding `value` (docs/adr/0418): every value
    /// that is not retired, and a retired one the record holds, its label saying so.
    pub fn offered_values(&self, value: &str) -> Vec<FieldChoice> {
        self.offered_values_on(value, None)
    }

    /// [`LayerField::offered_values`] on `today` (docs/adr/0438 §5): a value outside its validity is
    /// offered only to a record holding it, its label saying so.
    pub fn offered_values_on(&self, value: &str, today: Option<&str>) -> Vec<FieldChoice> {
        let held = self.retired_in_on(value, today);
        self.values
            .iter()
            .flatten()
            .filter(|c| !c.withdrawn_on(today) || held.contains(&c.code))
            .map(|c| FieldChoice {
                label: if c.retired {
                    format!("{} (kullanımdan kalktı)", c.label)
                } else if !c.holds_on(today) {
                    format!("{} (geçerli değil)", c.label)
                } else {
                    c.label.clone()
                },
                ..c.clone()
            })
            .collect()
    }

    /// The codes of a value (each of a multiple field's) that are retired.
    pub fn retired_in(&self, value: &str) -> Vec<String> {
        self.retired_in_on(value, None)
    }

    /// The codes of a value that a new choice on `today` does not offer (docs/adr/0438 §5).
    pub fn retired_in_on(&self, value: &str, today: Option<&str>) -> Vec<String> {
        if !self
            .values
            .as_ref()
            .is_some_and(|v| v.iter().any(|c| c.withdrawn_on(today)))
        {
            return Vec::new();
        }
        let parts: Vec<&str> = if self.multiple {
            value.split(';').map(js_trim).collect()
        } else {
            vec![value]
        };
        parts
            .into_iter()
            .filter(|p| self.is_withdrawn_on(p, today))
            .map(str::to_owned)
            .collect()
    }

    /// A value as shown with the field's unit (docs/adr/0438 §4): `12,5 m²`; a value without a unit as
    /// [`display_value`] shows it.
    pub fn shown_with_unit(&self, value: &str) -> String {
        let shown = display_value(self, value);
        match &self.unit {
            Some(u) if !shown.is_empty() => format!("{shown} {u}"),
            _ => shown,
        }
    }
}

/// A field of a layer (docs/adr/0199 §1): its name is the attribute's key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerField {
    /// The field's stable id (docs/adr/0401 §2): what a form binds to. None: its name is its id; a
    /// rename writes the old name here so a binding holds. Unique among the layer's fields' keys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<String>,
    pub name: String,
    /// The name the table and the form show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alias: Option<String>,
    pub kind: LayerFieldKind,
    /// A text's most characters (1–10 000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub length: Option<u32>,
    /// A decimal's most fraction digits (0–15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub scale: Option<u32>,
    /// A number's least value, in its canonical text; included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<String>,
    /// A number's greatest value, in its canonical text; included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<String>,
    /// The values a text or a number takes, when only some.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub values: Option<Vec<FieldChoice>>,
    /// An object may not leave it empty; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub required: bool,
    /// The value a new object takes, in its canonical text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub default: Option<String>,
    /// A text field holds several values (docs/adr/0408 §1, 0409 §1): a listed one its codes in the
    /// list's order, an unlisted one its values in their order (objects' persistent ids for a
    /// many-to-many reference), joined with `; `; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub multiple: bool,
    /// What the field's value becomes in the pieces when its object is split (Alan böl, Parçala,
    /// Kır, Parçalara ayır; docs/adr/0419). None: each piece keeps it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub split: Option<SplitPolicy>,
    /// What the field's value becomes when objects are merged into one (Alan birleştir, Birleştir,
    /// Parçaları birleştir; docs/adr/0419). None: the first object's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub merge: Option<MergePolicy>,
    /// The project's value list it links (docs/adr/0430): its `values` (or `min` and `max`) are that
    /// list's copy, written anew whenever the list changes. None: its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub domain: Option<String>,
    /// No two objects of the layer hold the same value (empty ones aside; docs/adr/0438 §3): every
    /// write is refused `not_unique` that would; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub unique: bool,
    /// `min` is not itself in the range (docs/adr/0438 §4); only with `min`; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub min_exclusive: bool,
    /// `max` is not itself in the range; only with `max`; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub max_exclusive: bool,
    /// The unit a number is in, shown after it (`m²`, `kg`; 1–16 characters; docs/adr/0438 §4); the value
    /// stays the number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
    /// Dışarı verilmez (docs/adr/0638 §5): the values stay in the drawing, as a form's `redact` keeps them (an atlas, a
    /// table's copy and file, a statistic and a chart, the recorder and the server's copies leave them out), with no
    /// form needed; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub withheld: bool,
}

/// A field's value in the pieces of a split object (docs/adr/0419).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SplitPolicy {
    /// Each piece keeps the value.
    Copy,
    /// The pieces' values are emptied.
    Clear,
    /// The pieces take the field's default (none: empty).
    Default,
    /// The value is shared by the pieces' areas (lines: lengths), the total kept to the field's last
    /// digit: integers and decimals with fraction digits.
    Ratio,
}

impl SplitPolicy {
    pub const ALL: [SplitPolicy; 4] = [
        SplitPolicy::Copy,
        SplitPolicy::Clear,
        SplitPolicy::Default,
        SplitPolicy::Ratio,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SplitPolicy::Copy => "Kopyala",
            SplitPolicy::Clear => "Boşalt",
            SplitPolicy::Default => "Varsayılan",
            SplitPolicy::Ratio => "Alan oranında paylaştır",
        }
    }
}

/// A field's value in the object merged from several (docs/adr/0419).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MergePolicy {
    /// The first object's value.
    First,
    /// Empty.
    Clear,
    /// The values' sum, exact to the field's last digit: integers and decimals with fraction digits.
    Sum,
    /// The least value (numbers).
    Min,
    /// The greatest value (numbers).
    Max,
}

impl MergePolicy {
    pub const ALL: [MergePolicy; 5] = [
        MergePolicy::First,
        MergePolicy::Clear,
        MergePolicy::Sum,
        MergePolicy::Min,
        MergePolicy::Max,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MergePolicy::First => "İlkinin değeri",
            MergePolicy::Clear => "Boşalt",
            MergePolicy::Sum => "Topla",
            MergePolicy::Min => "En küçüğü",
            MergePolicy::Max => "En büyüğü",
        }
    }
}

/// What joins a multiple field's codes (docs/adr/0408 §1).
pub const MULTIPLE_JOIN: &str = "; ";

impl LayerField {
    /// A field of `kind` named `name`, with no other rule.
    pub fn new(name: impl Into<String>, kind: LayerFieldKind) -> LayerField {
        LayerField {
            withheld: false,
            id: None,
            name: name.into(),
            alias: None,
            kind,
            length: None,
            scale: None,
            min: None,
            max: None,
            values: None,
            required: false,
            default: None,
            multiple: false,
            split: None,
            merge: None,
            domain: None,
            unique: false,
            min_exclusive: false,
            max_exclusive: false,
            unit: None,
        }
    }

    /// Whether it holds several values: a multiple text field.
    pub fn holds_many(&self) -> bool {
        self.multiple && self.kind == LayerFieldKind::Text
    }

    /// A multiple field's codes in `value`, in its order (empty parts left out).
    pub fn codes_of<'a>(&self, value: &'a str) -> Vec<&'a str> {
        value
            .split(';')
            .map(js_trim)
            .filter(|c| !c.is_empty())
            .collect()
    }

    /// The field's key: its stable id, else its name (what a form binds to, docs/adr/0401 §2).
    pub fn key(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.name)
    }

    /// The name a message, the table and the form give it: its alias when
    /// it has one.
    pub fn label(&self) -> &str {
        self.alias
            .as_deref()
            .filter(|a| !a.is_empty())
            .unwrap_or(&self.name)
    }

    /// Its value list, when it has one with values.
    fn choices(&self) -> Option<&[FieldChoice]> {
        self.values.as_deref().filter(|v| !v.is_empty())
    }
}

/// Why a value does not go into a field: a short code (`type`, `magnitude`,
/// `scale`, `length`, `required`, `choice`, `range`) and the sentence that
/// says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
}

fn refusal(code: &'static str, message: String) -> Refusal {
    Refusal { code, message }
}

/// JavaScript's white space (`String.prototype.trim`).
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000d}'
            | ' '
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

/// `text.trim()`.
pub fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

/// A letter folded the Turkish way (docs/adr/0178 §2): I is ı's, İ is i's,
/// any other its lowercase when that is one character.
fn fold_char(c: char) -> char {
    match c {
        'I' => 'ı',
        'İ' => 'i',
        _ => {
            let mut low = c.to_lowercase();
            match (low.next(), low.next()) {
                (Some(l), None) => l,
                _ => c,
            }
        }
    }
}

/// `text` folded the Turkish way, letter by letter.
pub fn fold(text: &str) -> String {
    text.chars().map(fold_char).collect()
}

// ── Kinds ──────────────────────────────────────────────────────────────

/// An optional sign and what follows it: whether it was a minus.
fn signed(s: &str) -> (bool, &str) {
    match s.as_bytes().first() {
        Some(b'+') => (false, &s[1..]),
        Some(b'-') => (true, &s[1..]),
        _ => (false, s),
    }
}

fn digits(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_digit())
}

/// A whole number's canonical text, or `Err` when it is past [`MAX_INTEGER`];
/// none when `s` is no whole number.
fn integer(s: &str) -> Option<Result<String, ()>> {
    let (minus, rest) = signed(s);
    if rest.is_empty() || !digits(rest) {
        return None;
    }
    let rest = rest.trim_start_matches('0');
    if rest.is_empty() {
        return Some(Ok("0".to_owned()));
    }
    if rest.len() > 16 || rest.parse::<u64>().map_or(true, |n| n > MAX_INTEGER) {
        return Some(Err(()));
    }
    Some(Ok(if minus {
        format!("-{rest}")
    } else {
        rest.to_owned()
    }))
}

/// A decimal number as read: its canonical text, how many digits it was
/// written with and how many of them after the separator.
struct Decimal {
    text: String,
    digits: usize,
    fraction: usize,
}

fn decimal(s: &str) -> Option<Decimal> {
    let (minus, rest) = signed(s);
    let (whole, fraction) = match rest.find(['.', ',']) {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => (rest, ""),
    };
    if !digits(whole) || !digits(fraction) || (whole.is_empty() && fraction.is_empty()) {
        return None;
    }
    let count = whole.len() + fraction.len();
    let whole = match whole.trim_start_matches('0') {
        "" => "0",
        w => w,
    };
    let zero = whole == "0" && fraction.bytes().all(|b| b == b'0');
    let mut text = String::with_capacity(count + 2);
    if minus && !zero {
        text.push('-');
    }
    text.push_str(whole);
    if !fraction.is_empty() {
        text.push('.');
        text.push_str(fraction);
    }
    Some(Decimal {
        text,
        digits: count,
        fraction: fraction.len(),
    })
}

fn leap(y: u32) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

fn days_in(y: u32, m: u32) -> u32 {
    match m {
        2 if leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A date's canonical text (YYYY-AA-GG) from YYYY-AA-GG or GG.AA.YYYY (day
/// and month one or two digits): a day of the calendar, year 1–9999.
fn date(s: &str) -> Option<String> {
    let number = |t: &str, lo: usize, hi: usize| {
        (lo..=hi)
            .contains(&t.len())
            .then_some(t)
            .filter(|t| digits(t))
            .and_then(|t| t.parse::<u32>().ok())
    };
    let (y, m, d) = if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        (
            number(&s[0..4], 4, 4)?,
            number(&s[5..7], 2, 2)?,
            number(&s[8..10], 2, 2)?,
        )
    } else {
        let mut parts = s.split('.');
        let (d, m, y) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        (number(y, 4, 4)?, number(m, 1, 2)?, number(d, 1, 2)?)
    };
    ((1..=9999).contains(&y) && (1..=12).contains(&m) && (1..=days_in(y, m)).contains(&d))
        .then(|| format!("{y:04}-{m:02}-{d:02}"))
}

/// A time of day's canonical text (SS:DD:SS) from SS:DD or SS:DD:SS (the hour one or two digits,
/// 0–23; minutes and seconds two, 0–59).
fn time_of_day(s: &str) -> Option<String> {
    let mut parts = s.split(':');
    let (h, m) = (parts.next()?, parts.next()?);
    let sec = parts.next();
    if parts.next().is_some() {
        return None;
    }
    let number = |t: &str, lo: usize, hi: usize, most: u32| {
        (lo..=hi)
            .contains(&t.len())
            .then_some(t)
            .filter(|t| digits(t))
            .and_then(|t| t.parse::<u32>().ok())
            .filter(|v| *v <= most)
    };
    let (h, m) = (number(h, 1, 2, 23)?, number(m, 2, 2, 59)?);
    let sec = match sec {
        Some(t) => number(t, 2, 2, 59)?,
        None => 0,
    };
    Some(format!("{h:02}:{m:02}:{sec:02}"))
}

/// A date and time's canonical text (YYYY-AA-GGTSS:DD:SS): a date as `date` reads it, then a `T`
/// or a space and a time of day as `time_of_day` reads it; a date alone is its midnight.
fn date_time(s: &str) -> Option<String> {
    let (day, clock) = match s.find(['T', ' ']) {
        Some(i) => (&s[..i], Some(js_trim(&s[i + 1..]))),
        None => (s, None),
    };
    let day = date(day)?;
    let clock = match clock {
        Some(c) => time_of_day(c)?,
        None => "00:00:00".to_owned(),
    };
    Some(format!("{day}T{clock}"))
}

/// A 64-bit whole number's canonical text (docs/adr/0629 §2), or `Err` past the 64-bit range; none when `s` is no
/// whole number.
fn big_integer(s: &str) -> Option<Result<String, ()>> {
    let (minus, rest) = signed(s);
    if rest.is_empty() || !digits(rest) {
        return None;
    }
    let rest = rest.trim_start_matches('0');
    if rest.is_empty() {
        return Some(Ok("0".to_owned()));
    }
    let text = if minus {
        format!("-{rest}")
    } else {
        rest.to_owned()
    };
    Some(if rest.len() <= 19 && text.parse::<i64>().is_ok() {
        Ok(text)
    } else {
        Err(())
    })
}

/// An instant's canonical text (docs/adr/0629 §2): a date as `date` reads it, a `T` or a space, a time of day
/// `S:DD`, `SS:DD` or `SS:DD:SS` with at most nine fraction digits, and its offset from UTC: `Z`, `±SS:DD`, `±SSDD` or
/// `±SS`, at most 18 hours. The fraction's trailing zeros are dropped; a zero offset is `Z`.
fn instant(s: &str) -> Option<String> {
    let i = s.find(['T', 't', ' '])?;
    let day = date(&s[..i])?;
    let rest = js_trim(&s[i + 1..]);
    let (clock, minutes) = if let Some(c) = rest.strip_suffix(['Z', 'z']) {
        (js_trim(c), 0i32)
    } else {
        let at = rest.rfind(['+', '-'])?;
        let (clock, zone) = (js_trim(&rest[..at]), &rest[at..]);
        let sign = if zone.starts_with('-') { -1 } else { 1 };
        let zone = &zone[1..];
        let (h, m) = match zone.len() {
            2 => (zone, "00"),
            4 => (&zone[..2], &zone[2..]),
            5 if zone.as_bytes()[2] == b':' => (&zone[..2], &zone[3..]),
            _ => return None,
        };
        if !digits(h) || !digits(m) {
            return None;
        }
        let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
        if m > 59 || h * 60 + m > 18 * 60 {
            return None;
        }
        (clock, sign * (h * 60 + m))
    };
    let (base, fraction) = match clock.split_once('.') {
        // A fraction follows the seconds only.
        Some((b, f)) if b.matches(':').count() == 2 => (b, Some(f)),
        Some(_) => return None,
        None => (clock, None),
    };
    let base = time_of_day(base)?;
    let fraction = match fraction {
        Some(f) if (1..=9).contains(&f.len()) && digits(f) => f.trim_end_matches('0'),
        Some(_) => return None,
        None => "",
    };
    let mut out = format!("{day}T{base}");
    if !fraction.is_empty() {
        out.push('.');
        out.push_str(fraction);
    }
    if minutes == 0 {
        out.push('Z');
    } else {
        let m = minutes.abs();
        out.push_str(&format!(
            "{}{:02}:{:02}",
            if minutes < 0 { '-' } else { '+' },
            m / 60,
            m % 60
        ));
    }
    Some(out)
}

/// Days since 1970-01-01 of a day of the proleptic Gregorian calendar (Howard Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// A canonical instant's moment: its seconds since 1970-01-01T00:00:00Z and its nanoseconds; none for a text that is
/// not one.
pub fn instant_moment(v: &str) -> Option<(i64, u32)> {
    let canonical = instant(v)?;
    let b = canonical.as_bytes();
    let n = |a: usize, z: usize| canonical[a..z].parse::<i64>().ok();
    let (y, mo, d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    let (h, mi, sec) = (n(11, 13)?, n(14, 16)?, n(17, 19)?);
    let mut at = 19;
    let mut nanos = 0u32;
    if b.get(19) == Some(&b'.') {
        let end = canonical[20..]
            .find(|c: char| !c.is_ascii_digit())
            .map_or(canonical.len(), |e| 20 + e);
        let digits = &canonical[20..end];
        nanos = format!("{digits:0<9}").parse().ok()?;
        at = end;
    }
    let offset = match &canonical[at..] {
        "Z" => 0,
        z => {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            sign * (z[1..3].parse::<i64>().ok()? * 60 + z[4..6].parse::<i64>().ok()?)
        }
    };
    let seconds = days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + sec - offset * 60;
    Some((seconds, nanos))
}

/// The day of the proleptic Gregorian calendar `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// An instant written in UTC (docs/adr/0629 §4: what a file holds): `YYYY-AA-GGTSS:DD:SS[.k]Z`, the same moment; none
/// for a text that is no instant, or one whose UTC day is outside the years 1–9999.
pub fn instant_utc(v: &str) -> Option<String> {
    let (seconds, nanos) = instant_moment(v)?;
    let (y, m, d) = civil_from_days(seconds.div_euclid(86_400));
    if !(1..=9999).contains(&y) {
        return None;
    }
    let s = seconds.rem_euclid(86_400);
    let mut out = format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        s / 3600,
        s % 3600 / 60,
        s % 60
    );
    if nanos > 0 {
        let f = format!("{nanos:09}");
        out.push('.');
        out.push_str(f.trim_end_matches('0'));
    }
    out.push('Z');
    Some(out)
}

/// Two instants' order by their moments (docs/adr/0629 §2: the same moment with another offset is equal); none when
/// either is not one.
pub fn compare_instants(a: &str, b: &str) -> Option<Ordering> {
    Some(instant_moment(a)?.cmp(&instant_moment(b)?))
}

/// A UUID's canonical text (docs/adr/0629 §2): 32 hexadecimal digits with or without the hyphens of 8-4-4-4-12, in
/// braces or after `urn:uuid:`, written lowercase with the hyphens.
fn uuid(s: &str) -> Option<String> {
    let s = match s.get(..9) {
        Some(head) if s.len() > 9 && head.eq_ignore_ascii_case("urn:uuid:") => &s[9..],
        _ => s,
    };
    let s = s
        .strip_prefix('{')
        .and_then(|t| t.strip_suffix('}'))
        .unwrap_or(s);
    let hex: String = match s.len() {
        32 => s.to_owned(),
        36 => {
            let b = s.as_bytes();
            if [8, 13, 18, 23].iter().any(|&i| b[i] != b'-') {
                return None;
            }
            s.replace('-', "")
        }
        _ => return None,
    };
    if hex.len() != 32 || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let h = hex.to_ascii_lowercase();
    Some(format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    ))
}

/// Why a JSON text is refused.
enum JsonFault {
    Syntax,
    Deep,
}

/// A JSON value's canonical text (docs/adr/0629 §2, RFC 8259): its insignificant white space dropped, its numbers and
/// strings as written.
fn json(s: &str) -> Result<String, JsonFault> {
    struct Scan<'a> {
        b: &'a [u8],
        at: usize,
        out: String,
    }
    impl Scan<'_> {
        fn space(&mut self) {
            while matches!(self.b.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.at += 1;
            }
        }
        fn value(&mut self, depth: usize) -> Result<(), JsonFault> {
            self.space();
            match self.b.get(self.at) {
                Some(b'{') | Some(b'[') if depth >= MAX_JSON_DEPTH => Err(JsonFault::Deep),
                Some(b'{') => {
                    self.at += 1;
                    self.out.push('{');
                    self.space();
                    if self.b.get(self.at) == Some(&b'}') {
                        self.at += 1;
                        self.out.push('}');
                        return Ok(());
                    }
                    loop {
                        self.space();
                        self.string()?;
                        self.space();
                        if self.b.get(self.at) != Some(&b':') {
                            return Err(JsonFault::Syntax);
                        }
                        self.at += 1;
                        self.out.push(':');
                        self.value(depth + 1)?;
                        self.space();
                        match self.b.get(self.at) {
                            Some(b',') => {
                                self.at += 1;
                                self.out.push(',');
                            }
                            Some(b'}') => {
                                self.at += 1;
                                self.out.push('}');
                                return Ok(());
                            }
                            _ => return Err(JsonFault::Syntax),
                        }
                    }
                }
                Some(b'[') => {
                    self.at += 1;
                    self.out.push('[');
                    self.space();
                    if self.b.get(self.at) == Some(&b']') {
                        self.at += 1;
                        self.out.push(']');
                        return Ok(());
                    }
                    loop {
                        self.value(depth + 1)?;
                        self.space();
                        match self.b.get(self.at) {
                            Some(b',') => {
                                self.at += 1;
                                self.out.push(',');
                            }
                            Some(b']') => {
                                self.at += 1;
                                self.out.push(']');
                                return Ok(());
                            }
                            _ => return Err(JsonFault::Syntax),
                        }
                    }
                }
                Some(b'"') => self.string(),
                Some(b't') => self.word("true"),
                Some(b'f') => self.word("false"),
                Some(b'n') => self.word("null"),
                Some(b'-' | b'0'..=b'9') => self.number(),
                _ => Err(JsonFault::Syntax),
            }
        }
        fn word(&mut self, w: &str) -> Result<(), JsonFault> {
            if self.b[self.at..].starts_with(w.as_bytes()) {
                self.at += w.len();
                self.out.push_str(w);
                Ok(())
            } else {
                Err(JsonFault::Syntax)
            }
        }
        fn number(&mut self) -> Result<(), JsonFault> {
            let start = self.at;
            let digits = |s: &mut Self| {
                let from = s.at;
                while s.b.get(s.at).is_some_and(u8::is_ascii_digit) {
                    s.at += 1;
                }
                s.at > from
            };
            if self.b.get(self.at) == Some(&b'-') {
                self.at += 1;
            }
            match self.b.get(self.at) {
                Some(b'0') => self.at += 1,
                Some(b'1'..=b'9') => {
                    digits(self);
                }
                _ => return Err(JsonFault::Syntax),
            }
            if self.b.get(self.at) == Some(&b'.') {
                self.at += 1;
                if !digits(self) {
                    return Err(JsonFault::Syntax);
                }
            }
            if matches!(self.b.get(self.at), Some(b'e' | b'E')) {
                self.at += 1;
                if matches!(self.b.get(self.at), Some(b'+' | b'-')) {
                    self.at += 1;
                }
                if !digits(self) {
                    return Err(JsonFault::Syntax);
                }
            }
            self.out
                .push_str(std::str::from_utf8(&self.b[start..self.at]).map_err(|_| JsonFault::Syntax)?);
            Ok(())
        }
        fn string(&mut self) -> Result<(), JsonFault> {
            if self.b.get(self.at) != Some(&b'"') {
                return Err(JsonFault::Syntax);
            }
            let start = self.at;
            self.at += 1;
            loop {
                match self.b.get(self.at) {
                    None => return Err(JsonFault::Syntax),
                    Some(b'"') => {
                        self.at += 1;
                        break;
                    }
                    Some(b'\\') => {
                        self.at += 1;
                        match self.b.get(self.at) {
                            Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => self.at += 1,
                            Some(b'u') => {
                                let hex = self.b.get(self.at + 1..self.at + 5).ok_or(JsonFault::Syntax)?;
                                if !hex.iter().all(u8::is_ascii_hexdigit) {
                                    return Err(JsonFault::Syntax);
                                }
                                self.at += 5;
                            }
                            _ => return Err(JsonFault::Syntax),
                        }
                    }
                    Some(c) if *c < 0x20 => return Err(JsonFault::Syntax),
                    Some(_) => self.at += 1,
                }
            }
            self.out
                .push_str(std::str::from_utf8(&self.b[start..self.at]).map_err(|_| JsonFault::Syntax)?);
            Ok(())
        }
    }
    let mut scan = Scan {
        b: s.as_bytes(),
        at: 0,
        out: String::with_capacity(s.len()),
    };
    scan.value(0)?;
    scan.space();
    if scan.at != scan.b.len() {
        return Err(JsonFault::Syntax);
    }
    Ok(scan.out)
}

/// A binary value's bytes from base64 (RFC 4648, the standard alphabet; the padding may be left out, white space is
/// skipped); none when it is not base64.
pub fn binary_bytes(s: &str) -> Option<Vec<u8>> {
    let t: String = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if t.is_empty() {
        return None;
    }
    let padded = match t.len() % 4 {
        0 => t,
        2 => format!("{t}=="),
        3 => format!("{t}="),
        _ => return None,
    };
    crate::encoding::base64_decode(&padded)
}

fn boolean(s: &str) -> Option<&'static str> {
    match fold(s).as_str() {
        "evet" | "true" | "1" => Some("true"),
        "hayır" | "false" | "0" => Some("false"),
        _ => None,
    }
}

/// Two decimal numbers' order by their exact values; none when either is
/// not one.
pub fn compare_decimals(a: &str, b: &str) -> Option<Ordering> {
    // (sign, whole without leading zeros, fraction without trailing zeros)
    fn parts(s: &str) -> Option<(i8, String, String)> {
        let d = decimal(s)?;
        let (minus, rest) = signed(&d.text);
        let (whole, fraction) = rest.split_once('.').unwrap_or((rest, ""));
        let fraction = fraction.trim_end_matches('0');
        let sign = if whole == "0" && fraction.is_empty() {
            0
        } else if minus {
            -1
        } else {
            1
        };
        Some((sign, whole.to_owned(), fraction.to_owned()))
    }
    let (sa, wa, fa) = parts(a)?;
    let (sb, wb, fb) = parts(b)?;
    let magnitude = wa
        .len()
        .cmp(&wb.len())
        .then_with(|| wa.cmp(&wb))
        .then_with(|| fa.cmp(&fb));
    Some(sa.cmp(&sb).then(if sa < 0 {
        magnitude.reverse()
    } else {
        magnitude
    }))
}

/// The kind's canonical text of trimmed `s` (`text` as given, for the
/// messages).
fn kind_value(field: &LayerField, s: &str, text: &str) -> Result<String, Refusal> {
    let label = field.label();
    match field.kind {
        LayerFieldKind::Integer => match integer(s) {
            None => Err(refusal(
                "type",
                format!("“{label}” alanı tam sayı ister; “{text}” verildi. Rakamlarla, ondalıksız yazın."),
            )),
            Some(Err(())) => Err(refusal(
                "magnitude",
                format!(
                    "“{label}” alanının sayısı çok büyük; “{text}” verildi. Mutlak değeri en çok {MAX_INTEGER} olabilir."
                ),
            )),
            Some(Ok(v)) => Ok(v),
        },
        LayerFieldKind::Decimal => {
            let Some(d) = decimal(s) else {
                return Err(refusal(
                    "type",
                    format!(
                        "“{label}” alanı ondalık sayı ister; “{text}” verildi. Rakamlarla, ondalığı nokta ya da virgülle yazın."
                    ),
                ));
            };
            if d.digits > MAX_DIGITS {
                return Err(refusal(
                    "magnitude",
                    format!(
                        "“{label}” alanının sayısı çok uzun; “{text}” verildi. En çok {MAX_DIGITS} rakam olabilir."
                    ),
                ));
            }
            let mut value = d.text;
            if let Some(scale) = field.scale
                && d.fraction > scale as usize
            {
                let cut = d.fraction - scale as usize;
                if value[value.len() - cut..].bytes().any(|b| b != b'0') {
                    return Err(refusal(
                        "scale",
                        format!(
                            "“{label}” alanı en çok {scale} ondalık basamak alır; “{text}” verildi. Değer yuvarlanmaz; basamakları azaltarak yazın."
                        ),
                    ));
                }
                // Zeros past the scale say nothing (a DBF's N(24,15), `12.500`): the value is cut to the scale, not
                // rounded (docs/adr/0628 §5).
                value.truncate(value.len() - cut);
                if value.ends_with('.') {
                    value.pop();
                }
            }
            Ok(value)
        }
        LayerFieldKind::Date => date(s).ok_or_else(|| {
            refusal(
                "type",
                format!("“{label}” alanı tarih ister; “{text}” verildi. GG.AA.YYYY ya da YYYY-AA-GG yazın."),
            )
        }),
        LayerFieldKind::DateTime => date_time(s).ok_or_else(|| {
            refusal(
                "type",
                format!(
                    "“{label}” alanı tarih ve saat ister; “{text}” verildi. GG.AA.YYYY SS:DD ya da YYYY-AA-GG SS:DD yazın."
                ),
            )
        }),
        LayerFieldKind::Time => time_of_day(s).ok_or_else(|| {
            refusal(
                "type",
                format!("“{label}” alanı saat ister; “{text}” verildi. SS:DD ya da SS:DD:SS yazın (00:00–23:59)."),
            )
        }),
        LayerFieldKind::Boolean => boolean(s).map(str::to_owned).ok_or_else(|| {
            refusal(
                "type",
                format!("“{label}” alanı evet ya da hayır ister; “{text}” verildi."),
            )
        }),
        LayerFieldKind::Text => Ok(text.to_owned()),
        LayerFieldKind::BigInt => match big_integer(s) {
            None => Err(refusal(
                "type",
                format!("“{label}” alanı tam sayı ister; “{text}” verildi. Rakamlarla, ondalıksız yazın."),
            )),
            Some(Err(())) => Err(refusal(
                "magnitude",
                format!(
                    "“{label}” alanının sayısı çok büyük; “{text}” verildi. {} ile {} arasında olabilir.",
                    i64::MIN,
                    i64::MAX
                ),
            )),
            Some(Ok(v)) => Ok(v),
        },
        LayerFieldKind::Instant => instant(s).ok_or_else(|| {
            refusal(
                "type",
                format!(
                    "“{label}” alanı saat dilimli tarih ve saat ister; “{text}” verildi. YYYY-AA-GG SS:DD:SS+03:00 ya da …Z yazın."
                ),
            )
        }),
        LayerFieldKind::Uuid => uuid(s).ok_or_else(|| {
            refusal(
                "type",
                format!(
                    "“{label}” alanı UUID ister; “{text}” verildi. 32 onaltılık rakamı 8-4-4-4-12 düzeninde yazın."
                ),
            )
        }),
        LayerFieldKind::Json => {
            if s.chars().count() > MAX_JSON {
                return Err(refusal(
                    "magnitude",
                    format!("“{label}” alanının JSON değeri çok uzun; en çok {MAX_JSON} karakter olabilir."),
                ));
            }
            json(s).map_err(|fault| match fault {
                JsonFault::Syntax => refusal(
                    "type",
                    format!("“{label}” alanı geçerli bir JSON değeri ister; “{text}” JSON değil."),
                ),
                JsonFault::Deep => refusal(
                    "magnitude",
                    format!("“{label}” alanının JSON değeri çok derin; en çok {MAX_JSON_DEPTH} düzey iç içe olabilir."),
                ),
            })
        }
        LayerFieldKind::Binary => {
            let Some(bytes) = binary_bytes(s) else {
                return Err(refusal(
                    "type",
                    format!("“{label}” alanı ikili değeri base64 olarak ister; “{text}” base64 değil."),
                ));
            };
            if bytes.len() > MAX_BINARY {
                return Err(refusal(
                    "magnitude",
                    format!(
                        "“{label}” alanının ikili değeri çok büyük: {} bayt. En çok {MAX_BINARY} bayt (1 MiB) olabilir.",
                        bytes.len()
                    ),
                ));
            }
            Ok(crate::encoding::base64_encode(&bytes))
        }
    }
}

/// A value written to `field` (docs/adr/0199 §1): its canonical text, or why
/// it does not go in. Empty (blank) is `""` unless the field is required; a
/// value list's label (folded, trimmed) gives its code; a range is exact,
/// its ends included.
pub fn check_value(field: &LayerField, text: &str) -> Result<String, Refusal> {
    let label = field.label();
    let s = js_trim(text);
    if s.is_empty() {
        if field.required {
            return Err(refusal(
                "required",
                format!("“{label}” alanı zorunlu; boş bırakılamaz."),
            ));
        }
        return Ok(String::new());
    }
    if field.kind == LayerFieldKind::Text
        && let Some(length) = field.length
    {
        let n = text.chars().count();
        if n > length as usize {
            return Err(refusal(
                "length",
                format!("“{label}” alanı en çok {length} karakter alır; {n} karakter verildi."),
            ));
        }
    }
    let choices = field.choices();
    // A multiple field without a list: its values in their order once each (docs/adr/0409 §1).
    if field.holds_many() && choices.is_none() {
        let mut seen: Vec<&str> = Vec::new();
        for part in field.codes_of(s) {
            if !seen.contains(&part) {
                seen.push(part);
            }
        }
        if seen.is_empty() && field.required {
            return Err(refusal(
                "required",
                format!("“{label}” alanı zorunlu; boş bırakılamaz."),
            ));
        }
        return Ok(seen.join(MULTIPLE_JOIN));
    }
    // A multiple field: each part a code or a label, written in the list's order once each.
    if field.holds_many()
        && let Some(choices) = choices
    {
        let mut picked = vec![false; choices.len()];
        for part in field.codes_of(s) {
            let wanted = fold(part);
            let at = choices
                .iter()
                .position(|c| fold(js_trim(&c.label)) == wanted)
                .or_else(|| choices.iter().position(|c| c.code == part));
            match at {
                Some(i) => picked[i] = true,
                None => {
                    return Err(refusal(
                        "choice",
                        format!("“{label}” alanı listedeki değerleri ister; “{part}” listede yok."),
                    ));
                }
            }
        }
        let codes: Vec<&str> = choices
            .iter()
            .zip(&picked)
            .filter(|(_, p)| **p)
            .map(|(c, _)| c.code.as_str())
            .collect();
        if codes.is_empty() && field.required {
            return Err(refusal(
                "required",
                format!("“{label}” alanı zorunlu; boş bırakılamaz."),
            ));
        }
        return Ok(codes.join(MULTIPLE_JOIN));
    }
    if let Some(choices) = choices {
        let wanted = fold(s);
        if let Some(c) = choices.iter().find(|c| fold(js_trim(&c.label)) == wanted) {
            return Ok(c.code.clone());
        }
    }
    let canonical = kind_value(field, s, text)?;
    // A text field's code is matched as trimmed; without a list the text stays as given.
    let value = if field.kind == LayerFieldKind::Text && choices.is_some() {
        s.to_owned()
    } else {
        canonical
    };
    if let Some(choices) = choices
        && !choices.iter().any(|c| c.code == value)
    {
        return Err(refusal(
            "choice",
            format!("“{label}” alanı listedeki değerlerden birini ister; “{text}” listede yok."),
        ));
    }
    if field.kind.is_number() || field.kind.is_temporal() {
        // An excluded end is outside too (docs/adr/0438 §4).
        let below =
            field
                .min
                .as_deref()
                .is_some_and(|lo| match range_order(field.kind, &value, lo) {
                    Some(Ordering::Less) => true,
                    Some(Ordering::Equal) => field.min_exclusive,
                    _ => false,
                });
        let above =
            field
                .max
                .as_deref()
                .is_some_and(|hi| match range_order(field.kind, &value, hi) {
                    Some(Ordering::Greater) => true,
                    Some(Ordering::Equal) => field.max_exclusive,
                    _ => false,
                });
        if below || above {
            // A date said as the interface shows it (07.10.2026; docs/adr/0436), a number as it is,
            // with its unit (docs/adr/0438 §4).
            let shown = |v: &str| field.shown_with_unit(v);
            let value = shown(&value);
            let message = match (&field.min, &field.max) {
                (Some(lo), Some(hi)) => {
                    let left_out = match (field.min_exclusive, field.max_exclusive) {
                        (true, true) => " (iki uç da hariç)".to_owned(),
                        (true, false) => format!(" ({} hariç)", shown(lo)),
                        (false, true) => format!(" ({} hariç)", shown(hi)),
                        (false, false) => String::new(),
                    };
                    format!(
                        "“{label}” alanı {} ile {} arasında olmalı{left_out}; {value} verildi.",
                        shown(lo),
                        shown(hi)
                    )
                }
                (Some(lo), None) if field.min_exclusive => format!(
                    "“{label}” alanı {} değerinden büyük olmalı; {value} verildi.",
                    shown(lo)
                ),
                (Some(lo), None) => format!(
                    "“{label}” alanı en az {} olmalı; {value} verildi.",
                    shown(lo)
                ),
                (_, hi) if field.max_exclusive => format!(
                    "“{label}” alanı {} değerinden küçük olmalı; {value} verildi.",
                    shown(hi.as_deref().unwrap_or_default())
                ),
                (_, hi) => format!(
                    "“{label}” alanı en çok {} olmalı; {value} verildi.",
                    shown(hi.as_deref().unwrap_or_default())
                ),
            };
            return Err(refusal("range", message));
        }
    }
    Ok(value)
}

/// Two canonical values' order in a range: numbers by their exact values, a date, a date and time or
/// a time of day by its text (docs/adr/0436: `YYYY-AA-GG`, `…TSS:DD:SS`, `SS:DD:SS` sort as written).
fn range_order(kind: LayerFieldKind, a: &str, b: &str) -> Option<Ordering> {
    if kind == LayerFieldKind::Instant {
        // The same moment with another offset is the same (docs/adr/0629 §2).
        compare_instants(a, b)
    } else if kind.is_temporal() {
        Some(a.cmp(b))
    } else {
        compare_decimals(a, b)
    }
}

/// Whether `v` is already the canonical text of `field`'s kind (its scale
/// counted): a range's end, a number's code.
fn canonical_of(field: &LayerField, v: &str) -> bool {
    let s = js_trim(v);
    !s.is_empty() && kind_value(field, s, v).is_ok_and(|c| c == v)
}

// ── A field list's problems ────────────────────────────────────────────

/// What is wrong with a field, when anything is (docs/adr/0199 §1), in the
/// words the Alanlar window, the project file and the server say it.
pub fn field_problem(field: &LayerField) -> Option<String> {
    let name = &field.name;
    if let Some(id) = &field.id {
        let bad = js_trim(id).is_empty()
            || js_trim(id) != id
            || id.chars().count() > MAX_NAME
            || id.chars().any(char::is_control);
        if bad {
            return Some(format!(
                "“{name}” alanının kimliği boş olamaz, en çok {MAX_NAME} karakterdir, boşlukla başlayıp bitmez ve denetim karakteri içermez."
            ));
        }
    }
    if js_trim(name).is_empty() {
        return Some("Alanın adı boş olamaz.".to_owned());
    }
    if let Some(d) = &field.domain
        && (d.is_empty() || d.chars().count() > MAX_NAME)
    {
        return Some(format!(
            "“{name}” alanının bağlı olduğu değer listesinin kimliği boş olamaz ve en çok {MAX_NAME} karakterdir."
        ));
    }
    if js_trim(name) != name {
        return Some(format!(
            "“{name}” alanının adının başında ya da sonunda boşluk var."
        ));
    }
    if name.chars().count() > MAX_NAME {
        return Some(format!(
            "“{name}” alanının adı en çok {MAX_NAME} karakter olabilir."
        ));
    }
    if name
        .chars()
        .any(|c| (c as u32) < 32 || (127..160).contains(&(c as u32)))
    {
        return Some(format!("“{name}” alanının adında denetim karakteri var."));
    }
    if let Some(alias) = &field.alias {
        if js_trim(alias).is_empty() {
            return Some(format!("“{name}” alanının takma adı boş olamaz."));
        }
        if alias.chars().count() > MAX_NAME {
            return Some(format!(
                "“{name}” alanının takma adı en çok {MAX_NAME} karakter olabilir."
            ));
        }
    }
    let kind = field.kind;
    if let Some(length) = field.length {
        if kind != LayerFieldKind::Text {
            return Some(format!(
                "“{name}” alanında uzunluk yalnız metin alanında olur."
            ));
        }
        if !(1..=MAX_LENGTH).contains(&length) {
            return Some(format!(
                "“{name}” alanının uzunluğu 1 ile {MAX_LENGTH} arasında olmalı."
            ));
        }
    }
    if let Some(scale) = field.scale {
        if kind != LayerFieldKind::Decimal {
            return Some(format!(
                "“{name}” alanında ondalık basamak yalnız ondalık sayı alanında olur."
            ));
        }
        if scale > MAX_SCALE {
            return Some(format!(
                "“{name}” alanının ondalık basamağı 0 ile {MAX_SCALE} arasında olmalı."
            ));
        }
    }
    if field.min.is_some() || field.max.is_some() {
        if !kind.is_number() && !kind.is_temporal() {
            return Some(format!(
                "“{name}” alanında aralık yalnız sayı, tarih ve saat alanlarında olur."
            ));
        }
        for (end, word) in [(&field.min, "en azı"), (&field.max, "en çoğu")] {
            if let Some(v) = end
                && !canonical_of(field, v)
            {
                return Some(format!(
                    "“{name}” alanının {word} “{v}” alanın türüne uymuyor."
                ));
            }
        }
        if let (Some(lo), Some(hi)) = (&field.min, &field.max)
            && range_order(kind, lo, hi) == Some(Ordering::Greater)
        {
            return Some(format!(
                "“{name}” alanının en azı en çoğundan büyük olamaz."
            ));
        }
    }
    if let Some(values) = &field.values {
        if !matches!(
            kind,
            LayerFieldKind::Text
                | LayerFieldKind::Integer
                | LayerFieldKind::Decimal
                | LayerFieldKind::BigInt
        ) {
            return Some(format!(
                "“{name}” alanında değer listesi yalnız metin ve sayı alanlarında olur."
            ));
        }
        if values.is_empty() {
            return Some(format!(
                "“{name}” alanının değer listesi boş; listeyi kaldırın ya da değer ekleyin."
            ));
        }
        for (i, v) in values.iter().enumerate() {
            let (code, label) = (&v.code, &v.label);
            if code.is_empty() {
                return Some(format!("“{name}” alanının değer listesinde boş kod var."));
            }
            if kind != LayerFieldKind::Text && !canonical_of(field, code) {
                return Some(format!(
                    "“{name}” alanının değer listesindeki “{code}” kodu alanın türüne uymuyor."
                ));
            }
            if js_trim(label).is_empty() {
                return Some(format!(
                    "“{name}” alanının değer listesinde “{code}” kodunun etiketi boş."
                ));
            }
            if values[..i].iter().any(|w| w.code == *code) {
                return Some(format!(
                    "“{name}” alanının değer listesinde “{code}” kodu iki kez var."
                ));
            }
            let key = fold(js_trim(label));
            if values[..i].iter().any(|w| fold(js_trim(&w.label)) == key) {
                return Some(format!(
                    "“{name}” alanının değer listesinde “{label}” etiketi iki kez var."
                ));
            }
            if field.multiple && (code.contains(';') || label.contains(';')) {
                return Some(format!(
                    "“{name}” çok değerli; değer listesinin kodlarında ve etiketlerinde “;” olamaz."
                ));
            }
        }
    }
    // Sharing and summing keep the total to the last digit: whole units only (docs/adr/0419).
    let units = kind == LayerFieldKind::Integer
        || (kind == LayerFieldKind::Decimal && field.scale.is_some());
    if field.split == Some(SplitPolicy::Ratio) && !units {
        return Some(format!(
            "“{name}” alanı oranla paylaştırılamaz: bu tam sayı ya da ondalık basamağı verilmiş ondalık sayı alanında olur."
        ));
    }
    if field.merge == Some(MergePolicy::Sum) && !units {
        return Some(format!(
            "“{name}” alanı toplanamaz: bu tam sayı ya da ondalık basamağı verilmiş ondalık sayı alanında olur."
        ));
    }
    if matches!(field.merge, Some(MergePolicy::Min | MergePolicy::Max))
        && !matches!(kind, LayerFieldKind::Integer | LayerFieldKind::Decimal)
    {
        return Some(format!(
            "“{name}” alanında en küçüğü ve en büyüğü yalnız sayı alanlarında olur."
        ));
    }
    if field.min_exclusive && field.min.is_none() {
        return Some(format!(
            "“{name}” alanının alt ucu en azı olmadan hariç tutulamaz."
        ));
    }
    if field.max_exclusive && field.max.is_none() {
        return Some(format!(
            "“{name}” alanının üst ucu en çoğu olmadan hariç tutulamaz."
        ));
    }
    if let (Some(lo), Some(hi)) = (&field.min, &field.max)
        && (field.min_exclusive || field.max_exclusive)
        && range_order(kind, lo, hi) == Some(Ordering::Equal)
    {
        return Some(format!(
            "“{name}” alanının aralığı boş: en azı en çoğuna eşitken uçlar hariç tutulamaz."
        ));
    }
    if let Some(unit) = &field.unit {
        if !kind.is_number() {
            return Some(format!(
                "“{name}” alanında birim yalnız sayı alanlarında olur."
            ));
        }
        if js_trim(unit).is_empty()
            || js_trim(unit) != unit
            || unit.chars().count() > MAX_UNIT
            || unit.chars().any(char::is_control)
        {
            return Some(format!(
                "“{name}” alanının birimi boş olamaz, en çok {MAX_UNIT} karakterdir, boşlukla başlayıp bitmez."
            ));
        }
    }
    if field.unique && kind == LayerFieldKind::Boolean {
        return Some(format!(
            "“{name}” evet/hayır alanı; tekil olamaz (iki değeri var)."
        ));
    }
    if field.unique && field.multiple {
        return Some(format!("“{name}” çok değerli; tekil olamaz."));
    }
    for v in field.values.iter().flatten() {
        let code = &v.code;
        if let Some(d) = &v.description
            && (js_trim(d).is_empty() || d.chars().count() > MAX_DESCRIPTION)
        {
            return Some(format!(
                "“{name}” alanının “{code}” değerinin açıklaması boş olamaz ve en çok {MAX_DESCRIPTION} karakterdir."
            ));
        }
        for (end, word) in [(&v.from, "başlangıcı"), (&v.until, "bitişi")] {
            if let Some(day) = end
                && date(day).as_deref() != Some(day.as_str())
            {
                return Some(format!(
                    "“{name}” alanının “{code}” değerinin geçerlilik {word} “{day}” bir tarih değil (YYYY-AA-GG)."
                ));
            }
        }
        if let (Some(f), Some(u)) = (&v.from, &v.until)
            && f > u
        {
            return Some(format!(
                "“{name}” alanının “{code}” değerinin geçerliliği bitişinden sonra başlıyor."
            ));
        }
    }
    if field
        .values
        .as_ref()
        .is_some_and(|v| !v.is_empty() && v.iter().all(|c| c.retired))
    {
        return Some(format!(
            "“{name}” alanının bütün değerleri kullanımdan kalkmış; en az biri sunulmalı."
        ));
    }
    if field.multiple && kind != LayerFieldKind::Text {
        return Some(format!(
            "“{name}” alanı çok değerli olamaz: bu yalnız metin alanında olur."
        ));
    }
    if let Some(d) = &field.default {
        let optional = LayerField {
            required: false,
            ..field.clone()
        };
        let fits = !js_trim(d).is_empty() && check_value(&optional, d).is_ok_and(|v| v == *d);
        if !fits {
            return Some(format!(
                "“{name}” alanının varsayılanı “{d}” alanın kurallarına uymuyor."
            ));
        }
        if let Some(r) = field.retired_in(d).first() {
            return Some(format!(
                "“{name}” alanının varsayılanı “{r}” kullanımdan kalkmış bir değer."
            ));
        }
    }
    None
}

/// The first problem of a layer's fields: one field's, or a name used
/// twice (folded the Turkish way).
pub fn fields_problem(fields: &[LayerField]) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    let mut keys = std::collections::HashSet::new();
    for f in fields {
        if let Some(p) = field_problem(f) {
            return Some(p);
        }
        if !keys.insert(f.key()) {
            return Some(format!(
                "“{}” kimliği iki alanda var; bir alanın kimliği ya da adı başka alanın kimliği olamaz.",
                f.key()
            ));
        }
        if !seen.insert(fold(&f.name)) {
            return Some(format!(
                "“{}” adlı iki alan var; alan adları bir kez kullanılır.",
                f.name
            ));
        }
    }
    None
}

/// The problem of a layer's stored fields: [`fields_problem`]'s, or an
/// empty list (a layer without fields does not write one).
pub fn layer_fields_problem(fields: &[LayerField]) -> Option<String> {
    if fields.is_empty() {
        return Some("katmanın alan listesi boş olamaz; alanı yoksa yazılmaz".to_owned());
    }
    fields_problem(fields)
}

// ── Inferred fields and display ────────────────────────────────────────

/// The field Verilerden al gives a key whose values are `values` (docs/adr/0199
/// §3): the first kind every value that is not blank takes, whole numbers,
/// decimals (with as many fraction digits as the longest has, at most
/// [`MAX_SCALE`]), dates and yes or no words in that order, else text.
pub fn infer_field<'a>(name: &str, values: impl IntoIterator<Item = &'a str>) -> LayerField {
    let values: Vec<&str> = values
        .into_iter()
        .map(js_trim)
        .filter(|v| !v.is_empty())
        .collect();
    let field = |kind| LayerField::new(name, kind);
    if values.is_empty() {
        return field(LayerFieldKind::Text);
    }
    if values.iter().all(|v| matches!(integer(v), Some(Ok(_)))) {
        return field(LayerFieldKind::Integer);
    }
    // Whole numbers past 2⁵³, within 64 bits (docs/adr/0629 §2).
    if values.iter().all(|v| matches!(big_integer(v), Some(Ok(_)))) {
        return field(LayerFieldKind::BigInt);
    }
    let decimals: Option<Vec<Decimal>> = values
        .iter()
        .map(|v| decimal(v).filter(|d| d.digits <= MAX_DIGITS))
        .collect();
    if let Some(decimals) = decimals {
        let scale = decimals.iter().map(|d| d.fraction).max().unwrap_or(0);
        if scale > MAX_SCALE as usize {
            return field(LayerFieldKind::Text);
        }
        return LayerField {
            scale: Some(scale as u32),
            ..field(LayerFieldKind::Decimal)
        };
    }
    if values.iter().all(|v| date(v).is_some()) {
        return field(LayerFieldKind::Date);
    }
    // A date with a time of day, and a time of day alone (docs/adr/0412 §1).
    if values
        .iter()
        .all(|v| v.contains(':') && date_time(v).is_some())
    {
        return field(LayerFieldKind::DateTime);
    }
    if values.iter().all(|v| instant(v).is_some()) {
        return field(LayerFieldKind::Instant);
    }
    if values.iter().all(|v| time_of_day(v).is_some()) {
        return field(LayerFieldKind::Time);
    }
    if values
        .iter()
        .all(|v| matches!(fold(v).as_str(), "evet" | "hayır" | "true" | "false"))
    {
        return field(LayerFieldKind::Boolean);
    }
    // Hyphenated UUIDs only: 32 bare hexadecimal digits may be a hash (docs/adr/0629 §2).
    if values.iter().all(|v| {
        let b = v.as_bytes();
        b.len() == 36
            && b.iter().enumerate().all(|(i, c)| {
                if [8, 13, 18, 23].contains(&i) {
                    *c == b'-'
                } else {
                    c.is_ascii_hexdigit()
                }
            })
    }) {
        return field(LayerFieldKind::Uuid);
    }
    field(LayerFieldKind::Text)
}

/// The fields Verilerden al gives objects' attributes (each object its
/// key–value pairs): one per key, in the order `order` puts the keys (the
/// natural order, docs/adr/0153 §6, which the geometry core has).
pub fn infer_fields<'a>(
    rows: impl IntoIterator<Item = &'a [(String, String)]>,
    order: impl Fn(&str, &str) -> Ordering,
) -> Vec<LayerField> {
    let mut keys: Vec<(&str, Vec<&str>)> = Vec::new();
    for row in rows {
        for (k, v) in row {
            match keys.iter_mut().find(|(name, _)| *name == k.as_str()) {
                Some((_, values)) => values.push(v),
                None => keys.push((k, vec![v])),
            }
        }
    }
    keys.sort_by(|a, b| order(a.0, b.0));
    keys.into_iter()
        .map(|(name, values)| infer_field(name, values))
        .collect()
}

/// How a value of `field` is shown (docs/adr/0199 §1): a code its label, a
/// yes or no Evet or Hayır, a date GG.AA.YYYY, anything else as it is.
pub fn display_value(field: &LayerField, value: &str) -> String {
    if field.holds_many() {
        let labels: Vec<String> = field
            .codes_of(value)
            .into_iter()
            .map(|code| {
                field
                    .values
                    .as_deref()
                    .and_then(|v| v.iter().find(|c| c.code == code))
                    .map_or_else(|| code.to_owned(), |c| c.label.clone())
            })
            .collect();
        return labels.join(MULTIPLE_JOIN);
    }
    if let Some(c) = field
        .values
        .as_deref()
        .and_then(|v| v.iter().find(|c| c.code == value))
    {
        return c.label.clone();
    }
    match field.kind {
        LayerFieldKind::Boolean => match value {
            "true" => "Evet".to_owned(),
            "false" => "Hayır".to_owned(),
            _ => value.to_owned(),
        },
        LayerFieldKind::Date
            if value.len() == 10
                && value.as_bytes()[4] == b'-'
                && value.as_bytes()[7] == b'-'
                && digits(&value[0..4])
                && digits(&value[5..7])
                && digits(&value[8..10]) =>
        {
            format!("{}.{}.{}", &value[8..10], &value[5..7], &value[0..4])
        }
        // GG.AA.YYYY SS:DD, the seconds when there are some (docs/adr/0412 §1).
        LayerFieldKind::DateTime if date_time(value).as_deref() == Some(value) => {
            let clock = &value[11..];
            let clock = clock.strip_suffix(":00").unwrap_or(clock);
            format!(
                "{}.{}.{} {clock}",
                &value[8..10],
                &value[5..7],
                &value[0..4]
            )
        }
        LayerFieldKind::Time if time_of_day(value).as_deref() == Some(value) => {
            value.strip_suffix(":00").unwrap_or(value).to_owned()
        }
        // GG.AA.YYYY SS:DD:SS[.k] and its offset, UTC said (docs/adr/0629 §3).
        LayerFieldKind::Instant if instant(value).as_deref() == Some(value) => {
            let rest = &value[11..];
            let at = rest.find(['Z', '+', '-']).unwrap_or(rest.len());
            let zone = match &rest[at..] {
                "Z" => "UTC",
                z => z,
            };
            format!(
                "{}.{}.{} {} {zone}",
                &value[8..10],
                &value[5..7],
                &value[0..4],
                &rest[..at]
            )
        }
        // The bytes' count, not the base64 (docs/adr/0629 §3).
        LayerFieldKind::Binary => match binary_bytes(value) {
            Some(b) => format!("ikili, {} bayt", b.len()),
            None => value.to_owned(),
        },
        _ => value.to_owned(),
    }
}

// A place that names a layer's field (`cad.fields.usage`, docs/adr/0414 §3): `kentos_forms::field_refs` finds them.
/// How a place names a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FieldUseHow {
    /// By its key (its stable id, else its name): a form item, a list, a relation.
    Key,
    /// By its name: an attribute's name (the time, the card's fields, a table rule's field).
    Name,
    /// In an expression (İfadeyle seç's language).
    Expression,
    /// In a template's `{…}` expressions.
    Template,
}

/// One place that names a field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldUse {
    /// The place in words: “Bina denetimi” formu › “Not” öğesi › Görünme koşulu.
    pub place: String,
    /// Where it is in the drawing: `settings/forms/0/draft/items/2/visibleWhen`, `layers/<id>/…`.
    pub path: String,
    pub how: FieldUseHow,
    /// A place a rename does not change: a published release of a form, a library symbol shared by layers
    /// (docs/adr/0635 §2). Only `true` is written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub kept: bool,
}

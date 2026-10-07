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
}

impl LayerFieldKind {
    /// Every kind, in the order the Alanlar window lists them.
    pub const ALL: [LayerFieldKind; 5] = [
        LayerFieldKind::Text,
        LayerFieldKind::Integer,
        LayerFieldKind::Decimal,
        LayerFieldKind::Date,
        LayerFieldKind::Boolean,
    ];

    /// Its name in the contract.
    pub fn name(self) -> &'static str {
        match self {
            LayerFieldKind::Text => "text",
            LayerFieldKind::Integer => "integer",
            LayerFieldKind::Decimal => "decimal",
            LayerFieldKind::Date => "date",
            LayerFieldKind::Boolean => "boolean",
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
        }
    }

    /// Whether its values are numbers (a range and a value list take them).
    pub fn is_number(self) -> bool {
        matches!(self, LayerFieldKind::Integer | LayerFieldKind::Decimal)
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
}

/// A field of a layer (docs/adr/0199 §1): its name is the attribute's key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerField {
    pub name: String,
    /// The name the table and the form show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub alias: Option<String>,
    pub kind: LayerFieldKind,
    /// A text's most characters (1–10 000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub length: Option<u32>,
    /// A decimal's most fraction digits (0–15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
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
}

impl LayerField {
    /// A field of `kind` named `name`, with no other rule.
    pub fn new(name: impl Into<String>, kind: LayerFieldKind) -> LayerField {
        LayerField {
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
        }
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
    Some(Ok(if minus { format!("-{rest}") } else { rest.to_owned() }))
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
            if let Some(scale) = field.scale
                && d.fraction > scale as usize
            {
                return Err(refusal(
                    "scale",
                    format!(
                        "“{label}” alanı en çok {scale} ondalık basamak alır; “{text}” verildi. Değer yuvarlanmaz; basamakları azaltarak yazın."
                    ),
                ));
            }
            Ok(d.text)
        }
        LayerFieldKind::Date => date(s).ok_or_else(|| {
            refusal(
                "type",
                format!("“{label}” alanı tarih ister; “{text}” verildi. GG.AA.YYYY ya da YYYY-AA-GG yazın."),
            )
        }),
        LayerFieldKind::Boolean => boolean(s).map(str::to_owned).ok_or_else(|| {
            refusal(
                "type",
                format!("“{label}” alanı evet ya da hayır ister; “{text}” verildi."),
            )
        }),
        LayerFieldKind::Text => Ok(text.to_owned()),
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
    if field.kind.is_number() {
        let below = field
            .min
            .as_deref()
            .is_some_and(|lo| compare_decimals(&value, lo) == Some(Ordering::Less));
        let above = field
            .max
            .as_deref()
            .is_some_and(|hi| compare_decimals(&value, hi) == Some(Ordering::Greater));
        if below || above {
            let message = match (&field.min, &field.max) {
                (Some(lo), Some(hi)) => {
                    format!("“{label}” alanı {lo} ile {hi} arasında olmalı; {value} verildi.")
                }
                (Some(lo), None) => format!("“{label}” alanı en az {lo} olmalı; {value} verildi."),
                (_, hi) => format!(
                    "“{label}” alanı en çok {} olmalı; {value} verildi.",
                    hi.as_deref().unwrap_or_default()
                ),
            };
            return Err(refusal("range", message));
        }
    }
    Ok(value)
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
    if js_trim(name).is_empty() {
        return Some("Alanın adı boş olamaz.".to_owned());
    }
    if js_trim(name) != name {
        return Some(format!("“{name}” alanının adının başında ya da sonunda boşluk var."));
    }
    if name.chars().count() > MAX_NAME {
        return Some(format!("“{name}” alanının adı en çok {MAX_NAME} karakter olabilir."));
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
            return Some(format!("“{name}” alanında uzunluk yalnız metin alanında olur."));
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
        if !kind.is_number() {
            return Some(format!("“{name}” alanında aralık yalnız sayı alanlarında olur."));
        }
        for (end, word) in [(&field.min, "en azı"), (&field.max, "en çoğu")] {
            if let Some(v) = end
                && !canonical_of(field, v)
            {
                return Some(format!("“{name}” alanının {word} “{v}” alanın türüne uymuyor."));
            }
        }
        if let (Some(lo), Some(hi)) = (&field.min, &field.max)
            && compare_decimals(lo, hi) == Some(Ordering::Greater)
        {
            return Some(format!("“{name}” alanının en azı en çoğundan büyük olamaz."));
        }
    }
    if let Some(values) = &field.values {
        if !matches!(
            kind,
            LayerFieldKind::Text | LayerFieldKind::Integer | LayerFieldKind::Decimal
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
        }
    }
    if let Some(d) = &field.default {
        let optional = LayerField {
            required: false,
            ..field.clone()
        };
        let fits = !js_trim(d).is_empty() && check_value(&optional, d).is_ok_and(|v| v == *d);
        if !fits {
            return Some(format!("“{name}” alanının varsayılanı “{d}” alanın kurallarına uymuyor."));
        }
    }
    None
}

/// The first problem of a layer's fields: one field's, or a name used
/// twice (folded the Turkish way).
pub fn fields_problem(fields: &[LayerField]) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    for f in fields {
        if let Some(p) = field_problem(f) {
            return Some(p);
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
    if values
        .iter()
        .all(|v| matches!(fold(v).as_str(), "evet" | "hayır" | "true" | "false"))
    {
        return field(LayerFieldKind::Boolean);
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
        _ => value.to_owned(),
    }
}

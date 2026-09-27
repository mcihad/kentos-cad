//! Classes for the layer style window (the web's `style/classify.ts`, QGIS
//! “Classify”): the distinct values of an expression over a layer's
//! objects, numeric classes by equal interval or equal count, colour ramps,
//! and the plain symbols a new class starts with. The window only shows and
//! edits them. Both platforms are held to `fixtures/style/v1/classify.json`;
//! where the TypeScript leans on JavaScript (`Math.round`, `Number(text)`,
//! `localeCompare('tr', { numeric: true })`) this does the same.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;

use kentos_contracts::Entity;
use kentos_style_core::expr::rows::{As, MEASURE_STRIDE};
use kentos_style_core::expr::{Expr, Measured, Scope, Value, compile};
use kentos_style_core::js::collate::compare_tr;
use kentos_style_core::js::number;
use serde_json::json;

use crate::renderer::{Category, GeometryClass, GraduatedClass, SymbolSet};
use crate::table::{kind_label, vertex_count};

// ── What the expression gives ──────────────────────────────────────────

/// One object as an expression reads it.
struct Row<'a> {
    e: &'a Entity,
    fields: &'a [String],
    layer: &'a str,
    index: usize,
    measures: Option<&'a [f64]>,
}

impl Scope for Row<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        let name = self.fields.get(i)?;
        self.e.base().attrs.get(name).map(String::as_str)
    }

    fn measured(&self) -> Measured {
        let k = self.index * MEASURE_STRIDE;
        let Some(m) = self.measures.and_then(|m| m.get(k..k + MEASURE_STRIDE)) else {
            return Measured::default();
        };
        let flags = m[0] as u32;
        Measured {
            length: (flags & 1 != 0).then_some(m[1]),
            area: (flags & 2 != 0).then_some(m[2]),
            anchor: (flags & 4 != 0).then_some((m[3], m[4])),
        }
    }

    fn vertices(&self) -> Option<f64> {
        vertex_count(self.e)
    }

    fn kind(&self) -> &str {
        kind_label(self.e)
    }

    fn layer(&self) -> &str {
        self.layer
    }

    fn label(&self) -> Option<&str> {
        self.e.base().label.as_deref()
    }

    fn index(&self) -> f64 {
        (self.index + 1) as f64
    }

    fn id(&self) -> f64 {
        f64::from(self.e.base().id)
    }

    fn scale(&self) -> Option<f64> {
        None
    }
}

/// Where the objects' layer names and geometry values come from: the
/// drawing's geometry store answers `measures` (six numbers per object),
/// asked once, only when the expression reads `$alan`, `$uzunluk`, `$y` or `$x`.
pub struct ExprScope<'a> {
    pub layer_name: &'a dyn Fn(&str) -> String,
    pub measures: &'a dyn Fn(&[&Entity]) -> Vec<f64>,
}

/// The expression's value for every object, as `want` asks.
pub fn evaluate(expr: &Expr, list: &[&Entity], scope: &ExprScope, want: As) -> Vec<Value<'static>> {
    let measured = expr.needs.measured.then(|| (scope.measures)(list));
    let mut names: HashMap<&str, String> = HashMap::new();
    list.iter()
        .enumerate()
        .map(|(index, e)| {
            let layer = if expr.needs.layer {
                let id = e.base().layer_id.as_str();
                names
                    .entry(id)
                    .or_insert_with(|| (scope.layer_name)(id))
                    .clone()
            } else {
                String::new()
            };
            let row = Row {
                e,
                fields: &expr.fields,
                layer: &layer,
                index,
                measures: measured.as_deref(),
            };
            want.convert(expr.evaluate(&row)).into_owned()
        })
        .collect()
}

/// The values of an expression, or what is wrong with it: the message, and
/// the 1-based position it is about (the dialog shows both).
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluated<T> {
    pub values: Vec<T>,
    pub error: Option<String>,
    pub at: usize,
}

impl<T> Evaluated<T> {
    fn failed(message: String, at: usize) -> Self {
        Evaluated {
            values: Vec::new(),
            error: Some(message),
            at,
        }
    }

    /// The error as the window shows it: “12. karakterde: …” when it is not about the start.
    pub fn error_text(&self) -> Option<String> {
        self.error.as_ref().map(|e| {
            if self.at > 1 {
                format!("{}. karakterde: {e}", self.at)
            } else {
                e.clone()
            }
        })
    }
}

/// The expression's text per object (None when it gives nothing) (`valuesOf`).
pub fn values_of(list: &[&Entity], expr: &str, scope: &ExprScope) -> Evaluated<Option<String>> {
    match compile(expr) {
        Err(e) => Evaluated::failed(e.message, e.at),
        Ok(c) => Evaluated {
            values: evaluate(&c, list, scope, As::Text)
                .into_iter()
                .map(|v| match v {
                    Value::Text(t) => Some(t.into_owned()),
                    _ => None,
                })
                .collect(),
            error: None,
            at: 0,
        },
    }
}

/// The numbers the expression's values read as, text that reads as a number counting (`numbersOf`).
pub fn numbers_of(list: &[&Entity], expr: &str, scope: &ExprScope) -> Evaluated<f64> {
    match compile(expr) {
        Err(e) => Evaluated::failed(e.message, e.at),
        Ok(c) => Evaluated {
            values: evaluate(&c, list, scope, As::TextNumber)
                .into_iter()
                .filter_map(|v| match v {
                    Value::Num(x) if x.is_finite() => Some(x),
                    _ => None,
                })
                .collect(),
            error: None,
            at: 0,
        },
    }
}

/// The number each object's value reads as (None: none), in the objects' order:
/// what a graduated renderer draws each object by.
pub fn numbers_per_object(
    list: &[&Entity],
    expr: &str,
    scope: &ExprScope,
) -> Evaluated<Option<f64>> {
    match compile(expr) {
        Err(e) => Evaluated::failed(e.message, e.at),
        Ok(c) => Evaluated {
            values: evaluate(&c, list, scope, As::TextNumber)
                .into_iter()
                .map(|v| match v {
                    Value::Num(x) if x.is_finite() => Some(x),
                    _ => None,
                })
                .collect(),
            error: None,
            at: 0,
        },
    }
}

// ── Values and numeric classes ─────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueCount {
    pub value: String,
    pub count: usize,
}

/// Text whose Turkish order is `localeCompare`'s with `numeric: true`: each
/// run of digits, leading zeros dropped, becomes its length (a prefix code:
/// as many 9s as whole nines, then the rest) and its digits.
fn numeric_key(s: &str) -> Cow<'_, str> {
    if !s.bytes().any(|b| b.is_ascii_digit()) {
        return Cow::Borrowed(s);
    }
    fn flush(run: &mut String, out: &mut String) {
        if run.is_empty() {
            return;
        }
        let digits = run.trim_start_matches('0');
        let digits = if digits.is_empty() { "0" } else { digits };
        let mut n = digits.len();
        while n >= 9 {
            out.push('9');
            n -= 9;
        }
        out.push(char::from(b'0' + n as u8));
        out.push_str(digits);
        run.clear();
    }
    let mut out = String::with_capacity(s.len() + 4);
    let mut run = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    Cow::Owned(out)
}

/// `a.localeCompare(b, 'tr')`: Turkish order (field names).
pub use kentos_style_core::js::collate::compare_tr as compare_text;

/// `a.localeCompare(b, 'tr', { numeric: true })`: Turkish order, numbers by their value.
pub fn compare_values(a: &str, b: &str) -> Ordering {
    compare_tr(&numeric_key(a), &numeric_key(b))
}

/// Distinct non-empty values with their counts, ordered naturally (numbers
/// by value, text in Turkish order; equal ones as they came) (`uniqueValues`).
pub fn unique_values(values: &[Option<String>]) -> Vec<ValueCount> {
    let mut at: HashMap<&str, usize> = HashMap::new();
    let mut out: Vec<ValueCount> = Vec::new();
    for v in values.iter().flatten().filter(|v| !v.is_empty()) {
        match at.get(v.as_str()) {
            Some(&i) => out[i].count += 1,
            None => {
                at.insert(v, out.len());
                out.push(ValueCount {
                    value: v.clone(),
                    count: 1,
                });
            }
        }
    }
    let keys: Vec<String> = out
        .iter()
        .map(|v| numeric_key(&v.value).into_owned())
        .collect();
    let mut order: Vec<usize> = (0..out.len()).collect();
    order.sort_by(|&a, &b| compare_tr(&keys[a], &keys[b]));
    order.into_iter().map(|i| out[i].clone()).collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericClass {
    pub min: f64,
    pub max: f64,
}

/// The smallest and largest of `values` (none when empty).
fn span(values: &[f64]) -> Option<(f64, f64)> {
    values.iter().fold(None, |s, &v| match s {
        None => Some((v, v)),
        Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
    })
}

/// `n` classes of equal width between the smallest and largest value (`equalInterval`).
pub fn equal_interval(values: &[f64], n: usize) -> Vec<NumericClass> {
    let Some((lo, hi)) = span(values) else {
        return Vec::new();
    };
    if n < 1 {
        return Vec::new();
    }
    if hi == lo {
        return vec![NumericClass { min: lo, max: hi }];
    }
    let step = (hi - lo) / n as f64;
    (0..n)
        .map(|i| NumericClass {
            min: lo + i as f64 * step,
            max: if i == n - 1 {
                hi
            } else {
                lo + (i + 1) as f64 * step
            },
        })
        .collect()
}

/// `n` classes with about the same number of objects each (quantiles); equal bounds merge (`equalCount`).
pub fn equal_count(values: &[f64], n: usize) -> Vec<NumericClass> {
    if values.is_empty() || n < 1 {
        return Vec::new();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let last = sorted.len() - 1;
    let mut bounds = vec![sorted[0]];
    for i in 1..n {
        let k = js_round((i * sorted.len()) as f64 / n as f64);
        bounds.push(sorted[(k as usize).min(last)]);
    }
    bounds.push(sorted[last]);
    let mut out = Vec::new();
    for i in 0..bounds.len() - 1 {
        if bounds[i + 1] > bounds[i] || (i == bounds.len() - 2 && out.is_empty()) {
            out.push(NumericClass {
                min: bounds[i],
                max: bounds[i + 1],
            });
        }
    }
    out
}

/// Objects a class takes: from its lower bound (included) to its upper bound
/// (excluded; the last class includes it) (`countIn`).
pub fn count_in(numbers: &[f64], min: f64, max: f64, last: bool) -> usize {
    numbers
        .iter()
        .filter(|&&v| v >= min && (v < max || (last && v <= max)))
        .count()
}

// ── Colours ────────────────────────────────────────────────────────────

/// Well-separated colours for categories (QGIS-like “rastgele” but repeatable).
pub const QUALITATIVE: [&str; 12] = [
    "#E15759", "#4E79A7", "#F28E2B", "#76B7B2", "#59A14F", "#EDC948", "#B07AA1", "#FF9DA7",
    "#9C755F", "#BAB0AC", "#8CD17D", "#86BCB6",
];

/// A colour ramp of the graduated renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ramp {
    pub key: &'static str,
    pub label: &'static str,
    pub stops: &'static [&'static str],
}

pub const RAMPS: [Ramp; 5] = [
    Ramp {
        key: "sariKirmizi",
        label: "Sarıdan kırmızıya",
        stops: &["#FFF5B8", "#FDB863", "#E66101", "#A50F15"],
    },
    Ramp {
        key: "maviler",
        label: "Maviler",
        stops: &["#E3EEF9", "#9ECAE1", "#4292C6", "#08306B"],
    },
    Ramp {
        key: "yesiller",
        label: "Yeşiller",
        stops: &["#E5F5E0", "#A1D99B", "#41AB5D", "#005A32"],
    },
    Ramp {
        key: "griler",
        label: "Griler",
        stops: &["#F0F0F0", "#BDBDBD", "#737373", "#252525"],
    },
    Ramp {
        key: "spektral",
        label: "Spektral",
        stops: &["#2B83BA", "#ABDDA4", "#FFFFBF", "#FDAE61", "#D7191C"],
    },
];

/// The ramp of a key; the default for one this version does not know.
pub fn ramp(key: &str) -> &'static Ramp {
    RAMPS.iter().find(|r| r.key == key).unwrap_or(&RAMPS[0])
}

fn hex(c: &str) -> [f64; 3] {
    let channel = |i: usize| {
        c.get(i..i + 2)
            .and_then(|h| u8::from_str_radix(h, 16).ok())
            .map_or(f64::NAN, f64::from)
    };
    [channel(1), channel(3), channel(5)]
}

fn to_hex(rgb: [f64; 3]) -> String {
    let mut out = String::from("#");
    for v in rgb {
        out.push_str(&format!("{:02X}", js_round(v) as i64));
    }
    out
}

/// `n` colours along a ramp, ends included (`rampColors`).
pub fn ramp_colors(stops: &[&str], n: usize) -> Vec<String> {
    let Some(last) = stops.last() else {
        return Vec::new();
    };
    if n <= 1 || stops.len() < 2 {
        return vec![(*last).to_owned()];
    }
    (0..n)
        .map(|i| {
            let t = (i as f64 / (n - 1) as f64) * (stops.len() - 1) as f64;
            let k = (stops.len() - 2).min(t.floor() as usize);
            let (a, b) = (hex(stops[k]), hex(stops[k + 1]));
            let f = t - k as f64;
            to_hex([0, 1, 2].map(|j| a[j] + (b[j] - a[j]) * f))
        })
        .collect()
}

// ── Symbols for new classes ────────────────────────────────────────────

/// How many objects of each geometry class a layer has (text and dimensions have none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Present {
    pub fill: usize,
    pub line: usize,
    pub marker: usize,
}

impl Present {
    pub fn of(&self, class: GeometryClass) -> usize {
        match class {
            GeometryClass::Fill => self.fill,
            GeometryClass::Line => self.line,
            GeometryClass::Marker => self.marker,
        }
    }

    /// The classes the layer has; all three when it has none (the window's slots).
    pub fn classes(&self) -> Vec<GeometryClass> {
        let some: Vec<GeometryClass> = GeometryClass::ALL
            .into_iter()
            .filter(|c| self.of(*c) > 0)
            .collect();
        if some.is_empty() {
            GeometryClass::ALL.to_vec()
        } else {
            some
        }
    }
}

/// An object's geometry class: areas are polygons and hatches, circles and
/// ellipses stay curves; text and dimensions are drawn elsewhere (`geometryClassOf`).
pub fn geometry_class(e: &Entity) -> Option<GeometryClass> {
    match e {
        Entity::Point(_) => Some(GeometryClass::Marker),
        Entity::Polygon(_) | Entity::Hatch(_) => Some(GeometryClass::Fill),
        Entity::Text(_) | Entity::Dimension(_) => None,
        _ => Some(GeometryClass::Line),
    }
}

/// `classesPresent`.
pub fn classes_present(list: &[&Entity]) -> Present {
    let mut out = Present::default();
    for e in list {
        match geometry_class(e) {
            Some(GeometryClass::Fill) => out.fill += 1,
            Some(GeometryClass::Line) => out.line += 1,
            Some(GeometryClass::Marker) => out.marker += 1,
            None => {}
        }
    }
    out
}

/// A plain symbol of one colour for each geometry class the layer has
/// (areas get a thin ink edge); all three when it has none (`plainSymbols`).
pub fn plain_symbols(color: &str, present: &Present) -> SymbolSet {
    let any = present.fill + present.line + present.marker > 0;
    let mut out = SymbolSet::default();
    if present.fill > 0 || !any {
        out.fill = Some(json!({ "type": "fill", "layers": [
            { "id": "0", "type": "simpleFill", "color": color },
            { "id": "1", "type": "simpleLine", "color": "ink", "width": 0.18 },
        ] }));
    }
    if present.line > 0 || !any {
        out.line = Some(json!({ "type": "line", "layers": [
            { "id": "0", "type": "simpleLine", "color": color, "width": 0.35 },
        ] }));
    }
    if present.marker > 0 || !any {
        out.marker = Some(json!({ "type": "marker", "layers": [
            { "id": "0", "type": "shape", "shape": "circle", "size": 2.4, "fill": color, "stroke": "ink", "strokeWidth": 0.18 },
        ] }));
    }
    out
}

// ── JavaScript's arithmetic where the TypeScript leaned on it ─────────

/// `Math.round`: the nearest integer, halves toward +∞, the sign of zero kept.
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    if r == 0.0 && x.is_sign_negative() {
        -0.0
    } else {
        r
    }
}

/// `Number(text)`: blanks around ignored, empty text 0, decimal, `0x`/`0o`/`0b`
/// integers and `Infinity`; anything else NaN.
pub fn js_number(text: &str) -> f64 {
    let t = kentos_style_core::js::text::trim(text);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (prefixes, radix) in [(["0x", "0X"], 16), (["0o", "0O"], 8), (["0b", "0B"], 2)] {
        if let Some(rest) = prefixes.iter().find_map(|p| t.strip_prefix(p)) {
            if rest.is_empty() {
                return f64::NAN;
            }
            return rest
                .chars()
                .try_fold(0.0, |v, c| {
                    c.to_digit(radix)
                        .map(|d| v * f64::from(radix) + f64::from(d))
                })
                .unwrap_or(f64::NAN);
        }
    }
    // A decimal literal: [+-] digits [. digits] or . digits, then [e[+-]digits].
    let b = t.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let digits = |i: &mut usize| {
        let s = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - s
    };
    let whole = digits(&mut i);
    let mut fraction = 0;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        fraction = digits(&mut i);
    }
    if whole + fraction == 0 {
        return f64::NAN;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return f64::NAN;
        }
    }
    if i != b.len() {
        return f64::NAN;
    }
    t.parse().unwrap_or(f64::NAN)
}

/// Label for a numeric class: “12.5 – 30”, each bound to `digits` decimals (`classLabel`).
pub fn class_label(c: NumericClass, digits: u32) -> String {
    format!("{} – {}", rounded(c.min, digits), rounded(c.max, digits))
}

/// A number to `digits` decimals as a label writes it (`Math.round`, then its shortest text).
pub fn rounded(v: f64, digits: u32) -> String {
    let p = 10f64.powi(digits as i32);
    number::to_string(js_round(v * p) / p)
}

// ── What the layer style window makes of them ─────────────────────────

/// Colour of the “Diğer değerler” set when it is switched on.
pub const OTHER_COLOR: &str = "#BAB0AC";

/// Classes “Sınıfla” makes unless told otherwise, and the range the field keeps to.
pub const CLASS_COUNT_DEFAULT: usize = 5;
pub const CLASS_COUNT_MIN: usize = 1;
pub const CLASS_COUNT_MAX: usize = 20;

/// The ramp a graduated style starts with.
pub const DEFAULT_RAMP: &str = "sariKirmizi";

/// The class count typed in the field (`classCount`): an empty field or
/// text that is not a number is the default; any number is rounded and kept
/// from 1 to 20 (0 and -0.4 are 1 class, as a negative one is).
pub fn class_count(typed: &str) -> usize {
    // JavaScript's trim: its white space, line terminators and the byte order mark.
    let blank = typed
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
        .is_empty();
    let r = js_round(js_number(typed));
    if blank || r.is_nan() {
        return CLASS_COUNT_DEFAULT;
    }
    r.max(CLASS_COUNT_MIN as f64).min(CLASS_COUNT_MAX as f64) as usize
}

/// What the classify controls say (`CLASSIFY_TEXTS`).
pub mod texts {
    pub const NO_VALUES: &str = "Bu ifade nesnelerde değer vermiyor.";
    pub fn found(n: usize) -> String {
        format!("{n} değer bulundu.")
    }
    pub const NO_NUMBERS: &str = "Bu ifade nesnelerde sayı vermiyor.";
    pub fn classified(classes: usize, values: usize) -> String {
        format!("{classes} sınıf, {values} sayısal değerden.")
    }
    pub const NEW_CATEGORY: &str = "Yeni kategori";
    pub const OTHER: &str = "Diğer değerler";
    pub const CATEGORIES_HELP: &str =
        "Bir alan adı yazıp “Değerlerden sınıfla”ya basın: her farklı değer bir kategori olur.";
    pub const CLASSES_HELP: &str =
        "Bir değer alt sınıra eşitse o sınıfa girer; son sınıf üst sınırını da içerir.";
    pub const CLASSES_EMPTY_HELP: &str = "Sayı veren bir ifade yazıp “Sınıfla”ya basın.";
}

/// The categories “Değerlerden sınıfla” makes of the distinct values: one
/// each, labelled with the value, in QUALITATIVE's colours in turn; a value
/// that already has a category keeps it (its label and symbols) (`categoriesOf`).
pub fn categories_of(found: &[ValueCount], present: &Present, old: &[Category]) -> Vec<Category> {
    found
        .iter()
        .enumerate()
        .map(|(i, v)| {
            old.iter()
                .find(|k| k.value == v.value)
                .cloned()
                .unwrap_or_else(|| Category {
                    value: v.value.clone(),
                    label: v.value.clone(),
                    symbols: plain_symbols(QUALITATIVE[i % QUALITATIVE.len()], present),
                    enabled: None,
                    extra: Default::default(),
                })
        })
        .collect()
}

/// The category “Kategori ekle” adds after `count` others (`newCategory`).
pub fn new_category(count: usize, present: &Present) -> Category {
    Category {
        value: String::new(),
        label: texts::NEW_CATEGORY.to_owned(),
        symbols: plain_symbols(QUALITATIVE[count % QUALITATIVE.len()], present),
        enabled: None,
        extra: Default::default(),
    }
}

/// How many objects each category takes, and how many are left for “Diğer değerler” (`categoryCounts`).
pub fn category_counts(values: &[Option<String>], categories: &[Category]) -> (Vec<usize>, i64) {
    let counts: HashMap<String, usize> = unique_values(values)
        .into_iter()
        .map(|v| (v.value, v.count))
        .collect();
    let each: Vec<usize> = categories
        .iter()
        .map(|k| counts.get(&k.value).copied().unwrap_or(0))
        .collect();
    let rest = values.len() as i64 - each.iter().sum::<usize>() as i64;
    (each, rest)
}

/// How “Sınıfla” splits the numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Interval,
    Count,
}

/// The classes “Sınıfla” makes: `n` by equal interval or equal count,
/// coloured along the ramp, labelled by their bounds (`graduatedOf`).
pub fn graduated_of(
    numbers: &[f64],
    method: Method,
    n: usize,
    ramp_key: &str,
    present: &Present,
) -> Vec<GraduatedClass> {
    let classes = match method {
        Method::Interval => equal_interval(numbers, n),
        Method::Count => equal_count(numbers, n),
    };
    let colors = ramp_colors(ramp(ramp_key).stops, classes.len());
    classes
        .iter()
        .zip(colors)
        .map(|(c, color)| GraduatedClass {
            min: c.min,
            max: c.max,
            label: class_label(*c, 2),
            symbols: plain_symbols(&color, present),
            extra: Default::default(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn javascript_rounding_and_numbers() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert!(js_round(-0.4).is_sign_negative());
        assert_eq!(js_number(" 7 "), 7.0);
        assert_eq!(js_number(""), 0.0);
        assert_eq!(js_number("0x1F"), 31.0);
        assert_eq!(js_number("1e3"), 1000.0);
        assert_eq!(js_number(".5"), 0.5);
        assert_eq!(js_number("5."), 5.0);
        assert!(js_number("inf").is_nan());
        assert!(js_number("1,5").is_nan());
        assert!(js_number("-0x10").is_nan());
        assert_eq!(js_number("-Infinity"), f64::NEG_INFINITY);
        assert_eq!(class_count("Infinity"), 20);
        // The web's 4df48f3: any number is kept from 1 to 20; only blank or not a number is 5.
        assert_eq!(class_count("-0.4"), 1);
        assert_eq!(class_count("0"), 1);
        assert_eq!(class_count(" "), 5);
        assert_eq!(class_count("beş"), 5);
    }

    #[test]
    fn numbers_order_by_value_among_turkish_text() {
        let order = |a: &str, b: &str| compare_values(a, b);
        assert_eq!(order("2", "12"), Ordering::Less);
        assert_eq!(order("Ada 9", "Ada 10"), Ordering::Less);
        assert_eq!(order("007", "7"), Ordering::Equal);
        assert_eq!(order("123456789", "99999999"), Ordering::Greater);
        assert_eq!(order("1234567890123", "999999999999"), Ordering::Greater);
        assert_eq!(order("x", "12"), Ordering::Greater);
        assert_eq!(order("a2", "A1"), Ordering::Greater);
    }
}

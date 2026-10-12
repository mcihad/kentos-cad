//! A chart's data (docs/adr/0227 §2): rows and columns as the platforms read
//! them from a layer's objects (their fields' canonical values, ADR 0199 §1)
//! or from an İşlemler table. A cell is empty, a value (its canonical text
//! and what it shows) or one its column's kind refuses (as written).
//!
//! The core reads each column once ([`Read`]): numbers as exact decimals
//! (`kentos.statistics/1`), dates as days, yes or no, texts with the spaces
//! round them dropped (one empty after that is empty). Equal values meet by
//! their value ([`Key`]): `1.50` and `1.5` are one number. The order of the
//! values ([`Read::cmp`]) is the kind's: numbers by value, dates by the
//! calendar, Hayır before Evet, texts in the natural order (a field's value
//! list by what it shows); refused cells after the values in the natural
//! order of their text; the empty last.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// A map keyed by the core's quick hash (`store::IdHasher`): a column of
/// 100 000 rows looks every one up, where the standard hash's protection
/// against chosen keys only costs time.
pub(crate) type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<crate::store::IdHasher>>;

use crate::ops::statistics::{Dec, compare, read_number};
use crate::text::natural::natural_cmp;
use crate::time::days_from_civil;

/// The empty values' group, row and category.
pub const EMPTY: &str = "(boş)";
/// The refused values' row in a field's value list.
pub const INVALID: &str = "(uymayan)";

/// A column's kind: Sayı (whole and decimal numbers), Metin, Tarih, Evet/hayır.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Number,
    Text,
    Date,
    Boolean,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Number, Kind::Text, Kind::Date, Kind::Boolean];

    /// Its word across the boundary: `number`, `text`, `date`, `boolean`.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Number => "number",
            Kind::Text => "text",
            Kind::Date => "date",
            Kind::Boolean => "boolean",
        }
    }

    pub fn from_word(word: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.word() == word)
    }

    /// Its name in the statistics table.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Number => "Sayı",
            Kind::Text => "Metin",
            Kind::Date => "Tarih",
            Kind::Boolean => "Evet/hayır",
        }
    }
}

/// A cell as a platform read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    /// The canonical value and what it shows when that is not the value
    /// itself (a value list's label, Evet, GG.AA.YYYY).
    Value(String, Option<String>),
    /// A value the column's kind refuses, as written.
    Invalid(String),
}

/// A column of the data: its key (a field's name, a table's header), its
/// name as shown, its kind, whether its values sort by what they show (a
/// field's value list) and its cells, one per row.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub key: String,
    pub label: String,
    pub kind: Kind,
    pub labelled: bool,
    pub cells: Vec<Cell>,
}

impl Column {
    /// A geometry measure's column (Alan (m²), Uzunluk (m); docs/adr/0227 §2):
    /// each value the double's text at 3 fraction digits (its exact binary
    /// value, half to even); NaN is empty.
    pub fn of_measures(key: &str, label: &str, values: &[f64]) -> Column {
        Column {
            key: key.to_owned(),
            label: label.to_owned(),
            kind: Kind::Number,
            labelled: false,
            cells: values
                .iter()
                .map(|&v| {
                    if v.is_finite() {
                        Cell::Value(crate::ops::statistics::fixed(v, 3), None)
                    } else {
                        Cell::Empty
                    }
                })
                .collect(),
        }
    }
}

/// The geometry measures' keys and names (docs/adr/0227 §2).
pub const AREA_KEY: &str = "$alan";
pub const AREA_LABEL: &str = "Alan (m²)";
pub const LENGTH_KEY: &str = "$uzunluk";
pub const LENGTH_LABEL: &str = "Uzunluk (m)";

/// Alan (m²) and Uzunluk (m) from the geometry store's measure records
/// (`store::draw::measure_record`, one a row): an area where the object has
/// one, a length (an area's perimeter) where it has one, else empty. A
/// column no object has a value of is left out (a layer of lines has no
/// area, one of points neither).
pub fn geometry_columns(records: &[f64]) -> Vec<Column> {
    use crate::store::draw::{HAS_AREA, HAS_LENGTH, MEASURE_STRIDE};
    let mut area = Vec::new();
    let mut length = Vec::new();
    for r in records.chunks_exact(MEASURE_STRIDE) {
        let flags = r[0] as u32;
        area.push(if flags & HAS_AREA != 0 {
            r[2]
        } else {
            f64::NAN
        });
        length.push(if flags & HAS_LENGTH != 0 {
            r[1]
        } else {
            f64::NAN
        });
    }
    let some = |v: &[f64]| v.iter().any(|x| x.is_finite());
    let mut out = Vec::new();
    if some(&area) {
        out.push(Column::of_measures(AREA_KEY, AREA_LABEL, &area));
    }
    if some(&length) {
        out.push(Column::of_measures(LENGTH_KEY, LENGTH_LABEL, &length));
    }
    out
}

/// A cell read: what the statistics and the charts count with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Val {
    Empty,
    /// A refused value: its text's index in [`Read::texts`].
    Invalid(u32),
    Num(Dec),
    /// A date as days since 1970-01-01.
    Date(i64),
    Bool(bool),
    /// A text: its index in [`Read::texts`].
    Text(u32),
}

/// Equal values meet by their key: a number by its value (no trailing zeros
/// in its scale), a date by its day, a text by its interned index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Empty,
    Invalid(u32),
    Num(i128, u32),
    Date(i64),
    Bool(bool),
    Text(u32),
}

/// A column read once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Read {
    pub vals: Vec<Val>,
    /// Interned texts: the text values (trimmed) and the refused ones.
    pub texts: Vec<String>,
    /// What a key shows: the first shown text met for it (a date's
    /// GG.AA.YYYY, Evet, a label); a number without one its first value's
    /// canonical text (`first`), a text without one itself.
    pub(crate) shown: FastMap<Key, String>,
    /// Each number key's first value as written (1.50 shows as 1.50 where it came first).
    pub(crate) first: FastMap<Key, Dec>,
    /// The column's kind and whether it sorts by what it shows.
    pub kind: Option<Kind>,
    pub labelled: bool,
}

/// JavaScript's trim set (ADR 0199 §1).
pub fn js_trim(text: &str) -> &str {
    text.trim_matches(|c: char| {
        matches!(
            c,
            '\u{9}'
                | '\u{a}'
                | '\u{b}'
                | '\u{c}'
                | '\u{d}'
                | ' '
                | '\u{a0}'
                | '\u{1680}'
                | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    })
}

/// A canonical date (`YYYY-AA-GG`) as days since 1970-01-01.
pub fn read_date(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let digits = |s: &str| -> Option<i64> {
        if s.bytes().all(|c| c.is_ascii_digit()) {
            s.parse().ok()
        } else {
            None
        }
    };
    let (y, m, d) = (
        digits(&text[0..4])?,
        digits(&text[5..7])?,
        digits(&text[8..10])?,
    );
    if !(1..=9999).contains(&y) || !(1..=12).contains(&m) || d < 1 || d > month_days(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

fn month_days(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A number's key: its value with no trailing zeros in its scale.
pub fn number_key(d: Dec) -> Key {
    let (mut m, mut s) = (d.m, d.scale);
    while s > 0 && m % 10 == 0 {
        m /= 10;
        s -= 1;
    }
    Key::Num(m, s)
}

impl Read {
    /// A column read: each cell by the column's kind; a value the kind does
    /// not read is refused, a text empty once trimmed is empty.
    pub fn of(col: &Column) -> Read {
        let mut out = Read {
            vals: Vec::with_capacity(col.cells.len()),
            kind: Some(col.kind),
            labelled: col.labelled,
            ..Read::default()
        };
        let mut interned: FastMap<String, u32> = FastMap::default();
        // A text's index, and whether it is met for the first time.
        let mut intern = |texts: &mut Vec<String>, t: &str| -> (u32, bool) {
            if let Some(&i) = interned.get(t) {
                return (i, false);
            }
            let i = texts.len() as u32;
            texts.push(t.to_owned());
            interned.insert(t.to_owned(), i);
            (i, true)
        };
        for cell in &col.cells {
            let val = match cell {
                Cell::Empty => Val::Empty,
                Cell::Invalid(t) => {
                    let t = js_trim(t);
                    if t.is_empty() {
                        Val::Empty
                    } else {
                        Val::Invalid(intern(&mut out.texts, t).0)
                    }
                }
                Cell::Value(canon, shown) => {
                    let val = match col.kind {
                        Kind::Number => read_number(canon).map(Val::Num),
                        Kind::Date => read_date(canon).map(Val::Date),
                        Kind::Boolean => match canon.as_str() {
                            "true" => Some(Val::Bool(true)),
                            "false" => Some(Val::Bool(false)),
                            _ => None,
                        },
                        Kind::Text => {
                            let t = js_trim(canon);
                            if t.is_empty() {
                                Some(Val::Empty)
                            } else {
                                // A text met first: what it shows is decided now.
                                let (i, new) = intern(&mut out.texts, t);
                                if new && let Some(s) = shown {
                                    out.shown.insert(Key::Text(i), js_trim(s).to_owned());
                                }
                                Some(Val::Text(i))
                            }
                        }
                    };
                    match val {
                        Some(v) => {
                            match v {
                                Val::Empty | Val::Text(_) | Val::Invalid(_) => {}
                                // A number met first keeps its value as written, or what it shows.
                                Val::Num(d) => {
                                    let key = number_key(d);
                                    if let std::collections::hash_map::Entry::Vacant(e) =
                                        out.first.entry(key)
                                    {
                                        e.insert(d);
                                        if let Some(s) = shown {
                                            out.shown.insert(key, js_trim(s).to_owned());
                                        }
                                    }
                                }
                                Val::Date(_) | Val::Bool(_) => {
                                    let key = out.key(v);
                                    out.shown.entry(key).or_insert_with(|| {
                                        js_trim(shown.as_deref().unwrap_or(canon)).to_owned()
                                    });
                                }
                            }
                            v
                        }
                        None => {
                            let t = js_trim(canon);
                            if t.is_empty() {
                                Val::Empty
                            } else {
                                Val::Invalid(intern(&mut out.texts, t).0)
                            }
                        }
                    }
                }
            };
            out.vals.push(val);
        }
        out
    }

    /// A value's key.
    pub fn key(&self, v: Val) -> Key {
        match v {
            Val::Empty => Key::Empty,
            Val::Invalid(i) => Key::Invalid(i),
            Val::Num(d) => number_key(d),
            Val::Date(d) => Key::Date(d),
            Val::Bool(b) => Key::Bool(b),
            Val::Text(i) => Key::Text(i),
        }
    }

    /// What a key shows: a value its first shown text, a refused value its
    /// text, the empty “(boş)”.
    pub fn show(&self, key: Key) -> String {
        if let Some(s) = self.shown.get(&key) {
            return s.clone();
        }
        match key {
            Key::Empty => EMPTY.to_owned(),
            Key::Invalid(i) | Key::Text(i) => self.texts[i as usize].clone(),
            k @ Key::Num(..) => self.first.get(&k).map(|d| d.text()).unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// Two keys in the column's order (the module's description).
    pub fn cmp(&self, a: Key, b: Key) -> Ordering {
        let class = |k: Key| match k {
            Key::Empty => 2,
            Key::Invalid(_) => 1,
            _ => 0,
        };
        match class(a).cmp(&class(b)) {
            Ordering::Equal => {}
            o => return o,
        }
        match (a, b) {
            (Key::Invalid(x), Key::Invalid(y)) => {
                natural_cmp(&self.texts[x as usize], &self.texts[y as usize])
            }
            (Key::Empty, Key::Empty) => Ordering::Equal,
            _ if self.labelled => natural_cmp(&self.show(a), &self.show(b)),
            (Key::Num(m, s), Key::Num(n, t)) => {
                compare(Dec { m, scale: s }, Dec { m: n, scale: t })
            }
            (Key::Date(x), Key::Date(y)) => x.cmp(&y),
            (Key::Bool(x), Key::Bool(y)) => x.cmp(&y),
            (Key::Text(x), Key::Text(y)) => {
                natural_cmp(&self.texts[x as usize], &self.texts[y as usize])
            }
            _ => Ordering::Equal,
        }
    }

    /// The value of a row as a number, when it is one.
    pub fn number(&self, row: usize) -> Option<Dec> {
        match self.vals.get(row) {
            Some(Val::Num(d)) => Some(*d),
            _ => None,
        }
    }
}

/// A set of rows and their columns.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataSet {
    pub rows: usize,
    pub columns: Vec<Column>,
}

impl DataSet {
    /// The column with this key.
    pub fn find(&self, key: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.key == key)
    }
}

/// Groups of rows by their keys in one column: the keys in the column's
/// order, each with its rows (in the rows' order).
pub fn groups(read: &Read, rows: &[u32]) -> Vec<(Key, Vec<u32>)> {
    let mut at: FastMap<Key, usize> = FastMap::default();
    let mut out: Vec<(Key, Vec<u32>)> = Vec::new();
    for &r in rows {
        let key = read.key(read.vals[r as usize]);
        match at.get(&key) {
            Some(&i) => out[i].1.push(r),
            None => {
                at.insert(key, out.len());
                out.push((key, vec![r]));
            }
        }
    }
    out.sort_by(|a, b| read.cmp(a.0, b.0));
    out
}

/// The largest scale among the numbers of these rows.
pub fn top_scale(read: &Read, rows: &[u32]) -> u32 {
    rows.iter()
        .filter_map(|&r| read.number(r as usize))
        .map(|d| d.scale)
        .max()
        .unwrap_or(0)
}

/// `d` at `scale` digits, exactly (none when it does not fit).
pub fn scaled(d: Dec, scale: u32) -> Option<i128> {
    d.at(scale).map(|d| d.m)
}

/// A decimal's text at its scale.
pub fn dec_text(m: i128, scale: u32) -> String {
    Dec { m, scale }.text()
}

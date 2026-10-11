//! Tables written as workbooks (docs/adr/0223 §2): an Excel workbook
//! (XLSX), an OpenDocument spreadsheet (ODS) or a CSV file, from sheets of
//! named, typed columns and rows of words: Öznitelik tablosu's Dışa aktar
//! and İşlemler's tables. The independent reference is
//! scripts/fixtures/workbook_cases.py (fixtures/table/v1/written), which
//! reads every written file back with openpyxl and LibreOffice.
//!
//! A word is written by its column's kind: a number when it is a decimal
//! number's text with at most 15 significant digits (what a spreadsheet's
//! number keeps; more stay text, and the report says so), a date when it is
//! a day of the calendar YYYY-MM-DD (Excel's from 1 March 1900, the days
//! before it text, its count being a day off there), yes or no for true and
//! false, text otherwise; a column of no kind (`auto`) writes a plain
//! number (no leading zero, no trailing zero after the point, not -0) as a
//! number and anything else as text. An empty word is an empty cell. A
//! number's column with its fraction digits shows that many (0.00).
//!
//! The first row names the columns, bold, frozen, with filter buttons; the
//! columns are as wide as their words (8 to 60 characters). A sheet's name
//! keeps Excel's rule in both formats (1 to 31 characters, none of
//! : \ / ? * [ ], no apostrophe at either end, not History, one per name
//! whatever its case), a changed one said. XLSX and ODS take at most
//! 1 048 575 rows and 16 384 columns a sheet; a bigger table is refused,
//! with CSV offered. The same input gives the same bytes.
//!
//! CSV is Katman listesi's (Excel's in Turkish): a byte order mark, fields
//! between semicolons (quoted when they hold one, a quote or a line break),
//! lines ending in CR LF, every word as written.

mod ods;
mod xlsx;

use std::collections::HashSet;

use kentos_contracts::{CellKind, ExportReport, WorkbookFormat, WorkbookSheet, WorkbookWriteInput};

use crate::report::Report;
use crate::table_file::dates;

/// The most rows of a sheet XLSX and ODS take, the names' row with them.
pub const MAX_ROWS: usize = 1_048_576;
/// The most columns of a sheet.
pub const MAX_COLUMNS: usize = 16_384;
/// The most significant digits a written number keeps.
const DIGITS: usize = 15;

/// A word as it is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell<'a> {
    Empty,
    Text(&'a str),
    /// A decimal number's text (a sign, digits, a point and digits).
    Number(&'a str),
    /// A day of the calendar, YYYY-MM-DD.
    Date(&'a str),
    Boolean(bool),
}

/// How many significant digits a decimal number's text has: its digits
/// without the leading zeros and the fraction's trailing ones.
fn significant(t: &str) -> usize {
    let t = t.strip_prefix('-').unwrap_or(t);
    let (whole, fraction) = t.split_once('.').unwrap_or((t, ""));
    let fraction = fraction.trim_end_matches('0');
    let whole = whole.trim_start_matches('0');
    if whole.is_empty() {
        fraction.trim_start_matches('0').len()
    } else {
        whole.len() + fraction.len()
    }
}

/// A sign, digits, and a point with digits after it.
fn is_decimal(t: &str) -> bool {
    let t = t.strip_prefix('-').unwrap_or(t);
    let (whole, fraction) = match t.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (t, None),
    };
    !whole.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && fraction.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
}

/// A plain number: no leading zero, no trailing zero after the point, not -0.
fn is_plain(t: &str) -> bool {
    if !is_decimal(t) || t == "-0" {
        return false;
    }
    let u = t.strip_prefix('-').unwrap_or(t);
    let (whole, fraction) = u.split_once('.').unwrap_or((u, ""));
    let leading_zero = whole.len() > 1 && whole.starts_with('0');
    !(leading_zero || fraction.ends_with('0'))
}

/// Why a word was not written as its column says: a number with too many
/// digits or not a number, a date not a day or before Excel's 1 March 1900,
/// a yes or no that is neither.
#[derive(Default)]
struct Fallbacks {
    long_numbers: u32,
    not_numbers: u32,
    not_dates: u32,
    early_dates: u32,
    not_booleans: u32,
}

impl Fallbacks {
    fn say(&self, rep: &mut Report) {
        rep.note_n(
            "Uzun sayı",
            "15'ten çok anlamlı basamaklı sayılar metin olarak yazıldı: hesap tablosu sayıyı 15 basamakla tutar",
            0,
            self.long_numbers,
        );
        rep.note_n(
            "Sayı olmayan değer",
            "sayı sütunundaki sayı olmayan değerler metin olarak yazıldı",
            0,
            self.not_numbers,
        );
        rep.note_n(
            "Tarih olmayan değer",
            "tarih sütunundaki YYYY-AA-GG olmayan değerler metin olarak yazıldı",
            0,
            self.not_dates,
        );
        rep.note_n(
            "Eski tarih",
            "1 Mart 1900'den önceki tarihler metin olarak yazıldı: Excel'in gün sayımı o günlerde bir gün kayıktır",
            0,
            self.early_dates,
        );
        rep.note_n(
            "Evet/hayır olmayan değer",
            "evet/hayır sütunundaki true ya da false olmayan değerler metin olarak yazıldı",
            0,
            self.not_booleans,
        );
    }
}

/// What `word` of a column of `kind` is written as; `excel` when its dates
/// must be Excel's.
fn cell<'a>(kind: CellKind, word: &'a str, excel: bool, fb: &mut Fallbacks) -> Cell<'a> {
    if word.is_empty() {
        return Cell::Empty;
    }
    match kind {
        CellKind::Text => Cell::Text(word),
        CellKind::Number => {
            if !is_decimal(word) {
                fb.not_numbers += 1;
                Cell::Text(word)
            } else if significant(word) > DIGITS {
                fb.long_numbers += 1;
                Cell::Text(word)
            } else {
                Cell::Number(word)
            }
        }
        CellKind::Auto => {
            if is_plain(word) && significant(word) <= DIGITS {
                Cell::Number(word)
            } else {
                Cell::Text(word)
            }
        }
        CellKind::Date => {
            if dates::iso_day(word).is_none() {
                fb.not_dates += 1;
                Cell::Text(word)
            } else if excel && dates::serial(word).is_none() {
                fb.early_dates += 1;
                Cell::Text(word)
            } else {
                Cell::Date(word)
            }
        }
        CellKind::Boolean => match word {
            "true" => Cell::Boolean(true),
            "false" => Cell::Boolean(false),
            _ => {
                fb.not_booleans += 1;
                Cell::Text(word)
            }
        },
    }
}

/// A column's width in characters: its widest word (a date 10, yes or no 6)
/// and its name with room for the filter button, between 8 and 60.
fn width(sheet: &WorkbookSheet, j: usize) -> f64 {
    let name = sheet
        .columns
        .get(j)
        .map_or(0, |c| c.name.chars().count() + 3);
    let widest = sheet
        .rows
        .iter()
        .filter_map(|r| r.get(j))
        .map(|w| w.lines().map(|l| l.chars().count()).max().unwrap_or(0))
        .max()
        .unwrap_or(0);
    (name.max(widest) + 2).clamp(8, 60) as f64
}

/// The sheets' names by Excel's rule, each changed one said.
fn names(sheets: &[WorkbookSheet], rep: &mut Report) -> Vec<String> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(sheets.len());
    for (k, s) in sheets.iter().enumerate() {
        let mut name: String = s
            .name
            .chars()
            .map(|c| {
                if matches!(c, ':' | '\\' | '/' | '?' | '*' | '[' | ']') || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        name = name.trim().trim_matches('\'').trim().to_owned();
        if name.is_empty() {
            name = format!("Sayfa{}", k + 1);
        }
        if name.chars().count() > 31 {
            name = name.chars().take(31).collect();
        }
        if name.eq_ignore_ascii_case("History") {
            name.push('_');
        }
        let base = name.clone();
        let mut n = 1;
        while taken.contains(&name.to_lowercase()) {
            n += 1;
            let tail = format!(" ({n})");
            let room = 31 - tail.chars().count();
            name = base.chars().take(room).collect::<String>() + &tail;
        }
        taken.insert(name.to_lowercase());
        if name != s.name {
            rep.note(
                "Sayfa adı",
                &format!(
                    "“{}” adı “{name}” olarak yazıldı (sayfa adı en çok 31 karakter olur, : \\ / ? * [ ] taşımaz, iki sayfada aynı olmaz)",
                    s.name
                ),
                0,
            );
        }
        out.push(name);
    }
    out
}

/// A column's letters: 0 is A, 26 is AA.
pub(crate) fn column_letters(mut j: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (j % 26) as u8);
        if j < 26 {
            break;
        }
        j = j / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// XML's text of `t` with &, < and > (and " in an attribute) as entities.
pub(crate) fn escape(t: &str, out: &mut String) {
    let mut from = 0;
    for (i, b) in t.bytes().enumerate() {
        let entity = match b {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            _ => continue,
        };
        out.push_str(&t[from..i]);
        out.push_str(entity);
        from = i + 1;
    }
    out.push_str(&t[from..]);
}

/// A count's digits, without the formatting machinery (a sheet writes millions).
pub(crate) fn push_count(out: &mut String, mut n: usize) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    // ASCII digits only.
    out.push_str(std::str::from_utf8(&digits[at..]).unwrap_or("0"));
}

/// A CSV field: quoted when it holds a semicolon, a quote or a line break.
fn csv_field(t: &str, out: &mut String) {
    if t.contains([';', '"', '\r', '\n']) {
        out.push('"');
        out.push_str(&t.replace('"', "\"\""));
        out.push('"');
    } else {
        out.push_str(t);
    }
}

fn csv(sheet: &WorkbookSheet) -> Vec<u8> {
    let mut out = String::from('\u{feff}');
    let line = |words: &mut dyn Iterator<Item = &str>, out: &mut String| {
        for (k, w) in words.enumerate() {
            if k > 0 {
                out.push(';');
            }
            csv_field(w, out);
        }
        out.push_str("\r\n");
    };
    line(&mut sheet.columns.iter().map(|c| c.name.as_str()), &mut out);
    for r in &sheet.rows {
        line(&mut r.iter().map(String::as_str), &mut out);
    }
    out.into_bytes()
}

/// The file's bytes and what was written otherwise than asked; or why it is
/// not written.
pub fn write(input: &WorkbookWriteInput) -> Result<(Vec<u8>, ExportReport), String> {
    let mut rep = Report::default();
    if input.sheets.is_empty() {
        return Err("Yazılacak tablo yok.".into());
    }
    let rows: usize = input.sheets.iter().map(|s| s.rows.len()).sum();
    if input.format == WorkbookFormat::Csv {
        let sheet = &input.sheets[0];
        rep.fact("Satır", rows.to_string());
        return Ok((csv(sheet), rep.export()));
    }
    for s in &input.sheets {
        let width = s
            .rows
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .max(s.columns.len());
        if s.rows.len() + 1 > MAX_ROWS || width > MAX_COLUMNS {
            return Err(format!(
                "“{}” tablosu {} satır ve {width} sütun; XLSX ve ODS bir sayfada en çok {} satır ve {MAX_COLUMNS} sütun alır. Tabloyu süzün ya da CSV olarak kaydedin.",
                s.name,
                s.rows.len(),
                MAX_ROWS - 1
            ));
        }
    }
    let names = names(&input.sheets, &mut rep);
    let mut fb = Fallbacks::default();
    let bytes = match input.format {
        WorkbookFormat::Xlsx => xlsx::write(&input.sheets, &names, &mut fb)?,
        _ => ods::write(&input.sheets, &names, &mut fb, &mut rep)?,
    };
    fb.say(&mut rep);
    rep.fact("Satır", rows.to_string());
    Ok((bytes, rep.export()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_become_the_cells_their_columns_say() {
        let mut fb = Fallbacks::default();
        let mut c = |k: CellKind, w: &'static str| cell(k, w, true, &mut fb);
        assert_eq!(c(CellKind::Number, "-12.50"), Cell::Number("-12.50"));
        assert_eq!(
            c(CellKind::Number, "452345.123"),
            Cell::Number("452345.123")
        );
        assert_eq!(
            c(CellKind::Number, "1234567890123456"),
            Cell::Text("1234567890123456")
        );
        assert_eq!(
            c(CellKind::Number, "0.000000000000001234"),
            Cell::Number("0.000000000000001234")
        );
        assert_eq!(c(CellKind::Number, "1,5"), Cell::Text("1,5"));
        assert_eq!(c(CellKind::Number, ""), Cell::Empty);
        assert_eq!(c(CellKind::Auto, "007"), Cell::Text("007"));
        assert_eq!(c(CellKind::Auto, "1.10"), Cell::Text("1.10"));
        assert_eq!(c(CellKind::Auto, "-0"), Cell::Text("-0"));
        assert_eq!(c(CellKind::Auto, "0.5"), Cell::Number("0.5"));
        assert_eq!(c(CellKind::Auto, "101/5"), Cell::Text("101/5"));
        assert_eq!(c(CellKind::Date, "2024-10-01"), Cell::Date("2024-10-01"));
        assert_eq!(c(CellKind::Date, "1900-02-28"), Cell::Text("1900-02-28"));
        assert_eq!(c(CellKind::Date, "01.10.2024"), Cell::Text("01.10.2024"));
        assert_eq!(c(CellKind::Boolean, "true"), Cell::Boolean(true));
        assert_eq!(c(CellKind::Boolean, "Evet"), Cell::Text("Evet"));
        assert_eq!(fb.long_numbers, 1);
        assert_eq!(fb.not_numbers, 1);
        assert_eq!(fb.early_dates, 1);
        assert_eq!(fb.not_dates, 1);
        assert_eq!(fb.not_booleans, 1);
        assert_eq!(
            cell(CellKind::Date, "1900-02-28", false, &mut fb),
            Cell::Date("1900-02-28")
        );
    }

    #[test]
    fn sheet_names_keep_excels_rule() {
        let sheet = |n: &str| WorkbookSheet {
            name: n.to_owned(),
            ..WorkbookSheet::default()
        };
        let mut rep = Report::default();
        let got = names(
            &[
                sheet("Parseller"),
                sheet("parseller"),
                sheet("Ada/Parsel: [1]"),
                sheet("'Yol'"),
                sheet(""),
                sheet("Çok uzun bir katman adı ki otuz bir karakteri aşar"),
                sheet("History"),
            ],
            &mut rep,
        );
        assert_eq!(
            got,
            [
                "Parseller",
                "parseller (2)",
                "Ada_Parsel_ _1_",
                "Yol",
                "Sayfa5",
                "Çok uzun bir katman adı ki otuz",
                "History_",
            ]
        );
        assert_eq!(rep.export().notes.len(), 6);
    }

    #[test]
    fn columns_have_letters() {
        assert_eq!(column_letters(0), "A");
        assert_eq!(column_letters(25), "Z");
        assert_eq!(column_letters(26), "AA");
        assert_eq!(column_letters(16_383), "XFD");
    }
}

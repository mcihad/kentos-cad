//! A table file (docs/adr/0184 §4, 0223 §1): a CSV or text table, an Excel
//! workbook (XLSX) or an OpenDocument spreadsheet (ODS), read as rows of
//! words per sheet. Tablo ekle's cells are the core's rule over them
//! (`ops::table::from_rows`); İşlemler's file parameter takes a sheet's rows
//! as a table whose first row names the columns. Nothing is computed: a
//! number is the text the file holds; a workbook's date its ISO text. The
//! independent reference is scripts/fixtures/table_file_cases.py
//! (fixtures/table/v1/files).
//!
//! - A text file is read in its encoding (`text::sniff`: its byte order
//!   mark, UTF-8, else Windows-1254). Its records are CSV's: a field may be
//!   quoted ("" a quote inside, line breaks kept), an unquoted one loses the
//!   spaces round it; a line with nothing on it is no record. The separator
//!   is the first of tab, semicolon, comma and bar that splits the most of
//!   the first 50 records into the count most of them have (two or more);
//!   when none does, runs of spaces, if they split at least half of them
//!   alike; else each line is one field.
//! - A workbook's sheets in its order, by name (`xlsx`, `ods`): each cell
//!   where it is, its words as the module says; the missing ones empty.
//!   Their big parts are read as a stream (`xml_scan`), so a sheet of a
//!   quarter of a million rows is not held as a tree.
//! - At most `limits.rows` rows and `limits.columns` columns are read (Tablo
//!   ekle: one past a table's most, so that its rule says why; İşlemler: one
//!   past its own); a sheet with more rows says it was cut, and its reading
//!   stops there. An old Excel file (.xls) or another archive is refused
//!   with what to do instead; nothing here panics on any input.

pub(crate) mod dates;
mod ods;
pub(crate) mod xlsx;

use kentos_contracts::{MAX_TABLE_COLUMNS, MAX_TABLE_ROWS, TableFileRead, TableSheet};

use crate::text;
use crate::xml;
use crate::zip;

/// The largest file read, bytes.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
/// Records the separator is chosen from.
const SAMPLE: usize = 50;
/// The most rows of a sheet İşlemler takes, its column names' row aside (docs/adr/0223 §1).
pub const PROCESSING_ROWS: usize = 250_000;
/// The most columns of a sheet İşlemler takes.
pub const PROCESSING_COLUMNS: usize = 500;

/// How much of a sheet is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub rows: usize,
    pub columns: usize,
}

impl Limits {
    /// Tablo ekle's: one past a table's most, so that its rule says why.
    pub const TABLE: Limits = Limits {
        rows: MAX_TABLE_ROWS + 1,
        columns: MAX_TABLE_COLUMNS + 1,
    };
    /// İşlemler's file parameter: the column names' row and its most rows, and
    /// one column past its most (a sheet with more is refused, and says why).
    pub const PROCESSING: Limits = Limits {
        rows: PROCESSING_ROWS + 1,
        columns: PROCESSING_COLUMNS + 1,
    };
}

fn refused(problem: &str) -> TableFileRead {
    TableFileRead {
        problem: Some(problem.to_owned()),
        ..TableFileRead::default()
    }
}

/// The file's sheets as Tablo ekle reads them, or why it is not read.
pub fn read(bytes: &[u8]) -> TableFileRead {
    read_with(bytes, Limits::TABLE)
}

/// The file's sheets read within `limits`, or why it is not read.
pub fn read_with(bytes: &[u8], limits: Limits) -> TableFileRead {
    if bytes.len() > MAX_BYTES {
        return refused(&format!(
            "Dosya {} MB'tan büyük; bu kadar büyük tablo okunmuyor.",
            MAX_BYTES / (1024 * 1024)
        ));
    }
    if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        return refused(
            "Eski Excel biçimi (.xls) okunmuyor. Dosyayı Excel'de XLSX ya da CSV olarak kaydedip seçin.",
        );
    }
    if zip::sniff(bytes) {
        return match workbook(bytes, limits) {
            Ok(sheets) => TableFileRead {
                sheets,
                ..TableFileRead::default()
            },
            Err(why) => refused(&why),
        };
    }
    let (encoding, skip) = text::sniff(bytes);
    let words = text::decode(&bytes[skip..], encoding);
    let (rows, cut) = text_rows(&words, limits);
    TableFileRead {
        sheets: vec![TableSheet {
            name: None,
            rows,
            cut,
        }],
        encoding: Some(encoding.label().to_owned()),
        problem: None,
    }
}

// ── Text ────────────────────────────────────────────────────────────────

/// How a text file's fields are separated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sep {
    Char(char),
    Spaces,
    Line,
}

/// Up to `limit` records of `text`, split by `sep`, and whether there were more.
fn records(text: &str, sep: Sep, limit: usize) -> (Vec<Vec<String>>, bool) {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut chars = text.chars().peekable();
    while chars.peek().is_some() {
        if out.len() == limit {
            return (out, true);
        }
        // One record: up to its line break outside quotes.
        let mut fields: Vec<String> = Vec::new();
        let mut field = String::new();
        let mut quoted = false;
        let mut was_quoted = false;
        let mut any = false;
        while let Some(c) = chars.next() {
            if quoted {
                if c == '"' {
                    if chars.peek() == Some(&'"') {
                        field.push('"');
                        chars.next();
                    } else {
                        quoted = false;
                    }
                } else {
                    field.push(c);
                }
                continue;
            }
            if c == '\r' || c == '\n' {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                break;
            }
            any = true;
            match sep {
                Sep::Char(s) if c == s => {
                    fields.push(finish(&field, was_quoted));
                    field.clear();
                    was_quoted = false;
                }
                Sep::Char(_) if c == '"' && field.trim_matches(' ').is_empty() && !was_quoted => {
                    field.clear();
                    quoted = true;
                    was_quoted = true;
                }
                Sep::Spaces if c == ' ' || c == '\t' => {
                    if !field.is_empty() {
                        fields.push(std::mem::take(&mut field));
                    }
                }
                _ => field.push(c),
            }
        }
        if !any && field.is_empty() && fields.is_empty() && !was_quoted {
            continue;
        }
        match sep {
            Sep::Spaces => {
                if !field.is_empty() {
                    fields.push(field);
                }
            }
            _ => fields.push(finish(&field, was_quoted)),
        }
        out.push(fields);
    }
    (out, false)
}

/// A field as read: a quoted one as it is, else without the spaces round it.
fn finish(field: &str, quoted: bool) -> String {
    if quoted {
        field.to_owned()
    } else {
        field.trim_matches(' ').to_owned()
    }
}

/// How many of `rows` have the count most of them have, and that count
/// (the larger of two as common).
fn steadiness(rows: &[Vec<String>]) -> (usize, usize) {
    let mut counts: Vec<(usize, usize)> = Vec::new();
    for r in rows {
        match counts.iter_mut().find(|(n, _)| *n == r.len()) {
            Some((_, k)) => *k += 1,
            None => counts.push((r.len(), 1)),
        }
    }
    counts
        .into_iter()
        .map(|(n, k)| (k, n))
        .max()
        .unwrap_or((0, 0))
}

/// The separator of `text` (see the module comment).
fn separator(text: &str) -> Sep {
    let mut best: Option<(usize, Sep)> = None;
    for c in ['\t', ';', ',', '|'] {
        let (sample, _) = records(text, Sep::Char(c), SAMPLE);
        let (alike, count) = steadiness(&sample);
        if count >= 2 && best.is_none_or(|(b, _)| alike > b) {
            best = Some((alike, Sep::Char(c)));
        }
    }
    if let Some((_, sep)) = best {
        return sep;
    }
    let (sample, _) = records(text, Sep::Spaces, SAMPLE);
    let (alike, count) = steadiness(&sample);
    if count >= 2 && 2 * alike >= sample.len() {
        Sep::Spaces
    } else {
        Sep::Line
    }
}

/// A text file's rows, and whether there were more than are read.
fn text_rows(text: &str, limits: Limits) -> (Vec<Vec<String>>, bool) {
    let (mut rows, cut) = records(text, separator(text), limits.rows);
    for r in &mut rows {
        r.truncate(limits.columns);
    }
    (rows, cut)
}

// ── Workbooks ───────────────────────────────────────────────────────────

/// A workbook's sheets: an Excel workbook's, else an OpenDocument spreadsheet's.
fn workbook(bytes: &[u8], limits: Limits) -> Result<Vec<TableSheet>, String> {
    let listed = zip::list(bytes, &zip::Limits::default())?;
    let has = |name: &str| listed.iter().any(|l| l.name == name);
    if has("xl/workbook.xml") {
        return xlsx::sheets(bytes, &listed, limits);
    }
    if has("content.xml") {
        let kind = match entry(bytes, &listed, "mimetype")? {
            Some(m) => String::from_utf8_lossy(&m).trim().to_owned(),
            None => String::new(),
        };
        if kind.is_empty() || kind.starts_with("application/vnd.oasis.opendocument.spreadsheet") {
            return ods::sheets(bytes, &listed, limits);
        }
        return Err(
            "Arşiv bir OpenDocument belgesi ama hesap tablosu değil. Tabloyu ODS, XLSX, CSV ya da TXT olarak seçin."
                .into(),
        );
    }
    Err(
        "Arşiv bir Excel (XLSX) ya da OpenDocument (ODS) çalışma kitabı değil. Tabloyu XLSX, ODS, CSV ya da TXT olarak seçin."
            .into(),
    )
}

/// An entry of the archive by its name, unpacked.
fn entry(bytes: &[u8], listed: &[zip::Listed], name: &str) -> Result<Option<Vec<u8>>, String> {
    let found = zip::unpack(bytes, listed, &zip::Limits::default(), |l| l.name == name)?;
    Ok(found.into_iter().next().map(|e| e.data))
}

/// Why an XML part of the workbook is not read.
fn unparsed(why: xml::Unparsed, name: &str) -> String {
    match why {
        xml::Unparsed::NotXml => {
            format!("Çalışma kitabının “{name}” parçası okunamadı (XML değil).")
        }
        xml::Unparsed::TooDeep => {
            format!("Çalışma kitabının “{name}” parçası çok derin iç içe; okunmadı.")
        }
    }
}

/// A small XML part of the workbook as a tree, parsed with the depth guard.
fn part<'t>(text: &'t str, name: &str) -> Result<roxmltree::Document<'t>, String> {
    xml::document(text).map_err(|why| unparsed(why, name))
}

fn utf8(data: Vec<u8>, name: &str) -> Result<String, String> {
    String::from_utf8(data).map_err(|_| format!("Çalışma kitabının “{name}” parçası UTF-8 değil."))
}

/// A sheet's rows as they come: each row's cells placed where they are,
/// none past `limits`; whether a row past them had a cell.
struct Grid {
    rows: Vec<Vec<String>>,
    limits: Limits,
    cut: bool,
}

impl Grid {
    fn new(limits: Limits) -> Self {
        Self {
            rows: Vec::new(),
            limits,
            cut: false,
        }
    }

    /// `words` at row `i`, column `j` (from 0); false once a word lies in a
    /// row past the most read (the sheet then cut, and read no further).
    fn put(&mut self, i: usize, j: usize, words: String) -> bool {
        if words.is_empty() {
            return true;
        }
        if i >= self.limits.rows {
            self.cut = true;
            return false;
        }
        if j >= self.limits.columns {
            return true;
        }
        if self.rows.len() <= i {
            self.rows.resize(i + 1, Vec::new());
        }
        let r = &mut self.rows[i];
        if r.len() <= j {
            r.resize(j + 1, String::new());
        }
        r[j] = words;
        true
    }

    fn sheet(self, name: String) -> TableSheet {
        TableSheet {
            name: Some(name),
            rows: self.rows,
            cut: self.cut,
        }
    }
}

//! Tablo ekle's file (docs/adr/0184 §4): a CSV or text table, or an Excel
//! workbook (XLSX), read as rows of words per sheet; the table's cells are
//! the core's rule over them (`ops::table::from_rows`). Nothing is computed:
//! a number is the text the file holds. The independent reference is
//! scripts/fixtures/table_file_cases.py (fixtures/table/v1/files).
//!
//! - A text file is read in its encoding (`text::sniff`: its byte order
//!   mark, UTF-8, else Windows-1254). Its records are CSV's: a field may be
//!   quoted ("" a quote inside, line breaks kept), an unquoted one loses the
//!   spaces round it; a line with nothing on it is no record. The separator
//!   is the first of tab, semicolon, comma and bar that splits the most of
//!   the first 50 records into the count most of them have (two or more);
//!   when none does, runs of spaces, if they split at least half of them
//!   alike; else each line is one field.
//! - A workbook's sheets in its order, by name: each cell where its
//!   reference puts it (else after the one before), a shared string's text
//!   (its runs, without the phonetic ones), an inline string's, a boolean as
//!   DOĞRU or YANLIŞ, anything else (a number, an error, a formula's text)
//!   as written; the missing ones empty.
//! - At most `MAX_TABLE_ROWS` + 1 rows and `MAX_TABLE_COLUMNS` + 1 columns
//!   are read (the table's rule refuses more and says so); a sheet with
//!   more rows says it was cut. An old Excel file (.xls) or another archive
//!   is refused with what to do instead; nothing here panics on any input.

use kentos_contracts::{MAX_TABLE_COLUMNS, MAX_TABLE_ROWS, TableFileRead, TableSheet};
use roxmltree::Node;

use crate::text;
use crate::xml;
use crate::zip;

/// The largest file read, bytes.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
/// Records the separator is chosen from.
const SAMPLE: usize = 50;
/// The rows and columns read: one past a table's most, so that its rule says why.
const ROWS: usize = MAX_TABLE_ROWS + 1;
const COLUMNS: usize = MAX_TABLE_COLUMNS + 1;

const RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn refused(problem: &str) -> TableFileRead {
    TableFileRead {
        problem: Some(problem.to_owned()),
        ..TableFileRead::default()
    }
}

/// The file's sheets, or why it is not read.
pub fn read(bytes: &[u8]) -> TableFileRead {
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
        return match workbook(bytes) {
            Ok(sheets) => TableFileRead {
                sheets,
                ..TableFileRead::default()
            },
            Err(why) => refused(&why),
        };
    }
    let (encoding, skip) = text::sniff(bytes);
    let words = text::decode(&bytes[skip..], encoding);
    let (rows, cut) = text_rows(&words);
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
fn text_rows(text: &str) -> (Vec<Vec<String>>, bool) {
    let (mut rows, cut) = records(text, separator(text), ROWS);
    for r in &mut rows {
        r.truncate(COLUMNS);
    }
    (rows, cut)
}

// ── Workbook ────────────────────────────────────────────────────────────

/// An entry of the archive by its name, unpacked.
fn entry(bytes: &[u8], listed: &[zip::Listed], name: &str) -> Result<Option<Vec<u8>>, String> {
    let found = zip::unpack(bytes, listed, &zip::Limits::default(), |l| l.name == name)?;
    Ok(found.into_iter().next().map(|e| e.data))
}

/// An XML part of the workbook, parsed with the depth guard.
fn part<'t>(text: &'t str, name: &str) -> Result<roxmltree::Document<'t>, String> {
    xml::document(text).map_err(|why| match why {
        xml::Unparsed::NotXml => {
            format!("Çalışma kitabının “{name}” parçası okunamadı (XML değil).")
        }
        xml::Unparsed::TooDeep => {
            format!("Çalışma kitabının “{name}” parçası çok derin iç içe; okunmadı.")
        }
    })
}

fn utf8(data: Vec<u8>, name: &str) -> Result<String, String> {
    String::from_utf8(data).map_err(|_| format!("Çalışma kitabının “{name}” parçası UTF-8 değil."))
}

/// The text of `node`'s `t` descendants, those under a phonetic run (`rPh`) aside.
fn runs_text(node: Node<'_, '_>) -> String {
    let mut out = String::new();
    for d in node.descendants() {
        if d.has_tag_name("t") && !d.ancestors().any(|a| a.has_tag_name("rPh")) {
            out.push_str(d.text().unwrap_or(""));
        }
    }
    out
}

/// A cell reference's column and row, from 0 (`B3`: 1, 2).
fn reference(r: &str) -> Option<(usize, usize)> {
    let letters = r.bytes().take_while(u8::is_ascii_alphabetic).count();
    if letters == 0 || letters > 3 {
        return None;
    }
    let col = r[..letters].bytes().try_fold(0usize, |n, b| {
        Some(n * 26 + usize::from(b.to_ascii_uppercase().checked_sub(b'A')?) + 1)
    })?;
    let row: usize = r[letters..].parse().ok()?;
    Some((col.checked_sub(1)?, row.checked_sub(1)?))
}

/// A workbook's sheets, in its order.
fn workbook(bytes: &[u8]) -> Result<Vec<TableSheet>, String> {
    let listed = zip::list(bytes, &zip::Limits::default())?;
    let Some(book) = entry(bytes, &listed, "xl/workbook.xml")? else {
        return Err(
            "Arşiv bir Excel çalışma kitabı (XLSX) değil. Tabloyu XLSX, CSV ya da TXT olarak seçin."
                .into(),
        );
    };
    let book = utf8(book, "workbook.xml")?;
    let rels = match entry(bytes, &listed, "xl/_rels/workbook.xml.rels")? {
        Some(r) => utf8(r, "workbook.xml.rels")?,
        None => String::new(),
    };
    let shared = match entry(bytes, &listed, "xl/sharedStrings.xml")? {
        Some(s) => utf8(s, "sharedStrings.xml")?,
        None => String::new(),
    };
    let book = part(&book, "workbook.xml")?;
    let targets: Vec<(String, String)> = if rels.is_empty() {
        Vec::new()
    } else {
        part(&rels, "workbook.xml.rels")?
            .descendants()
            .filter(|n| n.has_tag_name("Relationship"))
            .filter_map(|n| {
                Some((
                    n.attribute("Id")?.to_owned(),
                    n.attribute("Target")?.to_owned(),
                ))
            })
            .collect()
    };
    let strings: Vec<String> = if shared.is_empty() {
        Vec::new()
    } else {
        part(&shared, "sharedStrings.xml")?
            .root_element()
            .children()
            .filter(|n| n.has_tag_name("si"))
            .map(runs_text)
            .collect()
    };
    let mut sheets = Vec::new();
    for sheet in book.descendants().filter(|n| n.has_tag_name("sheet")) {
        let name = sheet.attribute("name").unwrap_or("").to_owned();
        let Some(id) = sheet.attribute((RELATIONSHIPS, "id")) else {
            continue;
        };
        let Some((_, target)) = targets.iter().find(|(i, _)| i == id) else {
            continue;
        };
        let path = match target.strip_prefix('/') {
            Some(abs) => abs.to_owned(),
            None => format!("xl/{target}"),
        };
        let Some(data) = entry(bytes, &listed, &path)? else {
            continue;
        };
        let data = utf8(data, &path)?;
        let (rows, cut) = sheet_rows(&part(&data, &path)?, &strings);
        sheets.push(TableSheet {
            name: Some(name),
            rows,
            cut,
        });
    }
    if sheets.is_empty() {
        return Err("Çalışma kitabında okunacak sayfa yok.".into());
    }
    Ok(sheets)
}

/// A sheet's rows, and whether it has more than are read.
fn sheet_rows(doc: &roxmltree::Document<'_>, strings: &[String]) -> (Vec<Vec<String>>, bool) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut cut = false;
    let mut next_row = 0usize;
    for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
        let i = row
            .attribute("r")
            .and_then(|r| r.parse::<usize>().ok())
            .and_then(|r| r.checked_sub(1))
            .unwrap_or(next_row);
        next_row = i + 1;
        if i >= ROWS {
            cut = true;
            continue;
        }
        let mut next_col = 0usize;
        for c in row.children().filter(|n| n.has_tag_name("c")) {
            let j = c
                .attribute("r")
                .and_then(reference)
                .map_or(next_col, |(col, _)| col);
            next_col = j + 1;
            if j >= COLUMNS {
                continue;
            }
            let v = c
                .children()
                .find(|n| n.has_tag_name("v"))
                .and_then(|n| n.text())
                .unwrap_or("");
            let words = match c.attribute("t") {
                Some("s") => v
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .and_then(|k| strings.get(k))
                    .cloned()
                    .unwrap_or_default(),
                Some("inlineStr") => c
                    .children()
                    .find(|n| n.has_tag_name("is"))
                    .map(runs_text)
                    .unwrap_or_default(),
                Some("b") => match v.trim() {
                    "1" => "DOĞRU".to_owned(),
                    "0" => "YANLIŞ".to_owned(),
                    other => other.to_owned(),
                },
                _ => v.to_owned(),
            };
            if words.is_empty() {
                continue;
            }
            if rows.len() <= i {
                rows.resize(i + 1, Vec::new());
            }
            let r = &mut rows[i];
            if r.len() <= j {
                r.resize(j + 1, String::new());
            }
            r[j] = words;
        }
    }
    (rows, cut)
}

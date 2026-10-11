//! An Excel workbook (XLSX; docs/adr/0184 §4, 0223 §1): its sheets in
//! xl/workbook.xml's order, each by its name through its relationship (a
//! target relative to xl/, or absolute from the root). A cell where its
//! reference (A1, from 0) puts it, else after the one before in its row (a
//! row where its r says, else after the one before). Its words: a shared
//! string's text (its runs', the phonetic ones under rPh aside), an inline
//! string's alike, a formula's string result (t="str") as written, each
//! with Excel's `_xHHHH_` escapes read as the character they name; a
//! boolean DOĞRU or YANLIŞ; a number in a date or time format its ISO text
//! (`dates`); anything else (a number, an error, an ISO date cell) as
//! written. The sheets and the shared strings are read as streams; the
//! workbook, its relationships and its styles, which are small, as trees.

use std::borrow::Cow;
use std::collections::HashMap;

use kentos_contracts::TableSheet;

use super::dates::{self, Clock};
use super::{Grid, Limits, entry, part, unparsed, utf8};
use crate::xml::Unparsed;
use crate::xml_scan::{Event, Ns, Scan};
use crate::zip;

const RELATIONSHIPS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// ISO/IEC 29500's strict relationships.
const RELATIONSHIPS_STRICT: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships";

/// `_xHHHH_` read as the character it names, left to right (`_x005F_` an
/// underscore, so `_x005F_x0041_` stays `_x0041_`); one naming no
/// character stays as written.
pub(crate) fn unescape(text: &str) -> Cow<'_, str> {
    if !text.contains("_x") {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("_x") {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let named = tail
            .get(2..6)
            .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
            .filter(|_| tail.as_bytes().get(6) == Some(&b'_'))
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .and_then(char::from_u32);
        match named {
            Some(c) => {
                out.push(c);
                rest = &tail[7..];
            }
            None => {
                out.push_str("_x");
                rest = &tail[2..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
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

/// What each cell style (cellXfs, by index) shows of a date, if it is one.
fn clocks(doc: &roxmltree::Document<'_>) -> Vec<Option<Clock>> {
    let mut custom: HashMap<u32, &str> = HashMap::new();
    for n in doc.descendants().filter(|n| n.has_tag_name("numFmt")) {
        if let (Some(id), Some(code)) = (
            n.attribute("numFmtId").and_then(|v| v.parse::<u32>().ok()),
            n.attribute("formatCode"),
        ) {
            custom.insert(id, code);
        }
    }
    let Some(xfs) = doc.descendants().find(|n| n.has_tag_name("cellXfs")) else {
        return Vec::new();
    };
    xfs.children()
        .filter(|n| n.has_tag_name("xf"))
        .map(|xf| {
            let id = xf.attribute("numFmtId")?.parse::<u32>().ok()?;
            let format = custom.get(&id).copied().or_else(|| dates::builtin(id))?;
            dates::clock(format)
        })
        .collect()
}

/// The shared strings, in order.
fn shared_strings(text: &str) -> Result<Vec<String>, Unparsed> {
    let mut scan = Scan::new(text, &[]);
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    let (mut in_t, mut phonetic) = (0usize, 0usize);
    loop {
        match scan.event()? {
            None => break,
            Some(Event::Start(s)) => match s.local {
                "si" => current = Some(String::new()),
                "t" => in_t += 1,
                "rPh" => phonetic += 1,
                _ => {}
            },
            Some(Event::Text(t)) => {
                if in_t > 0
                    && phonetic == 0
                    && let Some(c) = current.as_mut()
                {
                    c.push_str(&t);
                }
            }
            Some(Event::End { local, .. }) => match local {
                "si" => out.push(unescape(&current.take().unwrap_or_default()).into_owned()),
                "t" => in_t = in_t.saturating_sub(1),
                "rPh" => phonetic = phonetic.saturating_sub(1),
                _ => {}
            },
        }
    }
    Ok(out)
}

/// What a cell's `t` says it holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    /// A number (no `t`, or `n`): a date when its style says so.
    #[default]
    Number,
    /// A shared string's index (`s`).
    Shared,
    /// Its own string (`inlineStr`).
    Inline,
    /// A formula's string result (`str`).
    Formula,
    /// Yes or no (`b`).
    Boolean,
    /// Anything else (an error, an ISO date) as written.
    Other,
}

fn kind_of(t: &str) -> Kind {
    match t {
        "" | "n" => Kind::Number,
        "s" => Kind::Shared,
        "inlineStr" => Kind::Inline,
        "str" => Kind::Formula,
        "b" => Kind::Boolean,
        _ => Kind::Other,
    }
}

/// A text's characters with Excel's escapes read, kept as it is when it has none.
fn unescaped(t: String) -> String {
    match unescape(&t) {
        Cow::Owned(o) => o,
        Cow::Borrowed(_) => t,
    }
}

/// The cell being read.
#[derive(Default)]
struct Cell {
    col: usize,
    kind: Kind,
    style: usize,
    value: String,
    inline: String,
    in_v: bool,
    in_is: bool,
    in_t: usize,
    phonetic: usize,
}

/// What a workbook's cells are read with.
struct Book<'b> {
    strings: &'b [String],
    clocks: &'b [Option<Clock>],
    date1904: bool,
}

impl Book<'_> {
    fn words(&self, c: Cell) -> String {
        match c.kind {
            Kind::Shared => c
                .value
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|k| self.strings.get(k))
                .cloned()
                .unwrap_or_default(),
            Kind::Inline => unescaped(c.inline),
            Kind::Formula => unescaped(c.value),
            Kind::Boolean => match c.value.trim() {
                "1" => "DOĞRU".to_owned(),
                "0" => "YANLIŞ".to_owned(),
                _ => c.value,
            },
            Kind::Number => match self.clocks.get(c.style).copied().flatten() {
                Some(clock) => dates::text(&c.value, clock, self.date1904).unwrap_or(c.value),
                None => c.value,
            },
            Kind::Other => c.value,
        }
    }

    /// A sheet's rows, read as a stream; its reading stops at the first
    /// word past `limits`' rows (the sheet then cut).
    fn rows(&self, text: &str, limits: Limits) -> Result<(Vec<Vec<String>>, bool), Unparsed> {
        let mut scan = Scan::new(text, &[]);
        let mut grid = Grid::new(limits);
        let (mut row, mut next_row, mut next_col) = (0usize, 0usize, 0usize);
        let mut cell: Option<Cell> = None;
        loop {
            match scan.event()? {
                None => break,
                Some(Event::Start(s)) => match s.local {
                    "row" => {
                        row = s
                            .attr(Ns::None, "r")?
                            .and_then(|r| r.parse::<usize>().ok())
                            .and_then(|r| r.checked_sub(1))
                            .unwrap_or(next_row);
                        next_row = row.saturating_add(1);
                        next_col = 0;
                    }
                    "c" => {
                        let col = s
                            .attr(Ns::None, "r")?
                            .and_then(|r| reference(&r))
                            .map_or(next_col, |(col, _)| col);
                        next_col = col.saturating_add(1);
                        cell = Some(Cell {
                            col,
                            kind: s.attr(Ns::None, "t")?.map_or(Kind::Number, |t| kind_of(&t)),
                            style: s
                                .attr(Ns::None, "s")?
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(0),
                            ..Cell::default()
                        });
                    }
                    "v" => {
                        if let Some(c) = cell.as_mut() {
                            c.in_v = true;
                        }
                    }
                    "is" => {
                        if let Some(c) = cell.as_mut() {
                            c.in_is = true;
                        }
                    }
                    "t" => {
                        if let Some(c) = cell.as_mut().filter(|c| c.in_is) {
                            c.in_t += 1;
                        }
                    }
                    "rPh" => {
                        if let Some(c) = cell.as_mut() {
                            c.phonetic += 1;
                        }
                    }
                    _ => {}
                },
                Some(Event::Text(t)) => {
                    if let Some(c) = cell.as_mut() {
                        if c.in_v {
                            c.value.push_str(&t);
                        } else if c.in_t > 0 && c.phonetic == 0 {
                            c.inline.push_str(&t);
                        }
                    }
                }
                Some(Event::End { local, .. }) => match local {
                    "c" => {
                        if let Some(c) = cell.take() {
                            let col = c.col;
                            if !grid.put(row, col, self.words(c)) {
                                break;
                            }
                        }
                    }
                    "v" => {
                        if let Some(c) = cell.as_mut() {
                            c.in_v = false;
                        }
                    }
                    "is" => {
                        if let Some(c) = cell.as_mut() {
                            c.in_is = false;
                        }
                    }
                    "t" => {
                        if let Some(c) = cell.as_mut() {
                            c.in_t = c.in_t.saturating_sub(1);
                        }
                    }
                    "rPh" => {
                        if let Some(c) = cell.as_mut() {
                            c.phonetic = c.phonetic.saturating_sub(1);
                        }
                    }
                    _ => {}
                },
            }
        }
        Ok((grid.rows, grid.cut))
    }
}

/// A workbook's sheets, in its order.
pub(super) fn sheets(
    bytes: &[u8],
    listed: &[zip::Listed],
    limits: Limits,
) -> Result<Vec<TableSheet>, String> {
    let book = entry(bytes, listed, "xl/workbook.xml")?.unwrap_or_default();
    let book = utf8(book, "workbook.xml")?;
    let rels = match entry(bytes, listed, "xl/_rels/workbook.xml.rels")? {
        Some(r) => utf8(r, "workbook.xml.rels")?,
        None => String::new(),
    };
    let styles = match entry(bytes, listed, "xl/styles.xml")? {
        Some(s) => utf8(s, "styles.xml")?,
        None => String::new(),
    };
    let shared = match entry(bytes, listed, "xl/sharedStrings.xml")? {
        Some(s) => utf8(s, "sharedStrings.xml")?,
        None => String::new(),
    };
    let book = part(&book, "workbook.xml")?;
    let date1904 = book
        .descendants()
        .find(|n| n.has_tag_name("workbookPr"))
        .and_then(|n| n.attribute("date1904"))
        .is_some_and(|v| v == "1" || v == "true");
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
    let clocks = if styles.is_empty() {
        Vec::new()
    } else {
        clocks(&part(&styles, "styles.xml")?)
    };
    let strings = if shared.is_empty() {
        Vec::new()
    } else {
        shared_strings(&shared).map_err(|why| unparsed(why, "sharedStrings.xml"))?
    };
    drop(shared);
    let reader = Book {
        strings: &strings,
        clocks: &clocks,
        date1904,
    };
    let mut sheets = Vec::new();
    for sheet in book.descendants().filter(|n| n.has_tag_name("sheet")) {
        let name = sheet.attribute("name").unwrap_or("").to_owned();
        let Some(id) = sheet
            .attribute((RELATIONSHIPS, "id"))
            .or_else(|| sheet.attribute((RELATIONSHIPS_STRICT, "id")))
        else {
            continue;
        };
        let Some((_, target)) = targets.iter().find(|(i, _)| i == id) else {
            continue;
        };
        let path = match target.strip_prefix('/') {
            Some(abs) => abs.to_owned(),
            None => format!("xl/{target}"),
        };
        let Some(data) = entry(bytes, listed, &path)? else {
            continue;
        };
        let data = utf8(data, &path)?;
        let (rows, cut) = reader
            .rows(&data, limits)
            .map_err(|why| unparsed(why, &path))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_name_characters() {
        assert_eq!(unescape("a_x000D_b"), "a\rb");
        assert_eq!(unescape("_x005F_x0041_"), "_x0041_");
        assert_eq!(unescape("_x00e7_"), "ç");
        assert_eq!(unescape("_xD800_ _x12_ _x"), "_xD800_ _x12_ _x");
        assert!(matches!(unescape("plain"), Cow::Borrowed(_)));
    }

    #[test]
    fn references_place_cells() {
        assert_eq!(reference("A1"), Some((0, 0)));
        assert_eq!(reference("AB12"), Some((27, 11)));
        assert_eq!(reference("XFD1048576"), Some((16_383, 1_048_575)));
        assert_eq!(reference("1A"), None);
        assert_eq!(reference("A0"), None);
    }
}

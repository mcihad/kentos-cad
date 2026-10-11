//! An Excel workbook written (docs/adr/0223 §2): the package's few parts
//! (content types, relationships, the workbook, its styles, the shared
//! strings, a worksheet a sheet), no document properties, dated so the same
//! sheets give the same bytes. A text is a shared string, its characters
//! XML's own and Excel's `_xHHHH_` for the rest: a control character (but
//! tab and LF) or a text's own `_xHHHH_` (its underscore `_x005F_`), so the
//! words come back as written. A number's cell holds its text; a date its
//! day count in the 1900 system, shown by the built-in date format (the
//! reader's locale: GG.AA.YYYY in Turkish); yes or no a boolean.

use std::collections::HashMap;
use std::fmt::Write as _;

use kentos_contracts::{CellKind, WorkbookSheet};

use super::{Cell, Fallbacks, cell, column_letters, escape, push_count, width};
use crate::table_file::dates;
use crate::zip;

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// A text as an Excel string's characters (see the module comment), written
/// in runs between the bytes that need more.
fn text(t: &str, out: &mut String) {
    let b = t.as_bytes();
    let (mut from, mut i) = (0, 0);
    while i < b.len() {
        let width = match b[i] {
            b'_' if b.get(i + 1) == Some(&b'x')
                && b.get(i + 6) == Some(&b'_')
                && b.get(i + 2..i + 6)
                    .is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)) =>
            {
                1
            }
            b'&' | b'<' | b'>' => 1,
            b'\t' | b'\n' => 0,
            0x00..=0x1f | 0x7f => 1,
            // U+0080 to U+009F, the other control characters.
            0xc2 if b.get(i + 1).is_some_and(|c| (0x80..=0x9f).contains(c)) => 2,
            // U+FFFE and U+FFFF.
            0xef if b.get(i + 1) == Some(&0xbf) && matches!(b.get(i + 2), Some(0xbe | 0xbf)) => 3,
            _ => 0,
        };
        if width == 0 {
            i += 1;
            continue;
        }
        out.push_str(&t[from..i]);
        match b[i] {
            b'_' => out.push_str("_x005F_"),
            b'&' => out.push_str("&amp;"),
            b'<' => out.push_str("&lt;"),
            b'>' => out.push_str("&gt;"),
            _ => {
                let c = t[i..].chars().next().map_or(0, u32::from);
                let _ = write!(out, "_x{c:04X}_");
            }
        }
        i += width;
        from = i;
    }
    out.push_str(&t[from..]);
}

/// The shared strings: each text once, in the order first written.
#[derive(Default)]
struct Strings<'a> {
    index: HashMap<&'a str, u32>,
    order: Vec<&'a str>,
    count: u32,
}

impl<'a> Strings<'a> {
    fn at(&mut self, t: &'a str) -> u32 {
        self.count += 1;
        let next = self.order.len() as u32;
        *self.index.entry(t).or_insert_with(|| {
            self.order.push(t);
            next
        })
    }

    fn xml(&self) -> String {
        let mut out =
            String::with_capacity(64 + self.order.iter().map(|t| t.len() + 40).sum::<usize>());
        let _ = write!(
            out,
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<sst xmlns=\"{MAIN}\" count=\"{}\" uniqueCount=\"{}\">",
            self.count,
            self.order.len()
        );
        for t in &self.order {
            out.push_str("<si><t xml:space=\"preserve\">");
            text(t, &mut out);
            out.push_str("</t></si>");
        }
        out.push_str("</sst>");
        out
    }
}

/// The cell styles: 0 plain, 1 the names' row (bold), 2 a date, then a
/// number's fraction digits (0 the built-in "0", more "0.00…").
struct Styles {
    decimals: Vec<u32>,
}

impl Styles {
    fn of(sheets: &[WorkbookSheet]) -> Self {
        let mut decimals: Vec<u32> = sheets
            .iter()
            .flat_map(|s| &s.columns)
            .filter(|c| c.kind == CellKind::Number)
            .filter_map(|c| c.decimals)
            .map(|d| d.min(15))
            .collect();
        decimals.sort_unstable();
        decimals.dedup();
        Self { decimals }
    }

    fn number(&self, decimals: Option<u32>) -> usize {
        decimals
            .and_then(|d| self.decimals.iter().position(|&x| x == d.min(15)))
            .map_or(0, |k| 3 + k)
    }

    fn xml(&self) -> String {
        let custom: Vec<(u32, u32)> = self
            .decimals
            .iter()
            .filter(|&&d| d > 0)
            .enumerate()
            .map(|(k, &d)| (164 + k as u32, d))
            .collect();
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"{MAIN}\">"
        );
        if !custom.is_empty() {
            let _ = write!(out, "<numFmts count=\"{}\">", custom.len());
            for (id, d) in &custom {
                let _ = write!(
                    out,
                    "<numFmt numFmtId=\"{id}\" formatCode=\"0.{}\"/>",
                    "0".repeat(*d as usize)
                );
            }
            out.push_str("</numFmts>");
        }
        out.push_str(concat!(
            "<fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/><family val=\"2\"/></font>",
            "<font><b/><sz val=\"11\"/><name val=\"Calibri\"/><family val=\"2\"/></font></fonts>",
            "<fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills>",
            "<borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders>",
            "<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>",
        ));
        let _ = write!(out, "<cellXfs count=\"{}\">", 3 + self.decimals.len());
        out.push_str("<xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>");
        out.push_str("<xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/>");
        out.push_str("<xf numFmtId=\"14\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/>");
        for d in &self.decimals {
            let id = if *d == 0 {
                1
            } else {
                custom.iter().find(|(_, x)| x == d).map_or(0, |(id, _)| *id)
            };
            let _ = write!(
                out,
                "<xf numFmtId=\"{id}\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/>"
            );
        }
        out.push_str("</cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>");
        out
    }
}

/// A sheet's range of its names' row and rows (A1 alone when empty).
fn range(sheet: &WorkbookSheet) -> String {
    let width = sheet
        .rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .max(sheet.columns.len())
        .max(1);
    format!("A1:{}{}", column_letters(width - 1), sheet.rows.len() + 1)
}

fn worksheet<'a>(
    sheet: &'a WorkbookSheet,
    strings: &mut Strings<'a>,
    styles: &Styles,
    fb: &mut Fallbacks,
) -> String {
    let headed = !sheet.columns.is_empty();
    let mut out = String::with_capacity(256 + sheet.rows.len() * sheet.columns.len().max(1) * 32);
    let _ = write!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"{MAIN}\" xmlns:r=\"{RELS}\"><dimension ref=\"{}\"/>",
        range(sheet)
    );
    if headed {
        out.push_str("<sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/><selection pane=\"bottomLeft\" activeCell=\"A2\" sqref=\"A2\"/></sheetView></sheetViews>");
    } else {
        out.push_str("<sheetViews><sheetView workbookViewId=\"0\"/></sheetViews>");
    }
    out.push_str("<sheetFormatPr defaultRowHeight=\"15\"/>");
    if headed {
        out.push_str("<cols>");
        for j in 0..sheet.columns.len() {
            let _ = write!(
                out,
                "<col min=\"{n}\" max=\"{n}\" width=\"{}\" customWidth=\"1\"/>",
                width(sheet, j),
                n = j + 1
            );
        }
        out.push_str("</cols>");
    }
    out.push_str("<sheetData>");
    let letters: Vec<String> = (0..sheet
        .rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .max(sheet.columns.len()))
        .map(column_letters)
        .collect();
    if headed {
        out.push_str("<row r=\"1\">");
        for (j, c) in sheet.columns.iter().enumerate() {
            let at = strings.at(&c.name);
            let _ = write!(
                out,
                "<c r=\"{}1\" s=\"1\" t=\"s\"><v>{at}</v></c>",
                letters[j]
            );
        }
        out.push_str("</row>");
    }
    // A number's style attribute a column, made once.
    let number_style: Vec<String> = sheet
        .columns
        .iter()
        .map(|c| match styles.number(c.decimals) {
            0 => String::new(),
            s => format!(" s=\"{s}\""),
        })
        .collect();
    for (i, r) in sheet.rows.iter().enumerate() {
        let mut row = String::new();
        push_count(&mut row, i + 2);
        out.push_str("<row r=\"");
        out.push_str(&row);
        out.push_str("\">");
        for (j, word) in r.iter().enumerate() {
            let kind = sheet.columns.get(j).map_or(CellKind::Auto, |c| c.kind);
            let at = cell(kind, word, true, fb);
            if at == Cell::Empty {
                continue;
            }
            out.push_str("<c r=\"");
            out.push_str(&letters[j]);
            out.push_str(&row);
            match at {
                Cell::Empty => {}
                Cell::Text(t) => {
                    out.push_str("\" t=\"s\"><v>");
                    push_count(&mut out, strings.at(t) as usize);
                }
                Cell::Number(t) => {
                    out.push('"');
                    out.push_str(number_style.get(j).map_or("", String::as_str));
                    out.push_str("><v>");
                    out.push_str(t);
                }
                Cell::Date(d) => {
                    out.push_str("\" s=\"2\"><v>");
                    let serial = dates::serial(d).and_then(|n| usize::try_from(n).ok());
                    push_count(&mut out, serial.unwrap_or(0));
                }
                Cell::Boolean(b) => {
                    out.push_str("\" t=\"b\"><v>");
                    out.push(if b { '1' } else { '0' });
                }
            }
            out.push_str("</v></c>");
        }
        out.push_str("</row>");
    }
    out.push_str("</sheetData>");
    if headed {
        let _ = write!(out, "<autoFilter ref=\"{}\"/>", range(sheet));
    }
    out.push_str("<pageMargins left=\"0.7\" right=\"0.7\" top=\"0.75\" bottom=\"0.75\" header=\"0.3\" footer=\"0.3\"/></worksheet>");
    out
}

pub(super) fn write(
    sheets: &[WorkbookSheet],
    names: &[String],
    fb: &mut Fallbacks,
) -> Result<Vec<u8>, String> {
    let styles = Styles::of(sheets);
    let mut strings = Strings::default();
    let parts: Vec<String> = sheets
        .iter()
        .map(|s| worksheet(s, &mut strings, &styles, fb))
        .collect();
    let n = sheets.len();
    let mut types = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>",
    );
    for k in 1..=n {
        let _ = write!(
            types,
            "<Override PartName=\"/xl/worksheets/sheet{k}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"
        );
    }
    types.push_str("<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/><Override PartName=\"/xl/sharedStrings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml\"/></Types>");
    let root = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"{PACKAGE_RELS}\"><Relationship Id=\"rId1\" Type=\"{RELS}/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>"
    );
    let mut book = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"{MAIN}\" xmlns:r=\"{RELS}\"><bookViews><workbookView/></bookViews><sheets>"
    );
    for (k, name) in names.iter().enumerate() {
        book.push_str("<sheet name=\"");
        escape(name, &mut book);
        let _ = write!(book, "\" sheetId=\"{}\" r:id=\"rId{}\"/>", k + 1, k + 1);
    }
    book.push_str("</sheets>");
    let filtered: Vec<usize> = (0..n).filter(|&k| !sheets[k].columns.is_empty()).collect();
    if !filtered.is_empty() {
        book.push_str("<definedNames>");
        for k in filtered {
            let _ = write!(
                book,
                "<definedName name=\"_xlnm._FilterDatabase\" localSheetId=\"{k}\" hidden=\"1\">"
            );
            let quoted = format!("'{}'", names[k].replace('\'', "''"));
            escape(&quoted, &mut book);
            let r = range(&sheets[k]);
            let (a, b) = r.split_once(':').unwrap_or((&r, &r));
            let absolute = |c: &str| {
                let digits = c.find(|ch: char| ch.is_ascii_digit()).unwrap_or(c.len());
                format!("${}${}", &c[..digits], &c[digits..])
            };
            let _ = write!(book, "!{}:{}</definedName>", absolute(a), absolute(b));
        }
        book.push_str("</definedNames>");
    }
    book.push_str("</workbook>");
    let mut rels = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"{PACKAGE_RELS}\">"
    );
    for k in 1..=n {
        let _ = write!(
            rels,
            "<Relationship Id=\"rId{k}\" Type=\"{RELS}/worksheet\" Target=\"worksheets/sheet{k}.xml\"/>"
        );
    }
    let _ = write!(
        rels,
        "<Relationship Id=\"rId{}\" Type=\"{RELS}/styles\" Target=\"styles.xml\"/><Relationship Id=\"rId{}\" Type=\"{RELS}/sharedStrings\" Target=\"sharedStrings.xml\"/></Relationships>",
        n + 1,
        n + 2
    );
    let shared = strings.xml();
    let style = styles.xml();
    let sheet_names: Vec<String> = (1..=n)
        .map(|k| format!("xl/worksheets/sheet{k}.xml"))
        .collect();
    // The sheets and their strings, the big parts, packed fast (docs/adr/0223 §8).
    let mut entries: Vec<(&str, &[u8], zip::Pack)> = vec![
        ("[Content_Types].xml", types.as_bytes(), zip::Pack::Best),
        ("_rels/.rels", root.as_bytes(), zip::Pack::Best),
        ("xl/workbook.xml", book.as_bytes(), zip::Pack::Best),
        (
            "xl/_rels/workbook.xml.rels",
            rels.as_bytes(),
            zip::Pack::Best,
        ),
        ("xl/styles.xml", style.as_bytes(), zip::Pack::Best),
        ("xl/sharedStrings.xml", shared.as_bytes(), zip::Pack::Fast),
    ];
    for (name, part) in sheet_names.iter().zip(&parts) {
        entries.push((name, part.as_bytes(), zip::Pack::Fast));
    }
    zip::write_with(&entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table_file::xlsx::unescape;

    #[test]
    fn texts_come_back_as_written() {
        for t in [
            "düz",
            "a&b<c>",
            "satır\nsonu\r\n",
            "_x0041_ ve _x005F_",
            "bel\u{7}",
            "\u{ffff}",
            "x_x00",
        ] {
            let mut out = String::new();
            text(t, &mut out);
            let read = roxmltree::Document::parse(&format!("<t>{out}</t>"))
                .expect("XML")
                .root_element()
                .text()
                .unwrap_or("")
                .to_owned();
            assert_eq!(unescape(&read), t, "{out}");
        }
    }
}

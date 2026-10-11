//! An OpenDocument spreadsheet (ODS; docs/adr/0223 §1): content.xml's
//! tables in order, each by its name, read as a stream.
//!
//! A row and a cell take their repeat counts (table:number-rows-repeated,
//! table:number-columns-repeated): an empty one only moves the place on (a
//! sheet's last row repeated a million times costs nothing), a filled one
//! is written that many times within the limits. Rows under header rows,
//! row groups and row lists are rows like any other; a covered cell (under
//! a merged one) is empty, as pandas reads it.
//!
//! A cell's words by its office:value-type: float, percentage and currency
//! its office:value as written; date its office:date-value (ISO, as
//! written, but a time's fraction of a second rounded to the second, as a
//! workbook's are); time its office:time-value as HH:MM:SS (PT12H30M00S is
//! 12:30:00; hours past 24 kept, seconds rounded); boolean DOĞRU or YANLIŞ;
//! string, or no type, its office:string-value if it has one, else the text
//! of its paragraphs (text:p and text:h, one line each; their white space as
//! written, as LibreOffice Calc reads it; text:s its count of spaces,
//! text:tab a tab, text:line-break a line break; spans and links by their
//! text; notes, annotations, drawings and sub-tables left out).

use std::borrow::Cow;

use kentos_contracts::TableSheet;

use super::dates::{civil_from_days, days_from_civil};
use super::{Grid, Limits, entry, unparsed, utf8};
use crate::xml::Unparsed;
use crate::xml_scan::{Event, Ns, Scan, Start};
use crate::zip;

const OFFICE: &str = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";
const TABLE: &str = "urn:oasis:names:tc:opendocument:xmlns:table:1.0";
const TEXT: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const URIS: [&str; 3] = [OFFICE, TABLE, TEXT];
const O: Ns = Ns::Is(0);
const T: Ns = Ns::Is(1);
const X: Ns = Ns::Is(2);

/// The most spaces one text:s gives (a hostile count must not allocate gigabytes).
const MOST_SPACES: usize = 10_000;

/// An OpenDocument spreadsheet's sheets, in its order.
pub(super) fn sheets(
    bytes: &[u8],
    listed: &[zip::Listed],
    limits: Limits,
) -> Result<Vec<TableSheet>, String> {
    let content = entry(bytes, listed, "content.xml")?.unwrap_or_default();
    let content = utf8(content, "content.xml")?;
    let sheets = read(&content, limits).map_err(|why| unparsed(why, "content.xml"))?;
    if sheets.is_empty() {
        return Err("Hesap tablosunda okunacak sayfa yok.".into());
    }
    Ok(sheets)
}

fn read(text: &str, limits: Limits) -> Result<Vec<TableSheet>, Unparsed> {
    let mut scan = Scan::new(text, &URIS);
    let mut sheets = Vec::new();
    loop {
        let name = match scan.event()? {
            None => break,
            Some(Event::Start(s)) if s.ns == T && s.local == "table" => {
                s.attr(T, "name")?.map(Cow::into_owned).unwrap_or_default()
            }
            Some(_) => continue,
        };
        sheets.push(table(&mut scan, name, limits)?);
    }
    Ok(sheets)
}

/// A repeat count: 1 when not given or not a count.
fn repeat(s: &Start<'_, '_>, name: &str) -> Result<usize, Unparsed> {
    Ok(s.attr(T, name)?
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(1))
}

enum Step {
    Row(usize),
    Cell { n: usize, words: Option<String> },
    Covered(usize),
    Paragraph,
    Push(String),
    Skip,
    End,
    Other,
}

/// A table just started, read to its end; once a filled row lies past
/// `limits`' rows, the rest of its rows are skipped (the sheet cut).
fn table(scan: &mut Scan<'_, '_>, name: String, limits: Limits) -> Result<TableSheet, Unparsed> {
    let level = scan.depth();
    let mut grid = Grid::new(limits);
    let mut row = 0usize;
    let mut stopped = false;
    loop {
        let step = match scan.event()? {
            None => return Err(Unparsed::NotXml),
            Some(Event::Start(s)) if s.ns == T && s.local == "table-row" => {
                Step::Row(repeat(&s, "number-rows-repeated")?)
            }
            Some(Event::Start(s)) if s.ns == T && s.local == "table" => Step::Skip,
            Some(Event::End { .. }) => Step::End,
            Some(_) => Step::Other,
        };
        match step {
            Step::Row(n) => {
                if stopped {
                    scan.skip_to_end()?;
                    continue;
                }
                let cells = row_cells(scan, limits)?;
                if !cells.is_empty() {
                    'rows: for k in 0..n {
                        let i = row.saturating_add(k);
                        for (j, words) in &cells {
                            if !grid.put(i, *j, words.clone()) {
                                stopped = true;
                                break 'rows;
                            }
                        }
                    }
                }
                row = row.saturating_add(n);
            }
            Step::Skip => scan.skip_to_end()?,
            Step::End if scan.depth() < level => break,
            _ => {}
        }
    }
    Ok(grid.sheet(name))
}

/// A date's ISO text with its seconds' fraction rounded to the second
/// (2024-10-01T23:59:59.9 is 2024-10-02T00:00:00); any other text as written.
fn date_value(v: &str) -> String {
    let parse = || -> Option<String> {
        let t = v.trim();
        let b = t.as_bytes();
        if b.len() <= 20
            || b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b'T'
            || b[13] != b':'
            || b[16] != b':'
            || b[19] != b'.'
        {
            return None;
        }
        if !t[20..].bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let num = |r: std::ops::Range<usize>| -> Option<i64> {
            let s = &t[r];
            s.bytes()
                .all(|c| c.is_ascii_digit())
                .then(|| s.parse().ok())?
        };
        let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
        let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
        let day = days_from_civil(y, m, d);
        if civil_from_days(day) != (y, m, d) || h > 23 || mi > 59 || s > 59 {
            return None;
        }
        let secs = h * 3600 + mi * 60 + s + i64::from(b[20] >= b'5');
        let (y, m, d) = civil_from_days(day + secs / 86_400);
        let secs = secs % 86_400;
        Some(format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
            secs / 3600,
            secs / 60 % 60,
            secs % 60
        ))
    };
    parse().unwrap_or_else(|| v.to_owned())
}

/// A time's ISO duration (PT12H30M00S) as HH:MM:SS; as written when it is not one.
fn clock(t: &str) -> String {
    let parse = || -> Option<String> {
        let rest = t.trim().strip_prefix("PT")?;
        let (h, rest) = rest.split_once('H')?;
        let (m, rest) = rest.split_once('M')?;
        let s = rest.strip_suffix('S')?;
        let (h, m): (u64, u64) = (h.parse().ok()?, m.parse().ok()?);
        let s: f64 = s.parse().ok()?;
        if !s.is_finite() || s < 0.0 {
            return None;
        }
        let total = h
            .checked_mul(3600)?
            .checked_add(m.checked_mul(60)?)?
            .checked_add(s.round() as u64)?;
        Some(format!(
            "{:02}:{:02}:{:02}",
            total / 3600,
            total / 60 % 60,
            total % 60
        ))
    };
    parse().unwrap_or_else(|| t.to_owned())
}

/// A row just started, read to its end: its filled cells' columns (from 0,
/// within `limits`) and words.
fn row_cells(scan: &mut Scan<'_, '_>, limits: Limits) -> Result<Vec<(usize, String)>, Unparsed> {
    let level = scan.depth();
    let mut col = 0usize;
    let mut out = Vec::new();
    loop {
        let step = match scan.event()? {
            None => return Err(Unparsed::NotXml),
            Some(Event::Start(s)) if s.ns == T && s.local == "covered-table-cell" => {
                Step::Covered(repeat(&s, "number-columns-repeated")?)
            }
            Some(Event::Start(s)) if s.ns == T && s.local == "table-cell" => {
                let n = repeat(&s, "number-columns-repeated")?;
                let kind = s.attr(O, "value-type")?;
                let words = match kind.as_deref() {
                    Some("float" | "percentage" | "currency") => {
                        s.attr(O, "value")?.map(Cow::into_owned)
                    }
                    Some("date") => s.attr(O, "date-value")?.map(|d| date_value(&d)),
                    Some("time") => s.attr(O, "time-value")?.map(|t| clock(&t)),
                    Some("boolean") => s.attr(O, "boolean-value")?.map(|b| match b.trim() {
                        "true" => "DOĞRU".to_owned(),
                        "false" => "YANLIŞ".to_owned(),
                        other => other.to_owned(),
                    }),
                    _ => s.attr(O, "string-value")?.map(Cow::into_owned),
                };
                Step::Cell { n, words }
            }
            Some(Event::End { .. }) => Step::End,
            Some(_) => Step::Other,
        };
        match step {
            Step::Covered(n) => {
                scan.skip_to_end()?;
                col = col.saturating_add(n);
            }
            Step::Cell { n, words } => {
                let words = match words {
                    Some(w) => {
                        scan.skip_to_end()?;
                        w
                    }
                    None => paragraphs(scan)?,
                };
                if !words.is_empty() && col < limits.columns {
                    let upto = col.saturating_add(n).min(limits.columns);
                    for j in col..upto {
                        out.push((j, words.clone()));
                    }
                }
                col = col.saturating_add(n);
            }
            Step::End if scan.depth() < level => return Ok(out),
            _ => {}
        }
    }
}

/// A cell just started, read to its end: its paragraphs' text, a line each.
fn paragraphs(scan: &mut Scan<'_, '_>) -> Result<String, Unparsed> {
    let level = scan.depth();
    let mut out = String::new();
    let mut first = true;
    loop {
        let step = match scan.event()? {
            None => return Err(Unparsed::NotXml),
            Some(Event::Start(s)) if s.ns == X && (s.local == "p" || s.local == "h") => {
                Step::Paragraph
            }
            Some(Event::Start(_)) => Step::Skip,
            Some(Event::End { .. }) => Step::End,
            Some(Event::Text(_)) => Step::Other,
        };
        match step {
            Step::Paragraph => {
                if !first {
                    out.push('\n');
                }
                first = false;
                paragraph(scan, &mut out)?;
            }
            Step::Skip => scan.skip_to_end()?,
            Step::End if scan.depth() < level => return Ok(out),
            _ => {}
        }
    }
}

/// A paragraph just started, its text added to `out` up to its end. Its
/// white space is kept as written, as LibreOffice Calc reads a cell (not
/// folded as ODF §6.1.2 says for documents; writers put runs of spaces in
/// text:s, tabs in text:tab).
fn paragraph(scan: &mut Scan<'_, '_>, out: &mut String) -> Result<(), Unparsed> {
    let level = scan.depth();
    loop {
        let step = match scan.event()? {
            None => return Err(Unparsed::NotXml),
            Some(Event::Text(t)) => {
                out.push_str(&t);
                Step::Other
            }
            Some(Event::Start(s)) if s.ns == X => match s.local {
                "s" => {
                    let c = s
                        .attr(X, "c")?
                        .and_then(|c| c.trim().parse::<usize>().ok())
                        .unwrap_or(1)
                        .min(MOST_SPACES);
                    Step::Push(" ".repeat(c))
                }
                "tab" => Step::Push("\t".to_owned()),
                "line-break" => Step::Push("\n".to_owned()),
                "note" | "ruby-text" => Step::Skip,
                _ => Step::Other,
            },
            Some(Event::Start(_)) => Step::Skip,
            Some(Event::End { .. }) => Step::End,
        };
        match step {
            Step::Push(p) => out.push_str(&p),
            Step::Skip => scan.skip_to_end()?,
            Step::End if scan.depth() < level => return Ok(()),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = r#"<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"><office:body><office:spreadsheet>"#;
    const TAIL: &str = "</office:spreadsheet></office:body></office:document-content>";

    fn sheets_of(tables: &str, limits: Limits) -> Vec<TableSheet> {
        read(&format!("{HEAD}{tables}{TAIL}"), limits).expect("XML")
    }

    #[test]
    fn cells_read_by_their_kinds() {
        let s = sheets_of(
            r#"<table:table table:name="Noktalar"><table:table-column table:number-columns-repeated="3"/>
<table:table-row><table:table-cell office:value-type="string"><text:p>Ad</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="452345.123"><text:p>452345,12</text:p></table:table-cell><table:table-cell office:value-type="boolean" office:boolean-value="true"><text:p>DOĞRU</text:p></table:table-cell></table:table-row>
<table:table-row><table:table-cell office:value-type="date" office:date-value="2024-10-01"><text:p>01.10.2024</text:p></table:table-cell><table:table-cell office:value-type="time" office:time-value="PT36H05M09.6S"/><table:table-cell office:value-type="percentage" office:value="0.25"><text:p>%25</text:p></table:table-cell></table:table-row>
<table:table-row><table:table-cell><text:p>a<text:s text:c="3"/>b<text:tab/>c<text:line-break/>d<text:span>e</text:span><text:note><text:note-body><text:p>not</text:p></text:note-body></text:note></text:p><text:p>ikinci</text:p><office:annotation><text:p>yorum</text:p></office:annotation></table:table-cell></table:table-row>
</table:table>"#,
            Limits::TABLE,
        );
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name.as_deref(), Some("Noktalar"));
        assert_eq!(
            s[0].rows,
            vec![
                vec!["Ad".to_owned(), "452345.123".into(), "DOĞRU".into()],
                vec!["2024-10-01".to_owned(), "36:05:10".into(), "0.25".into()],
                vec!["a   b\tc\nde\nikinci".to_owned()],
            ]
        );
        assert!(!s[0].cut);
    }

    #[test]
    fn repeats_and_covered_cells_take_their_places() {
        let s = sheets_of(
            r#"<table:table table:name="A"><table:table-header-rows><table:table-row><table:table-cell table:number-columns-repeated="2"><text:p>x</text:p></table:table-cell><table:table-cell table:number-columns-spanned="2"><text:p>y</text:p></table:table-cell><table:covered-table-cell><text:p>gizli</text:p></table:covered-table-cell><table:table-cell><text:p>z</text:p></table:table-cell></table:table-row></table:table-header-rows>
<table:table-row table:number-rows-repeated="2"><table:table-cell table:number-columns-repeated="2"/><table:table-cell><text:p>r</text:p></table:table-cell></table:table-row>
<table:table-row-group><table:table-row table:number-rows-repeated="3"><table:table-cell table:number-columns-repeated="1024"/></table:table-row></table:table-row-group>
<table:table-row><table:table-cell><draw:frame><draw:text-box><text:p>çizim</text:p></draw:text-box></draw:frame><text:p>son</text:p></table:table-cell></table:table-row>
<table:table-row table:number-rows-repeated="1048570"><table:table-cell table:number-columns-repeated="16384"/></table:table-row>
</table:table><table:table table:name="B"/>"#,
            Limits::TABLE,
        );
        assert_eq!(s.len(), 2);
        assert_eq!(
            s[0].rows,
            vec![
                vec![
                    "x".to_owned(),
                    "x".into(),
                    "y".into(),
                    String::new(),
                    "z".into()
                ],
                vec![String::new(), String::new(), "r".into()],
                vec![String::new(), String::new(), "r".into()],
                vec![],
                vec![],
                vec![],
                vec!["son".to_owned()],
            ]
        );
        assert_eq!(s[1].name.as_deref(), Some("B"));
        assert!(s[1].rows.is_empty());
    }

    #[test]
    fn a_sheet_past_the_limits_is_cut_and_read_no_further() {
        let limits = Limits {
            rows: 3,
            columns: 2,
        };
        let s = sheets_of(
            r#"<table:table table:name="A"><table:table-row table:number-rows-repeated="1000000"><table:table-cell table:number-columns-repeated="5"><text:p>v</text:p></table:table-cell></table:table-row></table:table><table:table table:name="B"><table:table-row><table:table-cell><text:p>w</text:p></table:table-cell></table:table-row></table:table>"#,
            limits,
        );
        assert_eq!(s[0].rows, vec![vec!["v".to_owned(), "v".into()]; 3]);
        assert!(s[0].cut);
        assert_eq!(s[1].rows, vec![vec!["w".to_owned()]]);
        assert!(!s[1].cut);
    }

    #[test]
    fn white_space_is_kept_as_calc_reads_it() {
        let s = sheets_of(
            "<table:table table:name=\"A\"><table:table-row><table:table-cell><text:p>\n bir  iki\t<text:span> üç</text:span><text:s/> dört</text:p></table:table-cell></table:table-row></table:table>",
            Limits::TABLE,
        );
        assert_eq!(s[0].rows, vec![vec!["\n bir  iki\t üç  dört".to_owned()]]);
    }

    #[test]
    fn date_values_round_to_the_second() {
        assert_eq!(
            date_value("2024-10-01T23:59:59.9136"),
            "2024-10-02T00:00:00"
        );
        assert_eq!(date_value("2024-12-31T23:59:59.5"), "2025-01-01T00:00:00");
        assert_eq!(
            date_value("2024-10-01T12:30:00.4999"),
            "2024-10-01T12:30:00"
        );
        assert_eq!(date_value("2024-10-01"), "2024-10-01");
        assert_eq!(date_value("2024-10-01T12:30:00"), "2024-10-01T12:30:00");
        assert_eq!(date_value("2023-02-29T00:00:00.5"), "2023-02-29T00:00:00.5");
        assert_eq!(date_value("10000-01-02"), "10000-01-02");
        assert_eq!(
            date_value("2024-10-01T12:30:00.5Z"),
            "2024-10-01T12:30:00.5Z"
        );
    }

    #[test]
    fn times_read_as_clocks() {
        assert_eq!(clock("PT12H30M00S"), "12:30:00");
        assert_eq!(clock("PT00H00M59.5S"), "00:01:00");
        assert_eq!(clock("P1D"), "P1D");
    }
}

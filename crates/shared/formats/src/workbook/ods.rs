//! An OpenDocument spreadsheet written (docs/adr/0223 §2): mimetype first
//! and stored, its manifest, content.xml (the cell and column styles, the
//! tables, a filter range a sheet), styles.xml and settings.xml (the names'
//! row frozen), dated so the same sheets give the same bytes.
//!
//! A text is a cell's paragraphs: a line each (CR LF, CR and LF break
//! lines; a CR comes back as a LF), a run of spaces, or one at a line's
//! start, as text:s, a tab as text:tab, so a reader that folds white space
//! (ODF's rule for documents) reads what LibreOffice Calc reads. ODF has no
//! way to hold a control character: it is left out, and the report says
//! so. A number's cell holds its text as its value; a date its ISO day,
//! shown GG.AA.YYYY; yes or no a boolean, shown DOĞRU or YANLIŞ.

use std::fmt::Write as _;

use kentos_contracts::{CellKind, WorkbookSheet};

use super::{Cell, Fallbacks, cell, column_letters, escape, width};
use crate::report::Report;
use crate::zip;

const OFFICE: &str = "urn:oasis:names:tc:opendocument:xmlns:office:1.0";
const STYLE: &str = "urn:oasis:names:tc:opendocument:xmlns:style:1.0";
const TEXT: &str = "urn:oasis:names:tc:opendocument:xmlns:text:1.0";
const TABLE: &str = "urn:oasis:names:tc:opendocument:xmlns:table:1.0";
const FO: &str = "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0";
const NUMBER: &str = "urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0";
const CONFIG: &str = "urn:oasis:names:tc:opendocument:xmlns:config:1.0";
const MIMETYPE: &str = "application/vnd.oasis.opendocument.spreadsheet";

/// What a text's writing left out or changed.
#[derive(Default)]
struct Lost {
    controls: u32,
    returns: u32,
}

/// One line of a text as a paragraph's content, written in runs between the
/// bytes that need more (a sheet writes millions of words).
fn line(l: &str, out: &mut String, lost: &mut Lost) {
    let b = l.as_bytes();
    let (mut from, mut i) = (0, 0);
    while i < b.len() {
        // What takes the place of the bytes from i to `next`: a space's run,
        // markup, an entity, or nothing (a control character left out).
        let (put, next): (Option<&str>, usize) = match b[i] {
            b' ' => {
                let run = b[i..].iter().take_while(|&&c| c == b' ').count();
                if run == 1 && i > 0 {
                    i += 1;
                    continue;
                }
                out.push_str(&l[from..i]);
                if run == 1 {
                    out.push_str("<text:s/>");
                } else {
                    let _ = write!(out, "<text:s text:c=\"{run}\"/>");
                }
                i += run;
                from = i;
                continue;
            }
            b'\t' => (Some("<text:tab/>"), i + 1),
            b'&' => (Some("&amp;"), i + 1),
            b'<' => (Some("&lt;"), i + 1),
            b'>' => (Some("&gt;"), i + 1),
            0x00..=0x1f | 0x7f => (None, i + 1),
            // U+0080 to U+009F, the other control characters.
            0xc2 if b.get(i + 1).is_some_and(|c| (0x80..=0x9f).contains(c)) => (None, i + 2),
            // U+FFFE and U+FFFF, no characters of XML's.
            0xef if b.get(i + 1) == Some(&0xbf) && matches!(b.get(i + 2), Some(0xbe | 0xbf)) => {
                (None, i + 3)
            }
            _ => {
                i += 1;
                continue;
            }
        };
        out.push_str(&l[from..i]);
        match put {
            Some(text) => out.push_str(text),
            None => lost.controls += 1,
        }
        i = next;
        from = i;
    }
    out.push_str(&l[from..]);
}

/// A text as a cell's paragraphs.
fn paragraphs(t: &str, out: &mut String, lost: &mut Lost) {
    let put = |l: &str, out: &mut String, lost: &mut Lost| {
        if l.is_empty() {
            out.push_str("<text:p/>");
        } else {
            out.push_str("<text:p>");
            line(l, out, lost);
            out.push_str("</text:p>");
        }
    };
    if t.contains('\r') {
        lost.returns += 1;
        let normal = t.replace("\r\n", "\n").replace('\r', "\n");
        normal.split('\n').for_each(|l| put(l, out, lost));
    } else {
        t.split('\n').for_each(|l| put(l, out, lost));
    }
}

/// An ISO day shown as GG.AA.YYYY.
fn shown(d: &str, out: &mut String) {
    out.push_str(&d[8..10]);
    out.push('.');
    out.push_str(&d[5..7]);
    out.push('.');
    out.push_str(&d[0..4]);
}

/// A sheet's address for a filter range, its name quoted.
fn address(name: &str, sheet: &WorkbookSheet) -> String {
    let width = sheet
        .rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .max(sheet.columns.len())
        .max(1);
    let quoted = format!("'{}'", name.replace('\'', "''"));
    format!(
        "{quoted}.A1:{quoted}.{}{}",
        column_letters(width - 1),
        sheet.rows.len() + 1
    )
}

/// The fraction digits the number columns show, in order.
fn decimals_of(sheets: &[WorkbookSheet]) -> Vec<u32> {
    let mut d: Vec<u32> = sheets
        .iter()
        .flat_map(|s| &s.columns)
        .filter(|c| c.kind == CellKind::Number)
        .filter_map(|c| c.decimals.map(|d| d.min(15)))
        .collect();
    d.sort_unstable();
    d.dedup();
    d
}

fn content(
    sheets: &[WorkbookSheet],
    names: &[String],
    fb: &mut Fallbacks,
    lost: &mut Lost,
) -> String {
    let decimals = decimals_of(sheets);
    let widths: Vec<Vec<f64>> = sheets
        .iter()
        .map(|s| (0..s.columns.len()).map(|j| width(s, j)).collect())
        .collect();
    let mut distinct: Vec<f64> = widths.iter().flatten().copied().collect();
    distinct.sort_by(f64::total_cmp);
    distinct.dedup();
    let rows: usize = sheets
        .iter()
        .map(|s| s.rows.len() * s.columns.len().max(1))
        .sum();
    let mut out = String::with_capacity(2048 + rows * 64);
    let _ = write!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-content xmlns:office=\"{OFFICE}\" xmlns:style=\"{STYLE}\" xmlns:text=\"{TEXT}\" xmlns:table=\"{TABLE}\" xmlns:fo=\"{FO}\" xmlns:number=\"{NUMBER}\" office:version=\"1.3\"><office:automatic-styles>"
    );
    for (k, w) in distinct.iter().enumerate() {
        let _ = write!(
            out,
            "<style:style style:name=\"co{}\" style:family=\"table-column\"><style:table-column-properties fo:break-before=\"auto\" style:column-width=\"{:.3}cm\"/></style:style>",
            k + 1,
            w * 0.21
        );
    }
    out.push_str("<number:date-style style:name=\"N1\"><number:day number:style=\"long\"/><number:text>.</number:text><number:month number:style=\"long\"/><number:text>.</number:text><number:year number:style=\"long\"/></number:date-style>");
    for d in &decimals {
        let _ = write!(
            out,
            "<number:number-style style:name=\"N2_{d}\"><number:number number:decimal-places=\"{d}\" number:min-decimal-places=\"{d}\" number:min-integer-digits=\"1\"/></number:number-style>"
        );
    }
    out.push_str("<style:style style:name=\"ceH\" style:family=\"table-cell\" style:parent-style-name=\"Default\"><style:text-properties fo:font-weight=\"bold\"/></style:style>");
    out.push_str("<style:style style:name=\"ceD\" style:family=\"table-cell\" style:parent-style-name=\"Default\" style:data-style-name=\"N1\"/>");
    for d in &decimals {
        let _ = write!(
            out,
            "<style:style style:name=\"ceN{d}\" style:family=\"table-cell\" style:parent-style-name=\"Default\" style:data-style-name=\"N2_{d}\"/>"
        );
    }
    out.push_str("</office:automatic-styles><office:body><office:spreadsheet>");
    for ((s, name), ws) in sheets.iter().zip(names).zip(&widths) {
        out.push_str("<table:table table:name=\"");
        escape(name, &mut out);
        out.push_str("\">");
        for w in ws {
            let k = distinct.iter().position(|d| d == w).unwrap_or(0) + 1;
            let _ = write!(
                out,
                "<table:table-column table:style-name=\"co{k}\" table:default-cell-style-name=\"Default\"/>"
            );
        }
        if s.columns.is_empty() && s.rows.is_empty() {
            out.push_str(
                "<table:table-column/><table:table-row><table:table-cell/></table:table-row>",
            );
        }
        if !s.columns.is_empty() {
            out.push_str("<table:table-row>");
            for c in &s.columns {
                out.push_str(
                    "<table:table-cell table:style-name=\"ceH\" office:value-type=\"string\">",
                );
                paragraphs(&c.name, &mut out, lost);
                out.push_str("</table:table-cell>");
            }
            out.push_str("</table:table-row>");
        }
        // A number's style a column, made once.
        let number_style: Vec<String> = s
            .columns
            .iter()
            .map(|c| {
                c.decimals
                    .map(|d| format!(" table:style-name=\"ceN{}\"", d.min(15)))
                    .unwrap_or_default()
            })
            .collect();
        for r in &s.rows {
            out.push_str("<table:table-row>");
            let mut empty = 0usize;
            for (j, word) in r.iter().enumerate() {
                let column = s.columns.get(j);
                let kind = column.map_or(CellKind::Auto, |c| c.kind);
                let at = cell(kind, word, false, fb);
                if at == Cell::Empty {
                    empty += 1;
                    continue;
                }
                match empty {
                    0 => {}
                    1 => out.push_str("<table:table-cell/>"),
                    n => {
                        let _ = write!(
                            out,
                            "<table:table-cell table:number-columns-repeated=\"{n}\"/>"
                        );
                    }
                }
                empty = 0;
                match at {
                    Cell::Empty => {}
                    Cell::Text(t) => {
                        out.push_str("<table:table-cell office:value-type=\"string\">");
                        paragraphs(t, &mut out, lost);
                        out.push_str("</table:table-cell>");
                    }
                    Cell::Number(t) => {
                        out.push_str("<table:table-cell");
                        out.push_str(number_style.get(j).map_or("", String::as_str));
                        out.push_str(" office:value-type=\"float\" office:value=\"");
                        out.push_str(t);
                        out.push_str("\"><text:p>");
                        out.push_str(t);
                        out.push_str("</text:p></table:table-cell>");
                    }
                    Cell::Date(d) => {
                        out.push_str("<table:table-cell table:style-name=\"ceD\" office:value-type=\"date\" office:date-value=\"");
                        out.push_str(d);
                        out.push_str("\"><text:p>");
                        shown(d, &mut out);
                        out.push_str("</text:p></table:table-cell>");
                    }
                    Cell::Boolean(b) => out.push_str(if b {
                        "<table:table-cell office:value-type=\"boolean\" office:boolean-value=\"true\"><text:p>DOĞRU</text:p></table:table-cell>"
                    } else {
                        "<table:table-cell office:value-type=\"boolean\" office:boolean-value=\"false\"><text:p>YANLIŞ</text:p></table:table-cell>"
                    }),
                }
            }
            if r.iter().all(String::is_empty) {
                out.push_str("<table:table-cell/>");
            }
            out.push_str("</table:table-row>");
        }
        out.push_str("</table:table>");
    }
    let filtered: Vec<usize> = (0..sheets.len())
        .filter(|&k| !sheets[k].columns.is_empty())
        .collect();
    if !filtered.is_empty() {
        out.push_str("<table:database-ranges>");
        for k in filtered {
            let _ = write!(
                out,
                "<table:database-range table:name=\"__Anonymous_Sheet_DB__{k}\" table:target-range-address=\""
            );
            escape(&address(&names[k], &sheets[k]), &mut out);
            out.push_str("\" table:display-filter-buttons=\"true\"/>");
        }
        out.push_str("</table:database-ranges>");
    }
    out.push_str("</office:spreadsheet></office:body></office:document-content>");
    out
}

/// The view settings: each sheet with columns its names' row frozen.
fn settings(sheets: &[WorkbookSheet], names: &[String]) -> String {
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-settings xmlns:office=\"{OFFICE}\" xmlns:config=\"{CONFIG}\" office:version=\"1.3\"><office:settings><config:config-item-set config:name=\"ooo:view-settings\"><config:config-item-map-indexed config:name=\"Views\"><config:config-item-map-entry><config:config-item config:name=\"ViewId\" config:type=\"string\">view1</config:config-item><config:config-item-map-named config:name=\"Tables\">"
    );
    for (s, name) in sheets.iter().zip(names) {
        if s.columns.is_empty() {
            continue;
        }
        out.push_str("<config:config-item-map-entry config:name=\"");
        escape(name, &mut out);
        out.push_str("\">");
        for (item, kind, value) in [
            ("HorizontalSplitMode", "short", 0),
            ("VerticalSplitMode", "short", 2),
            ("HorizontalSplitPosition", "int", 0),
            ("VerticalSplitPosition", "int", 1),
            ("ActiveSplitRange", "short", 2),
            ("PositionLeft", "int", 0),
            ("PositionRight", "int", 0),
            ("PositionTop", "int", 0),
            ("PositionBottom", "int", 1),
        ] {
            let _ = write!(
                out,
                "<config:config-item config:name=\"{item}\" config:type=\"{kind}\">{value}</config:config-item>"
            );
        }
        out.push_str("</config:config-item-map-entry>");
    }
    out.push_str("</config:config-item-map-named></config:config-item-map-entry></config:config-item-map-indexed></config:config-item-set></office:settings></office:document-settings>");
    out
}

pub(super) fn write(
    sheets: &[WorkbookSheet],
    names: &[String],
    fb: &mut Fallbacks,
    rep: &mut Report,
) -> Result<Vec<u8>, String> {
    let mut lost = Lost::default();
    let content = content(sheets, names, fb, &mut lost);
    let settings = settings(sheets, names);
    let styles = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<office:document-styles xmlns:office=\"{OFFICE}\" xmlns:style=\"{STYLE}\" office:version=\"1.3\"><office:styles><style:default-style style:family=\"table-cell\"/><style:style style:name=\"Default\" style:family=\"table-cell\"/></office:styles></office:document-styles>"
    );
    let manifest = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.3\"><manifest:file-entry manifest:full-path=\"/\" manifest:version=\"1.3\" manifest:media-type=\"{MIMETYPE}\"/><manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/><manifest:file-entry manifest:full-path=\"styles.xml\" manifest:media-type=\"text/xml\"/><manifest:file-entry manifest:full-path=\"settings.xml\" manifest:media-type=\"text/xml\"/></manifest:manifest>"
    );
    rep.note_n(
        "Denetim karakteri",
        "ODS denetim karakterlerini (sekme ve satır sonu dışındakileri) tutamaz; yazılmadı",
        0,
        lost.controls,
    );
    rep.note_n(
        "Satır başı",
        "ODS'de satır başı (CR) yok: satır sonu olarak yazıldı",
        0,
        lost.returns,
    );
    zip::write_with(&[
        ("mimetype", MIMETYPE.as_bytes(), zip::Pack::Stored),
        (
            "META-INF/manifest.xml",
            manifest.as_bytes(),
            zip::Pack::Best,
        ),
        ("content.xml", content.as_bytes(), zip::Pack::Fast),
        ("styles.xml", styles.as_bytes(), zip::Pack::Best),
        ("settings.xml", settings.as_bytes(), zip::Pack::Best),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_are_paragraphs_with_their_spaces() {
        let mut lost = Lost::default();
        let mut out = String::new();
        paragraphs(" a  b\tc \r\nd<e>\u{7}", &mut out, &mut lost);
        assert_eq!(
            out,
            "<text:p><text:s/>a<text:s text:c=\"2\"/>b<text:tab/>c </text:p><text:p>d&lt;e&gt;</text:p>"
        );
        assert_eq!((lost.controls, lost.returns), (1, 1));
    }
}

//! `file.export.table` v1 (docs/adr/0733 §7): a table's cells as CSV, XLSX or ODS. A schedule is written as the
//! document writes it now: its headings, subtotals and total with its data rows (`groups`), its worked-out numbers as
//! shown or read again from the drawing and rounded to 15 significant digits (`numbers`); a column written to its
//! objects (a number, a name, a finish …) always as written. Another table's cells as they stand.

use kentos_commands::{
    CellKind, Entity, ExportTable, ScheduleField, TableExported, TableNumbers, TableSource, WorkbookColumn,
    WorkbookFormat, WorkbookSheet, WorkbookWriteInput,
};
use kentos_domain::{Document, Uuid};

use crate::codes;

/// What `export` made: the file's bytes and what it says.
pub struct Made {
    pub bytes: Vec<u8>,
    pub output: TableExported,
}

/// A refusal: its code, its words and the input's path.
pub type Refusal = (String, String, String);

fn refuse(code: &str, words: impl Into<String>, at: &str) -> Refusal {
    (code.to_owned(), words.into(), at.to_owned())
}

/// `x` to 15 significant digits (what a spreadsheet keeps), its shortest text.
pub fn fifteen(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return if x == 0.0 { "0".into() } else { x.to_string() };
    }
    let rounded: f64 = format!("{x:.14e}").parse().unwrap_or(x);
    format!("{rounded}")
}

/// The fraction digits a schedule's numeric field shows (the project's), none for a word.
fn decimals(doc: &Document, field: ScheduleField) -> Option<u32> {
    let s = doc.settings();
    if field.length() {
        Some(s.length_decimals)
    } else {
        match field {
            ScheduleField::Area | ScheduleField::Volume => Some(s.area_decimals),
            ScheduleField::Count => Some(0),
            _ => None,
        }
    }
}

/// The file `input` asks for, made from `doc` (nothing written).
pub fn export(doc: &Document, input: &ExportTable) -> Result<Made, Refusal> {
    let lower = input.path.to_lowercase();
    let format = if lower.ends_with(".csv") {
        WorkbookFormat::Csv
    } else if lower.ends_with(".xlsx") {
        WorkbookFormat::Xlsx
    } else if lower.ends_with(".ods") {
        WorkbookFormat::Ods
    } else {
        return Err(refuse(
            codes::INVALID_FILE_PATH,
            "Tablo .csv, .xlsx ya da .ods dosyasına yazılır; yolun uzantısını verin.",
            "path",
        ));
    };
    if !crate::checks::is_uid_text(&input.uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!("“{}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin.", input.uid),
            "uid",
        ));
    }
    let Some(e) = Uuid::parse_str(&input.uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
        return Err(refuse(codes::ENTITY_NOT_FOUND, format!("“{}” kimlikli nesne çizimde yok.", input.uid), "uid"));
    };
    let Entity::Table(t) = e else {
        return Err(refuse(codes::NOT_A_TABLE, "Nesne bir tablo değil; bir tablonun kimliğini verin.", "uid"));
    };
    let groups = input.groups.unwrap_or(true);
    let mut notes: Vec<String> = Vec::new();
    let (name, columns, rows) = match &t.source {
        Some(TableSource::Schedule { of, columns, filter, sort, group, total, .. }) => {
            let written = doc.schedule_rows(*of, columns, filter.as_ref(), sort, *group, *total);
            let last = written.rows.len().saturating_sub(1);
            let mut rows: Vec<Vec<String>> = Vec::new();
            for (i, cells) in written.rows.iter().enumerate().skip(1) {
                let data = written.members.get(i - 1).is_some_and(|m| !m.is_empty());
                let is_total = *total && i == last;
                if !data && !groups {
                    continue;
                }
                let row = cells
                    .iter()
                    .enumerate()
                    .map(|(j, text)| {
                        let worked_out = !crate::schedules::editable(*of, columns[j].field);
                        match (input.numbers, worked_out, written.numbers.get(i).and_then(|r| r.get(j)).copied().flatten()) {
                            // A subtotal's or the total's label stays words.
                            (TableNumbers::Exact, true, Some(x)) if data || is_total || !text.ends_with("toplamı") => fifteen(x),
                            _ => text.clone(),
                        }
                    })
                    .collect();
                rows.push(row);
            }
            if input.numbers == TableNumbers::Exact {
                notes.push("Hesaplanan sayılar çizimden yeniden okunup 15 anlamlı basamağa yuvarlandı; kaynaklar değişmedi.".into());
            } else {
                notes.push("Sayılar çizelgede görüldüğü gibi, projenin basamağıyla yazıldı; kaynaklar değişmedi.".into());
            }
            let heads: Vec<WorkbookColumn> = columns
                .iter()
                .zip(&written.rows[0])
                .map(|(c, name)| {
                    let numeric = kentos_domain::schedules::numeric(c.field);
                    WorkbookColumn {
                        name: name.clone(),
                        kind: if numeric { CellKind::Number } else { CellKind::Text },
                        decimals: match input.numbers {
                            TableNumbers::Shown if numeric => decimals(doc, c.field),
                            _ => None,
                        },
                    }
                })
                .collect();
            (of.label().to_owned(), heads, rows)
        }
        _ => {
            let width = t.cells.iter().map(Vec::len).max().unwrap_or(0);
            let (heads, rest): (Vec<String>, &[Vec<String>]) = if t.header && !t.cells.is_empty() {
                (t.cells[0].clone(), &t.cells[1..])
            } else {
                ((1..=width).map(|i| format!("Sütun {i}")).collect(), &t.cells[..])
            };
            let heads = (0..width)
                .map(|j| WorkbookColumn {
                    name: heads.get(j).cloned().unwrap_or_default(),
                    kind: CellKind::Auto,
                    decimals: None,
                })
                .collect();
            ("Tablo".to_owned(), heads, rest.to_vec())
        }
    };
    let width = columns.len();
    let rows: Vec<Vec<String>> = rows
        .into_iter()
        .map(|mut r| {
            r.resize(width, String::new());
            r
        })
        .collect();
    let (bytes, report) = kentos_formats::workbook::write(&WorkbookWriteInput {
        format,
        sheets: vec![WorkbookSheet { name, columns, rows: rows.clone() }],
    })
    .map_err(|why| refuse(codes::FILE_UNWRITABLE, why, "path"))?;
    notes.extend(report.notes.iter().map(|n| n.what.clone()));
    Ok(Made {
        output: TableExported {
            path: input.path.clone(),
            rows: u32::try_from(rows.len()).unwrap_or(u32::MAX),
            columns: u32::try_from(width).unwrap_or(u32::MAX),
            numbers: input.numbers,
            notes,
            bytes: bytes.len() as u64,
        },
        bytes,
    })
}

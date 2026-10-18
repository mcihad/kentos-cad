//! `cad.tables.edit` v1 (docs/adr/0327): a table's cells, rows and columns edited in order (Tabloyu düzenle), cell
//! styles, formulas and fields; tables written again from their sources (Tabloyu güncelle), linked to a file (Dosyaya
//! bağla) or let go of it; each as one undo step. The cells and merged ranges change by the core's rules
//! (`ops::table_edit`), the styles, formulas' references and fields move with the rows and columns
//! (`table_formula::shift`); a table's computed cells are written by the document's rule (`table_worked_out`). The web
//! is paused (docs/adr/0228): the desktop's handler alone, passing `fixtures/commands/v1/cad.tables.edit.json`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use kentos_contracts::table_formula::{self, Axis};
use kentos_contracts::{
    CellField, CellFormula, CellRange, CellStyle, CommandResult, CommandWarning, Entity, FieldFormat,
    MAX_TABLE_SIZE, SourceOverrides, TableEdit, TableEntity, TableRefresh, TableReport, TableSource,
    TablesEdit, TablesEdited, TablesOperation, TablesPlan, cells_problem,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::table::{LEAST, ScheduleKind};
use kentos_geometry_core::ops::table_edit::{self as core_edit, Edit, grown, shrunk};
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::geometry::{core_table, drawing_font, with_shape};
pub use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The undo step's name: the operation's.
fn label(op: &TablesOperation) -> &'static str {
    match op {
        TablesOperation::Edit { .. } => "Tabloyu düzenle",
        TablesOperation::Refresh { .. } => "Tabloyu güncelle",
        TablesOperation::Link { .. } => "Dosyaya bağla",
        TablesOperation::Unlink { .. } => "Kaynağı bırak",
    }
}

/// The table `uid` names, on an unlocked layer.
fn table_at(doc: &Document, uid: &str, path: &str) -> Result<(Slot, Uuid, TableEntity), Stop> {
    if !is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            path,
        ));
    }
    let Some((slot, u)) = Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u).map(|s| (s, u)))
    else {
        return Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!(
                "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
            ),
            path,
        ));
    };
    let Some(Entity::Table(t)) = doc.get(slot) else {
        return Err(fail(
            codes::NOT_A_TABLE,
            "Verilen nesne bir tablo değil; bu komut yalnız tabloları değiştirir.".into(),
            path,
        ));
    };
    if doc.layers().is_locked(&t.base.layer_id) {
        let layers = doc.layers();
        let name = layers
            .get(&t.base.layer_id)
            .map_or(t.base.layer_id.as_str(), |n| n.name.as_str());
        return Err(fail(
            codes::LAYER_LOCKED,
            format!("“{name}” katmanı kilitli; üzerindeki tablo değişmez. Kilidi Katmanlar panelinden açın."),
            path,
        ));
    }
    Ok((slot, u, t.clone()))
}

fn tables_at(doc: &Document, uids: &[String]) -> Result<Vec<(Slot, Uuid, TableEntity)>, Stop> {
    if uids.is_empty() {
        return Err(fail(
            codes::NO_ENTITIES,
            "Tablo verilmedi. En az bir tablonun kimliğini verin.".into(),
            "operation.uids",
        ));
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (i, u) in uids.iter().enumerate() {
        if seen.insert(u.as_str()) {
            out.push(table_at(doc, u, &format!("operation.uids[{i}]"))?);
        }
    }
    Ok(out)
}

// ── Edits ──────────────────────────────────────────────────────────────────

fn out_of_range(words: String, path: &str) -> Stop {
    fail(codes::CELL_OUT_OF_RANGE, words, path)
}

fn invalid(words: String, path: &str) -> Stop {
    fail(codes::INVALID_TABLE, words, path)
}

/// `t` with the core's edit `e` applied to its rows, columns, cells and merged ranges.
fn core_applied(t: &TableEntity, e: &Edit, path: &str) -> Result<TableEntity, Stop> {
    let shape = core_table(t);
    let done = core_edit::edit(&shape, e);
    if let Some(p) = done.problem {
        return Err(invalid(p, path));
    }
    let Some(Entity::Table(out)) = done.table.and_then(|s| with_shape(&Entity::Table(t.clone()), s)) else {
        return Err(invalid("Tablo düzenlenemedi.".into(), path));
    };
    Ok(out)
}

/// A range along one axis after `count` lines inserted at `at` (positive) or deleted from `at` (negative); none when
/// it is gone.
fn range_moved(start: u32, len: u32, at: usize, count: isize) -> Option<(u32, u32)> {
    let (s, l) = if count >= 0 {
        grown(start as usize, len as usize, at, count as usize)
    } else {
        shrunk(start as usize, len as usize, at, count.unsigned_abs())
    };
    (l > 0).then_some((s as u32, l as u32))
}

/// The styles, formulas and fields of `t` moved as `count` rows or columns at `at` move them.
fn extras_moved(t: &mut TableEntity, axis: Axis, at: usize, count: isize) {
    t.styles.retain_mut(|s| {
        let r = &mut s.range;
        let moved = match axis {
            Axis::Rows => range_moved(r.row, r.rows, at, count).map(|(a, b)| (a, b, r.col, r.cols)),
            Axis::Columns => range_moved(r.col, r.cols, at, count).map(|(a, b)| (r.row, r.rows, a, b)),
        };
        match moved {
            Some((row, rows, col, cols)) => {
                *r = CellRange { row, rows, col, cols };
                true
            }
            None => false,
        }
    });
    let place = |row: u32, col: u32| -> Option<(u32, u32)> {
        match axis {
            Axis::Rows => table_formula::moved(row as usize, at, count).map(|r| (r as u32, col)),
            Axis::Columns => table_formula::moved(col as usize, at, count).map(|c| (row, c as u32)),
        }
    };
    t.formulas.retain_mut(|f| match place(f.row, f.col) {
        Some((r, c)) => {
            f.row = r;
            f.col = c;
            f.formula = table_formula::shift(&f.formula, axis, at, count);
            true
        }
        None => false,
    });
    t.fields.retain_mut(|f| match place(f.row, f.col) {
        Some((r, c)) => {
            f.row = r;
            f.col = c;
            true
        }
        None => false,
    });
}

fn cell_in(t: &TableEntity, row: u32, col: u32, path: &str) -> Result<(usize, usize), Stop> {
    let (n, m) = (t.rows.len(), t.columns.len());
    if row as usize >= n || col as usize >= m {
        return Err(out_of_range(
            format!(
                "{}. satır, {}. sütun tablonun dışında; tablonun {n} satırı, {m} sütunu var.",
                row + 1,
                col + 1
            ),
            path,
        ));
    }
    // A cell inside a merged range is its top left's.
    Ok(match t.merge_at(row as usize, col as usize) {
        Some(r) => (r.row as usize, r.col as usize),
        None => (row as usize, col as usize),
    })
}

fn range_in(t: &TableEntity, row: u32, col: u32, rows: u32, cols: u32, path: &str) -> Result<CellRange, Stop> {
    let (n, m) = (t.rows.len() as u64, t.columns.len() as u64);
    if rows == 0 || cols == 0 || u64::from(row) + u64::from(rows) > n || u64::from(col) + u64::from(cols) > m {
        return Err(out_of_range(
            format!(
                "{}. satır, {}. sütundan {rows} × {cols} aralık tablonun içinde olmalı; tablonun {n} satırı, {m} sütunu var.",
                row + 1,
                col + 1
            ),
            path,
        ));
    }
    Ok(CellRange { row, col, rows, cols })
}

fn span_in(len: usize, from: u32, count: u32, what: &str, path: &str) -> Result<(usize, usize), Stop> {
    let (from, count) = (from as usize, count as usize);
    if count == 0 || from + count > len {
        return Err(out_of_range(
            format!("{}. {what}dan {count} {what} tablonun içinde olmalı; tablonun {len} {what}ı var.", from + 1),
            path,
        ));
    }
    Ok((from, count))
}

fn size_holds(x: f64, what: &str, path: &str) -> Result<(), Stop> {
    if !x.is_finite() {
        return Err(fail(codes::NOT_FINITE, format!("{what} sonlu bir sayı olmalı."), path));
    }
    if !(x > 0.0 && x <= MAX_TABLE_SIZE) {
        return Err(invalid(format!("{what} {x}; sıfırdan büyük ve en çok {MAX_TABLE_SIZE} m olmalı."), path));
    }
    Ok(())
}

/// `t` with edit `e` applied.
fn edited(doc: &Document, mut t: TableEntity, e: &TableEdit, path: &str) -> Result<TableEntity, Stop> {
    // A schedule's rows and cells are the document's (docs/adr/0344 §3): only its sizes and lines change here.
    if matches!(t.source, Some(TableSource::Schedule { .. }))
        && !matches!(
            e,
            TableEdit::RowHeight { .. }
                | TableEdit::ColumnWidth { .. }
                | TableEdit::Fit { .. }
                | TableEdit::Grid { .. }
                | TableEdit::Frame { .. }
        )
    {
        return Err(fail(
            codes::SCHEDULE_CELL,
            "Çizelgenin hücreleri tablo olarak düzenlenmez; Çizelgeyi düzenle onları nesnelere yazar.".into(),
            path,
        ));
    }
    match e {
        TableEdit::SetCell { row, col, words } => {
            let (r, c) = cell_in(&t, *row, *col, path)?;
            t.formulas.retain(|f| (f.row as usize, f.col as usize) != (r, c));
            t.fields.retain(|f| (f.row as usize, f.col as usize) != (r, c));
            if table_formula::is_formula(words) {
                t.formulas.push(CellFormula {
                    row: r as u32,
                    col: c as u32,
                    formula: words.clone(),
                });
                t.formulas.sort_by_key(|f| (f.row, f.col));
            }
            let edit = Edit::SetCell {
                row: r,
                col: c,
                words: words.clone(),
            };
            let mut out = core_applied(&t, &edit, path)?;
            out.formulas = t.formulas;
            out.fields = t.fields;
            Ok(out)
        }
        TableEdit::InsertRows { at, count } | TableEdit::InsertColumns { at, count } => {
            let rows = matches!(e, TableEdit::InsertRows { .. });
            let len = if rows { t.rows.len() } else { t.columns.len() };
            if *at as usize > len || *count == 0 {
                return Err(out_of_range(
                    format!(
                        "{} {}. sıradan önce eklenemez: en az bir tane, en çok {len}. sıranın ardına eklenir.",
                        if rows { "Satır" } else { "Sütun" },
                        at + 1
                    ),
                    path,
                ));
            }
            let edit = if rows {
                Edit::InsertRows { at: *at as usize, count: *count as usize }
            } else {
                Edit::InsertColumns { at: *at as usize, count: *count as usize }
            };
            let mut out = core_applied(&t, &edit, path)?;
            out.styles = t.styles;
            out.formulas = t.formulas;
            out.fields = t.fields;
            extras_moved(
                &mut out,
                if rows { Axis::Rows } else { Axis::Columns },
                *at as usize,
                *count as isize,
            );
            Ok(out)
        }
        TableEdit::DeleteRows { from, count } | TableEdit::DeleteColumns { from, count } => {
            let rows = matches!(e, TableEdit::DeleteRows { .. });
            let len = if rows { t.rows.len() } else { t.columns.len() };
            let (f, k) = span_in(len, *from, *count, if rows { "satır" } else { "sütun" }, path)?;
            if k == len {
                return Err(invalid(
                    format!("Tablonun bütün {}ları silinemez; en az biri kalır.", if rows { "satır" } else { "sütun" }),
                    path,
                ));
            }
            let edit = if rows {
                Edit::DeleteRows { from: f, count: k }
            } else {
                Edit::DeleteColumns { from: f, count: k }
            };
            let mut out = core_applied(&t, &edit, path)?;
            out.styles = t.styles;
            out.formulas = t.formulas;
            out.fields = t.fields;
            extras_moved(&mut out, if rows { Axis::Rows } else { Axis::Columns }, f, -(k as isize));
            Ok(out)
        }
        TableEdit::Merge { row, col, rows, cols } => {
            let r = range_in(&t, *row, *col, *rows, *cols, path)?;
            let edit = Edit::Merge(kentos_geometry_core::entity::CellRange {
                row: r.row as usize,
                col: r.col as usize,
                rows: r.rows as usize,
                cols: r.cols as usize,
            });
            let mut out = core_applied(&t, &edit, path)?;
            // The merged range's words are its cells': their formulas and fields go.
            out.styles = t.styles;
            out.formulas = t
                .formulas
                .into_iter()
                .filter(|f| !r.holds(f.row as usize, f.col as usize))
                .collect();
            out.fields = t
                .fields
                .into_iter()
                .filter(|f| !r.holds(f.row as usize, f.col as usize))
                .collect();
            Ok(out)
        }
        TableEdit::Unmerge { row, col } => {
            cell_in(&t, *row, *col, path)?;
            if t.merge_at(*row as usize, *col as usize).is_none() {
                return Err(invalid(
                    format!("{}. satır, {}. sütun birleşik bir alanda değil.", row + 1, col + 1),
                    path,
                ));
            }
            let edit = Edit::Unmerge {
                row: *row as usize,
                col: *col as usize,
            };
            let mut out = core_applied(&t, &edit, path)?;
            out.styles = t.styles;
            out.formulas = t.formulas;
            out.fields = t.fields;
            Ok(out)
        }
        TableEdit::Style {
            row,
            col,
            rows,
            cols,
            align,
            bold,
            color,
            fill,
            decimals,
        } => {
            let range = range_in(&t, *row, *col, *rows, *cols, path)?;
            t.styles.push(CellStyle {
                range,
                align: *align,
                bold: *bold,
                color: color.clone(),
                fill: fill.clone(),
                decimals: *decimals,
            });
            Ok(t)
        }
        TableEdit::ClearStyle { row, col, rows, cols } => {
            let r = range_in(&t, *row, *col, *rows, *cols, path)?;
            t.styles.retain(|s| {
                let q = s.range;
                !(q.row < r.row + r.rows && r.row < q.row + q.rows && q.col < r.col + r.cols && r.col < q.col + q.cols)
            });
            Ok(t)
        }
        TableEdit::Field { row, col, source, format } => {
            let (r, c) = cell_in(&t, *row, *col, path)?;
            if let Some(why) = source.problem() {
                return Err(invalid(why, path));
            }
            if let Some(why) = format.as_ref().and_then(FieldFormat::problem) {
                return Err(invalid(why, path));
            }
            if let Some(o) = source.object()
                && doc.slot_of(Uuid::from_bytes(o.0)).is_none()
            {
                return Err(fail(
                    codes::ENTITY_NOT_FOUND,
                    format!("Alanın nesnesi “{}” çizimde yok.", Uuid::from_bytes(o.0)),
                    path,
                ));
            }
            t.formulas.retain(|f| (f.row as usize, f.col as usize) != (r, c));
            t.fields.retain(|f| (f.row as usize, f.col as usize) != (r, c));
            t.fields.push(CellField {
                row: r as u32,
                col: c as u32,
                source: source.clone(),
                format: format.clone(),
                broken: None,
            });
            t.fields.sort_by_key(|f| (f.row, f.col));
            Ok(t)
        }
        TableEdit::Unfield { row, col } => {
            let (r, c) = cell_in(&t, *row, *col, path)?;
            let before = t.fields.len();
            t.fields.retain(|f| (f.row as usize, f.col as usize) != (r, c));
            if t.fields.len() == before {
                return Err(invalid(format!("{}. satır, {}. sütunda alan yok.", r + 1, c + 1), path));
            }
            Ok(t)
        }
        TableEdit::RowHeight { from, count, height } => {
            let (f, k) = span_in(t.rows.len(), *from, *count, "satır", path)?;
            size_holds(*height, "Satır yüksekliği", path)?;
            for h in &mut t.rows[f..f + k] {
                *h = *height;
            }
            Ok(t)
        }
        TableEdit::ColumnWidth { from, count, width } => {
            let (f, k) = span_in(t.columns.len(), *from, *count, "sütun", path)?;
            size_holds(*width, "Sütun genişliği", path)?;
            for w in &mut t.columns[f..f + k] {
                *w = *width;
            }
            if let Some(fr) = t.frame
                && 2.0 * fr >= t.columns.iter().sum::<f64>()
            {
                return Err(invalid(
                    "Sütunlar çerçeveden dar kalıyor; çerçeve tablonun eninin yarısından küçük olmalı.".into(),
                    path,
                ));
            }
            Ok(t)
        }
        TableEdit::Fit {
            from,
            count,
            grow,
            rows,
        } => {
            let (f, k) = span_in(t.columns.len(), *from, *count, "sütun", path)?;
            let font = drawing_font(doc.settings().drawing_font);
            let shape = core_table(&t);
            let Some(g) = kentos_geometry_core::geom::table::table_geom(&shape) else {
                return Err(invalid("Tablo ölçülemedi.".into(), path));
            };
            let fitted = kentos_geometry_core::geom::table::fit_widths(&g, font, LEAST * t.height);
            for (w, fit) in t.columns[f..f + k].iter_mut().zip(&fitted[f..f + k]) {
                *w = if *grow { w.max(*fit) } else { *fit };
            }
            if *rows {
                let h = kentos_geometry_core::ops::table::ROW * t.height;
                t.rows = vec![h; t.rows.len()];
            }
            Ok(t)
        }
        TableEdit::ColumnAlign { from, count, align } => {
            let (f, k) = span_in(t.columns.len(), *from, *count, "sütun", path)?;
            let m = t.columns.len();
            let mut aligns = t.aligns.take().unwrap_or_else(|| vec![kentos_contracts::TableAlign::Left; m]);
            for a in &mut aligns[f..f + k] {
                *a = *align;
            }
            t.aligns = (!aligns.iter().all(|a| *a == kentos_contracts::TableAlign::Left)).then_some(aligns);
            Ok(t)
        }
        TableEdit::Header { on } => {
            t.header = *on;
            Ok(t)
        }
        TableEdit::Grid { grid } => {
            t.grid = *grid;
            Ok(t)
        }
        TableEdit::Frame { width } => {
            if let Some(w) = width {
                if !w.is_finite() {
                    return Err(fail(codes::NOT_FINITE, "Çerçeve kalınlığı sonlu bir sayı olmalı.".into(), path));
                }
                let (wd, d): (f64, f64) = (t.columns.iter().sum(), t.rows.iter().sum());
                if !(*w > 0.0 && 2.0 * w < wd.min(d)) {
                    return Err(invalid(
                        format!("Çerçeve kalınlığı {w}; sıfırdan büyük, tablonun eninin ve boyunun yarısından küçük olmalı."),
                        path,
                    ));
                }
            }
            t.frame = *width;
            Ok(t)
        }
        TableEdit::Unlink => {
            t.source = None;
            Ok(t)
        }
    }
}

// ── Sources ────────────────────────────────────────────────────────────────

/// Where a linked file is: its path as given when absolute, else from `base` (the drawing's folder).
fn resolved(path: &str, base: Option<&str>) -> PathBuf {
    let p = Path::new(path);
    match base {
        Some(b) if !p.is_absolute() => Path::new(b).join(p),
        _ => p.to_path_buf(),
    }
}

/// The SHA-256 of `bytes`, lower case hex.
pub fn digest_of(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// A file's sheet's cells as a table takes them (its rows, the empty ones after the last with words dropped), or
/// why they cannot be.
fn sheet_rows(bytes: &[u8], sheet: Option<&str>, header: bool) -> Result<(Vec<Vec<String>>, Vec<String>), String> {
    let book = kentos_formats::table_file::read(bytes);
    if let Some(p) = book.problem {
        return Err(p);
    }
    let found = match sheet {
        None => book.sheets.first(),
        Some(want) => book.sheets.iter().find(|s| s.name.as_deref() == Some(want)),
    };
    let Some(found) = found else {
        return Err(format!("Dosyada “{}” sayfası yok.", sheet.unwrap_or_default()));
    };
    let cells = kentos_geometry_core::ops::table::from_rows(found.rows.clone(), header);
    if let Some(p) = cells.problem {
        return Err(if found.cut {
            "Sayfada 10.000'den çok satır var; bir tablo en çok 10.000 satırdır.".to_owned()
        } else {
            p
        });
    }
    Ok((cells.cells, cells.aligns))
}

/// `t` with `cells` as its source's new cells (Tabloyu güncelle's rule, ADR 0184 §6), its styles, formulas and fields
/// kept within its new rows and columns.
fn refreshed(doc: &Document, t: &TableEntity, cells: &[Vec<String>], aligns: &[String]) -> Option<TableEntity> {
    let font = drawing_font(doc.settings().drawing_font);
    let next = core_edit::refresh(&core_table(t), cells, aligns, font)?;
    let Some(Entity::Table(mut out)) = with_shape(&Entity::Table(t.clone()), next) else {
        return None;
    };
    let (n, m) = (out.rows.len(), out.columns.len());
    out.clip_to(n, m);
    Some(out)
}

/// The file's new cells with the cells changed by hand kept as `rule` says: a cell is changed by hand when it differs
/// from the file's copy last read (a formula's or a field's is not); kept while the file's cell there is as it was.
/// The cells, how many kept and how many written over.
fn with_overrides(
    t: &TableEntity,
    copy: Option<&Vec<Vec<String>>>,
    fresh: &[Vec<String>],
    rule: SourceOverrides,
) -> (Vec<Vec<String>>, u32, u32) {
    let mut out = fresh.to_vec();
    let Some(copy) = copy else {
        return (out, 0, 0);
    };
    let (mut kept, mut replaced) = (0, 0);
    for (r, row) in out.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            let (Some(own), Some(was)) = (
                t.cells.get(r).and_then(|x| x.get(c)),
                copy.get(r).and_then(|x| x.get(c)),
            ) else {
                continue;
            };
            if own == was || t.formula_at(r, c).is_some() || t.field_at(r, c).is_some() {
                continue;
            }
            if rule == SourceOverrides::Keep && cell == was {
                *cell = own.clone();
                kept += 1;
            } else {
                replaced += 1;
            }
        }
    }
    (out, kept, replaced)
}

struct Checked {
    changed: Vec<(Slot, Uuid, TableEntity)>,
    report: Vec<TableReport>,
    warnings: Vec<CommandWarning>,
}

fn warning(code: &str, message: String) -> CommandWarning {
    CommandWarning {
        code: code.to_owned(),
        message,
        path: None,
    }
}

fn check(doc: &Document, input: &TablesEdit) -> Result<Checked, Stop> {
    let mut changed: Vec<(Slot, Uuid, TableEntity)> = Vec::new();
    let mut report = Vec::new();
    let mut warnings = Vec::new();
    match &input.operation {
        TablesOperation::Edit { uid, edits } => {
            let (slot, u, t) = table_at(doc, uid, "operation.uid")?;
            if edits.is_empty() {
                return Err(invalid("Düzenleme verilmedi; en az bir düzenleme verin.".into(), "operation.edits"));
            }
            let mut next = t.clone();
            for (i, e) in edits.iter().enumerate() {
                next = edited(doc, next, e, &format!("operation.edits[{i}]"))?;
            }
            if let Some((field, words)) = next.problem().or_else(|| cells_problem(&next)) {
                return Err(invalid(words, &format!("operation.{field}")));
            }
            changed.push((slot, u, doc.table_worked_out(&next)));
        }
        TablesOperation::Refresh { uids, base } => {
            let format = kentos_project::format::Format::of(doc.settings());
            for (slot, u, t) in tables_at(doc, uids)? {
                let mut status = TableRefresh::Updated;
                let (mut kept, mut replaced) = (0, 0);
                let next: Option<TableEntity> = match &t.source {
                    None => {
                        return Err(fail(
                            codes::NO_SOURCE,
                            "Tablonun kaynağı yok: Tabloyu güncelle çizelgeleri ve dosyaya bağlı tabloları yazar.".into(),
                            "operation.uids",
                        ));
                    }
                    Some(TableSource::Blocks { .. }) => {
                        status = TableRefresh::Live;
                        None
                    }
                    Some(TableSource::File { path: None, name, .. }) => {
                        status = TableRefresh::Unlinked;
                        warnings.push(warning(
                            codes::SOURCE_UNLINKED,
                            format!("“{name}” tablosu dosyasına bağlı değil; Dosyaya bağla ile dosyayı seçin."),
                        ));
                        None
                    }
                    Some(
                        source @ TableSource::File {
                            path: Some(path),
                            sheet,
                            digest,
                            copy,
                            overrides,
                            ..
                        },
                    ) => match std::fs::read(resolved(path, base.as_deref())) {
                        Err(_) => {
                            status = TableRefresh::Missing;
                            warnings.push(warning(
                                codes::SOURCE_MISSING,
                                format!("“{path}” dosyası yolunda yok; tablo olduğu gibi kaldı. Dosyaya bağla ile yeniden seçin."),
                            ));
                            None
                        }
                        Ok(bytes) => {
                            let d = digest_of(&bytes);
                            if digest.as_deref() == Some(d.as_str()) {
                                status = TableRefresh::Unchanged;
                                None
                            } else {
                                match sheet_rows(&bytes, sheet.as_deref(), t.header) {
                                    Err(why) => {
                                        status = TableRefresh::Unreadable;
                                        warnings.push(warning(
                                            codes::SOURCE_UNREADABLE,
                                            format!("“{path}”: {why} Tablo olduğu gibi kaldı."),
                                        ));
                                        None
                                    }
                                    Ok((fresh, aligns)) => {
                                        let (cells, k, r) = with_overrides(&t, copy.as_ref(), &fresh, *overrides);
                                        (kept, replaced) = (k, r);
                                        refreshed(doc, &t, &cells, &aligns).map(|mut n| {
                                            let mut s = source.clone();
                                            if let TableSource::File { digest, copy, .. } = &mut s {
                                                *digest = Some(d);
                                                *copy = Some(fresh);
                                            }
                                            n.source = Some(s);
                                            n
                                        })
                                    }
                                }
                            }
                        }
                    },
                    Some(schedule) => {
                        let kind = match schedule {
                            TableSource::Coordinates { .. } => ScheduleKind::Coordinates,
                            TableSource::Areas { .. } => ScheduleKind::Areas,
                            _ => ScheduleKind::Attributes,
                        };
                        let objects: Vec<Slot> = schedule
                            .objects()
                            .iter()
                            .filter_map(|id| doc.slot_of(Uuid::from_bytes(id.0)))
                            .collect();
                        let missing = schedule.objects().len() - objects.len();
                        if missing > 0 {
                            warnings.push(warning(
                                codes::ENTITY_NOT_FOUND,
                                format!("Çizelgenin {missing} nesnesi çizimde yok: tablo kalanlardan yazıldı."),
                            ));
                        }
                        let cells = crate::table_schedule::schedule(doc, kind, &objects, &format);
                        match cells.problem {
                            Some(p) => {
                                status = TableRefresh::Unreadable;
                                warnings.push(warning(codes::SOURCE_UNREADABLE, p));
                                None
                            }
                            None => refreshed(doc, &t, &cells.cells, &cells.aligns),
                        }
                    }
                };
                let next = next.map(|n| doc.table_worked_out(&n));
                match next {
                    Some(n) if n != t => changed.push((slot, u, n)),
                    Some(_) if status == TableRefresh::Updated => status = TableRefresh::Unchanged,
                    _ => {}
                }
                report.push(TableReport {
                    uid: u.to_string(),
                    status,
                    kept,
                    replaced,
                });
            }
        }
        TablesOperation::Link {
            uid,
            path,
            sheet,
            base,
            overrides,
        } => {
            let (slot, u, t) = table_at(doc, uid, "operation.uid")?;
            if path.trim().is_empty() || path.chars().any(char::is_control) {
                return Err(invalid("Dosyanın yolu boş olamaz.".into(), "operation.path"));
            }
            let Ok(bytes) = std::fs::read(resolved(path, base.as_deref())) else {
                return Err(fail(
                    codes::SOURCE_MISSING,
                    format!("“{path}” dosyası okunamadı: yolu doğru mu, dosya var mı?"),
                    "operation.path",
                ));
            };
            let (fresh, aligns) = sheet_rows(&bytes, sheet.as_deref(), t.header)
                .map_err(|why| fail(codes::SOURCE_UNREADABLE, format!("“{path}”: {why}"), "operation.path"))?;
            let Some(mut next) = refreshed(doc, &t, &fresh, &aligns) else {
                return Err(invalid("Tablo dosyanın satırlarıyla yazılamadı.".into(), "operation.path"));
            };
            let name = Path::new(path)
                .file_name()
                .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
            next.source = Some(TableSource::File {
                name,
                sheet: sheet.clone(),
                path: Some(path.clone()),
                digest: Some(digest_of(&bytes)),
                copy: Some(fresh),
                overrides: overrides.unwrap_or_default(),
            });
            if let Some((field, words)) = next.problem() {
                return Err(invalid(words, &format!("operation.{field}")));
            }
            let next = doc.table_worked_out(&next);
            report.push(TableReport {
                uid: u.to_string(),
                status: TableRefresh::Updated,
                kept: 0,
                replaced: 0,
            });
            changed.push((slot, u, next));
        }
        TablesOperation::Unlink { uids } => {
            for (slot, u, mut t) in tables_at(doc, uids)? {
                if t.source.take().is_some() {
                    changed.push((slot, u, t));
                }
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    changed.retain(|(slot, _, t)| doc.get(*slot) != Some(&Entity::Table(t.clone())));
    Ok(Checked {
        changed,
        report,
        warnings,
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TablesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &TablesEdit) -> CommandResult<TablesPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (the table editor's preview).
pub fn plan_in(doc: &Document, input: &TablesEdit) -> CommandResult<TablesPlan> {
    match check(doc, input) {
        Ok(c) => CommandResult::Completed {
            output: TablesPlan {
                tables: c.changed.into_iter().map(|(_, _, t)| Entity::Table(t)).collect(),
                report: c.report,
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: TablesEdit) -> CommandResult<TablesEdited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_TABLES_EDIT,
        kentos_contracts::CAD_TABLES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: TablesEdit) -> CommandResult<TablesEdited> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let changed = c.changed.iter().map(|(_, u, _)| u.to_string()).collect();
    let name = label(&input.operation);
    if !c.changed.is_empty() {
        let _: Result<(), ()> = doc.transact(name, |doc| {
            doc.update_many(
                c.changed
                    .into_iter()
                    .map(|(s, _, t)| (s, Entity::Table(t)))
                    .collect(),
                name,
            );
            Ok(())
        });
    }
    CommandResult::Completed {
        output: TablesEdited {
            changed,
            report: c.report,
            revision: doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}

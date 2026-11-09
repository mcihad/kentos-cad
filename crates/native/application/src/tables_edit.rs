//! `cad.tables.edit` v1 (docs/adr/0327): a table's cells, rows and columns edited in order (Tabloyu düzenle), cell
//! styles, formulas and fields; tables written again from their sources (Tabloyu güncelle), linked to a file (Dosyaya
//! bağla) or let go of it; each as one undo step. The cells and merged ranges change by the core's rules
//! (`ops::table_edit`), the styles, formulas' references and fields move with the rows and columns
//! (`table_formula::shift`); a table's computed cells are written by the document's rule (`table_worked_out`). The web
//! is paused (docs/adr/0228): the desktop's handler alone, passing `fixtures/commands/v1/cad.tables.edit.json`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use kentos_commands::table_formula::{self, Axis};
use kentos_commands::{
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
    // A schedule's rows and cells are the document's (docs/adr/0344 §3): only its sizes and lines change here; Emsal ve
    // TAKS tablosu's too (docs/adr/3030).
    let zoning = t.source.as_ref().is_some_and(TableSource::of_regulation);
    if (zoning || matches!(t.source, Some(TableSource::Schedule { .. })))
        && !matches!(
            e,
            TableEdit::RowHeight { .. }
                | TableEdit::ColumnWidth { .. }
                | TableEdit::Fit { .. }
                | TableEdit::Grid { .. }
                | TableEdit::Frame { .. }
                | TableEdit::Breaks { .. }
        )
    {
        return Err(fail(
            codes::SCHEDULE_CELL,
            if zoning {
                "Emsal ve TAKS tablosunun hücreleri çizimden ve imar durumundan yazılır; parseli ve oranları Proje \
                 ayarları › Mimari › İmar durumu'nda değiştirin."
                    .into()
            } else {
                "Çizelgenin hücreleri tablo olarak düzenlenmez; Çizelgeyi düzenle onları nesnelere yazar.".into()
            },
            path,
        ));
    }
    if matches!(&t.source, Some(TableSource::File { key_column: Some(_), .. }))
        && matches!(e, TableEdit::InsertRows { .. } | TableEdit::DeleteRows { .. } | TableEdit::InsertColumns { .. } | TableEdit::DeleteColumns { .. } | TableEdit::Merge { .. } | TableEdit::Unmerge { .. })
    {
        return Err(invalid("Anahtarlı bağlı tabloda satır/sütun yapısı değiştirilmez; önce kaynaktan ayırın.".into(), path));
    }
    match e {
        TableEdit::SetCell { row, col, words } => {
            let (r, c) = cell_in(&t, *row, *col, path)?;
            if r > 0 && matches!(&t.source, Some(TableSource::File { key_column: Some(key), .. }) if *key as usize == c) {
                return Err(invalid("Bağlı tablonun satır anahtarı hücresi değiştirilemez; kaynak dosyayı düzeltin.".into(), path));
            }
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
            // Words of several lines (docs/adr/1030 §7): the core writes a cell's words as one line (ADR 0184 §6);
            // their line breaks are kept in the cell it wrote them to (a merged range's top left), the table's rule
            // (at most 20 lines, no other control character) holding them.
            if words.contains('\n') {
                let (tr, tc) = t.merge_at(r, c).map_or((r, c), |m| (m.row as usize, m.col as usize));
                if let Some(cell) = out.cells.get_mut(tr).and_then(|row| row.get_mut(tc)) {
                    *cell = words.clone();
                }
            }
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
                // Each row as deep as its cells' lines ask (docs/adr/1030 §7).
                t.rows = (0..t.rows.len())
                    .map(|i| kentos_geometry_core::geom::table::fit_depth(&g, i, kentos_geometry_core::ops::table::ROW))
                    .collect();
            }
            Ok(t)
        }
        TableEdit::ColumnAlign { from, count, align } => {
            let (f, k) = span_in(t.columns.len(), *from, *count, "sütun", path)?;
            let m = t.columns.len();
            let mut aligns = t.aligns.take().unwrap_or_else(|| vec![kentos_commands::TableAlign::Left; m]);
            for a in &mut aligns[f..f + k] {
                *a = *align;
            }
            t.aligns = (!aligns.iter().all(|a| *a == kentos_commands::TableAlign::Left)).then_some(aligns);
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
        TableEdit::Breaks { breaks } => {
            if let Some(b) = breaks {
                if !(b.height.is_finite() && b.gap.is_finite()) {
                    return Err(fail(codes::NOT_FINITE, "Parça derinliği ve aralığı sonlu sayılar olmalı.".into(), path));
                }
                if let Some(words) = b.problem() {
                    return Err(invalid(words, path));
                }
            }
            t.breaks = *breaks;
            Ok(t)
        }
        TableEdit::SourceKey { column } => {
            let Some(TableSource::File { key_column, copy, .. }) = &mut t.source else {
                return Err(invalid("Satır anahtarı yalnız dosyaya bağlı tabloda seçilir.".into(), path));
            };
            if let Some(col) = column {
                if !t.header {
                    return Err(invalid("Satır anahtarı için başlık satırı açık olmalı.".into(), path));
                }
                let Some(copy) = copy else {
                    return Err(invalid("Satır anahtarı için son kaynak kopyası gerekli; dosyaya yeniden bağlayın.".into(), path));
                };
                if t.cells.len() != copy.len() || t.cells.iter().zip(copy.iter()).any(|(own, was)| own.len() != was.len()) {
                    return Err(invalid("Tablonun satır ve sütun yapısı son kaynak kopyasından farklı; önce kaynağa yeniden bağlayın.".into(), path));
                }
                let keys = row_keys(copy, *col as usize).map_err(|why| invalid(why, path))?;
                if keys.values().any(|r| t.cells.get(*r).and_then(|row| row.get(*col as usize)) != copy[*r].get(*col as usize)) {
                    return Err(invalid("Satır anahtarı sütununda elle değişmiş değer var; önce kaynağı düzeltin.".into(), path));
                }
            }
            *key_column = *column;
            Ok(t)
        }
        TableEdit::SourceRule { rule } => {
            let Some(TableSource::File { overrides, .. }) = &mut t.source else {
                return Err(invalid("Elle değişen hücre kuralı yalnız dosyaya bağlı tabloda seçilir.".into(), path));
            };
            *overrides = *rule;
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

/// Without an explicit row key, a hand edit belongs to a position only. If the old row is
/// identifiable from its unedited cells at a different position, or a unique unchanged column
/// no longer identifies it in that position, applying it by row index could attach the edit to
/// another item. Leave the linked table untouched instead.
fn moved_hand_edit(t: &TableEntity, copy: &[Vec<String>], fresh: &[Vec<String>]) -> bool {
    let first = usize::from(t.header);
    let mut frequencies: Option<HashMap<(usize, &str), usize>> = None;
    for (r, was) in copy.iter().enumerate().skip(first) {
        let Some(own) = t.cells.get(r) else { continue };
        let changed = was.iter().enumerate().any(|(c, value)| {
            own.get(c).is_some_and(|cell| cell != value)
                && t.formula_at(r, c).is_none()
                && t.field_at(r, c).is_none()
        });
        if !changed {
            continue;
        }
        let stable: Vec<(usize, &str)> = was
            .iter()
            .enumerate()
            .filter(|(c, value)| !value.is_empty() && own.get(*c) == Some(value))
            .map(|(c, value)| (c, value.as_str()))
            .collect();
        if stable.is_empty() {
            continue;
        }
        let matches = |row: &[String]| {
            stable
                .iter()
                .all(|(c, value)| row.get(*c).is_some_and(|v| v == value))
        };
        let changed_position = fresh.get(r).is_none_or(|row| !matches(row));
        if !changed_position {
            continue;
        }
        if fresh.get(r).is_none() {
            return true;
        }
        let counts = frequencies.get_or_insert_with(|| {
            let mut counts = HashMap::new();
            for row in copy.iter().skip(first) {
                for (c, value) in row.iter().enumerate().filter(|(_, value)| !value.is_empty()) {
                    *counts.entry((c, value.as_str())).or_insert(0) += 1;
                }
            }
            counts
        });
        let unique_anchor = stable.iter().find(|(c, value)| {
            counts.get(&(*c, *value)) == Some(&1)
        });
        let unique_anchor_changed = unique_anchor.is_some_and(|(c, value)| {
            fresh.get(r).and_then(|row| row.get(*c)).is_none_or(|v| v != value)
        });
        if unique_anchor_changed
            || fresh
                .iter()
                .enumerate()
                .skip(first)
                .any(|(i, row)| i != r && matches(row))
        {
            return true;
        }
    }
    false
}

/// A Keep override and the file changed the same cell since the embedded copy. Without an
/// explicit resolution, refreshing would discard the hand edit. Formulas and fields are computed
/// cells, so they are not treated as hand edits here.
fn conflicting_hand_edits(t: &TableEntity, copy: &[Vec<String>], fresh: &[Vec<String>]) -> usize {
    copy.iter()
        .enumerate()
        .map(|(r, row)| {
            row.iter()
                .enumerate()
                .filter(|(c, was)| {
                    t.cells
                        .get(r)
                        .and_then(|own| own.get(*c))
                        .is_some_and(|own| own != *was)
                        && t.formula_at(r, *c).is_none()
                        && t.field_at(r, *c).is_none()
                        && fresh
                            .get(r)
                            .and_then(|new| new.get(*c))
                            .is_some_and(|new| new != *was)
                })
                .count()
        })
        .sum()
}

fn row_keys(rows: &[Vec<String>], column: usize) -> Result<HashMap<&str, usize>, String> {
    if rows.first().is_none_or(|header| column >= header.len()) {
        return Err(format!("Satır anahtarı sütunu {} dosyada yok.", column + 1));
    }
    let mut keys = HashMap::new();
    for (r, row) in rows.iter().enumerate().skip(1) {
        let Some(key) = row.get(column).map(String::as_str).filter(|key| !key.trim().is_empty()) else {
            return Err(format!("{}. veri satırının anahtarı boş.", r));
        };
        if key != key.trim() {
            return Err(format!("“{key}” satır anahtarının başında veya sonunda boşluk var."));
        }
        if keys.insert(key, r).is_some() {
            return Err(format!("“{key}” satır anahtarı dosyada birden çok kez geçiyor."));
        }
    }
    Ok(keys)
}

/// Match file rows by a declared, unique column. Source order is retained; hand edits follow the
/// key, and conflicting edits or disappearing edited rows stop the whole refresh. Row-scoped table
/// styles, merges, formulas and fields need their own reindexing before a reorder can be accepted.
fn keyed_overrides(
    t: &TableEntity,
    copy: &[Vec<String>],
    fresh: &[Vec<String>],
    column: usize,
    rule: SourceOverrides,
) -> Result<(Vec<Vec<String>>, u32, u32), String> {
    if !t.header || copy.first() != fresh.first() {
        return Err("Anahtarlı tablo için başlık satırı ve sütun sırası aynı kalmalı.".to_owned());
    }
    let old_keys = row_keys(copy, column)?;
    let new_keys = row_keys(fresh, column)?;
    if rule == SourceOverrides::Keep
        && old_keys.values().any(|r| t.cells.get(*r).and_then(|row| row.get(column)) != copy[*r].get(column))
    {
        return Err("Satır anahtarı hücresi elle değiştirildi; kaynak satırı güvenle eşlenemiyor.".to_owned());
    }
    for (key, old_r) in &old_keys {
        if new_keys.contains_key(key) {
            continue;
        }
        let Some(own) = t.cells.get(*old_r) else { continue };
        if rule == SourceOverrides::Keep
            && copy[*old_r].iter().enumerate().any(|(c, was)| {
                own.get(c).is_some_and(|value| value != was)
                    && t.formula_at(*old_r, c).is_none()
                    && t.field_at(*old_r, c).is_none()
            })
        {
            return Err(format!("“{key}” anahtarlı elle düzeltilmiş satır kaynaktan silindi; tablo korunuyor."));
        }
    }
    let mut out = fresh.to_vec();
    let (mut kept, mut replaced) = (0, 0);
    for (c, cell) in out[0].iter_mut().enumerate() {
        let (Some(own), Some(was)) = (
            t.cells.first().and_then(|row| row.get(c)),
            copy.first().and_then(|row| row.get(c)),
        ) else { continue };
        if own == was || t.formula_at(0, c).is_some() || t.field_at(0, c).is_some() {
            continue;
        }
        if rule == SourceOverrides::Keep {
            *cell = own.clone();
            kept += 1;
        } else {
            replaced += 1;
        }
    }
    for row in out.iter_mut().skip(1) {
        let Some(key) = row.get(column).cloned() else { continue };
        let Some(old_r) = old_keys.get(key.as_str()).copied() else { continue };
        for (c, cell) in row.iter_mut().enumerate() {
            let (Some(own), Some(was)) = (
                t.cells.get(old_r).and_then(|row| row.get(c)),
                copy.get(old_r).and_then(|row| row.get(c)),
            ) else { continue };
            if own == was || t.formula_at(old_r, c).is_some() || t.field_at(old_r, c).is_some() {
                continue;
            }
            if rule == SourceOverrides::Keep {
                if cell != was {
                    return Err(format!("“{key}” anahtarlı satırın {}. hücresini hem kaynak hem kullanıcı değiştirdi; tablo korunuyor.", c + 1));
                }
                *cell = own.clone();
                kept += 1;
            } else {
                replaced += 1;
            }
        }
    }
    Ok((out, kept, replaced))
}

/// Preserve style precedence while moving row-scoped styles with keyed records.
/// A range split into runs remains at the same position in the style stack.
fn keyed_styles(t: &TableEntity, copy: &[Vec<String>], fresh: &[Vec<String>], column: usize) -> Result<Vec<CellStyle>, String> {
    let old_keys = row_keys(copy, column)?;
    let new_keys = row_keys(fresh, column)?;
    let reordered = old_keys.iter().any(|(key, old)| new_keys.get(key) != Some(old));
    if !reordered {
        return Ok(t.styles.clone());
    }
    let mut styles = Vec::new();
    for style in &t.styles {
        let range = &style.range;
        if range.row == 0 && range.rows == 1 {
            styles.push(style.clone());
            continue;
        }
        let whole = (range.row == 0 && range.rows as usize == copy.len())
            || (range.row == 1 && range.rows as usize == copy.len() - 1);
        if whole {
            if fresh.len() <= range.row as usize {
                return Err("Biçimli veri satırları kaynaktan silindi; tablo korunuyor.".to_owned());
            }
            let mut next = style.clone();
            next.range.rows = (fresh.len() - range.row as usize) as u32;
            styles.push(next);
            continue;
        }
        let mut positions = Vec::new();
        for old_r in range.row as usize..range.row as usize + range.rows as usize {
            if old_r == 0 {
                positions.push(0);
                continue;
            }
            let key = copy.get(old_r).and_then(|row| row.get(column))
                .ok_or_else(|| "Biçimli satır kaynak kopyasında bulunamadı; tablo korunuyor.".to_owned())?;
            positions.push(*new_keys.get(key.as_str())
                .ok_or_else(|| format!("“{key}” anahtarlı biçimli satır kaynaktan silindi; tablo korunuyor."))?);
        }
        positions.sort_unstable();
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for at in positions {
            match runs.last_mut() {
                Some((start, count)) if *start + *count == at => *count += 1,
                _ => runs.push((at, 1)),
            }
        }
        for (start, count) in runs {
            let mut next = style.clone();
            next.range.row = start as u32;
            next.range.rows = count as u32;
            styles.push(next);
        }
    }
    if styles.len() > kentos_commands::MAX_CELL_STYLES {
        return Err("Yenilenen tablo çok fazla hücre stili gerektiriyor; tablo korunuyor.".to_owned());
    }
    Ok(styles)
}

/// A merge within one keyed row follows that row; a merge across data rows
/// stays blocked because moving its anchor alone could change its contents.
fn keyed_merges(t: &TableEntity, copy: &[Vec<String>], fresh: &[Vec<String>], column: usize) -> Result<Vec<CellRange>, String> {
    let old_keys = row_keys(copy, column)?;
    let new_keys = row_keys(fresh, column)?;
    let reordered = old_keys.iter().any(|(key, old)| new_keys.get(key) != Some(old));
    if !reordered {
        return Ok(t.merges.clone());
    }
    t.merges.iter().map(|range| {
        if range.row == 0 && range.rows == 1 {
            return Ok(*range);
        }
        if range.rows != 1 || range.row == 0 {
            return Err("Satır sırası değişti; birden çok satıra yayılan birleşim güvenle taşınamadığı için tablo korunuyor.".to_owned());
        }
        let key = copy.get(range.row as usize).and_then(|row| row.get(column))
            .ok_or_else(|| "Birleşik satır kaynak kopyasında bulunamadı; tablo korunuyor.".to_owned())?;
        let row = *new_keys.get(key.as_str())
            .ok_or_else(|| format!("“{key}” anahtarlı birleşik satır kaynaktan silindi; tablo korunuyor."))? as u32;
        Ok(CellRange { row, ..*range })
    }).collect()
}

/// Field sources use persistent object or project identity, so only the
/// field's cell address changes when its keyed row moves.
fn keyed_fields(t: &TableEntity, copy: &[Vec<String>], fresh: &[Vec<String>], column: usize) -> Result<Vec<CellField>, String> {
    let old_keys = row_keys(copy, column)?;
    let new_keys = row_keys(fresh, column)?;
    let reordered = old_keys.iter().any(|(key, old)| new_keys.get(key) != Some(old));
    if !reordered {
        return Ok(t.fields.clone());
    }
    t.fields.iter().map(|field| {
        if field.row == 0 {
            return Ok(field.clone());
        }
        let key = copy.get(field.row as usize).and_then(|row| row.get(column))
            .ok_or_else(|| "Field satırı kaynak kopyasında bulunamadı; tablo korunuyor.".to_owned())?;
        let row = *new_keys.get(key.as_str())
            .ok_or_else(|| format!("“{key}” anahtarlı field satırı kaynaktan silindi; tablo korunuyor."))? as u32;
        let mut next = field.clone();
        next.row = row;
        Ok(next)
    }).collect()
}

/// Formula cells and their references follow keyed source rows. A range that
/// becomes disjoint, or a reference to a removed row, stops refresh.
fn keyed_formulas(t: &TableEntity, copy: &[Vec<String>], fresh: &[Vec<String>], column: usize) -> Result<Vec<CellFormula>, String> {
    let old_keys = row_keys(copy, column)?;
    let new_keys = row_keys(fresh, column)?;
    let reordered = old_keys.iter().any(|(key, old)| new_keys.get(key) != Some(old));
    if !reordered {
        return Ok(t.formulas.clone());
    }
    let mut rows = vec![None; copy.len()];
    rows[0] = Some(0);
    for (key, old_row) in old_keys {
        rows[old_row] = new_keys.get(key).copied();
    }
    t.formulas.iter().map(|formula| {
        let row = rows.get(formula.row as usize).copied().flatten()
            .ok_or_else(|| "Formül satırı kaynaktan silindi; tablo korunuyor.".to_owned())?;
        let words = table_formula::remap_rows(&formula.formula, &rows)
            .ok_or_else(|| "Formül aralığı bölündü veya satır başvurusu silindi; tablo korunuyor.".to_owned())?;
        let mut next = formula.clone();
        next.row = row as u32;
        next.formula = words;
        Ok(next)
    }).collect()
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
                    // A standalone table can still contain live cell fields.
                    // Refresh is the explicit, undoable cache update for those too.
                    None if !t.fields.is_empty() => Some(t.clone()),
                    None => {
                        return Err(fail(
                            codes::NO_SOURCE,
                            "Tablonun kaynağı yok: Tabloyu güncelle çizelgeleri ve dosyaya bağlı tabloları yazar.".into(),
                            "operation.uids",
                        ));
                    }
                    // Emsal ve TAKS tablosu (docs/adr/3030) too: the document writes it at every step.
                    Some(
                        TableSource::Blocks { .. }
                        | TableSource::Zoning {}
                        | TableSource::Parking {}
                        | TableSource::Shelter {},
                    ) => {
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
                            key_column,
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
                                        let merged = if let Some(column) = key_column {
                                            match copy.as_ref() {
                                                Some(old) => keyed_overrides(&t, old, &fresh, *column as usize, *overrides)
                                                    .and_then(|(cells, kept, replaced)| keyed_styles(&t, old, &fresh, *column as usize)
                                                        .and_then(|styles| keyed_merges(&t, old, &fresh, *column as usize)
                                                            .and_then(|merges| keyed_fields(&t, old, &fresh, *column as usize)
                                                                .and_then(|fields| keyed_formulas(&t, old, &fresh, *column as usize)
                                                                    .map(|formulas| (cells, kept, replaced, Some((styles, merges, fields, formulas)))))))),
                                                None => Err("Anahtarlı tablonun son kaynak kopyası yok; elle düzeltmeler güvenle eşlenemiyor.".to_owned()),
                                            }
                                        } else if *overrides == SourceOverrides::Keep
                                            && copy.as_ref().is_some_and(|old| moved_hand_edit(&t, old, &fresh))
                                        {
                                            Err("Elle düzeltilmiş bir satırın sırası veya kimliği değişti. Satır anahtarı tanımlı olmadığı için düzeltme başka kaleme aktarılmadı; tablo olduğu gibi kaldı.".to_owned())
                                        } else {
                                            let conflicts = if *overrides == SourceOverrides::Keep {
                                                copy.as_ref().map_or(0, |old| conflicting_hand_edits(&t, old, &fresh))
                                            } else {
                                                0
                                            };
                                            if conflicts > 0 {
                                                Err(format!("Kaynak ve elle düzeltme aynı {conflicts} hücreyi değiştirdi. Düzeltmeler sessizce silinmedi; tablo olduğu gibi kaldı. Kaynak değişikliğini inceleyip elle çözün veya açıkça 'üzerine yaz' seçeneğini kullanın."))
                                            } else {
                                                let (cells, kept, replaced) = with_overrides(&t, copy.as_ref(), &fresh, *overrides);
                                                Ok((cells, kept, replaced, None))
                                            }
                                        };
                                        match merged {
                                            Err(why) => {
                                                status = TableRefresh::Unreadable;
                                                warnings.push(warning(
                                                    codes::SOURCE_UNREADABLE,
                                                    format!("“{path}” dosyasında {why}"),
                                                ));
                                                None
                                            }
                                            Ok((cells, k, r, placement)) => {
                                                (kept, replaced) = (k, r);
                                                refreshed(doc, &t, &cells, &aligns).map(|mut n| {
                                                    if let Some((styles, merges, fields, formulas)) = placement {
                                                        n.styles = styles;
                                                        n.merges = merges;
                                                        n.fields = fields;
                                                        n.formulas = formulas;
                                                    }
                                                    if let (Some(column), Some(old)) = (key_column, copy.as_ref()) {
                                                        if let Ok(old_keys) = row_keys(old, *column as usize) {
                                                            for (new_row, fresh_row) in fresh.iter().enumerate().skip(1) {
                                                                if let Some(old_row) = fresh_row
                                                                    .get(*column as usize)
                                                                    .and_then(|key| old_keys.get(key.as_str()))
                                                                {
                                                                    if let (Some(height), Some(target)) =
                                                                        (t.rows.get(*old_row), n.rows.get_mut(new_row))
                                                                    {
                                                                        *target = *height;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
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
                            }
                        }
                    },
                    // Veri çıkarımı (docs/adr/1030 §6): its recipe worked out again from the drawings it reads.
                    Some(TableSource::Extraction(x)) => match crate::extract::refresh(doc, &t, x, base.as_deref(), None) {
                        crate::extract::Refreshed::Changed { table, kept: k, replaced: r, warnings: w } => {
                            (kept, replaced) = (k, r);
                            warnings.extend(w);
                            Some(*table)
                        }
                        crate::extract::Refreshed::Unchanged { warnings: w } => {
                            status = TableRefresh::Unchanged;
                            warnings.extend(w);
                            None
                        }
                        crate::extract::Refreshed::Missing(w) => {
                            status = TableRefresh::Missing;
                            warnings.push(w);
                            None
                        }
                        crate::extract::Refreshed::Kept(w) => {
                            status = TableRefresh::Unreadable;
                            warnings.push(w);
                            None
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
            key_column,
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
            let key_column = (*key_column).or(match &t.source {
                Some(TableSource::File { key_column, .. }) => *key_column,
                _ => None,
            });
            if let Some(column) = key_column {
                if !t.header {
                    return Err(invalid("Satır anahtarı için tabloda başlık satırı açık olmalı.".into(), "operation.keyColumn"));
                }
                row_keys(&fresh, column as usize).map_err(|why| invalid(why, "operation.keyColumn"))?;
            }
            let Some(mut next) = refreshed(doc, &t, &fresh, &aligns) else {
                return Err(invalid("Tablo dosyanın satırlarıyla yazılamadı.".into(), "operation.path"));
            };
            let name = Path::new(path)
                .file_name()
                .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
            let override_rule = overrides.unwrap_or(match &t.source {
                Some(TableSource::File { overrides, .. }) => *overrides,
                _ => SourceOverrides::default(),
            });
            next.source = Some(TableSource::File {
                name,
                sheet: sheet.clone(),
                path: Some(path.clone()),
                digest: Some(digest_of(&bytes)),
                copy: Some(fresh),
                overrides: override_rule,
                key_column,
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
        kentos_commands::CAD_TABLES_EDIT,
        kentos_commands::CAD_TABLES_EDIT_VERSION,
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

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::DocumentSnapshotV2;
    use serde_json::json;

    #[test]
    fn a_moved_hand_edit_does_not_attach_to_the_other_file_row() {
        let path = std::env::temp_dir().join(format!(
            "kentos-table-row-move-{}.csv",
            std::process::id()
        ));
        let original = b"ID,Price\nA,10\nB,10\n";
        let changed = b"ID,Price\nB,10\nA,10\n";
        let (copy, _) = sheet_rows(original, None, true).unwrap();
        let mut cells = copy.clone();
        cells[1][1] = "11".to_owned();
        std::fs::write(&path, changed).unwrap();
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/kcad/v2/dynamic-blocks.json"
        ))
        .unwrap();
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(raw).unwrap();
        let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
        let table: TableEntity = serde_json::from_value(json!({
            "id":0,"layerId":"duvar","attrs":{},"p":{"x":0,"y":-2},
            "rotation":0,"height":0.1,"rows":[0.3,0.6,0.4],"columns":[1.0,1.0],
            "cells":cells,"header":true,
            "styles":[
                {"row":1,"col":0,"rows":1,"cols":2,"fill":"#ff0000"},
                {"row":2,"col":0,"rows":1,"cols":2,"fill":"#0000ff"}
            ],
            "source":{"kind":"file","name":"rows.csv","path":path.to_string_lossy(),
                      "digest":digest_of(original),"copy":copy}
        }))
        .unwrap();
        let slot = doc.add(Entity::Table(table)).unwrap();
        let uid = doc.uid(slot).unwrap().to_string();
        let before = doc.to_snapshot_v2();
        let history = doc.undo_labels();
        let request = TablesEdit {
            operation: TablesOperation::Refresh { uids: vec![uid.clone()], base: None },
            expected_revision: None,
        };
        let result = execute(&mut ExecutionContext::new(&mut doc), request.clone());
        let CommandResult::Completed { output, warnings } = result else {
            panic!("refresh must report an ambiguous row: {result:?}");
        };
        assert_eq!(output.report[0].status, TableRefresh::Unreadable);
        assert!(warnings.iter().any(|w| w.message.contains("anahtarı")), "{warnings:?}");
        assert!(output.changed.is_empty());
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.undo_labels(), history);

        let set_key = TablesEdit {
            operation: TablesOperation::Edit {
                uid: uid.clone(),
                edits: vec![TableEdit::SourceKey { column: Some(0) }],
            },
            expected_revision: None,
        };
        assert!(matches!(execute(&mut ExecutionContext::new(&mut doc), set_key), CommandResult::Completed { .. }));
        let result = execute(&mut ExecutionContext::new(&mut doc), request);
        let CommandResult::Completed { output, .. } = result else { panic!("keyed refresh: {result:?}") };
        assert_eq!(output.report[0].status, TableRefresh::Updated);
        assert_eq!(output.report[0].kept, 1);
        let Some(Entity::Table(table)) = doc.get(slot) else { panic!("table") };
        assert_eq!(table.cells[1], vec!["B", "10"]);
        assert_eq!(table.cells[2], vec!["A", "11"]);
        assert_eq!(table.rows, vec![0.3, 0.4, 0.6]);
        assert_eq!(table.styles[0].range.row, 2);
        assert_eq!(table.styles[1].range.row, 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn changed_value_at_the_same_row_is_not_a_row_move() {
        let copy = vec![vec!["ID".into(), "Price".into()], vec!["A".into(), "10".into()]];
        let mut own = copy.clone();
        own[1][1] = "11".into();
        let table: TableEntity = serde_json::from_value(json!({
            "id":0,"layerId":"duvar","attrs":{},"p":{"x":0,"y":0},
            "rotation":0,"height":0.1,"rows":[0.3,0.3],"columns":[1.0,1.0],
            "cells":own,"header":true
        }))
        .unwrap();
        let fresh = vec![copy[0].clone(), vec!["A".into(), "12".into()]];
        assert!(!moved_hand_edit(&table, &copy, &fresh));
        assert!(moved_hand_edit(&table, &copy, &copy[..1]));
        assert!(moved_hand_edit(
            &table,
            &copy,
            &[copy[0].clone(), vec!["B".into(), "10".into()]]
        ));
    }

    #[test]
    fn a_stable_unique_id_allows_other_source_fields_to_change() {
        let copy = vec![
            vec!["ID".into(), "Description".into(), "Price".into()],
            vec!["A".into(), "Old".into(), "10".into()],
        ];
        let table: TableEntity = serde_json::from_value(json!({
            "id":0,"layerId":"duvar","attrs":{},"p":{"x":0,"y":0},
            "rotation":0,"height":0.1,"rows":[0.3,0.3],"columns":[1.0,1.0,1.0],
            "cells":[copy[0].clone(), ["A","Old","11"]],"header":true
        }))
        .unwrap();
        let fresh = vec![copy[0].clone(), vec!["A".into(), "New".into(), "10".into()]];
        assert!(!moved_hand_edit(&table, &copy, &fresh));
    }

    #[test]
    fn file_and_hand_edit_conflict_needs_explicit_replace() {
        let path = std::env::temp_dir().join(format!(
            "kentos-table-cell-conflict-{}.csv",
            std::process::id()
        ));
        let original = b"ID,Price\nA,10\n";
        std::fs::write(&path, b"ID,Price\nA,12\n").unwrap();
        let (copy, _) = sheet_rows(original, None, true).unwrap();
        let mut cells = copy.clone();
        cells[1][1] = "11".to_owned();
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/kcad/v2/dynamic-blocks.json"
        ))
        .unwrap();
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(raw).unwrap();
        let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
        let table: TableEntity = serde_json::from_value(json!({
            "id":0,"layerId":"duvar","attrs":{},"p":{"x":0,"y":-2},
            "rotation":0,"height":0.1,"rows":[0.3,0.3],"columns":[1.0,1.0],
            "cells":cells,"header":true,
            "source":{"kind":"file","name":"prices.csv","path":path.to_string_lossy(),
                      "digest":digest_of(original),"copy":copy}
        }))
        .unwrap();
        let slot = doc.add(Entity::Table(table)).unwrap();
        let uid = doc.uid(slot).unwrap().to_string();
        let before = doc.to_snapshot_v2();
        let history = doc.undo_labels();
        let refresh = || TablesEdit {
            operation: TablesOperation::Refresh { uids: vec![uid.clone()], base: None },
            expected_revision: None,
        };
        let result = execute(&mut ExecutionContext::new(&mut doc), refresh());
        let CommandResult::Completed { output, warnings } = result else {
            panic!("refresh must report the conflict: {result:?}");
        };
        assert_eq!(output.report[0].status, TableRefresh::Unreadable);
        assert!(warnings.iter().any(|w| w.message.contains("aynı 1 hücreyi")), "{warnings:?}");
        assert!(output.changed.is_empty());
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.undo_labels(), history);

        let choose = TablesEdit {
            operation: TablesOperation::Edit {
                uid: uid.clone(),
                edits: vec![TableEdit::SourceRule { rule: SourceOverrides::Replace }],
            },
            expected_revision: None,
        };
        assert!(matches!(execute(&mut ExecutionContext::new(&mut doc), choose), CommandResult::Completed { .. }));
        let result = execute(&mut ExecutionContext::new(&mut doc), refresh());
        let CommandResult::Completed { output, .. } = result else {
            panic!("explicit replace must refresh: {result:?}");
        };
        assert_eq!(output.report[0].status, TableRefresh::Updated);
        assert_eq!(output.report[0].replaced, 1);
        let Some(Entity::Table(table)) = doc.get(slot) else { panic!("table") };
        assert_eq!(table.cells[1][1], "12");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn declared_row_key_follows_reorder_and_rejects_ambiguous_values() {
        let copy = vec![
            vec!["ID".into(), "Price".into()],
            vec!["A".into(), "10".into()],
            vec!["B".into(), "20".into()],
        ];
        let table: TableEntity = serde_json::from_value(json!({
            "id":0,"layerId":"duvar","attrs":{},"p":{"x":0,"y":0},
            "rotation":0,"height":0.1,"rows":[0.3,0.3,0.3],"columns":[1.0,1.0],
            "cells":[["ID","Price"],["A","11"],["B","20"]],"header":true
        }))
        .unwrap();
        let reordered = vec![copy[0].clone(), copy[2].clone(), copy[1].clone()];
        let (cells, kept, replaced) = keyed_overrides(&table, &copy, &reordered, 0, SourceOverrides::Keep).unwrap();
        assert_eq!(cells[1], vec!["B", "20"]);
        assert_eq!(cells[2], vec!["A", "11"]);
        assert_eq!((kept, replaced), (1, 0));
        let (cells, kept, replaced) = keyed_overrides(&table, &copy, &reordered, 0, SourceOverrides::Replace).unwrap();
        assert_eq!(cells[2], vec!["A", "10"]);
        assert_eq!((kept, replaced), (0, 1));
        assert!(keyed_overrides(&table, &copy, &reordered[..2], 0, SourceOverrides::Keep)
            .unwrap_err().contains("kaynaktan silindi"));
        let duplicate = vec![copy[0].clone(), copy[1].clone(), copy[1].clone()];
        assert!(keyed_overrides(&table, &copy, &duplicate, 0, SourceOverrides::Keep)
            .unwrap_err().contains("birden çok"));
        let conflict = vec![copy[0].clone(), vec!["A".into(), "12".into()], copy[2].clone()];
        assert!(keyed_overrides(&table, &copy, &conflict, 0, SourceOverrides::Keep)
            .unwrap_err().contains("hem kaynak hem kullanıcı"));
        let mut merged = table.clone();
        merged.merges = vec![CellRange { row: 1, col: 0, rows: 1, cols: 2 }];
        assert_eq!(keyed_merges(&merged, &copy, &reordered, 0).unwrap()[0].row, 2);
        merged.merges[0].rows = 2;
        assert!(keyed_merges(&merged, &copy, &reordered, 0)
            .unwrap_err().contains("birden çok satıra"));
        let mut fielded = table.clone();
        fielded.fields = vec![serde_json::from_value(json!({
            "row":1,"col":1,"source":{"kind":"project","value":"name"}
        })).unwrap()];
        assert_eq!(keyed_fields(&fielded, &copy, &reordered, 0).unwrap()[0].row, 2);
        assert!(keyed_fields(&fielded, &copy, &reordered[..2], 0)
            .unwrap_err().contains("silindi"));
        let mut calculated = table.clone();
        calculated.formulas = vec![CellFormula { row: 1, col: 1, formula: "=$B$3+2".into() }];
        let formulas = keyed_formulas(&calculated, &copy, &reordered, 0).unwrap();
        assert_eq!((formulas[0].row, formulas[0].formula.as_str()), (2, "=$B$2+2"));
        assert!(keyed_formulas(&calculated, &copy, &reordered[..2], 0)
            .unwrap_err().contains("silindi"));
        calculated.formulas[0].formula = "=TOPLA(B2:B3)".into();
        assert_eq!(keyed_formulas(&calculated, &copy, &reordered, 0).unwrap()[0].formula, "=TOPLA(B2:B3)");
        let inserted = vec![copy[0].clone(), copy[1].clone(), vec!["C".into(), "30".into()], copy[2].clone()];
        calculated.formulas[0].formula = "=TOPLA(B2:B3)".into();
        assert!(keyed_formulas(&calculated, &copy, &inserted, 0)
            .unwrap_err().contains("aralığı bölündü"));
        let mut styled = table.clone();
        styled.styles = vec![serde_json::from_value(json!({
            "row":1,"col":0,"rows":2,"cols":1,"fill":"#ff0000"
        })).unwrap()];
        assert!(keyed_styles(&styled, &copy, &copy[..1], 0)
            .unwrap_err().contains("silindi"));
        let subset = vec![copy[0].clone(), copy[1].clone(), copy[2].clone(), vec!["C".into(), "30".into()]];
        let split = vec![subset[0].clone(), subset[1].clone(), subset[3].clone(), subset[2].clone()];
        let moved = keyed_styles(&styled, &subset, &split, 0).unwrap();
        assert_eq!(moved.iter().map(|s| (s.range.row, s.range.rows)).collect::<Vec<_>>(), vec![(1, 1), (3, 1)]);
        assert_eq!(moved[0].fill, styled.styles[0].fill);
        assert_eq!(moved[1].fill, styled.styles[0].fill);
        styled.styles[0].range.rows = 1;
        assert!(keyed_styles(&styled, &copy, &reordered[..2], 0)
            .unwrap_err().contains("silindi"));
    }
}

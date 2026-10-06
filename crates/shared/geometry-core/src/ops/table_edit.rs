//! Tabloyu düzenle's edits and Tabloyu güncelle's rule (docs/adr/0184 §5,
//! §6), one for both platforms (the web through its WASM: `tableEdit`,
//! `tableRefresh`). The independent reference is
//! scripts/fixtures/table_cases.py (fixtures/table/v1/cases.json, `edits`
//! and `refresh`).
//!
//! - A cell's words: one line (`table::one_line`); a cell inside a merged
//!   range but its top left takes none.
//! - Rows inserted before row `at` (`at` = the count: after the last) are
//!   empty and as high as the row now there (the last one's after it); a
//!   merged range below them moves down, one they fall inside grows. Rows
//!   deleted: at least one stays; a range loses the rows it had among them,
//!   one starting below them moves up, one whose top row went keeps its
//!   words in its first row that stays; a range of one cell is no range.
//!   Columns alike, a new column aligned left when the table has alignments.
//! - Merging a range of two cells or more inside the table: the ranges inside
//!   it are absorbed, one it cuts across refuses it; its cells' words, row by
//!   row, the empty ones left out, joined by a space, are its top left's
//!   (one line), the others' none; it is the last range. Unmerging the range
//!   a cell is in leaves its words in its top left.
//! - Tabloyu güncelle: the source's new cells; a row kept keeps its height,
//!   a new one is `ROW` text heights; a column kept is as wide as it was or
//!   as its new words ask (`fit_widths`), whichever is wider, a new one as
//!   they ask; the columns' alignment the table's while it has as many
//!   columns, else the source's; a merged range kept while it fits and its
//!   cells but the top left are empty; the rest of the table as it was.

use crate::api::Op;
use crate::api::json::{FromJson, Json, read_field};
use crate::entity::{CellRange, Shape};
use crate::geom::table::{fit_widths, table_geom};
use crate::jsmath::js_max;
use crate::op;
use crate::ops::table::{LEAST, MAX_CELLS, MAX_COLUMNS, MAX_ROWS, ROW, one_line};
use crate::text::Font;

/// One of Tabloyu düzenle's edits.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    SetCell {
        row: usize,
        col: usize,
        words: String,
    },
    InsertRows {
        at: usize,
        count: usize,
    },
    DeleteRows {
        from: usize,
        count: usize,
    },
    InsertColumns {
        at: usize,
        count: usize,
    },
    DeleteColumns {
        from: usize,
        count: usize,
    },
    Merge(CellRange),
    Unmerge {
        row: usize,
        col: usize,
    },
}

impl FromJson for Edit {
    fn from_json(v: &Json) -> Result<Edit, String> {
        let kind: String = read_field(v, "kind")?;
        let n = |k: &str| -> Result<usize, String> {
            let x: f64 = read_field(v, k)?;
            if x.is_finite() && x >= 0.0 && x.fract() == 0.0 && x <= 1e9 {
                Ok(x as usize)
            } else {
                Err(format!("“{k}” bir sıra sayısı olmalı"))
            }
        };
        Ok(match kind.as_str() {
            "setCell" => Edit::SetCell {
                row: n("row")?,
                col: n("col")?,
                words: read_field(v, "words")?,
            },
            "insertRows" => Edit::InsertRows {
                at: n("at")?,
                count: n("count")?,
            },
            "deleteRows" => Edit::DeleteRows {
                from: n("from")?,
                count: n("count")?,
            },
            "insertColumns" => Edit::InsertColumns {
                at: n("at")?,
                count: n("count")?,
            },
            "deleteColumns" => Edit::DeleteColumns {
                from: n("from")?,
                count: n("count")?,
            },
            "merge" => Edit::Merge(CellRange {
                row: n("row")?,
                col: n("col")?,
                rows: n("rows")?,
                cols: n("cols")?,
            }),
            "unmerge" => Edit::Unmerge {
                row: n("row")?,
                col: n("col")?,
            },
            other => return Err(format!("“{other}” diye bir tablo düzenlemesi yok")),
        })
    }
}

/// An edit's table, or why it is refused.
#[derive(Clone, Debug, PartialEq)]
pub struct Edited {
    pub table: Option<Shape>,
    pub problem: Option<String>,
}

crate::json_struct!(out Edited { table, problem });

impl Edited {
    fn refused(problem: &str) -> Edited {
        Edited {
            table: None,
            problem: Some(problem.to_owned()),
        }
    }
}

/// A table's fields as an edit changes them.
struct Parts {
    rows: Vec<f64>,
    columns: Vec<f64>,
    cells: Vec<Vec<String>>,
    merges: Vec<CellRange>,
    aligns: Option<Vec<String>>,
}

impl Parts {
    fn of(s: &Shape) -> Option<Parts> {
        let Shape::Table {
            rows,
            columns,
            cells,
            merges,
            aligns,
            ..
        } = s
        else {
            return None;
        };
        Some(Parts {
            rows: rows.clone(),
            columns: columns.clone(),
            cells: cells.clone(),
            merges: merges.clone().unwrap_or_default(),
            aligns: aligns.clone(),
        })
    }

    /// The table `s` with these fields.
    fn into_shape(self, s: &Shape) -> Shape {
        let mut out = s.clone();
        if let Shape::Table {
            rows,
            columns,
            cells,
            merges,
            aligns,
            ..
        } = &mut out
        {
            *rows = self.rows;
            *columns = self.columns;
            *cells = self.cells;
            *merges = (!self.merges.is_empty()).then_some(self.merges);
            *aligns = self.aligns;
        }
        out
    }

    fn merge_at(&self, row: usize, col: usize) -> Option<usize> {
        self.merges.iter().position(|r| r.holds(row, col))
    }
}

/// How a range's span along one axis meets `count` lines inserted before
/// `at`: its new start and length.
fn grown(start: usize, len: usize, at: usize, count: usize) -> (usize, usize) {
    if start >= at {
        (start + count, len)
    } else if at < start + len {
        (start, len + count)
    } else {
        (start, len)
    }
}

/// How a range's span along one axis meets lines `from..from + count`
/// deleted: its new start and length (0: gone).
fn shrunk(start: usize, len: usize, from: usize, count: usize) -> (usize, usize) {
    let end = start + len;
    let gone = end.min(from + count).saturating_sub(start.max(from));
    let new_start = if start >= from + count {
        start - count
    } else if start >= from {
        from
    } else {
        start
    };
    (new_start, len - gone)
}

/// `t` with edit `e`, or why not.
pub fn edit(t: &Shape, e: &Edit) -> Edited {
    let Some(mut p) = Parts::of(t) else {
        return Edited::refused("Tablo değil.");
    };
    let (n, m) = (p.rows.len(), p.columns.len());
    match e {
        Edit::SetCell { row, col, words } => {
            let (row, col) = (*row, *col);
            if row >= n || col >= m {
                return Edited::refused("Hücre tablonun dışında.");
            }
            if let Some(k) = p.merge_at(row, col) {
                let r = p.merges[k];
                if (r.row, r.col) != (row, col) {
                    return Edited::refused(
                        "Bu hücre birleşik bir alanın içinde; yazısı sol üst hücresindedir.",
                    );
                }
            }
            p.cells[row][col] = one_line(words).words;
        }
        Edit::InsertRows { at, count } => {
            let (at, count) = (*at, *count);
            if at > n || count == 0 {
                return Edited::refused("Satırların yeri tablonun dışında.");
            }
            if n + count > MAX_ROWS || (n + count) * m > MAX_CELLS {
                return Edited::refused("Tablo bu kadar satır alamaz.");
            }
            let height = p.rows[at.min(n - 1)];
            p.rows.splice(at..at, std::iter::repeat_n(height, count));
            p.cells
                .splice(at..at, std::iter::repeat_n(vec![String::new(); m], count));
            for r in &mut p.merges {
                (r.row, r.rows) = grown(r.row, r.rows, at, count);
            }
        }
        Edit::DeleteRows { from, count } => {
            let (from, count) = (*from, *count);
            if count == 0 || from + count > n {
                return Edited::refused("Silinecek satırlar tablonun dışında.");
            }
            if count == n {
                return Edited::refused("Tablonun en az bir satırı kalmalı.");
            }
            let before = std::mem::take(&mut p.merges);
            let mut moved: Vec<(CellRange, String)> = Vec::new();
            for r in before {
                let words = p.cells[r.row][r.col].clone();
                let lost_top = r.row >= from && r.row < from + count;
                let (row, rows) = shrunk(r.row, r.rows, from, count);
                if rows == 0 {
                    continue;
                }
                moved.push((
                    CellRange { row, rows, ..r },
                    if lost_top { words } else { String::new() },
                ));
            }
            p.rows.drain(from..from + count);
            p.cells.drain(from..from + count);
            for (r, words) in moved {
                if !words.is_empty() {
                    p.cells[r.row][r.col] = words;
                }
                if r.rows * r.cols >= 2 {
                    p.merges.push(r);
                }
            }
        }
        Edit::InsertColumns { at, count } => {
            let (at, count) = (*at, *count);
            if at > m || count == 0 {
                return Edited::refused("Sütunların yeri tablonun dışında.");
            }
            if m + count > MAX_COLUMNS || n * (m + count) > MAX_CELLS {
                return Edited::refused("Tablo bu kadar sütun alamaz.");
            }
            let width = p.columns[at.min(m - 1)];
            p.columns.splice(at..at, std::iter::repeat_n(width, count));
            for row in &mut p.cells {
                row.splice(at..at, std::iter::repeat_n(String::new(), count));
            }
            if let Some(a) = &mut p.aligns {
                a.splice(at..at, std::iter::repeat_n("left".to_owned(), count));
            }
            for r in &mut p.merges {
                (r.col, r.cols) = grown(r.col, r.cols, at, count);
            }
        }
        Edit::DeleteColumns { from, count } => {
            let (from, count) = (*from, *count);
            if count == 0 || from + count > m {
                return Edited::refused("Silinecek sütunlar tablonun dışında.");
            }
            if count == m {
                return Edited::refused("Tablonun en az bir sütunu kalmalı.");
            }
            let before = std::mem::take(&mut p.merges);
            let mut moved: Vec<(CellRange, String)> = Vec::new();
            for r in before {
                let words = p.cells[r.row][r.col].clone();
                let lost_left = r.col >= from && r.col < from + count;
                let (col, cols) = shrunk(r.col, r.cols, from, count);
                if cols == 0 {
                    continue;
                }
                moved.push((
                    CellRange { col, cols, ..r },
                    if lost_left { words } else { String::new() },
                ));
            }
            p.columns.drain(from..from + count);
            for row in &mut p.cells {
                row.drain(from..from + count);
            }
            if let Some(a) = &mut p.aligns {
                a.drain(from..from + count);
            }
            for (r, words) in moved {
                if !words.is_empty() {
                    p.cells[r.row][r.col] = words;
                }
                if r.rows * r.cols >= 2 {
                    p.merges.push(r);
                }
            }
        }
        Edit::Merge(r) => {
            if r.rows == 0 || r.cols == 0 || r.row + r.rows > n || r.col + r.cols > m {
                return Edited::refused("Birleştirilecek alan tablonun dışında.");
            }
            if r.rows * r.cols < 2 {
                return Edited::refused("Birleştirmek için birden çok hücre seçin.");
            }
            let inside = |o: &CellRange| {
                o.row >= r.row
                    && o.col >= r.col
                    && o.row + o.rows <= r.row + r.rows
                    && o.col + o.cols <= r.col + r.cols
            };
            let meets = |o: &CellRange| {
                o.row < r.row + r.rows
                    && r.row < o.row + o.rows
                    && o.col < r.col + r.cols
                    && r.col < o.col + o.cols
            };
            if p.merges.iter().any(|o| meets(o) && !inside(o)) {
                return Edited::refused(
                    "Seçilen alan birleşik bir alanı yarıda kesiyor; önce onu ayırın ya da tamamını seçin.",
                );
            }
            p.merges.retain(|o| !inside(o));
            let mut words: Vec<String> = Vec::new();
            for i in r.row..r.row + r.rows {
                for j in r.col..r.col + r.cols {
                    let w = std::mem::take(&mut p.cells[i][j]);
                    if !w.is_empty() {
                        words.push(w);
                    }
                }
            }
            p.cells[r.row][r.col] = one_line(&words.join(" ")).words;
            p.merges.push(*r);
        }
        Edit::Unmerge { row, col } => {
            let Some(k) = p.merge_at(*row, *col) else {
                return Edited::refused("Bu hücre birleşik değil.");
            };
            p.merges.remove(k);
        }
    }
    Edited {
        table: Some(p.into_shape(t)),
        problem: None,
    }
}

/// `t` with a source's new `cells` and their columns' `aligns`
/// (Tabloyu güncelle), its words measured in `font`; none for any other shape.
pub fn refresh(t: &Shape, cells: &[Vec<String>], aligns: &[String], font: Font) -> Option<Shape> {
    let old = Parts::of(t)?;
    let Shape::Table { height, .. } = t else {
        return None;
    };
    let h = *height;
    let (n, m) = (cells.len(), cells.first().map_or(0, Vec::len));
    let kept_aligns = if m == old.columns.len() {
        old.aligns.clone()
    } else {
        aligns.iter().any(|a| a != "left").then(|| aligns.to_vec())
    };
    let merges: Vec<CellRange> = old
        .merges
        .iter()
        .filter(|r| {
            r.row + r.rows <= n
                && r.col + r.cols <= m
                && (r.row..r.row + r.rows).all(|i| {
                    (r.col..r.col + r.cols)
                        .all(|j| (i, j) == (r.row, r.col) || cells[i][j].is_empty())
                })
        })
        .copied()
        .collect();
    let next = Parts {
        rows: (0..n)
            .map(|i| old.rows.get(i).copied().unwrap_or(ROW * h))
            .collect(),
        columns: vec![LEAST * h; m],
        cells: cells.to_vec(),
        merges,
        aligns: kept_aligns,
    };
    let mut out = next.into_shape(t);
    let fitted = fit_widths(&table_geom(&out)?, font, LEAST * h);
    if let Shape::Table { columns, .. } = &mut out {
        *columns = (0..m)
            .map(|j| {
                old.columns
                    .get(j)
                    .map_or(fitted[j], |&w| js_max(w, fitted[j]))
            })
            .collect();
    }
    Some(out)
}

pub(crate) static OPS: &[Op] = &[
    op!("tableEdit", |table: Shape, edit: Edit| self::edit(
        &table, &edit
    )),
    op!("tableRefresh", |table: Shape,
                         cells: Vec<Vec<String>>,
                         aligns: Vec<String>,
                         font: String| {
        refresh(&table, &cells, &aligns, Font::from_id(&font))
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> Json {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/table/v1/cases.json"
        );
        Json::parse(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
    }

    fn list<'a>(f: &'a Json, k: &str) -> &'a [Json] {
        match f.get(k) {
            Json::Arr(a) => a,
            _ => panic!("{k}"),
        }
    }

    fn text(v: &Json) -> String {
        match v {
            Json::Str(s) => s.clone(),
            _ => String::new(),
        }
    }

    /// The fields an edit changes, as the shared cases write them.
    fn as_said(s: &Shape) -> Json {
        let Shape::Table {
            rows,
            columns,
            cells,
            merges,
            aligns,
            ..
        } = s
        else {
            panic!("a table");
        };
        use crate::api::json::ToJson;
        let mut out = String::from("[");
        rows.write_json(&mut out);
        out.push(',');
        columns.write_json(&mut out);
        out.push(',');
        cells.write_json(&mut out);
        out.push(',');
        merges.clone().unwrap_or_default().write_json(&mut out);
        out.push(',');
        aligns.write_json(&mut out);
        out.push(']');
        Json::parse(&out).expect("JSON")
    }

    fn wanted(w: &Json) -> Json {
        Json::Arr(vec![
            w.get("rows").clone(),
            w.get("columns").clone(),
            w.get("cells").clone(),
            w.get("merges").clone(),
            w.get("aligns").clone(),
        ])
    }

    fn close(a: &Json, b: &Json) -> bool {
        match (a, b) {
            (Json::Num(x), Json::Num(y)) => (x - y).abs() <= 1e-9 * (1.0 + y.abs()),
            (Json::Arr(x), Json::Arr(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(a, b)| close(a, b))
            }
            (Json::Obj(x), Json::Obj(y)) => {
                x.len() == y.len()
                    && x.iter().all(|(k, a)| {
                        close(
                            a,
                            y.iter()
                                .find(|(l, _)| l == k)
                                .map_or(&Json::Null, |(_, b)| b),
                        )
                    })
            }
            _ => a == b,
        }
    }

    /// The shared cases (fixtures/table/v1/cases.json `edits`, `refresh`),
    /// written by scripts/fixtures/table_cases.py from docs/adr/0184, not
    /// from this code; the web runs them through WASM (model/ops/table.test.ts).
    #[test]
    fn edits_and_refreshes_as_the_shared_cases_say() {
        let f = file();
        for c in list(&f, "edits") {
            let name = text(c.get("name"));
            let table = Shape::from_json(c.get("table")).expect("a table");
            let e = Edit::from_json(c.get("edit")).expect("an edit");
            let got = edit(&table, &e);
            let want = c.get("want");
            match (&got.table, want.get("problem")) {
                (None, Json::Str(why)) => {
                    assert_eq!(got.problem.as_deref(), Some(why.as_str()), "{name}")
                }
                (Some(t), Json::Null) => {
                    let (g, w) = (as_said(t), wanted(want));
                    assert!(close(&g, &w), "{name}:\n{g:?}\n≠\n{w:?}");
                }
                other => panic!("{name}: {other:?} {:?}", got.problem),
            }
        }
        for c in list(&f, "refresh") {
            let name = text(c.get("name"));
            let table = Shape::from_json(c.get("table")).expect("a table");
            let cells = Vec::<Vec<String>>::from_json(c.get("cells")).expect("cells");
            let aligns = Vec::<String>::from_json(c.get("aligns")).expect("aligns");
            let got = refresh(&table, &cells, &aligns, Font::from_id(&text(c.get("font"))))
                .expect("a table");
            let (g, w) = (as_said(&got), wanted(c.get("want")));
            assert!(close(&g, &w), "{name}:\n{g:?}\n≠\n{w:?}");
        }
    }
}

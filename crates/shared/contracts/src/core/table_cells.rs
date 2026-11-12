//! A table's cells beyond their words (docs/adr/0327 §1–§3, §5): their styles (alignment, bold, colour, fill,
//! decimals of a worked out value, over ranges, a later style over an earlier), their formulas (table_formula.rs),
//! their fields (a source's value as the cell's words, ADR 0325's sources), and what a file source keeps of the file
//! (its path, the digest of the bytes last read, their cells, and what is done with a cell changed by hand).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::table::{CellRange, TableAlign, TableEntity};
use crate::table_formula::{MAX_DECIMALS, is_formula};
use crate::text_field::{FieldBroken, FieldFormat, FieldSource};

/// The most styles, formulas and fields a table has.
pub const MAX_CELL_STYLES: usize = 1000;
pub const MAX_FORMULAS: usize = 10_000;
pub const MAX_CELL_FIELDS: usize = 1000;
/// The most letters of a formula.
pub const MAX_FORMULA_LETTERS: usize = 1000;

/// A style over a range of cells (docs/adr/0327 §1); what it leaves absent the cells take from the styles before it,
/// then from the table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CellStyle {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub range: CellRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub align: Option<TableAlign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bold: Option<bool>,
    /// Its words' colour, `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Its background, `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    /// The decimals a formula's or a field's number is written with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(max = 10)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub decimals: Option<u32>,
}

/// A cell's formula (docs/adr/0327 §2): its words begin with `=`; the cell's words are its result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CellFormula {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub row: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub col: u32,
    pub formula: String,
}

/// A cell bound to a source (docs/adr/0327 §3): its words are the value last written, `####` when broken.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CellField {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub row: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub col: u32,
    pub source: FieldSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub format: Option<FieldFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<FieldBroken>,
}

/// What Tabloyu güncelle does with a cell changed by hand since its file was last read (docs/adr/0327 §5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SourceOverrides {
    /// Kept while the file's cell there stays as it was.
    #[default]
    Keep,
    /// The file's words written over it.
    Replace,
}

impl SourceOverrides {
    pub fn is_keep(&self) -> bool {
        *self == SourceOverrides::Keep
    }
}

/// A cell's style as its styles resolve it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolvedStyle {
    pub align: Option<TableAlign>,
    pub bold: Option<bool>,
    pub color: Option<String>,
    pub fill: Option<String>,
    pub decimals: Option<u32>,
}

impl TableEntity {
    /// The style of the cell at `row`, `col`: each field from the last style over it that sets it.
    pub fn cell_style(&self, row: usize, col: usize) -> ResolvedStyle {
        let mut out = ResolvedStyle::default();
        for s in &self.styles {
            if !s.range.holds(row, col) {
                continue;
            }
            if s.align.is_some() {
                out.align = s.align;
            }
            if s.bold.is_some() {
                out.bold = s.bold;
            }
            if s.color.is_some() {
                out.color = s.color.clone();
            }
            if s.fill.is_some() {
                out.fill = s.fill.clone();
            }
            if s.decimals.is_some() {
                out.decimals = s.decimals;
            }
        }
        out
    }

    /// The formula of the cell at `row`, `col`, if any.
    pub fn formula_at(&self, row: usize, col: usize) -> Option<&CellFormula> {
        self.formulas
            .iter()
            .find(|f| (f.row as usize, f.col as usize) == (row, col))
    }

    /// The field of the cell at `row`, `col`, if any.
    pub fn field_at(&self, row: usize, col: usize) -> Option<&CellField> {
        self.fields
            .iter()
            .find(|f| (f.row as usize, f.col as usize) == (row, col))
    }
}

impl TableEntity {
    /// Its merged ranges, styles, formulas and fields kept to `n` rows and `m` columns: those past them dropped, a
    /// style cut to them (a range a schedule or a file no longer reaches).
    pub fn clip_to(&mut self, n: usize, m: usize) {
        let fits = |r: &CellRange| r.row as usize + r.rows as usize <= n && r.col as usize + r.cols as usize <= m;
        self.merges.retain(fits);
        self.styles.retain_mut(|s| {
            let r = &mut s.range;
            if r.row as usize >= n || r.col as usize >= m {
                return false;
            }
            r.rows = r.rows.min((n - r.row as usize) as u32);
            r.cols = r.cols.min((m - r.col as usize) as u32);
            true
        });
        self.formulas
            .retain(|f| (f.row as usize) < n && (f.col as usize) < m);
        self.fields
            .retain(|f| (f.row as usize) < n && (f.col as usize) < m);
    }
}

/// Whether `c` is a colour `#rrggbb`.
pub fn is_colour(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// What is wrong with a table's styles, formulas and fields for its rows and columns and its merged ranges: the
/// field and the refusal's words.
pub fn cells_problem(t: &TableEntity) -> Option<(&'static str, String)> {
    cells_problem_of(t.rows.len(), t.columns.len(), &t.merges, &t.styles, &t.formulas, &t.fields)
}

/// [`cells_problem`] of `n` rows and `m` columns, their merged ranges, styles, formulas and fields.
pub fn cells_problem_of(
    n: usize,
    m: usize,
    merges: &[CellRange],
    styles: &[CellStyle],
    formulas: &[CellFormula],
    fields: &[CellField],
) -> Option<(&'static str, String)> {
    let inside = |r: u32, c: u32| (r as usize) < n && (c as usize) < m;
    let hidden = |r: u32, c: u32| {
        merges
            .iter()
            .find(|g| g.holds(r as usize, c as usize))
            .is_some_and(|g| (g.row, g.col) != (r, c))
    };
    if styles.len() > MAX_CELL_STYLES {
        return Some((
            "styles",
            format!("Bir tabloda en çok {MAX_CELL_STYLES} biçim olur; {} verildi.", styles.len()),
        ));
    }
    for (k, s) in styles.iter().enumerate() {
        let r = s.range;
        if r.rows == 0
            || r.cols == 0
            || r.row as usize + r.rows as usize > n
            || r.col as usize + r.cols as usize > m
        {
            return Some((
                "styles",
                format!(
                    "{}. biçimin aralığı ({}. satır, {}. sütundan {} × {}) tablonun içinde olmalı.",
                    k + 1,
                    r.row + 1,
                    r.col + 1,
                    r.rows,
                    r.cols
                ),
            ));
        }
        for (what, c) in [("Yazı rengi", &s.color), ("Dolgu", &s.fill)] {
            if let Some(c) = c
                && !is_colour(c)
            {
                return Some((
                    "styles",
                    format!("{}. biçimin {what} “{c}”; #rrggbb olmalı.", k + 1),
                ));
            }
        }
        if s.decimals.is_some_and(|d| d > MAX_DECIMALS as u32) {
            return Some((
                "styles",
                format!("{}. biçimin basamağı en çok {MAX_DECIMALS} olur.", k + 1),
            ));
        }
        if s.align.is_none() && s.bold.is_none() && s.color.is_none() && s.fill.is_none() && s.decimals.is_none() {
            return Some((
                "styles",
                format!("{}. biçim hiçbir şey vermiyor; boş biçim yazılmaz.", k + 1),
            ));
        }
    }
    if formulas.len() > MAX_FORMULAS {
        return Some((
            "formulas",
            format!("Bir tabloda en çok {MAX_FORMULAS} formül olur; {} verildi.", formulas.len()),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for f in formulas {
        let at = format!("{}. satırın {}. hücresi", f.row + 1, f.col + 1);
        if !inside(f.row, f.col) {
            return Some(("formulas", format!("Formülün hücresi ({at}) tablonun içinde olmalı.")));
        }
        if hidden(f.row, f.col) {
            return Some((
                "formulas",
                format!("Formülün hücresi ({at}) birleşik bir alanın içinde; formül alanın sol üst hücresine yazılır."),
            ));
        }
        if !is_formula(&f.formula)
            || f.formula.chars().count() > MAX_FORMULA_LETTERS
            || f.formula.chars().any(char::is_control)
        {
            return Some((
                "formulas",
                format!("{at}: formül “=” ile başlayan, tek satırlık ve en çok {MAX_FORMULA_LETTERS} harflik bir yazıdır."),
            ));
        }
        if !seen.insert((f.row, f.col)) {
            return Some(("formulas", format!("{at} için iki formül verildi.")));
        }
    }
    if fields.len() > MAX_CELL_FIELDS {
        return Some((
            "fields",
            format!("Bir tabloda en çok {MAX_CELL_FIELDS} alan olur; {} verildi.", fields.len()),
        ));
    }
    let mut fielded = std::collections::HashSet::new();
    for f in fields {
        let at = format!("{}. satırın {}. hücresi", f.row + 1, f.col + 1);
        if !inside(f.row, f.col) || hidden(f.row, f.col) {
            return Some((
                "fields",
                format!("Alanın hücresi ({at}) tablonun içinde ve birleşik alanın sol üst hücresi olmalı."),
            ));
        }
        if seen.contains(&(f.row, f.col)) {
            return Some(("fields", format!("{at} hem formül hem alan olamaz.")));
        }
        if !fielded.insert((f.row, f.col)) {
            return Some(("fields", format!("{at} için iki alan verildi.")));
        }
        if let Some(why) = f.source.problem() {
            return Some(("fields", format!("{at}: {why}")));
        }
        if let Some(why) = f.format.as_ref().and_then(FieldFormat::problem) {
            return Some(("fields", format!("{at}: {why}")));
        }
    }
    None
}

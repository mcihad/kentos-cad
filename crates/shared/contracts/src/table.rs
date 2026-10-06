//! Tables in the drawing (docs/adr/0184): rows and columns of one-line cells
//! hanging from their top left corner, turned with it. A table carries its
//! cells' words, its rows' heights and columns' widths, its merged ranges,
//! its columns' alignment, its heading row, which of its lines are drawn and
//! how wide its frame is, its cells' face (docs/adr/0183 §2) and, when its
//! rows came from objects or a file, that source (Tabloyu güncelle writes
//! them again).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{EntityBase, EntityId, TextFace, Vec2};

/// The most rows and columns a table may have, and cells in all.
pub const MAX_TABLE_ROWS: usize = 10_000;
pub const MAX_TABLE_COLUMNS: usize = 100;
pub const MAX_TABLE_CELLS: usize = 100_000;
/// The most letters a cell may hold.
pub const MAX_CELL_LETTERS: usize = 1000;
/// The longest a row or a column may be, and the largest text height, metres.
pub const MAX_TABLE_SIZE: f64 = 1.0e6;
/// The most objects a table's source may name.
pub const MAX_SOURCE_OBJECTS: usize = 100_000;

/// A table (docs/adr/0184 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    /// Its top left corner.
    pub p: Vec2,
    /// Degrees counter-clockwise from east: its rows run along it.
    pub rotation: f64,
    /// Its cells' text height, metres.
    pub height: f64,
    /// Its rows' heights, top to bottom, metres.
    pub rows: Vec<f64>,
    /// Its columns' widths, left to right, metres.
    pub columns: Vec<f64>,
    /// The cells' words, row by row, a row as many as there are columns;
    /// one line each, empty for an empty cell.
    pub cells: Vec<Vec<String>>,
    /// Merged ranges: each range's words in its top left cell, its others empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CellRange>>", optional))]
    pub merges: Vec<CellRange>,
    /// Each column's alignment, as many as there are columns; absent: all left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub aligns: Option<Vec<TableAlign>>,
    /// The first row is its heading: bold, centred.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub header: bool,
    /// Which of its lines are drawn; absent: all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grid: Option<TableGrid>,
    /// Its frame's width, metres: the outline drawn as a band that wide
    /// inside it (Kalın çerçeve); absent: a line as the others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub frame: Option<f64>,
    /// Its cells' style and face (docs/adr/0183 §2).
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub face: TextFace,
    /// Where its rows came from (Tabloyu güncelle); absent: written by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<TableSource>,
}

/// A merged range: `rows` × `cols` cells from row `row`, column `col`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CellRange {
    pub row: u32,
    pub col: u32,
    pub rows: u32,
    pub cols: u32,
}

impl CellRange {
    /// Whether the cell at `row`, `col` is in it.
    pub fn holds(&self, row: usize, col: usize) -> bool {
        let (r, c) = (self.row as usize, self.col as usize);
        row >= r && row < r + self.rows as usize && col >= c && col < c + self.cols as usize
    }
}

/// A column's alignment; its cells sit in the middle of their rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TableAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TableAlign {
    pub const ALL: [TableAlign; 3] = [TableAlign::Left, TableAlign::Center, TableAlign::Right];

    pub fn name(self) -> &'static str {
        match self {
            TableAlign::Left => "left",
            TableAlign::Center => "center",
            TableAlign::Right => "right",
        }
    }

    pub fn from_name(name: &str) -> Option<TableAlign> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }
}

/// Which of a table's lines are drawn, when not all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TableGrid {
    /// Its outline alone.
    Outer,
    /// Its outline and the lines between its rows.
    Rows,
    /// None: the words alone.
    None,
}

impl TableGrid {
    pub const ALL: [TableGrid; 3] = [TableGrid::Outer, TableGrid::Rows, TableGrid::None];

    pub fn name(self) -> &'static str {
        match self {
            TableGrid::Outer => "outer",
            TableGrid::Rows => "rows",
            TableGrid::None => "none",
        }
    }

    pub fn from_name(name: &str) -> Option<TableGrid> {
        Self::ALL.into_iter().find(|g| g.name() == name)
    }
}

/// Where a table's rows came from (docs/adr/0184 §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TableSource {
    /// Koordinat çizelgesi: the points and vertices of these objects.
    Coordinates { objects: Vec<EntityId> },
    /// Alan çizelgesi: these areas, their area and perimeter.
    Areas { objects: Vec<EntityId> },
    /// Öznitelik tablosu: these objects' attributes.
    Attributes { objects: Vec<EntityId> },
    /// A CSV or Excel file by its name (and sheet): it is chosen again to update.
    File {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        sheet: Option<String>,
    },
}

impl TableSource {
    /// Its kind's name in the contract (`coordinates` …).
    pub fn kind(&self) -> &'static str {
        match self {
            TableSource::Coordinates { .. } => "coordinates",
            TableSource::Areas { .. } => "areas",
            TableSource::Attributes { .. } => "attributes",
            TableSource::File { .. } => "file",
        }
    }

    /// The objects it names; none for a file.
    pub fn objects(&self) -> &[EntityId] {
        match self {
            TableSource::Coordinates { objects }
            | TableSource::Areas { objects }
            | TableSource::Attributes { objects } => objects,
            TableSource::File { .. } => &[],
        }
    }
}

/// Whether `x` may be a table's length or height: finite, over 0, at most `MAX_TABLE_SIZE`.
fn size_holds(x: f64) -> bool {
    x.is_finite() && x > 0.0 && x <= MAX_TABLE_SIZE
}

/// What is wrong with a cell's words, if anything: a line break or control
/// character, too many letters.
pub fn cell_problem(words: &str) -> Option<String> {
    if words.chars().any(char::is_control) {
        return Some("satır sonu ya da denetim karakteri var; hücre tek satırdır".to_owned());
    }
    let n = words.chars().count();
    (n > MAX_CELL_LETTERS).then(|| format!("{n} harf; en çok {MAX_CELL_LETTERS}"))
}

/// The table's shape as the fields say it (docs/adr/0184 §1), for `problem`.
pub struct TableShape<'a> {
    pub height: f64,
    pub rows: &'a [f64],
    pub columns: &'a [f64],
    pub cells: &'a [Vec<String>],
    pub merges: &'a [CellRange],
    pub aligns: Option<&'a [TableAlign]>,
    pub frame: Option<f64>,
    pub source: Option<&'a TableSource>,
}

impl TableShape<'_> {
    /// What is wrong with it: the field and the refusal's words; none when it
    /// may be written. Its rows and columns in their bounds, each cell's words
    /// one line, merged ranges inside it, of more than one cell, apart, their
    /// other cells empty; as many alignments as columns; a source of objects
    /// within its bounds, a file's name not empty.
    pub fn problem(&self) -> Option<(&'static str, String)> {
        if !size_holds(self.height) {
            return Some((
                "height",
                format!(
                    "Tablonun yazı yüksekliği {}; sıfırdan büyük ve sonlu olmalı.",
                    self.height
                ),
            ));
        }
        let (n, m) = (self.rows.len(), self.columns.len());
        if n == 0 || n > MAX_TABLE_ROWS {
            return Some((
                "rows",
                format!("Tablonun {n} satırı var; en az 1, en çok {MAX_TABLE_ROWS} olmalı."),
            ));
        }
        if m == 0 || m > MAX_TABLE_COLUMNS {
            return Some((
                "columns",
                format!("Tablonun {m} sütunu var; en az 1, en çok {MAX_TABLE_COLUMNS} olmalı."),
            ));
        }
        if n * m > MAX_TABLE_CELLS {
            return Some((
                "cells",
                format!(
                    "Tablonun {} hücresi var; en çok {MAX_TABLE_CELLS} olmalı. Tabloyu bölün.",
                    n * m
                ),
            ));
        }
        if let Some((i, h)) = self.rows.iter().enumerate().find(|(_, h)| !size_holds(**h)) {
            return Some((
                "rows",
                format!(
                    "Tablonun {}. satırının yüksekliği {h}; sıfırdan büyük ve sonlu olmalı.",
                    i + 1
                ),
            ));
        }
        if let Some((i, w)) = self
            .columns
            .iter()
            .enumerate()
            .find(|(_, w)| !size_holds(**w))
        {
            return Some((
                "columns",
                format!(
                    "Tablonun {}. sütununun genişliği {w}; sıfırdan büyük ve sonlu olmalı.",
                    i + 1
                ),
            ));
        }
        if self.cells.len() != n {
            return Some((
                "cells",
                format!(
                    "Tablonun {n} satırı var ama {} satırlık hücre verildi; her satırın hücreleri verilmeli.",
                    self.cells.len()
                ),
            ));
        }
        for (i, row) in self.cells.iter().enumerate() {
            if row.len() != m {
                return Some((
                    "cells",
                    format!(
                        "Tablonun {}. satırında {} hücre var; sütun sayısı kadar ({m}) olmalı.",
                        i + 1,
                        row.len()
                    ),
                ));
            }
            for (j, words) in row.iter().enumerate() {
                if let Some(why) = cell_problem(words) {
                    return Some((
                        "cells",
                        format!("{}. satırın {}. hücresi: {why}.", i + 1, j + 1),
                    ));
                }
            }
        }
        for (k, r) in self.merges.iter().enumerate() {
            let (row, col, rows, cols) = (
                r.row as usize,
                r.col as usize,
                r.rows as usize,
                r.cols as usize,
            );
            if rows == 0 || cols == 0 || rows * cols < 2 || row + rows > n || col + cols > m {
                return Some((
                    "merges",
                    format!(
                        "{}. birleşik alan ({}. satır, {}. sütundan {rows} × {cols}) tablonun içinde ve birden çok hücre olmalı.",
                        k + 1,
                        row + 1,
                        col + 1
                    ),
                ));
            }
            if let Some(other) = self.merges[..k].iter().find(|o| {
                o.row < r.row + r.rows
                    && r.row < o.row + o.rows
                    && o.col < r.col + r.cols
                    && r.col < o.col + o.cols
            }) {
                return Some((
                    "merges",
                    format!(
                        "{}. birleşik alan, {}. satır {}. sütundaki birleşik alanla örtüşüyor; birleşik alanlar ayrı olmalı.",
                        k + 1,
                        other.row + 1,
                        other.col + 1
                    ),
                ));
            }
            for i in row..row + rows {
                for j in col..col + cols {
                    if (i, j) != (row, col) && !self.cells[i][j].is_empty() {
                        return Some((
                            "merges",
                            format!(
                                "{}. satırın {}. hücresi birleşik bir alanın içinde ama boş değil; birleşik alanın yazısı sol üst hücresindedir.",
                                i + 1,
                                j + 1
                            ),
                        ));
                    }
                }
            }
        }
        // A frame is a band inside the outline: narrower than half its width and depth.
        if let Some(f) = self.frame {
            let (w, d): (f64, f64) = (self.columns.iter().sum(), self.rows.iter().sum());
            if !(f.is_finite() && f > 0.0 && 2.0 * f < w.min(d)) {
                return Some((
                    "frame",
                    format!(
                        "Tablonun çerçeve kalınlığı {f}; sıfırdan büyük, tablonun eninin ve boyunun yarısından küçük olmalı."
                    ),
                ));
            }
        }
        if let Some(a) = self.aligns
            && a.len() != m
        {
            return Some((
                "aligns",
                format!(
                    "Tablonun {m} sütunu var ama {} hiza verildi; her sütunun hizası verilmeli.",
                    a.len()
                ),
            ));
        }
        match self.source {
            Some(TableSource::File { name, sheet }) => {
                if name.trim().is_empty() || name.chars().any(char::is_control) {
                    return Some((
                        "source",
                        "Tablonun kaynağı olan dosyanın adı boş olamaz.".to_owned(),
                    ));
                }
                if sheet.as_deref().is_some_and(|s| s.trim().is_empty()) {
                    return Some((
                        "source",
                        "Tablonun kaynağı olan sayfanın adı boş olamaz.".to_owned(),
                    ));
                }
            }
            Some(s) if s.objects().is_empty() || s.objects().len() > MAX_SOURCE_OBJECTS => {
                return Some((
                    "source",
                    format!(
                        "Tablonun kaynağı {} nesne gösteriyor; en az 1, en çok {MAX_SOURCE_OBJECTS} olmalı.",
                        s.objects().len()
                    ),
                ));
            }
            _ => {}
        }
        None
    }
}

impl TableEntity {
    /// Its shape's fields, for `TableShape::problem`.
    pub fn shape(&self) -> TableShape<'_> {
        TableShape {
            height: self.height,
            rows: &self.rows,
            columns: &self.columns,
            cells: &self.cells,
            merges: &self.merges,
            aligns: self.aligns.as_deref(),
            frame: self.frame,
            source: self.source.as_ref(),
        }
    }

    /// The range the cell at `row`, `col` is merged into, if any.
    pub fn merge_at(&self, row: usize, col: usize) -> Option<&CellRange> {
        self.merges.iter().find(|r| r.holds(row, col))
    }
}

/// A table file as Tablo ekle reads it (docs/adr/0184 §4): its sheets'
/// rows of words, the text encoding a CSV or text file was read in, or why
/// it was not read.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableFileRead {
    pub sheets: Vec<TableSheet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
}

/// One sheet of a table file: a workbook's by its name, a text file's without one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TableSheet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    pub rows: Vec<Vec<String>>,
    /// Rows past the most a table holds were not read (`MAX_TABLE_ROWS`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub cut: bool,
}

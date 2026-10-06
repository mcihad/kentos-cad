//! A table's layout (docs/adr/0184 §2): its outline, the lines it draws,
//! its frame's band and where each cell's words stand, in its own axes turned
//! and placed at its top left corner. One function for both platforms (the web through its WASM,
//! `tableLayout`); scripts/fixtures/table_cases.py is its independent
//! reference (fixtures/table/v1/cases.json, `layout`).
//!
//! In the table's axes x runs along its rows (its `rotation`), y down its
//! columns from the top; a point (x, y) of it is `p + u·x − v·y`, u the
//! rotation's direction and v a quarter turn on. A line between two cells is
//! drawn unless one merged range holds both; `outer` draws the outline alone,
//! `rows` the outline and the lines between rows, `none` nothing. Lines on one
//! grid line that meet are one line. A table with a frame width draws its
//! outline as a band that wide inside it, four filled strips (the top and
//! bottom whole, the sides between them), not as lines; `none` draws no band. A cell's words sit `PAD` heights in from
//! its sides (left, centred or right as its column; a heading row's
//! centred), their baseline `DROP` heights under its middle; a merged range's
//! words are its top left cell's, in its whole box.

use crate::entity::{CellRange, Shape};
use crate::jsmath::{PI, cos, sin};
use crate::text::face::Face;
use crate::text::{Font, width_em_in};
use crate::vec2::Vec2;

/// How far a cell's words sit in from its sides, in the text height.
pub const PAD: f64 = 0.5;
/// How far a cell's baseline sits under its middle, in the text height (as a
/// centred dimension value's, docs/adr/0183 §3).
pub const DROP: f64 = 0.35;

/// Where one cell's words stand.
#[derive(Clone, Debug, PartialEq)]
pub struct CellPlace {
    pub row: usize,
    pub col: usize,
    /// Where its baseline starts.
    pub at: Vec2,
    /// Its words' width, metres.
    pub width: f64,
    /// Bold: a heading row's, or the face's.
    pub bold: bool,
}

crate::json_struct!(out CellPlace { row, col, at, width, bold });

/// Where a table's parts go.
#[derive(Clone, Debug, PartialEq)]
pub struct TableLayout {
    /// Its corners: top left, top right, bottom right, bottom left.
    pub outline: [Vec2; 4],
    /// The lines it draws.
    pub lines: Vec<[Vec2; 2]>,
    /// Its frame's band, four strips (top, bottom, left, right), each its
    /// corners in the outline's order; none without a frame width.
    pub frame: Vec<[Vec2; 4]>,
    /// Every cell with words, row by row.
    pub cells: Vec<CellPlace>,
}

crate::json_struct!(out TableLayout {
    outline,
    lines,
    frame,
    cells
});

/// A table's fields as its layout reads them.
pub struct TableGeom<'a> {
    pub p: Vec2,
    pub rotation: f64,
    pub height: f64,
    pub rows: &'a [f64],
    pub columns: &'a [f64],
    pub cells: &'a [Vec<String>],
    pub merges: &'a [CellRange],
    pub aligns: Option<&'a [String]>,
    pub header: bool,
    pub grid: Option<&'a str>,
    pub frame: Option<f64>,
    pub face: &'a Face,
    pub source: Option<&'a crate::api::json::Json>,
}

/// A table shape's fields; none for any other shape.
pub fn table_geom(s: &Shape) -> Option<TableGeom<'_>> {
    match s {
        Shape::Table {
            p,
            rotation,
            height,
            rows,
            columns,
            cells,
            merges,
            aligns,
            header,
            grid,
            frame,
            source,
            face,
        } => Some(TableGeom {
            p: *p,
            rotation: *rotation,
            height: *height,
            rows,
            columns,
            cells,
            merges: merges.as_deref().unwrap_or(&[]),
            aligns: aligns.as_deref(),
            header: *header == Some(true),
            grid: grid.as_deref(),
            frame: *frame,
            face,
            source: source.as_ref(),
        }),
        _ => None,
    }
}

/// Running sums from 0: `[0, a, a + b, …]`.
fn edges(sizes: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(sizes.len() + 1);
    let mut at = 0.0;
    out.push(at);
    for s in sizes {
        at += s;
        out.push(at);
    }
    out
}

impl TableGeom<'_> {
    /// Its rows' direction and the quarter turn on.
    fn axes(&self) -> (Vec2, Vec2) {
        let r = self.rotation * PI / 180.0;
        let (c, s) = (cos(r), sin(r));
        (Vec2::new(c, s), Vec2::new(-s, c))
    }

    /// The point `x` along its rows and `y` down its columns from its top left.
    pub fn point(&self, x: f64, y: f64) -> Vec2 {
        let (u, v) = self.axes();
        Vec2::new(self.p.x + u.x * x - v.x * y, self.p.y + u.y * x - v.y * y)
    }

    /// Its width and depth, metres.
    pub fn size(&self) -> (f64, f64) {
        (self.columns.iter().sum(), self.rows.iter().sum())
    }

    /// Its corners: top left, top right, bottom right, bottom left.
    pub fn outline(&self) -> [Vec2; 4] {
        let (w, h) = self.size();
        [
            self.point(0.0, 0.0),
            self.point(w, 0.0),
            self.point(w, h),
            self.point(0.0, h),
        ]
    }

    /// The range that holds the cell at `row`, `col`, if any.
    fn merge_at(&self, row: usize, col: usize) -> Option<&CellRange> {
        self.merges.iter().find(|r| r.holds(row, col))
    }

    /// Whether one merged range holds both cells.
    fn joined(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        self.merges
            .iter()
            .any(|r| r.holds(a.0, a.1) && r.holds(b.0, b.1))
    }

    /// The width of its frame's band, when it draws one: a frame width with
    /// any lines drawn.
    fn band(&self) -> Option<f64> {
        self.frame.filter(|_| self.grid != Some("none"))
    }

    /// The lines it draws, in its axes: along each grid line, the runs of
    /// edges drawn, each run one line; the outline's only without a band.
    fn axis_lines(&self) -> Vec<[(f64, f64); 2]> {
        let (n, m) = (self.rows.len(), self.columns.len());
        let (xs, ys) = (edges(self.columns), edges(self.rows));
        let (inner_rows, inner_columns) = match self.grid {
            Some("none") => return Vec::new(),
            Some("outer") => (false, false),
            Some("rows") => (true, false),
            _ => (true, true),
        };
        let banded = self.band().is_some();
        let mut out = Vec::new();
        // Horizontal grid lines, top to bottom: edge j of line k lies between rows k − 1 and k.
        for k in 0..=n {
            let outer = k == 0 || k == n;
            if (!outer && !inner_rows) || (outer && banded) {
                continue;
            }
            let mut run: Option<usize> = None;
            for j in 0..=m {
                let drawn = j < m && (outer || !self.joined((k - 1, j), (k, j)));
                match (drawn, run) {
                    (true, None) => run = Some(j),
                    (false, Some(start)) => {
                        out.push([(xs[start], ys[k]), (xs[j], ys[k])]);
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        // Vertical grid lines, left to right: edge i of line k lies between columns k − 1 and k.
        for k in 0..=m {
            let outer = k == 0 || k == m;
            if (!outer && !inner_columns) || (outer && banded) {
                continue;
            }
            let mut run: Option<usize> = None;
            for i in 0..=n {
                let drawn = i < n && (outer || !self.joined((i, k - 1), (i, k)));
                match (drawn, run) {
                    (true, None) => run = Some(i),
                    (false, Some(start)) => {
                        out.push([(xs[k], ys[start]), (xs[k], ys[i])]);
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// Its grips: its top left corner, then each column's right end on its top line.
    pub fn grips(&self) -> Vec<Vec2> {
        let xs = edges(self.columns);
        std::iter::once(self.p)
            .chain(xs[1..].iter().map(|&x| self.point(x, 0.0)))
            .collect()
    }

    /// The table with grip `index` at `to`: the corner moves the whole table;
    /// a column's end sets its width to the distance along the rows from its
    /// left end (none when that is not over a hundredth of the text height).
    pub fn moved_grip(&self, index: usize, to: Vec2) -> Option<Shape> {
        let mut p = self.p;
        let mut columns = self.columns.to_vec();
        if index == 0 {
            p = to;
        } else {
            let j = index - 1;
            let left: f64 = self.columns.get(..j)?.iter().sum();
            let (u, _) = self.axes();
            let along = (to.x - self.p.x) * u.x + (to.y - self.p.y) * u.y;
            let w = along - left;
            if !(w > self.height / 100.0) || !w.is_finite() {
                return None;
            }
            *columns.get_mut(j)? = w;
            // A frame wider than half the table it now is: no width for it.
            if let Some(f) = self.frame
                && 2.0 * f >= columns.iter().sum::<f64>()
            {
                return None;
            }
        }
        Some(Shape::Table {
            p,
            rotation: self.rotation,
            height: self.height,
            rows: self.rows.to_vec(),
            columns,
            cells: self.cells.to_vec(),
            merges: (!self.merges.is_empty()).then(|| self.merges.to_vec()),
            aligns: self.aligns.map(<[String]>::to_vec),
            header: self.header.then_some(true),
            grid: self.grid.map(str::to_owned),
            frame: self.frame,
            source: self.source.cloned(),
            face: self.face.clone(),
        })
    }

    /// The lines it draws, placed.
    pub fn lines(&self) -> Vec<[Vec2; 2]> {
        self.axis_lines()
            .into_iter()
            .map(|[a, b]| [self.point(a.0, a.1), self.point(b.0, b.1)])
            .collect()
    }

    /// Its frame's band, placed: the top and bottom strips the whole width,
    /// the left and right ones between them; none without one.
    pub fn band_strips(&self) -> Vec<[Vec2; 4]> {
        let Some(f) = self.band() else {
            return Vec::new();
        };
        let (w, d) = self.size();
        let quad = |x0: f64, y0: f64, x1: f64, y1: f64| {
            [
                self.point(x0, y0),
                self.point(x1, y0),
                self.point(x1, y1),
                self.point(x0, y1),
            ]
        };
        vec![
            quad(0.0, 0.0, w, f),
            quad(0.0, d - f, w, d),
            quad(0.0, f, f, d - f),
            quad(w - f, f, w, d - f),
        ]
    }

    /// Its layout: `font` the project's typeface, for cells whose face has none.
    pub fn layout(&self, font: Font) -> TableLayout {
        let (xs, ys) = (edges(self.columns), edges(self.rows));
        let typeface = self.face.font_or(font);
        let h = self.height;
        let mut cells = Vec::new();
        for (i, row) in self.cells.iter().enumerate() {
            for (j, words) in row.iter().enumerate() {
                if words.is_empty() {
                    continue;
                }
                // A merged range's other cells are empty; its top left cell spans its box.
                let (rows, cols) = match self.merge_at(i, j) {
                    Some(r) if (r.row, r.col) == (i, j) => (r.rows, r.cols),
                    Some(_) => continue,
                    None => (1, 1),
                };
                let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) =
                    (xs.get(j), xs.get(j + cols), ys.get(i), ys.get(i + rows))
                else {
                    continue;
                };
                let heading = self.header && i == 0;
                let bold = heading || self.face.is_bold();
                let width = width_em_in(words, typeface, bold) * h;
                let align = if heading {
                    "center"
                } else {
                    self.aligns
                        .and_then(|a| a.get(j))
                        .map_or("left", String::as_str)
                };
                let x = match align {
                    "center" => (x0 + x1) / 2.0 - width / 2.0,
                    "right" => x1 - PAD * h - width,
                    _ => x0 + PAD * h,
                };
                let y = (y0 + y1) / 2.0 + DROP * h;
                cells.push(CellPlace {
                    row: i,
                    col: j,
                    at: self.point(x, y),
                    width,
                    bold,
                });
            }
        }
        TableLayout {
            outline: self.outline(),
            lines: self.lines(),
            frame: self.band_strips(),
            cells,
        }
    }
}

/// The lines a table shape draws; none for any other shape.
pub fn lines_of(s: &Shape) -> Vec<[Vec2; 2]> {
    table_geom(s).map(|t| t.lines()).unwrap_or_default()
}

/// The strips of a table shape's frame band; none for any other shape.
pub fn band_of(s: &Shape) -> Vec<[Vec2; 4]> {
    table_geom(s).map(|t| t.band_strips()).unwrap_or_default()
}

/// A table shape's layout; none for any other shape.
pub fn layout_of(s: &Shape, font: Font) -> Option<TableLayout> {
    table_geom(s).map(|t| t.layout(font))
}

/// The column widths that fit each column's words in `font` (`PAD` heights
/// each side, a heading row's words bold), at least `least` metres: Tablo
/// ekle's and Yazıya sığdır's widths. A merged range's words do not widen
/// its columns.
pub fn fit_widths(t: &TableGeom<'_>, font: Font, least: f64) -> Vec<f64> {
    let typeface = t.face.font_or(font);
    let mut out = vec![least; t.columns.len()];
    for (i, row) in t.cells.iter().enumerate() {
        for (j, words) in row.iter().enumerate() {
            if words.is_empty() || j >= out.len() || t.merge_at(i, j).is_some() {
                continue;
            }
            let bold = (t.header && i == 0) || t.face.is_bold();
            let w = width_em_in(words, typeface, bold) * t.height + 2.0 * PAD * t.height;
            if w > out[j] {
                out[j] = w;
            }
        }
    }
    out
}

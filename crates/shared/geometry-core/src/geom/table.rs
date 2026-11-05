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
//!
//! A cell's words may hold line breaks (docs/adr/1030 §7): each line is placed apart (`CellPlace::line`), `LINE`
//! heights under the one before, the lines' block centred on the box's middle. A table with `breaks` is drawn in
//! parts ([`TableGeom::parts`]): each at most `height` deep, the next `gap` along its rows from the one before, the
//! heading row drawn again at the top of each but the first unless `once`; a part takes rows while they fit (at least
//! one, the first part the heading and one more), and never cuts a merged range. Each part is laid out as a table of
//! its rows, its lines and cells part after part; the outline holds them all.

use crate::entity::{CellRange, Shape, TableBreaks};
use crate::jsmath::{PI, cos, js_max, js_min, sin};
use crate::text::face::Face;
use crate::text::{Font, width_em_in};
use crate::vec2::Vec2;

/// How far a cell's words sit in from its sides, in the text height.
pub const PAD: f64 = 0.5;
/// How far a cell's baseline sits under its middle, in the text height (as a
/// centred dimension value's, docs/adr/0183 §3).
pub const DROP: f64 = 0.35;
/// How far apart a cell's lines are, baseline to baseline, in the text height (docs/adr/1030 §7): a single line
/// spacing of 5/3 of the height, as AutoCAD's multiline text's.
pub const LINE: f64 = 5.0 / 3.0;

/// Where one cell's words stand.
#[derive(Clone, Debug, PartialEq)]
pub struct CellPlace {
    pub row: usize,
    pub col: usize,
    /// Which of its words' lines (by the line breaks) stands here, from 0.
    pub line: usize,
    /// Where its baseline starts.
    pub at: Vec2,
    /// Its words' width, metres.
    pub width: f64,
    /// Bold: a heading row's, the face's or its style's.
    pub bold: bool,
    /// Its words' colour from its style, `#rrggbb` (docs/adr/0327 §1); none: the table's.
    pub color: Option<String>,
}

crate::json_struct!(out CellPlace { row, col, line, at, width, bold, color });

/// The line of a cell's words a [`CellPlace`] stands for: its `line`-th, the words split at their line breaks.
pub fn cell_line(words: &str, line: usize) -> &str {
    words.split('\n').nth(line).unwrap_or("")
}

/// A cell's (or a merged range's) background from its style (docs/adr/0327 §1): its corners in the outline's order.
#[derive(Clone, Debug, PartialEq)]
pub struct CellFill {
    pub corners: [Vec2; 4],
    pub fill: String,
}

crate::json_struct!(out CellFill { corners, fill });

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
    /// The cells with a background, row by row, under its lines and words.
    pub fills: Vec<CellFill>,
}

crate::json_struct!(out TableLayout {
    outline,
    lines,
    frame,
    cells,
    fills
});

/// One part of a table (docs/adr/1030 §7): its rows in their order (the heading's copy first when it is drawn again)
/// and how far along its rows it stands from the table's corner.
#[derive(Clone, Debug, PartialEq)]
pub struct TablePart {
    pub rows: Vec<usize>,
    pub x: f64,
}

crate::json_struct!(out TablePart { rows, x });

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
    pub styles: &'a [crate::entity::CellLook],
    pub breaks: Option<&'a TableBreaks>,
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
            styles,
            breaks,
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
            styles: styles.as_deref().unwrap_or(&[]),
            breaks: breaks.as_ref(),
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

    /// Its breaks, when they break it: a depth over zero and finite.
    fn broken(&self) -> Option<&TableBreaks> {
        self.breaks
            .filter(|b| b.height.is_finite() && b.height > 0.0)
    }

    /// Its parts (docs/adr/1030 §7), left to right; one part of every row without breaks.
    pub fn parts(&self) -> Vec<TablePart> {
        let n = self.rows.len();
        let Some(b) = self.broken() else {
            return vec![TablePart {
                rows: (0..n).collect(),
                x: 0.0,
            }];
        };
        // The heading drawn again: a heading row not merged with the rows under it.
        let again = self.header
            && b.once != Some(true)
            && !self.merges.iter().any(|r| r.row == 0 && r.rows > 1);
        let width: f64 = self.columns.iter().sum();
        let gap = b
            .gap
            .filter(|g| g.is_finite())
            .map_or(0.0, |g| js_max(g, 0.0));
        let mut parts: Vec<TablePart> = Vec::new();
        let mut start = 0;
        while start < n {
            let first = parts.is_empty();
            let head = if !first && again { self.rows[0] } else { 0.0 };
            let mut end = start;
            let mut used = head;
            while end < n && (end == start || used + self.rows[end] <= b.height) {
                used += self.rows[end];
                end += 1;
            }
            // The first part takes the heading and one row more.
            if first && self.header && end == 1 && n > 1 {
                end = 2;
            }
            // A merged range is not cut: the part ends before it, or (when that leaves the part without its rows)
            // after it; once it has moved on past one, only on.
            let mut on = false;
            while let Some(r) = self
                .merges
                .iter()
                .find(|r| r.row < end && end < r.row + r.rows)
            {
                let back = r.row;
                if !on && back > start && !(first && self.header && back <= 1) {
                    end = back;
                } else {
                    on = true;
                    end = r.row + r.rows;
                }
            }
            let mut rows = Vec::with_capacity(end - start + 1);
            if !first && again {
                rows.push(0);
            }
            rows.extend(start..end);
            parts.push(TablePart {
                rows,
                x: parts.len() as f64 * (width + gap),
            });
            start = end;
        }
        if parts.is_empty() {
            parts.push(TablePart {
                rows: Vec::new(),
                x: 0.0,
            });
        }
        parts
    }

    /// A part's row edges from its top.
    fn part_edges(&self, part: &TablePart) -> Vec<f64> {
        let mut out = Vec::with_capacity(part.rows.len() + 1);
        let mut at = 0.0;
        out.push(at);
        for &r in &part.rows {
            at += self.rows[r];
            out.push(at);
        }
        out
    }

    /// Its width and depth, metres: its parts side by side, as deep as the deepest.
    pub fn size(&self) -> (f64, f64) {
        if self.broken().is_none() {
            return (self.columns.iter().sum(), self.rows.iter().sum());
        }
        let parts = self.parts();
        let depth = parts
            .iter()
            .map(|p| p.rows.iter().map(|&r| self.rows[r]).sum::<f64>())
            .fold(0.0, js_max);
        let width: f64 = self.columns.iter().sum();
        let last = parts.last().map_or(0.0, |p| p.x);
        (last + width, depth)
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

    /// The lines it draws, in its axes, part after part: along each grid line, the runs of edges drawn, each run one
    /// line; the outline's only without a band.
    fn axis_lines(&self) -> Vec<[(f64, f64); 2]> {
        let (inner_rows, inner_columns) = match self.grid {
            Some("none") => return Vec::new(),
            Some("outer") => (false, false),
            Some("rows") => (true, false),
            _ => (true, true),
        };
        let banded = self.band().is_some();
        let m = self.columns.len();
        let mut out = Vec::new();
        for part in self.parts() {
            let rows = &part.rows;
            let n = rows.len();
            let xs: Vec<f64> = edges(self.columns)
                .into_iter()
                .map(|x| x + part.x)
                .collect();
            let ys = self.part_edges(&part);
            // Horizontal grid lines, top to bottom: edge j of line k lies between the part's rows k − 1 and k.
            for k in 0..=n {
                let outer = k == 0 || k == n;
                if (!outer && !inner_rows) || (outer && banded) {
                    continue;
                }
                let mut run: Option<usize> = None;
                for j in 0..=m {
                    let drawn = j < m && (outer || !self.joined((rows[k - 1], j), (rows[k], j)));
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
                    let drawn = i < n && (outer || !self.joined((rows[i], k - 1), (rows[i], k)));
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
            styles: (!self.styles.is_empty()).then(|| self.styles.to_vec()),
            breaks: self.breaks.copied(),
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

    /// Its frame's band, placed, part by part: the top and bottom strips the whole width, the left and right ones
    /// between them; none without one.
    pub fn band_strips(&self) -> Vec<[Vec2; 4]> {
        let Some(f) = self.band() else {
            return Vec::new();
        };
        let w: f64 = self.columns.iter().sum();
        let quad = |x0: f64, y0: f64, x1: f64, y1: f64| {
            [
                self.point(x0, y0),
                self.point(x1, y0),
                self.point(x1, y1),
                self.point(x0, y1),
            ]
        };
        let mut out = Vec::new();
        for part in self.parts() {
            let d: f64 = part.rows.iter().map(|&r| self.rows[r]).sum();
            let x = part.x;
            out.extend([
                quad(x, 0.0, x + w, f),
                quad(x, d - f, x + w, d),
                quad(x, f, x + f, d - f),
                quad(x + w - f, f, x + w, d - f),
            ]);
        }
        out
    }

    /// Its layout: `font` the project's typeface, for cells whose face has none.
    pub fn layout(&self, font: Font) -> TableLayout {
        let typeface = self.face.font_or(font);
        let xs = edges(self.columns);
        let mut cells = Vec::new();
        for part in self.parts() {
            let ys = self.part_edges(&part);
            for k in 0..part.rows.len() {
                for j in 0..self.columns.len() {
                    self.cell_places(&part, &ys, k, j, &xs, typeface, &mut cells);
                }
            }
        }
        TableLayout {
            outline: self.outline(),
            lines: self.lines(),
            frame: self.band_strips(),
            cells,
            fills: self.fills(),
        }
    }

    /// The most lines a cell of it holds.
    fn most_lines(&self) -> usize {
        self.cells
            .iter()
            .flatten()
            .map(|w| w.split('\n').count())
            .max()
            .unwrap_or(1)
    }

    /// The cells of [`TableGeom::layout`] whose words start inside `near` (closed), and its first cell wherever it
    /// is, in the layout's order (docs/adr/0786 §2): a view's text without the cells of a long table far from it.
    /// Only the rows that can hold such a start are laid out (a start sits in its rows' band, `DROP` heights under
    /// its middle); each cell is placed as the layout places it, so a cell kept is the layout's to the bit.
    pub fn cells_near(&self, font: Font, near: &crate::geometry::Bounds) -> Vec<CellPlace> {
        self.cells_near_in(font, near, 0..usize::MAX)
    }

    /// [`TableGeom::cells_near`]'s cells in the rows `rows` (docs/adr/0786 §2: a long table's rows in parts), the
    /// rows outside them not laid out.
    pub fn cells_near_in(
        &self,
        font: Font,
        near: &crate::geometry::Bounds,
        rows_in: std::ops::Range<usize>,
    ) -> Vec<CellPlace> {
        let inside = |p: Vec2| {
            p.x >= near.min_x && p.x <= near.max_x && p.y >= near.min_y && p.y <= near.max_y
        };
        // A broken table's parts stand apart (docs/adr/1030 §7): its cells are laid out whole and kept by where
        // they start.
        if self.broken().is_some() {
            let all = self.layout(font).cells;
            let first = all
                .first()
                .map(|c| (c.row, c.col, c.at.x.to_bits(), c.at.y.to_bits()));
            return all
                .into_iter()
                .filter(|c| rows_in.contains(&c.row))
                .filter(|c| {
                    inside(c.at)
                        || Some((c.row, c.col, c.at.x.to_bits(), c.at.y.to_bits())) == first
                })
                .collect();
        }
        let xs = edges(self.columns);
        let whole = TablePart {
            rows: (0..self.rows.len()).collect(),
            x: 0.0,
        };
        let ys = edges(self.rows);
        let typeface = self.face.font_or(font);
        let rows = self.cells.len();
        // The box's corners down the table's columns (the y of `point`, from the top): `point` turned back.
        let (_, v) = self.axes();
        let down = |x: f64, y: f64| -((x - self.p.x) * v.x + (y - self.p.y) * v.y);
        let ds = [
            down(near.min_x, near.min_y),
            down(near.max_x, near.min_y),
            down(near.max_x, near.max_y),
            down(near.min_x, near.max_y),
        ];
        // A start lies within DROP heights of its band, its lines' block about the middle more; a millimetre more
        // for the turn back's rounding.
        let spread = (self.most_lines().saturating_sub(1)) as f64 / 2.0 * LINE;
        let pad = (DROP.abs() + spread) * self.height.abs() + 1e-3;
        let lo = ds.iter().copied().fold(f64::INFINITY, js_min) - pad;
        let hi = ds.iter().copied().fold(f64::NEG_INFINITY, js_max) + pad;
        let mut picked: Vec<(usize, usize)> = Vec::new();
        // A row of a negative height, or one not a number, leaves the edges unordered: every row is looked at then.
        let ordered = self.rows.iter().all(|r| r.is_finite() && *r >= 0.0);
        if ordered && lo.is_finite() && hi.is_finite() {
            // The rows whose band meets lo..hi: from the first whose bottom edge reaches lo to the last whose top
            // edge is within hi (the edges climb; a row past the edges has no cells).
            let bands = rows.min(ys.len().saturating_sub(1));
            let first = ys[1..=bands].partition_point(|&b| b < lo);
            let last = ys[..bands].partition_point(|&t| t <= hi);
            for i in first.max(rows_in.start)..last.max(first).min(rows_in.end) {
                picked.extend((0..self.cells[i].len()).map(|j| (i, j)));
            }
            // A merged range begun above them that reaches into them.
            for r in self.merges {
                let (Some(&top), Some(&bottom)) = (ys.get(r.row), ys.get(r.row + r.rows)) else {
                    continue;
                };
                if r.row < first && bottom >= lo && top <= hi && rows_in.contains(&r.row) {
                    picked.push((r.row, r.col));
                }
            }
        } else {
            for (i, row) in self.cells.iter().enumerate() {
                if rows_in.contains(&i) {
                    picked.extend((0..row.len()).map(|j| (i, j)));
                }
            }
        }
        let place = |i: usize, j: usize| {
            let mut out = Vec::new();
            if i < whole.rows.len() {
                self.cell_places(&whole, &ys, i, j, &xs, typeface, &mut out);
            }
            out
        };
        // The layout's first cell holds the table's place among the view's texts: its outline keeps the labels
        // off, wherever the view is (store/legible.rs).
        let first = (0..rows)
            .find_map(|i| (0..self.cells[i].len()).find_map(|j| place(i, j).into_iter().next()));
        let first_at = first.as_ref().map(|c| (c.row, c.col, c.line));
        if let Some(c) = &first
            && rows_in.contains(&c.row)
        {
            picked.push((c.row, c.col));
        }
        picked.sort_unstable();
        picked.dedup();
        picked
            .into_iter()
            .flat_map(|(i, j)| place(i, j))
            .filter(|c| inside(c.at) || Some((c.row, c.col, c.line)) == first_at)
            .collect()
    }

    /// The `k`-th row of `part` at column `j` as the layout places it, a place for each of its words' lines, pushed
    /// on `out`: none for a cell without words, one a merged range covers (its top left cell's words span it) or one
    /// past the table's edges.
    #[allow(clippy::too_many_arguments)]
    fn cell_places(
        &self,
        part: &TablePart,
        ys: &[f64],
        k: usize,
        j: usize,
        xs: &[f64],
        typeface: Font,
        out: &mut Vec<CellPlace>,
    ) {
        let Some(&i) = part.rows.get(k) else { return };
        let Some(words) = self.cells.get(i).and_then(|r| r.get(j)) else {
            return;
        };
        if words.is_empty() {
            return;
        }
        let h = self.height;
        // A merged range's other cells are empty; its top left cell spans its box.
        let (rows, cols) = match self.merge_at(i, j) {
            Some(r) if (r.row, r.col) == (i, j) => (r.rows, r.cols),
            Some(_) => return,
            None => (1, 1),
        };
        let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) =
            (xs.get(j), xs.get(j + cols), ys.get(k), ys.get(k + rows))
        else {
            return;
        };
        let (x0, x1) = (x0 + part.x, x1 + part.x);
        let heading = self.header && i == 0;
        let style = self.style_at(i, j);
        let bold = style.bold.unwrap_or(heading || self.face.is_bold());
        let align = match style.align.as_deref() {
            Some(a) => a,
            None if heading => "center",
            None => self
                .aligns
                .and_then(|a| a.get(j))
                .map_or("left", String::as_str),
        };
        let lines: Vec<&str> = words.split('\n').collect();
        let count = lines.len() as f64;
        for (line, text) in lines.into_iter().enumerate() {
            if text.is_empty() {
                continue;
            }
            let width = width_em_in(text, typeface, bold) * h;
            let x = match align {
                "center" => (x0 + x1) / 2.0 - width / 2.0,
                "right" => x1 - PAD * h - width,
                _ => x0 + PAD * h,
            };
            let y = (y0 + y1) / 2.0 + DROP * h + (line as f64 - (count - 1.0) / 2.0) * LINE * h;
            out.push(CellPlace {
                row: i,
                col: j,
                line,
                at: self.point(x, y),
                width,
                bold,
                color: style.color.clone(),
            });
        }
    }

    /// The style of the cell at `row`, `col`: each field from the last style over it that sets it.
    pub fn style_at(&self, row: usize, col: usize) -> crate::entity::CellLook {
        let mut out = crate::entity::CellLook {
            row,
            col,
            rows: 1,
            cols: 1,
            align: None,
            bold: None,
            color: None,
            fill: None,
            decimals: None,
        };
        for s in self.styles.iter().filter(|s| s.holds(row, col)) {
            if s.align.is_some() {
                out.align = s.align.clone();
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

    /// The backgrounds its styles give, a merged range's over its whole box, part by part, row by row.
    pub fn fills(&self) -> Vec<CellFill> {
        if self.styles.iter().all(|s| s.fill.is_none()) {
            return Vec::new();
        }
        let xs = edges(self.columns);
        let mut out = Vec::new();
        for part in self.parts() {
            let ys = self.part_edges(&part);
            for (k, &i) in part.rows.iter().enumerate() {
                for j in 0..self.columns.len() {
                    let (rows, cols) = match self.merge_at(i, j) {
                        Some(r) if (r.row, r.col) == (i, j) => (r.rows, r.cols),
                        Some(_) => continue,
                        None => (1, 1),
                    };
                    let Some(fill) = self.style_at(i, j).fill else {
                        continue;
                    };
                    let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) =
                        (xs.get(j), xs.get(j + cols), ys.get(k), ys.get(k + rows))
                    else {
                        continue;
                    };
                    let (x0, x1) = (x0 + part.x, x1 + part.x);
                    out.push(CellFill {
                        corners: [
                            self.point(x0, y0),
                            self.point(x1, y0),
                            self.point(x1, y1),
                            self.point(x0, y1),
                        ],
                        fill,
                    });
                }
            }
        }
        out
    }

    /// The cells whose words do not fit their box (Sığmayan hücreler, docs/adr/1030 §7): a line wider than the box
    /// less `PAD` heights each side, or lines deeper than the box (one height for the first, `LINE` heights for each
    /// further one). A merged range's box is its whole range's.
    pub fn overflowing(&self, font: Font) -> Vec<(usize, usize)> {
        let typeface = self.face.font_or(font);
        let (xs, ys) = (edges(self.columns), edges(self.rows));
        let h = self.height;
        let mut out = Vec::new();
        for (i, row) in self.cells.iter().enumerate() {
            for (j, words) in row.iter().enumerate() {
                if words.is_empty() {
                    continue;
                }
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
                let bold = self
                    .style_at(i, j)
                    .bold
                    .unwrap_or((self.header && i == 0) || self.face.is_bold());
                let lines: Vec<&str> = words.split('\n').collect();
                let wide = lines.iter().any(|l| {
                    width_em_in(l, typeface, bold) * h + 2.0 * PAD * h > (x1 - x0) * (1.0 + 1e-9)
                });
                let deep = (lines.len() as f64 - 1.0) * LINE * h + h > (y1 - y0) * (1.0 + 1e-9);
                if wide || deep {
                    out.push((i, j));
                }
            }
        }
        out
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
/// each side, a heading row's words bold; a cell's widest line), at least `least` metres: Tablo
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
            // A cell style's bold over the heading's and the face's (docs/adr/0327 §1), as the layout's.
            let bold = t
                .style_at(i, j)
                .bold
                .unwrap_or((t.header && i == 0) || t.face.is_bold());
            for line in words.split('\n') {
                let w = width_em_in(line, typeface, bold) * t.height + 2.0 * PAD * t.height;
                if w > out[j] {
                    out[j] = w;
                }
            }
        }
    }
    out
}

/// A row's depth that holds its cells' lines: `ROW` heights for one line, `LINE` heights more for each further line
/// (docs/adr/1030 §7), `row` the table's `ROW`.
pub fn fit_depth(t: &TableGeom<'_>, i: usize, row: f64) -> f64 {
    let lines = t.cells.get(i).map_or(1, |r| {
        r.iter().map(|w| w.split('\n').count()).max().unwrap_or(1)
    });
    // In thirds of a height, divided once: 5/3 is not a binary fraction, so it is not rounded before the sum.
    (row * 3.0 + 5.0 * (lines.saturating_sub(1)) as f64) * t.height / 3.0
}

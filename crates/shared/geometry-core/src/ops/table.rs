//! Tablo's rules (docs/adr/0184 §3–§5), one for both platforms (the web
//! through its WASM): the rows a schedule writes from objects (Koordinat
//! çizelgesi, Alan çizelgesi, Öznitelik tablosu), a file's rows made a
//! table's cells, a cell's words made one line, the alignment a column's
//! words ask for and a new table's sizes. The independent reference is
//! scripts/fixtures/table_cases.py (fixtures/table/v1/cases.json).
//!
//! - Koordinat çizelgesi: a row for every place the objects' points and
//!   vertices stand at, in the objects' order, each its points or its paths'
//!   vertices in their order; a place two share (1 µm, Köşelere nokta's) is
//!   one row, its name the first name given there (a point's label; a
//!   multi-point object's later points the label and “ (2)”, “ (3)” …), its
//!   elevation the first given there. The unnamed are numbered 1, 2, … past
//!   the names the table has. Columns: Nokta, east and north (the project's
//!   axis names), Z when a place has an elevation.
//! - Alan çizelgesi: a row for every area (an area, a circle, a whole
//!   ellipse): its label or a number, its area in the project's area unit,
//!   its perimeter (holes too, as Öznitelikler's); with more than one area a
//!   last row, Toplam, their areas' sum.
//! - Öznitelik tablosu: a row for every object with a label or attributes:
//!   its label when one of them has one (Ad), then a column for every
//!   attribute name they have, sorted.
//!
//! Numbers are written by the display rule (`display::fixed`) in the
//! project's unit with its decimals; a schedule's first row is its heading.

use std::collections::{BTreeSet, HashSet};

use crate::api::Op;
use crate::api::json::{FromJson, Json, read_field};
use crate::display::fixed;
use crate::entity::{Shape, ellipse_geom, entity_area, entity_length};
use crate::geom::ellipse::is_full_ellipse;
use crate::geom::table::{fit_widths, layout_of, table_geom};
use crate::op;
use crate::ops::compare::Attrs;
use crate::ops::elevation::Elevated;
use crate::text::Font;

/// The most rows, columns and cells a table may have, and letters a cell
/// (the contract's `MAX_TABLE_ROWS` …).
pub const MAX_ROWS: usize = 10_000;
pub const MAX_COLUMNS: usize = 100;
pub const MAX_CELLS: usize = 100_000;
pub const MAX_LETTERS: usize = 1000;
/// A new table's rows' height, in its text height.
pub const ROW: f64 = 2.0;
/// The least a new table's column is wide, in its text height.
pub const LEAST: f64 = 2.0;

/// A cell's words made one line.
#[derive(Clone, Debug, PartialEq)]
pub struct OneLine {
    pub words: String,
    /// Whether they changed.
    pub changed: bool,
}

crate::json_struct!(out OneLine { words, changed });

/// `words` made one line: each control character (a line break, a tab) a
/// space; past `MAX_LETTERS` letters cut, the last kept letter “…”.
pub fn one_line(words: &str) -> OneLine {
    let mut out: String = words
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if out.chars().count() > MAX_LETTERS {
        out = out.chars().take(MAX_LETTERS - 1).chain(['…']).collect();
    }
    OneLine {
        changed: out != words,
        words: out,
    }
}

/// Whether `words` (spaces round it aside) read as a number: a sign, digits,
/// a point or a comma and digits.
fn is_number(words: &str) -> bool {
    let s = words.trim_matches(' ');
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (whole, part) = match s.find(['.', ',']) {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    digits(whole) && part.is_none_or(digits)
}

/// The alignment each column's words ask for: right when every cell of it
/// with words (under the heading row, with one) is a number and there is
/// one; left otherwise.
pub fn column_aligns(cells: &[Vec<String>], header: bool) -> Vec<String> {
    let m = cells.iter().map(Vec::len).max().unwrap_or(0);
    let body = &cells[usize::from(header).min(cells.len())..];
    (0..m)
        .map(|j| {
            let mut words = body
                .iter()
                .filter_map(|r| r.get(j))
                .filter(|w| !w.trim_matches(' ').is_empty())
                .peekable();
            let numbers = words.peek().is_some() && words.all(|w| is_number(w));
            if numbers { "right" } else { "left" }.to_owned()
        })
        .collect()
}

/// A table's cells and their columns' alignment, or why there is none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cells {
    pub cells: Vec<Vec<String>>,
    pub aligns: Vec<String>,
    /// The cells whose words were made one line (`one_line`).
    pub fitted: usize,
    pub problem: Option<String>,
}

crate::json_struct!(out Cells {
    cells,
    aligns,
    fitted,
    problem
});

impl Cells {
    fn refused(problem: String) -> Cells {
        Cells {
            problem: Some(problem),
            ..Cells::default()
        }
    }

    /// Rows of words as a table's cells: each made one line, every row as
    /// long as the longest, in the table's bounds.
    fn of(rows: Vec<Vec<String>>, header: bool) -> Cells {
        let n = rows.len();
        let m = rows.iter().map(Vec::len).max().unwrap_or(0);
        if n == 0 || m == 0 {
            return Cells::refused("Tabloya yazılacak satır yok.".to_owned());
        }
        if n > MAX_ROWS {
            return Cells::refused(format!(
                "Tablo {n} satır olur; bir tablo en çok {MAX_ROWS} satırdır. Daha azını seçin."
            ));
        }
        if m > MAX_COLUMNS {
            return Cells::refused(format!(
                "Tablo {m} sütun olur; bir tablo en çok {MAX_COLUMNS} sütundur."
            ));
        }
        if n * m > MAX_CELLS {
            return Cells::refused(format!(
                "Tablo {} hücre olur; bir tablo en çok {MAX_CELLS} hücredir. Daha azını seçin.",
                n * m
            ));
        }
        let mut fitted = 0;
        let cells: Vec<Vec<String>> = rows
            .into_iter()
            .map(|row| {
                let mut out: Vec<String> = row
                    .iter()
                    .map(|w| {
                        let l = one_line(w);
                        fitted += usize::from(l.changed);
                        l.words
                    })
                    .collect();
                out.resize(m, String::new());
                out
            })
            .collect();
        let aligns = column_aligns(&cells, header);
        Cells {
            cells,
            aligns,
            fitted,
            problem: None,
        }
    }
}

/// A file's rows as a table's cells: the rows and columns after the last
/// with words dropped, the rest as `Cells::of`.
pub fn from_rows(mut rows: Vec<Vec<String>>, header: bool) -> Cells {
    let filled = |w: &String| !w.trim_matches(' ').is_empty();
    while rows.last().is_some_and(|r| !r.iter().any(filled)) {
        rows.pop();
    }
    let m = rows
        .iter()
        .map(|r| r.iter().rposition(filled).map_or(0, |j| j + 1))
        .max()
        .unwrap_or(0);
    for r in &mut rows {
        r.truncate(m);
    }
    Cells::of(rows, header)
}

/// What a schedule is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleKind {
    Coordinates,
    Areas,
    Attributes,
}

impl FromJson for ScheduleKind {
    fn from_json(v: &Json) -> Result<ScheduleKind, String> {
        match String::from_json(v)?.as_str() {
            "coordinates" => Ok(ScheduleKind::Coordinates),
            "areas" => Ok(ScheduleKind::Areas),
            "attributes" => Ok(ScheduleKind::Attributes),
            other => Err(format!("“{other}” diye bir çizelge yok")),
        }
    }
}

/// A local project's drawing unit, or metres.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    M,
    Cm,
    Mm,
}

/// The project's area unit (a metric project's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaUnit {
    M2,
    Donum,
    Ha,
}

/// The project's settings a schedule writes numbers by (the platforms'
/// `Format`): its axes, unit, area unit and decimals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Units {
    /// A CAD project's X east and Y north; else a CBS project's Y and X.
    pub cad: bool,
    pub unit: Unit,
    pub area_unit: AreaUnit,
    pub length_decimals: usize,
    pub area_decimals: usize,
}

impl FromJson for Units {
    fn from_json(v: &Json) -> Result<Units, String> {
        let axes: String = read_field(v, "axes")?;
        let unit: String = read_field(v, "unit")?;
        let area: String = read_field(v, "areaUnit")?;
        let decimals = |k: &str| -> Result<usize, String> {
            let d: f64 = read_field(v, k)?;
            Ok(if d.is_finite() && d > 0.0 {
                (d as usize).min(12)
            } else {
                0
            })
        };
        Ok(Units {
            cad: axes == "cad",
            unit: match unit.as_str() {
                "cm" => Unit::Cm,
                "mm" => Unit::Mm,
                _ => Unit::M,
            },
            area_unit: match area.as_str() {
                "donum" => AreaUnit::Donum,
                "ha" => AreaUnit::Ha,
                _ => AreaUnit::M2,
            },
            length_decimals: decimals("lengthDecimals")?,
            area_decimals: decimals("areaDecimals")?,
        })
    }
}

impl Units {
    fn per_metre(&self) -> f64 {
        match self.unit {
            Unit::M => 1.0,
            Unit::Cm => 100.0,
            Unit::Mm => 1000.0,
        }
    }

    /// A length or a coordinate, metres, in the unit.
    fn length(&self, metres: f64) -> String {
        fixed(metres * self.per_metre(), self.length_decimals)
    }

    /// An area, square metres, in the area unit (a local project's unit squared).
    fn area(&self, square_metres: f64) -> String {
        let d = self.area_decimals;
        match (self.unit, self.area_unit) {
            (Unit::M, AreaUnit::M2) => fixed(square_metres, d),
            (Unit::M, AreaUnit::Donum) => fixed(square_metres / 1000.0, d),
            (Unit::M, AreaUnit::Ha) => fixed(square_metres / 10_000.0, d),
            _ => {
                let k = self.per_metre();
                fixed(square_metres * k * k, d)
            }
        }
    }

    fn length_label(&self) -> &'static str {
        match self.unit {
            Unit::M => "m",
            Unit::Cm => "cm",
            Unit::Mm => "mm",
        }
    }

    fn area_label(&self) -> &'static str {
        match (self.unit, self.area_unit) {
            (Unit::Cm, _) => "cm²",
            (Unit::Mm, _) => "mm²",
            (Unit::M, AreaUnit::M2) => "m²",
            (Unit::M, AreaUnit::Donum) => "dönüm",
            (Unit::M, AreaUnit::Ha) => "ha",
        }
    }

    fn east(&self) -> &'static str {
        if self.cad { "X" } else { "Y" }
    }

    fn north(&self) -> &'static str {
        if self.cad { "Y" } else { "X" }
    }
}

/// An object a schedule reads: its shape, its paths with their elevations
/// (a line's, a polyline's, an area's: `elevation::Elevated`), its label and
/// its attributes.
#[derive(Clone, Debug, PartialEq)]
pub struct Listed {
    pub shape: Shape,
    pub paths: Vec<Elevated>,
    pub label: Option<String>,
    pub attrs: Attrs,
}

impl FromJson for Listed {
    fn from_json(v: &Json) -> Result<Listed, String> {
        let paths: Option<Vec<Elevated>> = read_field(v, "paths")?;
        Ok(Listed {
            shape: Shape::from_json(v.get("shape"))?,
            paths: paths.unwrap_or_default(),
            label: read_field(v, "label")?,
            attrs: Attrs::from_json(v.get("attrs"))?,
        })
    }
}

impl Listed {
    /// Its label, spaces round it aside; none when that leaves nothing.
    pub(crate) fn name(&self) -> Option<&str> {
        self.label
            .as_deref()
            .map(|l| l.trim_matches(' '))
            .filter(|l| !l.is_empty())
    }
}

/// Names for the unnamed, 1, 2, … past `taken`.
struct Numbers<'a> {
    next: u64,
    taken: &'a HashSet<String>,
}

impl Numbers<'_> {
    fn give(&mut self) -> String {
        loop {
            let n = self.next.to_string();
            self.next += 1;
            if !self.taken.contains(&n) {
                return n;
            }
        }
    }
}

fn coordinates(objects: &[Listed], u: &Units) -> Cells {
    // The places and their names are Koordinat yaz's too (docs/adr/0185 §2).
    let places = crate::ops::coordinate_labels::places(objects);
    if places.is_empty() {
        return Cells::refused(
            "Seçili nesnelerde nokta ya da köşe yok; koordinat çizelgesi noktalardan, çizgilerden ve alanlardan yazılır."
                .to_owned(),
        );
    }
    let elevated = places.iter().any(|p| p.z.is_some());
    let mut rows = vec![
        ["Nokta", u.east(), u.north()]
            .into_iter()
            .chain(elevated.then_some("Z"))
            .map(str::to_owned)
            .collect::<Vec<_>>(),
    ];
    for place in places {
        let mut row = vec![
            place.name.unwrap_or_default(),
            u.length(place.p.x),
            u.length(place.p.y),
        ];
        if elevated {
            row.push(place.z.map(|z| u.length(z)).unwrap_or_default());
        }
        rows.push(row);
    }
    Cells::of(rows, true)
}

/// An area's area and perimeter: an area's, a circle's, a whole ellipse's.
fn area_and_perimeter(s: &Shape) -> Option<(f64, f64)> {
    let whole = match s {
        Shape::Polygon { .. } | Shape::Circle { .. } => true,
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => is_full_ellipse(&ellipse_geom(*c, *major, *ratio, *t0, *t1)),
        _ => false,
    };
    if !whole {
        return None;
    }
    Some((entity_area(s)?, entity_length(s)?))
}

fn areas(objects: &[Listed], u: &Units) -> Cells {
    let found: Vec<(&Listed, f64, f64)> = objects
        .iter()
        .filter_map(|o| area_and_perimeter(&o.shape).map(|(a, p)| (o, a, p)))
        .collect();
    if found.is_empty() {
        return Cells::refused(
            "Seçili nesnelerde alan yok; alan çizelgesi alanlardan, dairelerden ve elipslerden yazılır."
                .to_owned(),
        );
    }
    let taken: HashSet<String> = found
        .iter()
        .filter_map(|(o, ..)| o.name().map(str::to_owned))
        .collect();
    let mut numbers = Numbers {
        next: 1,
        taken: &taken,
    };
    let mut rows = vec![vec![
        "Ad".to_owned(),
        format!("Alan ({})", u.area_label()),
        format!("Çevre ({})", u.length_label()),
    ]];
    let mut sum = 0.0;
    for (o, area, perimeter) in &found {
        let name = o.name().map_or_else(|| numbers.give(), str::to_owned);
        rows.push(vec![name, u.area(*area), u.length(*perimeter)]);
        sum += area;
    }
    if found.len() > 1 {
        rows.push(vec!["Toplam".to_owned(), u.area(sum), String::new()]);
    }
    Cells::of(rows, true)
}

fn attributes(objects: &[Listed]) -> Cells {
    let listed: Vec<&Listed> = objects
        .iter()
        .filter(|o| o.name().is_some() || !o.attrs.0.is_empty())
        .collect();
    if listed.is_empty() {
        return Cells::refused("Seçili nesnelerin özniteliği ya da adı yok.".to_owned());
    }
    let names: BTreeSet<&str> = listed
        .iter()
        .flat_map(|o| o.attrs.0.iter().map(|(k, _)| k.as_str()))
        .collect();
    let labelled = listed.iter().any(|o| o.name().is_some());
    let mut rows = vec![
        labelled
            .then_some("Ad")
            .into_iter()
            .chain(names.iter().copied())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
    ];
    for o in &listed {
        let value = |k: &str| {
            o.attrs
                .0
                .iter()
                .find(|(name, _)| name == k)
                .map_or_else(String::new, |(_, v)| v.clone())
        };
        rows.push(
            labelled
                .then(|| o.name().unwrap_or("").to_owned())
                .into_iter()
                .chain(names.iter().map(|k| value(k)))
                .collect(),
        );
    }
    Cells::of(rows, true)
}

/// The schedule of `kind` the objects write, in the project's `units`.
pub fn schedule(kind: ScheduleKind, objects: &[Listed], units: &Units) -> Cells {
    match kind {
        ScheduleKind::Coordinates => coordinates(objects, units),
        ScheduleKind::Areas => areas(objects, units),
        ScheduleKind::Attributes => attributes(objects),
    }
}

/// A table's rows' heights and columns' widths.
#[derive(Clone, Debug, PartialEq)]
pub struct Sizes {
    pub rows: Vec<f64>,
    pub columns: Vec<f64>,
}

crate::json_struct!(out Sizes { rows, columns });

/// The sizes that fit a table's words (Tablo ekle's, Yazıya sığdır's): every
/// row `ROW` text heights deep, every column as wide as its words ask
/// (`fit_widths`), at least `LEAST` text heights. None for any other shape.
pub fn sizes(table: &Shape, font: Font) -> Option<Sizes> {
    let t = table_geom(table)?;
    Some(Sizes {
        rows: vec![ROW * t.height; t.rows.len()],
        columns: fit_widths(&t, font, LEAST * t.height),
    })
}

pub(crate) static OPS: &[Op] = &[
    op!("tableLayout", |table: Shape, font: String| layout_of(
        &table,
        Font::from_id(&font)
    )),
    op!("tableSizes", |table: Shape, font: String| sizes(
        &table,
        Font::from_id(&font)
    )),
    op!("tableSchedule", |kind: ScheduleKind,
                          objects: Vec<Listed>,
                          units: Units| {
        schedule(kind, &objects, &units)
    }),
    op!("tableFromRows", |rows: Vec<Vec<String>>, header: bool| {
        from_rows(rows, header)
    }),
    op!("tableOneLine", |words: String| one_line(&words)),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::table::table_geom;
    use crate::vec2::Vec2;

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

    fn num(v: &Json) -> f64 {
        match v {
            Json::Num(n) => *n,
            other => panic!("a number: {other:?}"),
        }
    }

    fn point(v: &Json) -> Vec2 {
        Vec2::new(num(v.get("x")), num(v.get("y")))
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * (1.0 + b.abs())
    }

    fn same(a: Vec2, b: Vec2, name: &str) {
        assert!(near(a.x, b.x) && near(a.y, b.y), "{name}: {a:?} ≠ {b:?}");
    }

    fn texts(v: &Json) -> Vec<String> {
        match v {
            Json::Arr(a) => a.iter().map(text).collect(),
            _ => Vec::new(),
        }
    }

    fn grid(v: &Json) -> Vec<Vec<String>> {
        match v {
            Json::Arr(a) => a.iter().map(texts).collect(),
            _ => Vec::new(),
        }
    }

    fn cells_as_said(got: &Cells, want: &Json, name: &str) {
        assert_eq!(
            got.problem.as_deref(),
            match want.get("problem") {
                Json::Str(s) => Some(s.as_str()),
                _ => None,
            },
            "{name}"
        );
        assert_eq!(got.cells, grid(want.get("cells")), "{name}");
        assert_eq!(got.aligns, texts(want.get("aligns")), "{name}");
        assert_eq!(got.fitted as f64, num(want.get("fitted")), "{name}");
    }

    /// The shared cases (fixtures/table/v1/cases.json), written by
    /// scripts/fixtures/table_cases.py from docs/adr/0184, not from this
    /// code; the web runs them through WASM (model/ops/table.test.ts).
    #[test]
    fn lays_out_sizes_and_schedules_as_the_shared_cases_say() {
        let f = file();
        assert_eq!(f.get("format"), &Json::Str("kentos.table-cases".into()));
        for c in list(&f, "layout") {
            let name = text(c.get("name"));
            let shape = Shape::from_json(c.get("table")).expect("a table");
            let got = layout_of(&shape, Font::from_id(&text(c.get("font")))).expect("a layout");
            let want = c.get("want");
            for (g, w) in got.outline.iter().zip(list(want, "outline")) {
                same(*g, point(w), &name);
            }
            let lines = list(want, "lines");
            assert_eq!(got.lines.len(), lines.len(), "{name}: lines");
            for (g, w) in got.lines.iter().zip(lines) {
                let Json::Arr(ends) = w else { panic!("{name}") };
                same(g[0], point(&ends[0]), &name);
                same(g[1], point(&ends[1]), &name);
            }
            let frame = list(want, "frame");
            assert_eq!(got.frame.len(), frame.len(), "{name}: frame");
            for (g, w) in got.frame.iter().zip(frame) {
                let Json::Arr(corners) = w else {
                    panic!("{name}")
                };
                for (a, b) in g.iter().zip(corners) {
                    same(*a, point(b), &name);
                }
            }
            let cells = list(want, "cells");
            assert_eq!(got.cells.len(), cells.len(), "{name}: cells");
            for (g, w) in got.cells.iter().zip(cells) {
                assert_eq!(
                    (g.row as f64, g.col as f64),
                    (num(w.get("row")), num(w.get("col"))),
                    "{name}"
                );
                same(g.at, point(w.get("at")), &name);
                assert!(near(g.width, num(w.get("width"))), "{name}: {}", g.width);
                assert_eq!(&Json::Bool(g.bold), w.get("bold"), "{name}");
            }
        }
        for c in list(&f, "grips") {
            let name = text(c.get("name"));
            let shape = Shape::from_json(c.get("table")).expect("a table");
            let t = table_geom(&shape).expect("a table");
            let grips = t.grips();
            let want = list(c, "grips");
            assert_eq!(grips.len(), want.len(), "{name}");
            for (g, w) in grips.iter().zip(want) {
                same(*g, point(w), &name);
            }
            let moved = t.moved_grip(num(c.get("index")) as usize, point(c.get("to")));
            match (moved, c.get("want")) {
                (None, Json::Null) => {}
                (Some(Shape::Table { p, columns, .. }), w) => {
                    same(p, point(w.get("p")), &name);
                    let widths: Vec<f64> = list(w, "columns").iter().map(num).collect();
                    assert_eq!(columns.len(), widths.len(), "{name}");
                    for (g, w) in columns.iter().zip(&widths) {
                        assert!(near(*g, *w), "{name}: {g} ≠ {w}");
                    }
                }
                other => panic!("{name}: {other:?}"),
            }
        }
        for c in list(&f, "sizes") {
            let name = text(c.get("name"));
            let shape = Shape::from_json(c.get("table")).expect("a table");
            let got = sizes(&shape, Font::from_id(&text(c.get("font")))).expect("sizes");
            let want = c.get("want");
            for (k, g) in [("rows", &got.rows), ("columns", &got.columns)] {
                let w: Vec<f64> = list(want, k).iter().map(num).collect();
                assert_eq!(g.len(), w.len(), "{name}: {k}");
                for (a, b) in g.iter().zip(&w) {
                    assert!(near(*a, *b), "{name}: {k} {a} ≠ {b}");
                }
            }
        }
        for c in list(&f, "schedules") {
            let name = text(c.get("name"));
            let kind = ScheduleKind::from_json(c.get("kind")).expect("a kind");
            let objects = Vec::<Listed>::from_json(c.get("objects")).expect("objects");
            let units = Units::from_json(c.get("units")).expect("units");
            cells_as_said(&schedule(kind, &objects, &units), c.get("want"), &name);
        }
        for c in list(&f, "rows") {
            let name = text(c.get("name"));
            let rows = grid(c.get("rows"));
            let header = c.get("header") == &Json::Bool(true);
            cells_as_said(&from_rows(rows, header), c.get("want"), &name);
        }
        for c in list(&f, "oneLine") {
            let got = one_line(&text(c.get("words")));
            let want = c.get("want");
            assert_eq!(got.words, text(want.get("words")));
            assert_eq!(&Json::Bool(got.changed), want.get("changed"));
        }
    }
}

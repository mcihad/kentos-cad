//! A table's parts and its cells of several lines (docs/adr/1030 §7) against fixtures/table/v1/parts.json, written by
//! scripts/fixtures/table_parts_cases.py from the ADR in exact fractions, not from this code: the parts' rows and
//! places, the outline, the lines part after part, the frame's strips, each line's start, the cells that do not fit
//! and the depths that hold their lines. The table's own layout cases (one part, one line) stay table_cases.py's.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::table::{fit_depth, table_geom};
use kentos_geometry_core::ops::table::ROW;
use kentos_geometry_core::text::Font;

fn file() -> Json {
    Json::parse(include_str!("../../../../../fixtures/table/v1/parts.json")).expect("parts.json")
}

fn list<'a>(v: &'a Json, key: &str) -> &'a [Json] {
    match v.get(key) {
        Json::Arr(a) => a,
        _ => &[],
    }
}

fn num(v: &Json) -> f64 {
    match v {
        Json::Num(n) => *n,
        other => panic!("not a number: {other:?}"),
    }
}

fn point(v: &Json) -> Vec2 {
    Vec2::new(num(v.get("x")), num(v.get("y")))
}

/// The reference is exact; the core's sums are binary, its places a few ulps off at most.
fn same(a: Vec2, b: Vec2, name: &str) {
    assert!(
        (a.x - b.x).abs() <= 1e-9 && (a.y - b.y).abs() <= 1e-9,
        "{name}: {a:?} ≠ {b:?}"
    );
}

fn shape(c: &Json) -> Shape {
    Shape::from_json(c.get("table")).expect("a table")
}

#[test]
fn a_table_breaks_into_parts_as_the_reference_says() {
    let f = file();
    assert_eq!(
        f.get("format"),
        &Json::Str("kentos.table-parts-cases".into())
    );
    let cases = list(&f, "layout");
    assert!(cases.len() >= 10);
    for c in cases {
        let name = match c.get("name") {
            Json::Str(s) => s.clone(),
            _ => String::new(),
        };
        let s = shape(c);
        let g = table_geom(&s).expect("a table");
        let want = c.get("want");
        let parts = g.parts();
        let wanted = list(want, "parts");
        assert_eq!(parts.len(), wanted.len(), "{name}: parts");
        for (p, w) in parts.iter().zip(wanted) {
            let rows: Vec<usize> = list(w, "rows").iter().map(|r| num(r) as usize).collect();
            assert_eq!(p.rows, rows, "{name}");
            assert!(
                (p.x - num(w.get("x"))).abs() <= 1e-9,
                "{name}: {} ≠ {}",
                p.x,
                num(w.get("x"))
            );
        }
        let laid = g.layout(Font::from_id("barlow"));
        for (a, b) in laid.outline.iter().zip(list(want, "outline")) {
            same(*a, point(b), &name);
        }
        let lines = list(want, "lines");
        assert_eq!(laid.lines.len(), lines.len(), "{name}: lines");
        for (l, w) in laid.lines.iter().zip(lines) {
            let Json::Arr(ends) = w else { panic!("{name}") };
            same(l[0], point(&ends[0]), &name);
            same(l[1], point(&ends[1]), &name);
        }
        let frame = list(want, "frame");
        assert_eq!(laid.frame.len(), frame.len(), "{name}: frame");
        for (q, w) in laid.frame.iter().zip(frame) {
            let Json::Arr(corners) = w else {
                panic!("{name}")
            };
            for (a, b) in q.iter().zip(corners) {
                same(*a, point(b), &name);
            }
        }
        let cells = list(want, "cells");
        assert_eq!(laid.cells.len(), cells.len(), "{name}: cells");
        for (got, w) in laid.cells.iter().zip(cells) {
            assert_eq!(
                (got.row, got.col, got.line),
                (
                    num(w.get("row")) as usize,
                    num(w.get("col")) as usize,
                    num(w.get("line")) as usize
                ),
                "{name}"
            );
            same(got.at, point(w.get("at")), &name);
        }
        // A broken table's cells near a box are its layout's that start in it (the first wherever it is).
        let everything = kentos_geometry_core::geometry::Bounds {
            min_x: -1e9,
            min_y: -1e9,
            max_x: 1e9,
            max_y: 1e9,
        };
        assert_eq!(
            g.cells_near(Font::from_id("barlow"), &everything),
            laid.cells,
            "{name}: near"
        );
    }
}

#[test]
fn cells_that_do_not_fit_and_depths_that_hold_their_lines() {
    let f = file();
    for c in list(&f, "overflow") {
        let s = shape(c);
        let g = table_geom(&s).expect("a table");
        let got: Vec<[usize; 2]> = g
            .overflowing(Font::from_id("barlow"))
            .into_iter()
            .map(|(i, j)| [i, j])
            .collect();
        let want: Vec<[usize; 2]> = list(c, "want")
            .iter()
            .map(|p| {
                let Json::Arr(ij) = p else { panic!() };
                [num(&ij[0]) as usize, num(&ij[1]) as usize]
            })
            .collect();
        assert_eq!(got, want);
    }
    for c in list(&f, "depths") {
        let s = shape(c);
        let g = table_geom(&s).expect("a table");
        let want: Vec<f64> = list(c, "want").iter().map(num).collect();
        for (i, w) in want.iter().enumerate() {
            assert!(
                (fit_depth(&g, i, ROW) - w).abs() <= 1e-12,
                "row {i}: {} ≠ {w}",
                fit_depth(&g, i, ROW)
            );
        }
    }
}

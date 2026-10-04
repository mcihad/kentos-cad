//! A DXF's unit against the project's (docs/adr/0165 §2): a local project
//! takes a file's values in the unit its $INSUNITS names and keeps metres; a
//! file that names none is in the project's own unit; a project with a
//! coordinate system takes the values as metres and is told of another
//! unit. The fixture is `fixtures/formats/v1/units.dxf` (inches), which the
//! web reads through the WASM module too (`apps/web/src/io/dxf.wasm.test.ts`);
//! the expected values are worked out by hand, an inch being 25.4 mm.

use kentos_contracts::{DrawingUnit, DxfReadOptions, Entity, ImportResult, Vec2};
use kentos_formats::dxf;

fn fixture() -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1/units.dxf");
    std::fs::read(path).expect("the fixture")
}

fn read(bytes: &[u8], unit: Option<DrawingUnit>) -> ImportResult {
    dxf::read(
        bytes,
        &DxfReadOptions {
            unit,
            ..DxfReadOptions::default()
        },
    )
    .expect("read")
}

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

/// The report's unit fact and its note, if any.
fn said(r: &ImportResult) -> (Option<&str>, Option<&str>) {
    let fact = r
        .report
        .source
        .iter()
        .find(|f| f.label == "Birim ($INSUNITS)")
        .map(|f| f.value.as_str());
    let note = r
        .report
        .notes
        .iter()
        .find(|n| n.what == "Birim")
        .map(|n| n.reason.as_str());
    (fact, note)
}

/// A file of one line from (0, 0) to (10, 0) with `header` groups.
fn one_line(header: &[(i32, &str)]) -> Vec<u8> {
    let mut t = String::from("  0\nSECTION\n  2\nHEADER\n");
    for (code, value) in header {
        t.push_str(&format!("{code:>3}\n{value}\n"));
    }
    t.push_str("  0\nENDSEC\n  0\nSECTION\n  2\nENTITIES\n");
    t.push_str("  0\nLINE\n  8\n0\n 10\n0\n 20\n0\n 11\n10\n 21\n0\n  0\nENDSEC\n  0\nEOF\n");
    t.into_bytes()
}

fn line_end(r: &ImportResult) -> Vec2 {
    match &r.entities[0] {
        Entity::Line(l) => l.b,
        e => panic!("a line, not {e:?}"),
    }
}

#[test]
fn inches_become_metres_in_a_local_project() {
    let r = read(&fixture(), Some(DrawingUnit::Mm));
    assert_eq!(
        said(&r),
        (
            Some("inç"),
            Some("dosya inç biriminde; değerler çizimin birimine, milimetreye çevrildi")
        )
    );
    let mut seen = 0;
    for e in &r.entities {
        match e {
            Entity::Line(l) => {
                assert_eq!((l.a, l.b), (v(0.0, 0.0), v(0.254, 0.0)));
                // Elevations are lengths too (docs/adr/0142).
                assert_eq!((l.za, l.zb), (Some(0.0508), Some(0.0508)));
            }
            Entity::Circle(c) => assert_eq!((c.c, c.r), (v(0.127, 0.1016), 0.0508)),
            Entity::Arc(a) => {
                assert_eq!((a.c, a.r, a.a0), (v(0.127, 0.1016), 0.0762, 0.0));
                // An angle is no length: a quarter turn stays one.
                assert!((a.a1 - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
            }
            Entity::Text(t) => assert_eq!((t.p, t.height), (v(0.0254, 0.1524), 0.0127)),
            Entity::Polygon(p) | Entity::Polyline(p) => assert_eq!(
                p.pts,
                [v(0.0, 0.0), v(0.254, 0.0), v(0.254, 0.2032), v(0.0, 0.2032)]
            ),
            // The definition is scaled with the drawing: the insert keeps its own scale.
            Entity::Insert(i) => assert_eq!((i.p, i.scale), (v(0.0762, 0.1016), 2.0)),
            e => panic!("an object the file does not hold: {e:?}"),
        }
        seen += 1;
    }
    assert_eq!(seen, 6);
    let [pim] = r.blocks.as_slice() else {
        panic!("one block, not {:?}", r.blocks);
    };
    assert_eq!(pim.base, v(0.0254, 0.0254));
    for e in &pim.entities {
        match e {
            Entity::Circle(c) => assert_eq!((c.c, c.r), (v(0.0254, 0.0254), 0.00635)),
            Entity::Line(l) => assert_eq!((l.a, l.b), (v(0.0127, 0.0254), v(0.0381, 0.0254))),
            e => panic!("an object the block does not hold: {e:?}"),
        }
    }
    // The extent and the view are the drawing's, in metres.
    let b = r.bounds.expect("an extent");
    assert_eq!((b.min_x, b.min_y, b.max_x), (0.0, 0.0, 0.254));
    let view = r.view.expect("a view");
    assert!(view.max_x <= 0.3 && view.max_y <= 0.3, "{view:?}");
}

#[test]
fn a_project_with_a_coordinate_system_takes_the_values_as_metres() {
    let r = read(&fixture(), None);
    assert_eq!(
        said(&r),
        (
            Some("inç"),
            Some(
                "dosya birimini inç olarak bildiriyor; koordinatlar ölçeklenmeden alındı (metre sayıldı)"
            )
        )
    );
    let line = r
        .entities
        .iter()
        .find_map(|e| match e {
            Entity::Line(l) => Some(l),
            _ => None,
        })
        .expect("the line");
    assert_eq!((line.b, line.za), (v(10.0, 0.0), Some(2.0)));
}

#[test]
fn a_file_in_the_projects_unit_says_nothing_of_it() {
    let r = read(
        &one_line(&[(9, "$INSUNITS"), (70, "4")]),
        Some(DrawingUnit::Mm),
    );
    assert_eq!(said(&r), (Some("milimetre"), None));
    assert_eq!(line_end(&r), v(0.01, 0.0));
    // In metres, a project with a coordinate system has nothing to say either.
    let r = read(&one_line(&[(9, "$INSUNITS"), (70, "6")]), None);
    assert_eq!(
        (said(&r), line_end(&r)),
        ((Some("metre"), None), v(10.0, 0.0))
    );
}

#[test]
fn a_file_without_a_unit_is_in_the_local_projects_own() {
    let r = read(
        &one_line(&[(9, "$INSUNITS"), (70, "0")]),
        Some(DrawingUnit::Cm),
    );
    assert_eq!(
        said(&r),
        (
            Some("birimsiz"),
            Some("dosya birim bildirmiyor; değerler çizimin biriminde (santimetre) sayıldı")
        )
    );
    assert_eq!(line_end(&r), v(0.1, 0.0));
    // No $INSUNITS group at all: the same.
    let r = read(&one_line(&[]), Some(DrawingUnit::Mm));
    assert_eq!(
        said(&r),
        (
            None,
            Some("dosya birim bildirmiyor; değerler çizimin biriminde (milimetre) sayıldı")
        )
    );
    assert_eq!(line_end(&r), v(0.01, 0.0));
    // A code AutoCAD does not define: the project's unit, and said.
    let r = read(
        &one_line(&[(9, "$INSUNITS"), (70, "99")]),
        Some(DrawingUnit::M),
    );
    assert_eq!(
        said(&r),
        (
            Some("başka bir birim"),
            Some(
                "dosyanın bildirdiği birim (99) tanınmıyor; değerler çizimin biriminde (metre) sayıldı"
            )
        )
    );
    assert_eq!(line_end(&r), v(10.0, 0.0));
}

#[test]
fn feet_and_kilometres_too() {
    let r = read(
        &one_line(&[(9, "$INSUNITS"), (70, "2")]),
        Some(DrawingUnit::M),
    );
    assert_eq!(line_end(&r), v(3.048, 0.0));
    let r = read(
        &one_line(&[(9, "$INSUNITS"), (70, "7")]),
        Some(DrawingUnit::Mm),
    );
    assert_eq!(line_end(&r), v(10_000.0, 0.0));
}

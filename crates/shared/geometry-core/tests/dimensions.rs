//! The dimensions of docs/adr/0147 in the core: their layout against the
//! independent reference, what each measures, and the store's and the
//! operations' rules for them (grips, snapping, extent, transforms, Patlat,
//! label records).

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::{Entity, Shape, dimension_geom, entity_bounds, entity_length};
use kentos_geometry_core::geom::dimension::{dimension_measure, layout_dimension};
use serde_json::Value;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

/// A dimension shape from its JSON fields (`kind` added).
fn dim(fields: &str) -> Shape {
    let json = Json::parse(&format!(r#"{{"kind":"dimension",{fields}}}"#)).expect("JSON");
    Entity::from_json(&json).expect("a dimension").shape
}

fn layout(s: &Shape) -> kentos_geometry_core::geom::dimension::DimensionLayout {
    dimension_geom(s)
        .and_then(|g| layout_dimension(&g))
        .expect("laid out")
}

/// The shared cases (fixtures/dimension/v1/layout.json), written from
/// docs/adr/0147 §2 alone by scripts/fixtures/dimension_cases.py: the core
/// lays every dimension out as the independent reference does, within 1e-9
/// m. The web runs the same file through its WASM (`dimension.test.ts`).
#[test]
fn every_shared_layout_case_is_laid_out_as_the_reference_lays_it_out() {
    let file: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/dimension/v1/layout.json"
    ))
    .expect("the cases are JSON");
    assert_eq!(file["format"], "kentos.dimension-cases");
    let cases = file["cases"].as_array().expect("a case list");
    assert_eq!(cases.len(), 25, "the cases are all there");
    let near = |a: f64, e: &Value| {
        let e = e.as_f64().expect("a number");
        (a - e).abs() <= 1e-9 + 1e-15 * e.abs()
    };
    let at = |p: Vec2, e: &Value| near(p.x, &e["x"]) && near(p.y, &e["y"]);
    let mut wrong = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap_or("?");
        let mut d = case["dimension"].clone();
        d["kind"] = "dimension".into();
        let json = Json::parse(&d.to_string()).expect("the dimension reads");
        let shape = Entity::from_json(&json).expect("a dimension").shape;
        let got = dimension_geom(&shape).and_then(|g| layout_dimension(&g));
        let want = &case["want"];
        let Some(got) = got else {
            if !want.is_null() {
                wrong.push(format!("{name}: no layout"));
            }
            continue;
        };
        if want.is_null() {
            wrong.push(format!("{name}: laid out, the reference says none"));
            continue;
        }
        let lines = want["lines"].as_array().expect("lines");
        let lines_ok = got.lines.len() == lines.len()
            && got
                .lines
                .iter()
                .zip(lines)
                .all(|([p, q], e)| at(*p, &e[0]) && at(*q, &e[1]));
        if !(lines_ok
            && at(got.text_at, &want["textAt"])
            && near(got.rotation, &want["rotation"])
            && near(got.value, &want["value"])
            && want["unit"] == got.unit
            && want["prefix"] == got.prefix
            && at(got.handle, &want["handle"]))
        {
            wrong.push(format!("{name}: {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The measure a style's label record and a block's dimension piece take from
/// its style is what its layout says, for all ten styles.
#[test]
fn every_style_measures_what_its_layout_says() {
    for (fields, style, angle) in [
        (r#""a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":2,"height":1"#, None, None),
        (r#""a":{"x":0,"y":0},"b":{"x":10,"y":3},"offset":2,"height":1,"style":"linear","angle":0"#, Some("linear"), Some(0.0)),
        (r#""a":{"x":10,"y":0},"b":{"x":0,"y":10},"c":{"x":0,"y":0},"offset":5,"height":1,"style":"angular""#, Some("angular"), None),
        (r#""a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":1,"height":1,"style":"radius""#, Some("radius"), None),
        (r#""a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":1,"height":1,"style":"diameter""#, Some("diameter"), None),
        (r#""a":{"x":0,"y":0},"b":{"x":0,"y":10},"offset":0,"height":1,"style":"ordinate","angle":0"#, Some("ordinate"), Some(0.0)),
        (r#""a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":0,"height":1,"style":"ordinate","angle":90"#, Some("ordinate"), Some(90.0)),
        (r#""a":{"x":10,"y":0},"b":{"x":0,"y":10},"c":{"x":0,"y":0},"offset":2,"height":1,"style":"arcLength""#, Some("arcLength"), None),
        (r#""a":{"x":0,"y":0},"b":{"x":300,"y":0},"c":{"x":280,"y":4},"offset":5,"height":1,"style":"jogged""#, Some("jogged"), None),
        (r#""a":{"x":0,"y":0},"b":{"x":10,"y":10},"offset":1,"height":1,"style":"azimuth""#, Some("azimuth"), None),
        (r#""a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":1,"height":1,"style":"slope","za":10,"zb":9"#, Some("slope"), None),
    ] {
        let l = layout(&dim(fields));
        assert_eq!((l.unit, l.prefix), dimension_measure(style, angle), "{fields}");
    }
}

#[test]
fn an_ordinate_or_a_slope_has_no_length_an_arc_length_and_a_jogged_radius_do() {
    let ordinate = dim(r#""a":{"x":452345.5,"y":4412000},"b":{"x":452345.5,"y":4412010},"offset":0,"height":1,"style":"ordinate""#);
    assert_eq!(layout(&ordinate).value, 452345.5);
    assert_eq!(entity_length(&ordinate), None, "a coordinate is no length");
    let slope = dim(r#""a":{"x":0,"y":0},"b":{"x":40,"y":0},"offset":1,"height":1,"style":"slope","za":105.25,"zb":104.75"#);
    assert_eq!(entity_length(&slope), None);
    let arc = dim(r#""a":{"x":10,"y":0},"b":{"x":0,"y":10},"c":{"x":0,"y":0},"offset":2,"height":1,"style":"arcLength""#);
    assert!((entity_length(&arc).expect("a length") - 5.0 * std::f64::consts::PI).abs() < 1e-12);
    let jogged = dim(r#""a":{"x":0,"y":0},"b":{"x":300,"y":0},"c":{"x":280,"y":4},"offset":5,"height":1,"style":"jogged""#);
    assert_eq!(entity_length(&jogged), Some(300.0));
}

/// Grips (docs/adr/0147 §4): an ordinate its line's end; an arc length the
/// arc's ends (kept on the arc) and the dimension arc; a jogged radius the
/// point on the arc (kept on the circle), the jog and the centre shown; an
/// azimuth or a slope its ends and its arrow.
#[test]
fn each_kind_has_its_own_grips() {
    use kentos_geometry_core::ops::grips::{entity_grips, move_grip};
    let moved = |s: &Shape, i: usize, p: Vec2| move_grip(&Entity::new(s.clone()), i, p).map(|e| e.shape);

    let ordinate = dim(r#""a":{"x":10,"y":20},"b":{"x":16,"y":40},"offset":0,"height":2.5,"style":"ordinate""#);
    assert_eq!(entity_grips(&ordinate), vec![v(16.0, 40.0)]);
    let Some(Shape::Dimension { a, b, .. }) = moved(&ordinate, 0, v(30.0, 45.0)) else {
        panic!("its end moves")
    };
    assert_eq!((a, b), (v(10.0, 20.0), v(30.0, 45.0)), "the point stays");
    // Not where the line has no room (closer than h/2 across the axis).
    assert_eq!(moved(&ordinate, 0, v(30.0, 21.0)), None);

    let arc = dim(r#""a":{"x":10,"y":0},"b":{"x":0,"y":10},"c":{"x":0,"y":0},"offset":2,"height":1,"style":"arcLength""#);
    let g = entity_grips(&arc);
    assert_eq!(g.len(), 3);
    assert_eq!((g[0], g[1]), (v(10.0, 0.0), v(0.0, 10.0)));
    let Some(Shape::Dimension { a, .. }) = moved(&arc, 0, v(20.0, 20.0)) else {
        panic!("an end moves")
    };
    let r = std::f64::consts::FRAC_1_SQRT_2 * 10.0;
    assert!((a.x - r).abs() < 1e-12 && (a.y - r).abs() < 1e-12, "on the arc: {a:?}");
    let Some(Shape::Dimension { offset, .. }) = moved(&arc, 2, v(0.0, 15.0)) else {
        panic!("the dimension arc moves")
    };
    assert!((offset - 5.0).abs() < 1e-12);
    assert_eq!(moved(&arc, 1, v(0.0, 0.0)), None, "not onto the centre");

    let jogged = dim(r#""a":{"x":0,"y":0},"b":{"x":300,"y":0},"c":{"x":280,"y":4},"offset":5,"height":1,"style":"jogged""#);
    let g = entity_grips(&jogged);
    assert_eq!(g, vec![v(300.0, 0.0), v(285.0, 4.0), v(280.0, 4.0)]);
    let Some(Shape::Dimension { b, c, .. }) = moved(&jogged, 0, v(0.0, 600.0)) else {
        panic!("the point on the arc moves")
    };
    assert_eq!(b, v(0.0, 300.0), "on its circle");
    assert_eq!(c, Some(v(280.0, 4.0)));
    // Its jog where it is dragged, within where a jog fits (before b by more than the 4 m across).
    let Some(Shape::Dimension { offset, .. }) = moved(&jogged, 1, v(290.0, 9.0)) else {
        panic!("the jog moves")
    };
    assert_eq!(offset, 10.0);
    let Some(Shape::Dimension { offset, .. }) = moved(&jogged, 1, v(299.0, 9.0)) else {
        panic!("the jog moves")
    };
    assert_eq!(offset, 16.0);
    // The centre shown may not go past the arc.
    assert_eq!(moved(&jogged, 2, v(305.0, 2.0)), None);
    let Some(Shape::Dimension { c, .. }) = moved(&jogged, 2, v(250.0, -6.0)) else {
        panic!("the centre shown moves")
    };
    assert_eq!(c, Some(v(250.0, -6.0)));

    let azimuth = dim(r#""a":{"x":0,"y":0},"b":{"x":30,"y":40},"offset":2,"height":1,"style":"azimuth""#);
    let g = entity_grips(&azimuth);
    assert_eq!(g.len(), 3);
    let Some(Shape::Dimension { offset, .. }) = moved(&azimuth, 2, v(15.0 - 4.0, 20.0 + 3.0)) else {
        panic!("the arrow moves")
    };
    assert!((offset - 5.0).abs() < 1e-12);
}

/// Snapping (docs/adr/0147 §4): an ordinate at its point and its line's end;
/// an arc length at the arc's ends and centre; a jogged radius at its point
/// on the arc and the centre shown, never the true centre; an azimuth at its
/// ends, not its arrow's.
#[test]
fn each_kind_snaps_where_its_rules_say() {
    use kentos_geometry_core::store::Store;
    use kentos_geometry_core::store::snap::SnapKind;
    let mut s = Store::new();
    s.put_json(
        r#"[{"id":1,"layerId":"o","kind":"dimension","style":"ordinate","a":{"x":0,"y":0},"b":{"x":0,"y":20},"offset":0,"height":2},
            {"id":2,"layerId":"o","kind":"dimension","style":"arcLength","a":{"x":110,"y":0},"b":{"x":100,"y":10},"c":{"x":100,"y":0},"offset":2,"height":1},
            {"id":3,"layerId":"o","kind":"dimension","style":"jogged","a":{"x":200,"y":-300},"b":{"x":200,"y":0},"c":{"x":204,"y":-20},"offset":3,"height":1},
            {"id":4,"layerId":"o","kind":"dimension","style":"azimuth","a":{"x":0,"y":100},"b":{"x":30,"y":140},"offset":2,"height":1}]"#,
    )
    .expect("the dimensions go in");
    let points = SnapKind::Endpoint.bit() | SnapKind::Node.bit();
    let hit = |x: f64, y: f64| s.snap(v(x, y), 0.3, points, None).map(|h| (h.point, h.id));
    assert_eq!(hit(0.1, 0.1), Some((v(0.0, 0.0), 1.0)));
    assert_eq!(hit(0.1, 19.9), Some((v(0.0, 20.0), 1.0)));
    // Not the line's start, h/2 off the point.
    assert_eq!(hit(0.0, 1.1), None);
    assert_eq!(hit(110.1, 0.1), Some((v(110.0, 0.0), 2.0)));
    assert_eq!(hit(100.1, 0.1), Some((v(100.0, 0.0), 2.0)));
    assert_eq!(hit(200.1, 0.1), Some((v(200.0, 0.0), 3.0)));
    assert_eq!(hit(204.1, -19.9), Some((v(204.0, -20.0), 3.0)));
    assert_eq!(hit(200.1, -299.9), None, "the true centre is not drawn");
    assert_eq!(hit(30.1, 139.9), Some((v(30.0, 140.0), 4.0)));
}

/// An azimuth's or a slope's extent holds its edge's ends (its grips and
/// snapping points), its arrow and its value; a jogged radius's leaves out
/// the true centre.
#[test]
fn the_extent_is_what_is_drawn_and_gripped() {
    let azimuth = dim(r#""a":{"x":0,"y":0},"b":{"x":100,"y":0},"offset":5,"height":1,"style":"azimuth""#);
    let b = entity_bounds(&azimuth);
    assert!(b.min_x <= 0.0 && b.max_x >= 100.0, "{b:?}");
    assert!(b.min_y <= 0.0 && b.max_y > 5.35 + 1.0, "its value too: {b:?}");
    let jogged = dim(r#""a":{"x":200,"y":-300},"b":{"x":200,"y":0},"c":{"x":204,"y":-20},"offset":3,"height":1,"style":"jogged""#);
    let b = entity_bounds(&jogged);
    assert!(b.max_x < 210.0 && b.min_y > -30.0, "{b:?}");
}

/// Transforms (docs/adr/0147 §4): an ordinate keeps its axis (the world's)
/// and is laid out across it again; an arc length mirrored measures the same
/// arc, its dimension arc on the same side; a slope keeps its elevations.
#[test]
fn the_transforms_keep_what_each_measures() {
    use kentos_geometry_core::geom::affine::{mirror, rotation, scaling};
    use kentos_geometry_core::ops::transform::transform_shape;
    let o = v(0.0, 0.0);
    let ordinate = dim(r#""a":{"x":10,"y":0},"b":{"x":16,"y":20},"offset":0,"height":1,"style":"ordinate","angle":0"#);
    let turned = transform_shape(&ordinate, &rotation(std::f64::consts::FRAC_PI_2, o));
    let Shape::Dimension { angle, .. } = &turned else {
        panic!()
    };
    assert_eq!(*angle, Some(0.0), "the axis stays");
    let l = layout(&turned);
    assert!(l.value.abs() < 1e-12, "the point's Y is now 0: {}", l.value);
    // Its line across the axis again: up from the point, jogged over to (−20, 16).
    assert!((l.rotation - 90.0).abs() < 1e-9, "{}", l.rotation);

    let arc = dim(r#""a":{"x":10,"y":0},"b":{"x":0,"y":10},"c":{"x":0,"y":0},"offset":2,"height":1,"style":"arcLength""#);
    let mirrored = transform_shape(&arc, &mirror(o, v(0.0, 1.0)));
    let (before, after) = (layout(&arc), layout(&mirrored));
    assert!((before.value - after.value).abs() < 1e-12, "the same arc's length");
    let Shape::Dimension { offset, .. } = &mirrored else {
        panic!()
    };
    assert_eq!(*offset, 2.0, "still outside");
    let doubled = transform_shape(&arc, &scaling(2.0, o));
    assert!((layout(&doubled).value - 2.0 * before.value).abs() < 1e-12);

    let jogged = dim(r#""a":{"x":0,"y":0},"b":{"x":300,"y":0},"c":{"x":280,"y":4},"offset":5,"height":1,"style":"jogged""#);
    let Shape::Dimension { offset, .. } = transform_shape(&jogged, &mirror(o, v(1.0, 0.0))) else {
        panic!()
    };
    assert_eq!(offset, 5.0, "the jog is along the radius");

    let slope = dim(r#""a":{"x":0,"y":0},"b":{"x":40,"y":0},"offset":1,"height":1,"style":"slope","za":105.25,"zb":104.75"#);
    let Shape::Dimension { za, zb, .. } = transform_shape(&slope, &rotation(1.0, o)) else {
        panic!()
    };
    assert_eq!((za, zb), (Some(105.25), Some(104.75)));
}

/// Patlat: lines and the value; an arc length's dimension arc one arc, its
/// symbol lines; a slope's arrow lines.
#[test]
fn each_kind_explodes_into_lines_and_its_value() {
    use kentos_geometry_core::ops::curve_cuts::Cut;
    use kentos_geometry_core::ops::explode::explode_entity;
    use kentos_geometry_core::text::Font;
    let pieces = |s: &Shape, value: &str| match explode_entity(s, value, Font::DEFAULT) {
        Cut::Pieces(p) => p.into_iter().map(|e| e.shape).collect::<Vec<_>>(),
        _ => panic!("{s:?} explodes"),
    };
    let arc = dim(r#""a":{"x":10,"y":0},"b":{"x":0,"y":10},"c":{"x":0,"y":0},"offset":2,"height":1,"style":"arcLength""#);
    let out = pieces(&arc, "15.708");
    let arcs = out.iter().filter(|s| matches!(s, Shape::Arc { .. })).count();
    let lines = out.iter().filter(|s| matches!(s, Shape::Line { .. })).count();
    let texts: Vec<&str> = out
        .iter()
        .filter_map(|s| match s {
            Shape::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    // Two extension lines, two ticks and the symbol's twelve chords; the dimension arc whole.
    assert_eq!((arcs, lines, texts), (1, 16, vec!["15.708"]));
    let slope = dim(r#""a":{"x":0,"y":0},"b":{"x":40,"y":0},"offset":1,"height":1,"style":"slope","za":105.25,"zb":104.75"#);
    let out = pieces(&slope, "%1.25");
    assert_eq!(out.len(), 4, "the arrow, its head's two sides and the value");
}

/// The label record (docs/adr/0147 §5): the unit and prefix codes and the mask.
#[test]
fn the_label_record_says_the_unit_the_prefix_and_the_mask() {
    use kentos_geometry_core::geometry::Bounds;
    use kentos_geometry_core::store::Store;
    use kentos_geometry_core::store::labels::{DIMENSION_PREFIXES, DIMENSION_UNITS, LABEL_STRIDE};
    let mut s = Store::new();
    s.put_json(
        r#"[{"id":1,"layerId":"o","kind":"dimension","style":"ordinate","angle":90,"a":{"x":0,"y":0},"b":{"x":20,"y":0},"offset":0,"height":2,"mask":true},
            {"id":2,"layerId":"o","kind":"dimension","style":"slope","a":{"x":0,"y":10},"b":{"x":40,"y":10},"offset":1,"height":2,"za":10,"zb":9},
            {"id":3,"layerId":"o","kind":"dimension","style":"azimuth","a":{"x":0,"y":30},"b":{"x":30,"y":70},"offset":1,"height":2}]"#,
    )
    .expect("the dimensions go in");
    let view = Bounds {
        min_x: -50.0,
        min_y: -50.0,
        max_x: 100.0,
        max_y: 100.0,
    };
    let records = s.labels(&view, 10.0, None);
    let said: Vec<(f64, &str, &str, f64)> = records
        .chunks_exact(LABEL_STRIDE)
        .map(|r| {
            (
                r[0],
                DIMENSION_UNITS[r[6] as usize],
                DIMENSION_PREFIXES[r[7] as usize],
                r[8],
            )
        })
        .collect();
    assert_eq!(
        said,
        vec![
            (1.0, "coordinate", "X=", 1.0),
            (2.0, "percent", "%", 0.0),
            (3.0, "angle", "t=", 0.0)
        ]
    );
}

/// A fault exactly when there is no layout (docs/adr/0147 §6): over the
/// shared cases, every fault by hand, and thousands of random dimensions of
/// every new kind near their edges.
#[test]
fn a_fault_says_why_exactly_when_there_is_no_layout() {
    use kentos_geometry_core::geom::dimension::{DimensionFault, DimensionGeom, dimension_fault};
    let fault = |s: &Shape| dimension_fault(&dimension_geom(s).expect("a dimension"));
    let file: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/dimension/v1/layout.json"
    ))
    .expect("the cases are JSON");
    for case in file["cases"].as_array().expect("cases") {
        let mut d = case["dimension"].clone();
        d["kind"] = "dimension".into();
        let s = Entity::from_json(&Json::parse(&d.to_string()).expect("JSON"))
            .expect("a dimension")
            .shape;
        assert_eq!(fault(&s).is_some(), case["want"].is_null(), "{}", case["name"]);
    }
    use DimensionFault::*;
    for (fields, want) in [
        (r#""a":{"x":0,"y":0},"b":{"x":5,"y":1},"offset":0,"height":2.5,"style":"ordinate""#, Some(OrdinateTooShort)),
        (r#""a":{"x":0,"y":0},"b":{"x":5,"y":1},"offset":0,"height":2.5,"style":"ordinate","angle":90"#, None),
        (r#""a":{"x":10,"y":0},"b":{"x":0,"y":10},"offset":2,"height":1,"style":"arcLength""#, Some(ArcNoCentre)),
        (r#""a":{"x":0,"y":0},"b":{"x":0,"y":10},"c":{"x":0,"y":0},"offset":2,"height":1,"style":"arcLength""#, Some(ArcNoRadius)),
        (r#""a":{"x":10,"y":0},"b":{"x":20,"y":0},"c":{"x":0,"y":0},"offset":2,"height":1,"style":"arcLength""#, Some(ArcNoSweep)),
        (r#""a":{"x":10,"y":0},"b":{"x":0,"y":10},"c":{"x":0,"y":0},"offset":-10,"height":1,"style":"arcLength""#, Some(ArcInside)),
        (r#""a":{"x":0,"y":0},"b":{"x":300,"y":0},"offset":5,"height":1,"style":"jogged""#, Some(JoggedNoCentre)),
        (r#""a":{"x":0,"y":0},"b":{"x":0,"y":0},"c":{"x":280,"y":4},"offset":5,"height":1,"style":"jogged""#, Some(JoggedNoRadius)),
        (r#""a":{"x":0,"y":0},"b":{"x":300,"y":0},"c":{"x":296,"y":5},"offset":5,"height":1,"style":"jogged""#, Some(JoggedCentre)),
        (r#""a":{"x":3,"y":3},"b":{"x":3,"y":3},"offset":1,"height":1,"style":"azimuth""#, Some(EdgeTooShort)),
        (r#""a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":1,"height":1,"style":"slope","za":10"#, Some(SlopeNoElevations)),
        (r#""a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":1,"height":1,"style":"linear","angle":0"#, None),
    ] {
        assert_eq!(fault(&dim(fields)), want, "{fields}");
    }
    // Random dimensions: small whole coordinates, so that points meet, line up and fall on the edges' thresholds.
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n) as f64
    };
    for _ in 0..20_000 {
        let style = ["ordinate", "arcLength", "jogged", "azimuth", "slope"][next(5) as usize];
        let (a, b, c) = (
            v(next(9) - 4.0, next(9) - 4.0),
            v(next(9) - 4.0, next(9) - 4.0),
            v(next(9) - 4.0, next(9) - 4.0),
        );
        let (offset, height, axis) = (next(9) - 4.0, 1.0 + next(3), next(2) * 90.0);
        let (has_c, za, zb) = (next(4) != 0.0, next(5), next(5));
        let (has_za, has_zb) = (next(4) != 0.0, next(4) != 0.0);
        let d = DimensionGeom {
            a,
            b,
            offset,
            height,
            style: Some(style.into()),
            angle: Some(axis),
            c: has_c.then_some(c),
            za: has_za.then_some(za),
            zb: has_zb.then_some(zb),
        };
        assert_eq!(
            dimension_fault(&d).is_some(),
            layout_dimension(&d).is_none(),
            "{d:?}"
        );
    }
}

/// Koordinat's axis from the cursor and Yay uzunluğu's arc from a picked
/// edge (docs/adr/0147 §7), worked out by hand.
#[test]
fn the_tools_take_an_ordinate_s_axis_and_an_arc_s_ends_as_the_rules_say() {
    use kentos_geometry_core::geom::dimension::ordinate_axis_for;
    use kentos_geometry_core::geom::intersect::Edge;
    use kentos_geometry_core::tools::editing::arc_length_ends;
    // Further up than across: its Y; further across: its X; a tie is its Y.
    assert_eq!(ordinate_axis_for(v(10.0, 20.0), v(12.0, 30.0)), 0.0);
    assert_eq!(ordinate_axis_for(v(10.0, 20.0), v(-5.0, 18.0)), 90.0);
    assert_eq!(ordinate_axis_for(v(0.0, 0.0), v(3.0, -3.0)), 0.0);

    let close = |p: Vec2, x: f64, y: f64| (p.x - x).abs() < 1e-12 && (p.y - y).abs() < 1e-12;
    let quarter = Edge::Arc {
        c: v(0.0, 0.0),
        r: 10.0,
        a0: 0.0,
        sweep: std::f64::consts::FRAC_PI_2,
    };
    let whole = arc_length_ends(&quarter, None).expect("an arc");
    assert!(close(whole.a, 10.0, 0.0) && close(whole.b, 0.0, 10.0), "{whole:?}");
    // A clockwise edge (a path's arc drawn the other way): the same arc, its ends counter-clockwise.
    let clockwise = Edge::Arc {
        c: v(0.0, 0.0),
        r: 10.0,
        a0: std::f64::consts::FRAC_PI_2,
        sweep: -std::f64::consts::FRAC_PI_2,
    };
    let back = arc_length_ends(&clockwise, None).expect("an arc");
    assert!(close(back.a, 10.0, 0.0) && close(back.b, 0.0, 10.0), "{back:?}");
    // Kısmi: two points, in either order, put on the circle.
    let part = arc_length_ends(&quarter, Some((v(0.0, 20.0), v(5.0, 5.0)))).expect("a part");
    let h = 10.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert!(close(part.a, h, h) && close(part.b, 0.0, 10.0), "{part:?}");
    // A point off the arc goes to its nearer end: just below the start, to the start.
    let off = arc_length_ends(&quarter, Some((v(10.0, -1.0), v(5.0, 5.0)))).expect("a part");
    assert!(close(off.a, 10.0, 0.0) && close(off.b, h, h), "{off:?}");
    // Nothing for one place twice, a point at the centre, a full turn or a straight edge.
    assert_eq!(arc_length_ends(&quarter, Some((v(5.0, 5.0), v(7.0, 7.0)))), None);
    assert_eq!(arc_length_ends(&quarter, Some((v(0.0, 0.0), v(7.0, 7.0)))), None);
    let turn = Edge::Arc {
        c: v(0.0, 0.0),
        r: 10.0,
        a0: 0.0,
        sweep: std::f64::consts::TAU,
    };
    assert_eq!(arc_length_ends(&turn, None), None);
    assert_eq!(
        arc_length_ends(&Edge::Seg { a: v(0.0, 0.0), b: v(1.0, 0.0) }, None),
        None
    );
}

/// Hızlı ölçü's shared cases (fixtures/dimension/v1/quick.json), written
/// from docs/adr/0147 §7 alone by scripts/fixtures/quick_dimension_cases.py:
/// for each selection, cursor and typed distance, the core gives the same
/// dimensions in the same order, within 1e-9 m, and the same count skipped.
/// The web runs the same file through its WASM (`dimension.test.ts`).
#[test]
fn quick_dimensions_are_the_references() {
    use kentos_geometry_core::geom::dimension::quick_dimensions;
    let file: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/dimension/v1/quick.json"
    ))
    .expect("the cases are JSON");
    assert_eq!(file["format"], "kentos.quick-dimension-cases");
    let cases = file["cases"].as_array().expect("a case list");
    assert_eq!(cases.len(), 31, "the cases are all there");
    let near = |a: f64, e: &Value| {
        let e = e.as_f64().expect("a number");
        (a - e).abs() <= 1e-9 + 1e-15 * e.abs()
    };
    let at = |p: Vec2, e: &Value| near(p.x, &e["x"]) && near(p.y, &e["y"]);
    let mut wrong = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap_or("?");
        let shapes: Vec<Shape> = case["objects"]
            .as_array()
            .expect("objects")
            .iter()
            .map(|o| {
                let json = Json::parse(&o.to_string()).expect("the object reads");
                Entity::from_json(&json).expect("an object").shape
            })
            .collect();
        let cursor = v(
            case["at"]["x"].as_f64().expect("x"),
            case["at"]["y"].as_f64().expect("y"),
        );
        let got = quick_dimensions(&shapes, cursor, case["typed"].as_f64(), 2.5);
        let want = &case["want"];
        let dims = want["dimensions"].as_array().expect("dimensions");
        let same = got.skipped as u64 == want["skipped"].as_u64().expect("skipped")
            && got.dimensions.len() == dims.len()
            && got.dimensions.iter().zip(dims).all(|(g, w)| {
                at(g.a, &w["a"])
                    && at(g.b, &w["b"])
                    && near(g.offset, &w["offset"])
                    && g.height == 2.5
                    && g.style.as_deref() == w["style"].as_str()
                    && match (g.c, w.get("c")) {
                        (Some(c), Some(wc)) => at(c, wc),
                        (None, None) => true,
                        _ => false,
                    }
            });
        if !same {
            wrong.push(format!("{name}: {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

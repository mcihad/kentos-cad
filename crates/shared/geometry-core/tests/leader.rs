//! A leader in the core (docs/adr/0146): its length, and its note's turn
//! under the transforms.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{Shape, entity_area, entity_length};
use kentos_geometry_core::geom::affine::{mirror, rotation, scaling};
use kentos_geometry_core::ops::transform::transform_shape;

fn leader(pts: &[(f64, f64)], rotation: f64) -> Shape {
    Shape::Leader {
        pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
        text: Some("Mevcut bina".into()),
        height: 2.5,
        rotation,
        arrow: Some("open".into()),
        mask: Some(true),
    }
}

#[test]
fn a_leader_s_length_is_its_vertices_and_it_has_no_area() {
    // 5 m, then 6 m; the landing (2 × 2.5 m past the last vertex) does not count (§2).
    let e = leader(&[(0.0, 0.0), (3.0, 4.0), (9.0, 4.0)], 0.0);
    assert_eq!(entity_length(&e), Some(11.0));
    assert_eq!(entity_area(&e), None);
}

#[test]
fn its_note_turns_as_a_text_does() {
    let e = leader(&[(0.0, 0.0), (8.0, 6.0)], 30.0);
    let turn = |m| match transform_shape(&e, &m) {
        Shape::Leader {
            pts,
            height,
            rotation,
            text,
            arrow,
            mask,
        } => {
            assert_eq!(text.as_deref(), Some("Mevcut bina"));
            assert_eq!(arrow.as_deref(), Some("open"));
            assert_eq!(mask, Some(true));
            (pts, height, rotation)
        }
        other => panic!("a leader stays a leader, not {other:?}"),
    };
    let o = Vec2::new(0.0, 0.0);
    // A quarter turn: the vertices turn and the note with them.
    let (pts, height, rot) = turn(rotation(std::f64::consts::FRAC_PI_2, o));
    assert!((pts[1].x + 6.0).abs() < 1e-12 && (pts[1].y - 8.0).abs() < 1e-12);
    assert_eq!(height, 2.5);
    assert!((rot - 120.0).abs() < 1e-9, "{rot}");
    // Twice the size: the note's height too.
    let (_, height, rot) = turn(scaling(2.0, o));
    assert_eq!(height, 5.0);
    assert!((rot - 30.0).abs() < 1e-9, "{rot}");
    // Mirrored across the north axis the vertices mirror, the note stays readable
    // (−30° mirrored is 150°, a half turn more is 330°).
    let (pts, _, rot) = turn(mirror(o, Vec2::new(0.0, 1.0)));
    assert!((pts[1].x + 8.0).abs() < 1e-12 && (pts[1].y - 6.0).abs() < 1e-12);
    assert!((rot - 330.0).abs() < 1e-9, "{rot}");
}

/// The shared cases (fixtures/leader/v1/layout.json), written from
/// docs/adr/0146 §2 alone by scripts/fixtures/leader_cases.py: the core lays
/// every leader out as the independent reference does, within 1e-9 m. The
/// web runs the same file through its WASM (`leader.wasm.test.ts`).
#[test]
fn every_shared_layout_case_is_laid_out_as_the_reference_lays_it_out() {
    use kentos_geometry_core::api::json::{FromJson, Json};
    use kentos_geometry_core::entity::Entity;
    use kentos_geometry_core::geom::leader::{Head, layout_of};
    use serde_json::Value;

    let file: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/leader/v1/layout.json"
    ))
    .expect("the cases are JSON");
    assert_eq!(file["format"], "kentos.leader-cases");
    let cases = file["cases"].as_array().expect("a case list");
    assert_eq!(cases.len(), 10, "the cases are all there");
    let near = |a: f64, e: &Value| {
        let e = e.as_f64().expect("a number");
        (a - e).abs() <= 1e-9 + 1e-15 * e.abs()
    };
    let at = |p: Vec2, e: &Value| near(p.x, &e["x"]) && near(p.y, &e["y"]);
    let all = |ps: &[Vec2], e: &Value| {
        let e = e.as_array().expect("points");
        ps.len() == e.len() && ps.iter().zip(e).all(|(p, q)| at(*p, q))
    };
    let mut wrong = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap_or("?");
        let mut leader = case["leader"].clone();
        leader["kind"] = "leader".into();
        let json = Json::parse(&leader.to_string()).expect("the leader reads");
        let shape = Entity::from_json(&json).expect("a leader").shape;
        let Some(got) = layout_of(&shape) else {
            wrong.push(format!("{name}: no layout"));
            continue;
        };
        let want = &case["want"];
        let head = &want["head"];
        let head_ok = match (&got.head, head["kind"].as_str()) {
            (Head::Filled { triangle }, Some("filled")) => all(triangle, &head["triangle"]),
            (Head::Open { lines }, Some("open")) => all(lines, &head["lines"]),
            (Head::Dot { center, radius }, Some("dot")) => {
                at(*center, &head["center"]) && near(*radius, &head["radius"])
            }
            (Head::None {}, Some("none")) => true,
            _ => false,
        };
        let note_ok = match (got.landing, got.note_point, got.note_align) {
            (Some(landing), Some(p), Some(align)) => {
                all(&landing, &want["landing"])
                    && at(p, &want["notePoint"])
                    && want["noteAlign"] == align.name()
            }
            (None, None, None) => want.get("landing").is_none() && want.get("notePoint").is_none(),
            _ => false,
        };
        if !(head_ok && near(got.side, &want["side"]) && note_ok) {
            wrong.push(format!("{name}: {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Two leaders by hand (docs/adr/0146 §4), h = 2: from (0, 0) up to (4, 3)
/// with a note, its landing on to (8, 3), its note's middle left at (9, 3);
/// from (20, 0) to (24, 3) without a note.
fn store() -> kentos_geometry_core::store::Store {
    let mut s = kentos_geometry_core::store::Store::new();
    s.put_json(
        r#"[{"id":1,"layerId":"k","kind":"leader","pts":[{"x":0,"y":0},{"x":4,"y":3}],"text":"Mevcut bina","height":2,"rotation":0},
            {"id":2,"layerId":"k","kind":"leader","pts":[{"x":20,"y":0},{"x":24,"y":3}],"height":2,"rotation":0,"arrow":"dot"}]"#,
    )
    .expect("the leaders go in");
    s
}

#[test]
fn it_snaps_to_its_vertices_and_its_landing_s_end() {
    use kentos_geometry_core::store::snap::SnapKind;
    let s = store();
    let end = SnapKind::Endpoint.bit();
    let hit = |x: f64, y: f64| s.snap(Vec2::new(x, y), 0.3, end, None).map(|h| (h.point, h.id));
    assert_eq!(hit(0.1, 0.1), Some((Vec2::new(0.0, 0.0), 1.0)));
    assert_eq!(hit(4.1, 3.1), Some((Vec2::new(4.0, 3.0), 1.0)));
    assert_eq!(hit(7.9, 3.2), Some((Vec2::new(8.0, 3.0), 1.0)));
    // Without a note there is no landing: nothing past its last vertex.
    assert_eq!(hit(28.0, 3.0), None);
    assert_eq!(hit(24.0, 3.1), Some((Vec2::new(24.0, 3.0), 2.0)));
}

#[test]
fn a_window_takes_it_whole_and_a_crossing_by_any_part() {
    use kentos_geometry_core::geometry::Bounds;
    let s = store();
    let b = |min_x, min_y, max_x, max_y| Bounds {
        min_x,
        min_y,
        max_x,
        max_y,
    };
    // Wholly inside only with its note's box; the arrowhead's reach stays inside the line's box here.
    assert_eq!(s.in_rect(&b(-1.0, -1.0, 9.5, 5.0), false), Vec::<f64>::new());
    assert_eq!(s.in_rect(&b(-1.0, -1.0, 40.0, 5.0), false), vec![1.0, 2.0]);
    // A crossing box over the note alone, over the landing alone.
    assert_eq!(s.in_rect(&b(10.0, 2.5, 11.0, 3.5), true), vec![1.0]);
    assert_eq!(s.in_rect(&b(6.0, 2.5, 7.0, 3.5), true), vec![1.0]);
    // A fence across the note, across the landing.
    let fence = |a: (f64, f64), b: (f64, f64)| s.in_fence(&[Vec2::new(a.0, a.1), Vec2::new(b.0, b.1)], 0.01);
    assert_eq!(fence((10.0, 1.0), (10.0, 5.0)), vec![1.0]);
    assert_eq!(fence((6.0, 2.0), (6.0, 4.0)), vec![1.0]);
    assert_eq!(fence((30.0, 0.0), (30.0, 5.0)), Vec::<f64>::new());
}

#[test]
fn it_explodes_into_a_polyline_its_arrowhead_and_its_note() {
    use kentos_geometry_core::ops::curve_cuts::Cut;
    use kentos_geometry_core::ops::explode::explode_entity;
    use kentos_geometry_core::text::{Font, TextAlign};
    let filled = Shape::Leader {
        pts: vec![Vec2::new(0.0, 0.0), Vec2::new(4.0, 3.0)],
        text: Some("Mevcut bina".into()),
        height: 2.0,
        rotation: 0.0,
        arrow: None,
        mask: Some(true),
    };
    let Cut::Pieces(pieces) = explode_entity(&filled, "", Font::DEFAULT) else {
        panic!("a leader explodes");
    };
    let shapes: Vec<&Shape> = pieces.iter().map(|e| &e.shape).collect();
    assert_eq!(shapes.len(), 3);
    assert!(
        matches!(shapes[0], Shape::Polyline { pts, .. } if pts == &[Vec2::new(0.0, 0.0), Vec2::new(4.0, 3.0), Vec2::new(8.0, 3.0)]),
        "{:?}",
        shapes[0]
    );
    assert!(
        matches!(shapes[1], Shape::Hatch { ring, pattern, .. } if ring.len() == 3 && ring[0] == Vec2::new(0.0, 0.0) && pattern.kind == "solid"),
        "{:?}",
        shapes[1]
    );
    assert!(
        matches!(shapes[2], Shape::Text { p, text, height, align: Some(TextAlign::MiddleLeft), mask: Some(true), .. }
            if *p == Vec2::new(9.0, 3.0) && text == "Mevcut bina" && *height == 2.0),
        "{:?}",
        shapes[2]
    );
    // An open arrowhead: its sides a polyline of their own; no note, no text.
    let open = Shape::Leader {
        pts: vec![Vec2::new(0.0, 0.0), Vec2::new(4.0, 3.0)],
        text: None,
        height: 2.0,
        rotation: 0.0,
        arrow: Some("open".into()),
        mask: None,
    };
    let Cut::Pieces(pieces) = explode_entity(&open, "", Font::DEFAULT) else {
        panic!("a leader explodes");
    };
    assert_eq!(pieces.len(), 2);
    assert!(matches!(&pieces[1].shape, Shape::Polyline { pts, .. } if pts.len() == 3 && pts[1] == Vec2::new(0.0, 0.0)));
}

#[test]
fn the_edge_edits_refuse_it_until_it_is_exploded() {
    use kentos_geometry_core::entity::Entity;
    use kentos_geometry_core::geom::intersect::Edge;
    use kentos_geometry_core::ops::breaking::break_entity;
    use kentos_geometry_core::ops::curve_cuts::{Cut, Geometry};
    use kentos_geometry_core::ops::offset::offset_entity;
    use kentos_geometry_core::ops::trim::{extend_entity, trim_entity};

    let e = Entity::new(leader(&[(0.0, 0.0), (3.0, 4.0), (9.0, 4.0)], 0.0));
    let wall = [Edge::Seg {
        a: Vec2::new(2.0, -10.0),
        b: Vec2::new(2.0, 10.0),
    }];
    let refused = |verb: &str| {
        format!("Kılavuz {verb}; önce Patlat (X) ile çizgisine, ok başına ve notuna ayırın.")
    };
    let cut = |c: Cut| match c {
        Cut::Error(m) => m,
        Cut::Pieces(p) => panic!("{} pieces", p.len()),
    };
    let geometry = |g: Geometry| match g {
        Geometry::Error(m) => m,
        Geometry::Ok(_) => panic!("a geometry"),
    };
    assert_eq!(cut(trim_entity(&e, Vec2::new(1.0, 1.0), &wall)), refused("budanamaz"));
    assert_eq!(geometry(extend_entity(&e, Vec2::new(9.0, 4.0), &wall)), refused("uzatılamaz"));
    assert_eq!(cut(break_entity(&e, Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0))), refused("kırılamaz"));
    assert_eq!(geometry(offset_entity(&e.shape, 1.0, Vec2::new(0.0, 5.0))), refused("ötelenemez"));
}

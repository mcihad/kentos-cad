//! Real tool picking of a circular path edge must create its own persistent reference.
use crate::common::{Bench, E, N};
use kentos_commands::{
    CadElement, CommandResult, ConstraintRule, ConstraintsQuery, Entity, SubPlace,
};
use kentos_domain::Slot;
use kentos_native_application::constraints_query;
use serde_json::{Value, json};
fn drawing() -> Bench {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/constraint-arc-edge.kcad"
    ))
    .unwrap();
    for e in raw["entities"].as_array_mut().unwrap() {
        if e["kind"] == "polyline" {
            for p in e["pts"].as_array_mut().unwrap() {
                p["x"] = json!(p["x"].as_f64().unwrap() + E);
                p["y"] = json!(p["y"].as_f64().unwrap() + N);
            }
        } else {
            for k in ["a", "b"] {
                e[k]["x"] = json!(e[k]["x"].as_f64().unwrap() + E);
                e[k]["y"] = json!(e[k]["y"].as_f64().unwrap() + N);
            }
        }
    }
    Bench::on(&raw.to_string())
}
#[test]
fn two_fixed_vertices_and_a_radius_from_the_arc_midpoint_need_only_one_confirmation() {
    let mut b = drawing();
    b.start("constraint");
    assert!(b.type_text("SB"));
    b.click(0., 0.);
    b.click(4., 0.);
    assert_eq!(b.doc.constraints().constraints.len(), 2, "{:?}", b.log);
    let before = b.doc.to_snapshot_v2();
    assert!(b.type_text("R"));
    b.move_to(2., -1.);
    assert_eq!(b.doc.to_snapshot_v2(), before, "hover is read only");
    b.click(2., -1.);
    assert!(b.type_text("3m"));
    let cs = &b.doc.constraints().constraints;
    assert_eq!(cs.len(), 3, "{:?}", b.log);
    let ConstraintRule::Radius { a, .. } = cs[2].rule else {
        panic!("radius")
    };
    assert_eq!(
        a.element,
        CadElement::ArcEdge {
            part: 0,
            ring: 0,
            index: 0
        }
    );
    for (i, c) in cs.iter().take(2).enumerate() {
        assert!(
            matches!(c.rule,ConstraintRule::Fix{a} if a.element==CadElement::Point{place:SubPlace::Vertex{part:0,ring:0,index:i as u32}})
        );
    }
    let Entity::Polyline(p) = b.doc.get(Slot(1)).unwrap() else {
        panic!("path")
    };
    assert_eq!(p.pts[0], kentos_commands::Vec2 { x: E, y: N });
    assert_eq!(p.pts[1], kentos_commands::Vec2 { x: E + 4., y: N });
    assert!((p.bulges.as_ref().unwrap()[0] - (3. - 5_f64.sqrt()) / 2.).abs() < 1e-8);
    let CommandResult::Completed { output, .. } =
        constraints_query::read(&b.doc, &ConstraintsQuery::default())
    else {
        panic!("query")
    };
    assert!(output.constraints.iter().all(|r| r.holds));
    assert_eq!(output.diagnosis.dof, 0);
    b.doc.undo();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
#[test]
fn radius_tool_does_not_offer_a_straight_path_as_a_circular_edge() {
    let mut b = drawing();
    let slot = b.add_path("cizim", &[[6., 0.], [10., 0.]], false);
    let before = b.doc.to_snapshot_v2();
    b.start("constraint");
    assert!(b.type_text("R"));
    b.click(8., 0.);
    assert_eq!(b.points(), 0);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.doc.get(slot).is_some());
}

#[test]
fn adjacent_edges_of_one_path_can_be_picked_independently_and_made_tangent() {
    let mut b = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    let slot = b.add_bulged("cizim", &[[-3., 4.], [0., 0.], [4., 0.]], &[0., 0.35, 0.]);
    b.start("constraint");
    assert!(b.type_text("SB"));
    for (x, y) in [(-3., 4.), (0., 0.), (4., 0.)] {
        b.click(x, y);
    }
    assert_eq!(b.doc.constraints().constraints.len(), 3, "{:?}", b.log);
    assert!(b.type_text("T"));
    b.click(-1.5, 2.);
    b.click(2., -0.7);
    assert_eq!(b.doc.constraints().constraints.len(), 4, "{:?}", b.log);
    let ConstraintRule::Tangent { a, b: arc, .. } = b.doc.constraints().constraints[3].rule else {
        panic!("tangent")
    };
    assert_eq!(
        a.element,
        CadElement::Segment {
            part: 0,
            ring: 0,
            index: 0
        }
    );
    assert_eq!(
        arc.element,
        CadElement::ArcEdge {
            part: 0,
            ring: 0,
            index: 1
        }
    );
    let Entity::Polyline(p) = b.doc.get(slot).unwrap() else {
        panic!("path")
    };
    assert!((p.bulges.as_ref().unwrap()[1] - 0.5).abs() < 1e-8);
}

/// UR-034: a line made horizontal, then made vertical as well: the two hold only on a line of no length, so the
/// second is refused, its words say why, and the line keeps its length; one constraint stays.
#[test]
fn horizontal_then_vertical_on_one_line_is_refused_not_collapsed() {
    let mut b = Bench::new("line");
    b.click(0., 0.);
    b.click(8., 6.);
    b.confirm();
    let line = b.newest().clone();
    let length = |e: &kentos_commands::Entity| match e {
        kentos_commands::Entity::Line(l) => (l.b.x - l.a.x).hypot(l.b.y - l.a.y),
        other => panic!("a line: {other:?}"),
    };
    b.start("constraint");
    assert!(b.type_text("Y"));
    b.click(4., 3.);
    assert_eq!(b.doc.constraints().constraints.len(), 1, "{:?}", b.log);
    let slot = Slot(line.base().id);
    let flat = b.doc.get(slot).cloned().expect("the line");
    let flat_length = length(&flat);
    assert!(flat_length > 1.0, "made horizontal, a line still: {flat_length}");
    assert!(b.type_text("D"));
    let ends = match &flat { kentos_commands::Entity::Line(l) => (l.a, l.b), _ => unreachable!() };
    b.click((ends.0.x + ends.1.x) / 2.0 - E, (ends.0.y + ends.1.y) / 2.0 - N);
    assert_eq!(b.doc.constraints().constraints.len(), 1, "the vertical not added: {:?}", b.log);
    assert!((length(b.doc.get(slot).expect("the line")) - flat_length).abs() < 1e-9, "not collapsed, unchanged");
    assert!(
        b.log.iter().any(|l| l.text.contains("sıfır uzunluğa")),
        "{:?}",
        b.log.iter().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
}

/// UR-035: Teğet takes a circle then a line as it takes a line then a circle: the same rule, the line first, the line
/// touching the circle after.
#[test]
fn tangent_takes_the_circle_or_the_line_first() {
    for circle_first in [true, false] {
        let mut b = Bench::new("line");
        b.click(-10., 5.);
        b.click(10., 5.);
        b.confirm();
        b.start("circle");
        b.click(0., 0.);
        assert!(b.type_text("3"));
        let line_at = (0., 5.);
        let circle_at = (3., 0.);
        b.start("constraint");
        assert!(b.type_text("T"));
        let (first, second) = if circle_first { (circle_at, line_at) } else { (line_at, circle_at) };
        b.click(first.0, first.1);
        b.click(second.0, second.1);
        let cs = &b.doc.constraints().constraints;
        assert_eq!(cs.len(), 1, "circle first: {circle_first}: {:?}", b.log.iter().map(|l| l.text.clone()).collect::<Vec<_>>());
        let kentos_commands::ConstraintRule::Tangent { a, b: c, .. } = &cs[0].rule else { panic!("a tangent") };
        assert!(matches!(a.element, CadElement::Segment { .. }), "the line first: {:?}", a.element);
        assert!(matches!(c.element, CadElement::Circle), "{:?}", c.element);
    }
}

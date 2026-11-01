//! Mouse/keyboard constraint tools must pick genuine centres without requiring a nearby edge.
use crate::common::{Bench, E, N};
use kentos_commands::{CadElement, ConstraintRule, SubPlace};
use kentos_domain::Slot;
use serde_json::{Value, json};
fn drawing() -> Bench {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ))
    .unwrap();
    raw["settings"]["workspace"] = json!("cad");
    raw["entities"] = json!([
        {"kind":"ellipse","id":1,"layerId":"cizim","attrs":{},"c":{"x":E,"y":N},"major":{"x":8,"y":0},"ratio":0.5,"t0":0,"t1":0},
        {"kind":"line","id":2,"layerId":"cizim","attrs":{},"a":{"x":E+8.,"y":N},"b":{"x":E+12.,"y":N}}
    ]);
    Bench::on(&raw.to_string())
}
#[test]
fn clicking_ellipse_center_fixes_that_point_and_nothing_else() {
    let mut b = drawing();
    b.start("constraint");
    assert!(b.type_text("SB"));
    b.click(0., 0.);
    let constraints = &b.doc.constraints().constraints;
    assert_eq!(constraints.len(), 1, "{:?}", b.log);
    let ConstraintRule::Fix { a } = constraints[0].rule else {
        panic!()
    };
    assert_eq!(a.uid.0, b.doc.uid(Slot(1)).unwrap().into_bytes());
    assert_eq!(
        a.element,
        CadElement::Point {
            place: SubPlace::Center
        }
    );
    let generation = b.doc.generation();
    b.move_to(1., 1.);
    assert_eq!(b.doc.generation(), generation, "hover stays read-only");
}
#[test]
fn ellipse_axis_distance_uses_two_clicks_and_one_typed_confirmation() {
    let mut b = drawing();
    b.start("constraint");
    assert!(b.type_text("U"));
    b.click(0., 0.);
    assert_eq!(b.points(), 1, "{:?}", b.log);
    b.click(8., 0.);
    assert!(b.type_text("10"));
    assert_eq!(b.doc.constraints().constraints.len(), 1, "{:?}", b.log);
    let Some(kentos_commands::Entity::Ellipse(e)) = b.doc.get(Slot(1)) else {
        panic!()
    };
    // Aligned distance leaves the two free points to share movement, but keeps the true 10 m radius.
    assert!((e.major.x.hypot(e.major.y) - 10.).abs() < 1e-7);
    assert_eq!(e.t0, e.t1);
}
#[test]
fn clicking_inside_ellipse_away_from_actual_parts_does_not_create_a_constraint() {
    let mut b = drawing();
    b.start("constraint");
    assert!(b.type_text("SB"));
    b.click(2., 1.5);
    assert!(b.doc.constraints().constraints.is_empty());
    assert_eq!(b.points(), 0);
}

#[test]
fn an_elliptical_arc_curve_can_designate_its_center_when_a_point_is_required() {
    let mut b = drawing();
    let old = b.doc.get(Slot(1)).unwrap().clone();
    let kentos_commands::Entity::Ellipse(mut e) = old else {
        panic!()
    };
    e.t0 = 0.3;
    e.t1 = 1.2;
    b.doc.update(Slot(1), kentos_commands::Entity::Ellipse(e));
    b.start("constraint");
    assert!(b.type_text("U"));
    // The centre is outside the arc's bounding box; the hovered curve designates it.
    b.click(8. * 0.7f64.cos(), 4. * 0.7f64.sin());
    assert_eq!(b.points(), 1, "{:?}", b.log);
    b.click(12., 0.);
    assert!(b.type_text("12"));
    let c = &b.doc.constraints().constraints[0];
    let ConstraintRule::Distance { a, .. } = c.rule else {
        panic!()
    };
    assert_eq!(
        a.element,
        CadElement::Point {
            place: SubPlace::Center
        }
    );
}
#[test]
fn fixing_a_circle_by_its_curve_still_fixes_center_and_radius() {
    let mut b = drawing();
    let slot = b.add_circle("cizim", [-20., 0.], 5.);
    b.start("constraint");
    assert!(b.type_text("SB"));
    b.click(-20. + 5. * 0.8f64.cos(), 5. * 0.8f64.sin());
    let c = &b.doc.constraints().constraints[0];
    let ConstraintRule::Fix { a } = c.rule else {
        panic!()
    };
    assert_eq!(a.uid.0, b.doc.uid(slot).unwrap().into_bytes());
    assert_eq!(a.element, CadElement::Circle);
}

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

#[test]
fn a_successful_single_point_constraint_does_not_immediately_preview_its_own_duplicate() {
    let mut b = drawing();
    b.start("constraint");
    assert!(b.type_text("SB"));
    b.move_to(0., 0.);
    b.click(0., 0.);
    b.move_to(0., 0.);
    assert!(
        b.session
            .preview(&Default::default())
            .unwrap()
            .tag
            .is_none(),
        "success must not be covered by a duplicate preview"
    );
    assert_eq!(b.doc.constraints().constraints.len(), 1);
    b.move_to(0.75, 0.); // Beyond the feedback holdoff, but inside the point-pick aperture.
    assert!(
        b.session
            .preview(&Default::default())
            .unwrap()
            .tag
            .is_some()
    );
    let refused = b.session.preview(&Default::default()).unwrap();
    assert_eq!(refused.tag_tone, kentos_interaction::Tone::Danger);
    assert!(
        refused
            .tag
            .as_ref()
            .unwrap()
            .lines
            .iter()
            .any(|s| s.contains("Sabit")),
        "{refused:?}"
    );
    b.click(0., 0.); // Explicit repetition is still refused, not silently accepted.
    assert_eq!(b.doc.constraints().constraints.len(), 1);
    assert!(b.last_text().unwrap().contains("tekrarlıyor"));
}

#[test]
fn undo_invalidates_success_hover_suppression_at_the_same_pointer_position() {
    let mut b = drawing();
    b.start("constraint");
    b.type_text("SB");
    b.move_to(0., 0.);
    b.click(0., 0.);
    b.move_to(0., 0.);
    assert!(
        b.session
            .preview(&Default::default())
            .unwrap()
            .tag
            .is_none()
    );
    b.doc.undo().unwrap();
    b.move_to(0., 0.);
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(preview.tag.is_some());
    assert_ne!(preview.tag_tone, kentos_interaction::Tone::Danger);
    assert!(
        b.doc.constraints().constraints.is_empty(),
        "hover never recreates an undone constraint"
    );
}

#[test]
fn quarter_turn_hover_is_read_only_then_two_clicks_apply_one_undoable_constraint() {
    let mut b = drawing();
    b.start("constraint");
    b.type_text("SB");
    b.click(0., 0.);
    assert_eq!(b.doc.constraints().constraints.len(), 1);
    let before = b.doc.to_snapshot_v2();
    b.type_text("D");
    b.click(0., 0.);
    b.move_to(8., 0.);
    assert_eq!(b.doc.to_snapshot_v2(), before, "preview must not write");
    let preview = b.session.preview(&Default::default()).unwrap();
    assert_ne!(preview.tag_tone, kentos_interaction::Tone::Danger);
    b.click(8., 0.);
    assert_eq!(b.doc.constraints().constraints.len(), 2, "{:?}", b.log);
    let Some(kentos_commands::Entity::Ellipse(e)) = b.doc.get(Slot(1)) else {
        panic!()
    };
    assert!(e.major.x.abs() < 1e-8);
    assert!((e.major.y.abs() - 8.).abs() < 1e-8);
    assert!((e.ratio - 0.5).abs() < 1e-8);
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

fn arc_drag_bench(fixed_start: bool) -> Bench {
    use kentos_commands::CommandResult;
    use kentos_native_application::{ExecutionContext, constraints_edit};
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/constraint-ellipse-drag.kcad"
    ))
    .unwrap();
    raw["origin"] = json!({"x":E,"y":N});
    for e in raw["entities"].as_array_mut().unwrap() {
        for key in ["c", "a", "b"] {
            if let Some(p) = e.get_mut(key) {
                p["x"] = json!(p["x"].as_f64().unwrap() + E);
                p["y"] = json!(p["y"].as_f64().unwrap() + N);
            }
        }
    }
    let mut b = Bench::on(&raw.to_string());
    b.draft.snap = false;
    let part = |slot, place: &str| json!({"uid":b.doc.uid(Slot(slot)).unwrap().to_string(),"element":{"kind":"point","place":{"kind":place}}});
    let a = part(1, "start");
    let end = part(1, "end");
    let mut rules = json!([{"rule":{"kind":"fix","a":part(1,"center")}},{"rule":{"kind":"distance","a":a,"b":end,"parameter":"kiris"},"value":"4.47213595499958m"},{"rule":{"kind":"coincident","a":end,"b":part(2,"start")}}]);
    if fixed_start {
        rules
            .as_array_mut()
            .unwrap()
            .push(json!({"rule":{"kind":"fix","a":a}}));
    }
    let r = constraints_edit::execute(
        &mut ExecutionContext::new(&mut b.doc),
        serde_json::from_value(json!({"edit":{"kind":"add","constraints":rules}})).unwrap(),
    );
    assert!(matches!(r, CommandResult::Completed { .. }), "{r:?}");
    b.selection.set([Slot(1)]);
    b
}
#[test]
fn an_elliptical_arc_end_grip_previews_readonly_and_follows_its_dimension_in_one_undo() {
    use kentos_commands::{CommandResult, ConstraintsQuery, Entity};
    let mut b = arc_drag_bench(false);
    let before = b.doc.to_snapshot_v2();
    let target = [4. * 0.3f64.cos(), 2. * 0.3f64.sin()];
    b.click(4., 0.);
    assert!(b.session.grip_active());
    b.move_to(target[0], target[1]);
    let preview = b.session.preview(&Default::default()).unwrap();
    assert_ne!(
        preview.tag_tone,
        kentos_interaction::Tone::Danger,
        "{preview:?}"
    );
    assert!(preview.strokes.len() >= 2, "{preview:?}");
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(target[0], target[1]);
    let Some(Entity::Ellipse(e)) = b.doc.get(Slot(1)) else {
        panic!()
    };
    let g = kentos_geometry_core::geom::ellipse::EllipseGeom {
        c: kentos_interaction::Vec2::new(e.c.x, e.c.y),
        major: kentos_interaction::Vec2::new(e.major.x, e.major.y),
        ratio: e.ratio,
        t0: e.t0,
        t1: e.t1,
    };
    let start = kentos_geometry_core::geom::ellipse::ellipse_point(&g, e.t0);
    let end = kentos_geometry_core::geom::ellipse::ellipse_point(&g, e.t1);
    assert!(
        (start.x - E - target[0]).hypot(start.y - N - target[1]) < 1e-8,
        "{:?}",
        b.log
    );
    let Some(Entity::Line(l)) = b.doc.get(Slot(2)) else {
        panic!()
    };
    assert!((l.a.x - end.x).hypot(l.a.y - end.y) < 1e-8);
    assert!(
        preview
            .strokes
            .iter()
            .flat_map(|s| &s.pts)
            .any(|p| (p.x - end.x).hypot(p.y - end.y) < 1e-8),
        "the hover must show the same solved endpoint as commit: {preview:?}"
    );

    let CommandResult::Completed { output: q, .. } =
        kentos_native_application::constraints_query::read(&b.doc, &ConstraintsQuery::default())
    else {
        panic!()
    };
    assert!(q.constraints.iter().all(|c| c.holds), "{q:?}");
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
#[test]
fn an_elliptical_arc_grip_conflict_keeps_the_last_valid_drawing() {
    let mut b = arc_drag_bench(true);
    let before = b.doc.to_snapshot_v2();
    b.click(4., 0.);
    b.move_to(4. * 0.3f64.cos(), 2. * 0.3f64.sin());
    assert_eq!(
        b.session.preview(&Default::default()).unwrap().tag_tone,
        kentos_interaction::Tone::Danger
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(4. * 0.3f64.cos(), 2. * 0.3f64.sin());
    assert_eq!(b.doc.to_snapshot_v2(), before, "{:?}", b.log);
}

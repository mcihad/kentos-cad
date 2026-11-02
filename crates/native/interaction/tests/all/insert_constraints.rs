//! The actual constraint picker must find a block's origin even outside every visible piece.
use crate::common::{Bench, E, N};
use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};
fn drawing() -> Bench {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/constraint-block-anchors.kcad"
    ))
    .unwrap();
    raw["origin"] = json!({"x":E,"y":N});
    for e in raw["entities"].as_array_mut().unwrap() {
        for key in ["p", "a", "b"] {
            if let Some(p) = e.get_mut(key) {
                p["x"] = json!(p["x"].as_f64().unwrap() + E);
                p["y"] = json!(p["y"].as_f64().unwrap() + N);
            }
        }
    }
    raw["settings"]["drawingUnit"] = json!("m");
    let mut b = Bench::on(&raw.to_string());
    b.draft.snap = false;
    b
}
#[test]
fn insertion_point_outside_geometry_is_clickable_and_a_single_confirmation_drives_placement() {
    let mut b = drawing();
    let old = b.doc.get(Slot(2)).unwrap().clone();
    b.start("constraint");
    b.type_text("SB");
    b.click(0., 0.);
    assert_eq!(b.doc.constraints().constraints.len(), 1, "{:?}", b.log);
    b.type_text("Y");
    b.click(0., 0.);
    b.click(6., 0.);
    assert_eq!(b.doc.constraints().constraints.len(), 2, "{:?}", b.log);
    b.type_text("U");
    b.click(0., 0.);
    b.click(6., 0.);
    assert!(
        b.session
            .prompt()
            .notes
            .iter()
            .any(|s| s.contains("ayna ve ölçek korunur"))
    );
    let before = b.doc.to_snapshot_v2();
    assert!(b.type_text("8m"));
    assert_eq!(b.doc.constraints().constraints.len(), 3, "{:?}", b.log);
    let Entity::Insert(mut expected) = old else {
        panic!()
    };
    expected.p.x = E + 8.;
    assert_eq!(b.doc.get(Slot(2)), Some(&Entity::Insert(expected)));
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
#[test]
fn a_block_inner_edge_is_explained_and_backtracking_removes_the_anchor_scope_note() {
    let mut b = drawing();
    let before = b.doc.to_snapshot_v2();
    b.start("constraint");
    b.type_text("U");
    b.click(6., 0.);
    assert!(b.session.prompt().notes.iter().any(|s| s.contains("Blok:")));
    assert!(b.undo_step());
    assert!(b.session.prompt().notes.is_empty());
    b.type_text("SB");
    // The transformed lower edge's midpoint: distinct from the insertion point and all solver-supported points.
    let c = 0.8660254037844386;
    let s = 0.5;
    b.click(6. + 3. * c + 0.375 * s, 3. * s - 0.375 * c);
    assert!(
        b.last_text().unwrap().contains("yerleştirme noktasına"),
        "{:?}",
        b.log
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn a_partly_constrained_insert_grip_keeps_its_target_and_the_attached_line_then_undoes_once() {
    use kentos_commands::CommandResult;
    use kentos_native_application::{ExecutionContext, constraints_edit};
    let mut b = drawing();
    let Entity::Line(mut l) = b.doc.get(Slot(3)).unwrap().clone() else {
        panic!()
    };
    l.a.x = E + 6.;
    b.doc.update(Slot(3), Entity::Line(l));
    let a = json!({"uid":b.doc.uid(Slot(2)).unwrap().to_string(),"element":{"kind":"point","place":{"kind":"node","part":0}}});
    let l = json!({"uid":b.doc.uid(Slot(3)).unwrap().to_string(),"element":{"kind":"point","place":{"kind":"start"}}});
    let result=constraints_edit::execute(&mut ExecutionContext::new(&mut b.doc),serde_json::from_value(json!({"edit":{"kind":"add","constraints":[{"rule":{"kind":"coincident","a":a,"b":l}}]}})).unwrap());
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    let before = b.doc.to_snapshot_v2();
    let Entity::Insert(mut expected) = b.doc.get(Slot(2)).unwrap().clone() else {
        panic!()
    };
    b.selection.set([Slot(2)]);
    b.drag([6., 0.], [8., 2.]);
    expected.p = kentos_commands::Vec2 {
        x: E + 8.,
        y: N + 2.,
    };
    assert_eq!(
        b.doc.get(Slot(2)),
        Some(&Entity::Insert(expected)),
        "{:?}",
        b.log
    );
    let Some(Entity::Line(l)) = b.doc.get(Slot(3)) else {
        panic!()
    };
    assert_eq!((l.a.x, l.a.y), (E + 8., N + 2.));
    assert_eq!((l.b.x, l.b.y), (E + 10., N));
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn a_hovered_anchor_previews_the_actual_affine_block_without_writing() {
    let mut b = drawing();
    b.start("constraint");
    b.type_text("SB");
    b.click(0., 0.);
    b.type_text("ÇK");
    b.click(0., 0.);
    let before = b.doc.to_snapshot_v2();
    b.move_to(6., 0.);
    let preview = b
        .session
        .preview(&kentos_interaction::Format::default())
        .unwrap();
    assert_eq!(
        preview.tag.as_ref().unwrap().lines[1],
        "tam kısıtlı",
        "a transient first-object hold must not call the saved Fix redundant"
    );
    assert!(
        preview.strokes.len() >= 2,
        "the ghost must include the placed outline and circle, not just an anchor mark: {preview:?}"
    );
    // Independently worked first point: (2,1.5)-(1,1), scales 1.5/0.75, mirror in x, then 30 degrees.
    let expected = [
        E + 1.5 * 0.8660254037844386 + 0.375 * 0.5,
        N + 1.5 * 0.5 - 0.375 * 0.8660254037844386,
    ];
    assert!(
        preview
            .strokes
            .iter()
            .flat_map(|s| &s.pts)
            .any(|p| (p.x - expected[0]).abs() < 1e-8 && (p.y - expected[1]).abs() < 1e-8),
        "{preview:?}"
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

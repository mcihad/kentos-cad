//! CAD-ADV-04: weighted NURBS edits, exact split pieces, live dimension refs and atomic history.
use kentos_commands::{
    CommandResult, CurveEdit, CurvesEdit, DimensionPoint, DimensionsReassociate,
    DocumentSnapshotV1, EntitiesBreak, Entity, PlaceOn, PointBinding, SubPlace, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{
    ExecutionContext, curves_edit, dimensions_reassociate, entities_break,
};
use serde_json::json;
fn drawing() -> Document {
    let mut raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["settings"]["workspace"] = json!("cad");
    raw["origin"] = json!({"x":0,"y":0});
    raw["entities"] = json!([
        {"kind":"nurbs","id":1,"layerId":"cizim","attrs":{"QA":"weighted"},"degree":2,"ctrl":[{"x":1,"y":0},{"x":1,"y":1},{"x":0,"y":1}],"knots":[2,2,2,6,6,6],"weights":[1,std::f64::consts::FRAC_1_SQRT_2,1]},
        {"kind":"dimension","id":2,"layerId":"cizim","attrs":{},"a":{"x":1,"y":0},"b":{"x":0,"y":1},"height":0.1,"offset":0.2},
        {"kind":"dimension","id":3,"layerId":"cizim","attrs":{},"a":{"x":1,"y":0},"b":{"x":0.7,"y":0.7},"height":0.1,"offset":0.4},
        {"kind":"line","id":4,"layerId":"cizim","attrs":{},"a":{"x":0.5,"y":-0.1},"b":{"x":0.5,"y":1.1}}
    ]);
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    for (dim, b) in [
        (2, SubPlace::End),
        (
            3,
            SubPlace::Along {
                part: 0,
                ring: 0,
                index: 0,
                t: 0.5,
            },
        ),
    ] {
        let input = DimensionsReassociate {
            uid: doc.uid(Slot(dim)).unwrap().to_string(),
            points: vec![
                PointBinding {
                    point: DimensionPoint::A,
                    to: Some(PlaceOn {
                        uid: doc.uid(Slot(1)).unwrap().to_string(),
                        place: SubPlace::Start,
                        path: vec![],
                    }),
                },
                PointBinding {
                    point: DimensionPoint::B,
                    to: Some(PlaceOn {
                        uid: doc.uid(Slot(1)).unwrap().to_string(),
                        place: b,
                        path: vec![],
                    }),
                },
            ],
            expected_revision: None,
        };
        completed(dimensions_reassociate::execute(
            &mut ExecutionContext::new(&mut doc),
            input,
        ));
    }
    doc
}
fn completed<T: std::fmt::Debug>(r: CommandResult<T>) {
    assert!(matches!(r, CommandResult::Completed { .. }), "{r:?}");
}
fn dimension(doc: &Document, i: u32) -> &kentos_commands::DimensionEntity {
    let Some(Entity::Dimension(d)) = doc.get(Slot(i)) else {
        panic!("dimension")
    };
    d
}
fn edit(doc: &mut Document, edit: CurveEdit) {
    let input = CurvesEdit {
        uid: doc.uid(Slot(1)).unwrap().to_string(),
        edit,
        expected_revision: None,
    };
    completed(curves_edit::execute(&mut ExecutionContext::new(doc), input));
}
#[test]
fn nurbs_control_edit_updates_bound_ends_and_along_points_in_one_undo() {
    let mut doc = drawing();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1));
    edit(
        &mut doc,
        CurveEdit::MoveControl {
            index: 0,
            at: Vec2 { x: 2., y: 0. },
        },
    );
    for slot in [2, 3] {
        let d = dimension(&doc, slot);
        assert_eq!(d.a, Vec2 { x: 2., y: 0. });
        assert!(d.refs.as_ref().unwrap().a.as_ref().unwrap().live());
    }
    let d = dimension(&doc, 3);
    assert!((d.b.x - 1.).abs() < 1e-14);
    assert!((d.b.y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-14);
    assert!(d.refs.as_ref().unwrap().b.as_ref().unwrap().live());
    assert_eq!(doc.uid(Slot(1)), uid);
    assert_eq!(doc.undo().as_deref(), Some("Eğriyi düzenle"));
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn nurbs_knot_insertion_keeps_bound_measurements_and_refs_live() {
    let mut doc = drawing();
    let before = doc.to_snapshot_v2();
    let a = dimension(&doc, 2).clone();
    let b = dimension(&doc, 3).clone();
    edit(&mut doc, CurveEdit::InsertKnot { at: 3. });
    assert_eq!(dimension(&doc, 2), &a);
    let d = dimension(&doc, 3);
    assert!((d.b.x - b.b.x).hypot(d.b.y - b.b.y) < 1e-14);
    assert_eq!(d.refs, b.refs);
    assert_eq!(doc.undo().as_deref(), Some("Eğriyi düzenle"));
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn nurbs_split_relocates_end_and_along_refs_to_the_actual_piece() {
    let mut doc = drawing();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1)).unwrap();
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let input = EntitiesBreak {
        uid: uid.to_string(),
        first: Vec2 { x: s, y: s },
        second: None,
        expected_revision: None,
    };
    let r = entities_break::execute(&mut ExecutionContext::new(&mut doc), input);
    completed(r);
    let d = dimension(&doc, 2);
    assert!(d.refs.as_ref().unwrap().a.as_ref().unwrap().live());
    let end = d.refs.as_ref().unwrap().b.as_ref().unwrap();
    assert!(end.live(), "{d:?}");
    assert_ne!(end.uid.0, uid.into_bytes());
    assert_eq!(d.b, Vec2 { x: 0., y: 1. });
    let middle = dimension(&doc, 3);
    assert!(
        middle.refs.as_ref().unwrap().b.as_ref().unwrap().live(),
        "{middle:?}"
    );
    assert!((middle.b.x - s).hypot(middle.b.y - s) < 1e-12);
    assert!(
        doc.entities()
            .filter(|e| matches!(e, Entity::Nurbs(_)))
            .count()
            == 2
    );
    assert_eq!(doc.undo().as_deref(), Some("Kır"));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn nurbs_trim_keeps_new_end_live_and_marks_a_removed_interior_place_broken() {
    use kentos_commands::{EntitiesTrim, TrimBoundaries, TrimPick};
    use kentos_native_application::entities_trim;
    let mut doc = drawing();
    let before = doc.to_snapshot_v2();
    let old_middle = dimension(&doc, 3).b;
    let request = EntitiesTrim {
        picks: vec![TrimPick {
            uid: doc.uid(Slot(1)).unwrap().to_string(),
            at: Vec2 { x: 1., y: 0. },
        }],
        boundaries: Some(TrimBoundaries::Objects {
            uids: vec![doc.uid(Slot(4)).unwrap().to_string()],
        }),
        ..Default::default()
    };
    completed(entities_trim::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    let d = dimension(&doc, 2);
    assert!((d.a.x - 0.5).abs() < 1e-9);
    assert!((d.a.y - 3f64.sqrt() / 2.).abs() < 1e-9);
    assert!(d.refs.as_ref().unwrap().a.as_ref().unwrap().live());
    assert_eq!(d.b, Vec2 { x: 0., y: 1. });
    let d = dimension(&doc, 3);
    assert_eq!(d.b, old_middle);
    assert!(
        d.refs
            .as_ref()
            .unwrap()
            .b
            .as_ref()
            .unwrap()
            .broken
            .is_some(),
        "{d:?}"
    );
    assert_eq!(doc.undo().as_deref(), Some("Buda"));
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn explicit_polyline_conversion_keeps_the_last_dimension_point_and_explains_the_kind_change() {
    let mut doc = drawing();
    let before = doc.to_snapshot_v2();
    let a = dimension(&doc, 2).clone();
    let b = dimension(&doc, 3).clone();
    edit(&mut doc, CurveEdit::ToPolyline { tolerance: 0.001 });
    for (slot, was) in [(2, a), (3, b)] {
        let now = dimension(&doc, slot);
        assert_eq!(now.a, was.a);
        assert_eq!(now.b, was.b);
        for r in [
            now.refs.as_ref().unwrap().a.as_ref().unwrap(),
            now.refs.as_ref().unwrap().b.as_ref().unwrap(),
        ] {
            assert_eq!(r.broken.as_deref(), Some("kind_changed"));
        }
    }
    assert_eq!(doc.undo().as_deref(), Some("Eğriyi düzenle"));
    assert_eq!(doc.to_snapshot_v2(), before);
}

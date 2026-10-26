//! A tag's sole grip is a placement, with the same semantic move as Python/MCP.
//! Coordinates and untouched entities are checked directly, without the geometry core.
use crate::common::{Bench, E, N};
use kentos_commands::{CommandResult, Entity, Vec2};
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, tags_create};
use serde_json::json;

fn bench() -> (Bench, Slot, [Slot; 2]) {
    let mut b = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    b.draft.snap = false;
    let room = b
        .doc
        .add(
            serde_json::from_value(json!({
                "kind":"space", "id":0, "layerId":"cizim", "attrs":{},
                "number":"A-001", "name":"Salon", "seed":{"x":E+3.,"y":N+2.},
                "parts":[{"outer":{"pts":[{"x":E,"y":N},{"x":E+6.,"y":N},
                    {"x":E+6.,"y":N+4.},{"x":E,"y":N+4.}]}}]
            }))
            .unwrap(),
        )
        .unwrap();
    let uid = b.doc.uid(room).unwrap().to_string();
    let made = tags_create::execute(
        &mut ExecutionContext::new(&mut b.doc),
        serde_json::from_value(json!({"layerId":"cizim", "tags":[
            {"of":uid,"at":{"x":E-10.,"y":N+2.},"leader":true},
            {"of":uid,"at":{"x":E+15.,"y":N+2.},"leader":true}
        ]}))
        .unwrap(),
    );
    let CommandResult::Completed { output, .. } = made else {
        panic!("{made:?}")
    };
    let tags = [Slot(output.ids[0]), Slot(output.ids[1])];
    b.selection.set([room, tags[0], tags[1]]);
    (b, room, tags)
}

#[test]
fn a_tag_grip_writes_only_its_placement_for_twenty_independent_destinations() {
    for [x, y] in [
        [-10., 6.],
        [-10., -6.],
        [-20., 2.],
        [-4., 2.],
        [-2., 7.],
        [-25., -8.],
        [0., 0.],
        [1., 1.],
        [30., 20.],
        [-30., -20.],
        [-9.5, 2.5],
        [-10.5, 1.5],
        [-10.125, 4.25],
        [-7.75, -2.125],
        [12., -4.],
        [25., 0.],
        [0., 20.],
        [-1., -1.],
        [-40., 10.],
        [40., -10.],
    ] {
        let (mut b, _, tags) = bench();
        let before = b.doc.to_snapshot_v2();
        let uid = b.doc.uid(tags[0]).unwrap();
        b.drag([-10., 2.], [x, y]);
        let mut expected = before.clone();
        let Entity::Tag(t) = expected
            .entities
            .iter_mut()
            .find(|e| e.base().id == tags[0].0)
            .unwrap()
        else {
            panic!()
        };
        t.tag.at = Vec2 { x: E + x, y: N + y };
        assert_eq!(b.doc.to_snapshot_v2(), expected, "destination {x},{y}");
        assert_eq!(b.doc.uid(tags[0]), Some(uid));
        assert!(!b.session.grip_active());
        let record = b.doc.journal().last().unwrap();
        assert_eq!(record.command.as_deref(), Some("cad.entities.transform"));
        assert_eq!(b.doc.undo().as_deref(), Some("Taşı"));
        assert_eq!(b.doc.to_snapshot_v2(), before);
        b.doc.redo().unwrap();
        assert_eq!(b.doc.to_snapshot_v2(), expected);
    }
}

#[test]
fn a_tag_grip_preview_and_escape_never_write_the_model() {
    let (mut b, _, tags) = bench();
    let before = b.doc.to_snapshot_v2();
    b.click(-10., 2.);
    assert!(b.session.grip_active());
    b.move_to(-10., 6.);
    assert!(
        b.session
            .preview(&Default::default())
            .unwrap()
            .strokes
            .iter()
            .any(|s| s.dash.is_some())
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(!b.session.grip_active());
    assert!(b.selection.ids().contains(&tags[0]));
}

fn heat(b: &mut Bench, x: f64, y: f64) {
    b.shift = true;
    b.move_to(x, y);
    b.click(x, y);
    b.shift = false;
}

#[test]
fn two_hot_tags_move_together_but_other_selected_objects_stay() {
    let (mut b, _, tags) = bench();
    heat(&mut b, -10., 2.);
    heat(&mut b, 15., 2.);
    assert_eq!(b.session.hot_grips().len(), 2);
    let before = b.doc.to_snapshot_v2();
    b.click(-10., 2.);
    b.move_to(-6., 6.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    let mut expected = before.clone();
    for (slot, x) in [(tags[0], -6.), (tags[1], 19.)] {
        let Entity::Tag(t) = expected
            .entities
            .iter_mut()
            .find(|e| e.base().id == slot.0)
            .unwrap()
        else {
            panic!()
        };
        t.tag.at = Vec2 {
            x: E + x,
            y: N + 6.,
        };
    }
    assert_eq!(b.doc.to_snapshot_v2(), expected);
    assert_eq!(b.doc.undo().as_deref(), Some("Taşı"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn mixed_tag_and_room_hot_grips_refuse_atomically() {
    let (mut b, _, _) = bench();
    heat(&mut b, -10., 2.);
    heat(&mut b, 3., 2.);
    assert_eq!(b.session.hot_grips().len(), 2);
    let before = b.doc.to_snapshot_v2();
    b.click(-10., 2.);
    b.move_to(-6., 6.);
    b.click(-6., 6.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.last_text().unwrap().contains("hiçbir nesne"));
}

#[test]
fn copy_from_the_tag_grip_keeps_the_source_and_the_same_target_link() {
    let (mut b, _, tags) = bench();
    let before = b.doc.to_snapshot_v2();
    let old = b.doc.get(tags[0]).unwrap().clone();
    b.click(-10., 2.);
    assert!(b.type_text("K"));
    b.move_to(-10., 8.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(-10., 8.);
    assert!(b.session.grip_active(), "copy stays hot");
    assert_eq!(b.doc.len(), before.entities.len() + 1);
    assert_eq!(b.doc.get(tags[0]), Some(&old));
    let mut expected = old;
    expected.base_mut().id = b.newest().base().id;
    let Entity::Tag(t) = &mut expected else {
        panic!()
    };
    t.tag.at = Vec2 {
        x: E - 10.,
        y: N + 8.,
    };
    assert_eq!(b.newest(), &expected);
    assert_eq!(b.doc.undo().as_deref(), Some("Kopyala"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn a_layer_locked_while_a_tag_grip_waits_is_not_written() {
    let (mut b, _, _) = bench();
    b.click(-10., 2.);
    assert!(b.session.grip_active());
    b.doc.toggle_layer_locked("cizim");
    let before = b.doc.to_snapshot_v2();
    b.move_to(-6., 6.);
    b.click(-6., 6.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.last_text().unwrap().contains("kilitli"));
}

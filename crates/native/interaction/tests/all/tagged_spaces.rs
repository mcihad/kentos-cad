//! A tagged space's tag is its label (docs/adr/0344's 27 October addendum): Hepsini etiketle used to leave the room's
//! own words under its tag, the same area twice in one place. A space with a tag draws no label of its own; its tag
//! gone (undone, deleted), the label comes back.

use crate::common::{Bench, E, N};
use kentos_commands::{Entity, SpacesCreate, TagsCreate};
use kentos_geometry_core::entity::Shape;
use kentos_native_application::{ExecutionContext, spaces_create, tags_create};
use serde_json::json;

/// How many texts the store draws for the object in `slot`.
fn texts(b: &Bench, slot: u32) -> usize {
    b.spatial
        .store()
        .get(f64::from(slot))
        .and_then(|it| it.expanded.as_ref())
        .map_or(0, |x| {
            x.shapes
                .iter()
                .filter(|s| matches!(s, Shape::Text { .. }))
                .count()
        })
}

#[test]
fn a_tagged_space_draws_its_tag_not_its_own_label() {
    let mut b = Bench::new("wall");
    b.session.exit();
    let layer = b.doc.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    let wall: Entity = serde_json::from_value(json!({"kind":"wall","id":0,"layerId":layer,"attrs":{},"width":0.2,"closed":true,
        "pts":[p(0., 0.), p(4., 0.), p(4., 3.), p(0., 3.)]}))
    .unwrap();
    b.doc.add_many(vec![wall], "Duvar").unwrap();
    let rooms: SpacesCreate = serde_json::from_value(json!({"layerId": layer, "spaces": [{"seed": p(2., 1.5)}]})).unwrap();
    let made = spaces_create::execute(&mut ExecutionContext::new(&mut b.doc), rooms);
    let kentos_commands::CommandResult::Completed { output, .. } = made else {
        panic!("the room: {made:?}");
    };
    let uid = output.uids[0].clone();
    let room = b.doc.slot_of(uid.parse().unwrap()).unwrap();
    b.spatial.sync(&b.doc);
    assert_eq!(texts(&b, room.0), 1, "an untagged room shows its own label");
    let tag: TagsCreate = serde_json::from_value(json!({"layerId": layer, "tags": [{"of": uid}]})).unwrap();
    assert!(matches!(
        tags_create::execute(&mut ExecutionContext::new(&mut b.doc), tag),
        kentos_commands::CommandResult::Completed { .. }
    ));
    b.spatial.sync(&b.doc);
    assert_eq!(texts(&b, room.0), 0, "the tag is its label now");
    // The room changed while tagged (put again): still no label of its own.
    b.spatial.reload(&b.doc);
    assert_eq!(texts(&b, room.0), 0, "read again: the same");
    b.doc.undo().unwrap();
    b.spatial.sync(&b.doc);
    assert_eq!(texts(&b, room.0), 1, "the tag undone, its label is back");
}

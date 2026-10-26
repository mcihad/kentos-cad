//! Slot exhaustion must not partly refresh existing labels; refresh-only needs no slot.
use kentos_commands::{CommandResult, DocumentSnapshotV1, Entity, TagsCreate};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, tags_create};
use serde_json::{Value, json};

fn drawing(all_tagged: bool) -> Document {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.tags.create.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap(),
    )
    .unwrap();
    let rooms: Vec<_> = doc
        .entities()
        .filter(|e| matches!(e, Entity::Space(_)))
        .map(|e| {
            doc.uid(kentos_domain::Slot(e.base().id))
                .unwrap()
                .to_string()
        })
        .collect();
    let tags: Vec<_> = rooms
        .into_iter()
        .take(if all_tagged { 2 } else { 1 })
        .map(|uid| json!({"of":uid,"at":{"x":-4.,"y":8.},"content":"{name}","leader":true}))
        .collect();
    assert!(matches!(
        tags_create::execute(
            &mut ExecutionContext::new(&mut doc),
            serde_json::from_value(json!({"layerId":"etiket","tags":tags})).unwrap()
        ),
        CommandResult::Completed { .. }
    ));
    let mut snapshot = doc.to_snapshot_v2();
    snapshot
        .entities
        .iter_mut()
        .find(|e| matches!(e, Entity::Line(_)))
        .unwrap()
        .base_mut()
        .id = u32::MAX;
    Document::from_snapshot_v2(snapshot).unwrap()
}
fn input() -> TagsCreate {
    serde_json::from_value(json!({"layerId":"etiket","all":{"kinds":["space"],"refresh":true,"content":"{number} / {name}"}})).unwrap()
}
#[test]
fn exhausted_slots_refuse_mixed_add_refresh_without_touching_old_tag_or_history() {
    let mut doc = drawing(false);
    let before = doc.to_snapshot_v2();
    let revision = doc.revision();
    let CommandResult::Completed { output: plan, .. } = tags_create::plan_in(&doc, &input()) else {
        panic!("plan")
    };
    assert_eq!(plan.tags.len(), 1);
    assert_eq!(plan.refreshed.len(), 1);
    assert_eq!(doc.to_snapshot_v2(), before);
    let result = tags_create::execute(&mut ExecutionContext::new(&mut doc), input());
    let CommandResult::Failed { error } = result else {
        panic!("must refuse")
    };
    assert_eq!(error.code, "slots_exhausted");
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.revision(), revision);
    assert!(!doc.is_dirty());
    assert!(!doc.can_undo());
    assert!(!doc.can_redo());
}
#[test]
fn refresh_without_new_tags_succeeds_at_slot_capacity_and_has_one_undo() {
    let mut doc = drawing(true);
    let before = doc.to_snapshot_v2();
    let CommandResult::Completed { output: plan, .. } = tags_create::plan_in(&doc, &input()) else {
        panic!("plan")
    };
    assert!(plan.tags.is_empty());
    assert_eq!(plan.refreshed.len(), 2);
    let CommandResult::Completed { output: result, .. } =
        tags_create::execute(&mut ExecutionContext::new(&mut doc), input())
    else {
        panic!("refresh uses no new slot")
    };
    assert!(result.uids.is_empty());
    assert!(result.ids.is_empty());
    assert_eq!(result.refreshed.len(), 2);
    let after = doc.to_snapshot_v2();
    let mut expected = before.clone();
    for entity in &mut expected.entities {
        if let Entity::Tag(t) = entity
            && t.base.layer_id == "etiket"
        {
            let source = if t.tag.of == before.uids[2].to_string() {
                "101 / Salon"
            } else {
                "102 / Mutfak"
            };
            t.tag.content = Some("{number} / {name}".into());
            t.tag.text = source.into();
        }
    }
    assert_eq!(after, expected);
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.can_undo());
    doc.redo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), after);
}

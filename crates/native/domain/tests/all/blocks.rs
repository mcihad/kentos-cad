//! Related block redefinitions are validated and recorded as a complete set.
use kentos_domain::Document;
use kentos_domain::contracts::{BlockDefinition, BlockId, DocumentSnapshotV2, Entity};
use serde_json::json;

fn drawing() -> Document {
    let snapshot: DocumentSnapshotV2 = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/dynamic-blocks.json"
    ))
    .expect("fixture");
    Document::from_snapshot_v2(snapshot).expect("drawing")
}

fn child() -> BlockDefinition {
    serde_json::from_value(json!({
        "id": "22222222-3333-4444-8555-666666666666", "name": "Yeni çocuk",
        "base": {"x": 0, "y": 0}, "entities": [
            {"kind": "line", "id": 1, "layerId": "", "attrs": {},
             "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 0}}
        ]
    }))
    .expect("child")
}

fn insert(block: BlockId, id: u32) -> Entity {
    serde_json::from_value(json!({
        "kind": "insert", "id": id, "layerId": "", "attrs": {},
        "block": block, "p": {"x": 1.5, "y": 0}, "scale": 1, "rotation": 0
    }))
    .expect("insert")
}

#[test]
fn existing_parent_can_acquire_a_new_child_with_one_undo_and_redo() {
    for child_first in [false, true] {
        let mut doc = drawing();
        let before = doc.to_snapshot_v2();
        let child = child();
        let mut parent = (*doc.blocks()[0]).clone();
        parent.entities.push(insert(child.id, 5));
        let changes = if child_first {
            vec![child, parent]
        } else {
            vec![parent, child]
        };
        doc.merge_blocks(&changes).expect("whole graph is valid");
        let after = doc.to_snapshot_v2();
        assert_eq!(after.blocks.len(), 2);
        assert_eq!(doc.undo_labels().len(), 1);
        assert_ne!(before, after);
        doc.undo().expect("one edit");
        assert_eq!(doc.to_snapshot_v2(), before);
        assert!(!doc.can_undo());
        doc.redo().expect("one redo");
        assert_eq!(doc.to_snapshot_v2(), after);
    }
}

#[test]
fn bad_sets_write_nothing_and_leave_pending_redo_usable() {
    let mut doc = drawing();
    let child = child();
    doc.merge_blocks(std::slice::from_ref(&child))
        .expect("add child");
    let after = doc.to_snapshot_v2();
    doc.undo();
    let before = doc.to_snapshot_v2();
    let revision = doc.revision();
    let mut parent = (*doc.blocks()[0]).clone();
    parent.entities.push(insert(child.id, 5));
    let mut cycle = child.clone();
    cycle.entities.push(insert(parent.id, 2));
    let mut unknown = child.clone();
    unknown.entities.push(insert(BlockId([7; 16]), 2));
    let mut duplicate_name = child.clone();
    duplicate_name.name.clone_from(&parent.name);
    for bad in [
        vec![parent.clone(), cycle],
        vec![parent, unknown],
        vec![child.clone(), child.clone()],
        vec![duplicate_name],
    ] {
        assert!(doc.merge_blocks(&bad).is_err());
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.revision(), revision);
        assert!(!doc.can_undo());
        assert!(doc.can_redo());
    }
    doc.merge_blocks(&[]).expect("empty is no-op");
    let same = (*doc.blocks()[0]).clone();
    doc.merge_blocks(&[same]).expect("identical is no-op");
    assert_eq!(doc.revision(), revision);
    assert!(doc.can_redo());
    doc.redo()
        .expect("pending redo survives refusals and no-ops");
    assert_eq!(doc.to_snapshot_v2(), after);
}

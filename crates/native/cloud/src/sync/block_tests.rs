//! Block definitions in the desktop's autosave (docs/adr/0144 §5), the
//! web's cases (apps/web/src/app/cloud/syncBlocks.test.ts) without a server:
//! what each edit sends and in which order, what the server's list does to
//! the drawing, the conflicts, the rules that keep data over a removal and a
//! name once, the device draft, and the local copy's base. The same rules
//! against the real server: crates/server/application/tests/blocks.rs.

use super::tests::{committed, editor, event, input, page, point, record, reopened};
use super::*;
use kentos_contracts::{
    BlockChange, BlockDefinition, EventBlock, EventPage, FeatureConflict, FeatureOp,
};

use crate::open::Opened;

fn bid(n: u8) -> BlockId {
    BlockId(
        Uuid::parse_str(&format!("00000000-0000-7000-8000-00000000b{n:03x}"))
            .unwrap()
            .into_bytes(),
    )
}

/// A definition `n` with a line and inserts of the blocks `inside`.
fn def(n: u8, name: &str, inside: &[u8]) -> BlockDefinition {
    let mut entities = vec![serde_json::json!({ "kind": "line", "id": 1, "layerId": "", "attrs": {},
        "a": { "x": 0, "y": 0 }, "b": { "x": 1, "y": 0 } })];
    for (k, b) in inside.iter().enumerate() {
        entities.push(serde_json::json!({ "kind": "insert", "id": k + 2, "layerId": "", "attrs": {},
            "block": bid(*b).to_string(), "p": { "x": 0, "y": 0 }, "scale": 1, "rotation": 0 }));
    }
    serde_json::from_value(serde_json::json!({ "id": bid(n).to_string(), "name": name,
        "base": { "x": 0, "y": 0 }, "entities": entities }))
    .unwrap()
}

fn renamed(b: &BlockDefinition, name: &str) -> BlockDefinition {
    BlockDefinition {
        name: name.into(),
        ..b.clone()
    }
}

/// An insert of block `n` on `parsel`.
fn insert_of(n: u8, x: f64) -> Entity {
    serde_json::from_value(serde_json::json!({ "kind": "insert", "id": 0, "layerId": "parsel", "attrs": {},
        "block": bid(n).to_string(), "p": { "x": x, "y": 4_420_210.0 }, "scale": 1, "rotation": 0 }))
    .unwrap()
}

fn listed(list: &[(&BlockDefinition, &str)]) -> Vec<BlockRecord> {
    list.iter()
        .map(|(b, v)| BlockRecord {
            version: v.to_string(),
            block: (*b).clone(),
        })
        .collect()
}

/// The sample project opened with these definitions at these versions.
fn opened_with(list: &[(&BlockDefinition, &str)]) -> Opened {
    let mut o = editor();
    with_blocks(&mut o, list);
    o
}

fn with_blocks(o: &mut Opened, list: &[(&BlockDefinition, &str)]) {
    let mut snap = o.document.to_snapshot_v2();
    snap.blocks = list.iter().map(|(b, _)| (*b).clone()).collect();
    o.document = Document::from_snapshot_v2(snap).unwrap();
    o.info.blocks = listed(list);
    if let Source::Database { blocks, .. } = &mut o.source {
        *blocks = list.iter().map(|(b, v)| (b.id, v.to_string())).collect();
    }
}

/// The server's answer: every created or updated object and definition at `revision`.
fn committed_all(env: &CommandEnvelope, revision: u64) -> CommitResult {
    let mut result = committed(env, revision);
    for c in input(env).blocks {
        match c {
            BlockChange::Create { block } | BlockChange::Update { block } => {
                result
                    .versions
                    .insert(block_key(block.id), revision.to_string());
            }
            BlockChange::Delete { id } => result.deleted.push(block_key(id)),
        }
    }
    result
}

fn block_event(seq: u64, features: &[(Uuid, FeatureOp)], blocks: &[(BlockId, FeatureOp)]) -> EventPage {
    let mut e = event(seq, None, features, false);
    e.blocks = blocks
        .iter()
        .map(|(id, op)| EventBlock {
            id: id.to_string(),
            op: *op,
            version: None,
        })
        .collect();
    page(vec![e])
}

fn take(
    sync: &mut ProjectSync,
    doc: &mut Document,
    page: &EventPage,
    records: Vec<FeatureRecord>,
    blocks: Vec<BlockRecord>,
) -> Taken {
    let incoming = sync.incoming(page);
    sync.take_remote(
        doc,
        incoming,
        Remote {
            records,
            info: None,
            blocks: Some(blocks),
        },
    )
    .unwrap()
}

fn names(doc: &Document) -> Vec<&str> {
    doc.blocks().iter().map(|b| b.name.as_str()).collect()
}

#[test]
fn an_opened_projects_definitions_are_known_and_nothing_waits() {
    let a = def(1, "Kapı", &[]);
    let o = opened_with(&[(&a, "3")]);
    let sync = ProjectSync::new(&o).unwrap();
    assert_eq!(sync.block_version_of(a.id), Some("3"));
    assert_eq!(sync.pending(), 0);
    assert!(sync.all_sent());
}

#[test]
fn a_block_made_and_placed_goes_with_its_insert_and_takes_its_version() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add_block(def(1, "Kapı", &[])).unwrap();
    o.document.add(insert_of(1, 486_600.0)).unwrap();
    sync.observe(&o.document);
    assert_eq!(sync.pending(), 2);
    let env = sync.next(&o.document).unwrap();
    let changes = input(&env);
    assert!(matches!(&changes.blocks[..], [BlockChange::Create { block }] if block.name == "Kapı"));
    assert_eq!(changes.features.len(), 1);
    assert!(env.expected_versions.is_empty());
    sync.answered(&o.document, &committed_all(&env, 2));
    assert_eq!(sync.block_version_of(bid(1)), Some("2"));
    assert!(sync.all_sent());
    assert_eq!(sync.next(&o.document), None);
}

#[test]
fn a_changed_definition_goes_over_its_version_and_a_removed_one_with_its_last_insert() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    let placed = o.document.add(insert_of(1, 486_600.0)).unwrap();
    let env = sync.next(&o.document).unwrap();
    sync.answered(&o.document, &committed_all(&env, 4));
    assert!(o.document.update_block(renamed(&a, "Pencere")).unwrap());
    let env = sync.next(&o.document).unwrap();
    assert!(matches!(&input(&env).blocks[..], [BlockChange::Update { block }] if block.name == "Pencere"));
    assert_eq!(env.expected_versions.get(&block_key(a.id)).map(String::as_str), Some("3"));
    sync.answered(&o.document, &committed_all(&env, 5));
    let uid = o.document.uid(placed).unwrap();
    o.document.remove(&[placed]);
    assert!(o.document.remove_block(a.id).unwrap());
    let env = sync.next(&o.document).unwrap();
    let changes = input(&env);
    assert!(matches!(&changes.features[..], [FeatureChange::Delete { id }] if *id == uid.to_string()));
    assert!(matches!(&changes.blocks[..], [BlockChange::Delete { id }] if *id == a.id));
    assert_eq!(env.expected_versions.get(&block_key(a.id)).map(String::as_str), Some("5"));
    sync.answered(&o.document, &committed_all(&env, 6));
    assert_eq!(sync.block_version_of(a.id), None);
    assert!(sync.all_sent());
}

#[test]
fn an_undo_back_to_the_servers_definitions_sends_nothing() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    assert!(o.document.update_block(renamed(&a, "Pencere")).unwrap());
    sync.observe(&o.document);
    assert_eq!(sync.pending(), 1);
    o.document.undo();
    sync.observe(&o.document);
    assert_eq!(sync.pending(), 0);
    assert_eq!(sync.next(&o.document), None);
}

#[test]
fn many_objects_the_definition_goes_first_and_its_removal_last() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add_block(def(1, "Kapı", &[])).unwrap();
    let mut made: Vec<Entity> = (0..2100).map(|i| point(486_000.0 + f64::from(i))).collect();
    made.push(insert_of(1, 486_600.0));
    let slots = o.document.add_many(made, "Ekle").unwrap();
    let counts = |env: &CommandEnvelope| {
        let c = input(env);
        (c.features.len(), c.blocks.len())
    };
    let first = sync.next(&o.document).unwrap();
    assert_eq!(counts(&first), (BATCH - 1, 1));
    sync.answered(&o.document, &committed_all(&first, 2));
    let second = sync.next(&o.document).unwrap();
    assert_eq!(counts(&second), (2101 - (BATCH - 1), 0));
    sync.answered(&o.document, &committed_all(&second, 3));
    o.document.remove(&slots);
    assert!(o.document.remove_block(bid(1)).unwrap());
    let third = sync.next(&o.document).unwrap();
    assert_eq!(counts(&third), (BATCH, 0));
    sync.answered(&o.document, &committed_all(&third, 4));
    let last = sync.next(&o.document).unwrap();
    assert_eq!(counts(&last), (2101 - BATCH, 1));
    assert!(matches!(&input(&last).blocks[..], [BlockChange::Delete { .. }]));
}

#[test]
fn long_lists_go_inner_first_when_made_and_outer_first_when_removed() {
    let mut o = editor();
    let sync = ProjectSync::new(&o).unwrap();
    // The outer one first in the drawing, placing the inner one only once redefined.
    o.document.add_block(def(1, "Dış", &[])).unwrap();
    o.document.add_block(def(2, "İç", &[])).unwrap();
    o.document.update_block(def(1, "Dış", &[2])).unwrap();
    let (upserts, _) = sync.ordered_blocks(&o.document, sync.plan_blocks(&o.document));
    let ids: Vec<BlockId> = upserts.iter().map(PlannedBlock::id).collect();
    assert_eq!(ids, [bid(2), bid(1)]);
    let (outer, inner) = (def(1, "Dış", &[2]), def(2, "İç", &[]));
    let mut o = opened_with(&[(&inner, "3"), (&outer, "3")]);
    let sync = ProjectSync::new(&o).unwrap();
    o.document.remove_block(bid(1)).unwrap();
    o.document.remove_block(bid(2)).unwrap();
    let (_, deletes) = sync.ordered_blocks(&o.document, sync.plan_blocks(&o.document));
    let ids: Vec<BlockId> = deletes.iter().map(PlannedBlock::id).collect();
    assert_eq!(ids, [bid(1), bid(2)]);
}

#[test]
fn anothers_definition_and_its_insert_come_in_one_change_without_an_undo_step() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    let theirs = Uuid::now_v7();
    let b = def(2, "Ağaç", &[]);
    let taken = take(
        &mut sync,
        &mut o.document,
        &block_event(10, &[(theirs, FeatureOp::Create)], &[(a.id, FeatureOp::Update), (b.id, FeatureOp::Create)]),
        vec![record(theirs, "10", insert_of(2, 486_700.0))],
        listed(&[(&renamed(&a, "Kapı 2"), "4"), (&b, "10")]),
    );
    assert_eq!((taken.conflicts, taken.changed), (0, 3));
    assert_eq!(names(&o.document), ["Kapı 2", "Ağaç"]);
    assert!(o.document.slot_of(theirs).is_some());
    assert_eq!((sync.block_version_of(a.id), sync.block_version_of(b.id)), (Some("4"), Some("10")));
    assert!(!o.document.is_dirty() && !o.document.can_undo());
    assert_eq!(sync.pending(), 0);
}

#[test]
fn a_definition_changed_here_and_there_is_a_conflict_until_theirs_is_taken() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.update_block(renamed(&a, "Benim")).unwrap();
    let theirs = listed(&[(&renamed(&a, "Onların"), "4")]);
    let taken = take(&mut sync, &mut o.document, &block_event(10, &[], &[(a.id, FeatureOp::Update)]), vec![], theirs.clone());
    assert_eq!(taken.conflicts, 1);
    assert_eq!(sync.conflicts()[0].id, block_key(a.id));
    assert_eq!(sync.conflicts()[0].actual.as_deref(), Some("4"));
    assert!(sync.conflicts_blocks());
    assert_eq!(names(&o.document), ["Benim"]);
    // Without the server's list the choice waits.
    sync.take_theirs(&mut o.document, None, None).unwrap();
    assert_eq!(sync.state(), SaveState::Conflict);
    sync.take_theirs(&mut o.document, None, Some(&theirs)).unwrap();
    assert_eq!(names(&o.document), ["Onların"]);
    assert_eq!((sync.state(), sync.block_version_of(a.id)), (SaveState::Saved, Some("4")));
}

#[test]
fn a_definition_changed_here_and_there_goes_over_theirs_when_mine_is_kept() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.update_block(renamed(&a, "Benim")).unwrap();
    let env = sync.next(&o.document).unwrap();
    let refused = ApiFailure::new(409, "conflict", "Çakışma").with_conflicts(vec![FeatureConflict {
        id: block_key(a.id),
        reason: ConflictReason::Changed,
        expected: Some("3".into()),
        actual: Some("4".into()),
        current: None,
    }]);
    assert_eq!(sync.failed(&refused), After::Stop);
    drop(env);
    assert!(!sync.may_give_back_blocks(), "not a guard: the versions differ");
    sync.keep_mine(&o.document, Some(&listed(&[(&renamed(&a, "Onların"), "4")])));
    let env = sync.next(&o.document).unwrap();
    assert!(matches!(&input(&env).blocks[..], [BlockChange::Update { block }] if block.name == "Benim"));
    assert_eq!(env.expected_versions.get(&block_key(a.id)).map(String::as_str), Some("4"));
}

#[test]
fn a_definition_removed_there_stays_while_unsent_inserts_here_place_it() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add(insert_of(1, 486_600.0)).unwrap();
    let taken = take(&mut sync, &mut o.document, &block_event(10, &[], &[(a.id, FeatureOp::Delete)]), vec![], vec![]);
    assert_eq!(names(&o.document), ["Kapı"]);
    assert_eq!(taken.notes, [kept_block_text("Kapı")]);
    assert_eq!(sync.block_version_of(a.id), None);
    let env = sync.next(&o.document).unwrap();
    assert!(matches!(&input(&env).blocks[..], [BlockChange::Create { .. }]));
    assert_eq!(input(&env).features.len(), 1);
}

#[test]
fn a_definition_removed_here_comes_back_with_an_arriving_insert() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.remove_block(a.id).unwrap();
    let theirs = Uuid::now_v7();
    let taken = take(
        &mut sync,
        &mut o.document,
        &block_event(10, &[(theirs, FeatureOp::Create)], &[]),
        vec![record(theirs, "10", insert_of(1, 486_700.0))],
        listed(&[(&a, "3")]),
    );
    assert_eq!(taken.notes, [restored_block_text("Kapı")]);
    assert_eq!(names(&o.document), ["Kapı"]);
    assert!(o.document.slot_of(theirs).is_some());
    sync.observe(&o.document);
    assert_eq!(sync.pending(), 0);
}

#[test]
fn removed_here_and_there_alike_asks_nothing() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.remove_block(a.id).unwrap();
    let taken = take(&mut sync, &mut o.document, &block_event(10, &[], &[(a.id, FeatureOp::Delete)]), vec![], vec![]);
    assert_eq!(taken.conflicts, 0);
    assert_eq!(sync.block_version_of(a.id), None);
    assert_eq!(sync.next(&o.document), None);
}

#[test]
fn a_name_someone_else_took_first_gives_way_here() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add_block(def(1, "Blok 1", &[])).unwrap();
    let theirs = def(2, "BLOK 1", &[]);
    let taken = take(&mut sync, &mut o.document, &block_event(10, &[], &[(theirs.id, FeatureOp::Create)]), vec![], listed(&[(&theirs, "10")]));
    assert_eq!(names(&o.document), ["Blok 1 (2)", "BLOK 1"]);
    assert_eq!(taken.notes, [renamed_block_text("Blok 1", "Blok 1 (2)")]);
    let env = sync.next(&o.document).unwrap();
    assert!(matches!(&input(&env).blocks[..], [BlockChange::Create { block }] if block.name == "Blok 1 (2)"));
}

#[test]
fn a_refused_removal_of_a_placed_definition_puts_it_back_and_asks_nothing() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.remove_block(a.id).unwrap();
    let env = sync.next(&o.document).unwrap();
    assert!(matches!(&input(&env).blocks[..], [BlockChange::Delete { .. }]));
    // Someone else's insert places it there: the server keeps it, and says so with its own version.
    let guard = ApiFailure::new(409, "conflict", "“Kapı” bloğu kullanılıyor").with_conflicts(vec![FeatureConflict {
        id: block_key(a.id),
        reason: ConflictReason::Changed,
        expected: Some("3".into()),
        actual: Some("3".into()),
        current: None,
    }]);
    assert_eq!(sync.failed(&guard), After::Stop);
    assert!(sync.may_give_back_blocks());
    let notes = sync.give_back_blocks(&mut o.document, &listed(&[(&a, "3")])).unwrap();
    assert_eq!(notes, [restored_block_text("Kapı")]);
    assert_eq!(names(&o.document), ["Kapı"]);
    assert!(sync.conflicts().is_empty());
    assert_eq!(sync.state(), SaveState::Saved);
    assert_eq!(sync.next(&o.document), None);
}

#[test]
fn unsent_definitions_go_to_the_draft_and_come_back_before_their_inserts() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add_block(def(1, "Kapı", &[])).unwrap();
    let placed = o.document.add(insert_of(1, 486_600.0)).unwrap();
    let uid = o.document.uid(placed).unwrap();
    let draft = sync.draft(&o.document, "ayse").unwrap();
    assert_eq!(
        draft.blocks[&bid(1).to_string()],
        DraftBlock {
            base: None,
            block: Some(def(1, "Kapı", &[])),
        }
    );
    let text = serde_json::to_string(&draft).unwrap();
    let draft: Draft = serde_json::from_str(&text).unwrap();
    let mut again = reopened(&o);
    let mut sync = ProjectSync::new(&again).unwrap();
    let restored = sync.restore(&mut again.document, draft).unwrap();
    assert_eq!((restored.conflicts, restored.held.len()), (0, 0));
    assert_eq!(names(&again.document), ["Kapı"]);
    assert!(again.document.slot_of(uid).is_some());
    assert_eq!(sync.pending(), 2);
}

#[test]
fn a_drafts_definition_someone_changed_meanwhile_is_a_conflict_shown_as_mine() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.update_block(renamed(&a, "Benim")).unwrap();
    let draft = sync.draft(&o.document, "ayse").unwrap();
    let mut again = reopened(&o);
    with_blocks(&mut again, &[(&renamed(&a, "Onların"), "4")]);
    let mut sync = ProjectSync::new(&again).unwrap();
    let restored = sync.restore(&mut again.document, draft).unwrap();
    assert_eq!(restored.conflicts, 1);
    assert_eq!(names(&again.document), ["Benim"]);
    assert_eq!(sync.conflicts()[0].id, block_key(a.id));
}

#[test]
fn a_drafts_removal_of_a_definition_someone_placed_meanwhile_is_left_out() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.remove_block(a.id).unwrap();
    let draft = sync.draft(&o.document, "ayse").unwrap();
    assert_eq!(draft.blocks[&a.id.to_string()].block, None);
    // Opened again: someone else placed it meanwhile.
    let mut again = opened_with(&[(&a, "3")]);
    again.document.add(insert_of(1, 486_700.0)).unwrap();
    let mut sync = ProjectSync::new(&again).unwrap();
    let restored = sync.restore(&mut again.document, draft).unwrap();
    assert_eq!(names(&again.document), ["Kapı"]);
    assert_eq!(restored.conflicts, 0);
    assert!(restored.notes[0].contains("taslaktaki silinmesi uygulanmadı"), "{:?}", restored.notes);
}

#[test]
fn a_command_with_definitions_on_its_way_goes_again_with_its_key() {
    let mut o = editor();
    let mut sync = ProjectSync::new(&o).unwrap();
    o.document.add_block(def(1, "Kapı", &[])).unwrap();
    o.document.add(insert_of(1, 486_600.0)).unwrap();
    let env = sync.next(&o.document).unwrap();
    let draft = sync.draft(&o.document, "ayse").unwrap();
    assert!(draft.inflight.is_some());
    let mut again = reopened(&o);
    let mut sync = ProjectSync::new(&again).unwrap();
    let restored = sync.restore(&mut again.document, draft).unwrap();
    assert!(restored.resends);
    assert_eq!(names(&again.document), ["Kapı"]);
    let same = sync.next(&again.document).unwrap();
    assert_eq!(same.idempotency_key, env.idempotency_key);
    sync.answered(&again.document, &committed_all(&same, 2));
    assert_eq!(sync.block_version_of(bid(1)), Some("2"));
    assert!(sync.all_sent());
}

#[test]
fn what_the_server_has_of_the_definitions_is_a_base_step_and_in_the_base() {
    let a = def(1, "Kapı", &[]);
    let mut o = opened_with(&[(&a, "3")]);
    let mut sync = ProjectSync::new(&o).unwrap();
    assert_eq!(sync.take_base_step(), None, "what the opening read is known already");
    let base = sync.base(&o.document);
    assert_eq!(base.block_versions, [(a.id, "3".to_string())]);
    assert_eq!(base.snapshot.blocks, std::slice::from_ref(&a));
    take(&mut sync, &mut o.document, &block_event(10, &[], &[(a.id, FeatureOp::Update)]), vec![], listed(&[(&renamed(&a, "Kapı 2"), "4")]));
    let step = sync.take_base_step().unwrap();
    assert_eq!(
        step.blocks,
        Some(vec![BaseBlock {
            version: "4".into(),
            block: renamed(&a, "Kapı 2"),
        }])
    );
}

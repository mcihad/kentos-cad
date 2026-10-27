//! Export boundaries are explicit: DXF keeps dimensions, WBLOCK cannot silently drop ownership.
use kentos_commands::{DocumentSnapshotV2, Entity, ReportSeverity};
use kentos_domain::Document;
use kentos_headless::{Session, export};
use serde_json::{Value, json};

const MODEL: &str = include_str!("../../../../../fixtures/kcad/v2/dimension-sets.json");

fn model() -> DocumentSnapshotV2 {
    serde_json::from_str(MODEL).unwrap()
}
fn session() -> Session {
    Session::from_bytes(&kentos_kcad::encode_verified(&model()).unwrap()).unwrap()
}
fn recipe_notes(input: &kentos_commands::DxfWriteInput) -> Vec<&kentos_commands::ReportItem> {
    input
        .notes
        .iter()
        .filter(|note| note.what == "İlişkili duvar ölçü grubu")
        .collect()
}
fn report_notes(answer: &Value) -> Vec<&Value> {
    answer["output"]["report"]["notes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|note| note["what"] == "İlişkili duvar ölçü grubu")
        .collect()
}

#[test]
fn dxf_recipe_note_counts_only_groups_with_selected_active_members() {
    let mut source = model();
    // A second independent recipe on the same source has a separate ordinary child.
    let mut other = source.dimension_sets[0].clone();
    other.id = kentos_commands::DimensionSetId([0x51; 16]);
    other.members[0].uid = kentos_commands::EntityId([0x52; 16]);
    let mut child = source.entities[1].clone();
    child.base_mut().id = 3;
    source.entities.push(child);
    source.uids.push(other.members[0].uid);
    source.dimension_sets.push(other);
    let doc = Document::from_snapshot_v2(source).unwrap();
    let all = export::dxf_input(&doc, &export::by_layer(&doc, doc.entities()));
    let notes = recipe_notes(&all);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].count, 2);
    assert_eq!(notes[0].severity, Some(ReportSeverity::Warning));
    assert!(notes[0].reason.contains("KCAD"));
    assert!(notes[0].reason.contains("otomatik üye ekleme/kaldırma"));
    let subset = export::dxf_input(
        &doc,
        &export::by_layer(&doc, doc.entities().filter(|e| e.base().id == 2)),
    );
    assert_eq!(recipe_notes(&subset)[0].count, 1);
    let source_only = export::dxf_input(
        &doc,
        &export::by_layer(&doc, doc.entities().filter(|e| e.base().id == 1)),
    );
    assert!(recipe_notes(&source_only).is_empty());
}

#[test]
fn no_group_dxf_input_is_unchanged_except_for_the_additive_recipe_note() {
    let source = model();
    let grouped = Document::from_snapshot_v2(source.clone()).unwrap();
    let mut plain = source;
    plain.dimension_sets.clear();
    let plain = Document::from_snapshot_v2(plain).unwrap();
    let mut with = export::dxf_input(&grouped, &export::by_layer(&grouped, grouped.entities()));
    let without = export::dxf_input(&plain, &export::by_layer(&plain, plain.entities()));
    assert!(recipe_notes(&without).is_empty());
    with.notes
        .retain(|note| note.what != "İlişkili duvar ölçü grubu");
    assert_eq!(
        serde_json::to_value(with).unwrap(),
        serde_json::to_value(without).unwrap()
    );
}

#[test]
fn real_dxf_plan_and_write_report_loss_and_keep_ordinary_measurable_dimensions() {
    let mut session = session();
    let folder = super::scratch("dimension-group-dxf");
    let path = folder.join("dimensions.dxf");
    let before = session.document().to_snapshot_v2();
    let plan = session
        .call(json!({"command":"file.export.dxf","op":"plan","input":{"path":path}}))
        .unwrap();
    assert_eq!(plan["status"], "completed", "{plan}");
    assert!(!path.exists());
    assert_eq!(report_notes(&plan).len(), 1);
    assert_eq!(report_notes(&plan)[0]["severity"], "warning");
    assert_eq!(report_notes(&plan)[0]["count"], 1);
    assert_eq!(session.document().to_snapshot_v2(), before);
    let written = session
        .call(json!({"command":"file.export.dxf","input":{"path":path}}))
        .unwrap();
    assert_eq!(written["status"], "completed", "{written}");
    assert_eq!(written["output"]["sha256"], plan["output"]["sha256"]);
    assert_eq!(report_notes(&written), report_notes(&plan));
    let read = kentos_formats::dxf::read(
        &std::fs::read(&path).unwrap(),
        &kentos_commands::DxfReadOptions::default(),
    )
    .unwrap();
    let dimensions: Vec<_> = read
        .entities
        .iter()
        .filter_map(|entity| {
            if let Entity::Dimension(d) = entity {
                Some(d)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(dimensions.len(), 1);
    assert!((dimensions[0].b.x - dimensions[0].a.x - 8.).abs() < 1e-9);
    assert_eq!(session.document().dimension_sets().len(), 1);
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn wblock_owned_member_refuses_validate_plan_execute_without_writing_or_replacing_files() {
    let mut session = session();
    let folder = super::scratch("dimension-group-wblock");
    let path = folder.join("must-stay.kcad");
    let original = b"existing file bytes";
    std::fs::write(&path, original).unwrap();
    let source = model();
    let member = source.uids[1].to_text();
    let host = source.uids[0].to_text();
    let before = session.document().to_snapshot_v2();
    for op in ["validate", "plan", "execute"] {
        for selection in [vec![member.clone()], vec![host.clone(), member.clone()]] {
            let refused=session.call(json!({"command":"file.export.block","op":op,"input":{"path":path,"uids":selection,"overwrite":true}})).unwrap();
            assert_eq!(refused["status"], "failed", "{refused}");
            assert_eq!(refused["error"]["code"], "dimension_set_owned");
            assert!(
                refused["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("bağımsızlaştır")
            );
            assert_eq!(std::fs::read(&path).unwrap(), original);
            assert_eq!(
                session.document().to_snapshot_v2().entities,
                before.entities
            );
            assert_eq!(
                session.document().to_snapshot_v2().dimension_sets,
                before.dimension_sets
            );
        }
    }
    let absent = folder.join("must-not-exist.kcad");
    let refused = session
        .call(json!({"command":"file.export.block","input":{"path":absent,"uids":[member]}}))
        .unwrap();
    assert_eq!(refused["error"]["code"], "dimension_set_owned");
    assert!(!absent.exists());
    let detached=session.call(json!({"command":"cad.dimensionsets.edit","input":{"id":source.dimension_sets[0].id,"operation":{"kind":"detach"}}})).unwrap();
    assert_eq!(detached["status"], "completed", "{detached}");
    let plan=session.call(json!({"command":"file.export.block","op":"plan","input":{"path":absent,"uids":[source.uids[1]]}})).unwrap();
    assert_eq!(plan["status"], "completed", "{plan}");
    assert!(!absent.exists());
    let written = session
        .call(
            json!({"command":"file.export.block","input":{"path":absent,"uids":[source.uids[1]]}}),
        )
        .unwrap();
    assert_eq!(written["status"], "completed", "{written}");
    assert_eq!(written["output"]["sha256"], plan["output"]["sha256"]);
    let opened = kentos_kcad::decode(&std::fs::read(&absent).unwrap()).unwrap();
    assert!(opened.dimension_sets.is_empty());
    assert_eq!(opened.entities.len(), 1);
    let _ = std::fs::remove_dir_all(folder);
}

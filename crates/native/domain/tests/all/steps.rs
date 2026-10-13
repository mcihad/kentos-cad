//! A command's step (`Document::run_step`, docs/adr/0253 §1), the state
//! stamps (§2), the journal and the request ledger (§3, §6), and the
//! integrity a step must keep (§8).

use kentos_domain::contracts::{
    CallKind, CallRecord, CallStatus, DocumentSnapshotV1, Entity, EntityBase, PointEntity,
    ProjectPart, RequestRecord, TextEntity, Vec2,
};
use kentos_domain::{Document, Sealed, Slot};

/// A small drawing: group g (layers a, b) and layer c, no objects.
fn empty() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Deneme","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000},"origin":{"x":0,"y":0},
        "layers":[{"id":"g","name":"G","type":"group","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[
        {"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]},
        {"id":"b","name":"B","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}]},
        {"id":"c","name":"C","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"b","entities":[],"styles":{"items":[],"categories":[]}}"#;
    Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens")
}

fn base(layer: &str) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.into(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

fn point(layer: &str, x: f64) -> Entity {
    Entity::Point(PointEntity {
        base: base(layer),
        p: Vec2 { x, y: 0.0 },
        z: None,
        parts: None,
    })
}

fn moved(doc: &Document, slot: Slot, x: f64) -> Entity {
    match doc.get(slot) {
        Some(Entity::Point(p)) => Entity::Point(PointEntity {
            p: Vec2 { x, y: p.p.y },
            ..p.clone()
        }),
        other => panic!("not a point: {other:?}"),
    }
}

/// A command's row as a seal gives it.
fn row(command: &str) -> CallRecord {
    CallRecord {
        seq: 0,
        kind: CallKind::Command,
        label: None,
        command: Some(command.into()),
        version: Some(1),
        input: None,
        input_digest: None,
        request_id: None,
        fingerprint: None,
        base: "0".into(),
        revision: String::new(),
        context: None,
        algorithm: None,
        seed: None,
        created: Vec::new(),
        changed: Vec::new(),
        removed: Vec::new(),
        truncated: false,
        warnings: Vec::new(),
        parts: Vec::new(),
        effects: Vec::new(),
        step: None,
        status: CallStatus::Done,
    }
}

#[test]
fn a_refused_step_takes_back_what_is_no_undo_step_too() {
    let mut doc = empty();
    let a = doc.add(point("a", 1.0)).unwrap();
    let revision = doc.revision();
    doc.mark_saved(revision);
    let (name, settings, flags) = (
        doc.settings_stamp(),
        doc.settings().clone(),
        doc.layers_stamp(),
    );
    let result: Result<((), _), &str> = doc.run_step(
        |doc| {
            let next = moved(doc, a, 5.0);
            doc.update(a, next);
            doc.set_name("Başka");
            let mut s = doc.settings().clone();
            s.plot_scale = 500.0;
            doc.set_settings(s);
            doc.set_layer_visible("a", false);
            doc.set_layer_locked("c", true);
            doc.rename_layer("b", "Bé");
            doc.set_active_layer("a");
            Err("hayır")
        },
        |_, _, _| Ok(None),
    );
    assert_eq!(result.err(), Some("hayır"));
    assert_eq!(doc.name(), "Deneme");
    assert_eq!(doc.settings(), &settings);
    assert_eq!(doc.settings_stamp(), name);
    assert_eq!(
        doc.layers_stamp(),
        flags,
        "the flags and the active layer came back"
    );
    assert_eq!(doc.layers().active(), "b");
    assert!(matches!(doc.get(a), Some(Entity::Point(p)) if p.p.x == 1.0));
    assert_eq!(doc.revision(), revision, "the revision came back");
    assert!(!doc.is_dirty(), "and the drawing is as clean as it was");
    assert!(doc.can_undo(), "the earlier step is still there");
    assert_eq!(
        doc.journal().count(),
        1,
        "only the earlier edit is in the journal"
    );
}

#[test]
fn a_seal_sees_the_step_as_it_will_be_recorded_and_may_refuse_it() {
    let mut doc = empty();
    let a = doc.add(point("a", 1.0)).unwrap();
    let uid = doc.uid(a).unwrap();
    let before = doc.stamp(a).unwrap();
    let mut seen = None;
    let result: Result<((), _), String> = doc.run_step(
        |doc| {
            let next = moved(doc, a, 2.0);
            doc.update(a, next);
            Ok(())
        },
        |doc, report, _| {
            seen = Some(report.clone());
            assert_ne!(doc.stamp(a).unwrap(), before, "the seal sees the write");
            Err("plan_changed".into())
        },
    );
    assert_eq!(result.err().as_deref(), Some("plan_changed"));
    let report = seen.expect("sealed");
    assert!(report.outermost);
    assert_eq!(
        report.label.as_deref(),
        Some("Değiştir"),
        "the first edit's name"
    );
    assert_eq!(report.objects.len(), 1);
    assert_eq!(report.objects[0].uid, uid);
    assert!(report.objects[0].changed());
    assert_eq!(doc.stamp(a).unwrap(), before, "refused: nothing stays");
    assert_eq!(doc.journal().count(), 1);
}

#[test]
fn a_kept_step_is_one_undo_step_with_its_row_and_undo_marks_it() {
    let mut doc = empty();
    let a = doc.add(point("a", 1.0)).unwrap();
    let (_, report) = doc
        .run_step(
            |doc| -> Result<(), ()> {
                let next = moved(doc, a, 3.0);
                doc.update(a, next);
                doc.add(point("b", 4.0)).map_err(|_| ())?;
                let mut s = doc.settings().clone();
                s.plot_scale = 250.0;
                doc.set_settings(s);
                Ok(())
            },
            |_, report, _| {
                assert_eq!(report.parts, vec![ProjectPart::Settings]);
                Ok(Some(Sealed {
                    row: row("cad.test"),
                    request: Some(RequestRecord {
                        request_id: "r-1".into(),
                        fingerprint: "f".repeat(32),
                        command: "cad.test".into(),
                        version: 1,
                        revision: String::new(),
                        status: CallStatus::Done,
                        answer: None,
                    }),
                }))
            },
        )
        .expect("kept");
    assert_eq!(report.label.as_deref(), Some("Değiştir"));
    let step = report.step.expect("an undo step");
    let call = report.call.expect("a row");
    let rows: Vec<_> = doc.journal().cloned().collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].kind,
        CallKind::Edit,
        "the first add, made without a command"
    );
    assert_eq!(rows[1].seq, call);
    assert_eq!(rows[1].step, Some(step));
    assert_eq!(rows[1].revision, doc.revision().to_string());
    let request = doc.request("r-1").expect("in the ledger");
    assert_eq!(request.revision, doc.revision().to_string());
    doc.set_request_answer(call, "{\"status\":\"completed\"}".into());
    assert_eq!(
        doc.request("r-1").and_then(|r| r.answer.as_deref()),
        Some("{\"status\":\"completed\"}")
    );
    // One undo takes the whole step back (the settings are no undo step).
    assert_eq!(doc.undo().as_deref(), Some("Değiştir"));
    assert_eq!(doc.len(), 1);
    assert_eq!(
        doc.journal_row(call).map(|r| r.status),
        Some(CallStatus::Undone)
    );
    assert_eq!(
        doc.request("r-1").map(|r| r.status),
        Some(CallStatus::Undone)
    );
    assert_eq!(doc.settings().plot_scale, 250.0);
    doc.redo();
    assert_eq!(
        doc.journal_row(call).map(|r| r.status),
        Some(CallStatus::Done)
    );
}

#[test]
fn an_inner_step_is_a_savepoint_of_the_outer() {
    let mut doc = empty();
    let a = doc.add(point("a", 1.0)).unwrap();
    let (_, outer) = doc
        .run_step(
            |doc| -> Result<(), ()> {
                let next = moved(doc, a, 2.0);
                doc.update(a, next);
                // An inner step that fails takes back only its own writes, its name too.
                let inner: Result<((), _), ()> = doc.run_step(
                    |doc| {
                        doc.set_name("İç");
                        doc.add(point("c", 9.0)).map_err(|_| ())?;
                        Err(())
                    },
                    |_, _, _| Ok(None),
                );
                assert!(inner.is_err());
                assert_eq!(doc.name(), "Deneme");
                // One that succeeds leaves its writes to the outer.
                let (_, inner) = doc.run_step(
                    |doc| -> Result<(), ()> {
                        doc.set_name("Dış");
                        Ok(())
                    },
                    |_, report, _| {
                        assert!(!report.outermost);
                        Ok(None)
                    },
                )?;
                assert_eq!(inner.step, None, "a savepoint is recorded by the outer");
                Ok(())
            },
            |_, report, _| {
                assert_eq!(report.objects.len(), 1, "the inner add was taken back");
                assert_eq!(report.parts, vec![ProjectPart::Name]);
                Ok(None)
            },
        )
        .expect("kept");
    assert!(outer.step.is_some());
    assert_eq!(doc.name(), "Dış");
    // And refusing the outer now would take the inner's name back too.
    let result: Result<((), _), ()> = doc.run_step(
        |doc| {
            let (_, _) = doc.run_step(
                |doc| -> Result<(), ()> {
                    doc.set_name("Üçüncü");
                    Ok(())
                },
                |_, _, _| Ok(None),
            )?;
            Err(())
        },
        |_, _, _| Ok(None),
    );
    assert!(result.is_err());
    assert_eq!(doc.name(), "Dış");
}

#[test]
fn a_step_inside_a_group_waits_for_the_group() {
    let mut doc = empty();
    let group = doc.begin_group("Betik");
    let (_, report) = doc
        .run_step(
            |doc| doc.add(point("a", 1.0)).map(|_| ()),
            |_, _, _| {
                Ok(Some(Sealed {
                    row: row("cad.test"),
                    request: None,
                }))
            },
        )
        .expect("kept");
    assert_eq!(report.step, None, "the group makes the step");
    let call = report.call.expect("queued");
    assert!(doc.journal_row(call).is_some(), "waiting, yet found");
    assert_eq!(doc.journal().count(), 0);
    doc.end_group(group);
    assert_eq!(doc.journal().count(), 1);
    let recorded = doc.journal_row(call).expect("recorded");
    assert!(recorded.step.is_some());
    // A cancelled group leaves no row.
    let group = doc.begin_group("İptal");
    let _ = doc
        .run_step(
            |doc| doc.add(point("a", 2.0)).map(|_| ()),
            |_, _, _| {
                Ok(Some(Sealed {
                    row: row("cad.test"),
                    request: None,
                }))
            },
        )
        .expect("kept");
    doc.cancel_group(group);
    assert_eq!(doc.journal().count(), 1);
    assert_eq!(doc.len(), 1);
}

#[test]
fn a_step_writing_only_parts_is_journaled_without_an_undo_step() {
    let mut doc = empty();
    let (_, report) = doc
        .run_step(
            |doc| -> Result<(), ()> {
                doc.set_layer_visible("c", false);
                Ok(())
            },
            |_, report, _| {
                assert_eq!(report.parts, vec![ProjectPart::LayerFlags]);
                Ok(Some(Sealed {
                    row: row("cad.layers.set"),
                    request: None,
                }))
            },
        )
        .expect("kept");
    assert_eq!(report.step, None);
    assert!(!doc.can_undo());
    let rows: Vec<_> = doc.journal().cloned().collect();
    assert_eq!(rows.len(), 1, "no edit row beside the command's");
    assert_eq!(rows[0].command.as_deref(), Some("cad.layers.set"));
    assert_eq!(rows[0].step, None);
}

#[test]
fn parts_written_outside_a_command_are_edit_rows_joined_while_they_follow() {
    let mut doc = empty();
    doc.set_layer_visible("a", false);
    doc.set_layer_visible("b", false);
    doc.set_name("Yeni ad");
    let rows: Vec<_> = doc.journal().cloned().collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].parts, vec![ProjectPart::LayerFlags]);
    assert_eq!(
        (rows[0].base.as_str(), rows[0].revision.as_str()),
        ("0", "2")
    );
    assert_eq!(rows[1].label.as_deref(), Some("Çizimin adı"));
}

#[test]
fn stamps_come_from_the_content_and_come_back_with_undo() {
    let mut doc = empty();
    let a = doc.add(point("a", 1.0)).unwrap();
    let (stamp, layers, settings, styles, sheets) = (
        doc.stamp(a).unwrap(),
        doc.layers_stamp(),
        doc.settings_stamp(),
        doc.styles_stamp(),
        doc.sheets_stamp(),
    );
    assert_eq!(stamp.len(), 16);
    let next = moved(&doc, a, 7.0);
    doc.update(a, next);
    assert_ne!(doc.stamp(a).unwrap(), stamp);
    doc.undo();
    assert_eq!(
        doc.stamp(a).unwrap(),
        stamp,
        "the same state, the same stamp"
    );
    // Across a save and an opening.
    let bytes = kentos_kcad_like_roundtrip(&doc);
    assert_eq!(bytes.stamp(a).unwrap(), stamp);
    assert_eq!(bytes.layers_stamp(), layers);
    assert_eq!(bytes.settings_stamp(), settings);
    assert_eq!(bytes.styles_stamp(), styles);
    assert_eq!(bytes.sheets_stamp(), sheets);
}

/// The drawing written as a v2 snapshot and read back (the file's content).
fn kentos_kcad_like_roundtrip(doc: &Document) -> Document {
    Document::from_snapshot_v2(doc.to_snapshot_v2()).expect("reads back")
}

#[test]
fn integrity_names_what_a_step_would_leave_broken() {
    let mut doc = empty();
    let a = doc.add(point("a", 1.0)).unwrap();
    let uid = doc.uid(a).unwrap();
    let text = Entity::Text(TextEntity {
        label_of: Some(kentos_domain::contracts::EntityId(uuid_bytes())),
        label_scale: Some(1000.0),
        ..match point_text() {
            Entity::Text(t) => t,
            _ => unreachable!(),
        }
    });
    let mut faults = Vec::new();
    let _ = doc.run_step(
        |doc| -> Result<(), ()> {
            doc.add(point("nowhere", 2.0)).map_err(|_| ())?;
            doc.add(point("g", 3.0)).map_err(|_| ())?;
            doc.add(text).map_err(|_| ())?;
            Ok(())
        },
        |doc, report, _| {
            faults = doc.faults(report);
            Err(())
        },
    );
    assert_eq!(faults.len(), 3, "{faults:?}");
    // An untouched object keeps whatever it had.
    assert!(doc.uid(a) == Some(uid));
}

fn uuid_bytes() -> [u8; 16] {
    *uuid::Uuid::now_v7().as_bytes()
}

fn point_text() -> Entity {
    serde_json::from_value(serde_json::json!({
        "kind": "text", "id": 0, "layerId": "a", "attrs": {},
        "p": { "x": 0.0, "y": 0.0 }, "text": "P1", "height": 2.0, "rotation": 0.0
    }))
    .expect("a text")
}

#[test]
fn the_ledger_goes_with_the_file() {
    let mut doc = empty();
    let (_, report) = doc
        .run_step(
            |doc| doc.add(point("a", 1.0)).map(|_| ()),
            |_, _, _| {
                Ok(Some(Sealed {
                    row: row("cad.point.create"),
                    request: Some(RequestRecord {
                        request_id: "ajan-1".into(),
                        fingerprint: "0123456789abcdef0123456789abcdef".into(),
                        command: "cad.point.create".into(),
                        version: 1,
                        revision: String::new(),
                        status: CallStatus::Done,
                        answer: None,
                    }),
                }))
            },
        )
        .expect("kept");
    doc.set_request_answer(report.call.unwrap(), "{}".into());
    doc.undo();
    let back = Document::from_snapshot_v2(doc.to_snapshot_v2()).expect("reads back");
    let request = back.request("ajan-1").expect("kept in the file");
    assert_eq!(request.status, CallStatus::Undone);
    assert_eq!(request.answer.as_deref(), Some("{}"));
    assert_eq!(request.revision, "1");
    assert_eq!(back.journal().count(), 0, "the journal is the session's");
}

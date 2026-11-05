//! Review notes and taking revision differences (docs/adr/1000) where the shared cases cannot reach: a block
//! definition's change (an anchor `blockChanged`, a taken insert's `block_definition_differs`, a missing definition's
//! `unknown_block`), the note and reply limits, and the notes' save–open round trip with their undo.

use kentos_commands::{
    CommandResult, CompareAgainst, DrawingTake, MAX_REVIEW_NOTES, MAX_REVIEW_REPLIES, ReviewAnchorState, ReviewNote,
    ReviewNoteOp, ReviewNotesEdit, ReviewNotesList, ReviewStatus,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{ExecutionContext, drawing_take, review_notes};
use serde_json::json;

const INSERT: &str = "0192f5b0-3000-7000-8000-000000000001";
const LINE: &str = "0192f5b0-3000-7000-8000-000000000002";
const BLOCK: &str = "0192f5b0-3000-7000-8000-0000000000b1";
const NOTE: &str = "0192f5b0-3000-7000-8000-0000000000aa";

/// A drawing with a definition “Kapı” (a line), an insert of it and a line.
fn drawing() -> Document {
    let style = json!({"color": "fg", "lineType": "continuous", "lineWeight": 0.25});
    let snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "Blok notu",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 100.0},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [{"id": "a", "name": "A", "type": "layer", "visible": true, "locked": false, "expanded": true, "style": style, "children": []}],
        "activeLayer": "a",
        "blocks": [{"id": BLOCK, "name": "Kapı", "base": {"x": 0.0, "y": 0.0},
                    "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 0.9, "y": 0.0}}]}],
        "entities": [
            {"kind": "insert", "id": 1, "layerId": "a", "attrs": {}, "block": BLOCK, "p": {"x": 10.0, "y": 10.0}, "scale": 1.0, "rotation": 0.0},
            {"kind": "line", "id": 2, "layerId": "a", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 5.0, "y": 0.0}}
        ],
        "uids": [INSERT, LINE],
        "styles": {"items": [], "categories": []}
    });
    Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap()
}

fn written(doc: &Document, name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kentos-review-take-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap()).unwrap();
    path.to_string_lossy().into_owned()
}

fn edit(doc: &mut Document, op: ReviewNoteOp) -> CommandResult<kentos_commands::ReviewNotesEdited> {
    review_notes::execute(&mut ExecutionContext::new(doc), ReviewNotesEdit { change: op, expected_revision: None })
}

fn add(id: &str, anchors: Vec<String>) -> ReviewNoteOp {
    ReviewNoteOp::Add {
        id: Some(id.into()),
        text: "Kapı açılış yönünü kontrol edin.".into(),
        anchors,
        at: None,
        sheet: None,
        view: None,
        revision: Some("B".into()),
        origin: None,
        author: Some("Ayşe".into()),
        created_at: Some("2026-11-05T21:00:00+03:00".into()),
    }
}

fn state(doc: &Document) -> ReviewAnchorState {
    review_notes::list(doc, &ReviewNotesList::default()).notes[0].anchor_states[0].state
}

fn changed_definition(doc: &Document) -> kentos_commands::BlockDefinition {
    let id = doc.blocks()[0].id;
    let mut def = doc.block(id).unwrap().clone();
    if let kentos_commands::Entity::Line(l) = &mut def.entities[0] {
        l.b.x = 1.0;
    }
    def
}

#[test]
fn a_changed_definition_makes_the_insert_note_stale_and_the_take_warns() {
    let mut doc = drawing();
    let old = written(&doc, "eski.kcad");
    assert!(matches!(edit(&mut doc, add(NOTE, vec![INSERT.into()])), CommandResult::Completed { .. }));
    assert_eq!(state(&doc), ReviewAnchorState::Current);
    let def = changed_definition(&doc);
    assert!(doc.update_block(def).unwrap());
    assert_eq!(state(&doc), ReviewAnchorState::BlockChanged, "the insert is as it was, its definition is not");
    assert!(review_notes::list(&doc, &ReviewNotesList::default()).notes[0].stale);

    // The insert moved too: taking it back from the old file replaces it and warns that the definition stays.
    let slot = doc.slot_of(Uuid::parse_str(INSERT).unwrap()).unwrap();
    let mut moved = doc.get(slot).unwrap().clone();
    if let kentos_commands::Entity::Insert(i) = &mut moved {
        i.p.x += 3.0;
    }
    assert!(doc.update(slot, moved.clone()));
    let input = DrawingTake {
        against: CompareAgainst::File { path: old.clone() },
        uids: vec![INSERT.into()],
        base: None,
        overwrite: false,
        expected_revision: None,
    };
    let CommandResult::Completed { output, warnings } = drawing_take::execute(&mut ExecutionContext::new(&mut doc), input) else {
        panic!("the take")
    };
    assert_eq!(output.replaced, [INSERT]);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "block_definition_differs");
    let kentos_commands::Entity::Insert(i) = doc.get(slot).unwrap() else { panic!("insert") };
    assert_eq!(i.p.x, 10.0);
    // The definition is the drawing's, changed: not taken.
    let kentos_commands::Entity::Line(l) = &doc.blocks()[0].entities[0] else { panic!("line") };
    assert_eq!(l.b.x, 1.0);
    assert_eq!(doc.undo().as_deref(), Some("Farkları al"));
    assert_eq!(doc.get(slot), Some(&moved));
}

#[test]
fn an_insert_whose_definition_the_drawing_lost_is_not_taken() {
    let mut doc = drawing();
    let old = written(&doc, "tanimli.kcad");
    let slot = doc.slot_of(Uuid::parse_str(INSERT).unwrap()).unwrap();
    doc.remove(&[slot]);
    let id = doc.blocks()[0].id;
    assert!(doc.remove_block(id).unwrap());
    let revision = doc.revision();
    let input = DrawingTake {
        against: CompareAgainst::File { path: old },
        uids: vec![LINE.into(), INSERT.into()],
        base: None,
        overwrite: false,
        expected_revision: None,
    };
    let CommandResult::Failed { error } = drawing_take::execute(&mut ExecutionContext::new(&mut doc), input) else {
        panic!("refused")
    };
    assert_eq!(error.code, "unknown_block");
    assert_eq!(error.path.as_deref(), Some("uids[1]"));
    assert_eq!(doc.revision(), revision);
}

#[test]
fn replies_and_notes_stop_at_their_limits() {
    let mut doc = drawing();
    assert!(matches!(edit(&mut doc, add(NOTE, vec![])), CommandResult::Completed { .. }));
    for i in 0..MAX_REVIEW_REPLIES {
        let reply = ReviewNoteOp::Reply { id: NOTE.into(), text: format!("Yanıt {i}"), author: None, created_at: None, status: None };
        assert!(matches!(edit(&mut doc, reply), CommandResult::Completed { .. }), "reply {i}");
    }
    let one_more = ReviewNoteOp::Reply { id: NOTE.into(), text: "Bir fazla".into(), author: None, created_at: None, status: None };
    let CommandResult::Failed { error } = edit(&mut doc, one_more) else { panic!("refused") };
    assert_eq!(error.code, "too_many_review_replies");

    let notes: Vec<ReviewNote> = (0..MAX_REVIEW_NOTES)
        .map(|n| ReviewNote {
            id: Uuid::from_u128(0x0192f5b0_4000_7000_8000_000000000000 + n as u128).to_string(),
            number: n as u32 + 1,
            text: "Not".into(),
            status: ReviewStatus::Open,
            origin: Default::default(),
            author: None,
            created_at: None,
            revision: None,
            at: None,
            sheet: None,
            view: None,
            anchors: vec![],
            replies: vec![],
        })
        .collect();
    doc.set_review_notes(notes, "Notlar");
    let CommandResult::Failed { error } = edit(&mut doc, add("0192f5b0-3000-7000-8000-0000000000ab", vec![])) else {
        panic!("refused")
    };
    assert_eq!(error.code, "too_many_review_notes");
}

#[test]
fn notes_survive_a_save_and_an_opening_and_undo_takes_them_back() {
    let mut doc = drawing();
    assert!(matches!(edit(&mut doc, add(NOTE, vec![INSERT.into(), LINE.into()])), CommandResult::Completed { .. }));
    let reply = ReviewNoteOp::Reply {
        id: NOTE.into(),
        text: "Yön düzeltildi.\nKontrol edin.".into(),
        author: Some("Cihad".into()),
        created_at: None,
        status: Some(ReviewStatus::Answered),
    };
    assert!(matches!(edit(&mut doc, reply), CommandResult::Completed { .. }));
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
    let opened = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert_eq!(opened.settings().review_notes, doc.settings().review_notes);
    let listed = review_notes::list(&opened, &ReviewNotesList::default());
    assert_eq!(listed.answered, 1);
    assert!(!listed.notes[0].stale, "the stamps are the content's: the same after an opening");
    assert_eq!(doc.undo().as_deref(), Some("İnceleme notu"));
    assert_eq!(doc.settings().review_notes[0].status, ReviewStatus::Open);
    assert_eq!(doc.undo().as_deref(), Some("İnceleme notu"));
    assert!(doc.settings().review_notes.is_empty());
    assert_eq!(doc.redo().as_deref(), Some("İnceleme notu"));
    assert_eq!(doc.settings().review_notes.len(), 1);
    // A plan writes nothing.
    let before = doc.revision();
    let plan = review_notes::plan(
        &ExecutionContext::new(&mut doc),
        &ReviewNotesEdit { change: ReviewNoteOp::Remove { id: NOTE.into() }, expected_revision: None },
    );
    assert!(matches!(plan, CommandResult::Completed { .. }));
    assert_eq!(doc.revision(), before);
    let _ = Slot(0);
}

/// docs/adr/1001 §3: a 16-element pattern stays in the drawing and in a save–open (schema 181), and a LIN export
/// leaves it out with `pattern_too_long_for_lin` while the 12-element one is written.
#[test]
fn a_long_pattern_is_kept_and_left_out_of_a_lin_file() {
    use kentos_commands::{LineTypeDef, LineTypesEdit, LineTypesOperation, PatternElement, PatternUnit};
    let mut doc = drawing();
    let pattern = |name: &str, n: usize| LineTypeDef {
        name: name.into(),
        description: String::new(),
        elements: (0..n)
            .map(|i| PatternElement { length: if i % 2 == 0 { 2.0 } else { -1.0 }, text: None, shape: None })
            .collect(),
        unit: PatternUnit::Drawing,
        source: Some("DXF".into()),
    };
    let mut settings = doc.settings().clone();
    settings.line_types = vec![pattern("UZUN16", 16), pattern("KISA12", 12)];
    doc.set_settings(settings);
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap();
    let back = kentos_kcad::decode(&bytes).unwrap();
    assert_eq!(back.settings.line_types[0].elements.len(), 16);
    let input: LineTypesEdit =
        serde_json::from_value(serde_json::json!({ "operation": "export" })).expect("an export");
    assert!(matches!(input.operation, LineTypesOperation::Export));
    let CommandResult::Completed { output, warnings } =
        kentos_native_application::line_types_edit::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!("the export")
    };
    assert_eq!(output.skipped, ["UZUN16"]);
    let lin = output.lin.expect("a LIN text");
    assert!(lin.contains("*KISA12") && !lin.contains("UZUN16"), "{lin}");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "pattern_too_long_for_lin");
}

/// docs/adr/1001 §4: opening a metre DXF keeps a drawing-unit pattern in metres (a 1 m dash stays 1, not 1000), and a
/// 300-element pattern comes cut to 256 with the report's note.
#[test]
fn an_opened_metre_dxf_keeps_its_pattern_lengths() {
    let dash = |n: usize| -> String {
        let mut out = format!("0\nLTYPE\n2\nU{n}\n70\n0\n3\n\n72\n65\n73\n{n}\n40\n{}\n", n as f64);
        for i in 0..n {
            out.push_str(if i % 2 == 0 { "49\n1.0\n74\n0\n" } else { "49\n-0.5\n74\n0\n" });
        }
        out
    };
    let text = format!(
        "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLTYPE\n{}{}0\nENDTAB\n0\nTABLE\n2\nLAYER\n0\nLAYER\n2\nA\n70\n0\n62\n1\n6\nU16\n0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nA\n10\n0\n20\n0\n30\n0\n11\n60\n21\n0\n31\n0\n0\nENDSEC\n0\nEOF\n",
        dash(16),
        dash(300)
    );
    let dir = std::env::temp_dir().join(format!("kentos-open-pattern-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("metre.dxf");
    std::fs::write(&path, &text).unwrap();
    let format = kentos_native_application::cad_open::Format::of(&path).unwrap();
    let mut watch = kentos_formats::watch::Steps(|_, _| true);
    let opened =
        kentos_native_application::cad_open::read(&path, text.as_bytes(), format, &mut watch).expect("opens");
    let s = opened.document.to_snapshot_v2();
    let u16 = s.settings.line_types.iter().find(|t| t.name == "U16").expect("the 16-element type");
    assert_eq!(u16.elements.len(), 16);
    assert_eq!((u16.elements[0].length, u16.elements[1].length), (1.0, -0.5), "metres, as the drawing's unit");
    let u300 = s.settings.line_types.iter().find(|t| t.name == "U300").expect("the long type");
    assert_eq!(u300.elements.len(), kentos_commands::MAX_STORED_PATTERN_ELEMENTS);
    assert!(opened.report.notes.iter().any(|n| n.what == "Çizgi tipi deseni" && n.reason.contains("U300")));
    let layer = s.layers.iter().find(|l| l.name == "A").expect("the layer");
    assert_eq!(layer.style.line_pattern.as_deref(), Some("U16"));
}

/// docs/adr/1002: an install refuses without a bound standard (`no_standard`) and with one it cannot read
/// (`invalid_standard`), writing nothing.
#[test]
fn installing_needs_a_readable_bound_standard() {
    use kentos_commands::{ProjectStandardDocument, StandardsInstall};
    let mut doc = drawing();
    let refused = |doc: &mut Document| match kentos_native_application::standards_install::execute(
        &mut ExecutionContext::new(doc),
        StandardsInstall::default(),
    ) {
        CommandResult::Failed { error } => error.code,
        other => panic!("refused: {other:?}"),
    };
    assert_eq!(refused(&mut doc), "no_standard");
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![ProjectStandardDocument {
        text: serde_json::json!({"format": "kentos.drawing-standard", "version": 2, "id": "bozuk", "revision": "1",
            "standard": {"name": "Bozuk", "layers": [], "bilinmeyen": true}, "resources": {}})
        .to_string(),
    }];
    doc.set_settings(settings);
    let revision = doc.revision();
    assert_eq!(refused(&mut doc), "invalid_standard");
    assert_eq!(doc.revision(), revision);
}

/// docs/adr/1003 §1: a journal that makes an associative array and edits it replays on another drawing: the row
/// that made it records the array's id, the replay maps it to the one it made, and the edit lands on that array.
#[test]
fn a_replayed_journal_maps_an_associative_arrays_id() {
    use kentos_native_application::registry;
    use kentos_native_application::replay::{journal_of, replay};
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.arrays.edit.json")).unwrap();
    let setup = kentos_commands::DocumentSnapshotV1::from_json(&cases["setup"].to_string()).unwrap();
    let mut source = Document::from_snapshot(setup.clone()).unwrap();
    let mut target = Document::from_snapshot(setup).unwrap();
    let circle = source.uid(Slot(1)).unwrap().to_string();
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| {
        let answer =
            registry::run(&mut ExecutionContext::new(doc), name, kentos_commands::CallOp::Execute, input).expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer
    };
    let made = run(
        &mut source,
        "cad.entities.array",
        serde_json::json!({"uids": [circle], "layout": {"kind": "grid", "rows": 2, "cols": 3, "dx": 5, "dy": 4}, "associative": true}),
    );
    let array = made["output"]["array"].as_str().expect("an array id").to_owned();
    run(
        &mut source,
        "cad.arrays.edit",
        serde_json::json!({"array": array, "edit": {"kind": "layout", "layout": {"kind": "grid", "rows": 3, "cols": 4, "dx": 5, "dy": 4}}}),
    );
    let journal = journal_of(&source);
    let row = journal.calls.iter().find(|r| r.command.as_deref() == Some("cad.entities.array")).unwrap();
    assert_eq!(row.arrays.added, vec![array.clone()]);
    let encoded = serde_json::to_value(&journal).unwrap();
    let decoded: kentos_commands::ReplayJournal = serde_json::from_value(encoded).unwrap();
    let report = replay(&mut target, None, &decoded);
    assert_eq!((report.failed, report.differs), (0, 0), "{report:?}");
    let mapped = report.mapped.get(&array).expect("the array's id is mapped").clone();
    assert_ne!(mapped, array, "the replay made an array of its own");
    let arrays = target.arrays();
    assert_eq!(arrays.len(), 1);
    assert_eq!(arrays[0].id.to_string(), mapped);
    assert_eq!(arrays[0].items.len(), 12, "the edit to 3×4 landed on the replay's array");
    assert_eq!(target.len(), source.len());
}

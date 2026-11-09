//! Harita işi's objects changed since an approval (map_work_panel.rs, plan/map-cad.md MAP-01, docs/adr/2880): the
//! design approved from the panel keeps each of its objects' digests; a line changed, a line drawn on its layer and a
//! line deleted afterwards are counted under its row, whose press selects the changed and the added; Yeniden onayla
//! takes the count back to none in one undo step; the digests are saved in the .kcad file (schema 210; docs/adr/3280
//! with their parts, schema 211) and the count is the same opened again. docs/adr/3280: the changed line's part is its
//! attributes, said in the row's tip. And the pictures.

use kentos_commands::{WorkChangedObject, WorkChangedParts, WorkChanges, WorkState as S, WorkStatus as W};
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::json;

use crate::app::{App, Message, Panel};
use crate::map_work_panel::Event as P;
use crate::map_work_stamps_tests::{change_line, opened, send, tree, uid};

fn run(app: &mut App, command: &str, input: serde_json::Value) -> serde_json::Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        command,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    answer
}

/// The design (is-4) approved, then a line of it changed (slot 6), a line drawn on its layer and its other line
/// (slot 7) deleted: the slot of the new line.
fn approved_then_changed(app: &mut App) -> Slot {
    send(app, P::Status("is-4".into(), W::Approved));
    change_line(app, 6, "1060");
    let made = run(
        app,
        "cad.entities.create",
        json!({"layerId": "tasarim", "objects": [{"geometry": {
            "kind": "line", "a": {"x": 500000.0, "y": 4400015.0}, "b": {"x": 500010.0, "y": 4400015.0}}}]}),
    );
    let new = made["output"]["created"][0].as_str().expect("its id").to_owned();
    let gone = uid(app, 7);
    run(app, "cad.entities.delete", json!({"uids": [gone]}));
    let doc = app.document.as_ref().expect("a drawing");
    doc.model.slot_of(kentos_domain::Uuid::parse_str(&new).expect("a uid")).expect("drawn")
}

fn said(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

#[test]
fn an_approved_design_counts_its_changed_objects_and_selects_them() {
    let mut app = opened();
    send(&mut app, P::Status("is-4".into(), W::Approved));
    let kept = app.document.as_ref().expect("a drawing").model.map_work().expect("a map project").items[3]
        .approved_objects
        .clone();
    assert_eq!(kept.len(), 2, "the stamp's two lines, each with its digest");
    assert_eq!(tree(&app).items[3].changes, None, "approved and unchanged: nothing to say");
    send(&mut app, P::Status("is-4".into(), W::Draft));
    let new = approved_then_changed(&mut app);
    let t = tree(&app);
    assert_eq!(t.items[3].state, S::Revised);
    let mut listed = vec![uid(&app, 6), uid(&app, new.0)];
    listed.sort();
    // Docs/adr/3280: the changed line's height attribute: its attributes part, and only it.
    let parts = Some(WorkChangedParts { attributes: 1, ..WorkChangedParts::default() });
    let objects = vec![WorkChangedObject { uid: uid(&app, 6), parts: vec!["attributes".into()] }];
    let changes = WorkChanges { added: 1, removed: 1, changed: 1, uids: listed, parts, objects };
    assert_eq!(t.items[3].changes.as_ref(), Some(&changes));
    assert_eq!(
        crate::map_work_panel::changes_words(&changes),
        "Onaydan sonra 1 nesne değişti (öznitelik 1), 1 nesne eklendi, 1 nesne silindi."
    );
    // The row's press: the changed line and the new one selected; the deleted one is said to be gone.
    app.selection.set(vec![Slot(8)]);
    send(&mut app, P::Changes("is-4".into()));
    let mut selected: Vec<Slot> = app.selection.ids().to_vec();
    selected.sort_by_key(|s| s.0);
    assert_eq!(selected, [Slot(6), new]);
    assert_eq!(
        said(&app).last().map(String::as_str),
        Some("Onaydan sonra değişen ya da eklenen 2 nesne seçildi; değişen: 1 nesnenin öznitelikleri; silinen 1 nesne çizimde yok.")
    );
    // Yeniden onayla: today's objects kept, nothing changed since; one undo step back to the count.
    send(&mut app, P::Status("is-4".into(), W::Approved));
    let t = tree(&app);
    assert_eq!((t.items[3].state, t.items[3].changes.as_ref()), (S::Current, None));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(tree(&app).items[3].changes.as_ref().map(|c| (c.added, c.removed, c.changed)), Some((1, 1, 1)));
    // Back to draft: the digests go with the stamp.
    send(&mut app, P::Status("is-4".into(), W::Draft));
    let item = &app.document.as_ref().expect("a drawing").model.map_work().expect("a map project").items[3];
    assert!(item.approved.is_none() && item.approved_objects.is_empty() && item.approved_parts.is_empty());
}

#[test]
fn the_digests_are_saved_and_the_count_is_the_same_opened_again() {
    let mut app = opened();
    approved_then_changed(&mut app);
    let before = tree(&app);
    let model = &app.document.as_ref().expect("a drawing").model;
    let bytes = kentos_kcad::encode(&model.to_snapshot_v2()).expect("encodes");
    assert!(bytes.windows(10).any(|w| w == b"\x67version\x18\xd3"), "the file is schema 211 (docs/adr/3280)");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(back.settings.map_work, model.map_work().cloned(), "the digests are saved whole");
    let (mut again, _) = App::boot(None);
    again.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(back, None).expect("opens");
    let _ = again.update(Message::Opened(Some(Ok(Box::new(doc)))));
    assert_eq!(tree(&again).items[3].changes, before.items[3].changes);
}

/// The panel: the approved design whose line changed, a line added and a line deleted, Değişenler under its row;
/// then the row pressed and the two lines selected (the orthophoto hidden, which lies over them); light wide and dark
/// narrow (docs/adr/3280: the status bar names the changed part),
/// `.run/gui-evidence/onaydan-sonra-parcalar-3280/` (before it `onaydan-sonra-2880`):
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_work_changes_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/onaydan-sonra-parcalar-3280");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapWork));
        // The tree's room: Harita işi taller than Öznitelikler, the campaign's branch folded.
        app.docks.update(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.68, 0.32]));
        send(&mut app, P::Fold("kind:campaign".into()));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        approved_then_changed(&mut app);
        run(&mut app, "cad.layers.set", json!({"operation": "visible", "layer": "ortofoto", "value": false}));
        app.selection.set(Vec::new());
        // The edits went straight to the drawing: fitted again, the scene draws them.
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "degisenler");
        send(&mut app, P::Changes("is-4".into()));
        shot(&mut app, "secildi");
    }
}

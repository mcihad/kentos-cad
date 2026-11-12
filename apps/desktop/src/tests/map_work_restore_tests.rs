//! Onaylı hâline getir in Harita işi (map_work_panel.rs, plan/map-cad.md MAP-01, docs/adr/4080): the design approved
//! from the panel, then a line of it changed, a line drawn on its layer and its other line deleted; the Değişenler row's
//! button puts the deleted line back with its persistent id and the changed one back to its approved value in one undo
//! step, the drawn line stays; the status bar says so; the row is then only the added line. And the pictures.

use kentos_commands::WorkStatus as W;
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

/// The design (is-4) approved; its line at slot 6 given a height, a line drawn on its layer, its line at slot 7
/// deleted. The deleted line's id and its value as approved, the changed line's id.
fn approved_then_changed(app: &mut App) -> (String, serde_json::Value, String) {
    send(app, P::Status("is-4".into(), W::Approved));
    let (changed, gone) = (uid(app, 6), uid(app, 7));
    let doc = app.document.as_ref().expect("a drawing");
    let mut then = serde_json::to_value(doc.model.get(kentos_domain::Slot(7)).expect("the line")).expect("JSON");
    then.as_object_mut().expect("an object").remove("id");
    change_line(app, 6, "1060");
    run(
        app,
        "cad.entities.create",
        json!({"layerId": "tasarim", "objects": [{"geometry": {
            "kind": "line", "a": {"x": 500000.0, "y": 4400015.0}, "b": {"x": 500010.0, "y": 4400015.0}}}]}),
    );
    run(app, "cad.entities.delete", json!({"uids": [gone]}));
    (gone, then, changed)
}

#[test]
fn onayli_haline_getir_puts_back_the_deleted_and_the_changed_line() {
    let mut app = opened();
    let (gone, then, changed) = approved_then_changed(&mut app);
    let c = tree(&app).items[3].changes.clone().expect("changes");
    assert_eq!((c.changed, c.added, c.removed), (1, 1, 1));
    send(&mut app, P::Restore("is-4".into()));
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|l| l == "Onaylı hâline getirildi: 1 silinen nesne aynı kimlikle geri geldi, 1 değişen nesne onaylanan değerine döndü. Onaydan sonra eklenen 1 nesne yerinde kaldı."),
        "{said:?}"
    );
    let doc = app.document.as_ref().expect("a drawing");
    let back = doc.model.slot_of(kentos_domain::Uuid::parse_str(&gone).expect("a uid")).expect("made again with its id");
    let mut now = serde_json::to_value(doc.model.get(back).expect("the line")).expect("JSON");
    now.as_object_mut().expect("an object").remove("id");
    assert_eq!(now, then, "the deleted line as approved");
    let line = doc.model.get(doc.model.slot_of(kentos_domain::Uuid::parse_str(&changed).expect("a uid")).expect("there")).expect("the line");
    assert!(!serde_json::to_value(line).expect("JSON")["attrs"].as_object().is_some_and(|a| a.contains_key("Kot")), "its height gone again");
    let c = tree(&app).items[3].changes.clone().expect("the added line is a change still");
    assert_eq!((c.changed, c.added, c.removed), (0, 1, 0));
    // One undo takes it back.
    let _ = app.update(Message::Run("edit.undo"));
    let c = tree(&app).items[3].changes.clone().expect("changes");
    assert_eq!((c.changed, c.added, c.removed), (1, 1, 1));
}

/// `.run/gui-evidence/onayli-haline-4080/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_work_restore_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/onayli-haline-4080");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapWork));
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
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "1-degisenler");
        send(&mut app, P::Restore("is-4".into()));
        shot(&mut app, "2-onayli-haline");
    }
}

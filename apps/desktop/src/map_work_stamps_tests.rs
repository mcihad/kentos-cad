//! Harita işi's approval stamps and designs as sources (map_work_panel.rs, plan/map-cad.md MAP-01, docs/adr/2080): the
//! design approved from the panel keeps its content revision as its stamp; a line of it changed afterwards turns the
//! row to “Onay eskidi”, whose Yeniden onayla stamps today's revision in one undo step; the user's own line is
//! recorded as made from the design (Seçimi türet offers it, never for the design's own lines) and goes out of date
//! when the design changes. And the pictures.

use kentos_commands::{WorkState as S, WorkStatus as W};
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message, Panel};
use crate::map_work_panel::Event as P;

fn fixtures() -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .canonicalize()
        .expect("the fixtures")
        .to_string_lossy()
        .into_owned()
}

/// `map.work.query`'s drawing (map_work_cases.py): the campaign, the orthophotos, the CAD reference, the design surface
/// (its two lines in slots 6 and 7) and the user's own line (slot 8).
fn opened() -> App {
    let text = include_str!("../../../fixtures/commands/v1/map.work.query.json")
        .replace("$fixture:", &format!("{}/", fixtures()));
    let cases: Value = serde_json::from_str(&text).expect("cases");
    let snapshot = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::MapWork(e));
}

fn tree(app: &App) -> kentos_commands::MapWorkTree {
    app.map_work_read().expect("a drawing").expect("a map project")
}

fn uid(app: &App, slot: u32) -> String {
    app.document.as_ref().expect("a drawing").model.uid(Slot(slot)).expect("a lasting id").to_string()
}

/// One of the design's lines given a height attribute: its content moves on.
fn change_line(app: &mut App, slot: u32, height: &str) {
    let input = json!({"uids": [uid(app, slot)], "attrs": {"Kot": height}, "operation": "attributes"});
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.entities.set",
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
}

/// Öznitelikler's Kaynağı rows of the object in `slot`, selected alone.
fn source_rows(app: &mut App, slot: u32) -> Vec<(String, String)> {
    app.selection.set(vec![Slot(slot)]);
    let doc = app.document.as_ref().expect("a drawing");
    let panel = app.properties_panel(doc);
    panel
        .sections
        .iter()
        .find(|s| s.id == "mapWork")
        .map(|s| s.rows.iter().map(|r| (r.label.to_string(), r.value.clone())).collect())
        .unwrap_or_default()
}

fn row<'a>(rows: &'a [(String, String)], label: &str) -> Option<&'a str> {
    rows.iter().find(|(l, _)| l == label).map(|(_, v)| v.as_str())
}

#[test]
fn an_approved_design_shows_its_change_and_is_approved_again() {
    let mut app = opened();
    // In review, without a stamp, its content revision is not worked out on a quick read.
    assert_eq!((tree(&app).items[3].content.as_deref(), tree(&app).items[3].approved.as_deref()), (None, None));
    send(&mut app, P::Status("is-4".into(), W::Approved));
    let design = tree(&app).items[3].clone();
    assert_eq!((design.status, design.state), (W::Approved, S::Current));
    assert!(design.approved.as_ref().is_some_and(|a| a.len() == 64), "{design:?}");
    assert_eq!(design.approved, design.content, "the stamp is its content revision");
    // A line of the design changed after the approval.
    change_line(&mut app, 6, "1060");
    let t = tree(&app);
    assert_eq!(t.items[3].state, S::Revised);
    assert!(t.items[3].why[0].starts_with("Onaydan sonra içeriği değişti"), "{:?}", t.items[3].why);
    assert_ne!(t.items[3].content, t.items[3].approved);
    assert_eq!(t.problems, 1);
    assert_eq!(crate::map_work_panel::state_words(S::Revised), "Onay eskidi");
    // Yeniden onayla: today's revision stamped, one undo step.
    send(&mut app, P::Status("is-4".into(), W::Approved));
    let t = tree(&app);
    assert_eq!((t.items[3].state, t.problems), (S::Current, 0));
    assert_eq!(t.items[3].approved, t.items[3].content);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(tree(&app).items[3].state, S::Revised, "undone: the old stamp again");
    // Back to draft: the stamp goes with the approval.
    send(&mut app, P::Status("is-4".into(), W::Draft));
    let t = tree(&app);
    assert_eq!((t.items[3].status, t.items[3].approved.as_deref(), t.items[3].state), (W::Draft, None, S::Current));
}

#[test]
fn a_line_is_recorded_as_made_from_the_design_and_follows_it() {
    let mut app = opened();
    // The user's own line: every item may be its source, the design (no file) too.
    app.selection.set(vec![Slot(8)]);
    let offered: Vec<String> = app.source_items().into_iter().map(|i| i.0).collect();
    assert_eq!(offered, ["is-1", "is-2", "is-3", "is-4", "is-5"]);
    // A line of the design: the design is not the source of its own objects.
    app.selection.set(vec![Slot(6)]);
    let offered: Vec<String> = app.source_items().into_iter().map(|i| i.0).collect();
    assert_eq!(offered, ["is-1", "is-2", "is-3", "is-5"]);
    // Seçimi türet: the line recorded as made from the design at its content revision.
    app.selection.set(vec![Slot(8)]);
    send(&mut app, P::DeriveFrom("is-4".into()));
    send(&mut app, P::Derive);
    let t = tree(&app);
    let d = t.derivations.iter().find(|d| d.source == "is-4").expect("recorded").clone();
    assert_eq!((d.state, d.objects), (S::Current, 1));
    assert_eq!(Some(&d.revision), t.items[3].content.as_ref(), "the design's content revision then");
    let rows = source_rows(&mut app, 8);
    assert_eq!(row(&rows, "Rol"), Some("Kaynaktan türetildi"));
    assert_eq!(row(&rows, "Kaynağı"), Some("Tasarım yüzeyi (is-4)"));
    assert_eq!(row(&rows, "Güncellik"), Some("Güncel"));
    // The design changes: the recorded line is out of date, in the tree and in Öznitelikler.
    change_line(&mut app, 7, "1061");
    let t = tree(&app);
    let d = t.derivations.iter().find(|d| d.source == "is-4").expect("recorded");
    assert_eq!((d.state, t.stale), (S::Stale, 1));
    assert_eq!(row(&source_rows(&mut app, 8), "Güncellik"), Some("Güncel değil: kaynağı değişti"));
    // Undone: up to date again.
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(tree(&app).stale, 0);
}

#[test]
fn a_stamp_and_a_design_source_survive_saving_and_opening_again() {
    let mut app = opened();
    app.selection.set(vec![Slot(8)]);
    send(&mut app, P::DeriveFrom("is-4".into()));
    send(&mut app, P::Derive);
    send(&mut app, P::Status("is-4".into(), W::Approved));
    let before = tree(&app);
    let model = &app.document.as_ref().expect("a drawing").model;
    let bytes = kentos_kcad::encode(&model.to_snapshot_v2()).expect("encodes");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(back.settings.map_work, model.map_work().cloned(), "the stamp and the record are saved whole");
    // Opened again: the objects' content gives the same revision, so the approval and the record stay up to date.
    let (mut again, _) = App::boot(None);
    again.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(back, None).expect("opens");
    let _ = again.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let after = tree(&again);
    assert_eq!(after.items, before.items);
    assert_eq!(after.derivations, before.derivations);
    assert_eq!((after.items[3].state, after.stale, after.problems), (S::Current, 0, 0), "{after:?}");
}

/// The panel: the approved design changed after its approval (Yeniden onayla on its row) and the line made from it out
/// of date; then the user's own line selected with Seçimi türet offering the design; light wide and dark narrow,
/// `.run/gui-evidence/harita-isi-damga-*`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_work_stamps_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::MapWork));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("harita-isi-damga-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        // The user's own line selected: Seçimi türet offers the design.
        app.selection.set(vec![Slot(8)]);
        send(&mut app, P::DeriveFrom("is-4".into()));
        for key in ["kind:design", "item:is-4"] {
            send(&mut app, P::Fold(key.into()));
        }
        shot(&mut app, "tasarimdan-turet");
        // Recorded, the design approved, then changed: the row wants a new approval, the line is out of date.
        send(&mut app, P::Derive);
        send(&mut app, P::Status("is-4".into(), W::Approved));
        change_line(&mut app, 6, "1060");
        // The design's branch now wants attention and starts open: the flips taken back.
        for key in ["kind:design", "item:is-4"] {
            send(&mut app, P::Fold(key.into()));
        }
        app.selection.set(vec![Slot(8)]);
        shot(&mut app, "onaydan-sonra-degisti");
    }
}

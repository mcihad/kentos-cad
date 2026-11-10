//! Yalnız değişeni yaz (Proje ağacı, docs/adr/4320, 4740; plan/map-cad.md MAP-14): the project's box, ticked by default,
//! draws a stale contour drawing again keeping the lines that come out as they are, says how many stayed, were written
//! and removed and the box where they changed; undone, the old lines are back; unticked (the project's choice now), the
//! drawing is drawn whole; and its pictures.

use kentos_commands::RoadNodeState as S;
use kentos_domain::{Slot, Uuid};
use serde_json::{Value, json};

use crate::app::{App, Message, Panel};
use crate::road_project_panel::Event as P;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.project.rebuild.regional.json")).expect("cases")
}

/// The shared cases' map project with the ground's contour lines drawn (cz-1) and one point of the second hill raised by
/// 0.6 m (their first case's steps).
fn regional_app() -> App {
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Map);
    let cases = cases();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    for step in &cases["cases"][0]["steps"].as_array().expect("steps")[..2] {
        let mut input = step["input"].clone();
        if let Some(uid) = input.pointer_mut("/changes/0/uid") {
            let slot: u32 = uid.as_str().and_then(|t| t.strip_prefix("$uidOf:")).and_then(|n| n.parse().ok()).expect("a slot");
            let model = &app.document.as_ref().expect("a drawing").model;
            *uid = json!(model.uid(Slot(slot)).expect("the point").to_string());
        }
        crate::program_scenes::command(&mut app, step["command"].as_str().expect("a command"), input);
    }
    app
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::RoadProject(e));
}

/// A rebuild run as a job (docs/adr/2440), the panel looked at until it is over.
fn rebuild(app: &mut App, e: P) {
    send(app, e);
    let start = std::time::Instant::now();
    while app.road_project.running() {
        assert!(start.elapsed() < std::time::Duration::from_secs(120), "the rebuild ends");
        std::thread::sleep(std::time::Duration::from_millis(5));
        send(app, P::Tick);
    }
}

fn drawing_uids(app: &App) -> Vec<String> {
    app.document.as_ref().expect("a drawing").model.road_drawings()[0].uids.clone()
}

fn said(app: &App) -> String {
    app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n")
}

/// Whether the project draws whole (docs/adr/4740): the tree's Yalnız değişeni yaz unticked.
fn whole(app: &App) -> Option<bool> {
    let t = app.road_project_read()?.ok()?;
    Some(!t.regional)
}

fn state(app: &App) -> S {
    let t = app.road_project_read().expect("a drawing").expect("a map project");
    t.nodes.iter().find(|n| n.kind == kentos_commands::RoadNodeKind::Drawing).expect("the contour lines").state
}

#[test]
fn yalniz_degiseni_yaz_keeps_the_lines_that_come_out_as_they_are() {
    let mut app = regional_app();
    let case = &cases()["cases"][0]["steps"][2]["result"]["output"]["rebuilt"][0];
    assert_eq!(state(&app), S::Stale);
    assert!(whole(&app).is_some_and(|w| !w), "Yalnız değişeni yaz is ticked by default");
    let before = drawing_uids(&app);
    rebuild(&mut app, P::Rebuild(None, false));
    assert_eq!(state(&app), S::Current);
    let after = drawing_uids(&app);
    let stayed = after.iter().filter(|u| before.contains(u)).count();
    assert_eq!(stayed as u64, case["kept"].as_u64().expect("the case's kept"), "the lines alike keep their ids");
    let removed = before.iter().filter(|u| !after.contains(u)).count();
    assert_eq!(removed as u64, case["removed"].as_u64().expect("the case's removed"));
    let words = said(&app);
    assert!(
        words.contains(&format!("{stayed} nesne yerinde kaldı, {} yazıldı, {removed} silindi; değişen bölge", after.len() - stayed)),
        "{words}"
    );
    // The lines that stayed are where they were: their numbers older than every line written now.
    let model = &app.document.as_ref().expect("a drawing").model;
    let slot = |u: &String| model.slot_of(Uuid::parse_str(u).expect("a uid")).expect("there").0;
    let kept = after.iter().filter(|u| before.contains(u)).map(slot).max().expect("lines stayed");
    let written = after.iter().filter(|u| !before.contains(u)).map(slot).min().expect("lines written");
    assert!(kept < written, "{kept} < {written}");
    // Undone, the old lines are back and the drawing is stale again.
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(drawing_uids(&app), before);
    assert_eq!(state(&app), S::Stale);
    // Unticked: drawn whole, every line new.
    send(&mut app, P::Regional(false));
    assert_eq!(whole(&app), Some(true), "the project's choice now (docs/adr/4740)");
    rebuild(&mut app, P::Rebuild(None, false));
    let whole = drawing_uids(&app);
    assert!(whole.iter().all(|u| !before.contains(u)), "drawn whole");
    assert_eq!(whole.len(), after.len());
    assert!(said(&app).contains(&format!("1 çizim yeniden üretildi: {} nesne.", whole.len())));
}

/// `.run/gui-evidence/yalniz-degisen-4320/`: the tree with the stale drawing and the box; drawn again, the lines written
/// selected (the changed hill) and the sentence; the same zoomed to the box.
///
/// ```text
/// cargo test -p kentos-desktop --lib road_project_regional_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yalniz-degisen-4320");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = regional_app();
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::RoadProject));
        app.docks.close(Panel::Properties);
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
        let _ = app.update(Message::RibbonTab("terrain"));
        shot(&mut app, "1-egriler-eskidi");
        let before = drawing_uids(&app);
        rebuild(&mut app, P::Rebuild(None, false));
        // The lines written, selected: where the drawing changed.
        let model = &app.document.as_ref().expect("a drawing").model;
        let written: Vec<Slot> = drawing_uids(&app)
            .iter()
            .filter(|u| !before.contains(u))
            .filter_map(|u| model.slot_of(Uuid::parse_str(u).ok()?))
            .collect();
        app.selection.set(written);
        shot(&mut app, "2-yalniz-degisenler-yazildi");
    }
}

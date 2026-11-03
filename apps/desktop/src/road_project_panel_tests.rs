//! Yol projesi (road_project_panel.rs, plan/road.md Y13.3): the panel reads the road project's tree, a drawing goes
//! stale when its template changes and comes back by Hepsini yeniden üret, Kendiliğinden yeniden üret turns on, a
//! source's row opens its editor; and its pictures.

use kentos_commands::RoadNodeState as S;
use serde_json::{Value, json};

use crate::app::{App, Message, Panel};
use crate::road_project_panel::Event as P;

/// The road project of `road.project.query`'s cases (road_pay_cases.py's drawing): its corridor's slope plan and its
/// guardrail drawn, then a template renamed.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.project.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::program_scenes::command(&mut app, "road.corridors.draw", json!({"corridor": "c-1", "layerId": "0"}));
    crate::program_scenes::command(&mut app, "road.guardrails.draw", json!({"guardrail": "o-1", "layerId": "korkuluk", "labels": true}));
    crate::program_scenes::command(&mut app, "road.templates.edit", json!({"template": "tk-2", "operation": "set", "name": "Tip 2b"}));
    app
}

fn send(app: &mut App, e: P) {
    let _ = app.update(Message::RoadProject(e));
}

fn tree(app: &App) -> kentos_commands::RoadProjectTree {
    app.road_project_read().expect("a drawing").expect("a road project")
}

#[test]
fn the_panel_rebuilds_what_went_stale() {
    let mut app = app();
    let t = tree(&app);
    let drawings: Vec<_> = t.nodes.iter().filter(|n| n.kind == kentos_commands::RoadNodeKind::Drawing).collect();
    assert_eq!(drawings.len(), 2);
    assert!(drawings.iter().all(|d| d.state == S::Stale), "{drawings:?}");
    assert_eq!(t.rebuildable, 2);
    // Hepsini yeniden üret: both up to date, one undo step.
    send(&mut app, P::Rebuild(None, false));
    let t = tree(&app);
    assert_eq!(t.rebuildable, 0);
    assert!(t.nodes.iter().filter(|n| n.kind == kentos_commands::RoadNodeKind::Drawing).all(|d| d.state == S::Current));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(tree(&app).rebuildable, 2, "undone, they are stale again");
    // Kendiliğinden yeniden üret on: the project says so.
    send(&mut app, P::Auto(true));
    assert!(tree(&app).auto_rebuild);
    // Deep: the corridors are made, the first leaves stretches unsolved.
    send(&mut app, P::Deep(true));
    assert!(tree(&app).nodes.iter().any(|n| n.state == S::Unsolved));
    // A corridor's row opens Koridor.
    let k = tree(&app).nodes.iter().position(|n| n.id == "c-1").expect("the corridor");
    send(&mut app, P::Open(k));
    assert!(app.corridor_window.is_some(), "Koridor opens");
}

/// The panel in the dock, wide and light, narrow and dark: two stale drawings, then drawn again; deep with its
/// problems; `.run/gui-evidence/yol-proje-agaci-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_project_panel_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(Panel::RoadProject));
        // The panel the dock's whole height, for the picture.
        app.docks.close(Panel::Properties);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-proje-agaci-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        shot(&mut app, "guncel-degil");
        if name == "acik-genis" {
            send(&mut app, P::Rebuild(None, false));
            shot(&mut app, "yeniden-uretildi");
            send(&mut app, P::Deep(true));
            send(&mut app, P::Auto(true));
            shot(&mut app, "kurarak");
        }
    }
}

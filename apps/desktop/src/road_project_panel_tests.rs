//! Yol projesi (road_project_panel.rs, plan/road.md Y13.3): the panel reads the road project's tree, a drawing goes
//! stale when its template changes and comes back by Hepsini yeniden üret, Kendiliğinden yeniden üret turns on, a
//! source's row opens its editor; a rebuild runs as a job whose Durdur writes nothing (docs/adr/2440); and its pictures.

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

/// A rebuild, run as a job (docs/adr/2440): the panel looked at until it is over.
fn rebuild(app: &mut App, e: P) {
    send(app, e);
    let start = std::time::Instant::now();
    while app.road_project.running() {
        assert!(start.elapsed() < std::time::Duration::from_secs(120), "the rebuild ends");
        std::thread::sleep(std::time::Duration::from_millis(5));
        send(app, P::Tick);
    }
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
    rebuild(&mut app, P::Rebuild(None, false));
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
            rebuild(&mut app, P::Rebuild(None, false));
            shot(&mut app, "yeniden-uretildi");
            send(&mut app, P::Deep(true));
            send(&mut app, P::Auto(true));
            shot(&mut app, "kurarak");
        }
    }
}

/// A KentOS Harita project (docs/adr/1720): road_terrain_zeroline_cases.py's map project, the zero line of Tümsek over
/// Düz drawn, then one of Tümsek's heights set (an edit of its surface): Proje ağacı shows the zero line stale under
/// its base, Hepsini yeniden üret draws it again, and the drawing saves (`.kcad` schema 198).
fn map_app() -> App {
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Map);
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.zeroline.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["setup"]["settings"]["cadWork"] == "map").expect("a map case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::program_scenes::command(&mut app, "road.terrains.zeroline", json!({"base": "Düz", "comparison": "Tümsek", "layerId": "sifir"}));
    let surface = json!({"id": "t-10", "name": "Tümsek", "points": {"layers": ["tumsek"]},
                         "edits": [{"op": "setHeight", "x": 6000.0, "y": 0.0, "z": 119.0}]});
    crate::program_scenes::command(&mut app, "road.terrains.edit", json!({"operation": "set", "surface": surface}));
    app
}

#[test]
fn a_map_project_s_tree_shows_its_zero_line_stale_and_draws_it_again() {
    let mut app = map_app();
    assert_eq!(app.panel_title(Panel::RoadProject), "Proje ağacı");
    assert!(app.shows_panel(Panel::RoadProject), "Harita has the tree");
    let t = tree(&app);
    let zero = t.nodes.iter().find(|n| n.kind == kentos_commands::RoadNodeKind::Drawing).expect("the zero line");
    assert_eq!(zero.state, S::Stale, "{zero:?}");
    assert_eq!(zero.why, ["“Tümsek” yüzeyi çizildiğinden beri değişti."]);
    rebuild(&mut app, P::Rebuild(None, false));
    let t = tree(&app);
    assert!(t.nodes.iter().filter(|n| n.kind == kentos_commands::RoadNodeKind::Drawing).all(|d| d.state == S::Current));
    let doc = app.document.as_ref().expect("a drawing");
    let bytes = kentos_kcad::encode(&doc.model.to_snapshot_v2());
    assert!(bytes.is_ok(), "a map project with its drawings saves: {:?}", bytes.err());
}

/// docs/adr/1840: cut and fill zones drawn in a Harita project are kept with both surfaces; one of Tümsek's heights set
/// makes them stale, Hepsini yeniden üret draws them again (the zones as many), and the project saves (schema 205).
#[test]
fn a_map_project_s_cut_and_fill_zones_go_stale_and_are_drawn_again() {
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Map);
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.cutfill.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["setup"]["settings"]["cadWork"] == "map").expect("a map case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let count = |app: &App| app.document.as_ref().map_or(0, |d| d.model.entities().filter(|e| e.base().layer_id == "kazi-dolgu").count());
    crate::program_scenes::command(&mut app, "road.terrains.cutfill", json!({"base": "Düz", "comparison": "Tümsek", "layerId": "kazi-dolgu"}));
    assert_eq!(count(&app), 4, "a cut and a fill zone, each a hatch and an area");
    let surface = json!({"id": "t-10", "name": "Tümsek", "points": {"layers": ["tumsek"]},
                         "edits": [{"op": "setHeight", "x": 6000.0, "y": 0.0, "z": 119.0}]});
    crate::program_scenes::command(&mut app, "road.terrains.edit", json!({"operation": "set", "surface": surface}));
    let t = tree(&app);
    let zones = t.nodes.iter().find(|n| n.kind == kentos_commands::RoadNodeKind::Drawing).expect("the zones");
    assert_eq!(zones.state, S::Stale, "{zones:?}");
    assert_eq!(zones.why, ["“Tümsek” yüzeyi çizildiğinden beri değişti."]);
    rebuild(&mut app, P::Rebuild(None, false));
    assert!(tree(&app).nodes.iter().filter(|n| n.kind == kentos_commands::RoadNodeKind::Drawing).all(|d| d.state == S::Current));
    assert_eq!(count(&app), 4, "drawn again in place");
    let doc = app.document.as_ref().expect("a drawing");
    let bytes = kentos_kcad::encode(&doc.model.to_snapshot_v2());
    assert!(bytes.is_ok(), "a map project with its zones saves: {:?}", bytes.err());
}

/// `.run/gui-evidence/harita-proje-agaci-1720/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_project_panel_tests::map_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn map_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/harita-proje-agaci-1720");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = map_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
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
        shot(&mut app, "1-sifir-hatti-eskidi");
        rebuild(&mut app, P::Rebuild(None, false));
        shot(&mut app, "2-yeniden-uretildi");
    }
}

/// docs/adr/2440: Hepsini yeniden üret runs as a job: the panel shows it running; Durdur stops it and nothing is
/// written (both drawings still stale, no undo step); let run, both are current in one step.
#[test]
fn a_rebuild_runs_as_a_job_and_durdur_writes_nothing() {
    let mut app = app();
    let steps = app.document.as_ref().map(|d| d.model.undo_labels().len());
    send(&mut app, P::Rebuild(None, false));
    assert!(app.road_project.running(), "a job, not the window's own thread");
    send(&mut app, P::Stop);
    rebuild(&mut app, P::Tick);
    assert_eq!(crate::files_testing::last_said(&app), "Yeniden üretim durduruldu; hiçbir şey yazılmadı.");
    assert_eq!(tree(&app).rebuildable, 2);
    assert_eq!(app.document.as_ref().map(|d| d.model.undo_labels().len()), steps, "nothing written");
    rebuild(&mut app, P::Rebuild(None, false));
    assert_eq!(tree(&app).rebuildable, 0);
    assert!(crate::files_testing::last_said(&app).starts_with("2 çizim yeniden üretildi"), "{}", crate::files_testing::last_said(&app));
    assert_eq!(app.document.as_ref().map(|d| d.model.undo_labels().len()), steps.map(|n| n + 1));
}

/// `.run/gui-evidence/yeniden-uretim-isi-2440/`: the panel with the rebuild running (its bar and Durdur), then done.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib road_project_panel_tests::job_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn job_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yeniden-uretim-isi-2440");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
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
        send(&mut app, P::Rebuild(None, false));
        shot(&mut app, "1-calisiyor");
        rebuild(&mut app, P::Tick);
        shot(&mut app, "2-bitti");
    }
}

/// pipes_profile_cases.py's Harita project (the sewer and storm networks, the ground surface).
fn pipes_app() -> App {
    // The network is built by İşlemler's tools, as the app registers them at its start.
    kentos_processing::command::register_as("desktop");
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Map);
    let cases: Value = serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.pipes.profile.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The sewer's run from P1, as Boru hattı denetimi draws it (pipes_profile_cases.py's first case).
fn pipe_profile() -> Value {
    json!({"network": "atiksu", "start": "0192f640-1640-7000-8000-0000000000f1",
           "inverts": {"from": "fields", "start": "Akar baş", "end": "Akar son"}, "diameter": {"field": "Çap", "unit": "mm"},
           "ground": {"from": "surface", "surface": "Zemin"}, "layerId": "profil", "at": [487000.0, 4419700.0]})
}

/// P1's material written: an attribute the profile does not read, so the profile drawn again has as many objects.
fn pipe_material(app: &mut App) {
    crate::program_scenes::command(
        app,
        "cad.entities.set",
        json!({"uids": ["0192f640-1640-7000-8000-0000000000f1"], "attrs": {"Malzeme": "PVC"}, "operation": "attributes"}),
    );
}

/// docs/adr/3560: a pipe run profile drawn in a Harita project is kept with its network: the tree has Atıksu with Hat
/// profili under it; P1's material written makes it stale, Hepsini yeniden üret draws it again in place (as many
/// objects), and the project saves (schema 212).
#[test]
fn a_pipe_profile_goes_stale_when_its_network_changes_and_is_drawn_again() {
    use kentos_commands::RoadNodeKind as N;
    let mut app = pipes_app();
    let count = |app: &App| app.document.as_ref().map_or(0, |d| d.model.entities().filter(|e| e.base().layer_id == "profil").count());
    crate::program_scenes::command(&mut app, "cad.pipes.profile", pipe_profile());
    let drawn = count(&app);
    assert_eq!(drawn, 112, "the shared case's objects");
    let t = tree(&app);
    let at = t.nodes.iter().position(|n| n.kind == N::Network).expect("the network");
    assert_eq!(t.nodes[at].name, "Atıksu");
    let profile = t.nodes.iter().find(|n| n.kind == N::Drawing).expect("the profile");
    assert_eq!((profile.name.as_str(), profile.state, profile.parent), ("Hat profili · Profil", S::Current, Some(at as u32)));
    pipe_material(&mut app);
    let t = tree(&app);
    let profile = t.nodes.iter().find(|n| n.kind == N::Drawing).expect("the profile");
    assert_eq!(profile.state, S::Stale, "{profile:?}");
    assert_eq!(profile.why, ["“Atıksu” ağı çizildiğinden beri değişti."]);
    rebuild(&mut app, P::Rebuild(None, false));
    assert!(tree(&app).nodes.iter().filter(|n| n.kind == N::Drawing).all(|d| d.state == S::Current));
    assert_eq!(count(&app), drawn, "drawn again in place");
    let doc = app.document.as_ref().expect("a drawing");
    let bytes = kentos_kcad::encode(&doc.model.to_snapshot_v2()).expect("a map project with its profile saves");
    assert!(bytes.windows(10).any(|w| w == b"\x67version\x18\xd4"), "schema 212");
}

/// docs/adr/3560: the profile stale after P1 changed, then drawn again; light wide and dark narrow,
/// `.run/gui-evidence/hat-profili-canli-3560/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib road_project_panel_tests::pipe_profile_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn pipe_profile_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/hat-profili-canli-3560");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = pipes_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        crate::program_scenes::command(&mut app, "cad.pipes.profile", pipe_profile());
        pipe_material(&mut app);
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
        shot(&mut app, "1-hat-profili-guncel-degil");
        rebuild(&mut app, P::Rebuild(None, false));
        shot(&mut app, "2-yeniden-uretildi");
    }
}

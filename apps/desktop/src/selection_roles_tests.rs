//! The contextual tabs' role panels (selection_roles.rs, docs/adr/2480) on the shared cases' drawings: a utility
//! network's pipe opens Çizgi with Altyapı, Boru hattı denetimi starting on that pipe's network; a surface's points
//! open Nokta with Yüzey, Pafta eğrileri, Yüzey analizi and Arazi yüzeyleri starting on that surface; a zoning plan's
//! decision area opens Alan with İmar; a selection whose objects do not all share the role has no such panel; a CBS
//! project none. And their pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};

fn open(file: &str, program: Option<Module>) -> App {
    let cases: Value = serde_json::from_str(&std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/commands/v1").join(file),
    )
    .expect("the cases"))
    .expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The objects of these layers (the first `n` of each) selected.
fn select(app: &mut App, layers: &[(&str, usize)]) {
    let doc = app.document.as_ref().expect("a drawing");
    let mut slots = Vec::new();
    for (layer, n) in layers {
        slots.extend(
            doc.model.entities().filter(|e| e.base().layer_id == *layer).take(*n).map(|e| kentos_domain::Slot(e.base().id)),
        );
    }
    assert!(!slots.is_empty(), "objects on {layers:?}");
    app.selection.set(slots);
}

/// The selection's contextual tab and its panels' names.
fn shown(app: &App) -> (&'static str, Vec<&'static str>) {
    let tab = app.selection_tab().expect("a contextual tab");
    (tab.id, app.tab_groups(tab).into_iter().filter_map(|(p, _)| p.map(|p| p.label)).collect())
}

#[test]
fn a_pipe_opens_altyapi_on_its_own_network() {
    let mut app = open("cad.pipes.check.json", Some(Module::Map));
    select(&mut app, &[("su", 2)]);
    let (tab, panels) = shown(&app);
    assert_eq!(tab, "ctx.line");
    assert!(panels.contains(&"Altyapı"), "{panels:?}");
    assert_eq!(app.selection_roles().network.as_deref(), Some("su"));
    let _ = app.update(Message::Run("pipes.check"));
    assert_eq!(app.pipes_check.network, "su", "the selection's network, not the first");
    let _ = app.update(Message::PipesCheck(crate::pipes_check_window::Event::Close));
    // A sewer pipe and a water pipe: no one network holds both.
    select(&mut app, &[("su", 1), ("atiksu", 1)]);
    assert!(!shown(&app).1.contains(&"Altyapı"));
    let _ = app.update(Message::Run("pipes.check"));
    assert_eq!(app.pipes_check.network, "atiksu", "the first network again");
}

#[test]
fn a_surface_s_points_open_yuzey_on_that_surface() {
    let mut app = open("road.terrains.sheetcontours.json", Some(Module::Map));
    select(&mut app, &[("cukur", 3)]);
    let (tab, panels) = shown(&app);
    assert_eq!(tab, "ctx.point");
    assert!(panels.contains(&"Yüzey") && !panels.contains(&"Altyapı"), "{panels:?}");
    assert_eq!(app.selection_roles().surface.as_deref(), Some("t-3"));
    let _ = app.update(Message::Run("terrain.sheetContours"));
    assert_eq!(app.sheet_contours.surface, "t-3", "Çukur, not the first surface");
    app.dialog = None;
    let _ = app.update(Message::Run("terrain.analysis"));
    let w = app.surface_analysis_window.as_ref().expect("the window");
    assert_eq!((w.surface.as_deref(), w.comparison.as_deref()), (Some("t-3"), Some("t-1")));
    app.dialog = None;
    app.surface_analysis_window = None;
    let _ = app.update(Message::Run("terrain.surfaces"));
    let w = app.terrain_window.as_ref().expect("the window");
    assert_eq!(w.chosen_def().map(|d| d.id.as_str()), Some("t-3"));
    // A building with the points: not a surface's source.
    app.terrain_window = None;
    app.dialog = None;
    select(&mut app, &[("cukur", 1), ("bina", 1)]);
    assert!(app.selection_roles().surface.is_none());
}

#[test]
fn a_decision_area_opens_imar() {
    let mut app = open("cad.zoning.check.json", Some(Module::Map));
    select(&mut app, &[("karar", 2)]);
    let (tab, panels) = shown(&app);
    assert_eq!(tab, "ctx.area");
    assert!(panels.contains(&"İmar"), "{panels:?}");
    // A plan boundary and a note: a mixed selection (Seçim) of the plan's objects still has İmar.
    select(&mut app, &[("plan", 1), ("notlar", 1)]);
    let (tab, panels) = shown(&app);
    assert_eq!(tab, "selection");
    assert!(panels.contains(&"İmar"), "{panels:?}");
    // A building of another drawing is no plan's object.
    let mut other = open("road.terrains.sheetcontours.json", Some(Module::Map));
    select(&mut other, &[("bina", 1)]);
    assert!(!shown(&other).1.contains(&"İmar"));
}

#[test]
fn a_gis_project_has_no_role_panels() {
    let mut app = open("cad.pipes.check.json", Some(Module::Gis));
    if let Some(doc) = app.document.as_mut() {
        let mut s = doc.model.settings().clone();
        s.workspace = Some(kentos_commands::Workspace::Gis);
        s.cad_work = None;
        let _ = doc.model.set_settings(s);
    }
    select(&mut app, &[("su", 1)]);
    let (_, panels) = shown(&app);
    assert!(!panels.contains(&"Altyapı") && !panels.contains(&"Yüzey"), "{panels:?}");
}

/// `.run/gui-evidence/baglamsal-roller-2480/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib selection_roles_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/baglamsal-roller-2480");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (file, layers, shot) in [
            ("cad.pipes.check.json", vec![("su", 2)], "1-altyapi"),
            ("road.terrains.sheetcontours.json", vec![("cukur", 3)], "2-yuzey"),
            ("cad.zoning.check.json", vec![("karar", 2)], "3-imar"),
        ] {
            let mut app = open(file, Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            select(&mut app, &layers);
            // The ribbon follows the selection after a message.
            let _ = app.update(Message::Run("view.zoomSelection"));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{shot}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

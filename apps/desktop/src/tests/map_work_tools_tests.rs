//! Harita işi on the contextual tabs and a moved project (docs/adr/1201): selecting a GNSS point in KentOS Harita opens
//! its tab with the Harita işi panel, whose Kaynağını göster brings the panel forward with the campaign's layer chosen
//! and Bağımsız yap lets the point go of its derivation in one undo step; Seçimi türet records a selection; the user's
//! own line leaves both off; KentOS GIS, Mimari and Yol offer neither. A project saved with its campaign beside it and
//! moved with its folder finds the file again by its relative path. And the pictures.

use crate::map;
use kentos_commands::{WorkFileState as F, WorkObjectRole};
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

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

/// `map.work.query`'s drawing (map_work_cases.py), its paths made absolute.
fn snapshot() -> kentos_commands::DocumentSnapshotV2 {
    let text = include_str!("../../../../fixtures/commands/v1/map.work.query.json")
        .replace("$fixture:", &format!("{}/", fixtures()));
    let cases: Value = serde_json::from_str(&text).expect("cases");
    serde_json::from_value(cases["setup"].clone()).expect("the drawing reads")
}

fn opened_at(snapshot: kentos_commands::DocumentSnapshotV2, path: Option<std::path::PathBuf>, program: Option<Module>) -> App {
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, path).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn opened(program: Option<Module>) -> App {
    opened_at(snapshot(), None, program)
}

fn select(app: &mut App, slots: &[u32]) {
    app.selection.set(slots.iter().map(|s| Slot(*s)));
}

fn tree(app: &App) -> kentos_commands::MapWorkTree {
    app.map_work_read().expect("a drawing").expect("a map project")
}

/// The commands of the selection's tab's Harita işi panel; none when the tab has no such panel.
fn work_panel(app: &App) -> Option<Vec<&'static str>> {
    let tab = app.selection_tab()?;
    tab.panels.iter().find(|p| p.label == "Harita işi").map(crate::catalog::Panel::commands)
}

#[test]
fn the_contextual_tabs_offer_the_source_and_letting_go_by_the_selections_role() {
    let mut app = opened(Some(Module::Map));
    // A GNSS point derived from the campaign: Nokta's tab, both on.
    select(&mut app, &[1]);
    assert_eq!(app.selection_tab().map(|t| t.id), Some("ctx.point"));
    assert_eq!(work_panel(&app), Some(vec!["map.workSource", "map.workRelease"]));
    assert!(app.available("map.workSource") && app.available("map.workRelease"));
    // The CAD reference's line (source data under a group): its source, nothing to let go of.
    select(&mut app, &[5]);
    assert_eq!(app.selection_tab().map(|t| t.id), Some("ctx.line"));
    assert!(work_panel(&app).is_some());
    assert!(app.available("map.workSource") && !app.available("map.workRelease"));
    // The user's own line on no item's layer: both off; the orthophoto's raster: Raster's tab has the panel too.
    select(&mut app, &[8]);
    assert!(!app.available("map.workSource") && !app.available("map.workRelease"));
    select(&mut app, &[4]);
    assert_eq!(app.selection_tab().map(|t| t.id), Some("ctx.raster"));
    assert!(work_panel(&app).is_some());
    // A mixed selection: Seçim's tab, letting go on while one of them is derived.
    select(&mut app, &[1, 4, 8]);
    assert_eq!(app.selection_tab().map(|t| t.id), Some("selection"));
    assert!(work_panel(&app).is_some() && app.available("map.workRelease"));
    // Kaynağını göster: the panel in front, the campaign's layer chosen, the source said.
    select(&mut app, &[1]);
    let _ = app.update(Message::Run("map.workSource"));
    assert_eq!(app.selected_layer.as_deref(), Some("gnss"));
    assert!(app.shows_panel(Panel::MapWork));
    // Bağımsız yap: G1 and G2 out of tr-1 (emptied, gone); G1 is the user's own now; one undo brings it back.
    select(&mut app, &[1, 2]);
    let _ = app.update(Message::Run("map.workRelease"));
    let t = tree(&app);
    assert_eq!(t.derivations.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(), ["tr-2"]);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let g1 = doc.uid(Slot(1)).expect("G1").to_string();
    let found = kentos_native_application::map_work::source_of(doc, &g1).expect("G1");
    assert_eq!(found.role, WorkObjectRole::Source, "a campaign point again: on the campaign's layer, not derived");
    assert_eq!(doc.undo_labels().first().map(String::as_str), Some("Bağımsız yap"));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(tree(&app).derivations.len(), 2);
    // Seçimi türet: G3 (a campaign point no record names) recorded as made from the orthophoto.
    select(&mut app, &[3]);
    let _ = app.update(Message::Map(map::Event::MapWork(P::DeriveFrom(
        "is-2".into(),
    ))));
    let _ = app.update(Message::Map(map::Event::MapWork(P::Derive)));
    let t = tree(&app);
    let made = t.derivations.last().expect("a new derivation");
    assert_eq!(
        (
            made.id.as_str(),
            made.source.as_str(),
            made.operation.as_str()
        ),
        ("tr-3", "is-2", "Elle türetme kaydı")
    );
}

#[test]
fn gis_mimari_and_yol_offer_no_map_work_tools() {
    for module in [Module::Gis, Module::Architecture, Module::Road] {
        let mut app = opened(Some(module));
        assert!(!app.offers("map.workSource") && !app.offers("map.workRelease"), "{module:?}");
        select(&mut app, &[1]);
        // A map project opened in another module's program: the panel's buttons are none of its own.
        assert!(work_panel(&app).unwrap_or_default().iter().all(|id| !app.offers(id)), "{module:?}");
        assert!(!app.available("map.workRelease"), "{module:?}");
    }
    // KentOS Harita and kentos-cad (every module) offer them.
    for program in [Some(Module::Map), None] {
        assert!(opened(program).offers("map.workSource"), "{program:?}");
    }
}

#[test]
fn a_project_moved_with_its_folder_finds_its_campaign_again() {
    let root = std::env::temp_dir().join(format!("kentos-map-work-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (first, second) = (root.join("ofis/proje"), root.join("arsiv/proje"));
    std::fs::create_dir_all(first.join("veri")).expect("a folder");
    let nmea = std::fs::read(format!("{}/map-work/v1/kampanya.nmea", fixtures())).expect("the log");
    std::fs::write(first.join("veri/kampanya.nmea"), &nmea).expect("writes");
    // The campaign's file beside the project; saved, its relative path written.
    let mut snap = snapshot();
    if let Some(f) = snap.settings.map_work.as_mut().and_then(|w| w.items[0].file.as_mut()) {
        f.path = first.join("veri/kampanya.nmea").display().to_string();
    }
    kentos_native_application::source_places::with_relative(&mut snap.settings, &first.join("ana.kcad"));
    let work = snap.settings.map_work.clone().expect("a work structure");
    assert_eq!(work.items[0].file.as_ref().and_then(|f| f.relative.as_deref()), Some("veri/kampanya.nmea"));
    let bytes = kentos_kcad::encode(&snap).expect("encodes (schema 193)");
    // Moved with its folder: the old place gone.
    std::fs::create_dir_all(second.join("veri")).expect("a folder");
    std::fs::write(second.join("veri/kampanya.nmea"), &nmea).expect("writes");
    std::fs::write(second.join("ana.kcad"), &bytes).expect("writes");
    std::fs::remove_dir_all(root.join("ofis")).expect("the old place gone");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    let app = opened_at(back, Some(second.join("ana.kcad")), Some(Module::Map));
    let t = tree(&app);
    let file = t.items[0].file.as_ref().expect("the campaign's file");
    assert_eq!(file.path, second.join("veri/kampanya.nmea").display().to_string(), "taken at its relative path");
    assert_eq!(file.state, F::Found);
    assert_eq!(t.problems, 0, "{t:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The contextual tab with its Harita işi panel and the panel's Seçimi türet, light and wide, dark and narrow;
/// `.run/gui-evidence/map-work-1201/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib map_work_tools_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/map-work-1201");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
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
        for key in ["kind:campaign", "kind:reference", "item:is-1"] {
            let _ = app.update(Message::Map(map::Event::MapWork(P::Fold(key.into()))));
        }
        // The GNSS point selected: Nokta's tab in front with Harita işi; Kaynağını göster brings the panel.
        select(&mut app, &[1]);
        app.follow_selection_tab();
        let _ = app.update(Message::Run("map.workSource"));
        shot(&mut app, "1-nokta-sekmesi-kaynagi");
        // Two points let go: the campaign's import record gone from the tree.
        select(&mut app, &[1, 2]);
        app.follow_selection_tab();
        let _ = app.update(Message::Run("map.workRelease"));
        shot(&mut app, "2-bagimsiz-yap");
        // The third point selected: Seçimi türet's row in the panel.
        let _ = app.update(Message::Run("edit.undo"));
        select(&mut app, &[3]);
        app.follow_selection_tab();
        shot(&mut app, "3-secimi-turet");
    }
}

//! The Yeni proje wizard driven by its events and keys, as the web's smoke run
//! drives its own (apps/web/scripts/e2e/smoke.mjs), and its pictures.

use iced::keyboard::key::Named;
use kentos_commands::{CadWork, DrawingUnit, Workspace};
use kentos_project::wizard::{Coords, Kind, Purpose, Step, Storage};

/// The works' cards (docs/adr/0626 §1, 0802: two CAD works): a building and a plan on real coordinates.
const BUILDING: usize = 0;
const PLAN: usize = 1;

use super::Event;
use crate::app::{App, Dialog, Message, Picker};
use crate::files_testing::{app_with_drawing, drawing, drive, last_said, scratch};
use crate::project::{Event as ProjectEvent, Window};

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Project(Box::new(ProjectEvent::New(e))));
}

/// An event whose tasks run to their end (a template read off the interface's thread).
fn send_driven(app: &mut App, e: Event) {
    let task = app.update(Message::Project(Box::new(ProjectEvent::New(e))));
    drive(app, task);
}

fn key(app: &mut App, named: Named) {
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key: iced::keyboard::Key::Named(named),
        physical: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        modifiers: iced::keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    }));
}

fn state(app: &App) -> &super::State {
    match &app.project {
        Some(Window::New(s)) => s,
        _ => panic!("the wizard is not open"),
    }
}

fn open(app: &mut App) {
    let _ = app.update(Message::Run("file.new"));
    assert_eq!(app.dialog, Some(Dialog::Project));
    assert_eq!(state(app).step, Step::Type);
}

#[test]
fn a_cad_project_on_real_coordinates_walks_the_three_steps() {
    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Cad));
    send(&mut app, Event::Next);
    assert_eq!(state(&app).step, Step::Coords);
    // A CAD project is a building first; a plan is on real coordinates, a province's and a system's.
    assert_eq!(state(&app).draft.coords, Coords::Local);
    send(&mut app, Event::Work(PLAN));
    assert_eq!(state(&app).draft.coords, Coords::Real);
    send(&mut app, Event::System(5254));
    send(&mut app, Event::Next);
    assert_eq!(state(&app).step, Step::Details);
    send(&mut app, Event::Name("  Ada 7  ".into()));
    send(&mut app, Event::Next);
    assert_eq!(app.dialog, None);
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.name(), "Ada 7");
    let settings = doc.settings();
    assert_eq!(
        (settings.srid, settings.plot_scale, settings.workspace),
        (5254, 1000.0, Some(Workspace::Cad))
    );
    assert_eq!(settings.drawing_unit, None);
    // A CAD project starts with technical drawing layers, Çizim active (docs/adr/0165 §3).
    assert_eq!(doc.model.layers().active(), "cizim");
    assert_eq!(doc.entity_count(), 0);
    assert!(doc.path.is_none() && !doc.dirty());
    assert!(
        last_said(&app)
            .starts_with("“Ada 7” yeni projesi açıldı: TUREF / TM30 (EPSG:5254), 1:1000."),
        "{}",
        last_said(&app)
    );
}

#[test]
fn a_local_cad_project_is_a_building_drawn_in_its_unit_from_one_to_one() {
    // Two CAD works (docs/adr/0802): a drawing of its own is a building's whatever its unit; in the
    // millimetres the app gives new projects it is drawn at full size.
    let mut app = app_with_drawing();
    let _ = app
        .settings
        .choose(&[("newProjects.drawingUnit", serde_json::Value::from("mm"))]);
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Cad));
    send(&mut app, Event::Next);
    assert_eq!(state(&app).draft.cad_work(), Some(CadWork::Architecture));
    send(&mut app, Event::Next);
    assert_eq!(state(&app).draft.scale(), 1.0, "a part at full size");
    send(&mut app, Event::Name("Mil plakası".into()));
    send(&mut app, Event::Next);
    let doc = app.document.as_ref().expect("a drawing");
    let s = doc.settings();
    assert_eq!(
        (s.srid, s.drawing_unit, s.plot_scale),
        (0, Some(DrawingUnit::Mm), 1.0)
    );
    assert_eq!(s.cad_work(), Some(CadWork::Architecture));
    assert!(
        last_said(&app).contains("Yerel (koordinat sistemi yok), 1:1."),
        "{}",
        last_said(&app)
    );
    // The type chosen, and a local project's unit, start the next wizard.
    assert_eq!(
        app.settings.effective("newProjects.drawingUnit"),
        serde_json::Value::from("mm")
    );
    open(&mut app);
    assert_eq!(
        (state(&app).draft.kind, state(&app).draft.unit),
        (Kind::Cad, DrawingUnit::Mm)
    );
}

#[test]
fn a_new_local_project_starts_in_the_unit_of_the_apps_settings() {
    let mut app = app_with_drawing();
    let _ = app.settings.choose(&[
        ("newProjects.drawingUnit", serde_json::Value::from("cm")),
        ("newProjects.workspace", serde_json::Value::from("cad")),
    ]);
    open(&mut app);
    assert_eq!(
        (state(&app).draft.kind, state(&app).draft.unit),
        (Kind::Cad, DrawingUnit::Cm)
    );
    // A drawing of its own in centimetres is a building's (docs/adr/0802); its own unit and its scale go on.
    assert_eq!(state(&app).draft.cad_work(), Some(CadWork::Architecture));
    assert_eq!(state(&app).draft.scale(), 50.0);
    // A project with a system keeps the unit asked for before.
    send(&mut app, Event::Go(1));
    send(&mut app, Event::Work(PLAN));
    send(&mut app, Event::Go(2));
    send(&mut app, Event::Next);
    assert_eq!(
        app.document
            .as_ref()
            .expect("a drawing")
            .settings()
            .drawing_unit,
        None
    );
    assert_eq!(
        app.settings.effective("newProjects.drawingUnit"),
        serde_json::Value::from("cm")
    );
}

#[test]
fn a_cbs_project_takes_its_provinces_zone_and_opens_on_it() {
    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Gis));
    send(&mut app, Event::Next);
    // A CBS project's place is on its one page (docs/adr/9184).
    assert_eq!(state(&app).step, Step::Details);
    // Enter in the search takes its first province.
    send(&mut app, Event::Search("izm".into()));
    send(&mut app, Event::SearchFirst);
    let d = &state(&app).draft;
    assert_eq!(
        (d.province, d.srid(), d.suggested_srid()),
        (Some(35), 5253, Some(5253))
    );
    // A system chosen, then another province: the system follows its zone again.
    send(&mut app, Event::System(2320));
    assert_eq!(state(&app).draft.srid(), 2320);
    send(&mut app, Event::Province(61));
    assert_eq!(state(&app).draft.srid(), 5257, "Trabzon: TM39");
    // The chosen province again chooses none: the app's default system.
    send(&mut app, Event::Province(61));
    assert_eq!(state(&app).draft.province, None);
    send(&mut app, Event::Province(35));
    send(&mut app, Event::Next);
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.settings().srid, 5253);
    assert_eq!(doc.model.layers().active(), "taslak");
    // İzmir is 0.14° east of TM27's meridian: its centre lies some 12 km east of 500 000.
    let home = doc.model.home_view().expect("a home view");
    let east = (home.min_x + home.max_x) / 2.0;
    assert!((505_000.0..520_000.0).contains(&east), "{east}");
}

/// Yeni proje in kentos-cad (docs/adr/9195 §2; the owner saw every menu after it): with nothing open the common
/// surfaces; the project made shows its module's alone, and so does the screen while Yeni proje is open again over
/// it; the next project made, of another module, shows that module's.
#[test]
fn a_new_project_shows_its_modules_surfaces_alone_and_so_does_the_next() {
    use crate::app::Panel;
    use crate::bottom::BottomTab;
    use kentos_launch::Module;
    let (mut app, _) = App::boot(None);
    assert_eq!(app.active_module(), None);
    assert_eq!(app.bottom_tabs(), [BottomTab::History, BottomTab::Works, BottomTab::Messages, BottomTab::Python]);
    // A map's project on real coordinates.
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Cad));
    send(&mut app, Event::Next);
    send(&mut app, Event::Work(PLAN));
    send(&mut app, Event::System(5254));
    send(&mut app, Event::Next);
    send(&mut app, Event::Name("Ada 7".into()));
    send(&mut app, Event::Next);
    assert_eq!(app.active_module(), Some(Module::Map));
    let map_tabs = app.bottom_tabs();
    assert!(map_tabs.contains(&BottomTab::CadInspection) && !map_tabs.contains(&BottomTab::Chart), "{map_tabs:?}");
    assert!(app.docks.contains(Panel::Sources) && !app.docks.contains(Panel::Navigator));
    // Yeni proje again: the screen behind it is still the map's.
    open(&mut app);
    assert_eq!(app.bottom_tabs(), map_tabs);
    assert!(!app.offers("tool.wall"), "no building's command over a map's project");
    // A CBS project made: its module's surfaces alone.
    send(&mut app, Event::Type(Workspace::Gis));
    send(&mut app, Event::Next);
    send(&mut app, Event::Province(6));
    send(&mut app, Event::Next);
    let _ = app.update(Message::Swallowed);
    assert_eq!(app.active_module(), Some(Module::Gis));
    let gis_tabs = app.bottom_tabs();
    assert!(!gis_tabs.contains(&BottomTab::CadInspection) && !gis_tabs.contains(&BottomTab::Audit), "{gis_tabs:?}");
    assert!(gis_tabs.contains(&BottomTab::Chart));
    assert!(!app.docks.contains(Panel::Blocks) && app.docks.contains(Panel::Processing));
    assert!(!app.shows_aid("view.lineWeights"));
}

#[test]
fn the_rail_goes_back_and_forth_and_a_step_that_cannot_be_left_stops_the_way() {
    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Cad));
    send(&mut app, Event::Go(1));
    send(&mut app, Event::Work(PLAN));
    send(&mut app, Event::System(1234));
    // The details step from the rail: the coordinates stop the way and say why.
    send(&mut app, Event::Go(2));
    let s = state(&app);
    assert_eq!(s.step, Step::Coords);
    assert_eq!(
        s.status.as_deref(),
        Some("EPSG:1234 bu sürümde tanımlı değil; listeden bir sistem seçin.")
    );
    send(&mut app, Event::System(5255));
    assert_eq!(state(&app).status, None, "a change clears it");
    send(&mut app, Event::Go(2));
    assert_eq!(state(&app).step, Step::Details);
    send(&mut app, Event::Back);
    assert_eq!(state(&app).step, Step::Coords);
    send(&mut app, Event::Go(2));
    // An empty name: Oluştur says so and nothing changes.
    send(&mut app, Event::Name("   ".into()));
    send(&mut app, Event::Next);
    assert_eq!(app.dialog, Some(Dialog::Project));
    assert_eq!(state(&app).status.as_deref(), Some("Proje adı boş olamaz."));
    assert_eq!(
        app.document.as_ref().expect("open").name(),
        "Örnek pafta.kcad"
    );
}

#[test]
fn enter_goes_on_and_the_arrows_choose_the_type() {
    let mut app = app_with_drawing();
    open(&mut app);
    let first = state(&app).draft.kind;
    key(&mut app, Named::ArrowRight);
    assert_ne!(state(&app).draft.kind, first);
    key(&mut app, Named::ArrowLeft);
    assert_eq!(state(&app).draft.kind, first);
    key(&mut app, Named::Enter);
    // CBS's one page, or what a CAD project is for (docs/adr/9184).
    assert_eq!(state(&app).step, state(&app).draft.steps()[1]);
    // A double click on a card chooses it and goes on.
    send(&mut app, Event::Back);
    send(&mut app, Event::Choose(Workspace::Cad));
    assert_eq!(
        (state(&app).draft.kind, state(&app).step),
        (Kind::Cad, Step::Coords)
    );
    // An announced type cannot be chosen.
    send(&mut app, Event::Type(Workspace::Plan3d));
    assert_eq!(state(&app).draft.kind, Kind::Cad);
    key(&mut app, Named::Escape);
    assert_eq!(app.dialog, None);
}

#[test]
fn a_work_brings_its_unit_and_scale_and_another_type_its_own() {
    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Go(1));
    assert_eq!(state(&app).draft.scale(), 1000.0, "CBS: 1:1000");
    send(&mut app, Event::Type(Workspace::Cad));
    // A new CAD project is a building: metres, 1:100, from 0,0 (docs/adr/0626 §1).
    let d = &state(&app).draft;
    assert_eq!(
        (d.coords, d.unit, d.scale()),
        (Coords::Local, DrawingUnit::M, 100.0)
    );
    send(&mut app, Event::Work(PLAN));
    let d = &state(&app).draft;
    assert_eq!(
        (d.coords, d.unit(), d.scale()),
        (Coords::Real, DrawingUnit::M, 1000.0)
    );
    // A road shares the map's coordinates and scale; its card says it is a road (plan/road.md).
    send(&mut app, Event::Work(2));
    let d = &state(&app).draft;
    assert_eq!(
        (d.work().map(|w| w.name), d.coords, d.scale()),
        (Some("Yol projesi"), Coords::Real, 1000.0)
    );
    send(&mut app, Event::Work(BUILDING));
    assert_eq!(state(&app).draft.scale(), 100.0);
    // A card that is no work's does nothing: there are three (docs/adr/0802, plan/road.md), the detail's is gone.
    send(&mut app, Event::Work(3));
    assert_eq!(
        state(&app).draft.work().map(|w| w.name),
        Some("Mimari proje")
    );
}

/// A CBS project is decided on one page (docs/adr/9184): its name, an empty city map instead of a cadastral sheet,
/// its place, and a GeoPackage to keep it in; the first Kaydet asks where, with its name, and writes nothing before.
#[test]
fn a_cbs_project_is_decided_on_one_page_and_its_first_save_goes_where_it_was_chosen() {
    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Gis));
    send(&mut app, Event::Next);
    let s = state(&app);
    assert_eq!((s.step, s.draft.steps().len()), (Step::Details, 2));
    assert_eq!((s.draft.purpose, s.draft.storage), (Purpose::Cadastre, Storage::File));
    send(&mut app, Event::Name("Ankara-Mahalle-Hizmetleri".into()));
    send(&mut app, Event::Purpose(Purpose::City));
    send(&mut app, Event::Province(6));
    send(&mut app, Event::Storage(Storage::Geopackage));
    send(&mut app, Event::Next);
    assert_eq!(app.dialog, None);
    let doc = app.document.as_ref().expect("the new project");
    assert_eq!(doc.name(), "Ankara-Mahalle-Hizmetleri");
    // One draft layer: no cadastral tree imposed.
    let leaves: Vec<String> = doc.model.layers().leaves().iter().map(|n| n.id.clone()).collect();
    assert_eq!(leaves, ["taslak"]);
    assert_eq!(doc.model.layers().active(), "taslak");
    assert_eq!((doc.settings().srid, doc.wanted_store()), (5255, Some(Storage::Geopackage)));
    assert!(doc.path.is_none() && !doc.dirty());
    assert!(
        last_said(&app).ends_with("İlk Kaydet, GeoPackage dosyasını ve proje adını sorar."),
        "{}",
        last_said(&app)
    );
    // Kaydet opens Veri kaynağı on a GeoPackage, the project's name offered; Vazgeç writes nothing.
    let _ = app.update(Message::Run("file.save"));
    let w = app.gis.project_store.as_ref().expect("Veri kaynağı");
    assert_eq!((w.database_for_test(), w.name_for_test().as_str()), (false, "Ankara-Mahalle-Hizmetleri"));
    app.gis.project_store = None;
    app.dialog = None;
    assert_eq!(app.document.as_ref().and_then(|d| d.wanted_store()), Some(Storage::Geopackage));

    // A database: its Kaydet looks in a database.
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Gis));
    send(&mut app, Event::Next);
    send(&mut app, Event::Storage(Storage::Database));
    send(&mut app, Event::Next);
    if matches!(app.dialog, Some(Dialog::Unsaved(_))) {
        let _ = app.update(Message::DialogConfirmed);
    }
    assert!(last_said(&app).ends_with("İlk Kaydet, veritabanı bağlantısını, şemayı ve proje adını sorar."));
    let _ = app.update(Message::Run("file.save"));
    assert!(app.gis.project_store.as_ref().expect("Veri kaynağı").database_for_test());
    app.gis.project_store = None;
    app.dialog = None;
    // Farklı kaydet writes a file: the project is a file's from then on.
    let dir = scratch("cbs-tek-sayfa");
    app.picker = Picker::File(dir.join("kent.kcad"));
    let task = app.update(Message::Run("file.saveAs"));
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("saved");
    assert_eq!((doc.wanted_store(), doc.path.as_deref()), (None, Some(dir.join("kent.kcad").as_path())));
}

/// Kurum şablonu (docs/adr/9184): the template's layers, scale, forms, relations and topology rules come; none of its
/// objects nor its topology exceptions; the wizard's name and system stay. A CAD project is no template.
#[test]
fn a_template_gives_its_layers_and_settings_and_none_of_its_objects() {
    let dir = scratch("kurum-sablonu");
    let path = dir.join("Belediye şablonu.kcad");
    let mut template = drawing(0);
    let mut s = template.settings().clone();
    s.forms = serde_json::from_value(serde_json::json!([
        { "id": "bina-formu", "name": "Bina formu", "layer": "bina", "draft": { "revision": 1, "items": [] } }
    ]))
    .expect("a form");
    s.relations = serde_json::from_value(serde_json::json!([
        { "id": "parsel-bina", "name": "Parselin binaları", "parent": "parsel", "child": "bina", "field": "parsel_id" }
    ]))
    .expect("a relation");
    s.topology = Some(
        serde_json::from_value(serde_json::json!({
            "rules": [{ "id": "kural-1", "kind": "mustNotOverlap", "layer": "parsel" }],
            "exceptions": [{ "rule": "kural-1", "objects": [], "at": { "x": 0.0, "y": 0.0 } }]
        }))
        .expect("topology"),
    );
    s.plot_scale = 500.0;
    template.model.set_settings(s);
    assert!(template.entity_count() > 0);
    crate::document::write(&template.model.to_snapshot_v2(), &path).expect("the template is written");

    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Gis));
    send(&mut app, Event::Next);
    send(&mut app, Event::Purpose(Purpose::Template));
    send(&mut app, Event::Next);
    assert_eq!(
        state(&app).status.as_deref(),
        Some("Kurum şablonu seçilmedi: Şablon seç… ile şablon projenin dosyasını gösterin.")
    );
    app.picker = Picker::File(path.clone());
    send_driven(&mut app, Event::PickTemplate);
    app.picker = Picker::Dialog;
    let s = state(&app);
    let t = s.template.as_ref().expect("read").as_ref().expect("a template");
    assert_eq!((t.name().as_str(), t.words().as_str()), ("Belediye şablonu.kcad", "3 katman, 1 form, 1 ilişki, 1 topoloji kuralı"));
    assert_eq!(s.draft.template.as_deref(), Some(path.display().to_string().as_str()));
    send(&mut app, Event::Province(35));
    send(&mut app, Event::Name("İzmir imar".into()));
    send(&mut app, Event::Next);
    let doc = app.document.as_ref().expect("the new project");
    assert_eq!((doc.name(), doc.entity_count()), ("İzmir imar", 0));
    let leaves: Vec<String> = doc.model.layers().leaves().iter().map(|n| n.id.clone()).collect();
    assert_eq!(leaves, ["parsel", "bina", "cizim"]);
    let st = doc.settings();
    // İzmir's zone and centre, not the template's TM36.
    assert_eq!((st.srid, st.plot_scale, st.workspace), (5253, 500.0, Some(Workspace::Gis)));
    assert_eq!((st.forms.len(), st.relations.len()), (1, 1));
    let topology = st.topology.as_ref().expect("the rules");
    assert_eq!((topology.rules.len(), topology.exceptions.len()), (1, 0));
    let home = doc.model.home_view().expect("a home view");
    let east = (home.min_x + home.max_x) / 2.0;
    assert!((505_000.0..520_000.0).contains(&east), "{east}");

    // A CAD project is refused as a template, with the reason; Oluştur stays closed.
    let cad = dir.join("Mimari.kcad");
    let mut c = drawing(0);
    let mut cs = c.settings().clone();
    cs.workspace = Some(Workspace::Cad);
    c.model.set_settings(cs);
    crate::document::write(&c.model.to_snapshot_v2(), &cad).expect("written");
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Gis));
    send(&mut app, Event::Next);
    app.picker = Picker::File(cad);
    send_driven(&mut app, Event::PickTemplate);
    app.picker = Picker::Dialog;
    let s = state(&app);
    assert_eq!(
        s.template.as_ref().and_then(|t| t.as_ref().err()).map(String::as_str),
        Some("“Mimari.kcad” bir CAD projesi; kurum şablonu bir CBS projesi olmalı. Kurumun CBS şablon projesini seçin.")
    );
    assert_eq!((s.draft.purpose, s.draft.template.as_deref()), (Purpose::Template, None));
    send(&mut app, Event::Next);
    assert_eq!(app.dialog, Some(Dialog::Project));
}

/// Pictures of the wizard for the owner, dark and light, at 1440 × 900 and
/// 1100 × 650, written to `.run/shots` (never committed); not run by default:
///
/// ```text
/// cargo test -p kentos-desktop project::wizard::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let wanted = |name: &str| {
        only.as_deref()
            .is_none_or(|o| o.split(',').any(|w| name.contains(w)))
    };
    for (w, h) in [(1440.0, 900.0), (1100.0, 650.0)] {
        for mode in ["dark", "light"] {
            let picture = |app: &mut App, name: &str| {
                if !wanted(name) {
                    return;
                }
                let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(app, App::view, &mut update);
                let file = out.join(format!("sihirbaz-{mode}-{}-{name}.png", w as u32));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            };
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            open(&mut app);
            send(&mut app, Event::Type(Workspace::Cad));
            picture(&mut app, "1-tur");
            send(&mut app, Event::Next);
            send(&mut app, Event::Work(BUILDING));
            picture(&mut app, "2-cad-mimari");
            let _ = app
                .settings
                .choose(&[("appearance.textSize", serde_json::Value::from(18))]);
            app.apply_settings();
            picture(&mut app, "2-cad-mimari-buyuk");
            let _ = app
                .settings
                .choose(&[("appearance.textSize", serde_json::Value::from(13))]);
            app.apply_settings();
            send(&mut app, Event::Work(PLAN));
            send(&mut app, Event::Search("trab".into()));
            send(&mut app, Event::SearchFirst);
            picture(&mut app, "3-cad-harita");
            send(&mut app, Event::Back);
            send(&mut app, Event::Type(Workspace::Gis));
            send(&mut app, Event::Next);
            send(&mut app, Event::Search("izm".into()));
            send(&mut app, Event::Province(35));
            picture(&mut app, "4-cbs-izmir");
            send(&mut app, Event::Back);
            send(&mut app, Event::Type(Workspace::Cad));
            send(&mut app, Event::Next);
            send(&mut app, Event::Work(BUILDING));
            send(&mut app, Event::Next);
            send(&mut app, Event::Name("Konut A blok".into()));
            picture(&mut app, "5-ad");
            let _ = app
                .settings
                .choose(&[("appearance.textSize", serde_json::Value::from(18))]);
            app.apply_settings();
            picture(&mut app, "5-ad-buyuk");
            let _ = app
                .settings
                .choose(&[("appearance.textSize", serde_json::Value::from(13))]);
            app.apply_settings();
            send(&mut app, Event::Name(String::new()));
            send(&mut app, Event::Next);
            picture(&mut app, "6-ad-bos");
        }
    }
}

/// The type's axes and scene on screen (docs/adr/0165 §4, §5): a CAD
/// project's X and Y, angles, coordinate axes' icon and dark ground beside a
/// CBS project's Y and X, semts, north and scale bar, in the status bar, the
/// coordinate list, Öznitelikler and the value field, dark and light;
/// `.run/shots/eksenler-*`.
///
/// ```text
/// cargo test -p kentos-desktop project::wizard::tests::axes_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn axes_screens() {
    use iced::{Point, Size};
    use kentos_commands::{Entity, EntityBase, PathEntity, Vec2};
    use kentos_ui::snapshot::Snapshot;

    use crate::bottom::BottomTab;
    use crate::viewport::Event as Pointer;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (kind, mode, name) in [
        (Workspace::Cad, "dark", "cad"),
        (Workspace::Gis, "dark", "cbs"),
        (Workspace::Cad, "light", "cad-acik"),
        (Workspace::Gis, "light", "cbs-acik"),
    ] {
        let mut app = app_with_drawing();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        if kind == Workspace::Cad {
            // A part in millimetres at full size: a building's drawing of its own in the unit the app gives
            // new projects (docs/adr/0802).
            let _ = app
                .settings
                .choose(&[("newProjects.drawingUnit", serde_json::Value::from("mm"))]);
        }
        open(&mut app);
        send(&mut app, Event::Type(kind));
        send(&mut app, Event::Next);
        if kind == Workspace::Gis {
            send(&mut app, Event::Province(35));
        }
        send(&mut app, Event::Next);
        send(&mut app, Event::Next);
        if matches!(app.dialog, Some(Dialog::Unsaved(_))) {
            let _ = app.update(Message::DialogConfirmed);
        }
        // A plate 120 × 80 (mm in CAD, m on the CBS sheet) from the home view's corner.
        let doc = app.document.as_mut().expect("a drawing");
        let home = doc.model.home_view().expect("a home view");
        let (x0, y0, s) = if kind == Workspace::Cad {
            (0.02, 0.02, 0.001)
        } else {
            (
                (home.min_x + home.max_x) / 2.0 - 60.0,
                (home.min_y + home.max_y) / 2.0 - 40.0,
                1.0,
            )
        };
        let pts = [(0.0, 0.0), (120.0, 0.0), (120.0, 80.0), (30.0, 80.0)]
            .map(|(x, y)| Vec2 {
                x: x0 + x * s,
                y: y0 + y * s,
            })
            .to_vec();
        let plate = doc
            .model
            .add(Entity::Polygon(PathEntity {
                space_separator: None,
                ms: None,
                base: EntityBase {
                    look: Default::default(),
                    id: 0,
                    layer_id: doc.model.layers().active().to_owned(),
                    color: None,
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                    line_weight: None,
                    label_pins: Vec::new(),
                    attachments: Vec::new(),
                },
                pts,
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
                closed: false,
                widths: None,
            }))
            .expect("a slot");
        app.selection.set(vec![plate]);
        let _ = app.update(Message::Run("view.zoomSelection"));
        let _ = app.update(Message::BottomTab(BottomTab::Coords));
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // Çizgi running, a value typed with the pointer resting on the drawing.
        let _ = app.update(Message::Run("tool.line"));
        let camera = app.viewport.camera;
        let (x, y) = (camera.width * 0.35, camera.height * 0.3);
        let _ = app.update(Message::Viewport(Pointer::Moved(Point::new(
            x as f32, y as f32,
        ))));
        app.field = Some(crate::input::Field::point(
            "@100<30".into(),
            camera.screen_to_world(x, y),
        ));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("eksenler-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
        if name == "cad-acik" {
            // The ribbon's Görünüm tab: the drawing's ground, “Türe göre” first.
            let _ = app.update(Message::Run("tool.cancel"));
            app.field = None;
            let _ = app.update(Message::RibbonTab("view"));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join("eksenler-gorunum-acik.png");
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// The CBS project's one page for the owner (docs/adr/9184): the page with an empty city map kept in a GeoPackage, a
/// template read, and the first Kaydet asking where; light 1440 × 900 and dark 1100 × 760 into
/// `.run/gui-evidence/kullanim-9184` (never committed).
///
/// ```text
/// cargo test -p kentos-desktop --lib project::wizard::tests::cbs_page_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn cbs_page_screens() {
    use crate::files_testing::usage_shots;

    let page = || {
        let mut app = app_with_drawing();
        open(&mut app);
        send(&mut app, Event::Type(Workspace::Gis));
        send(&mut app, Event::Next);
        send(&mut app, Event::Name("Ankara-Mahalle-Hizmetleri".into()));
        send(&mut app, Event::Purpose(Purpose::City));
        send(&mut app, Event::Search("ank".into()));
        send(&mut app, Event::Province(6));
        send(&mut app, Event::Storage(Storage::Geopackage));
        app
    };
    usage_shots("9184", "1-tek-sayfa", page);
    let dir = scratch("kurum-sablonu-resim");
    let path = dir.join("Belediye şablonu.kcad");
    let mut template = drawing(0);
    let mut s = template.settings().clone();
    s.forms = serde_json::from_value(serde_json::json!([
        { "id": "bina-formu", "name": "Bina formu", "layer": "bina", "draft": { "revision": 1, "items": [] } }
    ]))
    .expect("a form");
    template.model.set_settings(s);
    crate::document::write(&template.model.to_snapshot_v2(), &path).expect("the template is written");
    usage_shots("9184", "2-kurum-sablonu", || {
        let mut app = page();
        app.picker = Picker::File(path.clone());
        send_driven(&mut app, Event::PickTemplate);
        app.picker = Picker::Dialog;
        send(&mut app, Event::Storage(Storage::Database));
        app
    });
    usage_shots("9184", "3-ilk-kaydet", || {
        let mut app = page();
        send(&mut app, Event::Next);
        let _ = app.update(Message::Run("file.save"));
        app
    });
}

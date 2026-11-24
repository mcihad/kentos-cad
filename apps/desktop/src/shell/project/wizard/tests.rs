//! The Yeni proje wizard driven by its events and keys, as the web's smoke run
//! drives its own (apps/web/scripts/e2e/smoke.mjs), and its pictures.

use iced::keyboard::key::Named;
use kentos_commands::{CadWork, DrawingUnit, Workspace};
use kentos_project::wizard::{Coords, Kind, Step};

/// The works' cards (docs/adr/0626 §1, 0802: two CAD works): a building and a plan on real coordinates.
const BUILDING: usize = 0;
const PLAN: usize = 1;

use super::Event;
use crate::app::{App, Dialog, Message, Picker};
use crate::files_testing::{app_with_drawing, drive, last_said, scratch};
use crate::project::{Event as ProjectEvent, Window};

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Project(Box::new(ProjectEvent::New(e))));
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
    // A CBS project's tables are made with the data drivers, as the program registers them at start (launch.rs).
    crate::data_layers::register();
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
    // It opens empty, on its one annotation layer (docs/adr/9302).
    assert_eq!(doc.model.layers().active(), "aciklama");
    // İzmir is 0.14° east of TM27's meridian: its centre lies some 12 km east of 500 000.
    let home = doc.model.home_view().expect("a home view");
    let east = (home.min_x + home.max_x) / 2.0;
    assert!((505_000.0..520_000.0).contains(&east), "{east}");
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

/// A CBS project is decided on one page, its name and its place, and opens empty (docs/adr/9302; the owner's word of 24
/// November 2026: “amaç bölümü gereksiz”, “kaydederken soruyor bu yöntem de güzel”): no layer, nothing written, its
/// drawing tools off saying why until a layer is added; the first Kaydet asks for its file and writes it there.
#[test]
fn a_cbs_project_opens_empty_and_its_first_save_asks_where() {
    let dir = scratch("cbs-bos-proje");
    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Gis));
    send(&mut app, Event::Next);
    let s = state(&app);
    assert_eq!((s.step, s.draft.steps().len()), (Step::Details, 2));
    send(&mut app, Event::Name("Ada 101".into()));
    send(&mut app, Event::Province(58));
    let task = app.update(Message::Project(Box::new(ProjectEvent::New(Event::Next))));
    drive(&mut app, task);
    if matches!(app.dialog, Some(Dialog::Unsaved(_))) {
        let task = app.update(Message::DialogConfirmed);
        drive(&mut app, task);
    }
    let doc = app.document.as_ref().expect("the new project");
    assert_eq!(doc.name(), "Ada 101");
    assert_eq!(doc.settings().srid, 5256, "Sivas's zone");
    // No data or drawing layer: only Açıklama, for texts and notes the project file keeps (a tree holds one at least).
    let leaves: Vec<(&str, Option<kentos_commands::LayerGeometry>)> =
        doc.model.layers().leaves().iter().map(|n| (n.name.as_str(), n.geometry)).collect();
    assert_eq!(leaves, [("Açıklama", Some(kentos_commands::LayerGeometry::Annotation))]);
    assert!(doc.path.is_none() && doc.stored_source().is_none(), "kept nowhere yet");
    assert!(std::fs::read_dir(&dir).map_or(true, |mut d| d.next().is_none()), "nothing written");
    assert!(
        app.log.lines().any(|l| l.text.contains("Veri ekle") && l.text.contains("ilk Kaydet'te sorulur")),
        "{:?}",
        app.log.lines().map(|l| l.text.as_str()).collect::<Vec<_>>()
    );
    // No feature is drawn before a data layer is added: the drawing tools are off and say how; a text is written.
    assert!(!app.available("tool.polygon"));
    assert!(
        app.why_disabled("tool.polygon").is_some_and(|w| w.contains("bir açıklama katmanı") && w.contains("Veri ekle")),
        "{:?}",
        app.why_disabled("tool.polygon")
    );
    assert!(app.available("tool.text"), "{:?}", app.why_disabled("tool.text"));
    // The first Kaydet asks for its file (the dialog's answer given here) and writes it there.
    let file = dir.join("Ada 101.kcad");
    app.picker = Picker::File(file.clone());
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    app.picker = Picker::Dialog;
    assert_eq!(app.document.as_ref().and_then(|d| d.path.as_deref()), Some(file.as_path()));
    assert!(file.exists(), "written");
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

/// The CBS project's one page for the owner (docs/adr/9302): its name, province and system, and what comes after; then
/// the empty project it opens; light 1440 × 900 and dark 1100 × 760 into `.run/gui-evidence/kullanim-9302` (never
/// committed).
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
        send(&mut app, Event::Search("ank".into()));
        send(&mut app, Event::Province(6));
        app
    };
    usage_shots("9302", "1-tek-sayfa", page);
    usage_shots("9302", "2-bos-proje", || {
        let mut app = page();
        let task = app.update(Message::Project(Box::new(ProjectEvent::New(Event::Next))));
        drive(&mut app, task);
        if matches!(app.dialog, Some(Dialog::Unsaved(_))) {
            let task = app.update(Message::DialogConfirmed);
            drive(&mut app, task);
        }
        app.command_expanded = true;
        app
    });
}

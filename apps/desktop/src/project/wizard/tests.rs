//! The Yeni proje wizard driven by its events and keys, as the web's smoke run
//! drives its own (apps/web/scripts/e2e/smoke.mjs), and its pictures.

use iced::keyboard::key::Named;
use kentos_contracts::{DrawingUnit, Workspace};
use kentos_project::wizard::{Coords, Kind, Step};

use super::Event;
use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, last_said};
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
    // A CAD project is local first; real coordinates take a province and a system.
    assert_eq!(state(&app).draft.coords, Coords::Local);
    send(&mut app, Event::Coords(Coords::Real));
    send(&mut app, Event::System(5254));
    send(&mut app, Event::Next);
    assert_eq!(state(&app).step, Step::Details);
    send(&mut app, Event::Name("  Ada 7  ".into()));
    send(&mut app, Event::Scale(500.0));
    send(&mut app, Event::Next);
    assert_eq!(app.dialog, None);
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.name(), "Ada 7");
    let settings = doc.settings();
    assert_eq!(
        (settings.srid, settings.plot_scale, settings.workspace),
        (5254, 500.0, Some(Workspace::Cad))
    );
    assert_eq!(settings.drawing_unit, None);
    // A CAD project starts with technical drawing layers, Çizim active (docs/adr/0165 §3).
    assert_eq!(doc.model.layers().active(), "cizim");
    assert_eq!(doc.entity_count(), 0);
    assert!(doc.path.is_none() && !doc.dirty());
    assert!(
        last_said(&app)
            .starts_with("“Ada 7” yeni projesi açıldı: TUREF / TM30 (EPSG:5254), 1:500."),
        "{}",
        last_said(&app)
    );
}

#[test]
fn a_local_cad_project_is_drawn_in_its_unit_from_one_to_one() {
    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Type(Workspace::Cad));
    send(&mut app, Event::Next);
    send(&mut app, Event::Unit(DrawingUnit::Mm));
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
    // A project with a system keeps the unit asked for before.
    send(&mut app, Event::Go(1));
    send(&mut app, Event::Coords(Coords::Real));
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
    send(&mut app, Event::Next);
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.settings().srid, 5253);
    assert_eq!(doc.model.layers().active(), "taslak");
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
    send(&mut app, Event::Coords(Coords::Real));
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
    assert_eq!(state(&app).step, Step::Coords);
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
fn a_scale_typed_after_one_to_is_the_projects() {
    let mut app = app_with_drawing();
    open(&mut app);
    send(&mut app, Event::Go(2));
    assert_eq!(state(&app).draft.scale(), 1000.0, "CBS: 1:1000");
    send(&mut app, Event::OwnScale("2.500".into()));
    assert_eq!(state(&app).draft.scale(), 2500.0);
    // What is not a whole number over 0 leaves the scale.
    send(&mut app, Event::OwnScale("2.500x".into()));
    assert_eq!(state(&app).draft.scale(), 2500.0);
    assert_eq!(state(&app).own, "2.500x");
    // A chip clears the field; another type goes back to its own scale.
    send(&mut app, Event::Scale(5000.0));
    assert_eq!(
        (state(&app).draft.scale(), state(&app).own.as_str()),
        (5000.0, "")
    );
    send(&mut app, Event::Type(Workspace::Cad));
    assert_eq!(state(&app).draft.scale(), 1.0);
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
            send(&mut app, Event::Unit(DrawingUnit::Mm));
            picture(&mut app, "2-cad-yerel");
            send(&mut app, Event::Coords(Coords::Real));
            send(&mut app, Event::Search("trab".into()));
            send(&mut app, Event::SearchFirst);
            picture(&mut app, "3-cad-gercek");
            send(&mut app, Event::Back);
            send(&mut app, Event::Type(Workspace::Gis));
            send(&mut app, Event::Next);
            send(&mut app, Event::Search("izm".into()));
            send(&mut app, Event::Province(35));
            picture(&mut app, "4-cbs-izmir");
            send(&mut app, Event::Back);
            send(&mut app, Event::Type(Workspace::Cad));
            send(&mut app, Event::Next);
            send(&mut app, Event::Coords(Coords::Local));
            send(&mut app, Event::Next);
            send(&mut app, Event::Name("Mil plakası".into()));
            picture(&mut app, "5-ayrintilar");
            send(&mut app, Event::Name(String::new()));
            send(&mut app, Event::Next);
            picture(&mut app, "6-ad-bos");
        }
    }
}

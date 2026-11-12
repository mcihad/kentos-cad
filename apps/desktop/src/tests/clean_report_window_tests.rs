//! Taşınma raporu (clean_report_window.rs, docs/adr/1262) on the shared cases' drawing of `cad.topology.clean`: the
//! whole drawing reports each move and object, Uygula writes one undo step; a selection corrects only itself; Köşeler
//! joins the parcels' corners and says the areas' change; a quality profile's acceptance threshold disables Uygula
//! and the command refuses it; CSV kaydet writes every move. Harita and GIS offer it, Yol and Mimari do not. And its
//! pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::clean_report_window::Event as R;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.topology.clean.json")).expect("cases")
}

/// The drawing, with the profile of the case named `profile` when given.
fn opened(program: Option<Module>, profile_case: Option<&str>) -> App {
    let cases = cases();
    let setup = match profile_case {
        Some(name) => cases["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|c| c["name"] == name)
            .expect("the case")["setup"]
            .clone(),
        None => cases["setup"].clone(),
    };
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn start_of_b(app: &App) -> (f64, f64) {
    match app.document.as_ref().and_then(|d| d.model.get(Slot(2))) {
        Some(kentos_commands::Entity::Line(l)) => (l.a.x, l.a.y),
        _ => panic!("line B"),
    }
}

fn report(app: &App) -> kentos_commands::TopologyCleaned {
    app.clean_report.plan.clone().expect("a report").expect("reported")
}

#[test]
fn the_whole_drawing_reports_its_moves_and_writes_one_step() {
    let mut app = opened(Some(Module::Map), None);
    let _ = app.update(Message::Run("topology.cleanReport"));
    assert_eq!(app.dialog, Some(Dialog::CleanReport));
    let _ = app.update(Message::CleanReport(R::Tolerance(0.05)));
    let r = report(&app);
    assert_eq!((r.moved, r.changes.len(), r.objects, r.supports), (6, 6, 11, 2));
    assert!((r.max_moved - 0.04).abs() < 1e-9 && r.acceptance.is_none());
    let before = start_of_b(&app);
    let _ = app.update(Message::CleanReport(R::Apply));
    assert_eq!(app.dialog, None);
    assert_eq!(start_of_b(&app), (487010.0, 4420000.0), "B's start on A's end");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(start_of_b(&app), before, "one step");
}

#[test]
fn a_selection_corrects_only_itself_and_koseler_joins_the_parcels() {
    let mut app = opened(Some(Module::Map), None);
    app.selection.set([Slot(2)]);
    let _ = app.update(Message::Run("topology.cleanReport"));
    let _ = app.update(Message::CleanReport(R::Tolerance(0.05)));
    let r = report(&app);
    assert_eq!((r.objects, r.moved), (1, 1));
    app.selection.set([]);
    let _ = app.update(Message::CleanReport(R::Close));
    let _ = app.update(Message::Run("topology.cleanReport"));
    let _ = app.update(Message::CleanReport(R::Tolerance(0.05)));
    let _ = app.update(Message::CleanReport(R::Work(1, true)));
    let r = report(&app);
    assert_eq!((r.counts.vertices, r.moved), (2, 8));
    assert!((r.area_change - 0.2997499967922863).abs() < 1e-9);
}

#[test]
fn past_the_acceptance_threshold_uygula_is_refused() {
    let mut app = opened(Some(Module::Map), Some("uygula: kabul eşiği 0,03 m, aşan taşınma bütün temizliği reddeder"));
    let _ = app.update(Message::Run("topology.cleanReport"));
    let _ = app.update(Message::CleanReport(R::Tolerance(0.05)));
    let r = report(&app);
    assert_eq!((r.acceptance, r.beyond), (Some(0.03), 3));
    let before = start_of_b(&app);
    let _ = app.update(Message::CleanReport(R::Apply));
    assert_eq!(app.dialog, Some(Dialog::CleanReport), "the window stays");
    assert_eq!(start_of_b(&app), before, "nothing written");
}

#[test]
fn csv_kaydet_writes_every_move() {
    let mut app = opened(Some(Module::Map), None);
    let _ = app.update(Message::Run("topology.cleanReport"));
    let _ = app.update(Message::CleanReport(R::Tolerance(0.05)));
    let dir = std::env::temp_dir().join(format!("kentos-tasinma-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("rapor.csv");
    app.clean_report_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 7, "a head and six moves");
    assert!(lines[0].starts_with("Nesne;İş;Y önce;X önce"));
    assert!(lines[1].contains(";uç birleşir;487010.03;4420000.01;487010;4420000;"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn harita_and_gis_offer_it_yol_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Road, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module), None).offers("topology.cleanReport"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/tasinma-raporu-1262/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib clean_report_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tasinma-raporu-1262");
    std::fs::create_dir_all(&out).expect("a folder");
    let strict = "uygula: kabul eşiği 0,03 m, aşan taşınma bütün temizliği reddeder";
    for (profile, stem) in [(None, "1-rapor"), (Some(strict), "2-kabul-esigi")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")]
        {
            let mut app = opened(Some(Module::Map), profile);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("topology.cleanReport"));
            let _ = app.update(Message::CleanReport(R::Tolerance(0.05)));
            let _ = app.update(Message::CleanReport(R::Work(1, true)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

//! Katalog geçiş raporu (catalog_report_window.rs, docs/adr/4940) on the shared cases' drawing of
//! `cad.profiles.compare`, carrying the institution's new catalog: the window opens Harita varlıkları → the project's
//! catalog and gives the shared case's report; the four tables; Karar isteyenleri seç selects the rows' objects; the
//! same catalog on both sides needs no decision; Harita and CBS offer it. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::catalog_report_window::{Event as C, Show};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.profiles.compare.json")).expect("cases")
}

/// The first case: its new catalog and its report.
fn first() -> (Value, Value) {
    let steps = &cases()["cases"][0]["steps"];
    (steps[1]["input"]["toProfile"].clone(), steps[1]["result"]["output"].clone())
}

/// The shared drawing, carrying the new catalog as a project profile (docs/adr/0735).
pub(crate) fn opened(program: Option<Module>) -> App {
    let mut setup = cases()["setup"].clone();
    let (catalog, _) = first();
    setup["settings"]["profileFiles"] =
        serde_json::json!([{ "kind": "profile", "name": "belediye-harita", "text": catalog.to_string() }]);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn report(app: &App) -> Value {
    let mut v = serde_json::to_value(app.catalog_report.answer.clone().expect("an answer").expect("a report")).expect("json");
    v["revision"] = Value::from("$current");
    v
}

#[test]
fn the_window_gives_the_shared_cases_report() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.catalogReport"));
    assert_eq!(app.dialog, Some(Dialog::CatalogReport));
    assert_eq!(app.catalog_report.from, "harita-varliklari");
    assert_eq!(app.catalog_report.to, "belediye-harita");
    assert_eq!(report(&app), first().1);
    {
        let (lines, rows) = app.catalog_report_lines();
        assert_eq!(rows.len(), 12, "a row a class");
        assert!(lines.len() >= 3, "the summary and the notes");
        assert_eq!(rows[0][1], "Bölündü");
    }
    for (show, n) in [(Show::Fields, 7), (Show::Codes, 10), (Show::Decisions, 5)] {
        let _ = app.update(Message::CatalogReport(C::Show(show)));
        let (_, rows) = app.catalog_report_lines();
        assert_eq!(rows.len(), n, "{show}");
    }
}

#[test]
fn karar_isteyenleri_sec_selects_the_rows_objects() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.catalogReport"));
    let _ = app.update(Message::CatalogReport(C::Select));
    let want: usize = first().1["classes"]
        .as_array()
        .expect("classes")
        .iter()
        .filter(|c| c["decide"] == true)
        .map(|c| c["uids"].as_array().map_or(0, Vec::len))
        .sum();
    assert!(want > 0);
    assert_eq!(app.selection.ids().len(), want, "the deciding classes' objects");
}

#[test]
fn the_same_catalog_needs_no_decision() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.catalogReport"));
    let _ = app.update(Message::CatalogReport(C::To("harita-varliklari".into())));
    let r = app.catalog_report.answer.clone().expect("an answer").expect("a report");
    assert_eq!(r.decide, 0);
    assert!(r.fields.is_empty() && r.codes.is_empty());
    let len = app.selection.ids().len();
    let _ = app.update(Message::CatalogReport(C::Select));
    assert_eq!(app.selection.ids().len(), len, "nothing to select");
}

#[test]
fn harita_and_cbs_offer_it() {
    for program in [Module::Map, Module::Gis] {
        let mut app = opened(Some(program));
        let _ = app.update(Message::Run("map.catalogReport"));
        assert_eq!(app.dialog, Some(Dialog::CatalogReport), "{program:?}");
    }
}

/// `.run/gui-evidence/katalog-gecisi-4940/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib catalog_report_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/katalog-gecisi-4940");
    std::fs::create_dir_all(&out).expect("a folder");
    let shoot = |app: &mut App, size: Size, file: &str| {
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(file);
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    };
    let set = |app: &mut App, theme: &str, size: Size| {
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
    };
    let (wide, narrow) = (Size::new(1440.0, 900.0), Size::new(1100.0, 760.0));
    let mut app = opened(Some(Module::Map));
    set(&mut app, "light", wide);
    let _ = app.update(Message::Run("map.catalogReport"));
    shoot(&mut app, wide, "1-siniflar-acik-genis.png");
    let _ = app.update(Message::CatalogReport(C::Show(Show::Fields)));
    shoot(&mut app, wide, "2-alanlar-acik-genis.png");
    let _ = app.update(Message::CatalogReport(C::Show(Show::Codes)));
    shoot(&mut app, wide, "3-kodlar-acik-genis.png");
    let mut app = opened(Some(Module::Map));
    set(&mut app, "dark", narrow);
    let _ = app.update(Message::Run("map.catalogReport"));
    let _ = app.update(Message::CatalogReport(C::Show(Show::Decisions)));
    shoot(&mut app, narrow, "4-kararlar-koyu-dar.png");
    // Its place in the ribbon: Ölçme › Noktalar' ▾ (beside Detay eşleştirme), in map work.
    let mut app = opened(Some(Module::Map));
    set(&mut app, "light", wide);
    let _ = app.update(Message::RibbonTab("survey"));
    shoot(&mut app, wide, "5-serit-olcme-acik-genis.png");
}

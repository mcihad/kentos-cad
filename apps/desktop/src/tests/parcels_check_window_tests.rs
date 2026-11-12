//! Parsel alan denetimi (parcels_check_window.rs, docs/adr/1400) on the shared cases' drawing of `cad.parcels.check`:
//! the parcel layer's areas checked (within, outside, no record, unreadable; the duplicate identity), the choice
//! filters the table, Dışındakileri seç selects the one outside, CSV kaydet writes the table, nothing is written to
//! the drawing; a selection checks only its areas. Harita and GIS offer it, Yol and Mimari do not. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::parcels_check_window::{Event as C, Show};

fn opened(program: Option<Module>) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.check.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn the_parcel_layer_is_checked_and_the_ones_outside_selected() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("parcels.check"));
    assert_eq!(app.dialog, Some(Dialog::ParcelsCheck));
    let r = app.parcels_check.answer.clone().expect("an answer").expect("checked");
    assert_eq!((r.parcels.len(), r.within, r.outside, r.no_record, r.invalid_record), (7, 3, 1, 1, 2));
    assert_eq!(r.duplicates, ["Ankara/Çankaya/Merkez/101/2"]);
    let _ = app.update(Message::ParcelsCheck(C::Show(Show::Outside)));
    assert_eq!(app.parcels_check.show, Show::Outside);
    let _ = app.update(Message::ParcelsCheck(C::Select));
    assert_eq!(app.dialog, None);
    assert_eq!(app.selection.ids(), [Slot(2)], "Ada 101 Parsel 2, 10 m² over ±6.8");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn a_selection_checks_its_areas_only() {
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(1), Slot(8)]);
    let _ = app.update(Message::Run("parcels.check"));
    let r = app.parcels_check.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.parcels.len(), 1, "the line is no area");
    assert_eq!(r.parcels[0].status, "within");
}

#[test]
fn csv_kaydet_writes_the_table() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("parcels.check"));
    let dir = std::env::temp_dir().join(format!("kentos-parsel-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("denetim.csv");
    app.parcels_check_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 8, "a head and seven parcels");
    assert!(lines[1].contains(";Ankara;Çankaya;Merkez;101;1;") && lines[1].contains(";301,2;"), "{}", lines[1]);
    assert!(lines[1].ends_with(";İçinde;") && lines[2].ends_with(";Dışında;evet"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn harita_and_gis_offer_it_yol_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Road, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("parcels.check"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/parsel-alan-1400/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcels_check_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/parsel-alan-1400");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("parcels.check"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

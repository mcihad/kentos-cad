//! Düzenleme dağıtım taslağı (readjustment_window.rs, docs/adr/1460): the rule package typed, a CSV of synthetic
//! records read, the shared cases' first answer reproduced (totals kept, the remainder's receiver said), a refusal
//! shown with its place, nothing written; Harita and GIS offer it, Yol and Mimari do not. And its pictures.

use kentos_launch::Module;

use crate::app::{App, Dialog, Message};
use crate::readjustment_window::{Event as R, View, records};

fn opened(program: Option<Module>) -> App {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.readjustment.distribute.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

const CSV: &str = "parsel;hak sahibi;tapu alanı;pay\n101/1;H1;1000;1/3\n101/1;H2;1000;2/3\n101/2;H2;500.50;1/2\n101/2;H3;500.50;1/2\nP;K1;100.01;1/3\nP;K2;100.01;1/3\nP;K3;100.01;1/3\n";

fn filled(app: &mut App) {
    for e in [
        R::Id("ornek-paket".into()),
        R::Jurisdiction("Örnek kurum (sentetik)".into()),
        R::Source("Sentetik deneme belgesi".into()),
        R::Effective("2026-01-01".into()),
        R::Deduction("0.40".into()),
    ] {
        let _ = app.update(Message::Readjustment(e));
    }
    let dir = std::env::temp_dir().join(format!("kentos-duzenleme-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("kayitlar.csv");
    std::fs::write(&file, CSV).expect("written");
    app.readjustment_file(file);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_records_are_read_and_the_totals_kept() {
    assert_eq!(records(CSV).expect("reads").len(), 7);
    assert!(records("a;b;c").is_err());
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("readjustment.distribute"));
    assert_eq!(app.dialog, Some(Dialog::Readjustment));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    filled(&mut app);
    let a = app.readjustment.answer.clone().expect("an answer").expect("distributed");
    assert!(a.draft && a.conserved);
    assert_eq!((a.registered.as_str(), a.deduction.as_str(), a.net.as_str()), ("1600.51", "640.20", "960.31"));
    let p = a.parcels.iter().find(|p| p.parcel == "P").expect("P");
    assert_eq!((p.remainder.as_str(), p.remainder_to.as_slice()), ("0.01", &["K1".to_owned()][..]));
    // A rule that refuses a remainder says so, with its place.
    let _ = app.update(Message::Readjustment(R::Remainder("reject".into())));
    let why = app.readjustment.answer.clone().expect("an answer").expect_err("refused");
    assert!(why.contains("reject") && why.contains("entries[4].share"), "{why}");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn harita_and_gis_offer_it_yol_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Road, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("readjustment.distribute"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/duzenleme-taslagi-1460/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib readjustment_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/duzenleme-taslagi-1460");
    std::fs::create_dir_all(&out).expect("a folder");
    for (view, stem) in [(View::Holders, "1-hak-sahipleri"), (View::Parcels, "2-parseller")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("readjustment.distribute"));
            filled(&mut app);
            let _ = app.update(Message::Readjustment(R::View(view)));
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

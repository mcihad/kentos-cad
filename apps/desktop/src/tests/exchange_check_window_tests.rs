//! Dışa aktarma kaybı denetimi (exchange_check_window.rs, docs/adr/1660) on fixtures/exchange/v1/devir.json: GeoPackage keeps ids,
//! arcs, heights and measures (the text is not written, the circle comes back an area); DXF loses the measures; a
//! tolerance that is no number is said; Kayıplıları seç selects the objects the losses name; nothing is written to
//! the drawing. Every module offers it from Dosya › Dışa aktar. On fixtures/exchange/v1/devir-blok.json the matrix's
//! Bloklar, Tablolar and İlişkiler rows (docs/adr/2380). And its pictures.

use kentos_launch::Module;

use crate::app::{App, Dialog, Message};
use crate::exchange_check_window::Event as X;

fn opened(program: Option<Module>) -> App {
    opened_with(program, include_str!("../../../../fixtures/exchange/v1/devir.json"))
}

fn opened_with(program: Option<Module>, drawing: &str) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(drawing).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn a_geopackage_and_a_dxf_trip() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("file.exchangeCheck"));
    assert_eq!(app.dialog, Some(Dialog::ExchangeCheck));
    assert!(app.exchange_check.answer.is_none(), "Denetle runs it");
    let _ = app.update(Message::ExchangeCheck(X::Run));
    let r = app.exchange_check.answer.clone().expect("an answer").expect("checked");
    assert_eq!((r.objects, r.matched), (8, 7));
    assert!(r.losses.iter().all(|l| l.kind != "identityLost" && l.kind != "mLost"), "{:?}", r.losses);
    let _ = app.update(Message::ExchangeCheck(X::Format("dxf".into())));
    let _ = app.update(Message::ExchangeCheck(X::Run));
    let r = app.exchange_check.answer.clone().expect("an answer").expect("checked");
    assert!(r.losses.iter().any(|l| l.kind == "mLost"), "{:?}", r.losses);
    let _ = app.update(Message::ExchangeCheck(X::Select));
    assert_eq!(app.dialog, None);
    assert!(!app.selection.ids().is_empty());
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn a_tolerance_that_is_no_number_is_said() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("file.exchangeCheck"));
    let _ = app.update(Message::ExchangeCheck(X::Tolerance("bir".into())));
    let _ = app.update(Message::ExchangeCheck(X::Run));
    assert_eq!(app.exchange_check.answer, Some(Err("Tolerans “bir” bir sayı değil.".into())));
}

#[test]
fn every_module_offers_it() {
    for module in [Module::Map, Module::Gis, Module::Road, Module::Architecture] {
        assert!(opened(Some(module)).offers("file.exchangeCheck"), "{module:?}");
    }
}

/// `.run/gui-evidence/aktarma-kaybi-1660/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib exchange_check_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/aktarma-kaybi-1660");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name, format) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis", "parquet"),
        ("dark", Size::new(1100.0, 760.0), "koyu-dar", "dxf"),
    ] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("file.exchangeCheck"));
        let _ = app.update(Message::ExchangeCheck(X::Format(format.into())));
        let _ = app.update(Message::ExchangeCheck(X::Run));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-{format}-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/1880: Tüm biçimler shows the support matrix of all six formats (the text lost but in DXF, ids kept only by
/// GeoPackage and GeoParquet); Denetle on one format brings its losses back; nothing is written.
#[test]
fn all_formats_give_the_support_matrix() {
    use kentos_commands::{ExchangeFormat as F, ExchangeState as S};
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("file.exchangeCheck"));
    let _ = app.update(Message::ExchangeCheck(X::Matrix));
    let m = app.exchange_check.matrix.clone().expect("a matrix").expect("checked");
    assert_eq!(m.checks.len(), 6);
    let cell = |aspect: &str, f: F| m.rows.iter().find(|r| r.aspect == aspect).and_then(|r| r.cells.iter().find(|c| c.format == f)).map(|c| c.state);
    assert_eq!(cell("objects", F::Gpkg), Some(S::Lost));
    assert_eq!(cell("objects", F::Dxf), Some(S::Kept));
    assert_eq!(cell("identity", F::Gpkg), Some(S::Kept));
    assert_eq!(cell("identity", F::Shp), Some(S::Lost));
    assert_eq!(
        crate::exchange_check_window::cell_words("objects", &m.rows[0].cells[0]),
        "Kaybolur: yazı 1",
        "the words of a cell"
    );
    let _ = app.update(Message::ExchangeCheck(X::Run));
    assert!(app.exchange_check.matrix.is_none() && app.exchange_check.answer.is_some());
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

/// The matrix; `.run/gui-evidence/destek-matrisi-1880/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn matrix_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/destek-matrisi-1880");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("file.exchangeCheck"));
        let _ = app.update(Message::ExchangeCheck(X::Matrix));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-matris-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

const BLOCK_DRAWING: &str = include_str!("../../../../fixtures/exchange/v1/devir-blok.json");

/// docs/adr/2380: on the drawing with a block, a table and a relation, Tüm biçimler's Bloklar and Tablolar rows keep
/// them in DXF alone and İlişkiler in GeoPackage alone, in the cells' words; nothing is written.
#[test]
fn the_matrix_shows_blocks_tables_and_relations() {
    use kentos_commands::{ExchangeFormat as F, ExchangeState as S};
    use crate::exchange_check_window::cell_words;
    let mut app = opened_with(Some(Module::Map), BLOCK_DRAWING);
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("file.exchangeCheck"));
    let _ = app.update(Message::ExchangeCheck(X::Matrix));
    let m = app.exchange_check.matrix.clone().expect("a matrix").expect("checked");
    let cell = |aspect: &str, f: F| {
        m.rows.iter().find(|r| r.aspect == aspect).and_then(|r| r.cells.iter().find(|c| c.format == f)).cloned().expect(aspect)
    };
    for f in F::ALL {
        assert_eq!(cell("blocks", f).state, if f == F::Dxf { S::Kept } else { S::Lost }, "{f:?}");
        assert_eq!(cell("tables", f).state, if f == F::Dxf { S::Kept } else { S::Lost }, "{f:?}");
        assert_eq!(cell("relations", f).state, if f == F::Gpkg { S::Kept } else { S::Lost }, "{f:?}");
    }
    assert_eq!(cell_words("blocks", &cell("blocks", F::Gpkg)), "Kaybolur: blok 1");
    assert_eq!(cell_words("tables", &cell("tables", F::Shp)), "Kaybolur: tablo 1");
    assert_eq!(cell_words("relations", &cell("relations", F::Dxf)), "Kaybolur: Parselin malikleri");
    assert_eq!(cell_words("relations", &cell("relations", F::Gpkg)), "Korunur");
    assert_eq!(crate::exchange_check_window::aspect_word("relations"), "İlişkiler");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

/// `.run/gui-evidence/blok-tablo-iliski-2380/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib exchange_check_window_tests::block_matrix_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn block_matrix_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/blok-tablo-iliski-2380");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened_with(Some(Module::Map), BLOCK_DRAWING);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("file.exchangeCheck"));
        let _ = app.update(Message::ExchangeCheck(X::Matrix));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-matris-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

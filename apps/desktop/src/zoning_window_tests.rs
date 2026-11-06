//! İmar planı denetimi (zoning_window.rs, docs/adr/1420) on the shared cases' drawings of `cad.zoning.check`: the
//! messy plan's sixteen findings, the Alan group's four, Seç ve göster selecting their objects; the clean plan none;
//! a drawing without the profile's layers offered Profili uygula, after which the layers are there. Harita and GIS
//! offer it, Yol and Mimari do not. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::zoning_window::{Event as Z, Group};

fn opened(program: Option<Module>, case: &str) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.zoning.check.json")).expect("cases");
    let setup = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == case).expect("the case")["setup"]
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

const MESSY: &str = "karışık plan: her bulgu türü, türün sırasıyla";
const GOOD: &str = "temiz plan: bulgu yok";
const BARE: &str = "profil katmanları yoksa söylenir";

#[test]
fn the_messy_plan_is_found_out_and_its_areas_shown() {
    let mut app = opened(Some(Module::Map), MESSY);
    let _ = app.update(Message::Run("zoning.check"));
    assert_eq!(app.dialog, Some(Dialog::Zoning));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!((r.plans, r.decisions, r.notes, r.findings.len()), (4, 7, 3, 16));
    let _ = app.update(Message::Zoning(Z::Group(Group::Areas)));
    let _ = app.update(Message::Zoning(Z::Show));
    assert_eq!(app.dialog, None);
    // UIP-1 (uncovered), K1, K2 and K5 (the two overlaps, K5 outside).
    assert_eq!(app.selection.ids(), [Slot(5), Slot(9), Slot(6), Slot(1)]);
}

#[test]
fn the_clean_plan_has_no_finding() {
    let mut app = opened(Some(Module::Map), GOOD);
    let _ = app.update(Message::Run("zoning.check"));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert!(r.findings.is_empty(), "{:?}", r.findings);
}

#[test]
fn profili_uygula_adds_the_missing_layers() {
    let mut app = opened(Some(Module::Map), BARE);
    let _ = app.update(Message::Run("zoning.check"));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.counts.get("missingLayer"), Some(&2));
    let _ = app.update(Message::Zoning(Z::ApplyProfile));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.counts.get("missingLayer"), None, "{:?}", r.findings);
    let _ = app.update(Message::Run("edit.undo"));
    let _ = app.update(Message::Zoning(Z::Tolerance(0.01)));
    let r = app.zoning.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.counts.get("missingLayer"), Some(&2), "one undo step");
}

#[test]
fn harita_and_gis_offer_it_yol_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, true), (Module::Road, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module), GOOD).offers("zoning.check"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/imar-denetimi-1420/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zoning_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/imar-denetimi-1420");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map), MESSY);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("zoning.check"));
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

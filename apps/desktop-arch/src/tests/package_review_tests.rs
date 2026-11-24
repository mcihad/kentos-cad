//! Gelen paketi incele (docs/adr/8104; plan/bim-plan.md BIM-18's first acceptance) worked by hand: the package broken
//! on purpose (fixtures/bim/v1/paket/bozuk: the model on TM33, the east wall without its fire rating, the ground floor's
//! plan from the drawing's revision 8) reviewed against the administration's EPSG:5256; each layer says what failed; the
//! fire rating's finding opens the model in the IFC viewer with the wall chosen; the return note is written as BCF and
//! words. KentOS's own package holds on every layer.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib package_review_tests::screens -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use crate::arch::ArchState;

use kentos_launch::Module;

use crate::app::{App, Message, Picker};
use crate::files_testing::{drive, scratch};
use crate::package_review_window::{Event, Outcome, Tab};
use crate::recovery::Recovery;
use crate::settings::Settings;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.bcf.read.json");
const WALL: &str = "01alMXGa9m080000000002";

fn fixture(rel: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(rel).canonicalize().expect(rel).display().to_string()
}

fn send(app: &mut App, e: Event) {
    let task = app.update(Message::from(crate::arch::Event::ReviewPackage(e)));
    drive(app, task);
}

fn opened() -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("the cases read");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn reviewed(app: &mut App, package: &str) -> std::sync::Arc<kentos_commands::PackageReviewed> {
    let _ = app.update(Message::Run("arch.reviewPackage"));
    send(app, Event::Path(fixture(package)));
    send(app, Event::System("5256".into()));
    send(app, Event::Review);
    match app.modules.arch().review_window.as_ref().and_then(|w| w.outcome.clone()) {
        Some(Outcome::Done(r)) => r,
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_broken_package_is_found_out_and_its_wall_shown_and_returned() {
    let mut app = opened();
    let r = reviewed(&mut app, "bim/v1/paket/bozuk");
    let failed: Vec<&str> = r.layers.iter().filter(|l| l.holds == Some(false)).map(|l| l.key.as_str()).collect();
    assert_eq!(failed, ["pafta", "model", "konum", "form"]);
    // The fire rating's finding: chosen, shown on its wall.
    let k = r.findings.iter().position(|f| f.layer == "model" && f.objects.iter().any(|g| g == WALL)).expect("on the wall");
    send(&mut app, Event::Choose(k));
    send(&mut app, Event::Show);
    let w = app.ifc_viewer.as_ref().expect("the viewer open");
    let view = w.viewer.view().expect("the model read");
    let chosen = view.selected().expect("an object chosen");
    assert_eq!(view.input().objects[chosen].key, WALL);
    // The return note: BCF and words.
    let _ = app.update(Message::Run("arch.reviewPackage"));
    send(&mut app, Event::Author("inceleyen@belediye".into()));
    let path = scratch("iade").join("iade-notu.bcf");
    app.picker = Picker::File(path.clone());
    send(&mut app, Event::Return);
    let said = app.modules.arch().review_window.as_ref().and_then(|w| w.said.clone()).expect("said");
    assert!(said.1.starts_with("İade notu"), "{said:?}");
    let words = std::fs::read_to_string(path.with_extension("txt")).expect("the words");
    assert!(words.contains(WALL) && words.contains("EPSG:5256"), "{words}");
    let bcf = kentos_ifc::bcf::read(&std::fs::read(&path).expect("the BCF")).expect("reads");
    assert!(bcf.topics.iter().any(|t| t.viewpoints.iter().any(|v| v.selection.iter().any(|g| g == WALL))));
}

#[test]
fn kentos_s_own_package_holds_on_every_layer() {
    let mut app = opened();
    let r = reviewed(&mut app, "bim/v1/paket/kentos");
    assert!(r.layers.iter().all(|l| l.holds == Some(true)), "{:?}", r.layers);
    send(&mut app, Event::Return);
    let said = app.modules.arch().review_window.as_ref().and_then(|w| w.said.clone()).expect("said");
    assert!(said.1.contains("iade gerekçesi yazılmaz"), "{said:?}");
}

/// Another discipline's model (docs/adr/8114): the section building's structural model by another program, named by
/// EK-2 for the structural discipline, opened alone: its form is EK-9 Table 9.2's 16 questions, said in the summary.
fn structural(app: &mut App) -> std::sync::Arc<kentos_commands::PackageReviewed> {
    let file = scratch("statik-92").join("260123_00_ST_XXXX_MD_XXX_NNN_01_000.ifc");
    std::fs::copy(fixture("ifc/v1/statik/kesit-binasi-statik-datum.ifc"), &file).expect("the model copied");
    let _ = app.update(Message::Run("arch.reviewPackage"));
    send(app, Event::Path(file.display().to_string()));
    send(app, Event::System("5256".into()));
    send(app, Event::Review);
    match app.modules.arch().review_window.as_ref().and_then(|w| w.outcome.clone()) {
        Some(Outcome::Done(r)) => r,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_structural_model_is_reviewed_by_table_9_2() {
    let mut app = opened();
    let r = structural(&mut app);
    assert_eq!((r.table.as_deref(), r.rows.len()), (Some("9.2"), 16));
    assert!(r.findings.iter().any(|f| f.layer == "konum"), "{:?}", r.findings);
}

/// The structural model's form side (`.run/gui-evidence/tablo-9-2-8114/`), light and dark.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn structural_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tablo-9-2-8114");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let _ = structural(&mut app);
        send(&mut app, Event::Tab(Tab::Form));
        app.toasts.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-statik-form-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The window's pictures (`.run/gui-evidence/paket-inceleme-8104/`), light and dark: the broken package's findings,
/// its form side by side, the wall shown in the IFC viewer, KentOS's own package, the place on the ribbon.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/paket-inceleme-8104");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let shot = |app: &mut App, snapshot: &mut Snapshot, file: &str| {
            app.toasts.clear();
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let r = reviewed(&mut app, "bim/v1/paket/bozuk");
        send(&mut app, Event::Author("inceleyen@belediye".into()));
        let k = r.findings.iter().position(|f| f.layer == "model" && f.objects.iter().any(|g| g == WALL)).unwrap_or(0);
        send(&mut app, Event::Choose(k));
        shot(&mut app, &mut snapshot, "1-bozuk-paket");
        send(&mut app, Event::Tab(Tab::Form));
        shot(&mut app, &mut snapshot, "2-form");
        send(&mut app, Event::Tab(Tab::Findings));
        send(&mut app, Event::Show);
        // The viewer's own window: the model with the wall chosen and painted.
        snapshot.settle(&mut app, App::ifc_viewer_shown, &mut update);
        snapshot.settle(&mut app, App::ifc_viewer_shown, &mut update);
        let file = out.join(format!("3-duvar-modelde-{name}.png"));
        snapshot.render(app.ifc_viewer_shown(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        app.ifc_viewer_gone();
        let _ = reviewed(&mut app, "bim/v1/paket/kentos");
        shot(&mut app, &mut snapshot, "4-kentos-paketi");
        send(&mut app, Event::Close);
        let _ = app.update(Message::RibbonSearch("Gelen paketi incele".into()));
        let _ = app.update(Message::RibbonSearchReveal(0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(&mut app, App::view, &mut update, Input::Click(iced::Point::new(1278.0, 133.0)));
        shot(&mut app, &mut snapshot, "5-serit-yeri");
    }
}

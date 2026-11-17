//! Denetim of Altlık modeller (docs/adr/8102; plan/bim-plan.md BIM-17, B19) worked by hand on the shared cases' drawing
//! (fixtures/commands/v1/cad.underlay.check.json: two 30/30 columns at (0, 0) and (6, 0), a 20 cm wall from (0, 5) to
//! (6, 5) and grids A, B, 1, 2 on TUREF / TM36, ±0.00 at 1285.40 m above Antalya height) with a structural model
//! IfcOpenShell wrote beneath it:
//!
//! - **Aligned:** Denetle says both columns stand where the drawing's do and nothing clashes; the beam whose ends touch
//!   the columns is the file's alone.
//! - **Shifted:** the column moved 20 cm, the beam through the wall, the member inside it and the slab sunk 2 cm on the
//!   columns are clashes, the clashes' table first; 1. Kat's +0,150, B's 0,100 and 2's turn are marked. A clash chosen
//!   selects the drawing's column and shows the file's under Seçilen nesne.
//! - **Another height datum:** Denetle says why and checks nothing.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib underlay_check_tests::screens -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use kentos_launch::Module;

use crate::app::{App, Message, Picker};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;
use crate::underlay_check_view::{Outcome, Tab};
use crate::underlays_window::Event;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.underlay.check.json");

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").canonicalize().expect("the fixtures")
}

fn send(app: &mut App, e: Event) {
    let task = app.update(Message::Arch(crate::arch::Event::Underlays(e)));
    drive(app, task);
}

/// The shared cases' drawing with no underlay, opened in Mimari.
fn opened() -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("the cases read");
    let mut setup = cases["setup"].clone();
    setup["settings"]["arch"].as_object_mut().expect("the architecture settings").remove("underlays");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Altlık modeller open and the structural file `name` added through Ekle…, its read waited for.
fn added(app: &mut App, name: &str) {
    if app.arch.underlays_window.is_none() {
        let _ = app.update(Message::Run("arch.underlays"));
    }
    app.picker = Picker::File(fixtures().join("ifc/v1/statik").join(name));
    send(app, Event::Add);
    let task = app.follow_underlays();
    drive(app, task);
}

fn outcome(app: &App) -> Outcome {
    let w = app.arch.underlays_window.as_ref().expect("the window");
    w.check.outcome.clone().expect("checked").1
}

#[test]
fn the_aligned_model_matches_and_a_shifted_one_is_found_out() {
    let mut app = opened();
    added(&mut app, "denetim-hizali.ifc");
    send(&mut app, Event::Check);
    let Outcome::Done { checked, warnings } = outcome(&app) else { panic!("{:?}", outcome(&app)) };
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!((checked.matched, checked.clashes.len(), checked.unmatched.len()), (2, 0, 1));
    assert_eq!(app.arch.underlays_window.as_ref().unwrap().check.tab, Tab::Levels);

    // The shifted model: its clashes first; one chosen selects the drawing's column and shows the file's.
    send(&mut app, Event::Remove);
    added(&mut app, "denetim-kaymis.ifc");
    send(&mut app, Event::Tolerance("0,001".into()));
    send(&mut app, Event::Check);
    let Outcome::Done { checked, .. } = outcome(&app) else { panic!("{:?}", outcome(&app)) };
    let pairs: Vec<(&str, &str)> = checked.clashes.iter().map(|c| (c.class.as_str(), c.object_class.as_str())).collect();
    assert_eq!(
        pairs,
        [("IfcColumn", "IfcColumn"), ("IfcWall", "IfcBeam"), ("IfcWall", "IfcMember"), ("IfcColumn", "IfcSlab"), ("IfcColumn", "IfcSlab")]
    );
    let summary = crate::underlay_check_view::summary(&checked);
    assert!(summary.contains("5 çakışma") && summary.contains("1 kat ve 2 aks farklı"), "{summary}");
    assert_eq!(app.arch.underlays_window.as_ref().unwrap().check.tab, Tab::Clashes);
    send(&mut app, Event::Clash(0));
    let doc = app.document.as_ref().expect("a drawing");
    let chosen: Vec<String> = app.selection.ids().iter().filter_map(|s| doc.model.uid(*s)).map(|u| u.to_string()).collect();
    assert_eq!(chosen, [checked.clashes[0].uid.clone()]);
    let w = app.arch.underlays_window.as_ref().unwrap();
    let (_, object) = w.object.clone().expect("the file's object shown");
    let read = match app.arch.underlays.states.values().next().map(|s| &s.1) {
        Some(crate::underlays::State::Ready(l)) => l.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(read.model.objects[object].global_id, checked.clashes[0].object);
}

#[test]
fn a_tolerance_that_is_no_number_and_another_height_datum_are_said() {
    let mut app = opened();
    added(&mut app, "denetim-hizali.ifc");
    send(&mut app, Event::Tolerance("bir mm".into()));
    send(&mut app, Event::Check);
    let Outcome::Refused(why) = outcome(&app) else { panic!("{:?}", outcome(&app)) };
    assert!(why.contains("bir sayı değil"), "{why}");
    // The datum file is refused at Ekle…: the window says why, and nothing is added to check.
    added(&mut app, "denetim-datum.ifc");
    let w = app.arch.underlays_window.as_ref().unwrap();
    let (_, why) = w.unplaced.clone().expect("said");
    assert!(why.contains("ITRF96"), "{why}");
}

/// The window's pictures (`.run/gui-evidence/altlik-denetim-8102/`), light and dark: the aligned model checked; the
/// shifted one's clashes with one chosen, its levels and its axes; the plan with the shifted model under the drawing.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/altlik-denetim-8102");
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
        added(&mut app, "denetim-hizali.ifc");
        send(&mut app, Event::Check);
        shot(&mut app, &mut snapshot, "1-hizali");

        send(&mut app, Event::Remove);
        added(&mut app, "denetim-kaymis.ifc");
        send(&mut app, Event::Check);
        send(&mut app, Event::Clash(1));
        shot(&mut app, &mut snapshot, "2-cakismalar");
        send(&mut app, Event::Tab(Tab::Levels));
        shot(&mut app, &mut snapshot, "3-katlar");
        send(&mut app, Event::Tab(Tab::Axes));
        shot(&mut app, &mut snapshot, "4-akslar");
        send(&mut app, Event::Tab(Tab::Unmatched));
        shot(&mut app, &mut snapshot, "5-yalniz-birinde");

        // The plan: the shifted model under the drawing, the wall's beam and the moved column in sight.
        send(&mut app, Event::Close);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, &mut snapshot, "6-plan");
    }
}

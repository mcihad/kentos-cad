//! Çizimden boykesit (road_profile_fit_window.rs, plan/road.md Y10.6b): the window reads the profile as the view is
//! typed, makes it as one undo step; and its pictures.

use kentos_domain::Slot;

use crate::app::{App, Dialog, Message};
use crate::road_profile_fit_window::Event as W;

/// The drawn grades, the crest arc and the sag arc of `road.profiles.fit`'s drawing (after its 120 terrain points).
const GRADES: Slot = Slot(121);
const CREST: Slot = Slot(123);
const SAG: Slot = Slot(124);

/// A road project with `road.profiles.fit`'s drawing: alignments, a profile and the lines drawn in a profile view.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.profiles.fit.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::RoadProfileFit(e));
}

/// The view of the drawing: its datum 90 m at (0; −500), a metre of elevation 10 units.
fn view(app: &mut App) {
    send(app, W::Field(0, "0; -500".into()));
    send(app, W::Field(2, "90".into()));
    send(app, W::Field(3, "10".into()));
}

#[test]
fn the_window_makes_the_profile_as_one_step() {
    let mut app = app();
    let _ = app.update(Message::Run("profile.fromLine"));
    assert!(app.road_profile_fit_window.is_none(), "nothing selected");
    app.selection.set([GRADES]);
    let _ = app.update(Message::Run("profile.fromLine"));
    assert_eq!(app.dialog, Some(Dialog::RoadProfileFit));
    let why = app.road_profile_fit_window.as_ref().and_then(|w| w.preview.why.clone()).expect("no datum yet");
    assert!(why.contains("kıyas kotu"), "{why}");
    view(&mut app);
    send(&mut app, W::Field(4, "60".into()));
    let w = app.road_profile_fit_window.as_ref().expect("open");
    let fitted = w.preview.fitted.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert_eq!(fitted.pvis.len(), 4);
    assert!((fitted.pvis[1].station - 150.0).abs() < 1e-9 && (fitted.pvis[1].elevation - 104.0).abs() < 1e-9);
    assert_eq!(fitted.curves, 2);
    send(&mut app, W::Field(4, "400".into()));
    let why = app.road_profile_fit_window.as_ref().and_then(|w| w.preview.why.clone()).expect("refused");
    assert!(why.contains("sığmıyor"), "{why}");
    send(&mut app, W::Field(4, String::new()));
    let before = app.document.as_ref().expect("a drawing").model.road_profiles().len();
    send(&mut app, W::Make);
    assert!(app.road_profile_fit_window.is_none());
    assert_eq!(app.document.as_ref().expect("a drawing").model.road_profiles().len(), before + 1);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().expect("a drawing").model.road_profiles().len(), before);
}

/// The window on drawn grades and on a crest and a sag arc, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y10-cizimden-boykesit-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_profile_fit_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y10-cizimden-boykesit-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        for (slot, file, length, tolerance) in [(GRADES, "egim", "60", ""), (CREST, "tepe", "", "0.05"), (SAG, "cukur", "", "")] {
            app.selection.set([slot]);
            let _ = app.update(Message::Run("profile.fromLine"));
            view(&mut app);
            send(&mut app, W::Field(4, length.into()));
            send(&mut app, W::Field(6, tolerance.into()));
            shot(&mut app, file);
            send(&mut app, W::Close);
        }
    }
}

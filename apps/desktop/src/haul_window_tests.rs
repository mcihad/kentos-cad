//! Brükner (haul_window.rs, plan/road.md Y9): the window reads the diagram as it is typed and writes through
//! `road.hauls.edit` (one undo step); and its pictures.

use crate::app::{App, Dialog, Message};
use crate::haul_window::Event as W;

/// A road project with `road.hauls.query`'s drawing: two corridors, two volumes and two diagrams.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.hauls.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Hauls(e));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

#[test]
fn the_window_makes_a_diagram_with_a_site_and_reads_it() {
    let mut app = app();
    let _ = app.update(Message::Run("haul.new"));
    assert_eq!(app.dialog, Some(Dialog::Hauls));
    send(&mut app, W::Name("Döküm Brükneri".into()));
    send(&mut app, W::Field(1, "200".into()));
    send(&mut app, W::Add);
    send(&mut app, W::Site(0, 2, "350".into()));
    send(&mut app, W::Site(0, 3, "9000".into()));
    let w = app.haul_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert!(rep.points.windows(2).any(|p| p[0][0] == 350.0 && p[1][0] == 350.0), "the site's step");
    assert!(rep.hauled > 1000.0 && rep.moment > 0.0, "{rep:?}");
    send(&mut app, W::Save);
    assert!(app.haul_window.is_none(), "Kaydet closes the window");
    let doc = model(&mut app);
    assert_eq!(doc.road_hauls().len(), 3);
    assert_eq!(doc.road_hauls()[2].free, Some(200.0));
    assert_eq!(doc.undo().as_deref(), Some("Brükner"));
}

#[test]
fn prices_and_a_payload_give_the_cost_and_are_written() {
    let mut app = app();
    let _ = app.update(Message::Run("haul.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Field(1, "100".into()));
    send(&mut app, W::Field(3, "12,5".into()));
    assert!(app.haul_window.as_ref().is_some_and(|w| w.preview.why.is_some()), "a price wants a number in the dot");
    send(&mut app, W::Field(3, "12.5".into()));
    assert!(
        app.haul_window.as_ref().and_then(|w| w.preview.why.as_deref()).is_some_and(|e| e.contains("para birimi")),
        "a price wants a currency"
    );
    send(&mut app, W::Field(2, "TL".into()));
    send(&mut app, W::Field(6, "15".into()));
    let w = app.haul_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    let cost = rep.cost.as_ref().expect("priced");
    assert_eq!(cost.currency, "TL");
    assert!((cost.total - rep.over_moment / 1000.0 * 12.5).abs() < 1e-6, "{cost:?} {rep:?}");
    assert!(cost.trips.is_some_and(|t| t >= 1.0));
    send(&mut app, W::Save);
    assert!(app.haul_window.is_none());
    let saved = model(&mut app).road_hauls()[0].cost.clone().expect("written");
    assert_eq!((saved.overhaul, saved.payload), (Some(12.5), Some(15.0)));
}

#[test]
fn a_bad_site_is_refused_and_the_window_stays() {
    let mut app = app();
    let _ = app.update(Message::Run("haul.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Add);
    send(&mut app, W::Site(0, 2, "9000".into()));
    send(&mut app, W::Site(0, 3, "10".into()));
    assert!(app.haul_window.as_ref().is_some_and(|w| w.preview.why.is_some()), "a site off the stations");
    send(&mut app, W::Save);
    assert!(app.haul_window.is_some());
    assert!(model(&mut app).road_hauls()[0].sites.is_empty());
}

/// The window, wide and narrow, light and dark; `.run/gui-evidence/yol-y9-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib haul_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y9-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("haul.editor"));
        // The main diagram with a borrow and a waste site, its balance line lowered: loops both ways.
        send(&mut app, W::Choose(0));
        send(&mut app, W::Field(0, "9000".into()));
        send(&mut app, W::Add);
        send(&mut app, W::Site(0, 2, "120".into()));
        send(&mut app, W::Site(0, 3, "2000".into()));
        send(&mut app, W::Kind(0, kentos_commands::HaulSiteKind::Borrow));
        send(&mut app, W::Field(1, "150".into()));
        send(&mut app, W::Field(2, "TL".into()));
        for (k, v) in [(3, "12.5"), (4, "40"), (5, "85"), (6, "15")] {
            send(&mut app, W::Field(k, v.into()));
        }
        send(&mut app, W::Add);
        send(&mut app, W::Site(1, 2, "560".into()));
        send(&mut app, W::Site(1, 3, "8000".into()));
        shot(&mut app, "bruckner");
        send(&mut app, W::Choose(1));
        shot(&mut app, "bruckner-yan");
        send(&mut app, W::Cancel);
    }
}

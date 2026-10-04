//! The GNSS window driven as the user drives it (docs/adr/0169 §6), with the
//! sample files of fixtures/gnss/v1 (scripts/fixtures/gnss_samples.py): the
//! sample drawing's parcel corners measured by a receiver, in WGS 84. The
//! drawing is in TUREF / TM36 (EPSG:5256), so the points fall on it.

use std::path::PathBuf;

use kentos_contracts::Entity;

use super::gnss_import::{Event as Gnss, State};
use super::tests::{count, run, send};
use super::{Event, Window};
use crate::app::{App, Dialog, Message, Picker};
use crate::files_testing::{app_with_drawing, last_said};

fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/gnss/v1")
        .join(name)
}

fn window(app: &App) -> &State {
    match &app.exchange {
        Some(Window::GnssImport(s)) => s,
        other => panic!("the GNSS window, not {other:?}"),
    }
}

fn gnss(app: &mut App, e: Gnss) {
    send(app, Event::GnssImport(e));
}

/// The open drawing's points named `name`.
fn point(app: &App, name: &str) -> kentos_contracts::PointEntity {
    app.document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .find_map(|e| match e {
            Entity::Point(p) if p.base.label.as_deref() == Some(name) => Some(p.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no point {name}"))
}

#[test]
fn a_gpx_goes_in_as_named_points_on_the_drawing_in_one_undo_step() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(sample("sample.gpx"));
    let before = count(&app);
    run(&mut app, "file.import.gnss");
    assert_eq!(app.dialog, Some(Dialog::Exchange));
    let s = window(&app);
    let plan = s.plan().expect("the project has a system");
    // Four waypoints and five track points; the waypoint without a fix and
    // the one whose latitude is not read are said, not taken.
    assert_eq!(plan.placed.len(), 9);
    assert_eq!(s.problems(), 2);
    let names: Vec<&str> = plan.placed.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "101/7-1", "101/7-2", "101/7-3", "101/7-4", "G1", "G2", "G3", "G4", "G5"
        ]
    );
    gnss(&mut app, Gnss::Run);
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    assert_eq!(count(&app) - before, 9);
    assert_eq!(
        last_said(&app),
        "“sample.gpx”: 2 kayıt alınmadı (okunamayan ya da projenin sistemine çevrilemeyen); nedenleri içe aktarma penceresinde yazılıydı."
    );
    // The corner falls on the drawing's corner (the sample's positions are
    // its TM36 coordinates through PROJ; EPSG's way from WGS 84 to TUREF is
    // the ±1 m null transformation both take).
    let corner = point(&app, "101/7-1");
    assert!(
        (corner.p.x - 486_512.352).abs() < 0.001 && (corner.p.y - 4_420_187.531).abs() < 0.001,
        "{:?}",
        corner.p
    );
    // Its elevation is the ellipsoidal height: the file's height plus the
    // geoid's separation; the corner without the separation has none.
    assert_eq!(corner.z, Some(1098.158));
    assert_eq!(point(&app, "101/7-4").z, None);
    let attr = |k: &str| corner.base.attrs.get(k).cloned().unwrap_or_default();
    assert_eq!(attr("Tür"), "GNSS noktası");
    assert_eq!(attr("Çözüm"), "3B");
    assert_eq!(attr("Yükseklik (dosyada, m)"), "1061.284");
    assert!(
        attr("Dönüşüm").starts_with("WGS 84 → TUREF / TM36: "),
        "{}",
        attr("Dönüşüm")
    );
    // On a new layer named after the file.
    let layer = &corner.base.layer_id;
    let doc = app.document.as_ref().expect("open");
    assert_eq!(
        doc.model.layers().get(layer).map(|l| l.name.as_str()),
        Some("sample")
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before, "one undo step takes all of it back");
}

#[test]
fn the_kinds_and_the_naming_are_chosen() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(sample("sample.gpx"));
    run(&mut app, "file.import.gnss");
    gnss(&mut app, Gnss::Kind("trkpt"));
    assert_eq!(window(&app).plan().expect("a plan").placed.len(), 4);
    gnss(&mut app, Gnss::Kind("trkpt"));
    gnss(&mut app, Gnss::Prefix("İZ-".to_owned()));
    gnss(&mut app, Gnss::Start("10".to_owned()));
    let names: Vec<String> = window(&app)
        .plan()
        .expect("a plan")
        .placed
        .iter()
        .skip(4)
        .map(|p| p.name.clone())
        .collect();
    assert_eq!(names, ["İZ-10", "İZ-11", "İZ-12", "İZ-13", "İZ-14"]);
    // A start that is not a number leaves the last good one.
    gnss(&mut app, Gnss::Start("1x".to_owned()));
    assert_eq!(window(&app).plan().expect("a plan").placed[4].name, "İZ-10");
}

#[test]
fn an_nmea_log_takes_its_fixes() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(sample("sample.nmea"));
    run(&mut app, "file.import.gnss");
    let s = window(&app);
    let plan = s.plan().expect("a plan");
    // Four GGA fixes; the one without a fix and the broken checksum are said.
    assert_eq!(plan.placed.len(), 4);
    assert_eq!(s.problems(), 2);
    let first = &plan.placed[0];
    assert_eq!(first.name, "G1");
    assert_eq!(
        first.attrs.get("Çözüm").map(String::as_str),
        Some("RTK sabit")
    );
    assert_eq!(
        first.attrs.get("Zaman").map(String::as_str),
        Some("2026-10-04T09:31:10.00Z")
    );
}

#[test]
fn a_project_without_a_system_takes_none() {
    let mut app = app_with_drawing();
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.srid = 0;
        doc.model.set_settings(settings);
    }
    app.picker = Picker::File(sample("sample.gpx"));
    let before = count(&app);
    run(&mut app, "file.import.gnss");
    assert!(window(&app).plan().is_none());
    gnss(&mut app, Gnss::Run);
    assert_eq!(count(&app), before);
    assert_eq!(
        app.dialog,
        Some(Dialog::Exchange),
        "the window stays, saying why"
    );
}

/// Pictures of the GNSS window for the owner, dark and light, at 1440×900
/// and at the smallest window (1100×650), written to `.run/shots` (never
/// committed); not run by default:
///
/// ```text
/// cargo test -p kentos-desktop exchange::gnss_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let only = std::env::var("KENTOS_SHOTS_ONLY").unwrap_or_default();
    let wanted = |name: &str| only.is_empty() || only.split(',').any(|w| name.contains(w));
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        let picture = |app: &mut App, name: &str| {
            if !wanted(name) {
                return;
            }
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(app, App::view, &mut update);
            let _ = snapshot.render(app.view(), &app.theme());
            let file = out.join(format!("{name}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        };
        for mode in ["dark", "light"] {
            let fresh = || {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                app
            };
            let mut app = fresh();
            app.picker = Picker::File(sample("sample.gpx"));
            run(&mut app, "file.import.gnss");
            picture(&mut app, &format!("gnss-{mode}-1-gpx"));
            gnss(&mut app, Gnss::Run);
            picture(&mut app, &format!("gnss-{mode}-2-alindi"));

            let mut app = fresh();
            app.picker = Picker::File(sample("sample.nmea"));
            run(&mut app, "file.import.gnss");
            picture(&mut app, &format!("gnss-{mode}-3-nmea"));

            let mut app = fresh();
            {
                let doc = app.document.as_mut().expect("a drawing");
                let mut settings = doc.settings().clone();
                settings.srid = 0;
                doc.model.set_settings(settings);
            }
            app.picker = Picker::File(sample("sample.gpx"));
            run(&mut app, "file.import.gnss");
            picture(&mut app, &format!("gnss-{mode}-4-sistemsiz"));
        }
    }
}

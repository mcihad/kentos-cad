//! The GNSS window driven as the user drives it (docs/adr/0169 §6), with the
//! sample files of fixtures/gnss/v1 (scripts/fixtures/gnss_samples.py): the
//! sample drawing's parcel corners measured by a receiver, in WGS 84. The
//! drawing is in TUREF / TM36 (EPSG:5256), so the points fall on it.

use std::path::PathBuf;

use kentos_commands::Entity;

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
fn point(app: &App, name: &str) -> kentos_commands::PointEntity {
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
    // Its track comes in as a line too (docs/adr/0222 §6), unless unticked.
    assert_eq!(plan.lines.len(), 1);
    gnss(&mut app, Gnss::Lines);
    assert!(window(&app).plan().expect("a plan").lines.is_empty());
    gnss(&mut app, Gnss::Run);
    assert_eq!(app.dialog, None, "{:?}", app.exchange);
    assert_eq!(count(&app) - before, 9);
    // What the preview said stays in the log (docs/adr/9329): the records not taken, with why.
    assert_eq!(
        last_said(&app),
        "“sample.gpx”: 2 kayıt alınmadı: Satır 20: konumu olmayan noktalar (1 nokta, fix none) okunmadı; Satır 21: konum \
         (lat “39,93”, lon “35,84”) okunmuyor; nokta okunmadı."
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

/// The track comes in as a polyline of its moved points (docs/adr/0222 §6),
/// whatever kinds of points are taken; one undo step takes it back with them.
#[test]
fn a_gpx_track_comes_in_as_a_line() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(sample("sample.gpx"));
    let before = count(&app);
    run(&mut app, "file.import.gnss");
    gnss(&mut app, Gnss::Kind("trkpt"));
    let plan = window(&app).plan().expect("a plan");
    assert_eq!((plan.placed.len(), plan.lines.len()), (4, 1));
    gnss(&mut app, Gnss::Run);
    assert_eq!(count(&app) - before, 5);
    let doc = app.document.as_ref().expect("open");
    let track = doc
        .model
        .entities()
        .find_map(|e| match e {
            Entity::Polyline(p)
                if p.base.attrs.get("Tür").map(String::as_str) == Some("GNSS izi") =>
            {
                Some(p.clone())
            }
            _ => None,
        })
        .expect("the track");
    assert_eq!(track.pts.len(), 5);
    assert_eq!(
        track.base.attrs.get("Kaynak").map(String::as_str),
        Some("GPX izi")
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), before);
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

/// GIS-034 (docs/adr/1089): the sample's positions (about 35.84° E) lie outside TUREF / TM30's area of use (west of
/// 31.5° E): the preview says how many, their longitudes and the zone; in TM36, the drawing's own, it says nothing.
#[test]
fn positions_outside_the_projects_zone_are_said() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(sample("sample.gpx"));
    run(&mut app, "file.import.gnss");
    let points = window(&app).positions();
    assert!(app.gnss_zone_line(&points).is_none(), "TM36 holds them");
    let doc = app.document.as_mut().expect("open");
    let mut settings = doc.settings().clone();
    settings.srid = 5254;
    doc.model.set_settings(settings);
    assert!(app.gnss_zone_line(&points).is_some(), "TM30 does not");
    let area = kentos_project::crs::area(5254).expect("TM30's area");
    let out = points
        .iter()
        .filter(|p| !area.holds_near(p.lon, p.lat))
        .count();
    assert_eq!(
        out,
        points.len(),
        "every sample position is east of TM30's zone"
    );
    if std::env::var_os("KENTOS_SHOTS").is_some() {
        send(&mut app, Event::Close);
        app.picker = Picker::File(sample("sample.gpx"));
        run(&mut app, "file.import.gnss");
        let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
        shot.settle(&mut app, App::view, &mut |app: &mut App, m| {
            drop(app.update(m))
        });
        crate::files_testing::evidence(&mut shot, &app, "gis-034", "tm30-dilim-disi");
    }
}

/// The layers of the open drawing by name, with how many objects each holds.
fn layers(app: &App) -> Vec<(String, usize)> {
    let model = &app.document.as_ref().expect("open").model;
    model
        .layers()
        .leaves()
        .into_iter()
        .map(|l| (l.name.clone(), model.count(&l.id)))
        .collect()
}

/// docs/adr/9329: a new layer is never one of the drawing's. The GPX's points go onto “sample”, its track onto a layer
/// of its own, “sample_iz”; the NMEA log of the same name is offered “sample_2” and “sample_2_iz”; a name a layer has
/// is refused with the way to it; the lines can go with the points. The new layers keep the file they came from, and a
/// project written and opened again has them, their objects and their records.
#[test]
fn a_new_layer_is_never_one_of_the_drawings_and_the_lines_have_their_own() {
    let mut app = app_with_drawing();
    app.picker = Picker::File(sample("sample.gpx"));
    run(&mut app, "file.import.gnss");
    gnss(&mut app, Gnss::Run);
    let had = layers(&app);
    assert!(had.contains(&("sample".to_owned(), 9)), "{had:?}");
    assert!(had.contains(&("sample_iz".to_owned(), 1)), "{had:?}");
    // The transformation and the heights are said after the import too.
    let log: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        log.iter().any(|l| l.starts_with("“sample.gpx”: dönüşüm ")),
        "{log:?}"
    );
    assert!(
        log.iter().any(|l| l.contains("Kot elipsoit yüksekliğidir")),
        "{log:?}"
    );
    // The NMEA log of the same name: free names.
    app.picker = Picker::File(sample("sample.nmea"));
    run(&mut app, "file.import.gnss");
    let s = window(&app);
    assert_eq!(
        (s.name_for_test(), s.lines_name_for_test()),
        ("sample_2", "sample_2_iz")
    );
    // A name a layer has: refused with the way to it.
    gnss(&mut app, Gnss::Name("SAMPLE".into()));
    let before = layers(&app);
    gnss(&mut app, Gnss::Run);
    assert_eq!(layers(&app), before, "nothing taken");
    assert!(
        matches!(app.exchange, Some(Window::GnssImport(_))),
        "the window stays"
    );
    // The lines with the points, onto a new layer.
    gnss(&mut app, Gnss::Name("nmea".into()));
    gnss(&mut app, Gnss::LinesTo(super::gnss_import::LinesTo::Points));
    gnss(&mut app, Gnss::Run);
    let now = layers(&app);
    assert!(
        now.contains(&("nmea".to_owned(), 5)),
        "4 fixes and the line together: {now:?}"
    );
    // Its record: the format, the file, its SHA-256.
    let model = &app.document.as_ref().expect("open").model;
    let id = model
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "nmea")
        .map(|l| l.id.clone())
        .expect("nmea");
    let origin = model
        .layers()
        .info_of(&id)
        .and_then(|i| i.origin)
        .expect("its origin");
    assert_eq!(origin.driver, "nmea");
    assert!(
        origin.address.ends_with("sample.nmea"),
        "{}",
        origin.address
    );
    // Written and opened again: the layers, their objects and their records are there.
    let path = crate::files_testing::scratch("gnss-yeniden").join("gnss.kcad");
    crate::document::write(&model.to_snapshot_v2(), &path).expect("written");
    let again = crate::document::Document::read(&path).expect("read again");
    let leaves = again.model.layers().leaves();
    let find = |name: &str| {
        leaves
            .iter()
            .find(|l| l.name == name)
            .map(|l| (l.id.clone(), again.model.count(&l.id)))
    };
    assert_eq!(find("sample").map(|(_, n)| n), Some(9));
    assert_eq!(find("sample_iz").map(|(_, n)| n), Some(1));
    assert_eq!(find("nmea").map(|(_, n)| n), Some(5));
    let (sample_id, _) = find("sample").expect("sample");
    let kept = again
        .model
        .layers()
        .info_of(&sample_id)
        .and_then(|i| i.origin)
        .expect("kept");
    assert_eq!(kept.driver, "gpx");
}

/// The pictures (`.run/gui-evidence/kullanim-9329/`): the NMEA log of the same name as the GPX taken before, its free
/// names offered and its lines' layer of their own; the name of a layer typed, refused with the way to it.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib gnss_tests::layer_screens -- --ignored --nocapture"]
fn layer_screens() {
    crate::drawing_fonts::load();
    let prepared = |typed: Option<&str>| {
        let mut app = app_with_drawing();
        app.picker = Picker::File(sample("sample.gpx"));
        run(&mut app, "file.import.gnss");
        gnss(&mut app, Gnss::Run);
        app.picker = Picker::File(sample("sample.nmea"));
        run(&mut app, "file.import.gnss");
        if let Some(name) = typed {
            gnss(&mut app, Gnss::Name(name.into()));
        }
        app
    };
    crate::files_testing::usage_shots("9329", "01-bos-adlar", || prepared(None));
    crate::files_testing::usage_shots("9329", "02-katmanin-adi", || prepared(Some("sample")));
}

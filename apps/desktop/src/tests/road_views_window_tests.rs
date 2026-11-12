//! Görünüşler (road_views_window.rs, plan/road.md Y13.4): the window opens on the first alignment with its profile and
//! its corridor, one km drives the plan, the profile and the section, the steps go between the sections' km's, a typed
//! km is taken with the alignment's km, the sample line set's sections come too; and its pictures.

use serde_json::Value;

use crate::app::{App, Message};
use crate::road_views_window::{Event as V, Source};

/// The road project of `road.project.query`'s cases: two alignments, profiles, sets, a corridor.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("road.views"));
    app
}

fn send(app: &mut App, e: V) {
    let _ = app.update(Message::RoadViews(e));
}

#[test]
fn one_km_drives_the_three_views() {
    let mut app = app();
    let w = app.road_views_window.as_ref().expect("Görünüşler opens");
    assert!(w.profile.is_some(), "the alignment's profile");
    assert!(matches!(w.source, Source::Corridor(_)), "its corridor: {:?}", w.source);
    assert!(w.read.why.is_empty(), "{:?}", w.read.why);
    let rows: Vec<f64> = w.read.rows.iter().map(|r| r.station).collect();
    assert!(rows.len() > 2, "the corridor's sections");
    assert_eq!(w.station, rows[0], "at the first section");
    assert!(w.read.line.len() > 100 && w.read.length > 0.0);
    // Sonraki and Önceki go between the sections' km's; Son to the last.
    send(&mut app, V::Step(1));
    assert_eq!(app.road_views_window.as_ref().unwrap().station, rows[1]);
    send(&mut app, V::Step(2));
    assert_eq!(app.road_views_window.as_ref().unwrap().station, *rows.last().unwrap());
    send(&mut app, V::Step(-1));
    assert_eq!(app.road_views_window.as_ref().unwrap().station, rows[rows.len() - 2]);
    // A km between two sections: the nearest is shown.
    let mid = 0.5 * (rows[0] + rows[1]) - 0.01;
    send(&mut app, V::Station(mid));
    let w = app.road_views_window.as_ref().unwrap();
    assert_eq!(w.shown().map(|s| s.0), Some(0));
    // A typed km, as the window writes it (“k+mmm.mmm”) and in metres, goes back to its station.
    send(&mut app, V::Station(50.0));
    let written = app.road_views_window.as_ref().unwrap().km.clone();
    let metres = written.replace('+', "").replace('.', ",");
    send(&mut app, V::Station(0.0));
    send(&mut app, V::Km(written.clone()));
    send(&mut app, V::KmGo);
    assert!((app.road_views_window.as_ref().unwrap().station - 50.0).abs() < 1e-6, "{written}");
    send(&mut app, V::Station(0.0));
    send(&mut app, V::Km(metres.clone()));
    send(&mut app, V::KmGo);
    assert!((app.road_views_window.as_ref().unwrap().station - 50.0).abs() < 1e-6, "{metres}");
    send(&mut app, V::Station(75.0));
    // Beyond the alignment: said, the km stays.
    send(&mut app, V::Km("99+000".into()));
    send(&mut app, V::KmGo);
    assert!((app.road_views_window.as_ref().unwrap().station - 75.0).abs() < 1e-9);
    // A sample line set's sections instead of the corridor's.
    let doc = &app.document.as_ref().unwrap().model;
    let alignment = app.road_views_window.as_ref().unwrap().alignment.clone().unwrap();
    if let Some(set) = doc.road_sections().iter().find(|s| s.alignment == alignment) {
        let id = set.id.clone();
        send(&mut app, V::Source(Source::Set(id)));
        let w = app.road_views_window.as_ref().unwrap();
        assert!(!w.read.rows.is_empty(), "the set's sections: {:?}", w.read.why);
    }
    send(&mut app, V::Close);
    assert!(app.road_views_window.is_none());
}

/// The window, wide and light, narrow and dark, at a km on a curve; `.run/gui-evidence/yol-gorunusler-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib road_views_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-gorunusler-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let rows: Vec<f64> = app.road_views_window.as_ref().unwrap().read.rows.iter().map(|r| r.station).collect();
        let k = rows.len() / 2;
        send(&mut app, V::Station(rows[k]));
        shot(&mut app, "koridor");
        if name == "acik-genis" {
            send(&mut app, V::Station(0.5 * (rows[k] + rows[k + 1]) + 0.3));
            shot(&mut app, "arada");
        }
        // Y60 (docs/adr/0966): on the circle the footer reads its radius, on a grade “düz eğim”.
        send(&mut app, V::Station(340.0));
        shot(&mut app, "y60-daire");
        send(&mut app, V::Station(100.0));
        shot(&mut app, "y60-egim");
    }
}

/// The red line at the km (Y60, docs/adr/0966; ROAD-PROFILE-12): the window reads elevation, grade and the radius of
/// curvature from the same function `road.profiles.query` answers with, at each km the slider takes.
#[test]
fn the_red_line_at_the_km_is_the_query_s() {
    let mut app = app();
    let w = app.road_views_window.as_ref().expect("open");
    let (id, length) = (w.profile.clone().expect("a profile"), w.read.length);
    for k in 0..=20 {
        let s = length * f64::from(k) / 20.0;
        send(&mut app, V::Station(s));
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        let def = doc.road_profiles().iter().find(|p| p.id == id).cloned().expect("the profile");
        let shown = kentos_native_application::road_profiles::point_at(doc, &def, s);
        let input = kentos_commands::ProfilesQuery { profile: Some(id.clone()), stations: vec![s], every: None, terrain: false };
        let kentos_commands::CommandResult::Completed { output, .. } =
            kentos_native_application::road_profiles_query::execute(&mut kentos_native_application::ExecutionContext::new(doc), input)
        else {
            panic!("the query reads")
        };
        let asked = output.at.first().cloned();
        assert_eq!(shown.as_ref().map(|p| (p.red, p.grade, p.rate, p.radius, p.along3d)), asked.map(|p| (p.red, p.grade, p.rate, p.radius, p.along3d)), "km {s}");
    }
}

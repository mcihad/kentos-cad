//! Ağ dengelemesi's Hata elipsleri row (survey_network_window.rs, plan/map-cad.md MAP-05, docs/adr/2960) on the
//! shared case's archive whose baselines' deviations are turned 30°: Kabul et with the box ticked draws the three
//! points' error ellipses on the layer chosen in Katmanlar, magnified as typed, in the same undo step as the points;
//! the archive records them as a use; unticked, none. And the picture.

use kentos_commands::Entity;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::survey_network_window::Event as N;

fn opened() -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.edit.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.contains("sapmalar 30° dönük: elipslerin")))
        .expect("the case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn ellipses(app: &App) -> Vec<kentos_commands::EllipseEntity> {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model
        .entities()
        .filter_map(|e| match e {
            Entity::Ellipse(el) => Some(el.clone()),
            _ => None,
        })
        .collect()
}

/// The window opened, the ellipses' box as given, magnified `times`, accepted.
fn accepted(draw: bool, times: f64) -> App {
    let mut app = opened();
    app.selected_layer = Some("gnss".into());
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::Ellipses(draw)));
    let _ = app.update(Message::SurveyNetwork(N::Magnify(times)));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    app
}

#[test]
fn kabul_et_draws_the_ellipses_with_the_points_in_one_step() {
    let app = accepted(true, 2000.0);
    let drawn = ellipses(&app);
    assert_eq!(drawn.len(), 3);
    assert!(drawn.iter().all(|e| e.base.layer_id == "gnss" && e.base.attrs.get("Büyütme").map(String::as_str) == Some("2000")));
    // Turned some 30° east of the map's north, the convergence taken off; the minor axis 0.375 of the major.
    let e = &drawn[0];
    let bearing = e.major.x.atan2(e.major.y).to_degrees();
    assert!((bearing - 29.67).abs() < 0.05 && (e.ratio - 0.375).abs() < 0.001, "{bearing} {}", e.ratio);
    let survey = app.document.as_ref().and_then(|d| d.model.map_survey()).cloned().expect("an archive");
    let used = survey.uses.iter().find(|u| u.operation == "Hata elipsleri (da-1)").expect("recorded");
    assert_eq!(used.uids.len(), 3);
    // One undo step takes the points and the ellipses away.
    let mut app = app;
    let _ = app.update(Message::Run("edit.undo"));
    assert!(ellipses(&app).is_empty());
    // Unticked: none.
    assert!(ellipses(&accepted(false, 2000.0)).is_empty());
}

/// The network accepted with ellipses 50 000 times (3 mm as 150 m), fitted, light wide and dark narrow,
/// `.run/gui-evidence/hata-elipsleri-2960/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib network_ellipses_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/hata-elipsleri-2960");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.selected_layer = Some("gnss".into());
        let _ = app.update(Message::Run("map.survey.adjustment"));
        let _ = app.update(Message::SurveyNetwork(N::Ellipses(true)));
        let _ = app.update(Message::SurveyNetwork(N::Magnify(50_000.0)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::SurveyNetwork(N::Accept));
        app.dialog = None;
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("elipsler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// docs/adr/3360: Kot çubuğu ticked beside the ellipses draws each point's height half-interval as a bar through the
/// point, magnified as the ellipse (the bar's half-length the attribute's half-interval times the magnification), in
/// the same use; the message counts both.
#[test]
fn kot_cubugu_draws_the_height_bars_with_the_ellipses() {
    let mut app = opened();
    app.selected_layer = Some("gnss".into());
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::Ellipses(true)));
    let _ = app.update(Message::SurveyNetwork(N::Magnify(2000.0)));
    let _ = app.update(Message::SurveyNetwork(N::Bars(true)));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    let doc = app.document.as_ref().expect("a drawing");
    let bars: Vec<&kentos_commands::PathEntity> = doc
        .model
        .entities()
        .filter_map(|e| match e {
            Entity::Polyline(p) if p.base.attrs.contains_key("Kot yarı aralığı (m)") => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(bars.len(), 3);
    for b in &bars {
        let half: f64 = b.base.attrs["Kot yarı aralığı (m)"].parse().expect("a number");
        let length = b.pts[2].y - b.pts[3].y;
        assert!((length - 2.0 * half * 2000.0).abs() < 2.0 * 0.0001 * 2000.0, "{length} {half}");
        assert_eq!(b.pts.len(), 6, "the bar and its two ticks");
    }
    let survey = doc.model.map_survey().cloned().expect("an archive");
    let used = survey.uses.iter().find(|u| u.operation == "Hata elipsleri (da-1)").expect("recorded");
    assert_eq!(used.uids.len(), 6, "three ellipses and three bars");
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("3 hata elipsi ve 3 kot çubuğu 2000 kat büyütülerek çizildi"), "{said}");
}

/// The network accepted with ellipses and height bars 50 000 times, fitted, `.run/gui-evidence/kot-cubugu-3360/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib network_ellipses_tests::bar_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn bar_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kot-cubugu-3360");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.selected_layer = Some("gnss".into());
        let _ = app.update(Message::Run("map.survey.adjustment"));
        let _ = app.update(Message::SurveyNetwork(N::Ellipses(true)));
        let _ = app.update(Message::SurveyNetwork(N::Magnify(50_000.0)));
        let _ = app.update(Message::SurveyNetwork(N::Bars(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::SurveyNetwork(N::Accept));
        app.dialog = None;
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("cubuklar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// docs/adr/3760: Etiket ticked beside the ellipses writes each ellipse's semi-axes in millimetres beside it, 2.5 mm on
/// paper at the drawing's plot scale, east of the ellipse, in the same use; the message counts the labels.
#[test]
fn etiket_writes_each_ellipse_s_semi_axes_beside_it() {
    let mut app = opened();
    app.selected_layer = Some("gnss".into());
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::Ellipses(true)));
    let _ = app.update(Message::SurveyNetwork(N::Magnify(2000.0)));
    let _ = app.update(Message::SurveyNetwork(N::Labels(true)));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    let doc = app.document.as_ref().expect("a drawing");
    let plot = doc.model.settings().plot_scale;
    let drawn = ellipses(&app);
    let labels: Vec<&kentos_commands::TextEntity> = doc
        .model
        .entities()
        .filter_map(|e| match e {
            Entity::Text(t) if t.base.attrs.contains_key("Dengeleme") => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(labels.len(), 3);
    for (t, e) in labels.iter().zip(&drawn) {
        // The ellipse drawn 2000 times: its semi-axes in millimetres are the drawn ones over 2000, times 1000.
        let major = e.major.x.hypot(e.major.y) / 2000.0 * 1000.0;
        assert_eq!(t.text, format!("a {:.1} · b {:.1} mm", major, major * e.ratio));
        assert!((t.height - 2.5 * plot / 1000.0).abs() < 1e-12);
        assert_eq!(t.align, Some(kentos_commands::TextAlign::MiddleLeft));
        assert!(t.p.x > e.c.x + (e.major.x.hypot(e.major.y)), "east of the ellipse");
        assert_eq!(t.base.attrs.get("İşaret"), e.base.attrs.get("İşaret"));
    }
    let survey = doc.model.map_survey().cloned().expect("an archive");
    let used = survey.uses.iter().find(|u| u.operation == "Hata elipsleri (da-1)").expect("recorded");
    assert_eq!(used.uids.len(), 6, "three ellipses and three labels");
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("3 etiket yazıldı"), "{said}");
}

/// The window with Etiket ticked, then the network accepted with ellipses, bars and labels 2000 times, seen close to N1,
/// `.run/gui-evidence/elips-etiketi-3760/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib network_ellipses_tests::label_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn label_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/elips-etiketi-3760");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.selected_layer = Some("gnss".into());
        let _ = app.update(Message::Run("map.survey.adjustment"));
        let _ = app.update(Message::SurveyNetwork(N::Ellipses(true)));
        let _ = app.update(Message::SurveyNetwork(N::Magnify(2000.0)));
        let _ = app.update(Message::SurveyNetwork(N::Bars(true)));
        let _ = app.update(Message::SurveyNetwork(N::Labels(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::SurveyNetwork(N::Accept));
        app.dialog = None;
        snapshot.settle(&mut app, App::view, &mut update);
        // Close to N1: its ellipse 6 m across the 3 mm major, its bar and the label 2.5 m high east of them.
        let n1 = ellipses(&app)[0].c;
        app.zoom_to_bounds(&kentos_commands::Bounds { min_x: n1.x - 12.0, min_y: n1.y - 16.0, max_x: n1.x + 52.0, max_y: n1.y + 16.0 });
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("etiketler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The shared case's network with N6 15 m east of N1 (docs/adr/4160), opened.
fn opened_near() -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.edit.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Ağı kabul et, etiketler: N1")))
        .expect("the case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// docs/adr/4160: with N6 15 m east of N1, N1's label east would lie over N6's ellipse: Kabul et puts it at the top
/// right of N1's ellipse (its bottom left (a + h)/√2 up and right), the others east, and the message says one went
/// elsewhere.
#[test]
fn a_label_that_would_cover_a_neighbour_goes_to_a_free_place() {
    let mut app = opened_near();
    app.selected_layer = Some("gnss".into());
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::Ellipses(true)));
    let _ = app.update(Message::SurveyNetwork(N::Magnify(1000.0)));
    let _ = app.update(Message::SurveyNetwork(N::Labels(true)));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    let doc = app.document.as_ref().expect("a drawing");
    let drawn = ellipses(&app);
    let label_of = |mark: &str| {
        doc.model
            .entities()
            .find_map(|e| match e {
                Entity::Text(t) if t.base.attrs.get("İşaret").map(String::as_str) == Some(mark) => Some(t.clone()),
                _ => None,
            })
            .expect("a label")
    };
    let n1 = drawn.iter().find(|e| e.base.attrs.get("İşaret").map(String::as_str) == Some("N1")).expect("N1's ellipse");
    let t = label_of("N1");
    assert_eq!(t.align, Some(kentos_commands::TextAlign::BottomLeft));
    let d = (n1.major.x.hypot(n1.major.y) + t.height) / 2f64.sqrt();
    assert!((t.p.x - (n1.c.x + d)).abs() < 1e-9 && (t.p.y - (n1.c.y + d)).abs() < 1e-9, "{:?}", t.p);
    for mark in ["N4", "N5", "N6"] {
        assert_eq!(label_of(mark).align, Some(kentos_commands::TextAlign::MiddleLeft), "{mark}");
    }
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("4 etiket yazıldı (1'i elipse ya da etikete düşmesin diye başka yana kondu)"), "{said}");
}

/// N1's label moved to the top right beside N6, seen close, `.run/gui-evidence/etiket-yeri-4160/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib network_ellipses_tests::label_place_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn label_place_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/etiket-yeri-4160");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened_near();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.selected_layer = Some("gnss".into());
        let _ = app.update(Message::Run("map.survey.adjustment"));
        let _ = app.update(Message::SurveyNetwork(N::Ellipses(true)));
        let _ = app.update(Message::SurveyNetwork(N::Magnify(1000.0)));
        let _ = app.update(Message::SurveyNetwork(N::Labels(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let _ = app.update(Message::SurveyNetwork(N::Accept));
        app.dialog = None;
        snapshot.settle(&mut app, App::view, &mut update);
        // N1 and N6 15 m apart: N1's label above, to the right; N6's east.
        let n1 = ellipses(&app).into_iter().find(|e| e.base.attrs.get("İşaret").map(String::as_str) == Some("N1")).expect("N1").c;
        app.zoom_to_bounds(&kentos_commands::Bounds { min_x: n1.x - 10.0, min_y: n1.y - 12.0, max_x: n1.x + 45.0, max_y: n1.y + 14.0 });
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("etiketler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

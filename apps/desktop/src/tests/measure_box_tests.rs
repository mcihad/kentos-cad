//! Ölçü kutusu (the CBS review's 10th item, docs/adr/9328) through the real app, on the review's triangle: A (0, 0),
//! C (30, 0), B (30, 40) from the scenes' origin; AC 30 m, CB 40 m, AB 50 m, the right angle at C. Mesafe ölç says
//! every part and the total, Alan hesapla the area and the perimeter, Açı ölç the angle and its explement in the
//! project's unit by name, Koordinat oku the east and the north by name; each with how it was reckoned and the system.
//! The box shows while its tool runs; Kopyala writes it out; × puts it away. The ribbon's Hızlı ölçüm runs the same
//! commands as Harita › Sorgu.

use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{Key, Modifiers};
use iced::{Point, Rectangle, Size};

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, last_said};
use crate::keys::KeyPress;
use crate::tools_scenes::{E, N, Objects, open, run, typed};
use crate::viewport::Event;

fn key(app: &mut App, named: Named, code: Code) {
    let _ = app.update(Message::Key(KeyPress {
        key: Key::Named(named),
        physical: Physical::Code(code),
        modifiers: Modifiers::default(),
        text: None,
        repeat: false,
    }));
}

/// The point `p` from the scenes' origin, typed as the command line takes it (Y,X: east first).
fn at(app: &mut App, p: [f64; 2]) {
    typed(app, &format!("{},{}", E + p[0], N + p[1]));
}

fn project() -> App {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0)))));
    open(&mut app, Objects::new());
    app
}

fn rows(app: &App) -> Vec<(String, String)> {
    app.measure_shown().expect("the box shows").rows.clone()
}

const A: [f64; 2] = [0.0, 0.0];
const C: [f64; 2] = [30.0, 0.0];
const B: [f64; 2] = [30.0, 40.0];

#[test]
fn the_triangle_is_measured_into_the_box_with_its_units_way_and_system() {
    let mut app = project();
    let f = app.format();
    // Mesafe ölç: A, C, B and Enter: 30 + 40 = 70 m.
    run(&mut app, "tool.measure");
    assert!(app.measure_box_view().is_none(), "nothing measured yet");
    for p in [A, C, B] {
        at(&mut app, p);
    }
    key(&mut app, Named::Enter, Code::Enter);
    let m = app.measure_shown().expect("the box shows").clone();
    assert_eq!(m.rows[..3], [
        ("1. parça".to_owned(), f.length(30.0)),
        ("2. parça".to_owned(), f.length(40.0)),
        ("Toplam".to_owned(), f.length(70.0)),
    ]);
    assert_eq!(m.units, "metre");
    assert!(m.method.starts_with("Düzlemde: projeksiyon koordinatlarıyla"), "{}", m.method);
    assert!(m.system.as_deref().is_some_and(|s| s.contains("EPSG:")), "{:?}", m.system);
    assert!(app.measure_box_view().is_some());
    // A and B: 50 m.
    at(&mut app, A);
    at(&mut app, B);
    key(&mut app, Named::Enter, Code::Enter);
    assert_eq!(rows(&app)[0], ("1. parça".to_owned(), f.length(50.0)));
    // Kopyala writes every line; the log says so.
    let text = app.measure_shown().expect("shown").text();
    let _ = app.update(Message::MeasureBox(crate::measure_box::Event::Copy));
    assert_eq!(last_said(&app), "Ölçü panoya kopyalandı.");
    assert!(text.starts_with("Mesafe\n1. parça\t") && text.contains("\nHesap\t") && text.contains("\nKoordinat sistemi\t"));
    // Another tool: the box goes; Mesafe ölç again: it is back, as it was.
    run(&mut app, "tool.area");
    assert!(app.measure_box_view().is_none(), "the area tool measured nothing yet");
    // Alan hesapla: A, C, B and Enter: 30 × 40 / 2 = 600 m², 30 + 40 + 50 = 120 m.
    for p in [A, C, B] {
        at(&mut app, p);
    }
    key(&mut app, Named::Enter, Code::Enter);
    assert_eq!(rows(&app)[..2], [("Alan".to_owned(), f.area(600.0)), ("Çevre".to_owned(), f.length(120.0))]);
    assert_eq!(app.measure_shown().map(|m| m.units.as_str()), Some("metre, metrekare"));
    // ×: put away until the next measurement.
    let _ = app.update(Message::MeasureBox(crate::measure_box::Event::Close));
    assert!(app.measure_box_view().is_none());
    // Esc leaves the tool.
    key(&mut app, Named::Escape, Code::Escape);
}

#[test]
fn the_right_angle_reads_100_gon_or_90_degrees_and_a_point_its_east_and_north() {
    let mut app = project();
    // Açı ölç: the vertex C first, then its arms A and B.
    run(&mut app, "tool.measureAngle");
    for p in [C, A, B] {
        at(&mut app, p);
    }
    let m = app.measure_shown().expect("the box shows").clone();
    assert_eq!(m.title, "Açı");
    assert!(m.rows[0].1.starts_with("100.0000"), "{:?}", m.rows);
    assert!(m.rows[1].1.starts_with("300.0000"), "{:?}", m.rows);
    assert_eq!(m.units, "gon (bir tur 400 g)");
    // In degrees: 90° and 270°.
    let doc = app.document.as_mut().expect("open");
    let mut settings = doc.model.settings().clone();
    settings.angle_unit = kentos_commands::AngleUnit::Deg;
    doc.model.set_settings(settings);
    for p in [C, A, B] {
        at(&mut app, p);
    }
    let m = app.measure_shown().expect("the box shows").clone();
    assert!(m.rows[0].1.starts_with("90"), "{:?}", m.rows);
    assert_eq!(m.units, "derece (bir tur 360°)");
    // Koordinat oku: A's east and north by name.
    key(&mut app, Named::Escape, Code::Escape);
    run(&mut app, "crs.query");
    let f = app.format();
    let p = app.viewport.camera.world_to_screen(kentos_render_wgpu::Vec2::new(E + A[0], N + A[1]));
    let p = Point::new(p[0] as f32, p[1] as f32);
    let _ = app.update(Message::Viewport(Event::Moved(p)));
    let _ = app.update(Message::Viewport(Event::Pressed(p)));
    let _ = app.update(Message::Viewport(Event::Released(p)));
    let m = app.measure_shown().expect("the box shows").clone();
    assert_eq!(m.title, "Koordinat");
    assert_eq!(m.rows[0].0, "Y (doğu)");
    assert_eq!(m.rows[1].0, "X (kuzey)");
    let east: f64 = m.rows[0].1.replace(',', ".").parse().unwrap_or(f64::NAN);
    assert!((east - E).abs() < 0.5, "{:?} ({})", m.rows, f.coord(E));
}

#[test]
fn hizli_olcum_runs_harita_sorgus_commands() {
    let app = project();
    for id in ["tool.measure", "tool.area", "tool.measureAngle", "crs.query"] {
        assert!(app.available(id), "{id}");
    }
    let ribbon: serde_json::Value =
        serde_json::from_str(include_str!("../../../../catalog/v1/ribbon.json")).expect("the ribbon");
    let survey = ribbon["tabs"]["gis"]
        .as_array()
        .and_then(|tabs| tabs.iter().find(|t| t["id"] == "survey"))
        .expect("the CBS Ölçme tab");
    let quick = &survey["panels"][0];
    assert_eq!(quick["label"], "Hızlı ölçüm");
    let ids: Vec<&str> = quick["items"].as_array().expect("items").iter().filter_map(|i| i["command"].as_str()).collect();
    assert_eq!(ids, ["tool.measure", "tool.area", "tool.measureAngle", "crs.query"]);
}

/// A CBS project with the triangle drawn as a parcel, the view on it, the Ölçme tab open and `tool` run over A, C, B.
fn measured(tool: &'static str, enter: bool) -> App {
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.workspace = Some(kentos_commands::Workspace::Gis);
    doc.model.set_settings(settings);
    let mut o = Objects::new();
    o.path("parsel", &[A, C, B], true);
    open(&mut app, o);
    let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(Point::ORIGIN, Size::new(1100.0, 650.0)))));
    app.viewport.show(&kentos_render_wgpu::Bounds { min_x: E - 20.0, min_y: N - 10.0, max_x: E + 50.0, max_y: N + 50.0 });
    let _ = app.update(Message::RibbonTab("survey"));
    run(&mut app, tool);
    let order = if tool == "tool.measureAngle" { [C, A, B] } else { [A, C, B] };
    for p in order {
        at(&mut app, p);
    }
    if enter {
        key(&mut app, Named::Enter, Code::Enter);
    }
    app
}

/// The pictures (`.run/gui-evidence/kullanim-9328/`): Ölçme › Hızlı ölçüm with Mesafe ölç's box (A, C, B: 30, 40,
/// 70 m), Alan hesapla's (600 m², 120 m) and Açı ölç's (100 gon at C).
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib measure_box_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9328", "01-hizli-olcum-mesafe", || measured("tool.measure", true));
    crate::files_testing::usage_shots("9328", "02-alan", || measured("tool.area", true));
    crate::files_testing::usage_shots("9328", "03-aci", || measured("tool.measureAngle", false));
}

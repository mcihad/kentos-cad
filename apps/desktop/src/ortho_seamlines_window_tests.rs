//! Mozaik birleşme çizgisi (ortho_seamlines_window.rs, docs/adr/4700) on the shared cases' drawings of
//! `cad.orthophoto.seamlines`: opened from the raster tab it plans the strips' equal-distance (Voronoi) line as the case
//! does; two selected areas taken are the footprints (ayak izi; the staggered strips' AEFC); radyometrik fark with
//! the buildings' layer gives the case's line; Mozaik elemanlarını çiz writes the two elements on the new layer in one
//! undo step and selects them, and Ortofoto karşılaştırması's Seçili mozaik elemanlarını al then measures along their
//! shared boundary, which is the line. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::ortho_compare_window::Event as C;
use crate::ortho_compare_window_tests::{open, with_fixtures};
use crate::ortho_seamlines_window::{Event as E, Method, Prints, Show};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.orthophoto.seamlines.json")).expect("cases")
}

/// A shared case by the start of its name.
fn case(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).cloned().expect(name)
}

/// The same JSON, numbers within the cases' 10⁻⁷, the revision left out (the window's drawing has its own).
fn near(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().expect("a number"), b.as_f64().expect("a number"));
            assert!((a - b).abs() <= 1e-7, "{at}: {a} ≠ {b}");
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                near(x, y, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).filter(|k| *k != "revision").collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                near(a.get(k).unwrap_or(&Value::Null), b.get(k).unwrap_or(&Value::Null), &format!("{at}.{k}"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

fn planned(app: &App) -> Value {
    serde_json::to_value(app.ortho_seamlines.answer.clone().expect("an answer").expect("planned")).expect("json")
}

fn opened(name: &str) -> (App, Value) {
    let c = case(name);
    let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
    let _ = app.update(Message::Run("raster.seamlines"));
    assert_eq!(app.dialog, Some(Dialog::OrthoSeamlines), "{name}");
    (app, c)
}

/// The two footprint areas (the cases' entities 3 and 4) selected and taken.
fn take_areas(app: &mut App) {
    app.selection.set(vec![Slot(3), Slot(4)]);
    let _ = app.update(Message::OrthoSeamlines(E::TakeAreas));
    assert_eq!(app.ortho_seamlines.prints, Prints::Areas);
}

#[test]
fn the_strips_voronoi_seam_is_the_cases() {
    let (app, c) = opened("iki şerit yan yana");
    near(&planned(&app), &c["steps"][0]["result"]["output"], "plan");
    let (_, rows) = app.ortho_seamlines_lines();
    assert_eq!(rows.len(), 2, "the seam's two points");
}

#[test]
fn two_selected_areas_are_the_footprints() {
    let (mut app, c) = opened("kaydırılmış şeritler");
    take_areas(&mut app);
    near(&planned(&app), &c["steps"][0]["result"]["output"], "AEFC");
    // One area selected: said, nothing taken.
    app.selection.set(vec![Slot(3)]);
    let _ = app.update(Message::OrthoSeamlines(E::TakeAreas));
    assert!(app.ortho_seamlines.areas.is_some(), "the earlier two stay");
}

#[test]
fn radiometry_keeping_out_of_the_buildings_is_the_cases() {
    let (mut app, c) = opened("radyometrik fark, binalardan geçmeden");
    take_areas(&mut app);
    let _ = app.update(Message::OrthoSeamlines(E::Method(Method::Radiometry)));
    let _ = app.update(Message::OrthoSeamlines(E::Avoid("bina".into())));
    near(&planned(&app), &c["steps"][0]["result"]["output"], "radiometry");
}

#[test]
fn written_polygons_are_selected_and_measured_along_by_the_comparison() {
    let (mut app, _) = opened("iki şerit yan yana");
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let seam = planned(&app)["seam"].clone();
    let _ = app.update(Message::OrthoSeamlines(E::Write));
    assert_eq!(app.dialog, None, "closed after writing");
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.model.entities().count(), before + 2);
    assert!(doc.model.layers().get(kentos_commands::SEAMLINE_LAYER).is_some(), "the layer made");
    let chosen = app.selection.ids().to_vec();
    assert_eq!(chosen.len(), 2, "the two polygons selected");
    // Ortofoto karşılaştırması takes them: its shared boundary is the seam.
    let _ = app.update(Message::Run("raster.compare"));
    for e in [C::Seam(true), C::Step(1.0), C::Window(21.0), C::Search(5.0), C::Correlation("0,8".into())] {
        let _ = app.update(Message::OrthoCompare(e));
    }
    let _ = app.update(Message::OrthoCompare(C::TakePolygons));
    let got = serde_json::to_value(app.ortho_compare.answer.clone().expect("an answer").expect("compared")).expect("json");
    near(&got["seam"]["shared"]["parts"], &Value::Array(vec![seam]), "shared boundary");
    // One undo step takes both away, with the layer.
    let _ = app.update(Message::Run("edit.undo"));
    let doc = app.document.as_ref().expect("a drawing");
    assert_eq!(doc.model.entities().count(), before);
    assert!(doc.model.layers().get(kentos_commands::SEAMLINE_LAYER).is_none(), "the layer undone");
}

/// `.run/gui-evidence/birlesme-cizgisi-4700/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ortho_seamlines_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/birlesme-cizgisi-4700");
    std::fs::create_dir_all(&out).expect("a folder");
    let shoot = |app: &mut App, size: Size, file: &str| {
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(file);
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    };
    let theme = |app: &mut App, theme: &str, size: Size| {
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
    };
    let (wide, narrow) = (Size::new(1440.0, 900.0), Size::new(1100.0, 760.0));
    // 1: the staggered strips' Voronoi seam (AEFC), dark and narrow.
    let (mut app, _) = opened("kaydırılmış şeritler");
    theme(&mut app, "dark", narrow);
    take_areas(&mut app);
    shoot(&mut app, narrow, "1-voronoi-kaydirilmis-koyu-dar.png");
    // 2: radiometry keeping out of the building, light and wide.
    let (mut app, _) = opened("radyometrik fark, binalardan geçmeden");
    theme(&mut app, "light", wide);
    take_areas(&mut app);
    let _ = app.update(Message::OrthoSeamlines(E::Method(Method::Radiometry)));
    let _ = app.update(Message::OrthoSeamlines(E::Avoid("bina".into())));
    shoot(&mut app, wide, "2-radyometri-bina-acik-genis.png");
    // 3: its seam's points.
    let _ = app.update(Message::OrthoSeamlines(E::Show(Show::Points)));
    shoot(&mut app, wide, "3-noktalar-acik-genis.png");
    // 4: written on the drawing, selected, the comparison measuring along them.
    let (mut app, _) = opened("eğri kesim: köşe ile kenar arasında parabol");
    theme(&mut app, "light", wide);
    take_areas(&mut app);
    let _ = app.update(Message::OrthoSeamlines(E::Write));
    let _ = app.update(Message::Run("view.zoomExtents"));
    shoot(&mut app, wide, "4-yazildi-cizimde-acik-genis.png");
    // 5: its place in the ribbon: a raster selected, the raster tab beside Karşılaştır.
    let c = case("iki şerit yan yana");
    let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
    theme(&mut app, "light", wide);
    app.selection.set(vec![Slot(1)]);
    shoot(&mut app, wide, "5-serit-raster-sekmesi-acik-genis.png");
}

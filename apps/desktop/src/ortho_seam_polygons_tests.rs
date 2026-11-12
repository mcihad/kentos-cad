//! Ortofoto karşılaştırması's Birleşme çizgisi from the two images' mosaic elements (seamline polygons) (ortho_compare_window.rs,
//! docs/adr/4460), on the shared seam cases' drawing of `cad.orthophoto.compare`: the two polygons chosen (Seçili
//! çokgenleri al) give the shared case's places along their shared boundary; one polygon chosen is said and nothing is
//! taken; the two-run case numbers each distance by its part in the table; Çizgiyi kaldır goes back to the overlap's
//! grid; nothing is written. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;

use crate::app::{App, Dialog, Message};
use crate::ortho_compare_window::{Event as C, Show};
use crate::ortho_compare_window_tests::{case, close, open, with_fixtures};

/// The seam asked as the cases ask it.
fn seam(app: &mut App) {
    for e in [C::Seam(true), C::Step(1.0), C::Window(21.0), C::Search(5.0), C::Correlation("0,8".into())] {
        let _ = app.update(Message::OrthoCompare(e));
    }
}

#[test]
fn the_seam_is_measured_along_the_seamline_polygons_shared_boundary() {
    let c = case("dikiş çokgenlerinden");
    let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("raster.compare"));
    assert_eq!(app.dialog, Some(Dialog::OrthoCompare));
    seam(&mut app);
    // One polygon chosen: said, nothing taken.
    app.selection.set(vec![Slot(3)]);
    let _ = app.update(Message::OrthoCompare(C::TakePolygons));
    assert!(app.ortho_compare.seam_polygons.is_none(), "one polygon: none taken");
    // The two, in the case's order.
    app.selection.set(vec![Slot(3), Slot(4)]);
    let _ = app.update(Message::OrthoCompare(C::TakePolygons));
    assert_eq!(app.ortho_compare.seam_line_name, "İki mozaik elemanı");
    assert_eq!(app.ortho_compare.show, Show::Seam);
    let got = serde_json::to_value(app.ortho_compare.answer.clone().expect("an answer").expect("compared")).expect("json");
    close(&got["seam"], &c["steps"][0]["result"]["output"]["seam"], "seam");
    let (_, rows) = app.ortho_compare_lines();
    assert_eq!(rows[0][0], "4.000", "one part: the distance alone");
    let notes = got["notes"].to_string();
    assert!(notes.contains("mozaik elemanlarının ortak sınırı boyunca (15.31 m, 1 parça)"), "{notes}");
    let _ = app.update(Message::OrthoCompare(C::DropLine));
    assert!(app.ortho_compare.seam_polygons.is_none());
    let got = serde_json::to_value(app.ortho_compare.answer.clone().expect("an answer").expect("compared")).expect("json");
    assert!(got["seam"].get("shared").is_none(), "the overlap's grid again");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn two_runs_number_their_distances_by_part() {
    let c = case("dikiş çokgenlerinin iki ortak parçası");
    let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
    let _ = app.update(Message::Run("raster.compare"));
    seam(&mut app);
    app.selection.set(vec![Slot(3), Slot(4)]);
    let _ = app.update(Message::OrthoCompare(C::TakePolygons));
    let got = serde_json::to_value(app.ortho_compare.answer.clone().expect("an answer").expect("compared")).expect("json");
    close(&got["seam"], &c["steps"][0]["result"]["output"]["seam"], "seam");
    let (_, rows) = app.ortho_compare_lines();
    let firsts: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(firsts.contains(&"1 · 4.000") && firsts.contains(&"2 · 0.000"), "{firsts:?}");
}

/// `.run/gui-evidence/dikis-cokgenleri-4460/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ortho_seam_polygons_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dikis-cokgenleri-4460");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name, case_name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis", "dikiş çokgenlerinden"),
        ("dark", Size::new(1100.0, 760.0), "koyu-dar", "dikiş çokgenlerinin iki ortak parçası"),
    ] {
        let c = case(case_name);
        let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set(vec![Slot(3), Slot(4)]);
        let _ = app.update(Message::Run("raster.compare"));
        seam(&mut app);
        let _ = app.update(Message::OrthoCompare(C::TakePolygons));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-dikis-cokgenleri-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

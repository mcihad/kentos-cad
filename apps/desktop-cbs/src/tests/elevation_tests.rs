//! Kot oku beside Kot noktası ekle in a CBS project (the CBS review's 19th item, docs/adr/9335), its picture: Analiz ›
//! Arazi with Kot noktası ekle, Kot oku and Mesh sor; Kot oku on a GNSS point saying its elevation first, in the log
//! and the measurement box. What it says is tested beside Koordinat oku (kentos-interaction's `constructing`).

use serde_json::json;

use crate::app::{App, Message};
use crate::files_testing::drive;

/// A CBS project in EPSG:5256 whose layer `gnss` holds three surveyed points with their elevations.
fn gnss() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = crate::tools_scenes::Objects::new();
    for (i, z) in [1099.214, 1101.870, 1098.455].into_iter().enumerate() {
        let id = o.point("gnss", [20.0 * i as f64, 10.0 * i as f64], Some(z));
        o.fields(id, json!({ "attrs": { "ad": format!("G{}", i + 1) } }));
    }
    crate::tools_scenes::open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        d["layers"] = json!([
            { "id": "gnss", "name": "gnss", "type": "layer", "visible": true, "locked": false, "expanded": true,
              "geometry": "point", "children": [],
              "style": { "color": "#E5484D", "lineType": "continuous", "lineWeight": 0.25 } }
        ]);
        d["activeLayer"] = json!("gnss");
    });
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(iced::Rectangle::new(
        iced::Point::ORIGIN,
        iced::Size::new(1000.0, 700.0),
    ))));
    let task = app.update(Message::Run("view.zoomExtents"));
    drive(&mut app, task);
    app
}

#[test]
#[ignore = "pictures: cargo test -p kentos-desktop-cbs elevation_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9335", "03-kot-oku", || {
        let mut app = gnss();
        let _ = app.update(Message::RibbonTab("analysis"));
        let task = app.update(Message::Run("tool.readElevation"));
        drive(&mut app, task);
        let (e, n) = (crate::tools_scenes::E, crate::tools_scenes::N);
        let [sx, sy] = app.viewport.camera.world_to_screen(kentos_interaction::Vec2::new(e + 20.0, n + 10.0));
        let at = iced::Point::new(sx as f32, sy as f32);
        for ev in [crate::viewport::Event::Moved(at), crate::viewport::Event::Pressed(at), crate::viewport::Event::Released(at)] {
            let task = app.update(Message::Viewport(ev));
            drive(&mut app, task);
        }
        app
    });
}

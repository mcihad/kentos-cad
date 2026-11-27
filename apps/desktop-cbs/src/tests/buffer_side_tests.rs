//! Tampon's Yan in a CBS project (the CBS review's 17th item, docs/adr/9333), its pictures: three points with Sol and
//! Sağ off and why; points and a line, what Sol reaches; İşlemler's Geçmiş saying Yan was not used. What the window
//! and the record decide is tested beside the tool (the desktop's `buffer_cases_tests`).

use serde_json::json;

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::processing::Event;

/// A CBS project in EPSG:5256: “qa_noktalar” with three points 30 m apart; “Karışık” (a layer of any kind) with the
/// same three points and a 100 m line drawn west to east 50 m north of them.
fn points() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = crate::tools_scenes::Objects::new();
    for layer in ["noktalar", "karisik"] {
        for i in 0..3 {
            o.point(layer, [30.0 * f64::from(i), 0.0], None);
        }
    }
    o.path("karisik", &[[0.0, 50.0], [100.0, 50.0]], false);
    crate::tools_scenes::open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        d["layers"] = json!([
            { "id": "noktalar", "name": "qa_noktalar", "type": "layer", "visible": true, "locked": false,
              "expanded": true, "geometry": "point", "children": [],
              "style": { "color": "#E5484D", "lineType": "continuous", "lineWeight": 0.25 } },
            { "id": "karisik", "name": "Karışık", "type": "layer", "visible": true, "locked": false,
              "expanded": true, "children": [],
              "style": { "color": "#0090FF", "lineType": "continuous", "lineWeight": 0.25 } }
        ]);
        d["activeLayer"] = json!("noktalar");
    });
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(iced::Rectangle::new(
        iced::Point::ORIGIN,
        iced::Size::new(1000.0, 700.0),
    ))));
    let task = app.update(Message::Run("view.zoomExtents"));
    drive(&mut app, task);
    app
}

fn event(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(crate::gis::Event::Processing(e)));
    drive(app, task);
}

/// Tampon on `layer`, 10 m, Yan: Sol, into “Tampon”.
fn tampon(app: &mut App, layer: &str) {
    let task = app.update(Message::Run("processing.run.geometry.buffer"));
    drive(app, task);
    event(app, Event::Value("input".into(), json!({ "scope": "layer", "layerId": layer })));
    event(app, Event::Value("distance".into(), json!(10.0)));
    event(app, Event::Value("layer".into(), json!({ "newName": "Tampon" })));
    event(app, Event::Value("side".into(), json!("left")));
}

#[test]
#[ignore = "pictures: cargo test -p kentos-desktop-cbs buffer_side_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9333", "04-tampon-noktalar", || {
        let mut app = points();
        tampon(&mut app, "noktalar");
        app
    });
    crate::files_testing::usage_shots("9333", "05-tampon-karisik", || {
        let mut app = points();
        tampon(&mut app, "karisik");
        app
    });
    crate::files_testing::usage_shots("9333", "06-tampon-gecmis", || {
        let mut app = points();
        tampon(&mut app, "noktalar");
        event(&mut app, Event::Run);
        event(&mut app, Event::Close);
        let task = app.update(Message::Run("processing.history"));
        drive(&mut app, task);
        app
    });
}

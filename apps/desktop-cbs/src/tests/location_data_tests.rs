//! Konuma göre seç over data layers in a CBS project (the 27 November CBS review's 11th item, docs/adr/9340), its
//! picture: only data layers of a GeoPackage, the 30–40–50 m triangle, its corners and a control point 5 m west; the
//! corners chosen in their table by Kesişen, the control point left out. What the run decides is tested beside the
//! tool (the desktop's `data_location_tests`).

use kentos_data::{Coord, Curve, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};
use serde_json::json;

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::processing::Event;

const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;

fn triangle() -> std::path::PathBuf {
    let path = crate::files_testing::scratch("konum-veri-resim").join("ucgen.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    for (table, kind) in [("ucgen", "LINESTRING"), ("koseler", "POINT"), ("kontrol", "POINT")] {
        tx.create_table(
            &NewTable {
                name: table.into(),
                title: None,
                description: None,
                key: "fid".into(),
                fields: vec![Field::new("ad", FieldKind::Text { max: Some(16) })],
                geometry: Some(GeometryField { name: "geom".into(), type_name: kind.into(), z: false, m: false, srid: Some(5256) }),
                spatial_index: true,
                system: None,
            },
            false,
        )
        .expect("table");
    }
    let at = |x: f64, y: f64| Coord::xy(X0 + x, Y0 + y);
    let row = |ad: &str, g: Geometry| NewFeature { fid: None, geometry: Some(g), values: vec![("ad".into(), Value::Text(ad.into()))] };
    let ring = Curve::Line(vec![at(0.0, 0.0), at(40.0, 0.0), at(0.0, 30.0), at(0.0, 0.0)]);
    tx.insert("ucgen", &[row("ABC", Geometry::Curve(ring))]).expect("the triangle");
    let corners: Vec<NewFeature> =
        [("A", 0.0, 0.0), ("B", 40.0, 0.0), ("C", 0.0, 30.0)].iter().map(|(ad, x, y)| row(ad, Geometry::Point(Some(at(*x, *y))))).collect();
    tx.insert("koseler", &corners).expect("the corners");
    tx.insert("kontrol", &[row("K", Geometry::Point(Some(at(-5.0, 0.0))))]).expect("the control");
    tx.commit().expect("commit");
    path
}

fn event(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(crate::gis::Event::Processing(e)));
    drive(app, task);
}

fn scene(file: &std::path::Path) -> App {
    crate::data_layers::register();
    kentos_processing::command::register_as("desktop");
    let mut app = crate::files_testing::app_with_drawing();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let all: Vec<kentos_domain::Slot> = model.entities().map(|e| kentos_domain::Slot(e.base().id)).collect();
        model.remove(&all);
        let mut settings = model.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Gis);
        model.set_settings(settings);
    }
    let address = file.display().to_string();
    let tables = ["ucgen", "koseler", "kontrol"].map(|t| crate::data_layers::describe(&address, t).expect("described"));
    let ids = app.add_data_layers(tables.to_vec()).expect("added");
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(iced::Rectangle::new(
        iced::Point::ORIGIN,
        iced::Size::new(1000.0, 700.0),
    ))));
    let task = app.update(Message::Run("view.zoomExtents"));
    drive(&mut app, task);
    let task = app.update(Message::Run("processing.run.selection.byLocation"));
    drive(&mut app, task);
    event(&mut app, Event::Value("input".into(), json!({ "scope": "all" })));
    event(&mut app, Event::Value("reference".into(), json!({ "scope": "layer", "layerId": ids[0] })));
    app
}

#[test]
#[ignore = "pictures: cargo test -p kentos-desktop-cbs location_data_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    let file = triangle();
    crate::files_testing::usage_shots("9340", "01-konuma-gore-sec-veri", || {
        let mut app = scene(&file);
        event(&mut app, Event::Run);
        app
    });
}

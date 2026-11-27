//! Konuma göre seç over data layers (docs/adr/9340; the CBS review of 27 November 2026, item 11). Only data layers of a
//! GeoPackage in EPSG:5256: `ucgen`, the 30–40–50 m triangle as a closed line; `koseler`, its three corners; `kontrol`,
//! a point 5 m west of the right angle.
//!
//! - Seçilecek nesneler Tümü, Başvuru `ucgen`, Kesişen: the three corners are selected in `koseler`'s table by their
//!   keys, the control point is not; the drawing's selection stays empty.
//! - Uzaklıkta 6 m: the control point too, said apart; the table shows the layer with most rows.
//! - The window counts the rows: no “uygun nesne yok”.
//!
//! Picture: the CBS package's `location_data_tests::screens` (İşlemler is CBS's panel).

use std::collections::BTreeSet;

use kentos_data::{Coord, Curve, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};
use serde_json::json;

use super::super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive, scratch};
use crate::gis;

const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;

fn event(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Processing(e)));
    drive(app, task);
}

fn status(app: &App) -> RunStatus {
    app.gis.processing.dialog.as_ref().map_or(RunStatus::Idle, |w| w.status.clone())
}

/// The GeoPackage of the review: the triangle, its corners and the control point.
fn triangle(tag: &str) -> std::path::PathBuf {
    let path = scratch(tag).join("ucgen.gpkg");
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
    let corners = [("A", 0.0, 0.0), ("B", 40.0, 0.0), ("C", 0.0, 30.0)];
    let ring = Curve::Line(vec![at(0.0, 0.0), at(40.0, 0.0), at(0.0, 30.0), at(0.0, 0.0)]);
    tx.insert("ucgen", &[row("ABC", Geometry::Curve(ring))]).expect("the triangle");
    let points: Vec<NewFeature> = corners.iter().map(|(ad, x, y)| row(ad, Geometry::Point(Some(at(*x, *y))))).collect();
    tx.insert("koseler", &points).expect("the corners");
    tx.insert("kontrol", &[row("K", Geometry::Point(Some(at(-5.0, 0.0))))]).expect("the control");
    tx.commit().expect("commit");
    path
}

/// The project: the sample drawing's objects taken away, the three tables as data layers; Konuma göre seç open.
fn opened(tag: &str) -> (App, String) {
    crate::data_layers::register();
    // The program registers İşlemler's tools for `cad.processing.run` at its start (launch.rs).
    kentos_processing::command::register_as("desktop");
    let address = triangle(tag).display().to_string();
    let mut app = app_with_drawing();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let all: Vec<kentos_domain::Slot> = model.entities().map(|e| kentos_domain::Slot(e.base().id)).collect();
        model.remove(&all);
    }
    app.add_data_layers(
        ["ucgen", "koseler", "kontrol"]
            .into_iter()
            .map(|t| crate::data_layers::describe(&address, t).expect("described"))
            .collect(),
    )
    .expect("added");
    let task = app.update(Message::Run("processing.run.selection.byLocation"));
    drive(&mut app, task);
    let task = app.data_rows_tasks();
    drive(&mut app, task);
    let ucgen = crate::data_layers_tests::node(&app, "ucgen").expect("ucgen").id.clone();
    event(&mut app, Event::Value("input".into(), json!({ "scope": "all" })));
    event(&mut app, Event::Value("reference".into(), json!({ "scope": "layer", "layerId": ucgen })));
    (app, ucgen)
}

#[test]
fn the_corners_are_selected_by_their_keys_and_the_control_point_is_not() {
    let (mut app, _) = opened("konum-ucgen");
    // The rows counted: the window has objects to try.
    let window = app.gis.processing.dialog.as_ref().expect("the window");
    let input = window.inputs.get("input").expect("the input's line");
    assert_eq!(input.count, 5, "{}", input.description);
    assert_eq!(window.inputs.get("reference").map(|r| r.count), Some(1));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(text, "3 nesne ilişkiyi sağladı; “koseler” katmanında 3 satır tabloda seçildi (anahtar: 1, 2, 3).");
    let koseler = crate::data_layers_tests::node(&app, "koseler").expect("koseler").id.clone();
    assert_eq!(app.gis.features.data.selected_layer(), Some(koseler.as_str()));
    assert_eq!(app.gis.features.data.selected, BTreeSet::from([1, 2, 3]));
    assert_eq!(app.data_table_layer().map(|n| n.id), Some(koseler));
    assert!(app.selection.is_empty(), "no object of the drawing");
}

#[test]
fn near_takes_the_control_point_too_and_says_its_layer() {
    let (mut app, _) = opened("konum-ucgen-yakin");
    event(&mut app, Event::Value("relation".into(), json!("near")));
    event(&mut app, Event::Value("distance".into(), json!(6)));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(
        text,
        "4 nesne ilişkiyi sağladı; “koseler” katmanında 3 satır tabloda seçildi (anahtar: 1, 2, 3); “kontrol” katmanında 1 \
         satır da sağladı (tabloda görmek için Seçilecek nesneler'de o katmanı seçin)."
    );
    // 4 m: the control point stays out again.
    event(&mut app, Event::Value("distance".into(), json!(4)));
    event(&mut app, Event::Run);
    let RunStatus::Ok { text, .. } = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert!(text.starts_with("3 nesne ilişkiyi sağladı"), "{text}");
}

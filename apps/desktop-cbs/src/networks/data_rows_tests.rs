//! A network over a data layer (the second round's T32, docs/adr/9353): the 30–40–50 m triangle's three edges as rows
//! of a GeoPackage line table, no object of the drawing on it. Ağlar's Denetle builds the network over the rows: three
//! nodes, three pieces, 120 m. Before, the tools found no network near any click.

use kentos_data::{
    Coord, Curve, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable,
    Value,
};

use super::window::Event;
use crate::app::Message;
use crate::cbs::CbsState;
use crate::files_testing::drive;

const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;

/// The triangle's edges A–C 30 m, C–B 40 m, B–A 50 m.
fn triangle() -> std::path::PathBuf {
    let path = crate::files_testing::scratch("ag-veri").join("yollar.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg
        .open(path.to_str().expect("utf-8"))
        .expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "yollar".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![Field::new("ad", FieldKind::Text { max: Some(8) })],
            geometry: Some(GeometryField {
                name: "geom".into(),
                type_name: "LINESTRING".into(),
                z: false,
                m: false,
                srid: Some(5256),
            }),
            spatial_index: true,
            system: None,
        },
        false,
    )
    .expect("table");
    let at = |x: f64, y: f64| Coord::xy(X0 + x, Y0 + y);
    let (a, c, b) = (at(0.0, 0.0), at(30.0, 0.0), at(30.0, 40.0));
    let row = |ad: &str, p: Coord, q: Coord| NewFeature {
        fid: None,
        geometry: Some(Geometry::Curve(Curve::Line(vec![p, q]))),
        values: vec![("ad".into(), Value::Text(ad.into()))],
    };
    tx.insert(
        "yollar",
        &[row("AC", a, c), row("CB", c, b), row("BA", b, a)],
    )
    .expect("rows");
    tx.commit().expect("commit");
    path
}

#[test]
fn denetle_builds_the_network_over_the_data_layers_rows() {
    crate::data_layers::register();
    let path = triangle();
    let mut app = crate::files_testing::app_with_drawing();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let all: Vec<kentos_domain::Slot> = model
            .entities()
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        model.remove(&all);
        let mut settings = model.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Gis);
        model.set_settings(settings);
    }
    let ids = app
        .add_data_layers(vec![
            crate::data_layers::describe(&path.display().to_string(), "yollar").expect("described"),
        ])
        .expect("added");
    // The network on the layer, as Ağlar's Yeni makes one.
    let form = kentos_interaction::network::form::new_form(
        "ag-1",
        "Yollar ağı",
        kentos_commands::NetworkKind::Road,
        &ids[0],
        |m| m,
    );
    let def = kentos_interaction::network::form::def_of(&form, |v| v).expect("a network");
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let mut settings = model.settings().clone();
        settings.networks.push(def);
        model.set_settings(settings);
    }
    // The rows read, as the copy asks for them.
    let _ = app.source_copy();
    let task = app.data_rows_tasks();
    drive(&mut app, task);
    let task = app.update(Message::Run("network.manage"));
    drive(&mut app, task);
    let task = app.update(Message::from(crate::cbs::Event::Networks(
        super::Event::Window(Event::Check),
    )));
    drive(&mut app, task);
    let checked = app
        .modules
        .cbs()
        .networks
        .window
        .as_ref()
        .and_then(|w| w.checked())
        .expect("Denetle answered");
    assert_eq!((checked.nodes, checked.pieces), (3, 3));
    assert!((checked.length - 120.0).abs() < 1e-6, "{}", checked.length);
    let _ = std::fs::remove_file(&path);
}

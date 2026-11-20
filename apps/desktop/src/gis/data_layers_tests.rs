//! Veri katmanı on the desktop (docs/adr/9101): a GeoPackage table added as a data layer without its rows, its columns
//! as the layer's fields and its kind as the layer's; one undo step; the rows a view reads moved between systems.

use std::path::PathBuf;

use kentos_commands::{LayerGeometry, LayerNode};
use kentos_data::{Coord, Curve, Driver, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};

use crate::files_testing::app_with_drawing;

/// A GeoPackage with a table of `n` parcels in TM36 around the sample drawing.
fn parcels(name: &str, n: usize) -> PathBuf {
    let dir = std::env::temp_dir().join("kentos-desktop-data-layers");
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join(format!("{name}-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&path);
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "parseller".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![
                Field::new("ada", FieldKind::Int),
                Field::new("kullanim", FieldKind::Text { max: Some(20) }),
                Field::new("alan", FieldKind::Real),
            ],
            geometry: Some(GeometryField {
                name: "geom".into(),
                type_name: "MULTIPOLYGON".into(),
                z: false,
                m: false,
                srid: Some(5256),
            }),
            spatial_index: true,
        },
        false,
    )
    .expect("table");
    let rows: Vec<NewFeature> = (0..n)
        .map(|i| {
            let (x, y) = (486_500.0 + (i % 20) as f64 * 30.0, 4_420_200.0 + (i / 20) as f64 * 30.0);
            NewFeature {
                fid: None,
                geometry: Some(Geometry::Polygon(vec![Curve::Line(vec![
                    Coord::xy(x, y),
                    Coord::xy(x + 25.0, y),
                    Coord::xy(x + 25.0, y + 25.0),
                    Coord::xy(x, y + 25.0),
                    Coord::xy(x, y),
                ])])),
                values: vec![
                    ("ada".into(), Value::Int(100 + (i / 20) as i64)),
                    ("kullanim".into(), Value::Text(["Konut", "Park"][i % 2].into())),
                    ("alan".into(), Value::Real(625.0)),
                ],
            }
        })
        .collect();
    tx.insert("parseller", &rows).expect("rows");
    tx.commit().expect("commit");
    path
}

fn node<'a>(app: &'a crate::app::App, name: &str) -> Option<&'a LayerNode> {
    app.document
        .as_ref()?
        .model
        .layers()
        .leaves()
        .into_iter()
        .find(|n| n.name == name)
}

#[test]
fn a_geopackage_table_is_added_as_a_data_layer_without_its_rows() {
    crate::data_layers::register();
    let path = parcels("eklenir", 400);
    let address = path.display().to_string();
    let described = crate::data_layers::describe(&address, "parseller").expect("described");
    assert_eq!(described.data.driver, "gpkg");
    assert_eq!(described.data.key, "fid");
    assert_eq!(
        described.data.geometry.as_ref().map(|g| (g.type_name.as_str(), g.srid)),
        Some(("MULTIPOLYGON", Some(5256)))
    );
    assert!(described.data.extent.is_some_and(|b| b[0] >= 486_500.0 && b[2] <= 486_500.0 + 600.0));
    let names: Vec<&str> = described.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["ada", "kullanim", "alan"]);
    assert_eq!(described.fields[1].length, Some(20));

    let mut app = app_with_drawing();
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    let added = app.add_data_layers(vec![described]).expect("added");
    assert_eq!(added.len(), 1);
    let n = node(&app, "parseller").expect("the layer");
    assert_eq!(n.geometry, Some(LayerGeometry::Area));
    assert_eq!(n.data.as_ref().map(|d| d.table.as_str()), Some("parseller"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    // Its 400 rows stay in the file: the drawing holds no object of it.
    assert_eq!(doc.entities().count(), before);
    assert_eq!(doc.by_layer(&n.id).count(), 0);
    // One undo step takes it away.
    let _ = app.update(crate::app::Message::Run("edit.undo"));
    assert!(node(&app, "parseller").is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_table_in_another_system_is_drawn_in_the_projects() {
    let settings = app_with_drawing().document.expect("a drawing").model.settings().clone();
    let system = kentos_native_application::sources::TableSystem::of(&settings, Some(4326))
        .expect("the systems meet")
        .expect("another system");
    // A point near Sivas in WGS 84 lands in TM36's eastings and northings.
    let (x, y) = system.to_project(37.0, 39.75).expect("moved");
    assert!((500_000.0..700_000.0).contains(&x) && (4_350_000.0..4_450_000.0).contains(&y), "{x} {y}");
    let b = system.to_table([x - 100.0, y - 100.0, x + 100.0, y + 100.0]).expect("the box moved back");
    assert!(b[0] < 37.0 && b[2] > 37.0 && b[1] < 39.75 && b[3] > 39.75, "{b:?}");
}

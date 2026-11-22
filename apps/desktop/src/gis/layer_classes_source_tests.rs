//! A data layer's style classes counted by its source where it can (docs/adr/9110 §2): the same rows in a PostGIS table,
//! where a categorized style is one grouped count and a graduated style's or the rules' classes each one count by its
//! condition, and in a GeoPackage, where those number conditions are not exact and the rows are read and counted here,
//! give the same counts. Runs with `KENTOS_DATA_PG` (`postgresql://user@host:port/db`, its password in `PGPASSWORD`), as
//! the drivers' conformance does; without it the test says so and passes.

use kentos_data::conformance;
use serde_json::{Value, json};

const TABLE: &str = "public.kentos_siniflar";

fn made(address: &str, table: &str) {
    let source = kentos_data::registry().source(address).expect("the source");
    let mut tx = source.begin().expect("a transaction");
    tx.create_table(&conformance::table(table, 5256), true).expect("the table made");
    tx.insert(table, &conformance::new_features(&conformance::rows())).expect("its rows");
    tx.commit().expect("written");
}

fn node(driver: &str, address: &str, table: &str, renderer: &Value) -> kentos_commands::LayerNode {
    serde_json::from_value(json!({
        "id": "p", "name": "Parseller", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25, "renderer": renderer },
        "children": [], "geometry": "area",
        "fields": [{ "name": "ada", "kind": "integer" }, { "name": "alan", "kind": "decimal" },
                   { "name": "kullanim", "kind": "text" }, { "name": "mahalle", "kind": "text" }],
        "data": { "driver": driver, "address": address, "table": table, "key": "fid",
                  "geometry": { "column": "geom", "typeName": "POLYGON", "srid": 5256 } }
    }))
    .expect("a data layer")
}

fn renderers() -> Vec<Value> {
    let fill = |c: &str| json!({ "fill": { "type": "fill", "layers": [{ "id": "f", "type": "simple", "color": c }] } });
    vec![
        json!({ "type": "categorized", "expr": "kullanim", "other": fill("#888888"), "categories": [
            { "value": "Konut", "label": "Konut", "symbols": fill("#d9c27a") },
            { "value": "Park", "label": "Park", "symbols": fill("#2b7a3d") },
            { "value": "", "label": "(boş)", "symbols": fill("#cccccc") }] }),
        json!({ "type": "graduated", "expr": "alan", "classes": [
            { "min": 0.0, "max": 300.0, "label": "küçük", "symbols": fill("#eeeeee") },
            { "min": 300.0, "max": 600.0, "label": "orta", "symbols": fill("#999999") },
            { "min": 600.0, "max": 100000.0, "label": "büyük", "symbols": fill("#333333") }] }),
        json!({ "type": "rules", "rules": [
            { "label": "Parklar", "filter": "kullanim = 'Park'", "symbols": fill("#2b7a3d"), "children": [
                { "label": "büyük parklar", "filter": "alan > 500", "symbols": fill("#1b5a2d") }] },
            { "label": "Ada 105 üstü konut", "filter": "ada > 105 ve kullanim = 'Konut'", "symbols": fill("#d9c27a") },
            { "label": "Mahallesi boş", "filter": "mahalle boş", "symbols": fill("#cccccc") }] }),
    ]
}

#[test]
fn the_source_counts_classes_as_reading_them_does() {
    let Ok(address) = std::env::var("KENTOS_DATA_PG") else {
        eprintln!("KENTOS_DATA_PG yok: kaynakta sınıf sayımı sınaması atlandı");
        return;
    };
    crate::data_layers::register();
    made(&address, TABLE);
    let dir = crate::files_testing::scratch("kaynakta-siniflar");
    let gpkg = dir.join("siniflar.gpkg").display().to_string();
    kentos_geopackage::GeoPackage::create(std::path::Path::new(&gpkg)).expect("a GeoPackage");
    made(&gpkg, "parseller");
    let settings: kentos_commands::ProjectSettings =
        serde_json::from_value(json!({ "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2",
                                       "angleUnit": "deg", "plotScale": 1000, "workspace": "gis" }))
        .expect("settings");
    for renderer in renderers() {
        let counts = |driver: &str, address: &str, table: &str| {
            kentos_native_application::layer_classes::data_counts(&node(driver, address, table, &renderer), &settings, &|| false)
                .expect("counted")
        };
        let at_the_source = counts("postgis", &address, TABLE);
        let read_here = counts("gpkg", &gpkg, "parseller");
        assert_eq!(at_the_source, read_here, "{}", renderer["type"]);
        let (classes, total) = &at_the_source;
        assert_eq!(*total, conformance::rows().len() as u64);
        assert!(classes.iter().all(Result::is_ok) && !classes.is_empty(), "{classes:?}");
    }
    let source = kentos_data::registry().source(&address).expect("the source");
    let mut tx = source.begin().expect("a transaction");
    tx.drop_table(TABLE).expect("dropped");
    tx.commit().expect("written");
    let _ = std::fs::remove_dir_all(dir);
}

//! GIS-009 (docs/adr/1075): a layer's field rules go through a GeoPackage and come back — shown names, a 0–20 whole
//! number's range, a unique code, a required code — as KentOS's own document and as the schema extension other
//! programs read; what the objects break of them is said before the file is taken as a delivery.

use kentos_commands::{DocumentSnapshotV1, LayerFieldKind, check_value};
use kentos_domain::Document;
use kentos_geopackage::GeoPackage;
use kentos_geopackage::schema::Rule;
use kentos_native_application::geopackage;
use serde_json::json;

/// Two parks (codes `a`, `b`, benches `bench_a`, `bench_b`) and a route whose required code is `route`.
fn drawing(a: &str, b: &str, bench_a: &str, bench_b: &str, route: Option<&str>) -> Document {
    let mut route_attrs = json!({});
    if let Some(r) = route {
        route_attrs["rota_kodu"] = json!(r);
    }
    let v = json!({
        "format": "kentos.document", "version": 1, "name": "Envanter",
        "settings": { "srid": 5255, "lengthDecimals": 3, "areaDecimals": 2, "workspace": "gis", "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "drawingFont": "barlow" },
        "origin": { "x": 487000, "y": 4422000 },
        "layers": [
            { "id": "park", "name": "Park sınırları", "type": "layer", "visible": true, "locked": false, "expanded": true,
              "style": { "color": "#3E63DD", "lineType": "continuous", "lineWeight": 0.35 }, "geometry": "area", "children": [],
              "fields": [
                  { "name": "park_kodu", "alias": "Park kodu", "kind": "text", "unique": true },
                  { "name": "bank_sayisi", "alias": "Bank sayısı", "kind": "integer", "min": "0", "max": "20" },
              ] },
            { "id": "rota", "name": "Bakım rotaları", "type": "layer", "visible": true, "locked": false, "expanded": true,
              "style": { "color": "#E5484D", "lineType": "continuous", "lineWeight": 0.5 }, "geometry": "line", "children": [],
              "fields": [ { "name": "rota_kodu", "alias": "Rota kodu", "kind": "text", "required": true } ] },
        ],
        "activeLayer": "park",
        "entities": [
            { "kind": "polygon", "id": 1, "layerId": "park", "attrs": { "park_kodu": a, "bank_sayisi": bench_a },
              "pts": [{ "x": 487000, "y": 4422000 }, { "x": 487020, "y": 4422000 }, { "x": 487020, "y": 4422015 }] },
            { "kind": "polygon", "id": 2, "layerId": "park", "attrs": { "park_kodu": b, "bank_sayisi": bench_b },
              "pts": [{ "x": 487030, "y": 4422000 }, { "x": 487050, "y": 4422000 }, { "x": 487050, "y": 4422015 }] },
            { "kind": "polyline", "id": 3, "layerId": "rota", "attrs": route_attrs,
              "pts": [{ "x": 486990, "y": 4421995 }, { "x": 487060, "y": 4421995 }] },
        ],
        "styles": { "items": [], "categories": [] },
    });
    Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).unwrap()).unwrap()
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-gis009-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let _ = std::fs::remove_file(&path);
    path
}

#[test]
fn field_rules_come_back_from_a_geopackage() {
    let doc = drawing("ANK-01", "ANK-02", "0", "20", Some("RT-01"));
    let path = scratch("kurallar.gpkg");
    let written = geopackage::write(&doc, &[], &path).expect("written");
    assert!(written.rule_notes.is_empty(), "{:?}", written.rule_notes);
    let read = geopackage::read(&path, None).expect("read");
    let park = read
        .tables
        .iter()
        .find(|t| t.name == "Park sınırları")
        .expect("the parks");
    let code = park
        .fields
        .iter()
        .find(|f| f.name == "park_kodu")
        .expect("the code");
    assert_eq!(code.alias.as_deref(), Some("Park kodu"));
    assert!(code.unique);
    let benches = park
        .fields
        .iter()
        .find(|f| f.name == "bank_sayisi")
        .expect("the benches");
    assert_eq!(benches.alias.as_deref(), Some("Bank sayısı"));
    assert_eq!(benches.kind, LayerFieldKind::Integer);
    for ok in ["0", "20"] {
        assert!(check_value(benches, ok).is_ok(), "{ok}");
    }
    for bad in ["21", "yirmi"] {
        assert!(check_value(benches, bad).is_err(), "{bad}");
    }
    let route = read
        .tables
        .iter()
        .find(|t| t.name == "Bakım rotaları")
        .expect("the routes");
    assert!(
        route
            .fields
            .iter()
            .any(|f| f.name == "rota_kodu" && f.required)
    );
}

#[test]
fn other_programs_read_the_schema_extension_and_the_columns_rules() {
    let doc = drawing("ANK-01", "ANK-02", "0", "20", Some("RT-01"));
    let path = scratch("sema.gpkg");
    geopackage::write(&doc, &[], &path).expect("written");
    let gp = GeoPackage::open(&path, false).unwrap();
    let columns = gp.data_columns("Park sınırları").unwrap();
    let benches = columns
        .iter()
        .find(|(c, _)| c.column == "bank_sayisi")
        .expect("the benches' row");
    assert_eq!(benches.0.name.as_deref(), Some("Bank sayısı"));
    assert_eq!(
        benches.1,
        Some(Rule::Range {
            min: Some(0.0),
            min_included: true,
            max: Some(20.0),
            max_included: true
        })
    );
    let rules = gp.constraints("Park sınırları").unwrap();
    assert!(
        rules.uniques.iter().any(|u| u.columns == ["park_kodu"]),
        "{:?}",
        rules.uniques
    );
    let layers = gp.layers().unwrap();
    let route = layers.iter().find(|l| l.name == "Bakım rotaları").unwrap();
    assert!(
        route
            .columns
            .iter()
            .any(|c| c.name == "rota_kodu" && c.not_null)
    );
}

/// A file GDAL 3.13 wrote (`fixtures/geopackage/v1/gdal-sema.gpkg`): a whole-number field with its alternative name, a
/// 0–20 range domain, NOT NULL and UNIQUE; a text field with a coded domain. KentOS reads them as the field's rules and
/// the unique index as the layer's.
#[test]
fn another_programs_schema_extension_reads_as_field_rules() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/geopackage/v1/gdal-sema.gpkg");
    let read = geopackage::read(&path, None).expect("read");
    let park = read
        .tables
        .iter()
        .find(|t| t.name == "park")
        .expect("the table");
    let benches = park
        .fields
        .iter()
        .find(|f| f.name == "bank_sayisi")
        .expect("the benches");
    assert_eq!(benches.alias.as_deref(), Some("Bank sayısı"));
    assert_eq!(
        (benches.min.as_deref(), benches.max.as_deref()),
        (Some("0"), Some("20"))
    );
    assert!(!benches.min_exclusive && !benches.max_exclusive);
    assert!(benches.required, "NOT NULL");
    // UNIQUE is the layer's unique index (docs/adr/0758 §5).
    assert!(
        park.indexes
            .iter()
            .any(|x| x.unique && x.fields == ["bank_sayisi"]),
        "{:?}",
        park.indexes
    );
    let kind = park
        .fields
        .iter()
        .find(|f| f.name == "tur")
        .expect("the kind");
    let values: Vec<(&str, &str)> = kind
        .values
        .iter()
        .flatten()
        .map(|c| (c.code.as_str(), c.label.as_str()))
        .collect();
    assert_eq!(values, [("A", "Ağaç"), ("R", "Rögar")]);
}

#[test]
fn what_the_objects_break_of_the_rules_is_said() {
    let doc = drawing("ANK-01", "ANK-01", "0", "21", None);
    let path = scratch("bozuk.gpkg");
    let written = geopackage::write(&doc, &[], &path).expect("a field copy is still written");
    let notes = written.rule_notes.join("\n");
    assert!(
        notes.contains("zorunlu “Rota kodu” alanı 1 nesnede boş"),
        "{notes}"
    );
    assert!(
        notes.contains("“Bank sayısı” alanında 1 değer kuralına uymuyor"),
        "{notes}"
    );
    assert!(
        notes.contains("tekil “Park kodu” alanında 1 değer yineleniyor (ANK-01)"),
        "{notes}"
    );
    // The data breaks them, so the file does not claim them: the code column is nullable, the unique index plain.
    let gp = GeoPackage::open(&path, false).unwrap();
    let route = gp
        .layers()
        .unwrap()
        .into_iter()
        .find(|l| l.name == "Bakım rotaları")
        .unwrap();
    assert!(
        route
            .columns
            .iter()
            .all(|c| !c.not_null || c.name != "rota_kodu")
    );
    let rules = gp.constraints("Park sınırları").unwrap();
    assert!(rules.uniques.iter().all(|u| u.columns != ["park_kodu"]));
}

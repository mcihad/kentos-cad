//! A layer's indexes in a GeoPackage (docs/adr/0758 §5): written with CREATE INDEX over the columns their fields went
//! to and read back; a file GDAL and SQLite made (`fixtures/layer-indexes/v1/dizinli.gpkg`, written by
//! `scripts/fixtures/layer_indexes_cases.py --files`) gives its layers the indexes that are their fields'. With
//! `KENTOS_WRITE_GPKG=1` the written file goes to `fixtures/layer-indexes/v1/out/kentos-dizinler.gpkg`, which
//! `layer_indexes_cases.py --written` reads with SQLite alone. The timing:
//! `cargo test --release -p kentos-native-application --test all layer_indexes::timing -- --ignored --nocapture`.

use kentos_commands::{
    CommandResult, EntitiesSetProperties, FieldsWrite, ImportGpkg, LayerIndex, LayersIndexes,
    PolygonCreate,
};
use kentos_domain::Document;
use kentos_native_application::{
    ExecutionContext, fields_write, file_import_gpkg, geopackage, layers_indexes, polygon, set,
};
use serde_json::{Value, json};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn index(name: &str, fields: &[&str], unique: bool, nulls_equal: bool) -> LayerIndex {
    LayerIndex {
        name: name.into(),
        fields: fields.iter().map(|f| (*f).to_owned()).collect(),
        unique,
        nulls_equal,
    }
}

/// A parcel layer of `n` records, (ada, parsel) running through 1 000 islands, with `indexes`.
fn parcels(n: usize, indexes: &[LayerIndex]) -> Document {
    let records: Vec<Value> = (0..n)
        .map(|i| {
            let x = 487_000.0 + (i % 1000) as f64 * 20.0;
            let y = 4_420_000.0 + (i / 1000) as f64 * 20.0;
            json!({ "kind": "polygon", "id": i + 1, "layerId": "parsel",
                    "attrs": { "Ada": (101 + i / 1000).to_string(), "Parsel": (i % 1000 + 1).to_string(),
                               "Mahalle": if i % 2 == 0 { "Merkez" } else { "Yalı" },
                               "Bina no": format!("B{}", i + 1) },
                    "pts": [{ "x": x, "y": y }, { "x": x + 10.0, "y": y }, { "x": x + 10.0, "y": y + 10.0 },
                            { "x": x, "y": y + 10.0 }] })
        })
        .collect();
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Parseller",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 487_000.0, "y": 4_420_000.0 },
        "layers": [{ "id": "parsel", "name": "Parsel", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                     "fields": [{ "name": "Ada", "kind": "integer" }, { "name": "Parsel", "kind": "integer" },
                                { "name": "Mahalle", "kind": "text" },
                                { "id": "bina-no", "name": "Bina no", "kind": "text" }],
                     "indexes": indexes,
                     "children": [] }],
        "activeLayer": "parsel",
        "entities": records,
        "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

fn three() -> Vec<LayerIndex> {
    vec![
        index("Ada ve parsel", &["Ada", "Parsel"], true, false),
        index("Mahalle", &["Mahalle"], false, false),
        index("Bina", &["bina-no"], true, true),
    ]
}

#[test]
fn a_layers_indexes_go_into_its_geopackage_and_come_back() {
    let doc = parcels(30, &three());
    let dir = std::env::temp_dir().join(format!("kentos-dizinler-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("kentos-dizinler.gpkg");
    let _ = std::fs::remove_file(&path);
    let written = geopackage::write(&doc, &[], &path).expect("written");
    assert_eq!(written.tables.len(), 1);
    assert_eq!(written.tables[0].indexes, 3);
    // SQLite takes every NULL as different: the third index is written unique, and said.
    assert_eq!(written.index_notes.len(), 1, "{:?}", written.index_notes);
    assert!(
        written.index_notes[0].contains("“Bina”"),
        "{:?}",
        written.index_notes
    );
    let read = geopackage::read(&path, None).expect("read");
    assert_eq!(
        read.tables[0].indexes,
        vec![
            index("Ada ve parsel", &["Ada", "Parsel"], true, false),
            index("Mahalle", &["Mahalle"], false, false),
            // The file names columns, not the fields' stable ids; empty values are SQLite's.
            index("Bina", &["Bina no"], true, false),
        ]
    );
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        let out = root().join("fixtures/layer-indexes/v1/out");
        std::fs::create_dir_all(&out).expect("out");
        std::fs::copy(&path, out.join("kentos-dizinler.gpkg")).expect("copied");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_indexes_of_a_file_made_elsewhere_come_to_its_layers() {
    // A drawing without a parcel layer: the import makes the tables' layers (a layer of the drawing the table goes to
    // keeps its own indexes).
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Boş",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                     "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let mut doc = Document::from_snapshot(
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup"),
    )
    .expect("setup");
    let path = root().join("fixtures/layer-indexes/v1/dizinli.gpkg");
    let input: ImportGpkg =
        serde_json::from_value(json!({ "path": path.display().to_string() })).expect("input");
    let CommandResult::Completed { output, warnings } =
        file_import_gpkg::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!("not imported");
    };
    assert_eq!(output.indexes, 4, "{warnings:?}");
    let by_table = |table: &str| -> Vec<LayerIndex> {
        let id = &output
            .tables
            .iter()
            .find(|t| t.table == table)
            .expect("table")
            .layer;
        doc.layers().get(id).expect("layer").indexes.clone()
    };
    // A partial index, one over an expression, one over a BLOB and one over the fid are no layer's indexes; a name
    // the table's does not begin is kept whole.
    assert_eq!(
        by_table("parsel"),
        vec![
            index("Ada ve parsel", &["ada", "parsel"], true, false),
            index("Mahalle", &["mahalle"], false, false),
            index("baska_ad", &["parsel"], false, false),
        ]
    );
    // A UNIQUE constraint's index is named by its columns.
    assert_eq!(by_table("malik"), vec![index("tc", &["tc"], true, false)]);
    // The key holds in the drawing: a third (101, 5) is refused.
    let layer = output
        .tables
        .iter()
        .find(|t| t.table == "parsel")
        .expect("parsel")
        .layer
        .clone();
    let input: PolygonCreate = serde_json::from_value(json!({
        "layerId": layer,
        "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 1.0, "y": 0.0 }, { "x": 1.0, "y": 1.0 }],
        "attrs": { "ada": "101", "parsel": "5" }
    }))
    .expect("input");
    let CommandResult::Failed { error } =
        polygon::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!("a duplicate key was written");
    };
    assert_eq!(error.code, "not_unique");
}

#[test]
#[ignore = "a timing, run on its own in release"]
fn timing() {
    let n: usize = std::env::var("KENTOS_PERF_OBJECTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    let mut doc = parcels(n, &[]);
    let key = index("Ada ve parsel", &["Ada", "Parsel"], true, false);
    let t = std::time::Instant::now();
    let done = layers_indexes::execute(
        &mut ExecutionContext::new(&mut doc),
        LayersIndexes {
            layer: "parsel".into(),
            indexes: vec![key, index("Bina", &["bina-no"], true, true)],
            expected_revision: None,
        },
    );
    assert!(matches!(done, CommandResult::Completed { .. }), "{done:?}");
    let made = t.elapsed();
    let first = doc.uid(kentos_domain::Slot(1)).expect("uid").to_string();
    let set_one = |doc: &mut Document, parsel: &str| {
        let input: EntitiesSetProperties = serde_json::from_value(json!({
            "uids": [first], "attrs": { "Parsel": parsel }, "operation": "attributes"
        }))
        .expect("input");
        set::execute(&mut ExecutionContext::new(doc), input)
    };
    let t = std::time::Instant::now();
    let refused = set_one(&mut doc, "2");
    let asked = t.elapsed();
    assert!(matches!(refused, CommandResult::Failed { .. }));
    let t = std::time::Instant::now();
    let written = set_one(&mut doc, "5000");
    let wrote = t.elapsed();
    assert!(matches!(written, CommandResult::Completed { .. }));
    // A batch of 10 000 values, every tenth a key another record holds.
    let rows: Vec<Value> = doc
        .by_layer_with_uids("parsel")
        .take(10_000)
        .enumerate()
        .map(|(i, (uid, _))| {
            let parsel = if i % 10 == 0 {
                "2".to_owned()
            } else {
                format!("{}", 2000 + i)
            };
            json!({ "uid": uid.to_string(), "values": { "Parsel": parsel } })
        })
        .collect();
    let input: FieldsWrite = serde_json::from_value(json!({
        "layer": "parsel", "write": { "kind": "values", "rows": rows }, "policy": "valid"
    }))
    .expect("input");
    let t = std::time::Instant::now();
    let batch = fields_write::execute(&mut ExecutionContext::new(&mut doc), input);
    let batched = t.elapsed();
    let CommandResult::Completed { output, .. } = batch else {
        panic!("not written: {batch:?}");
    };
    println!(
        "{n} kayıt: iki tekil dizin kuruldu {:.1} ms · tek kayıt reddi {:.1} ms · tek kayıt yazımı {:.1} ms · \
         10 000 değer ({} geçersiz) {:.1} ms",
        made.as_secs_f64() * 1e3,
        asked.as_secs_f64() * 1e3,
        wrote.as_secs_f64() * 1e3,
        output.invalid,
        batched.as_secs_f64() * 1e3
    );
}

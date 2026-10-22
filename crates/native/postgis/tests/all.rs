// A bound layer on PostGIS (docs/adr/0505), end to end through the product commands. Runs against the throwaway
// database `scripts/fixtures/postgis_cases.py` starts (docker, never the developer's 5432), which loads QGIS's
// GeoPackage into the schema `kadastro` with GDAL's ogr2ogr; without KENTOS_PG_URL it is skipped. What it saw goes to
// KENTOS_PG_OUT as JSON, and the script checks it, and the database after the writes, with psycopg2.

use kentos_contracts::CallOp;
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn run(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), command, op, input).expect("runs")
}

fn ok(answer: &Value) -> &Value {
    assert_eq!(answer["status"], "completed", "{answer}");
    &answer["output"]
}

fn doc() -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "PostGIS",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

fn state(doc: &mut Document) -> Value {
    ok(&run(
        doc,
        "cad.sources.describe",
        CallOp::Execute,
        json!({}),
    ))["links"][0]
        .clone()
}

#[test]
fn without_a_provider_or_with_a_password_nothing_is_reached() {
    let mut d = doc();
    let refused = run(
        &mut d,
        "cad.sources.describe",
        CallOp::Execute,
        json!({ "path": "postgresql://cbs:gizli@db/kent" }),
    );
    assert_eq!(refused["error"]["code"], "invalid_address");
}

#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_postgis_table_is_bound_written_and_read_again() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let mut d = doc();
    let mut seen = serde_json::Map::new();
    let tables = ok(&run(
        &mut d,
        "cad.sources.describe",
        CallOp::Execute,
        json!({ "path": url }),
    ))
    .clone();
    seen.insert("tables".into(), tables["tables"].clone());
    let linked = ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "kadastro.parsel" } }),
    ))
    .clone();
    seen.insert("linked".into(), linked.clone());
    let layer = linked["link"]["layer"].as_str().expect("layer").to_owned();
    let objects: Vec<Value> = d
        .by_layer_with_uids(&layer)
        .map(|(u, e)| json!({ "uid": u.to_string(), "kind": serde_json::to_value(e).expect("json")["kind"], "attrs": e.base().attrs }))
        .collect();
    seen.insert("objects".into(), json!(objects));
    let first = state(&mut d);
    assert_eq!(
        (
            first["edited"].clone(),
            first["added"].clone(),
            first["modified"].clone(),
            first["removed"].clone()
        ),
        (json!(false), json!(0), json!(0), json!(0)),
        "a bound table matches its layer: {first}"
    );
    // Edit: a parcel renamed, one deleted, a triangle drawn, a field the table lacks.
    let uids: Vec<String> = objects
        .iter()
        .map(|o| o["uid"].as_str().expect("uid").to_owned())
        .collect();
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uids[0]], "attrs": { "ada": "Yeni Mahalle", "durum": "onaylı" }, "operation": "attributes" }),
    ));
    ok(&run(
        &mut d,
        "cad.entities.delete",
        CallOp::Execute,
        json!({ "uids": [uids[1]] }),
    ));
    let drawn = ok(&run(
        &mut d,
        "cad.polygon.create",
        CallOp::Execute,
        json!({ "layerId": layer, "pts": [{ "x": 487500.0, "y": 4420000.0 }, { "x": 487510.0, "y": 4420000.0 },
                                         { "x": 487510.0, "y": 4420010.0 }] }),
    ))["uid"]
        .as_str()
        .expect("uid")
        .to_owned();
    let mut fields = d.layers().get(&layer).expect("layer").fields.clone();
    fields.push(serde_json::from_value(json!({ "name": "durum", "kind": "text" })).expect("field"));
    d.set_layer_fields(&layer, fields, &[]).expect("fields");
    let written = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    ))
    .clone();
    assert_eq!(
        (
            written["added"].clone(),
            written["modified"].clone(),
            written["removed"].clone(),
            written["columns"].clone()
        ),
        (json!(1), json!(1), json!(1), json!(["durum"])),
        "{written}"
    );
    seen.insert("written".into(), written);
    seen.insert("drawn".into(), json!(drawn));
    let after = state(&mut d);
    assert_eq!(
        (
            after["changed"].clone(),
            after["edited"].clone(),
            after["modified"].clone()
        ),
        (json!(false), json!(false), json!(0)),
        "{after}"
    );
    // A fresh drawing binds the table again: the triangle comes back with its id (kentos_uids).
    let mut fresh = doc();
    ok(&run(
        &mut fresh,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "kadastro.parsel" } }),
    ));
    let fresh_layer = fresh.settings().sources[0].layer.clone();
    assert!(
        fresh
            .by_layer_with_uids(&fresh_layer)
            .any(|(u, _)| u.to_string() == drawn)
    );
    // Another writer changes the table: the old binding's write is refused, its read takes the change.
    ok(&run(
        &mut fresh,
        "cad.entities.delete",
        CallOp::Execute,
        json!({ "uids": [uids[0]] }),
    ));
    ok(&run(
        &mut fresh,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": fresh_layer, "direction": "write" }),
    ));
    let changed = state(&mut d);
    assert_eq!(changed["changed"], true, "{changed}");
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [drawn], "attrs": { "ada": "çakışma" }, "operation": "attributes" }),
    ));
    let refused = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(refused["error"]["code"], "source_changed", "{refused}");
    let read = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "read", "discard": true }),
    ))
    .clone();
    assert_eq!(
        (read["removed"].clone(), read["modified"].clone()),
        (json!(1), json!(1)),
        "{read}"
    );
    // A table in another system: the WGS 84 point bound with reproject.
    let refused = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Plan,
        json!({ "edit": { "kind": "link", "path": url, "table": "kadastro.yer" } }),
    );
    assert_eq!(refused["error"]["code"], "crs_mismatch", "{refused}");
    ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "kadastro.yer", "reproject": true } }),
    ));
    let yer = state_of_layer(&mut d, "import-yer");
    assert_eq!(
        (yer["edited"].clone(), yer["modified"].clone()),
        (json!(false), json!(0)),
        "{yer}"
    );
    if let Ok(out) = std::env::var("KENTOS_PG_OUT") {
        std::fs::write(
            out,
            serde_json::to_string_pretty(&Value::Object(seen)).expect("json"),
        )
        .expect("written");
    }
}

fn state_of_layer(doc: &mut Document, layer: &str) -> Value {
    let links = ok(&run(
        doc,
        "cad.sources.describe",
        CallOp::Execute,
        json!({}),
    ))["links"]
        .clone();
    links
        .as_array()
        .expect("links")
        .iter()
        .find(|l| l["link"]["layer"] == layer)
        .cloned()
        .unwrap_or(Value::Null)
}

/// The same questions of `fixtures/sources/v1/sorgu.gpkg` and of its tables loaded into the schema `sorgu` by
/// ogr2ogr (fids kept; `nokta`'s one text in a number column, which PostgreSQL cannot hold, made NULL): the same rows,
/// keys, values, pages, counts and statistics (docs/adr/0581 §2). The questions leave that one value out of their
/// answers.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn the_queries_answer_alike_from_postgis() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/sources/v1/sorgu.gpkg")
        .display()
        .to_string();
    let questions = [
        json!({ "table": "nokta", "fields": ["ad", "tur", "no"], "limit": 100 }),
        json!({ "table": "nokta", "fields": ["ad", "no"], "limit": 100, "cursor": "k:100" }),
        json!({ "table": "nokta", "filter": "tur = 'rögar'", "count": true, "fields": ["ad"] }),
        json!({ "table": "nokta", "filter": "no = 50", "fields": ["ad", "no"] }),
        json!({ "table": "nokta", "filter": "kot arasında 110 ve 112", "fields": ["kot"], "count": true }),
        json!({ "table": "nokta", "filter": "kot < '101'", "fields": ["kot"] }),
        json!({ "table": "nokta", "filter": "tur boş", "fields": ["ad"], "count": true, "limit": 5 }),
        json!({ "table": "nokta", "filter": "tur içinde ('direk', 'ağaç') and no > 40", "fields": ["tur", "no"],
                "count": true, "limit": 7 }),
        json!({ "table": "nokta", "filter": "tur = 'direk' veya no <= 3", "fields": ["ad"], "statistics": ["no"] }),
        json!({ "table": "nokta", "bbox": [487090.0, 4420030.0, 487150.0, 4420070.0], "fields": ["ad"],
                "geometry": true }),
        json!({ "table": "nokta", "near": { "at": [487100.0, 4420100.0], "tolerance": 25.0 }, "fields": ["ad"] }),
        json!({ "table": "bina", "bbox": [487055.0, 4420595.0, 487125.0, 4420640.0] }),
        json!({ "table": "bina", "filter": "($alan > 150)", "count": true, "fields": ["ad"] }),
        json!({ "table": "kayit", "statistics": ["tutar"], "limit": 0 }),
        json!({ "table": "kayit", "filter": "tutar > 100", "statistics": ["tutar"], "limit": 3, "fields": ["ad"] }),
    ];
    let mut d = doc();
    for (k, q) in questions.iter().enumerate() {
        let table = q["table"].as_str().expect("a table");
        let mut on_file = q.clone();
        on_file["path"] = json!(file);
        let mut on_db = q.clone();
        on_db["path"] = json!(url);
        on_db["table"] = json!(format!("sorgu.{table}"));
        let a = run(&mut d, "cad.sources.query", CallOp::Execute, on_file);
        let b = run(&mut d, "cad.sources.query", CallOp::Execute, on_db);
        let (a, b) = (ok(&a), ok(&b));
        let rows = |v: &Value| -> Vec<Value> {
            v["rows"]
                .as_array()
                .expect("rows")
                .iter()
                .map(|r| json!([r["key"], r["attrs"], r["wkt"]]))
                .collect()
        };
        assert_eq!(rows(a), rows(b), "{k}: {q}");
        for part in ["total", "next", "statistics"] {
            assert_eq!(a[part], b[part], "{k}: {q}: {part}");
        }
        assert_eq!(b["source"]["kind"], "postgis");
        eprintln!("{k}: {} satır; PostGIS planı {}", rows(b).len(), b["plan"]);
    }
}

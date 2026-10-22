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
    // Another writer deletes a parcel the old binding then changes: refused, with that object's id (docs/adr/0582 §2).
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
        json!({ "uids": [uids[0]], "attrs": { "ada": "çakışma" }, "operation": "attributes" }),
    ));
    let refused = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(refused["error"]["code"], "source_changed", "{refused}");
    assert_eq!(refused["error"]["uids"], json!([uids[0]]), "{refused}");
    let read = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "read", "discard": true }),
    ))
    .clone();
    assert_eq!(read["removed"], json!(1), "{read}");
    // Two writers on different rows: both go through; the second is told of the first's row and reads it.
    ok(&run(
        &mut fresh,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [drawn], "attrs": { "ada": "taze" }, "operation": "attributes" }),
    ));
    ok(&run(
        &mut fresh,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": fresh_layer, "direction": "write" }),
    ));
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uids[2]], "attrs": { "ada": "eski" }, "operation": "attributes" }),
    ));
    let both = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    );
    let written = ok(&both).clone();
    assert_eq!(written["modified"], json!(1), "{both}");
    assert_eq!(both["warnings"][0]["code"], "source_others", "{both}");
    assert_eq!(state(&mut d)["changed"], true);
    let read = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "read" }),
    ))
    .clone();
    assert_eq!(read["modified"], json!(1), "{read}");
    // The same row by both: the second is refused with it.
    ok(&run(
        &mut fresh,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": fresh_layer, "direction": "read", "discard": true }),
    ));
    ok(&run(
        &mut fresh,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uids[2]], "attrs": { "ada": "taze" }, "operation": "attributes" }),
    ));
    ok(&run(
        &mut fresh,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": fresh_layer, "direction": "write" }),
    ));
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uids[2]], "attrs": { "ada": "benim" }, "operation": "attributes" }),
    ));
    let refused = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(refused["error"]["code"], "source_changed", "{refused}");
    assert_eq!(refused["error"]["uids"], json!([uids[2]]), "{refused}");
    ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "read", "discard": true }),
    ));
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

/// The same ordered questions of `fixtures/sources/v1/sira.gpkg` and of its table loaded into the schema `sira` by
/// ogr2ogr (docs/adr/0591): every page of each the same rows and keys in the same order, the same `next` (the raw
/// value: a number, a date's text, 0 or 1), the same totals. The file's orders are the shared cases'
/// (`source_order_cases.py`); PostgreSQL orders the real, the integer, the date and the boolean itself.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn orders_answer_alike_from_postgis() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/sources/v1/sira.gpkg")
        .display()
        .to_string();
    let questions = [
        json!({ "order": { "field": "puan" } }),
        json!({ "order": { "field": "puan", "descending": true }, "fields": ["ad", "puan"] }),
        json!({ "order": { "field": "sayi" }, "filter": "puan > 100", "count": true }),
        json!({ "order": { "field": "tarih", "descending": true }, "fields": ["tarih"] }),
        json!({ "order": { "field": "acik" }, "fields": ["acik", "sayi"] }),
        json!({ "order": { "field": "ad" }, "fields": ["ad"] }),
        json!({ "order": { "field": "ad", "descending": true }, "filter": "sayi < 0", "count": true }),
        json!({ "order": { "field": "puan" }, "filter": "ad gibi '% 7'", "fields": ["ad", "puan"] }),
    ];
    let mut d = doc();
    for (k, q) in questions.iter().enumerate() {
        let mut on_file = q.clone();
        on_file["path"] = json!(file);
        on_file["table"] = json!("olay");
        on_file["limit"] = json!(173);
        let mut on_db = on_file.clone();
        on_db["path"] = json!(url);
        on_db["table"] = json!("sira.olay");
        let mut read = 0;
        loop {
            let a = run(
                &mut d,
                "cad.sources.query",
                CallOp::Execute,
                on_file.clone(),
            );
            let b = run(&mut d, "cad.sources.query", CallOp::Execute, on_db.clone());
            let (a, b) = (ok(&a), ok(&b));
            let rows = |v: &Value| -> Vec<Value> {
                v["rows"]
                    .as_array()
                    .expect("rows")
                    .iter()
                    .map(|r| json!([r["key"], r["attrs"]]))
                    .collect()
            };
            assert_eq!(rows(a), rows(b), "{k}: {q} ({read} okundu)");
            for part in ["total", "next"] {
                assert_eq!(a[part], b[part], "{k}: {q}: {part}");
            }
            assert_eq!(a["plan"]["order"], b["plan"]["order"], "{k}: {q}");
            read += rows(a).len();
            match a["next"].as_str() {
                Some(next) => {
                    on_file["cursor"] = json!(next);
                    on_db["cursor"] = json!(next);
                    on_file.as_object_mut().expect("an object").remove("count");
                    on_db.as_object_mut().expect("an object").remove("count");
                }
                None => break,
            }
        }
        eprintln!("{k}: {read} satır sırayla aynı");
    }
}

/// Views, queries and areas (docs/adr/0582): a view and a query bound read-only, keyed by `fid`; a view without a key
/// refused; a query that would be two statements refused; a table bound by an area holds only the area's rows, and its
/// area moves.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn views_queries_and_areas() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let mut d = doc();
    let tables = ok(&run(
        &mut d,
        "cad.sources.describe",
        CallOp::Execute,
        json!({ "path": url }),
    ))
    .clone();
    let view = tables["tables"]
        .as_array()
        .expect("tables")
        .iter()
        .find(|t| t["name"] == "sorgu.nokta_v")
        .cloned()
        .expect("the view is listed");
    assert_eq!(view["capabilities"]["update"], false, "{view}");
    assert!(
        !tables["tables"]
            .as_array()
            .expect("tables")
            .iter()
            .any(|t| t["name"] == "sorgu.anahtarsiz"),
        "a view without a key is not listed"
    );
    // The view: bound read-only.
    let linked = ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "sorgu.nokta_v" } }),
    ))
    .clone();
    assert_eq!(linked["objects"], json!(50), "{linked}");
    assert_eq!(linked["link"]["writable"], false, "{linked}");
    let layer = linked["link"]["layer"].as_str().expect("layer").to_owned();
    let first = d
        .by_layer_with_uids(&layer)
        .next()
        .map(|(u, _)| u.to_string())
        .expect("a point");
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [first], "attrs": { "ad": "yeni" }, "operation": "attributes" }),
    ));
    let refused = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(refused["error"]["code"], "source_readonly", "{refused}");
    // A view without a key, and a query of two statements, refused.
    let refused = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "sorgu.anahtarsiz" } }),
    );
    assert_eq!(refused["error"]["code"], "no_row_key", "{refused}");
    let refused = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "SELECT fid, geom FROM sorgu.nokta; DROP TABLE sorgu.nokta" } }),
    );
    assert_eq!(refused["error"]["code"], "import_unreadable", "{refused}");
    let refused = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "SELECT 1 AS fid UNION ALL SELECT 1" } }),
    );
    assert_eq!(refused["error"]["code"], "row_key_not_unique", "{refused}");
    // A query: its rows, the same ids twice.
    let query = "SELECT fid, ad, tur, geom FROM sorgu.nokta WHERE tur = 'direk'";
    let linked = ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": query } }),
    ))
    .clone();
    assert_eq!(linked["objects"], json!(50), "{linked}");
    let asked = ok(&run(
        &mut d,
        "cad.sources.query",
        CallOp::Execute,
        json!({ "path": url, "table": query, "filter": "ad içinde ('N1', 'N6', 'N7', 'N11')", "count": true,
                "fields": ["ad"] }),
    ))
    .clone();
    // N7 is not a pole: three of the four.
    assert_eq!(asked["total"], json!(3), "{asked}");
    // A table by an area: the area's rows only, then the area moved.
    let mut part = doc();
    let linked = ok(&run(
        &mut part,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "sorgu.nokta",
                          "extent": [486995.0, 4419995.0, 487105.0, 4420025.0] } }),
    ))
    .clone();
    assert_eq!(linked["objects"], json!(12), "{linked}");
    let id = linked["link"]["id"].as_str().expect("id").to_owned();
    let moved = ok(&run(
        &mut part,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "extent", "id": id, "extent": [486995.0, 4419995.0, 487105.0, 4420065.0] } }),
    ))
    .clone();
    assert_eq!(moved["link"]["extent"][3], json!(4420065.0), "{moved}");
    let layer = part.settings().sources[0].layer.clone();
    assert_eq!(part.by_layer_with_uids(&layer).count(), 24);
}

/// The measurement of docs/adr/0582 §5 on `zaman.parsel`, a million parcels `postgis_cases.py --timing` makes with
/// `generate_series` (a 1000 × 1000 grid of 20 m squares, a GiST index): the first map by the view's area, the area's
/// state, a row written with its check, a question's first page and the whole table's statistics. Run in release:
/// `KENTOS_PG_TIMING=1`.
/// Binds `table` and returns its layer and its first object's lasting id.
fn bind(d: &mut Document, url: &str, table: &str) -> (String, String) {
    let linked = ok(&run(
        d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": table } }),
    ))
    .clone();
    let layer = linked["link"]["layer"].as_str().expect("layer").to_owned();
    let uid = d
        .by_layer_with_uids(&layer)
        .map(|(u, _)| u.to_string())
        .next()
        .expect("a row");
    (layer, uid)
}

fn rename(d: &mut Document, uid: &str, ad: &str) {
    ok(&run(
        d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uid], "attrs": { "ad": ad }, "operation": "attributes" }),
    ));
}

/// The value of `ad` in `table`'s first row as a fresh drawing reads it.
fn ad_in(url: &str, table: &str) -> String {
    let mut d = doc();
    let (layer, _) = bind(&mut d, url, table);
    d.by_layer_with_uids(&layer)
        .next()
        .and_then(|(_, e)| e.base().attrs.get("ad").cloned())
        .unwrap_or_default()
}

#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn layers_of_one_database_go_in_together() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let mut d = doc();
    let (duz, duz_uid) = bind(&mut d, &url, "boyut.duz");
    let (cizgi, cizgi_uid) = bind(&mut d, &url, "boyut.cizgi");
    // Both changed, written in one transaction (docs/adr/0584 §1).
    rename(&mut d, &duz_uid, "birlikte");
    rename(&mut d, &cizgi_uid, "birlikte");
    let written = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": duz, "direction": "write", "with": [cizgi] }),
    ))
    .clone();
    assert_eq!(written["modified"], json!(2), "{written}");
    assert_eq!(
        written["layers"].as_array().map(Vec::len),
        Some(2),
        "{written}"
    );
    assert_eq!(ad_in(&url, "boyut.duz"), "birlikte");
    assert_eq!(ad_in(&url, "boyut.cizgi"), "birlikte");
    // Someone else changes the line's row; the next write of both is refused whole.
    let mut other = doc();
    let (other_cizgi, other_uid) = bind(&mut other, &url, "boyut.cizgi");
    rename(&mut other, &other_uid, "baskasi");
    ok(&run(
        &mut other,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": other_cizgi, "direction": "write" }),
    ));
    rename(&mut d, &duz_uid, "ikinci");
    rename(&mut d, &cizgi_uid, "ikinci");
    let refused = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": duz, "direction": "write", "with": [cizgi] }),
    );
    assert_eq!(refused["status"], "conflict", "{refused}");
    assert_eq!(refused["error"]["code"], "source_changed", "{refused}");
    assert_eq!(refused["error"]["path"], "with", "{refused}");
    assert_eq!(refused["error"]["uids"], json!([cizgi_uid]), "{refused}");
    assert_eq!(
        ad_in(&url, "boyut.duz"),
        "birlikte",
        "the plane's table is not written either"
    );
    assert_eq!(ad_in(&url, "boyut.cizgi"), "baskasi");
    // A file and a database: no transaction spans two sources.
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/geopackage/v1/boyut.gpkg");
    let copy = std::env::temp_dir().join(format!("kentos-birlikte-{}.gpkg", std::process::id()));
    std::fs::copy(&file, &copy).expect("a copy");
    let (kotlu_file, _) = bind(&mut d, copy.to_str().expect("path"), "kotlu");
    let split = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": duz, "direction": "write", "with": [kotlu_file] }),
    );
    let _ = std::fs::remove_file(&copy);
    assert_eq!(split["error"]["code"], "transaction_split", "{split}");
    // A Z column takes no object without elevations (docs/adr/0584 §2).
    let (kotlu, kotlu_uid) = bind(&mut d, &url, "boyut.kotlu");
    let flat = json!({ "kind": "polygon", "pts": [
        { "x": 487000.0, "y": 4420000.0 }, { "x": 487010.0, "y": 4420000.0 },
        { "x": 487010.0, "y": 4420010.0 }, { "x": 487000.0, "y": 4420010.0 }],
        "zs": [null, null, null, null] });
    ok(&run(
        &mut d,
        "cad.entities.edit",
        CallOp::Execute,
        json!({ "operation": "elevation", "changes": [{ "kind": "update", "uid": kotlu_uid, "geometry": flat }] }),
    ));
    let z = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": kotlu, "direction": "write" }),
    );
    assert_eq!(z["error"]["code"], "geometry_mismatch", "{z}");
}

/// The row of `table` whose `ad` is `ad`: its lasting id and its WKT, as a question to the database reads them.
fn row_in(url: &str, table: &str, ad: &str) -> (String, String) {
    let mut d = doc();
    let asked = ok(&run(
        &mut d,
        "cad.sources.query",
        CallOp::Execute,
        json!({ "path": url, "table": table, "filter": format!("ad = '{ad}'"), "fields": ["ad"], "geometry": true }),
    ))
    .clone();
    let row = &asked["rows"][0];
    (
        row["uid"].as_str().expect("uid").to_owned(),
        row["wkt"].as_str().expect("wkt").to_owned(),
    )
}

/// The part's layer of `table` (only A in its area) with A's corner at (dx, dy) from the origin moved by `by`.
fn part_moved(url: &str, table: &str, corner: (f64, f64), by: (f64, f64)) -> (Document, String) {
    let mut d = doc();
    let linked = ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": table,
                          "extent": [486990.0, 4419990.0, 487005.0, 4420030.0] } }),
    ))
    .clone();
    assert_eq!(linked["objects"], json!(1), "{linked}");
    let layer = linked["link"]["layer"].as_str().expect("layer").to_owned();
    let a = d
        .by_layer_with_uids(&layer)
        .map(|(u, _)| u.to_string())
        .next()
        .expect("A");
    let (x, y) = (487000.0 + corner.0, 4420000.0 + corner.1);
    ok(&run(
        &mut d,
        "cad.entities.stretch",
        CallOp::Execute,
        json!({ "window": { "min": { "x": x - 0.5, "y": y - 0.5 }, "max": { "x": x + 0.5, "y": y + 0.5 } },
                "dx": by.0, "dy": by.1, "uids": [a] }),
    ));
    (d, layer)
}

// docs/adr/0585: a part of a PostGIS table written with its neighbours outside the area (komsu-gecerlilik.gpkg in the
// schema komsu): B follows A's moved corner in the same transaction; a neighbour someone changes between the read and
// the lock refuses the write; a neighbour the account sees but may not change (row security) refuses it too.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn neighbours_outside_the_area_follow_in_the_database() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    // B follows A's corner (10, 4) moved one metre east.
    let (mut d, layer) = part_moved(&url, "komsu.parsel", (10.0, 4.0), (1.0, 0.0));
    let input = json!({ "layer": layer, "direction": "write", "topology": "strict" });
    let plan = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        input.clone(),
    ))
    .clone();
    assert_eq!(plan["followed"], json!(1), "{plan}");
    let (_, b_before) = row_in(&url, "komsu.parsel", "B");
    assert!(
        b_before.contains("487010 4420004"),
        "a plan writes nothing: {b_before}"
    );
    let written = ok(&run(&mut d, "cad.sources.sync", CallOp::Execute, input)).clone();
    assert_eq!(
        (written["modified"].clone(), written["followed"].clone()),
        (json!(1), json!(1)),
        "{written}"
    );
    let (_, b) = row_in(&url, "komsu.parsel", "B");
    assert!(
        b.contains("487011 4420004") && !b.contains("487010 4420004"),
        "{b}"
    );
    // A corner B would cross itself over: strict writes nothing.
    let (mut d, layer) = part_moved(&url, "komsu.parsel", (11.0, 4.0), (14.0, -2.0));
    let refused = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write", "topology": "strict" }),
    );
    assert_eq!(
        refused["error"]["code"], "outside_neighbour_invalid",
        "{refused}"
    );
    assert_eq!(row_in(&url, "komsu.parsel", "B").1, b);
    // C changed by someone else after the write read it, before the lock: the write waits for the lock, finds C
    // changed and writes nothing; written again, it follows C as it is now.
    let (c_uid, _) = row_in(&url, "komsu.parsel", "C");
    let (mut d, layer) = part_moved(&url, "komsu.parsel", (10.0, 6.0), (0.0, 1.0));
    let other = std::thread::spawn({
        let url = url.clone();
        move || {
            std::process::Command::new("psql")
                .args([
                    url.as_str(),
                    "-q",
                    "-v",
                    "ON_ERROR_STOP=1",
                    "-c",
                    "BEGIN; UPDATE komsu.parsel SET ad = 'C2' WHERE ad = 'C'; SELECT pg_sleep(3); COMMIT;",
                ])
                .output()
                .expect("psql runs")
        }
    });
    std::thread::sleep(std::time::Duration::from_millis(1200));
    let input = json!({ "layer": layer, "direction": "write", "topology": "strict" });
    let raced = run(&mut d, "cad.sources.sync", CallOp::Execute, input.clone());
    let psql = other.join().expect("the other editor");
    assert!(
        psql.status.success(),
        "{}",
        String::from_utf8_lossy(&psql.stderr)
    );
    assert_eq!(raced["status"], "conflict", "{raced}");
    assert_eq!(raced["error"]["code"], "source_changed", "{raced}");
    assert_eq!(raced["error"]["path"], "topology", "{raced}");
    assert_eq!(raced["error"]["uids"], json!([c_uid]), "{raced}");
    let again = ok(&run(&mut d, "cad.sources.sync", CallOp::Execute, input)).clone();
    assert_eq!(again["followed"], json!(1), "{again}");
    let (_, c) = row_in(&url, "komsu.parsel", "C2");
    assert!(c.contains("487010 4420007"), "{c}");
    // Row security: the account may change every row but B. With the neighbours, nothing is written; without them,
    // A alone is.
    let narrow = url.replacen("postgres@", "kisitli@", 1);
    let (b_uid, b_kept) = row_in(&narrow, "komsu.kisitli", "B");
    let (mut d, layer) = part_moved(&narrow, "komsu.kisitli", (10.0, 4.0), (1.0, 0.0));
    let denied = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write", "topology": "strict" }),
    );
    assert_eq!(denied["error"]["code"], "rows_denied", "{denied}");
    assert_eq!(denied["error"]["path"], "topology", "{denied}");
    assert_eq!(denied["error"]["uids"], json!([b_uid]), "{denied}");
    assert_eq!(row_in(&narrow, "komsu.kisitli", "B").1, b_kept);
    assert!(
        row_in(&narrow, "komsu.kisitli", "A")
            .1
            .contains("487010 4420004")
    );
    let alone = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    ))
    .clone();
    assert_eq!(alone["modified"], json!(1), "{alone}");
    assert!(
        row_in(&narrow, "komsu.kisitli", "A")
            .1
            .contains("487011 4420004")
    );
}

#[test]
#[ignore = "a measurement on the throwaway PostGIS of scripts/fixtures/postgis_cases.py --timing"]
fn postgis_timing() {
    let (Ok(url), Ok(_)) = (
        std::env::var("KENTOS_PG_URL"),
        std::env::var("KENTOS_PG_TIMING"),
    ) else {
        return;
    };
    kentos_postgis::register();
    let report = |what: &str, t: std::time::Instant| {
        println!("{what:<56} {:>10.1} ms", t.elapsed().as_secs_f64() * 1000.0);
    };
    let mut d = doc();
    // A view of 400 × 300 m in the middle of the city.
    let (mx, my) = (510_000.0, 4_410_000.0);
    let t = std::time::Instant::now();
    let linked = ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "zaman.parsel",
                          "extent": [mx, my, mx + 400.0, my + 300.0] } }),
    ))
    .clone();
    report(
        &format!("alanla bağlama, ilk harita ({} nesne)", linked["objects"]),
        t,
    );
    let layer = linked["link"]["layer"].as_str().expect("layer").to_owned();
    let t = std::time::Instant::now();
    let st = state(&mut d);
    report(&format!("alanın durumu (changed {})", st["changed"]), t);
    let first = d
        .by_layer_with_uids(&layer)
        .next()
        .map(|(u, _)| u.to_string())
        .expect("a parcel");
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [first], "attrs": { "ad": "ölçüldü" }, "operation": "attributes" }),
    ));
    let t = std::time::Instant::now();
    let written = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    ))
    .clone();
    report(
        &format!(
            "bir satırın yazılması, satır denetimiyle ({})",
            written["modified"]
        ),
        t,
    );
    let t = std::time::Instant::now();
    let page = ok(&run(
        &mut d,
        "cad.sources.query",
        CallOp::Execute,
        json!({ "path": url, "table": "zaman.parsel", "bbox": [mx, my, mx + 400.0, my + 300.0],
                "filter": "parsel > 500", "fields": ["ad"], "limit": 100 }),
    ))
    .clone();
    report(
        &format!("sorgu: kutu ve süzgeç, ilk sayfa ({} okundu)", page["read"]),
        t,
    );
    let t = std::time::Instant::now();
    let all = ok(&run(
        &mut d,
        "cad.sources.query",
        CallOp::Execute,
        json!({ "path": url, "table": "zaman.parsel", "filter": "ada > 500", "statistics": ["parsel"],
                "count": true, "limit": 0 }),
    ))
    .clone();
    report(
        &format!(
            "sorgu: bütün tablo, süzgeçli istatistik ({} okundu)",
            all["read"]
        ),
        t,
    );
}

const OWNER_1: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001";
const OWNER_2: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002";
const OWNER_3: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003";
const PARCEL_1: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2001";
const PARCEL_2: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2002";
const PARCEL_3: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2003";

/// The owners and parcels of `schema` bound, the parcels' `malikler` a many-to-many reference to the owners.
fn owners_and_parcels(url: &str, schema: &str) -> (Document, String, String) {
    let mut d = doc();
    let (owners, _) = bind(&mut d, url, &format!("{schema}.malik"));
    let (parcels, _) = bind(&mut d, url, &format!("{schema}.parsel"));
    ok(&run(
        &mut d,
        "cad.layers.edit",
        CallOp::Execute,
        json!({ "operation": "fields", "layer": parcels, "fields": [
            { "name": "ada", "kind": "text" }, { "name": "parsel_no", "kind": "text" },
            { "name": "malikler", "kind": "text", "multiple": true } ] }),
    ));
    ok(&run(
        &mut d,
        "cad.relations.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "create", "relation": { "id": "parsel-malik", "name": "Parselin malikleri",
            "parent": owners, "child": parcels, "field": "malikler" } } }),
    ));
    (d, owners, parcels)
}

fn relate(d: &mut Document, op: CallOp, action: &str) -> Value {
    run(
        d,
        "cad.sources.relate",
        op,
        json!({ "relation": "parsel-malik", "action": action }),
    )
}

fn references(d: &mut Document, uid: &str, value: &str) {
    ok(&run(
        d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uid], "attrs": { "malikler": value }, "operation": "attributes" }),
    ));
}

fn write_layer(d: &mut Document, layer: &str) -> Value {
    run(
        d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    )
}

fn junction(url: &str) -> Vec<(i64, i64)> {
    kentos_native_application::provider::postgis()
        .expect("registered")
        .mapping_rows(url, "iliski.parsel_malik")
        .expect("rows")
}

/// docs/adr/0590: KentOS's junction table in the database, made from the parcels' references, rewritten in the tables'
/// transaction (a new parcel by the key the same transaction gives it), read back in the references' order, refused
/// over a reference to a deleted owner; someone else's junction table reported, never written over.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn many_to_many_references_keep_a_junction_table() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let (mut d, owners, parcels) = owners_and_parcels(&url, "iliski");
    let state = |a: &Value| ok(a)["state"].as_str().unwrap_or_default().to_owned();
    assert_eq!(state(&relate(&mut d, CallOp::Execute, "check")), "text");
    // The plan says what the table would hold and writes nothing.
    let planned = relate(&mut d, CallOp::Plan, "map");
    assert_eq!(ok(&planned)["rows"], json!(4), "{planned}");
    assert_eq!(
        ok(&planned)["mapping"],
        json!("iliski.parsel_malik"),
        "{planned}"
    );
    assert_eq!(state(&relate(&mut d, CallOp::Execute, "check")), "text");
    let mapped = relate(&mut d, CallOp::Execute, "map");
    assert_eq!(state(&mapped), "mapped", "{mapped}");
    assert_eq!(junction(&url), vec![(1, 1), (1, 2), (2, 2), (2, 3)]);
    let again = relate(&mut d, CallOp::Execute, "map");
    assert_eq!(again["error"]["code"], "relation_mapped", "{again}");
    // A reference added and one taken off: the rows follow in the parcels' write.
    references(&mut d, PARCEL_3, OWNER_3);
    let written = ok(&write_layer(&mut d, &parcels)).clone();
    assert_eq!(
        written["relations"],
        json!([{ "relation": "parsel-malik", "added": 1, "removed": 0 }]),
        "{written}"
    );
    references(&mut d, PARCEL_1, OWNER_2);
    let written = ok(&write_layer(&mut d, &parcels)).clone();
    assert_eq!(written["relations"][0]["removed"], json!(1), "{written}");
    // The parent order of a child is kept.
    references(&mut d, PARCEL_2, &format!("{OWNER_3}; {OWNER_2}"));
    ok(&write_layer(&mut d, &parcels));
    assert_eq!(junction(&url), vec![(1, 2), (2, 3), (2, 2), (3, 3)]);
    // A new parcel's row goes in by the key its insert gets.
    let made = ok(&run(
        &mut d,
        "cad.polygon.create",
        CallOp::Execute,
        json!({ "layerId": parcels, "pts": [
            { "x": 487100.0, "y": 4420100.0 }, { "x": 487110.0, "y": 4420100.0 },
            { "x": 487110.0, "y": 4420110.0 }, { "x": 487100.0, "y": 4420110.0 } ] }),
    ))
    .clone();
    let new_uid = made["uid"].as_str().expect("uid").to_owned();
    references(&mut d, &new_uid, OWNER_1);
    let written = ok(&write_layer(&mut d, &parcels)).clone();
    assert_eq!(written["added"], json!(1), "{written}");
    let rows = junction(&url);
    assert_eq!(rows.len(), 5, "{rows:?}");
    let new_fid = rows.last().map(|r| r.0).expect("a row");
    assert!(
        new_fid > 3 && rows.last() == Some(&(new_fid, 1)),
        "{rows:?}"
    );
    // Read back by a fresh drawing: the references from the junction, in their order.
    let (mut fresh, _, fresh_parcels) = owners_and_parcels(&url, "iliski");
    ok(&run(
        &mut fresh,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": fresh_parcels, "direction": "read" }),
    ));
    let value = |d: &Document, uid: &str| {
        let slot = d.slot_of(uid.parse().expect("uuid")).expect("held");
        d.get(slot)
            .and_then(|e| e.base().attrs.get("malikler").cloned())
            .unwrap_or_default()
    };
    assert_eq!(value(&fresh, PARCEL_1), OWNER_2);
    assert_eq!(value(&fresh, PARCEL_2), format!("{OWNER_3}; {OWNER_2}"));
    assert_eq!(value(&fresh, PARCEL_3), OWNER_3);
    assert_eq!(value(&fresh, &new_uid), OWNER_1);
    // A parcel naming an owner the database does not hold yet: refused alone, written with the owners in one
    // transaction, the row by the owner's new key.
    let owner = ok(&run(
        &mut d,
        "cad.records.create",
        CallOp::Execute,
        json!({ "layerId": owners, "rows": [{ "attrs": { "ad": "Yeni Malik" } }] }),
    ))
    .clone();
    let owner_uid = owner["uids"][0].as_str().expect("uid").to_owned();
    references(&mut d, PARCEL_3, &owner_uid);
    let refused = write_layer(&mut d, &parcels);
    assert_eq!(refused["error"]["code"], "relation_dangling", "{refused}");
    assert_eq!(junction(&url).len(), 5);
    let together = ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": parcels, "direction": "write", "with": [owners] }),
    ))
    .clone();
    assert_eq!(together["added"], json!(1), "{together}");
    let rows = junction(&url);
    let third: Vec<&(i64, i64)> = rows.iter().filter(|r| r.0 == 3).collect();
    assert!(third.len() == 1 && third[0].1 > 3, "{rows:?}");
    // Someone else's junction table: reported, not written over, the reference written as text with a warning.
    let (mut other, _, other_parcels) = owners_and_parcels(&url, "iliski2");
    let foreign = relate(&mut other, CallOp::Execute, "check");
    assert_eq!(state(&foreign), "foreign", "{foreign}");
    assert!(
        ok(&foreign)["why"]
            .as_str()
            .unwrap_or_default()
            .contains("iliski2.sahiplik"),
        "{foreign}"
    );
    let unsupported = relate(&mut other, CallOp::Execute, "map");
    assert_eq!(
        unsupported["error"]["code"], "relation_unsupported",
        "{unsupported}"
    );
    references(&mut other, PARCEL_3, OWNER_1);
    let text = write_layer(&mut other, &other_parcels);
    assert!(
        text["warnings"]
            .as_array()
            .is_some_and(|w| w.iter().any(|w| w["code"] == "relation_unmapped")),
        "{text}"
    );
    // Taken away and made again: the junction holds the parcels' references as they now are.
    let unmapped = relate(&mut d, CallOp::Execute, "unmap");
    assert_eq!(state(&unmapped), "text", "{unmapped}");
    assert_eq!(state(&relate(&mut d, CallOp::Execute, "check")), "text");
    let remapped = relate(&mut d, CallOp::Execute, "map");
    assert_eq!(ok(&remapped)["rows"], json!(5), "{remapped}");
}

/// İşlemler's result written to the database (docs/adr/0592 §2): Nesneden nokta on sorgu.gpkg's whole `bina` table to
/// a new table of a new schema, in one transaction; again refused (the table is there) and with `replaceTables` made
/// again; a plan writes nothing; then the new table read back as a source input (§1). The script checks the table,
/// its points, its lasting ids and its comment with psycopg2.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn processing_results_go_to_postgis() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    kentos_processing::command::register();
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/sources/v1/sorgu.gpkg")
        .display()
        .to_string();
    let points = |sink: Value| {
        json!({ "tool": "convert.toPoints", "parameters": {
            "input": { "scope": "source", "path": file, "table": "bina" } }, "sink": sink })
    };
    let mut d = doc();
    let made = run(
        &mut d,
        "cad.processing.run",
        CallOp::Execute,
        points(json!({ "path": url, "schema": "sonuc" })),
    );
    let out = ok(&made);
    assert_eq!(out["sink"]["path"], json!(url));
    assert_eq!(
        out["sink"]["tables"][0]["table"], "sonuc.Merkezler",
        "{out}"
    );
    assert_eq!(out["sink"]["tables"][0]["rows"], 12);
    assert_eq!(out["sinkReport"]["ids"], "kentos_uids");
    assert_eq!(out["sinkReport"]["tables"][0]["replaced"], Value::Null);
    assert!(!d.can_undo(), "the drawing does not change");
    // There now: refused, the table as it was.
    let again = run(
        &mut d,
        "cad.processing.run",
        CallOp::Execute,
        points(json!({ "path": url, "schema": "sonuc" })),
    );
    assert_eq!(again["error"]["code"], "table_exists", "{again}");
    let planned = run(
        &mut d,
        "cad.processing.run",
        CallOp::Plan,
        points(json!({ "path": url, "schema": "sonuc" })),
    );
    assert_eq!(
        planned["error"]["code"], "table_exists",
        "a plan asks the database too: {planned}"
    );
    let planned = run(
        &mut d,
        "cad.processing.run",
        CallOp::Plan,
        points(json!({ "path": url, "schema": "sonuc", "mode": "replaceTables" })),
    );
    assert_eq!(ok(&planned)["sinkReport"]["tables"][0]["replaced"], true);
    let replaced = run(
        &mut d,
        "cad.processing.run",
        CallOp::Execute,
        points(json!({ "path": url, "schema": "sonuc", "mode": "replaceTables" })),
    );
    assert_eq!(
        ok(&replaced)["sinkReport"]["tables"][0]["replaced"],
        true,
        "{replaced}"
    );
    // The table read back as a source input, its result to a file.
    let dir = std::env::temp_dir().join(format!("kentos-pg-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let back = run(
        &mut d,
        "cad.processing.run",
        CallOp::Execute,
        json!({ "tool": "convert.toPoints", "parameters": {
            "input": { "scope": "source", "path": url, "table": "sonuc.Merkezler", "filter": "kat >= 3" } },
            "sink": { "path": dir.join("geri.gpkg").display().to_string() } }),
    );
    let input = &ok(&back)["inputs"][0];
    assert_eq!(input["rows"], 6, "{back}");
    assert!(
        input["revision"].is_null(),
        "a database's table has no file to hash"
    );
    assert!(
        input["table"]
            .as_str()
            .is_some_and(|t| t.starts_with("PostGIS: ") && t.ends_with(" › sonuc.Merkezler"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

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

/// A question's time from PostGIS (docs/adr/0595): the same records as from the GeoPackage, the dates as texts as written
/// or as date and timestamptz columns (but the one text a date column cannot hold), the condition sent as a regular
/// expression on the column's text.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn times_answer_alike_from_postgis() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/sources/v1/zaman.gpkg")
        .display()
        .to_string();
    let versions =
        json!({ "start": "bas", "end": "bit", "recordedStart": "kayit", "recordedEnd": "silindi" });
    let events = json!({ "start": "zaman" });
    let questions = [
        ("surum", json!({ "at": "2021-06-01" }), versions.clone()),
        ("surum", json!({ "at": "2022-06-01" }), versions.clone()),
        (
            "surum",
            json!({ "from": "2022-01-01", "to": "2023-01-01" }),
            versions.clone(),
        ),
        ("surum", json!({ "at": "2024-03-01" }), versions.clone()),
        (
            "surum",
            json!({ "at": "2022-06-01", "knownAt": "2022-06-02" }),
            versions.clone(),
        ),
        (
            "surum",
            json!({ "at": "2022-01-01", "knownAt": "2025-03-01" }),
            versions.clone(),
        ),
        (
            "surum",
            json!({ "knownAt": "2022-06-03T10:00:00Z" }),
            versions.clone(),
        ),
        (
            "olay",
            json!({ "at": "2024-06-01T10:00:00Z" }),
            events.clone(),
        ),
        (
            "olay",
            json!({ "from": "2024-06-01", "to": "2024-06-02" }),
            events.clone(),
        ),
    ];
    let names = |v: &Value| -> Vec<String> {
        ok(v)["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| r["attrs"]["ad"].as_str().unwrap_or("").to_owned())
            .collect()
    };
    let mut d = doc();
    for (table, at, fields) in &questions {
        let mut time = at.clone();
        time["fields"] = fields.clone();
        let asked = |path: &str, table: &str, filter: Option<&str>| {
            let mut q =
                json!({ "path": path, "table": table, "time": time.clone(), "limit": 1000 });
            if let Some(f) = filter {
                q["filter"] = json!(f);
            }
            q
        };
        let a = run(
            &mut d,
            "cad.sources.query",
            CallOp::Execute,
            asked(&file, table, None),
        );
        let b = run(
            &mut d,
            "cad.sources.query",
            CallOp::Execute,
            asked(&url, &format!("zaman.{table}"), None),
        );
        assert_eq!(names(&a), names(&b), "{table} {at}");
        let pushed = ok(&b)["plan"]["pushed"].as_str().unwrap_or("").to_owned();
        assert!(
            pushed.contains("!~") && pushed.contains("left(CAST("),
            "{pushed}"
        );
        assert_eq!(ok(&b)["plan"]["local"], json!(["time"]));
        // The typed columns: the same records, but P4 whose start no date column holds.
        if *table == "surum" {
            let a = run(
                &mut d,
                "cad.sources.query",
                CallOp::Execute,
                asked(&file, table, Some("ad != 'P4'")),
            );
            let c = run(
                &mut d,
                "cad.sources.query",
                CallOp::Execute,
                asked(&url, "zaman.surum_tipli", Some("ad != 'P4'")),
            );
            assert_eq!(names(&a), names(&c), "{table} {at}: tipli");
        }
        eprintln!("{table} {at}: {:?}", names(&a));
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

/// docs/adr/0648 §3: layers of one database are reconciled together, field by field, in one transaction: here the
/// plane moved and the table renamed, the line renamed in the table; nothing is a conflict and both tables take both.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn layers_of_one_database_are_reconciled_together() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    // Tables of its own: the other tests write boyut's at the same time.
    psql(
        &url,
        "CREATE SCHEMA uzlas; \
         CREATE TABLE uzlas.alan (fid bigint PRIMARY KEY, ad text, geom geometry(Polygon, 5256)); \
         CREATE TABLE uzlas.yol (fid bigint PRIMARY KEY, ad text, geom geometry(LineString, 5256)); \
         INSERT INTO uzlas.alan VALUES (1, 'ilk', ST_GeomFromText('POLYGON((487000 4420000,487010 4420000,487010 4420010,487000 4420010,487000 4420000))', 5256)); \
         INSERT INTO uzlas.yol VALUES (1, 'ilk', ST_GeomFromText('LINESTRING(487000 4420020,487010 4420020)', 5256));",
    );
    let mut d = doc();
    let (duz, duz_uid) = bind(&mut d, &url, "uzlas.alan");
    let (cizgi, _) = bind(&mut d, &url, "uzlas.yol");
    let (_, wkt_before) = row_in(&url, "uzlas.alan", "ilk");
    // Someone else renames both rows in the database.
    let mut other = doc();
    let (o_duz, o_duz_uid) = bind(&mut other, &url, "uzlas.alan");
    let (o_cizgi, o_cizgi_uid) = bind(&mut other, &url, "uzlas.yol");
    rename(&mut other, &o_duz_uid, "uzlas");
    rename(&mut other, &o_cizgi_uid, "uzlas");
    ok(&run(
        &mut other,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": o_duz, "direction": "write", "with": [o_cizgi] }),
    ));
    // Here the plane moves.
    ok(&run(
        &mut d,
        "cad.entities.transform",
        CallOp::Execute,
        json!({ "uids": [duz_uid], "transform": { "kind": "move", "dx": 0.5, "dy": 0.25 } }),
    ));
    let planned = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Plan,
        json!({ "layer": duz, "with": [cizgi] }),
    ))
    .clone();
    assert_eq!(planned["unresolved"], json!(0), "{planned}");
    assert!(planned.get("conflicts").is_none(), "{planned}");
    let done = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Execute,
        json!({ "layer": duz, "with": [cizgi] }),
    ))
    .clone();
    assert_eq!(done["merged"].as_array().map(Vec::len), Some(1), "{done}");
    assert_eq!(ad_in(&url, "uzlas.alan"), "uzlas");
    assert_eq!(ad_in(&url, "uzlas.yol"), "uzlas");
    let (_, wkt_after) = row_in(&url, "uzlas.alan", "uzlas");
    assert_ne!(wkt_after, wkt_before, "the plane's move is in the table");
    // Both layers are at their tables now: a second reconcile finds nothing.
    let again = ok(&run(
        &mut d,
        "cad.sources.reconcile",
        CallOp::Plan,
        json!({ "layer": duz, "with": [cizgi] }),
    ))
    .clone();
    assert!(again.get("conflicts").is_none(), "{again}");
    assert!(again.get("merged").is_none(), "{again}");
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
    // A batch reading that table, a row for each filter, to a schema of its own (docs/adr/0593 §1): a table for
    // each row's layer in one transaction; the record keeps the schema and no file, the journal no file either.
    let rows = |filter: &str, name: &str| {
        json!({ "parameters": {
            "input": { "scope": "source", "path": url, "table": "sonuc.Merkezler", "filter": filter },
            "layer": { "newName": name } } })
    };
    let batch = run(
        &mut d,
        "cad.processing.batch",
        CallOp::Execute,
        json!({ "tool": "convert.toPoints", "rows": [rows("kat >= 3", "Yüksek"), rows("kat < 3", "Alçak")],
                "sink": { "path": url, "schema": "toplu" } }),
    );
    let out = ok(&batch);
    let tables: Vec<(&str, u64)> = out["sink"]["tables"]
        .as_array()
        .expect("tables")
        .iter()
        .map(|t| {
            (
                t["table"].as_str().unwrap_or(""),
                t["rows"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    assert_eq!(tables, [("toplu.Yüksek", 6), ("toplu.Alçak", 6)], "{out}");
    let record: kentos_commands::ProcessingRecord =
        serde_json::from_value(out["record"].clone()).expect("a record");
    let sink = record.sink.as_ref().expect("where it went");
    assert_eq!(
        (sink.schema.as_deref(), sink.bytes, sink.sha256.as_deref()),
        (Some("toplu"), None, None)
    );
    assert_eq!(sink.tables[1].row.as_deref(), Some("rows[1]"));
    assert!(d.journal().last().is_some_and(|r| r.effects.is_empty()));
    assert!(matches!(
        kentos_processing::record::rerun_kind(&record, &d),
        kentos_processing::record::Rerun::UnknownSource(_)
    ));
    assert!(!d.can_undo(), "the drawing does not change");
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/0596 §2: Paketle copies the database's tables into the package, each a GeoPackage of the rows its link reads
/// (a part's area only), and the package opens with the database out of reach; a table changed since its layer last
/// matched it is refused. The package goes to KENTOS_PG_PACKAGE for `postgis_cases.py`, which reads the copies with OGR
/// against the database.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_database_table_goes_into_the_package() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let mut d = doc();
    let part = ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "paket.olay",
                          "extent": [486995.0, 4419995.0, 487595.0, 4420015.0] } }),
    ))
    .clone();
    let olay = part["link"]["layer"].as_str().expect("layer").to_owned();
    let held = d.by_layer_with_uids(&olay).count();
    assert!(held > 0 && held < 2400, "{held}");
    let (parsel, _) = bind(&mut d, &url, "paket.parsel");
    let parcels = d.by_layer_with_uids(&parsel).count();
    let dir = std::env::var("KENTOS_PG_PACKAGE").map_or_else(
        |_| std::env::temp_dir().join(format!("kentos-pg-paket-{}", std::process::id())),
        std::path::PathBuf::from,
    );
    let _ = std::fs::remove_dir_all(&dir);
    let input = kentos_contracts::PackageProject {
        path: dir.display().to_string(),
        name: Some("saha".into()),
        databases: Some(kentos_contracts::PackageWay::Copy),
        ..Default::default()
    };
    let planned = kentos_headless::package::plan(&d, &input).expect("planned");
    let copies: Vec<(String, Option<String>, Option<u64>, String)> = planned
        .output
        .files
        .iter()
        .map(|f| (f.path.clone(), f.table.clone(), f.rows, f.from.clone()))
        .collect();
    assert_eq!(
        copies,
        vec![
            (
                "veri/paket-olay.gpkg".to_owned(),
                Some("paket.olay".to_owned()),
                Some(held as u64),
                url.clone()
            ),
            (
                "veri/paket-parsel.gpkg".to_owned(),
                Some("paket.parsel".to_owned()),
                Some(parcels as u64),
                url.clone()
            ),
        ]
    );
    assert!(planned.output.left.is_empty(), "{:?}", planned.output.left);
    kentos_headless::package::write(&planned).expect("packed");
    // The package opens with no database: its layers hold the same objects, bound to the copies, which match them.
    let mut s = kentos_headless::Session::open(&dir.join("saha.kcad")).expect("opens");
    let links: Vec<(String, String, Option<String>)> = s
        .document()
        .settings()
        .sources
        .iter()
        .map(|l| {
            (
                serde_json::to_value(l.kind)
                    .expect("kind")
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                l.table.clone(),
                l.relative.clone(),
            )
        })
        .collect();
    assert_eq!(
        links,
        vec![
            (
                "geopackage".to_owned(),
                "olay".to_owned(),
                Some("veri/paket-olay.gpkg".to_owned())
            ),
            (
                "geopackage".to_owned(),
                "parsel".to_owned(),
                Some("veri/paket-parsel.gpkg".to_owned())
            ),
        ]
    );
    assert_eq!(s.document().by_layer_with_uids(&olay).count(), held);
    assert_eq!(s.document().by_layer_with_uids(&parsel).count(), parcels);
    let described = s
        .call(json!({ "command": "cad.sources.describe", "input": {} }))
        .expect("answers");
    for state in described["output"]["links"].as_array().expect("links") {
        assert_eq!(
            (
                state["found"].clone(),
                state["changed"].clone(),
                state["edited"].clone()
            ),
            (json!(true), json!(false), json!(false)),
            "{state}"
        );
    }
    // A table changed since its layer last matched it is not packed.
    let mut other = doc();
    let (theirs, uid) = bind(&mut other, &url, "paket.parsel");
    ok(&run(
        &mut other,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uid], "attrs": { "not": "değişti" }, "operation": "attributes" }),
    ));
    ok(&write_layer(&mut other, &theirs));
    let again = kentos_contracts::PackageProject {
        path: dir.with_extension("ikinci").display().to_string(),
        ..input
    };
    let refused = kentos_headless::package::plan(&d, &again)
        .err()
        .expect("refused");
    assert_eq!(
        (refused.code.as_str(), refused.path.as_deref()),
        ("source_changed", Some("databases"))
    );
}

/// Two halves of `uid`'s bounding box in `layer`, as polygons.
fn halves(d: &Document, layer: &str, uid: &str) -> (Value, Value) {
    let (_, e) = d
        .by_layer_with_uids(layer)
        .find(|(u, _)| u.to_string() == uid)
        .expect("object");
    let v = serde_json::to_value(e).expect("json");
    let pts = v["pts"].as_array().expect("pts");
    let xs: Vec<f64> = pts.iter().map(|p| p["x"].as_f64().expect("x")).collect();
    let ys: Vec<f64> = pts.iter().map(|p| p["y"].as_f64().expect("y")).collect();
    let (x0, x1) = (
        xs.iter().copied().fold(f64::MAX, f64::min),
        xs.iter().copied().fold(f64::MIN, f64::max),
    );
    let (y0, y1) = (
        ys.iter().copied().fold(f64::MAX, f64::min),
        ys.iter().copied().fold(f64::MIN, f64::max),
    );
    let xm = (x0 + x1) / 2.0;
    let rect = |a: f64, b: f64| json!({ "kind": "polygon", "pts": [{ "x": a, "y": y0 }, { "x": b, "y": y0 }, { "x": b, "y": y1 }, { "x": a, "y": y1 }] });
    (rect(x0, xm), rect(xm, x1))
}

/// The schema soy's lineage rows, (child, parent, operation) in that order.
fn lineage_in(url: &str) -> Vec<(String, String, String)> {
    let out = std::process::Command::new("psql")
        .arg(url)
        .args(["-At", "-F", "|", "-c"])
        .arg("SELECT child, parent, operation FROM soy.kentos_lineage ORDER BY 1, 2, 3")
        .output()
        .expect("psql");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut p = l.split('|');
            Some((
                p.next()?.to_owned(),
                p.next()?.to_owned(),
                p.next()?.to_owned(),
            ))
        })
        .collect()
}

// docs/adr/0598 §3, §4: a parcel split in a bound PostGIS table writes its piece, the piece's lasting id and its parent
// in the one transaction; undo is the drawing's and the next write takes the piece and its parent out, as a write
// checked against the rows it changes: someone else's change of the kept parcel in between refuses it whole.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_split_parcel_writes_its_parent_in_the_database() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let mut d = doc();
    let (layer, first) = bind(&mut d, &url, "soy.parsel");
    let (a, b) = halves(&d, &layer, &first);
    let split = ok(&run(
        &mut d,
        "cad.entities.edit",
        CallOp::Execute,
        json!({ "operation": "areaSplit", "changes": [
            { "kind": "replace", "uid": first, "geometry": a, "keepData": true },
            { "kind": "add", "from": first, "geometry": b, "keepData": true } ] }),
    ))
    .clone();
    let piece = split["created"][0].as_str().expect("piece").to_owned();
    let write = json!({ "layer": layer, "direction": "write" });
    ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        write.clone(),
    ));
    let one = vec![(piece.clone(), first.clone(), "split".to_owned())];
    assert_eq!(lineage_in(&url), one);
    // Undone and written: the piece and its parent leave; done again and written: they come back.
    assert_eq!(d.undo().as_deref(), Some("Alan böl"));
    ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        write.clone(),
    ));
    assert!(lineage_in(&url).is_empty());
    assert_eq!(d.redo().as_deref(), Some("Alan böl"));
    ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        write.clone(),
    ));
    assert_eq!(lineage_in(&url), one);
    // Someone else renames the kept parcel; the undone split's write keeps their name and takes the piece out.
    let mut other = doc();
    let (other_layer, _) = bind(&mut other, &url, "soy.parsel");
    ok(&run(
        &mut other,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [first], "attrs": { "ada": "baskasi" }, "operation": "attributes" }),
    ));
    ok(&run(
        &mut other,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": other_layer, "direction": "write" }),
    ));
    // The undone split's write changes that row too: refused whole, the piece, its parent and their name stay.
    assert_eq!(d.undo().as_deref(), Some("Alan böl"));
    let refused = run(&mut d, "cad.sources.sync", CallOp::Execute, write);
    assert_eq!(refused["status"], "conflict", "{refused}");
    assert_eq!(refused["error"]["code"], "source_changed", "{refused}");
    assert_eq!(refused["error"]["uids"], json!([first]), "{refused}");
    assert_eq!(lineage_in(&url), one);
    let mut seen = doc();
    let (seen_layer, _) = bind(&mut seen, &url, "soy.parsel");
    let ada = seen
        .by_layer_with_uids(&seen_layer)
        .find(|(u, _)| u.to_string() == first)
        .and_then(|(_, e)| e.base().attrs.get("ada").cloned());
    assert_eq!(ada.as_deref(), Some("baskasi"), "their name stays");
    assert!(
        seen.by_layer_with_uids(&seen_layer)
            .any(|(u, _)| u.to_string() == piece)
    );
}

/// The throwaway database's connection as the window would hold it (docs/adr/0623): KENTOS_PG_URL's fields, no
/// password in it (PGPASSWORD), the device's connections in memory.
fn window_connection(url: &str) -> kentos_postgis::connections::DbConnection {
    kentos_postgis::connections::keep_in_memory();
    kentos_postgis::register();
    let mut parsed = kentos_postgis::connections::parse(url).expect("KENTOS_PG_URL reads");
    parsed.connection.name = "Geçici".into();
    parsed.connection
}

#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_connection_is_tried_and_its_tables_listed_from_the_catalogue() {
    use kentos_postgis::probe::{self, Problem};
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    let c = window_connection(&url);
    // Bağlantıyı dene: the server and its PostGIS.
    let facts = probe::probe(&c, None).expect("reached");
    assert!(facts.version.starts_with("PostgreSQL "), "{facts:?}");
    assert!(facts.postgis.is_some(), "{facts:?}");
    assert_eq!(facts.database, c.database);
    assert!(facts.line().contains("PostGIS"), "{}", facts.line());
    // The catalogue: the parcels with their geometry, system and key, without counting their rows.
    let catalog = probe::browse(&c, None).expect("listed");
    let parsel = catalog
        .relations
        .iter()
        .find(|r| r.schema == "kadastro" && r.name == "parsel")
        .expect("kadastro.parsel listed");
    assert!(parsel.geometry.as_deref().is_some_and(|g| g.contains("POLYGON")), "{parsel:?}");
    assert!(parsel.srid.is_some());
    assert_eq!(parsel.key.as_deref(), Some("fid"));
    assert!(parsel.why_not().is_none());
    assert!(parsel.readable && parsel.writable);
    assert_eq!(catalog.facts.tables, catalog.relations.len());
    // What the catalogue says can be bound, the provider binds; what it says cannot, the provider refuses.
    let bindable = catalog.relations.iter().filter(|r| r.why_not().is_none()).take(6);
    let not = catalog.relations.iter().filter(|r| r.why_not().is_some()).take(3);
    for r in bindable.chain(not) {
        let mut doc = doc();
        let answer = run(
            &mut doc,
            "cad.sources.edit",
            CallOp::Execute,
            json!({ "edit": { "kind": "link", "path": c.address(), "table": r.table(), "reproject": true } }),
        );
        assert_eq!(
            answer["status"] == "completed",
            r.why_not().is_none(),
            "{}: {answer}",
            r.table()
        );
    }
    // Failures said for people, with what to do.
    let wrong = probe::probe(&c, Some("yanlış-parola")).unwrap_err();
    assert_eq!(wrong.problem, Problem::Password, "{wrong:?}");
    let none = kentos_postgis::connections::DbConnection {
        database: "boyle_bir_veritabani_yok".into(),
        ..c.clone()
    };
    assert_eq!(probe::probe(&none, None).unwrap_err().problem, Problem::Database);
    let closed = kentos_postgis::connections::DbConnection {
        port: 1,
        ..c.clone()
    };
    assert_eq!(probe::probe(&closed, None).unwrap_err().problem, Problem::Refused);
    let nowhere = kentos_postgis::connections::DbConnection {
        host: "boyle-bir-sunucu-yok.invalid".into(),
        ..c.clone()
    };
    assert_eq!(probe::probe(&nowhere, None).unwrap_err().problem, Problem::Host);
    // The throwaway server offers no TLS: Zorunlu says so.
    let tls = kentos_postgis::connections::DbConnection {
        ssl: kentos_postgis::connections::SslMode::Require,
        ..c.clone()
    };
    if let Err(d) = probe::probe(&tls, None) {
        assert_eq!(d.problem, Problem::Tls, "{d:?}");
    }
}

#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_password_typed_for_the_session_reaches_the_bound_layer() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    let Ok(password) = std::env::var("PGPASSWORD") else {
        return;
    };
    let mut c = window_connection(&url);
    // Another user of the database, its password nowhere but the session (PGPASSWORD is the owner's).
    c.user = "kentos_oturum".into();
    let owner = window_connection(&url);
    let admin = kentos_postgis::probe::browse(&owner, Some(&password));
    assert!(admin.is_ok(), "{admin:?}");
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
    runtime.block_on(async {
        use sqlx::{Connection, Executor};
        use std::str::FromStr;
        let options = sqlx::postgres::PgConnectOptions::from_str(&url).expect("url").password(&password);
        let mut conn = sqlx::postgres::PgConnection::connect_with(&options).await.expect("owner");
        for sql in [
            "DROP OWNED BY kentos_oturum",
            "DROP ROLE IF EXISTS kentos_oturum",
        ] {
            let _ = conn.execute(sql).await;
        }
        for sql in [
            "CREATE ROLE kentos_oturum LOGIN PASSWORD 'oturum-sifresi'",
            "GRANT USAGE ON SCHEMA kadastro TO kentos_oturum",
            "GRANT SELECT ON kadastro.parsel TO kentos_oturum",
        ] {
            conn.execute(sql).await.expect(sql);
        }
    });
    assert_eq!(
        kentos_postgis::probe::probe(&c, None).unwrap_err().problem,
        kentos_postgis::probe::Problem::Password,
        "PGPASSWORD is the owner's"
    );
    kentos_postgis::connections::remember(&c, "oturum-sifresi".into());
    let facts = kentos_postgis::probe::probe(&c, None).expect("the session's password");
    assert_eq!(facts.user, "kentos_oturum");
    // The bound layer's command reaches the database with it too: the drawing keeps no password.
    let mut doc = doc();
    let bound = run(
        &mut doc,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": c.address(), "table": "kadastro.parsel", "reproject": true } }),
    );
    assert_eq!(bound["status"], "completed", "{bound}");
    assert!(!serde_json::to_string(&doc.to_snapshot_v2()).unwrap().contains("oturum-sifresi"));
    kentos_postgis::connections::forget(&c);
}

/// The kinds of docs/adr/0629 §4 from their PostgreSQL types: `turler.olcum` (postgis_cases.py) has a `bigint` past
/// 2⁵³, a `timestamptz`, a `uuid`, a `jsonb`, a `bytea` and a `numeric(40, 10)`. Bound, each column is its field kind
/// and every value its canonical text; right after binding nothing differs; a changed time stamp, written back, is the
/// same moment in the database.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn wide_kinds_come_from_postgis() {
    use kentos_contracts::LayerFieldKind;
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        return;
    };
    kentos_postgis::register();
    let mut d = doc();
    let linked = run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "turler.olcum" } }),
    );
    assert_eq!(ok(&linked)["objects"], 2, "{linked}");
    assert_eq!(linked["warnings"], json!([]), "{linked}");
    let layer = linked["output"]["link"]["layer"]
        .as_str()
        .expect("layer")
        .to_owned();
    let kind = |d: &Document, name: &str| {
        d.layers()
            .get(&layer)
            .and_then(|n| n.fields.iter().find(|f| f.name == name).map(|f| f.kind))
    };
    assert_eq!(kind(&d, "kimlik"), Some(LayerFieldKind::BigInt));
    assert_eq!(kind(&d, "an"), Some(LayerFieldKind::Instant));
    assert_eq!(kind(&d, "cihaz"), Some(LayerFieldKind::Uuid));
    assert_eq!(kind(&d, "ek"), Some(LayerFieldKind::Json));
    assert_eq!(kind(&d, "imza"), Some(LayerFieldKind::Binary));
    assert_eq!(kind(&d, "tutar"), Some(LayerFieldKind::Decimal));
    let first = d
        .by_layer_with_uids(&layer)
        .find(|(_, e)| e.base().attrs.get("kimlik").map(String::as_str) == Some("9007199254740993"))
        .map(|(u, e)| (u.to_string(), e.base().attrs.clone()))
        .expect("the first row");
    let (uid, attrs) = first;
    // PostgreSQL's text of a timestamptz is in the session's zone; the moment is the one written.
    let an = attrs.get("an").expect("a time stamp");
    assert_eq!(
        kentos_contracts::instant_utc(an).as_deref(),
        Some("2024-03-01T07:20:30.5Z"),
        "{attrs:?}"
    );
    assert_eq!(attrs["cihaz"], "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001");
    assert_eq!(attrs["ek"], r#"{"kot":[1.25,12345678901234567890],"not":"ç"}"#);
    assert_eq!(attrs["imza"], "QUJD");
    assert_eq!(attrs["tutar"], "12345678901234567890.1234567890");
    let fresh = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Plan,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(ok(&fresh)["modified"], 0, "{fresh}");
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [uid], "attrs": { "an": "2024-03-01T12:00:00+03:00" }, "operation": "attributes" }),
    ));
    let written = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "write" }),
    );
    assert_eq!(ok(&written)["modified"], 1, "{written}");
    let mut again = doc();
    let back = run(
        &mut again,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "turler.olcum" } }),
    );
    let layer = back["output"]["link"]["layer"].as_str().expect("layer");
    let an = again
        .by_layer_with_uids(layer)
        .find(|(_, e)| e.base().attrs.get("kimlik").map(String::as_str) == Some("9007199254740993"))
        .and_then(|(_, e)| e.base().attrs.get("an").cloned())
        .expect("the time stamp");
    assert_eq!(
        kentos_contracts::instant_utc(&an).as_deref(),
        Some("2024-03-01T09:00:00Z")
    );
}

fn psql(url: &str, sql: &str) -> String {
    let out = std::process::Command::new("psql")
        .arg(url)
        .args(["-At", "-v", "ON_ERROR_STOP=1", "-c", sql])
        .output()
        .expect("psql");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// docs/adr/0634 §3: the drawing's derived dataset as new tables of a schema (each table's record its comment, its
/// rows' lasting ids in the schema's kentos_uids), checked against the drawing; a row a GIS edited in the database
/// taken back to its source; the links broken, the records notes. `postgis_cases.py` reads the schema with psycopg2.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_drawing_goes_to_postgis_as_a_derived_dataset() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    let drawing = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/gis-derivation/v1/drawing.json");
    let mut s = kentos_headless::Session::from_bytes(&std::fs::read(drawing).expect("the drawing"))
        .expect("a drawing");
    let mut call = |command: &str, input: Value| {
        s.call(json!({ "command": command, "input": input }))
            .expect("answers")
    };
    let export = json!({ "path": url, "format": "postgis", "schema": "turev" });
    let made = call("file.export.gis", export.clone());
    assert_eq!(made["status"], "completed", "{made}");
    let names: Vec<&str> = made["output"]["tables"]
        .as_array()
        .expect("tables")
        .iter()
        .map(|t| t["table"].as_str().expect("name"))
        .collect();
    assert!(names.contains(&"turev.Parsel"), "{names:?}");
    let again = call("file.export.gis", export);
    assert_eq!(again["error"]["code"], "table_exists", "{again}");
    let checked = call("cad.gis.derivation", json!({ "path": url }));
    assert_eq!(checked["output"]["current"], 17, "{checked}");
    assert_eq!(checked["output"]["file"], json!(url));
    // A GIS renames a road in the database; it is taken back to its source.
    let uid = psql(&url, "SELECT kentos_uid FROM turev.\"Yol\" WHERE ad = 'D-200'");
    psql(&url, "UPDATE turev.\"Yol\" SET ad = 'E-200' WHERE ad = 'D-200'");
    let edited = call("cad.gis.derivation", json!({ "path": url }));
    assert_eq!(edited["output"]["edited"], 1, "{edited}");
    let taken = call(
        "cad.gis.takeback",
        json!({ "path": url, "rows": [{ "table": "turev.Yol", "uid": uid }] }),
    );
    assert_eq!(taken["status"], "completed", "{taken}");
    assert_eq!(taken["output"]["changed"], 1, "{taken}");
    let road = s
        .document()
        .entities()
        .find(|e| e.base().attrs.get("ad").map(String::as_str) == Some("E-200"));
    assert!(road.is_some(), "the source renamed");
    // The links broken: every record a note, the ids gone; the dataset no longer derived.
    let mut call = |command: &str, input: Value| {
        s.call(json!({ "command": command, "input": input }))
            .expect("answers")
    };
    let detached = call("cad.gis.detach", json!({ "path": url, "tables": ["turev.Parsel"], "reason": "GIS'te sürecek" }));
    assert_eq!(detached["output"]["linked"], 5, "{detached}");
    assert_eq!(
        psql(&url, "SELECT count(*) FROM turev.kentos_uids WHERE table_name = 'turev.Parsel'"),
        "0"
    );
    let note = psql(&url, "SELECT obj_description('turev.\"Parsel\"'::regclass, 'pg_class')");
    let note: Value = serde_json::from_str(&note).expect("the note");
    assert_eq!(note["format"], "kentos.derivation-detached", "{note}");
    assert_eq!(note["reason"], "GIS'te sürecek");
    let rest = call("cad.gis.detach", json!({ "path": url }));
    assert_eq!(rest["output"]["tables"].as_array().map(Vec::len), Some(5), "{rest}");
    let gone = call("cad.gis.derivation", json!({ "path": url }));
    assert_eq!(gone["error"]["code"], "not_a_derivation", "{gone}");
}

/// docs/adr/0635 §1: a column renamed in the database (`ALTER TABLE … RENAME COLUMN`) keeps its place and type; the
/// bound layer's field and its filter follow on the next read.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_column_renamed_in_postgis_is_followed() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    psql(
        &url,
        "CREATE SCHEMA adlar; CREATE TABLE adlar.parsel (fid bigint PRIMARY KEY, ada text, parsel_no bigint, \
         geom geometry(Polygon, 5256)); INSERT INTO adlar.parsel VALUES \
         (1, '101', 7, ST_GeomFromText('POLYGON((487000 4420000,487040 4420000,487040 4420030,487000 4420000))', 5256)), \
         (2, '102', 8, ST_GeomFromText('POLYGON((487050 4420000,487090 4420000,487090 4420030,487050 4420000))', 5256))",
    );
    let mut d = doc();
    let (layer, _) = bind(&mut d, &url, "adlar.parsel");
    ok(&run(
        &mut d,
        "cad.layers.filter",
        CallOp::Execute,
        json!({ "layer": layer, "filter": { "expression": "ada = '101'" } }),
    ));
    psql(&url, "ALTER TABLE adlar.parsel RENAME COLUMN ada TO ada_no");
    let read = run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": layer, "direction": "read" }),
    );
    let out = ok(&read);
    assert_eq!(out["renamed"], json!([{ "from": "ada", "to": "ada_no" }]), "{read}");
    let node = d.layers().get(&layer).expect("the layer");
    assert!(node.fields.iter().any(|f| f.name == "ada_no"));
    assert!(node.fields.iter().all(|f| f.name != "ada"));
    assert_eq!(
        node.filter.as_ref().and_then(|f| f.expression.clone()).as_deref(),
        Some("ada_no = '101'")
    );
    assert!(
        d.by_layer(&layer)
            .all(|e| e.base().attrs.contains_key("ada_no") && !e.base().attrs.contains_key("ada"))
    );
}

#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_postgis_layer_reads_what_a_file_layer_reads() {
    // The same table bound from its GeoPackage and from PostGIS gives the same objects (docs/adr/0635 §11), so a look
    // classes them and draws its legend alike (crates/native/style/tests/all/bound_legend.rs shows that for a bound
    // layer and its copy). `kot` is left out: its one text value is NULL in the database (postgis_cases.py).
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/sources/v1/sorgu.gpkg")
        .display()
        .to_string();
    let read = |path: &str, table: &str| {
        let mut d = doc();
        let (layer, _) = bind(&mut d, path, table);
        let mut rows: Vec<String> = d
            .by_layer(&layer)
            .map(|e| {
                let a = &e.base().attrs;
                let pick = |k: &str| a.get(k).cloned().unwrap_or_default();
                let mut g = serde_json::to_value(e).expect("json");
                let shape = g
                    .as_object_mut()
                    .map(|o| {
                        o.retain(|k, _| !matches!(k.as_str(), "id" | "uid" | "layerId" | "attrs"));
                        Value::Object(o.clone())
                    })
                    .unwrap_or_default();
                format!("{}|{}|{}|{shape}", pick("ad"), pick("tur"), pick("no"))
            })
            .collect();
        rows.sort();
        rows
    };
    let from_file = read(&file, "nokta");
    let from_db = read(&url, "sorgu.nokta");
    assert!(from_file.len() > 50, "{}", from_file.len());
    assert_eq!(from_file, from_db);
}

#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn a_postgis_part_has_its_band() {
    // docs/adr/0636 §1, §3: the rows round a part bound from PostGIS hold room for its labels as a file's do, read as
    // the table holds them now; the layer's filter leaves out what it would not show.
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    // Nine 10 m parcels in three rows, named K0 … K8 from the south-west.
    let mut rows = Vec::new();
    for i in 0..9 {
        let (x, y) = (487_000 + (i % 3) * 10, 4_420_000 + (i / 3) * 10);
        rows.push(format!(
            "({id}, 'K{i}', ST_GeomFromText('POLYGON(({x} {y},{x2} {y},{x2} {y2},{x} {y2},{x} {y}))', 5256))",
            id = i + 1,
            x2 = x + 10,
            y2 = y + 10
        ));
    }
    psql(
        &url,
        &format!(
            "CREATE SCHEMA bant; CREATE TABLE bant.parsel (fid bigint PRIMARY KEY, ad text, \
             geom geometry(Polygon, 5256)); INSERT INTO bant.parsel VALUES {};",
            rows.join(", ")
        ),
    );
    let mut d = doc();
    let linked = ok(&run(
        &mut d,
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": url, "table": "bant.parsel",
                          "extent": [487_011.0, 4_420_011.0, 487_019.0, 4_420_019.0] } }),
    ))
    .clone();
    let layer = linked["link"]["layer"].as_str().expect("layer").to_owned();
    assert_eq!(d.by_layer(&layer).count(), 1, "the part holds the middle parcel");
    let report = |d: &mut Document| {
        ok(&run(
            d,
            "cad.labels.report",
            CallOp::Execute,
            json!({ "paperScale": 1000, "layers": [layer] }),
        ))["layers"][0]
            .clone()
    };
    let r = report(&mut d);
    assert_eq!(r["neighbours"], 8, "{r}");
    assert_eq!(r["band"].as_array().map(Vec::len), Some(4), "{r}");
    // The layer's filter shows no K1: that neighbour holds no room either.
    ok(&run(
        &mut d,
        "cad.layers.filter",
        CallOp::Execute,
        json!({ "layer": layer, "filter": { "expression": "ad <> 'K1'" } }),
    ));
    assert_eq!(report(&mut d)["neighbours"], 7);
}

/// The GIS acceptance project in a database (plan/gis.md CBS-44, docs/adr/0655 §5): the shared scenario
/// (fixtures/acceptance/v1/parcels.json) replayed up to its exports, the drawing written to PostGIS (`file.export.gis`,
/// schema `kabul`), and its parcels and buildings read back with `cad.sources.query`: every row by its key with the
/// values and the area the scenario's GeoPackage has.
#[test]
#[ignore = "needs the throwaway PostGIS of scripts/fixtures/postgis_cases.py (KENTOS_PG_URL)"]
fn the_parcels_scenario_goes_to_postgis_and_comes_back() {
    let Ok(url) = std::env::var("KENTOS_PG_URL") else {
        eprintln!("KENTOS_PG_URL yok: atlandı");
        return;
    };
    kentos_postgis::register();
    let scenario: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/acceptance/v1/parcels.json"
    ))
    .expect("the scenario");
    let choices = kentos_headless::new_drawing(&scenario["project"]).expect("the choices");
    let mut s = kentos_headless::Session::new(&choices).expect("a new project");
    let mut names: std::collections::HashMap<String, String> = Default::default();
    fn bind(v: &Value, names: &std::collections::HashMap<String, String>) -> Value {
        match v {
            Value::String(t) if t.starts_with('$') && !t.starts_with("$out:") => json!(names[&t[1..]]),
            Value::Object(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), bind(v, names))).collect()),
            Value::Array(a) => Value::Array(a.iter().map(|v| bind(v, names)).collect()),
            other => other.clone(),
        }
    }
    for step in scenario["steps"].as_array().expect("steps") {
        let Some(command) = step["command"].as_str() else {
            if step["undo"].is_string() {
                s.undo();
            }
            continue;
        };
        if command.starts_with("file.") {
            break;
        }
        let answer = s
            .call(json!({ "command": command, "input": bind(&step["input"], &names) }))
            .expect("answers");
        assert_eq!(answer["status"], "completed", "{}: {answer}", step["name"]);
        if let Some(key) = step["as"].as_str() {
            names.insert(key.into(), answer["output"]["uid"].as_str().expect("uid").into());
        }
        for (key, field) in step["bind"].as_object().into_iter().flatten() {
            let value = answer["output"][field.as_str().expect("a field")].as_str().expect("bound");
            names.insert(key.clone(), value.into());
        }
    }
    let made = s
        .call(json!({ "command": "file.export.gis",
                      "input": { "path": url, "format": "postgis", "schema": "kabul" } }))
        .expect("answers");
    assert_eq!(made["status"], "completed", "{made}");
    let mut d = doc();
    for (layer, spec) in scenario["files"]["gpkg"]["layers"].as_object().expect("layers") {
        let key = spec["key"].as_str().expect("a key");
        let want = spec["rows"].as_object().expect("rows");
        let answer = run(
            &mut d,
            "cad.sources.query",
            CallOp::Execute,
            json!({ "path": url, "table": format!("kabul.{layer}"), "limit": 100, "geometry": true }),
        );
        let rows = ok(&answer)["rows"].as_array().expect("rows").clone();
        assert_eq!(rows.len(), want.len(), "{layer}");
        for r in &rows {
            let k = r["attrs"][key].as_str().unwrap_or_default().to_owned();
            let w = &want[&k];
            for (field, value) in w["attrs"].as_object().expect("attrs") {
                let have = r["attrs"][field.as_str()].as_str().unwrap_or_default();
                let same = have == value.as_str().unwrap_or_default()
                    || have.parse::<f64>().ok().zip(value.as_str().and_then(|v| v.parse::<f64>().ok()))
                        .is_some_and(|(a, b)| a == b);
                assert!(same, "{layer}/{k}.{field}: {have:?}, beklenen {value}");
            }
            let ring: Vec<(f64, f64)> = w["ring"]
                .as_array()
                .expect("ring")
                .iter()
                .map(|p| (p[0].as_f64().expect("x"), p[1].as_f64().expect("y")))
                .collect();
            let wkt = r["wkt"].as_str().expect("wkt");
            let numbers: Vec<f64> = wkt
                .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                .filter_map(|t| t.parse().ok())
                .collect();
            for (x, y) in &ring {
                let found = numbers
                    .windows(2)
                    .any(|p| (p[0] - x).abs() < 1e-6 && (p[1] - y).abs() < 1e-6);
                assert!(found, "{layer}/{k}: {wkt} köşe {x} {y}");
            }
        }
        eprintln!("{layer}: {} satır PostGIS'ten geri okundu", rows.len());
    }
}

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

//! The provider's baseline on a large GeoPackage (plan/gis.md CBS-42, docs/perf/provider-2026-10-18.md): a synthetic
//! city of `KENTOS_PERF_ROWS` parcels (1 000 000 by default), ten vertices each, written once to
//! `.run/perf/provider-<rows>.gpkg`; then the first table page, the next page by key, a box query through the
//! spatial index, the count, and linking the table as a bound layer (the whole table read into the drawing, as the
//! working copy does today), each with its time and the process's resident memory after it.
//!
//! ```text
//! cargo test --release -p kentos-native-application --test all provider_timing -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;

use kentos_contracts::{CallOp, DocumentSnapshotV1};
use kentos_domain::Document;
use kentos_geopackage::geom::{Coord, Curve, Geometry};
use kentos_geopackage::{
    Column, ColumnKind, DataType, GeoPackage, GeometryColumn, NewRow, Query, TableDef, Value,
};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::json;

fn resident_mb() -> (f64, f64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<f64>().ok())
            .map_or(0.0, |kb| kb / 1024.0)
    };
    (field("VmRSS:"), field("VmHWM:"))
}

fn report(what: &str, ms: f64) {
    let (rss, peak) = resident_mb();
    println!("{what:<44} {ms:>10.1} ms   RSS {rss:>8.0} MB   tepe {peak:>8.0} MB");
}

/// The city: parcels of 20 × 20 m in rows of 1000, each a ring of ten vertices (corners and midpoints of three
/// sides), with Ada, Parsel and a name.
fn write_city(path: &std::path::Path, rows: usize) {
    let src = GeoPackage::open(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/geopackage/v1/qgis.gpkg"),
        false,
    )
    .expect("opens");
    let srs = src
        .srs(5256)
        .expect("srs")
        .expect("TUREF / TM36 in the sample");
    let mut gp = GeoPackage::create(path).expect("creates");
    gp.add_srs(&srs).expect("srs");
    let def = TableDef {
        name: "parsel".into(),
        data_type: DataType::Features,
        identifier: Some("parsel".into()),
        description: None,
        srs_id: Some(5256),
        geometry: Some(GeometryColumn {
            name: "geom".into(),
            type_name: "POLYGON".into(),
            z: 0,
            m: 0,
        }),
        columns: ["Ada", "Parsel"]
            .iter()
            .map(|n| Column {
                name: (*n).into(),
                decl: String::new(),
                kind: ColumnKind::Integer,
                not_null: false,
                default: None,
            })
            .chain(std::iter::once(Column {
                name: "Ad".into(),
                decl: String::new(),
                kind: ColumnKind::Text(None),
                not_null: false,
                default: None,
            }))
            .collect(),
    };
    let c = |x: f64, y: f64| Coord::xy(x, y);
    let all: Vec<NewRow> = (0..rows)
        .map(|i| {
            let (col, row) = ((i % 1000) as f64, (i / 1000) as f64);
            let (x, y) = (500_000.0 + col * 20.0, 4_400_000.0 + row * 20.0);
            let ring = vec![
                c(x, y),
                c(x + 10.0, y),
                c(x + 20.0, y),
                c(x + 20.0, y + 10.0),
                c(x + 20.0, y + 20.0),
                c(x + 10.0, y + 20.0),
                c(x, y + 20.0),
                c(x, y + 10.0),
                c(x + 0.5, y + 5.0),
                c(x, y + 1.0),
                c(x, y),
            ];
            NewRow {
                fid: None,
                geometry: Some(Geometry::Polygon(vec![Curve::Line(ring)])),
                values: vec![
                    Value::Integer((i / 1000) as i64 + 1),
                    Value::Integer((i % 1000) as i64 + 1),
                    Value::Text(format!("Parsel {i}")),
                ],
            }
        })
        .collect();
    gp.write_table(&def, &all, None).expect("written");
    gp.close().expect("closed");
}

fn empty_drawing() -> Document {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/processing/v1/parcels.kcad"),
    )
    .expect("reads");
    let mut snapshot: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    snapshot["entities"] = json!([]);
    snapshot["settings"]["srid"] = json!(5256);
    Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("a drawing"),
    )
    .expect("opens")
}

#[test]
#[ignore = "a measurement, run by hand"]
fn provider_timing() {
    let rows: usize = std::env::var("KENTOS_PERF_ROWS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000_000);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.run/perf");
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join(format!("provider-{rows}.gpkg"));
    if !path.exists() {
        let t = Instant::now();
        write_city(&path, rows);
        report(
            &format!("yazma ({rows} parsel)"),
            t.elapsed().as_secs_f64() * 1000.0,
        );
    }
    let file = std::fs::File::open(&path).expect("opens");
    let (bytes, sha) = kentos_contracts::encoding::sha256_reader_hex(file).expect("reads");
    println!("dosya {} bayt, SHA-256 {sha}", bytes);
    report("başlangıç", 0.0);

    let t = Instant::now();
    let gp = GeoPackage::open(&path, false).expect("opens");
    let layer = gp
        .layers()
        .expect("layers")
        .into_iter()
        .find(|l| l.name == "parsel")
        .expect("parsel");
    report("aç ve tablolar", t.elapsed().as_secs_f64() * 1000.0);

    let t = Instant::now();
    let n = gp.count(&layer).expect("count");
    report(&format!("say ({n})"), t.elapsed().as_secs_f64() * 1000.0);

    let t = Instant::now();
    let first = gp
        .rows(
            &layer,
            &Query {
                bbox: None,
                after: None,
                limit: Some(100),
            },
        )
        .expect("rows");
    report("ilk sayfa (100 satır)", t.elapsed().as_secs_f64() * 1000.0);

    let t = Instant::now();
    let _next = gp
        .rows(
            &layer,
            &Query {
                bbox: None,
                after: first.last().map(|r| r.fid),
                limit: Some(100),
            },
        )
        .expect("rows");
    report(
        "sonraki sayfa (anahtarla)",
        t.elapsed().as_secs_f64() * 1000.0,
    );

    // A view of 400 × 300 m (about 300 parcels) in the middle of the city.
    let (mx, my) = (
        500_000.0 + 10_000.0,
        4_400_000.0 + (rows as f64 / 1000.0) * 10.0,
    );
    let t = Instant::now();
    let seen = gp
        .rows(
            &layer,
            &Query {
                bbox: Some([mx, my, mx + 400.0, my + 300.0]),
                after: None,
                limit: None,
            },
        )
        .expect("rows");
    report(
        &format!("görünen kutu ({} satır)", seen.len()),
        t.elapsed().as_secs_f64() * 1000.0,
    );
    drop(gp);

    let mut doc = empty_drawing();
    let t = Instant::now();
    let linked = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.sources.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "link", "path": path.display().to_string(), "table": "parsel" } }),
    )
    .expect("runs");
    assert_eq!(linked["status"], "completed", "{linked}");
    report(
        &format!(
            "bağlı katman (bütün tablo, {} nesne)",
            linked["output"]["objects"]
        ),
        t.elapsed().as_secs_f64() * 1000.0,
    );
}

//! The field kinds' times (docs/adr/0629 Doğrulama): 100 000 points with a big integer, a time stamp, a UUID, JSON and
//! binary, each value checked to its canonical text, written as a GeoPackage and a GeoParquet and read back; beside it the
//! same values in text fields, what the kinds add.
//!
//! `cargo test --release -p kentos-native-application --test all kinds_timing -- --ignored --nocapture --test-threads=1`

use std::time::Instant;

use kentos_commands::{
    CommandResult, DocumentSnapshotV1, ExportParquet, ImportParquet, LayerField, LayerFieldKind,
    ParquetCompression, check_value,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, file_import_parquet, geopackage, parquet_write};
use serde_json::json;

const POINTS: u32 = 100_000;
const NAMES: [&str; 5] = ["kimlik", "an", "cihaz", "ek", "imza"];

fn values(i: u32) -> [String; 5] {
    [
        (9_007_199_254_740_993_i64 + i64::from(i)).to_string(),
        format!(
            "2024-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}+03:00",
            1 + i % 12,
            1 + i % 28,
            i % 24,
            i % 60,
            (i / 60) % 60,
            i % 1000
        ),
        format!("0192f5a0-7c3e-7d4a-9b1e-{:012x}", i),
        format!(
            r#"{{"no":{i},"kot":[1.25,{}],"not":"ç"}}"#,
            u64::from(i) * 1_000_003
        ),
        ["QUJD", "a2VudG9z", "AAECAwQ=", "S2VudE9TIENBRA=="][(i % 4) as usize].to_owned(),
    ]
}

fn drawing(kinds: bool) -> Document {
    let entities: Vec<serde_json::Value> = (0..POINTS)
        .map(|i| {
            let attrs: serde_json::Map<String, serde_json::Value> = NAMES
                .iter()
                .zip(values(i))
                .map(|(k, v)| ((*k).to_owned(), json!(v)))
                .collect();
            json!({"kind": "point", "id": i + 1, "layerId": "olcum", "attrs": attrs,
                "p": {"x": 487_000.0 + f64::from(i % 300), "y": 4_420_000.0 + f64::from(i / 300)}})
        })
        .collect();
    let kind = |k: &'static str| if kinds { k } else { "text" };
    let fields = json!([
        {"name": "kimlik", "kind": kind("bigint")}, {"name": "an", "kind": kind("instant")},
        {"name": "cihaz", "kind": kind("uuid")}, {"name": "ek", "kind": kind("json")},
        {"name": "imza", "kind": kind("binary")}
    ]);
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Türler","settings":{"srid":5256,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":1000},
        "origin":{"x":0,"y":0},"layers":[{"id":"olcum","name":"olcum","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[],
        "fields": fields}],
        "activeLayer":"olcum","entities":entities,"styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn empty() -> Document {
    Document::from_snapshot(
        DocumentSnapshotV1::from_json(
            &json!({"format":"kentos.document","version":1,"name":"Boş","settings":{"srid":5256,
                "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":1000},
                "origin":{"x":0,"y":0},"layers":[{"id":"cizim","name":"Çizim","type":"layer","visible":true,
                "locked":false,"expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},
                "children":[]}],"activeLayer":"cizim","entities":[],"styles":{"items":[],"categories":[]}})
            .to_string(),
        )
        .expect("reads"),
    )
    .expect("opens")
}

#[test]
#[ignore = "a timing, run on purpose"]
fn kinds_timing() {
    let dir = std::env::temp_dir().join(format!("kentos-kinds-timing-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    // Each value to its canonical text, by kind.
    let all: Vec<[String; 5]> = (0..POINTS).map(values).collect();
    for (j, (name, kind)) in NAMES
        .iter()
        .zip([
            LayerFieldKind::BigInt,
            LayerFieldKind::Instant,
            LayerFieldKind::Uuid,
            LayerFieldKind::Json,
            LayerFieldKind::Binary,
        ])
        .enumerate()
    {
        let f = LayerField::new(*name, kind);
        let t = Instant::now();
        let n = all
            .iter()
            .filter(|v| check_value(&f, &v[j]).is_ok())
            .count();
        println!("denetim ({}): {POINTS} değer, {:.1} ms", kind.name(), ms(t));
        assert_eq!(n, POINTS as usize);
    }
    for kinds in [false, true] {
        let word = if kinds { "türlerle" } else { "metin" };
        let doc = drawing(kinds);
        let path = dir.join(format!("olcum-{word}.gpkg"));
        let _ = std::fs::remove_file(&path);
        let t = Instant::now();
        geopackage::write(&doc, &["olcum".to_owned()], &path).expect("written");
        println!("GeoPackage yazma ({word}): {:.0} ms", ms(t));
        let t = Instant::now();
        let read = geopackage::read(&path, None).expect("read");
        println!(
            "GeoPackage okuma ({word}): {:.0} ms, {} satır",
            ms(t),
            read.tables[0].objects.len()
        );
        let kinds_read: Vec<&str> = read.tables[0]
            .fields
            .iter()
            .map(|f| f.kind.name())
            .collect();
        println!("  alanlar: {kinds_read:?}");
        let path = dir.join(format!("olcum-{word}.parquet"));
        let input = ExportParquet {
            path: path.display().to_string(),
            layers: Vec::new(),
            overwrite: true,
            compression: ParquetCompression::Snappy,
            covering: true,
        };
        let t = Instant::now();
        let made = parquet_write::export(&doc, &input).expect("made");
        println!("GeoParquet yazma ({word}): {:.0} ms", ms(t));
        std::fs::write(&path, &made.bytes).expect("written");
        let mut into = empty();
        let t = Instant::now();
        let out = file_import_parquet::execute(
            &mut ExecutionContext::new(&mut into),
            ImportParquet {
                path: path.display().to_string(),
                layer: None,
                srid: None,
                reproject: false,
                merge: None,
                rejects: None,
            },
        );
        println!("GeoParquet içe aktarma ({word}): {:.0} ms", ms(t));
        assert!(matches!(out, CommandResult::Completed { .. }), "{out:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

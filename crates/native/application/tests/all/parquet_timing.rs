//! GeoParquet's write, read and merge times (docs/adr/0628 Doğrulama): 100 000 parcels of 12 vertices with five typed
//! fields written as one GeoParquet (Snappy and without compression), planned and taken in, and taken again into the
//! same layer with a merge by key (every row meets its object, a tenth of them changed).
//!
//! `cargo test --release -p kentos-native-application --test all parquet_timing -- --ignored --nocapture --test-threads=1`

use std::time::Instant;

use kentos_commands::{
    CommandResult, DocumentSnapshotV1, ExportParquet, ImportMerge, ImportParquet, MergeDuplicates,
    MergeMatched, MergeUnmatched, ParquetCompression,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, file_import_parquet, parquet_write};
use serde_json::json;

const PARCELS: u32 = 100_000;

/// The parcels; `stale`: a tenth of them with an older owner (what a merge of the file updates).
fn drawing(stale: bool) -> Document {
    let mut entities = Vec::with_capacity(PARCELS as usize);
    for i in 0..PARCELS {
        let (x, y) = (
            487_000.0 + f64::from(i % 300) * 30.0,
            4_420_000.0 + f64::from(i / 300) * 30.0,
        );
        let pts: Vec<serde_json::Value> = (0..12)
            .map(|k| {
                let a = f64::from(k) * std::f64::consts::TAU / 12.0;
                json!({"x": x + 12.0 * a.cos(), "y": y + 12.0 * a.sin()})
            })
            .collect();
        let owner = if stale && i % 10 == 0 {
            format!("Eski malik {}", i % 1000)
        } else {
            format!("Malik {}", i % 1000)
        };
        entities.push(json!({"kind": "polygon", "id": i + 1, "layerId": "parsel", "pts": pts,
            "attrs": {"parsel_no": i.to_string(), "ada": (i / 40).to_string(), "alan": format!("{}.25", 400 + i % 97),
                      "malik": owner, "tescil": "2020-01-15"}}));
    }
    let fields = json!([
        {"name": "parsel_no", "kind": "integer"},
        {"name": "ada", "kind": "integer"},
        {"name": "alan", "kind": "decimal", "scale": 2},
        {"name": "malik", "kind": "text", "length": 40},
        {"name": "tescil", "kind": "date"}
    ]);
    let snapshot = json!({"format":"kentos.document","version":1,"name":"GeoParquet","settings":{"srid":5256,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":1000},
        "origin":{"x":0,"y":0},"layers":[{"id":"parsel","name":"parsel","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[],
        "fields": fields}],
        "activeLayer":"parsel","entities":entities,"styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

fn imported(answer: CommandResult<kentos_commands::GpkgImported>) -> kentos_commands::GpkgImported {
    match answer {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

#[test]
#[ignore = "a timing, run on purpose"]
fn parquet_timing() {
    let dir = std::env::temp_dir().join(format!("kentos-parquet-timing-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let t = Instant::now();
    let doc = drawing(false);
    println!("çizim: {PARCELS} parsel, {:.0} ms", ms(t));
    let mut written = None;
    for compression in [ParquetCompression::Snappy, ParquetCompression::None] {
        let path = dir.join(format!("parsel-{compression:?}.parquet").to_lowercase());
        let input = ExportParquet {
            time: None,
            path: path.display().to_string(),
            layers: Vec::new(),
            overwrite: true,
            compression,
            covering: true,
        };
        let t = Instant::now();
        let made = parquet_write::export(&doc, &input).expect("made");
        let wrote = ms(t);
        std::fs::write(&path, &made.bytes).expect("written");
        println!(
            "yazma ({compression:?}): {wrote:.0} ms, {:.1} MB, {} satır grubu",
            made.bytes.len() as f64 / 1e6,
            made.output.row_groups
        );
        if compression == ParquetCompression::Snappy {
            written = Some(path);
        }
    }
    let path = written.expect("the Snappy file");
    // Taken into an empty drawing of the same system: a new layer with the columns as its fields.
    let mut empty = Document::from_snapshot(
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
    .expect("opens");
    let input = ImportParquet {
        path: path.display().to_string(),
        layer: None,
        srid: None,
        reproject: false,
        merge: None,
        rejects: None,
    };
    let t = Instant::now();
    let plan = imported(file_import_parquet::plan(
        &ExecutionContext::new(&mut empty.clone()),
        &input,
    ));
    println!(
        "plan (yeni katman): {:.0} ms, {} nesne",
        ms(t),
        plan.objects
    );
    let t = Instant::now();
    let out = imported(file_import_parquet::execute(
        &mut ExecutionContext::new(&mut empty),
        input,
    ));
    println!(
        "içe aktarma (yeni katman): {:.0} ms, {} nesne",
        ms(t),
        out.objects
    );
    assert_eq!(out.objects, PARCELS);
    // The rows into a drawing of the same parcels, a tenth of them with an older owner, met by parsel_no.
    let mut doc = drawing(true);
    let merge = ImportParquet {
        path: path.display().to_string(),
        layer: Some("parsel".into()),
        srid: None,
        reproject: false,
        merge: Some(ImportMerge {
            key: Some("parsel_no".into()),
            target_key: None,
            matched: MergeMatched::Update,
            geometry: true,
            unmatched: MergeUnmatched::Add,
            duplicates: MergeDuplicates::Stop,
        }),
        rejects: Some(dir.join("reddedilenler.csv").display().to_string()),
    };
    let t = Instant::now();
    let plan = imported(file_import_parquet::plan(
        &ExecutionContext::new(&mut doc.clone()),
        &merge,
    ));
    let report = &plan.merged[0];
    println!(
        "birleştirme planı: {:.0} ms ({} eklenecek, {} güncellenecek, {} değişmeyecek, {} reddedilecek)",
        ms(t),
        report.added,
        report.updated,
        report.unchanged,
        report.rejected
    );
    assert_eq!(
        (report.added, report.updated, report.unchanged),
        (0, PARCELS / 10, PARCELS - PARCELS / 10)
    );
    let t = Instant::now();
    let out = imported(file_import_parquet::execute(
        &mut ExecutionContext::new(&mut doc),
        merge,
    ));
    println!("birleştirme: {:.0} ms", ms(t));
    assert_eq!(out.merged[0].added, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

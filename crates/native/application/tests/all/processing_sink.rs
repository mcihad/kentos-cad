//! `cad.processing.run` writing its result to a GeoPackage (docs/adr/0508 §4): G1's parcels × zoning, a buffer of
//! the roads and a dissolve by Ada go to new files, the drawing unchanged; a file that is there is replaced only with
//! `overwrite`. With `KENTOS_WRITE_GPKG=1` the files are kept in fixtures/geopackage/v1 (`kentos-run-*.gpkg`) for
//! `scripts/fixtures/processing_sink_cases.py`, which reads them with OGR and checks them against the source.

use std::path::PathBuf;

use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures")
}

fn drawing() -> Document {
    let text =
        std::fs::read_to_string(fixtures().join("processing/v1/geometry.kcad")).expect("reads");
    Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("a drawing"))
        .expect("opens")
}

fn run(doc: &mut Document, input: Value) -> Value {
    kentos_processing::command::register();
    registry::run(
        &mut ExecutionContext::new(doc),
        "cad.processing.run",
        CallOp::Execute,
        input,
    )
    .expect("runs")
}

#[test]
fn g1_results_go_to_geopackages_and_the_drawing_stays() {
    let dir = std::env::temp_dir().join(format!("kentos-sink-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let runs = [
        (
            "kesisim",
            json!({ "tool": "geometry.intersection", "parameters": {
                "input": { "scope": "layer", "layerId": "parsel" },
                "overlay": { "scope": "layer", "layerId": "imar" },
                "prefix": "", "apportion": "Değer" } }),
        ),
        (
            "tampon",
            json!({ "tool": "geometry.buffer", "parameters": {
                "input": { "scope": "layer", "layerId": "yol" }, "distance": 5 } }),
        ),
        (
            "birlestirilen",
            json!({ "tool": "geometry.dissolve", "parameters": {
                "input": { "scope": "layer", "layerId": "parsel" }, "group": "Ada", "sums": "Değer" } }),
        ),
    ];
    for (name, mut input) in runs {
        let mut doc = drawing();
        let before = doc.revision();
        let path = dir.join(format!("kentos-run-{name}.gpkg"));
        input["sink"] = json!({ "path": path.display().to_string() });
        let got = run(&mut doc, input.clone());
        assert_eq!(got["status"], "completed", "{name}: {got}");
        let sink = &got["output"]["sink"];
        assert!(path.exists(), "{name}: the file is written");
        let bytes = std::fs::read(&path).expect("reads");
        assert_eq!(sink["bytes"], json!(bytes.len()), "{name}");
        assert_eq!(
            sink["sha256"],
            json!(kentos_commands::encoding::sha256_hex(&bytes)),
            "{name}"
        );
        assert_eq!(
            doc.revision(),
            before,
            "{name}: the drawing does not change"
        );
        assert!(!doc.can_undo(), "{name}: no undo step");
        assert!(got["output"].get("step").is_none(), "{name}");
        // No passing file is left beside it.
        let stray: Vec<_> = std::fs::read_dir(&dir)
            .expect("lists")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".kentos-"))
            .collect();
        assert!(stray.is_empty(), "{name}: {stray:?}");
        // There now: refused without overwrite, the file as it was; replaced with it.
        let again = run(&mut doc, input.clone());
        assert_eq!(again["error"]["code"], "file_exists", "{name}: {again}");
        assert_eq!(std::fs::read(&path).expect("reads"), bytes);
        input["sink"]["overwrite"] = json!(true);
        let replaced = run(&mut doc, input);
        assert_eq!(replaced["status"], "completed", "{name}: {replaced}");
        if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
            std::fs::copy(
                &path,
                fixtures().join(format!("geopackage/v1/kentos-run-{name}.gpkg")),
            )
            .expect("keeps the file");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ── Into a GeoPackage that is there (docs/adr/0549) ─────────────────────────

fn buffer_to(path: &std::path::Path, mode: &str) -> Value {
    json!({ "tool": "geometry.buffer", "parameters": {
        "input": { "scope": "layer", "layerId": "yol" }, "distance": 5 },
        "sink": { "path": path.display().to_string(), "mode": mode } })
}

/// QGIS's file in a folder of its own.
fn qgis_copy(name: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("kentos-sink-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("var.gpkg");
    std::fs::copy(fixtures().join("geopackage/v1/qgis.gpkg"), &path).expect("copied");
    (dir, path)
}

/// Every table's rows (fid, geometry and values), by table.
fn contents(path: &std::path::Path) -> Vec<(String, Vec<String>)> {
    let gp = kentos_geopackage::GeoPackage::open(path, false).expect("opens");
    gp.layers()
        .expect("layers")
        .iter()
        .map(|l| {
            let rows = gp
                .rows(l, &kentos_geopackage::Query::default())
                .expect("rows")
                .iter()
                .map(|r| format!("{} {:?} {:?}", r.fid, r.geometry, r.values))
                .collect();
            (l.name.clone(), rows)
        })
        .collect()
}

fn no_stray(dir: &std::path::Path) {
    let stray: Vec<_> = std::fs::read_dir(dir)
        .expect("lists")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".kentos-"))
        .collect();
    assert!(stray.is_empty(), "{stray:?}");
}

fn names_of(list: &Value) -> Vec<String> {
    list.as_array()
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

#[test]
fn tables_are_added_to_a_file_that_is_there_and_no_table_of_it_is_touched() {
    let (dir, path) = qgis_copy("add");
    let before = contents(&path);
    let mut doc = drawing();
    let got = run(&mut doc, buffer_to(&path, "addTables"));
    assert_eq!(got["status"], "completed", "{got}");
    let report = &got["output"]["sinkReport"];
    assert_eq!(report["mode"], "addTables");
    let table = report["tables"][0]["table"].as_str().expect("a table").to_owned();
    assert_eq!(report["tables"][0]["layer"], got["output"]["sink"]["tables"][0]["layer"]);
    assert!(report["batches"].as_u64().unwrap_or(0) >= 1);
    assert_eq!(report["ids"], "kentos_uids");
    assert!(report["tables"][0]["columns"].is_array(), "{report}");
    // Every table the file had is named as untouched, and is as it was, row for row.
    assert_eq!(
        names_of(&report["untouched"]),
        before.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>()
    );
    let after = contents(&path);
    assert_eq!(after.len(), before.len() + 1);
    for (name, rows) in &before {
        let kept = after.iter().find(|(n, _)| n == name).expect("kept");
        assert_eq!(&kept.1, rows, "{name} is as it was");
    }
    let added = after.iter().find(|(n, _)| *n == table).expect("the new table");
    assert_eq!(added.1.len() as u64, report["tables"][0]["rows"].as_u64().unwrap_or(0));
    // The answer's file is the file: its bytes and digest.
    let bytes = std::fs::read(&path).expect("reads");
    assert_eq!(got["output"]["sink"]["bytes"], json!(bytes.len()));
    assert_eq!(
        got["output"]["sink"]["sha256"],
        json!(kentos_commands::encoding::sha256_hex(&bytes))
    );
    // Its lineage: one document, the table's own.
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("opens");
    assert_eq!(gp.metadata(&table).expect("metadata").len(), 1);
    assert_eq!(gp.uids(&table).expect("ids").len(), added.1.len());
    drop(gp);
    no_stray(&dir);
    // The same table again: refused, the file byte for byte as it was.
    let again = run(&mut doc, buffer_to(&path, "addTables"));
    assert_eq!(again["error"]["code"], "table_exists", "{again}");
    assert_eq!(std::fs::read(&path).expect("reads"), bytes);
    // Replace takes only that table out and writes it again; the others stay.
    let replaced = run(&mut doc, buffer_to(&path, "replaceTables"));
    assert_eq!(replaced["status"], "completed", "{replaced}");
    assert_eq!(replaced["output"]["sinkReport"]["tables"][0]["replaced"], true);
    let now = contents(&path);
    assert_eq!(now.len(), after.len());
    for (name, rows) in &before {
        assert_eq!(&now.iter().find(|(n, _)| n == name).expect("kept").1, rows, "{name}");
    }
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("opens");
    assert_eq!(gp.metadata(&table).expect("metadata").len(), 1, "no orphan document");
    assert_eq!(gp.uids(&table).expect("ids").len(), added.1.len(), "no orphan id");
    assert_eq!(
        names_of(&replaced["output"]["sinkReport"]["untouched"]).len(),
        before.len(),
        "the replaced table is not among the untouched"
    );
    drop(gp);
    no_stray(&dir);
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&path, fixtures().join("geopackage/v1/kentos-run-ekle.gpkg")).expect("kept");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_cannot_take_tables_is_refused_and_left_alone() {
    let (dir, path) = qgis_copy("refuse");
    let mut doc = drawing();
    let missing = dir.join("yok.gpkg");
    let got = run(&mut doc, buffer_to(&missing, "addTables"));
    assert_eq!(got["error"]["code"], "file_not_found", "{got}");
    assert!(!missing.exists());
    let mut with = buffer_to(&path, "addTables");
    with["sink"]["overwrite"] = json!(true);
    let before = std::fs::read(&path).expect("reads");
    assert_eq!(run(&mut doc, with)["error"]["code"], "invalid_input");
    // A file that is no GeoPackage.
    let text = dir.join("metin.gpkg");
    let said = "bu bir GeoPackage değil, düz metin; SQLite başlığı yok";
    std::fs::write(&text, said).expect("written");
    let got = run(&mut doc, buffer_to(&text, "replaceTables"));
    assert_eq!(got["error"]["code"], "import_unreadable", "{got}");
    assert_eq!(std::fs::read_to_string(&text).expect("reads"), said);
    assert_eq!(std::fs::read(&path).expect("reads"), before);
    no_stray(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A flow that asks to stop at its `n`th question and keeps how far the rows have come.
struct StopAt {
    n: usize,
    asked: std::cell::Cell<usize>,
    last: std::cell::Cell<(u64, u64)>,
}

impl kentos_native_application::processing_host::BatchFlow for StopAt {
    fn stopped(&self) -> bool {
        self.asked.set(self.asked.get() + 1);
        self.asked.get() == self.n
    }

    fn progress(&self, done: u64, total: u64) {
        self.last.set((done, total));
    }
}

#[test]
fn a_run_stopped_at_any_point_leaves_the_file_and_the_drawing_as_they_were() {
    use kentos_native_application::processing_host::{RunEnd, tools};
    kentos_processing::command::register();
    let host = tools().expect("the tools are registered");
    let mut cancelled = 0;
    let mut finished = false;
    for mode in ["addTables", "replaceTables", "new"] {
        for n in 1..=12 {
            let (dir, path) = qgis_copy("stop");
            let before = std::fs::read(&path).expect("reads");
            let mut doc = drawing();
            let revision = doc.revision();
            let mut input = buffer_to(&path, mode);
            if mode == "new" {
                input["sink"] = json!({ "path": dir.join("yeni.gpkg").display().to_string() });
            }
            let input: kentos_commands::ProcessingRun =
                serde_json::from_value(input).expect("an input");
            let flow = StopAt {
                n,
                asked: Default::default(),
                last: Default::default(),
            };
            let end = host
                .run(&mut doc, &input, true, &flow)
                .map_err(|e| e.code)
                .expect("runs");
            assert_eq!(doc.revision(), revision, "{mode} {n}: the drawing is as it was");
            match end {
                RunEnd::Cancelled => {
                    cancelled += 1;
                    assert_eq!(std::fs::read(&path).expect("reads"), before, "{mode} {n}: the file");
                    assert!(!dir.join("yeni.gpkg").exists(), "{mode} {n}: no new file");
                }
                RunEnd::Ran(ran, _) => {
                    finished = true;
                    let (done, total) = flow.last.get();
                    assert!(
                        done == total && total > 0,
                        "{mode} {n}: the rows said how far: {done}/{total}"
                    );
                    assert!(ran.sink.is_some());
                }
            }
            no_stray(&dir);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
    assert!(
        cancelled >= 6 && finished,
        "stopped at different points ({cancelled}), and finished once it was not"
    );
}

#[test]
fn rows_go_in_batches_and_the_batches_are_counted() {
    use kentos_native_application::geopackage::{Flow, WriteStop, write_in};
    let doc = drawing();
    let dir = std::env::temp_dir().join(format!("kentos-sink-batches-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("parcalar.gpkg");
    let mut gp = kentos_geopackage::GeoPackage::create(&path).expect("created");
    let session = gp.session().expect("a session");
    let seen = std::cell::RefCell::new(Vec::new());
    let progress = |done: u64, total: u64| seen.borrow_mut().push((done, total));
    let never = || false;
    let out = write_in(
        &doc,
        &["parsel".to_owned()],
        &session,
        None,
        2,
        &Flow {
            stopped: &never,
            progress: &progress,
        },
    )
    .map_err(|e: WriteStop| format!("{e:?}"))
    .expect("written");
    let rows: u32 = out.tables.iter().map(|t| t.rows).sum();
    assert!(rows > 2, "the parcels are more than a batch: {rows}");
    assert_eq!(out.batches, rows.div_ceil(2), "batches of two rows");
    let seen = seen.into_inner();
    assert_eq!(seen.len() as u32, out.batches);
    assert_eq!(seen.last(), Some(&(u64::from(rows), u64::from(rows))));
    assert!(seen.windows(2).all(|w| w[0].0 < w[1].0), "progress only goes forward");
    session.commit().expect("committed");
    gp.close().expect("closed");
    let _ = std::fs::remove_dir_all(&dir);
}

// ── A tool's table output as a table of the file (docs/adr/0551 §3) ─────────

fn summary_to(path: &std::path::Path, mode: &str, tables: bool) -> Value {
    json!({ "tool": "statistics.summary", "parameters": {
        "input": { "scope": "layer", "layerId": "parsel" }, "field": "Değer", "group": "Ada" },
        "sink": { "path": path.display().to_string(), "mode": mode, "tables": tables } })
}

#[test]
fn a_table_output_is_written_as_an_attribute_table_when_the_sink_asks_for_it() {
    let dir = std::env::temp_dir().join(format!("kentos-sink-tables-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("ozet.gpkg");
    let mut doc = drawing();
    let revision = doc.revision();
    // Without `tables` the tool made no objects: nothing to write, and the answer says how to ask.
    let refused = run(&mut doc, summary_to(&path, "new", false));
    assert_eq!(refused["error"]["code"], "nothing_to_export", "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap_or("").contains("sink.tables"),
        "{refused}"
    );
    assert!(!path.exists());
    let got = run(&mut doc, summary_to(&path, "new", true));
    assert_eq!(got["status"], "completed", "{got}");
    assert_eq!(doc.revision(), revision, "the drawing does not change");
    let report = &got["output"]["sinkReport"];
    let table = &report["tables"][0];
    assert_eq!(table["output"], "table");
    assert!(table["layer"].is_null(), "no layer: {table}");
    assert!(table["geometry"].is_null());
    assert_eq!((table["z"].as_str(), table["m"].as_str()), (Some("none"), Some("none")));
    let shown = &got["output"]["outputs"]["table"];
    let columns: Vec<&str> = shown["columns"]
        .as_array()
        .expect("columns")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let rows = shown["rows"].as_array().expect("rows");
    assert_eq!(table["rows"], json!(rows.len()));
    assert_eq!(
        table["columns"]
            .as_array()
            .expect("columns")
            .iter()
            .map(|c| c["attribute"].as_str().unwrap_or(""))
            .collect::<Vec<_>>(),
        columns
    );
    // The file holds it as text, as the tool shows it, under the output's label.
    let gp = kentos_geopackage::GeoPackage::open(&path, false).expect("opens");
    let layers = gp.layers().expect("layers");
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].name, "Özet tablosu");
    assert_eq!(table["table"], "Özet tablosu");
    assert!(layers[0].geometry.is_none(), "an attribute table");
    let stored = gp
        .rows(&layers[0], &kentos_geopackage::Query::default())
        .expect("rows");
    assert_eq!(stored.len(), rows.len());
    for (s, r) in stored.iter().zip(rows) {
        let want: Vec<kentos_geopackage::Value> = r
            .as_array()
            .expect("a row")
            .iter()
            .map(|c| kentos_geopackage::Value::Text(c.as_str().unwrap_or("").to_owned()))
            .collect();
        assert_eq!(s.values, want);
    }
    // The table's lineage carries the run's record beside the request.
    let meta = gp.metadata("Özet tablosu").expect("metadata");
    assert_eq!(meta.len(), 1);
    let doc_json: Value = serde_json::from_str(&meta[0].1).expect("JSON");
    assert_eq!(doc_json["output"], "table");
    assert!(doc_json["layer"].is_null());
    assert_eq!(doc_json["run"]["format"], "kentos.run-record");
    assert_eq!(doc_json["run"]["tool"], "statistics.summary");
    drop(gp);
    no_stray(&dir);
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&path, fixtures().join("geopackage/v1/kentos-run-ozet.gpkg")).expect("kept");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_table_output_is_added_to_a_file_that_is_there_and_replaced_by_name() {
    let (dir, path) = qgis_copy("table");
    let before = contents(&path);
    let mut doc = drawing();
    let got = run(&mut doc, summary_to(&path, "addTables", true));
    assert_eq!(got["status"], "completed", "{got}");
    assert_eq!(
        names_of(&got["output"]["sinkReport"]["untouched"]).len(),
        before.len()
    );
    let after = contents(&path);
    assert_eq!(after.len(), before.len() + 1);
    for (name, rows) in &before {
        assert_eq!(&after.iter().find(|(n, _)| n == name).expect("kept").1, rows);
    }
    // The same name again: refused, replaced when asked, the others untouched either way.
    let bytes = std::fs::read(&path).expect("reads");
    let again = run(&mut doc, summary_to(&path, "addTables", true));
    assert_eq!(again["error"]["code"], "table_exists", "{again}");
    assert_eq!(std::fs::read(&path).expect("reads"), bytes);
    let replaced = run(&mut doc, summary_to(&path, "replaceTables", true));
    assert_eq!(replaced["status"], "completed", "{replaced}");
    assert_eq!(replaced["output"]["sinkReport"]["tables"][0]["replaced"], true);
    let now = contents(&path);
    assert_eq!(now.len(), after.len());
    for (name, rows) in &before {
        assert_eq!(&now.iter().find(|(n, _)| n == name).expect("kept").1, rows);
    }
    // A stop at any question leaves the file as it was.
    use kentos_native_application::processing_host::{RunEnd, tools};
    kentos_processing::command::register();
    let host = tools().expect("the tools are registered");
    for n in 1..=8 {
        let (d2, p2) = qgis_copy("table-stop");
        let was = std::fs::read(&p2).expect("reads");
        let mut doc = drawing();
        let input: kentos_commands::ProcessingRun =
            serde_json::from_value(summary_to(&p2, "addTables", true)).expect("an input");
        let flow = StopAt {
            n,
            asked: Default::default(),
            last: Default::default(),
        };
        let end = host.run(&mut doc, &input, true, &flow).map_err(|e| e.code).expect("runs");
        if matches!(end, RunEnd::Cancelled) {
            assert_eq!(std::fs::read(&p2).expect("reads"), was, "stopped at {n}");
        }
        no_stray(&d2);
        let _ = std::fs::remove_dir_all(&d2);
    }
    no_stray(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_whose_system_of_that_id_is_another_one_takes_no_table() {
    let dir = std::env::temp_dir().join(format!("kentos-sink-crs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("baska-sistem.gpkg");
    // The drawing is in EPSG:5254 (id 5254); this file's 5254 is a made-up system.
    let mut gp = kentos_geopackage::GeoPackage::create(&path).expect("created");
    let session = gp.session().expect("a session");
    session
        .add_srs(&kentos_geopackage::Srs {
            name: "baska".into(),
            id: 5254,
            organization: "EPSG".into(),
            organization_id: 9999,
            definition: "undefined".into(),
            description: None,
        })
        .expect("added");
    session.commit().expect("committed");
    gp.close().expect("closed");
    let before = std::fs::read(&path).expect("reads");
    let mut doc = drawing();
    let got = run(&mut doc, buffer_to(&path, "addTables"));
    assert_eq!(got["error"]["code"], "crs_mismatch", "{got}");
    assert_eq!(std::fs::read(&path).expect("reads"), before, "the file is as it was");
    no_stray(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── An input read from its source (docs/adr/0592 §1) ──────────────────────

/// Nesneden nokta on sorgu.gpkg's whole `bina` table, its result to a new file: a point a square, at its centre, the
/// drawing unchanged. With `KENTOS_WRITE_GPKG=1` the file is kept as fixtures/geopackage/v1/kentos-run-kaynak.gpkg for
/// `scripts/fixtures/processing_source_cases.py --written`, which reads it with sqlite3 against the source's rows.
#[test]
fn an_input_read_from_its_source_goes_to_a_file() {
    let dir = std::env::temp_dir().join(format!("kentos-sink-kaynak-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("kentos-run-kaynak.gpkg");
    let source = fixtures().join("sources/v1/sorgu.gpkg");
    let mut doc = drawing();
    let before = doc.revision();
    let got = run(
        &mut doc,
        json!({ "tool": "convert.toPoints", "parameters": {
            "input": { "scope": "source", "path": source.display().to_string(), "table": "bina" } },
            "sink": { "path": path.display().to_string() } }),
    );
    assert_eq!(got["status"], "completed", "{got}");
    assert_eq!(got["output"]["inputs"][0]["rows"], json!(12), "{got}");
    assert_eq!(got["output"]["sink"]["tables"][0]["rows"], json!(12), "{got}");
    assert_eq!(doc.revision(), before, "the drawing does not change");
    assert!(
        doc.layers().get("kaynak-input").is_none(),
        "the records went into the run's copy, not the drawing"
    );
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        std::fs::copy(&path, fixtures().join("geopackage/v1/kentos-run-kaynak.gpkg")).expect("keeps the file");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

//! Batch runs and run records (docs/adr/0529): what the shared command cases cannot say because they cannot stop a
//! run between rows or look at the drawing's layers: a stop between rows takes back the rows before it, a batch is one
//! undo step, a model that stops in the middle leaves no layer behind, a record names the persistent ids and the seed,
//! and running a record again is told apart (same revision, same data, changed data). With `onError: continue` a row
//! that stops is taken back alone (docs/adr/0551 §1), and a stop heard inside a row takes the whole batch back (§4).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kentos_commands::{
    BatchOnError, BatchRow, DocumentSnapshotV1, ProcessingBatch, ProcessingRun,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::processing_host::{BatchEnd, BatchFlow, RunEnd, tools};
use kentos_processing::record::{Rerun, rerun_kind};
use serde_json::{Value, json};

fn load(name: &str) -> Document {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/processing/v1")
        .join(name);
    let text = std::fs::read_to_string(path).expect("the drawing is there");
    Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("reads")).expect("opens")
}

fn uid(doc: &Document, slot: u32) -> String {
    doc.uid(Slot(slot)).expect("an object").to_string()
}

/// A batch's flow that asks to stop at its `stop_at`-th question, and keeps what it was told of the progress.
struct Flow {
    stop_at: u32,
    asked: Cell<u32>,
    progress: RefCell<Vec<(u64, u64)>>,
    /// The stop as a flag the running tool reads (a job's), when the flow has one.
    flag: Option<Arc<AtomicBool>>,
}

impl Flow {
    fn new(stop_at: u32) -> Self {
        Flow {
            stop_at,
            asked: Cell::new(0),
            progress: RefCell::new(Vec::new()),
            flag: None,
        }
    }

    /// A flow never asked to stop between rows whose flag is `flag`.
    fn with_flag(flag: Arc<AtomicBool>) -> Self {
        Flow {
            flag: Some(flag),
            ..Flow::new(u32::MAX)
        }
    }
}

impl BatchFlow for Flow {
    fn stopped(&self) -> bool {
        let n = self.asked.get();
        self.asked.set(n + 1);
        n >= self.stop_at
    }

    fn progress(&self, done: u64, total: u64) {
        self.progress.borrow_mut().push((done, total));
    }

    fn stop_flag(&self) -> Option<Arc<AtomicBool>> {
        self.flag.clone()
    }
}

/// A run that is never stopped: its answer.
fn ran_it(doc: &mut Document, run: &ProcessingRun) -> kentos_commands::ProcessingRan {
    match tools()
        .expect("registered")
        .run(doc, run, true, &Flow::new(u32::MAX))
        .expect("runs")
    {
        RunEnd::Ran(ran, _) => ran,
        RunEnd::Cancelled => panic!("it was not stopped"),
    }
}

fn values(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

/// Three rows of Öznitelik hesapla over the three first parcels, each writing a field of its own.
fn three_rows(doc: &Document) -> ProcessingBatch {
    ProcessingBatch {
        environment: None,
        tool: "attributes.calculate".into(),
        parameters: values(&[("value", json!("$genişlik || ' x ' || $yükseklik"))]),
        rows: ["B1", "B2", "B3"]
            .iter()
            .map(|f| BatchRow {
                label: None,
                parameters: values(&[("field", json!(f))]),
            })
            .collect(),
        selection: (1..=3).map(|n| uid(doc, n)).collect(),
        view: None,
        drawing: None,
        on_error: Default::default(),
        sink: None,
    }
}

fn setup() {
    kentos_processing::command::register();
}

#[test]
fn a_stop_between_rows_takes_back_the_rows_before_it() {
    setup();
    let mut doc = load("parcels.kcad");
    let before = (
        doc.revision(),
        doc.get(Slot(1)).map(|e| e.base().attrs.clone()),
    );
    let batch = three_rows(&doc);
    // Asked before each row: yes, yes, then stop: two rows ran, the third did not.
    let flow = Flow::new(2);
    let end = tools()
        .expect("registered")
        .batch(&mut doc, &batch, true, &flow)
        .expect("not refused");
    assert!(matches!(end, BatchEnd::Cancelled));
    assert!(!doc.can_undo(), "nothing was written");
    assert_eq!(doc.revision(), before.0);
    assert_eq!(doc.get(Slot(1)).map(|e| e.base().attrs.clone()), before.1);
    assert_eq!(*flow.progress.borrow(), vec![(0, 3), (1, 3)]);
}

#[test]
fn a_batch_is_one_undo_step_and_tells_its_progress() {
    setup();
    let mut doc = load("parcels.kcad");
    let batch = three_rows(&doc);
    let flow = Flow::new(u32::MAX);
    let end = tools()
        .expect("registered")
        .batch(&mut doc, &batch, true, &flow)
        .expect("not refused");
    let BatchEnd::Ran(ran, _) = end else {
        panic!("ran");
    };
    assert_eq!(
        *flow.progress.borrow(),
        vec![(0, 3), (1, 3), (2, 3), (3, 3)]
    );
    assert_eq!(ran.rows.len(), 3);
    assert_eq!(ran.step.as_deref(), Some("Toplu: Öznitelik hesapla"));
    let attrs = doc.get(Slot(1)).expect("parcel").base().attrs.clone();
    assert!(attrs.contains_key("B1") && attrs.contains_key("B2") && attrs.contains_key("B3"));
    // One step takes all three rows back.
    assert_eq!(doc.undo().as_deref(), Some("Toplu: Öznitelik hesapla"));
    assert!(!doc.can_undo());
    let attrs = doc.get(Slot(1)).expect("parcel").base().attrs.clone();
    assert!(!attrs.contains_key("B1"));
}

/// The record of the batch has the rows as its steps, each with the counts of the row's run.
#[test]
fn a_batch_record_has_its_rows_as_steps() {
    setup();
    let mut doc = load("parcels.kcad");
    let batch = three_rows(&doc);
    let BatchEnd::Ran(ran, _) = tools()
        .expect("registered")
        .batch(&mut doc, &batch, true, &Flow::new(u32::MAX))
        .expect("not refused")
    else {
        panic!("ran");
    };
    let r = &ran.record;
    assert_eq!(
        (r.kind.as_str(), r.format.as_str(), r.version),
        ("batch", "kentos.run-record", 1)
    );
    assert_eq!(
        r.steps.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["rows[0]", "rows[1]", "rows[2]"]
    );
    assert!(
        r.steps
            .iter()
            .all(|s| s.records_in == 3 && s.records_out == 3)
    );
    // Nine objects read, the same three written by every row: the distinct ones counted once.
    assert_eq!((r.total.records_in, r.total.records_out), (9, 3));
    assert_eq!(r.srid, Some(5256));
}

/// A model that stops at its second step takes its first step's layer back too, not only its objects.
#[test]
fn a_model_that_stops_in_the_middle_leaves_no_layer_behind() {
    setup();
    let mut json_doc: Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../fixtures/processing/v1/parcels.kcad"),
        )
        .expect("file"),
    )
    .expect("json");
    json_doc["layers"].as_array_mut().expect("layers").push(json!({
        "id": "islem-kenar-olculeri", "name": "Kenar ölçüleri", "type": "layer", "visible": true,
        "locked": false, "expanded": true,
        "style": { "color": "fg-dim", "lineType": "continuous", "lineWeight": 0.18 }, "children": [],
        "fields": [{ "name": "Uzunluk (m)", "kind": "integer" }]
    }));
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&json_doc.to_string()).expect("reads"),
    )
    .expect("opens");
    let count = doc.entities().count();
    let revision = doc.revision();
    let run = ProcessingRun {
        environment: None,
        tool: "model:builtin.parcelSheet".into(),
        parameters: BTreeMap::new(),
        selection: vec![uid(&doc, 1), uid(&doc, 2)],
        view: None,
        sink: None,
        drawing: None,
    };
    let error = tools()
        .expect("registered")
        .run(&mut doc, &run, true, &Flow::new(u32::MAX))
        .expect_err("the second step stops");
    assert_eq!(error.code, "tool_failed");
    assert_eq!(doc.entities().count(), count);
    assert!(
        doc.layers().get("islem-kose-noktalari").is_none(),
        "the first step's new layer is taken back"
    );
    assert!(!doc.can_undo());
    assert_eq!(doc.revision(), revision);
}

/// The record names the objects by persistent id, keeps a tool's seed and the project's system.
#[test]
fn a_record_names_the_objects_the_seed_and_the_system() {
    setup();
    let mut doc = load("parcels.kcad");
    let one = uid(&doc, 1);
    let run = ProcessingRun {
        environment: None,
        tool: "sample.random".into(),
        parameters: values(&[
            ("input", json!({ "scope": "uids", "uids": [one.clone()] })),
            ("count", json!("3")),
            ("seed", json!(7)),
        ]),
        selection: Vec::new(),
        view: None,
        sink: None,
        drawing: None,
    };
    let ran = ran_it(&mut doc, &run);
    let r = &ran.record;
    assert_eq!(r.seed, Some(7));
    assert_eq!(r.srid, Some(5256));
    assert_eq!(
        r.parameters["input"],
        json!({ "scope": "uids", "uids": [one] })
    );
    assert_eq!(r.parameters["seed"], json!(7));
    assert_eq!((r.kind.as_str(), r.tool_version), ("tool", 1));
    assert_eq!(r.inputs.len(), 1);
    assert_eq!(r.inputs[0].layer, "parsel");
    assert_eq!(r.steps.len(), 1);
    assert_eq!(r.steps[0].records_in, 1);
    assert_eq!(r.steps[0].records_out, 3);
    // A model's definition is a digest, a tool has none.
    assert!(r.definition.is_none());
    // The started time reads as UTC with milliseconds.
    assert!(
        r.started_at.ends_with('Z') && r.started_at.len() == 24,
        "{}",
        r.started_at
    );
}

#[test]
fn running_a_record_again_is_told_apart() {
    setup();
    let mut doc = load("parcels.kcad");
    let run = ProcessingRun {
        environment: None,
        tool: "selection.byExpression".into(),
        parameters: values(&[("input", json!({ "scope": "layer", "layerId": "parsel" }))]),
        selection: Vec::new(),
        view: None,
        sink: None,
        drawing: None,
    };
    let ran = ran_it(&mut doc, &run);
    let record = ran.record;
    // The run did not edit: the drawing is still at the revision it began with.
    assert_eq!(rerun_kind(&record, &doc), Rerun::SameRevision);
    // Something else changes (an object on another layer): another revision, the layer it read the same.
    let other = doc
        .entities()
        .find(|e| e.base().layer_id != "parsel")
        .map(|e| e.base().id)
        .expect("an object elsewhere");
    doc.remove(&[Slot(other)]);
    assert_eq!(rerun_kind(&record, &doc), Rerun::SameData);
    // The layer it read changes: the same values would meet other data.
    let parcel = doc
        .entities()
        .find(|e| e.base().layer_id == "parsel")
        .map(|e| e.base().id)
        .expect("a parcel");
    doc.remove(&[Slot(parcel)]);
    assert_eq!(
        rerun_kind(&record, &doc),
        Rerun::ChangedData(vec!["parsel".into()])
    );
    // The record is a value: it still says what the run read.
    assert_eq!(record.inputs.len(), 1);
}

/// What a batch costs on a big layer (docs/adr/0529 §6): each row runs the tool and takes the record's digest of the
/// layer it read. Not run by default:
/// `cargo test --release -p kentos-processing --test batch batch_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timing, run by hand"]
fn batch_timing() {
    setup();
    let parcels: usize = std::env::var("KENTOS_PERF_PARCELS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);
    let mut json_doc: Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../fixtures/processing/v1/parcels.kcad"),
        )
        .expect("file"),
    )
    .expect("json");
    let squares: Vec<Value> = (0..parcels)
        .map(|i| {
            let (x, y) = (
                487_000.0 + (i % 200) as f64 * 30.0,
                4_420_000.0 + (i / 200) as f64 * 30.0,
            );
            json!({ "kind": "polygon", "id": 100 + i, "layerId": "parsel", "label": i.to_string(),
                "attrs": { "Ada": "1", "Parsel": i.to_string() },
                "pts": [{"x": x, "y": y}, {"x": x + 25.0, "y": y}, {"x": x + 25.0, "y": y + 25.0}, {"x": x, "y": y + 25.0}] })
        })
        .collect();
    json_doc["entities"] = Value::Array(squares);
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&json_doc.to_string()).expect("reads"),
    )
    .expect("opens");
    let rows = 20usize;
    let batch = ProcessingBatch {
        environment: None,
        tool: "attributes.calculate".into(),
        parameters: values(&[
            ("input", json!({ "scope": "layer", "layerId": "parsel" })),
            ("value", json!("$alan")),
        ]),
        rows: (0..rows)
            .map(|i| BatchRow {
                label: None,
                parameters: values(&[("field", json!(format!("A{i}")))]),
            })
            .collect(),
        selection: Vec::new(),
        view: None,
        drawing: None,
        on_error: Default::default(),
        sink: None,
    };
    let clock = std::time::Instant::now();
    let BatchEnd::Ran(ran, _) = tools()
        .expect("registered")
        .batch(&mut doc, &batch, true, &Flow::new(u32::MAX))
        .expect("runs")
    else {
        panic!("ran");
    };
    let total = clock.elapsed();
    let rows_ms: Vec<u64> = ran.rows.iter().map(|r| r.ran.record.ms).collect();
    println!(
        "{parcels} parcels, {rows} rows: {:.0} ms in all, {:.1} ms a row (rows' own record: {} to {} ms)",
        total.as_secs_f64() * 1000.0,
        total.as_secs_f64() * 1000.0 / rows as f64,
        rows_ms.iter().min().copied().unwrap_or(0),
        rows_ms.iter().max().copied().unwrap_or(0),
    );
    // Rows that only choose leave the layer as it is: its digest is taken once for all of them.
    let choose = ProcessingBatch {
        environment: None,
        tool: "selection.byExpression".into(),
        parameters: values(&[("input", json!({ "scope": "layer", "layerId": "parsel" }))]),
        rows: (0..rows)
            .map(|i| BatchRow {
                label: None,
                parameters: values(&[("condition", json!(format!("$alan > {}", 500 + i)))]),
            })
            .collect(),
        selection: Vec::new(),
        view: None,
        drawing: None,
        on_error: Default::default(),
        sink: None,
    };
    let clock = std::time::Instant::now();
    let BatchEnd::Ran(..) = tools()
        .expect("registered")
        .batch(&mut doc, &choose, true, &Flow::new(u32::MAX))
        .expect("runs")
    else {
        panic!("ran");
    };
    let total = clock.elapsed();
    println!(
        "{parcels} parcels, {rows} choosing rows: {:.0} ms in all, {:.1} ms a row",
        total.as_secs_f64() * 1000.0,
        total.as_secs_f64() * 1000.0 / rows as f64,
    );
    // The record's digest of the layer, alone.
    let clock = std::time::Instant::now();
    let digest = kentos_native_application::sources::layer_digest(&doc, "parsel");
    println!(
        "the layer's digest alone: {:.1} ms ({} chars)",
        clock.elapsed().as_secs_f64() * 1000.0,
        digest.len()
    );
}

// docs/adr/0535: a raster tool's record names its grids: the source read, its band and the cells worked out (an
// area's), and the result written on the source's grid from the window's first cell.
#[test]
fn a_raster_run_records_its_grids() {
    setup();
    let dir = std::env::temp_dir().join(format!("kentos-grids-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let tif = dir.join("tepe.tif");
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    std::fs::copy(root.join("terrain/v1/tepe.tif"), &tif).expect("copied");
    let text = std::fs::read_to_string(root.join("processing/v1/surface.kcad"))
        .expect("the drawing")
        .replace(
            "\"tepe.tif\"",
            &serde_json::to_string(&tif.display().to_string()).unwrap(),
        );
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("reads"))
        .expect("opens");
    let run = ProcessingRun {
        environment: None,
        tool: "surface.slope".into(),
        parameters: values(&[
            ("input", json!({ "scope": "uids", "uids": [uid(&doc, 1)] })),
            ("area", json!("objects")),
            (
                "areaObjects",
                json!({ "scope": "layer", "layerId": "alan" }),
            ),
            ("add", json!(false)),
        ]),
        selection: Vec::new(),
        view: None,
        sink: None,
        drawing: None,
    };
    let ran = ran_it(&mut doc, &run);
    let grids = &ran.record.grids;
    assert_eq!(grids.len(), 2, "{grids:?}");
    assert_eq!(grids[0].role, "input");
    assert_eq!(grids[0].window, Some([8, 10, 11, 11]));
    assert_eq!(
        (grids[0].width, grids[0].height, grids[0].band),
        (41, 33, Some(1))
    );
    assert_eq!(grids[1].role, "output");
    assert_eq!((grids[1].width, grids[1].height), (11, 11));
    assert_eq!(
        grids[1].affine,
        [500180.0, 10.0, 0.0, 4420230.0, 0.0, -10.0]
    );
    let out = grids[1].file.clone().expect("the result's file");
    assert!(std::path::Path::new(&out).exists(), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

// ── onError: continue and a stop inside a row (docs/adr/0551) ───────────────

/// Three rows of Öznitelik hesapla into the parcel layer's whole-number field “Ağaç sayısı”, over queries.kcad: the
/// second row's value is no number and the field refuses it when the row writes.
fn tree_rows(on_error: BatchOnError, values_of: [&str; 3]) -> ProcessingBatch {
    ProcessingBatch {
        environment: None,
        tool: "attributes.calculate".into(),
        parameters: values(&[
            ("input", json!({ "scope": "layer", "layerId": "parsel" })),
            ("field", json!("Ağaç sayısı")),
        ]),
        rows: values_of
            .iter()
            .map(|v| BatchRow {
                label: None,
                parameters: values(&[("value", json!(v))]),
            })
            .collect(),
        selection: Vec::new(),
        view: None,
        drawing: None,
        on_error,
        sink: None,
    }
}

fn trees(doc: &Document) -> Vec<Option<String>> {
    (1..=4)
        .map(|n| {
            doc.get(Slot(n))
                .and_then(|e| e.base().attrs.get("Ağaç sayısı").cloned())
        })
        .collect()
}

#[test]
fn with_continue_a_row_that_stops_is_taken_back_alone_and_the_others_are_written() {
    setup();
    let mut doc = load("queries.kcad");
    let before = trees(&doc);
    let batch = tree_rows(BatchOnError::Continue, ["5", "'çok'", "7"]);
    let BatchEnd::Ran(ran, warnings) = tools()
        .expect("registered")
        .batch(&mut doc, &batch, true, &Flow::new(u32::MAX))
        .expect("not refused")
    else {
        panic!("ran");
    };
    assert_eq!(
        ran.rows.iter().map(|r| r.index).collect::<Vec<_>>(),
        [0, 2],
        "the rows that ran"
    );
    assert_eq!(ran.failed.len(), 1);
    assert_eq!(
        (ran.failed[0].index, ran.failed[0].code.as_str()),
        (1, "tool_failed")
    );
    assert!(ran.failed[0].message.contains("tam sayı ister"), "{}", ran.failed[0].message);
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "batch_rows_failed");
    assert!(ran.summary.contains("1 satır çalışmadı"), "{}", ran.summary);
    // The record names the rows that ran as steps and the one that stopped as a failure.
    assert_eq!(
        ran.record.steps.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        ["rows[0]", "rows[2]"]
    );
    assert_eq!(ran.record.failed.len(), 1);
    assert_eq!(ran.record.failed[0].id, "rows[1]");
    // The last row's value stands, the step is one and takes it all back.
    assert_eq!(trees(&doc), vec![Some("7".to_owned()); 4]);
    assert_eq!(ran.step.as_deref(), Some("Toplu: Öznitelik hesapla"));
    assert_eq!(doc.undo().as_deref(), Some("Toplu: Öznitelik hesapla"));
    assert!(!doc.can_undo());
    assert_eq!(trees(&doc), before);
}

#[test]
fn with_stop_the_same_batch_takes_everything_back() {
    setup();
    let mut doc = load("queries.kcad");
    let before = trees(&doc);
    let revision = doc.revision();
    let batch = tree_rows(BatchOnError::Stop, ["5", "'çok'", "7"]);
    let error = tools()
        .expect("registered")
        .batch(&mut doc, &batch, true, &Flow::new(u32::MAX))
        .err()
        .expect("refused");
    assert_eq!(error.code, "batch_row_failed");
    assert_eq!(error.path.as_deref(), Some("rows[1]"));
    assert_eq!(trees(&doc), before);
    assert_eq!(doc.revision(), revision);
    assert!(!doc.can_undo());
}

#[test]
fn with_continue_a_batch_whose_rows_all_stop_writes_nothing() {
    setup();
    let mut doc = load("queries.kcad");
    let before = trees(&doc);
    let revision = doc.revision();
    let batch = tree_rows(BatchOnError::Continue, ["'a'", "'b'", "'c'"]);
    let BatchEnd::Ran(ran, warnings) = tools()
        .expect("registered")
        .batch(&mut doc, &batch, true, &Flow::new(u32::MAX))
        .expect("not refused")
    else {
        panic!("ran");
    };
    assert!(ran.rows.is_empty());
    assert_eq!(ran.failed.len(), 3);
    assert_eq!(warnings[0].code, "batch_rows_failed");
    assert!(ran.step.is_none(), "no step: nothing was written");
    assert_eq!(trees(&doc), before);
    assert_eq!(doc.revision(), revision);
    assert!(!doc.can_undo());
}

/// A model row that stops in its middle step has its first step's objects and layer taken back, the rows after it run.
#[test]
fn with_continue_a_model_row_that_stops_in_the_middle_leaves_no_layer_behind() {
    setup();
    let mut json_doc: Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../fixtures/processing/v1/parcels.kcad"),
        )
        .expect("file"),
    )
    .expect("json");
    json_doc["layers"].as_array_mut().expect("layers").push(json!({
        "id": "islem-kenar-olculeri", "name": "Kenar ölçüleri", "type": "layer", "visible": true,
        "locked": false, "expanded": true,
        "style": { "color": "fg-dim", "lineType": "continuous", "lineWeight": 0.18 }, "children": [],
        "fields": [{ "name": "Uzunluk (m)", "kind": "integer" }]
    }));
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&json_doc.to_string()).expect("reads"),
    )
    .expect("opens");
    let count = doc.entities().count();
    let revision = doc.revision();
    let batch = ProcessingBatch {
        environment: None,
        tool: "model:builtin.parcelSheet".into(),
        parameters: BTreeMap::new(),
        rows: vec![BatchRow::default(), BatchRow::default()],
        selection: vec![uid(&doc, 1), uid(&doc, 2)],
        view: None,
        drawing: None,
        on_error: BatchOnError::Continue,
        sink: None,
    };
    let BatchEnd::Ran(ran, _) = tools()
        .expect("registered")
        .batch(&mut doc, &batch, true, &Flow::new(u32::MAX))
        .expect("not refused")
    else {
        panic!("ran");
    };
    assert!(ran.rows.is_empty());
    assert_eq!(ran.failed.len(), 2, "both rows stop at their second step");
    assert!(ran.failed[0].message.contains("2. adım"), "{}", ran.failed[0].message);
    assert_eq!(doc.entities().count(), count, "each row's first step is taken back");
    assert!(doc.layers().get("islem-kose-noktalari").is_none());
    assert!(!doc.can_undo());
    assert_eq!(doc.revision(), revision);
}

/// A stop heard inside the row (the job's flag, read as the tool runs) is not a row's failure: whatever the
/// `onError`, the whole batch is taken back and nothing is written.
#[test]
fn a_stop_heard_inside_a_row_takes_the_whole_batch_back() {
    setup();
    for on_error in [BatchOnError::Stop, BatchOnError::Continue] {
        let mut doc = load("queries.kcad");
        let before = trees(&doc);
        let revision = doc.revision();
        let batch = tree_rows(on_error, ["5", "6", "7"]);
        // The between-rows question is never answered yes; only the flag the tool reads is set.
        let flag = Arc::new(AtomicBool::new(true));
        let flow = Flow::with_flag(Arc::clone(&flag));
        assert!(flow.flag.as_ref().is_some_and(|f| f.load(Ordering::Relaxed)));
        let end = tools()
            .expect("registered")
            .batch(&mut doc, &batch, true, &flow)
            .expect("not refused");
        assert!(matches!(end, BatchEnd::Cancelled), "{on_error:?}");
        assert_eq!(trees(&doc), before);
        assert_eq!(doc.revision(), revision);
        assert!(!doc.can_undo());
    }
}

/// The same for one run (`cad.processing.run` as a job): stopped inside, the drawing is as it was.
#[test]
fn a_stop_heard_inside_a_run_leaves_the_drawing_as_it_was() {
    setup();
    let mut doc = load("queries.kcad");
    let before = trees(&doc);
    let revision = doc.revision();
    let run = ProcessingRun {
        environment: None,
        tool: "attributes.calculate".into(),
        parameters: values(&[
            ("input", json!({ "scope": "layer", "layerId": "parsel" })),
            ("field", json!("Ağaç sayısı")),
            ("value", json!("5")),
        ]),
        selection: Vec::new(),
        view: None,
        sink: None,
        drawing: None,
    };
    let flag = Arc::new(AtomicBool::new(true));
    let end = tools()
        .expect("registered")
        .run(&mut doc, &run, true, &Flow::with_flag(flag))
        .expect("not refused");
    assert!(matches!(end, RunEnd::Cancelled));
    assert_eq!(trees(&doc), before);
    assert_eq!(doc.revision(), revision);
    assert!(!doc.can_undo());
}

/// A layer's digest is taken again for each state of its objects, also inside a command's open step (where the
/// document's generation does not move): the second row, which reads what the first wrote, has another digest.
#[test]
fn a_row_that_reads_what_the_row_before_it_wrote_takes_a_new_digest() {
    setup();
    let mut doc = load("queries.kcad");
    let batch = tree_rows(BatchOnError::Stop, ["5", "6", "7"]);
    // Inside an open transaction, as the command protocol runs a command.
    let ran: Result<kentos_commands::ProcessingBatched, ()> = doc.transact("Komut", |doc| {
        match tools()
            .expect("registered")
            .batch(doc, &batch, true, &Flow::new(u32::MAX))
            .map_err(|_| ())?
        {
            BatchEnd::Ran(ran, _) => Ok(ran),
            BatchEnd::Cancelled => Err(()),
        }
    });
    let ran = ran.expect("ran");
    let digests: Vec<&str> = ran
        .rows
        .iter()
        .map(|r| r.ran.record.inputs[0].digest.as_str())
        .collect();
    assert_eq!(digests.len(), 3);
    assert_ne!(digests[0], digests[1], "row 2 read what row 1 wrote");
    assert_ne!(digests[1], digests[2]);
}

/// A run given a `requestId` leaves its answer, record included, in the project's request ledger: what the file keeps
/// of it (docs/adr/0551 §6; the ledger is saved with the drawing, docs/adr/0253 §6).
#[test]
fn a_run_with_a_request_id_keeps_its_record_in_the_projects_ledger() {
    use kentos_commands::{CallEnvelope, CallOp, CommandResult};
    setup();
    let mut doc = load("queries.kcad");
    let envelope = CallEnvelope {
        relax_constraints: false,
        command: "cad.processing.run".into(),
        version: None,
        op: CallOp::Execute,
        input: json!({ "tool": "attributes.calculate", "parameters": {
            "input": { "scope": "layer", "layerId": "parsel" }, "field": "Ağaç sayısı", "value": "5" } }),
        request_id: Some("kayit:1".into()),
        expected: None,
    };
    let answer = kentos_native_application::call::call(&mut doc, None, envelope);
    assert!(matches!(answer.result, CommandResult::Completed { .. }), "{:?}", answer.result);
    let ledger = doc.requests();
    let kept = ledger.iter().find(|r| r.request_id == "kayit:1").expect("in the ledger");
    assert_eq!(kept.command, "cad.processing.run");
    let stored: Value = serde_json::from_str(kept.answer.as_deref().expect("the answer")).expect("JSON");
    let record = &stored["output"]["record"];
    assert_eq!(record["format"], "kentos.run-record");
    assert_eq!(record["tool"], "attributes.calculate");
    assert_eq!(record["inputs"][0]["layer"], "parsel");
    assert!(record["inputs"][0]["digest"].as_str().is_some_and(|d| d.len() == 64));
}

/// A copy of the sources' GeoPackage in a folder of its own, and that folder.
fn source_copy(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("kentos-batch-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("sorgu.gpkg");
    std::fs::copy(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/sources/v1/sorgu.gpkg"),
        &file,
    )
    .expect("copied");
    (dir, file)
}

/// docs/adr/0593 §3, §4: a run and a batch reading their input from a file keep the input as given, the file and its
/// SHA-256 in the record; the result's file is in the journal's row with its size and SHA-256; when the source file
/// changes, running the record again meets changed data, whatever the drawing's revision.
#[test]
fn a_record_of_a_source_keeps_the_file_and_the_journal_the_result() {
    use kentos_commands::{CallOp, ProcessingBatched, ProcessingRan};
    use kentos_native_application::{ExecutionContext, registry};
    kentos_processing::command::register();
    let (dir, file) = source_copy("kaynak");
    let mut doc = load("parcels.kcad");
    let given = json!({ "scope": "source", "path": file.display().to_string(), "table": "bina",
                        "filter": "kat >= 3" });
    let out = dir.join("sonuc.gpkg");
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.processing.run",
        CallOp::Execute,
        json!({ "tool": "convert.toPoints", "parameters": { "input": given },
                "sink": { "path": out.display().to_string() } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let ran: ProcessingRan = serde_json::from_value(answer["output"].clone()).expect("an answer");
    let record = &ran.record;
    assert_eq!(record.parameters["input"], given, "the input as given runs it again");
    let input = &record.inputs[0];
    assert_eq!(input.path.as_deref(), Some(file.to_str().expect("a path")));
    assert_eq!(input.table.as_deref(), Some("sorgu.gpkg › bina"));
    let sink = record.sink.as_ref().expect("where it went");
    let made = ran.sink.as_ref().expect("the file");
    assert_eq!(sink.sha256.as_deref(), Some(made.sha256.as_str()));
    assert_eq!(sink.tables[0].table, "Merkezler");
    let row = doc.journal().last().expect("a row").clone();
    assert_eq!(row.command.as_deref(), Some("cad.processing.run"));
    assert_eq!(row.effects.len(), 1);
    assert_eq!(row.effects[0].path, out.display().to_string());
    assert_eq!((row.effects[0].bytes, row.effects[0].sha256.as_str()), (made.bytes, made.sha256.as_str()));
    assert_eq!(rerun_kind(record, &doc), Rerun::SameRevision);

    // A batch to a second file: its record and its journal row say the same.
    let out2 = dir.join("toplu.gpkg");
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.processing.batch",
        CallOp::Execute,
        json!({ "tool": "convert.toPoints", "parameters": { "input": given },
                "rows": [{ "parameters": { "layer": { "newName": "Bir" } } },
                         { "parameters": { "layer": { "newName": "İki" } } }],
                "sink": { "path": out2.display().to_string() } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let done: ProcessingBatched = serde_json::from_value(answer["output"].clone()).expect("an answer");
    let tables: Vec<(&str, Option<&str>)> = done
        .record
        .sink
        .as_ref()
        .expect("where it went")
        .tables
        .iter()
        .map(|t| (t.table.as_str(), t.row.as_deref()))
        .collect();
    assert_eq!(tables, [("Bir", Some("rows[0]")), ("İki", Some("rows[1]"))]);
    let row = doc.journal().last().expect("a row");
    assert_eq!(row.command.as_deref(), Some("cad.processing.batch"));
    assert_eq!(row.effects[0].path, out2.display().to_string());
    assert_eq!(rerun_kind(&done.record, &doc), Rerun::SameRevision);

    // The source changes: both records meet changed data, the result files they name are as they were.
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().append(true).open(&file).expect("opens");
        f.write_all(b"\0").expect("writes");
    }
    let changed = Rerun::ChangedData(vec!["sorgu.gpkg › bina".into()]);
    assert_eq!(rerun_kind(record, &doc), changed);
    assert_eq!(rerun_kind(&done.record, &doc), changed);
    assert!(changed.sentence().contains("sorgu.gpkg › bina"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/0593 §3: an input read from a database is said as not known when the record is run again.
#[test]
fn a_record_of_a_database_source_is_not_known_again() {
    let doc = load("parcels.kcad");
    let record = kentos_commands::ProcessingRecord {
        format: "kentos.run-record".into(),
        version: 1,
        kind: "tool".into(),
        tool: "convert.toPoints".into(),
        label: "Nesneden nokta".into(),
        tool_version: 1,
        definition: None,
        parameters: BTreeMap::new(),
        selection: Vec::new(),
        view: None,
        inputs: vec![kentos_commands::RunInput {
            layer: "kaynak-input".into(),
            digest: String::new(),
            source: None,
            revision: None,
            table: Some("PostGIS: localhost/vt › bina".into()),
            rows: Some(12),
            path: Some("postgresql://kentos@localhost/vt".into()),
        }],
        document_revision: doc.revision().to_string(),
        srid: None,
        seed: None,
        engine: String::new(),
        host: "headless".into(),
        started_at: String::new(),
        ms: 0,
        grids: Vec::new(),
        steps: Vec::new(),
        total: kentos_commands::RecordTotal {
            records_in: 0,
            records_out: 0,
        },
        failed: Vec::new(),
        sink: None,
    };
    assert_eq!(
        rerun_kind(&record, &doc),
        Rerun::UnknownSource(vec!["PostGIS: localhost/vt › bina".into()])
    );
}

/// docs/adr/0593 §1, §2: a model reads its input from a file and writes to another, as a tool does; in a batch each
/// row runs the whole model. The parcel model's three steps on the 12 squares of `bina`: 48 corners numbered (12 × 4,
/// the squares apart), their lengths as texts (no GeoPackage counterpart: said), the area written to the squares of
/// the copy (not the drawing's, not the file's).
#[test]
fn a_model_reads_its_source_and_writes_to_a_file_alone_and_in_a_batch() {
    use kentos_commands::{CallOp, ProcessingBatched, ProcessingRan};
    use kentos_native_application::{ExecutionContext, registry};
    kentos_processing::command::register();
    let (dir, file) = source_copy("model");
    let mut doc = load("parcels.kcad");
    let before = doc.revision();
    let given = json!({ "scope": "source", "path": file.display().to_string(), "table": "bina" });
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.processing.run",
        CallOp::Execute,
        json!({ "tool": "model:builtin.parcelSheet", "parameters": { "parcels": given },
                "sink": { "path": dir.join("model.gpkg").display().to_string() } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let ran: ProcessingRan = serde_json::from_value(answer["output"].clone()).expect("an answer");
    assert_eq!(ran.record.kind, "model");
    assert_eq!(ran.record.steps.len(), 3);
    assert_eq!(ran.record.parameters["parcels"], given);
    assert_eq!(ran.inputs[0].rows, Some(12));
    let tables: Vec<(&str, u32)> = ran
        .sink
        .as_ref()
        .expect("the file")
        .tables
        .iter()
        .map(|t| (t.table.as_str(), t.rows))
        .collect();
    assert_eq!(tables, [("Köşe noktaları", 48)]);
    assert!(
        answer["warnings"]
            .as_array()
            .is_some_and(|w| w.iter().any(|w| w["code"] == "export_losses")),
        "the lengths' texts are said: {answer}"
    );
    assert_eq!(doc.revision(), before, "the drawing does not change");

    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.processing.batch",
        CallOp::Execute,
        json!({ "tool": "model:builtin.parcelSheet", "parameters": { "parcels": given },
                "rows": [{ "parameters": { "prefix": "A" } }, { "parameters": { "prefix": "B" } }],
                "sink": { "path": dir.join("toplu-model.gpkg").display().to_string() } }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let done: ProcessingBatched = serde_json::from_value(answer["output"].clone()).expect("an answer");
    assert_eq!(done.record.kind, "batch");
    assert!(done.rows.iter().all(|r| r.ran.record.steps.len() == 3));
    // Both rows write to the model's one layer. Numbering finds the first row's corners where the second's would go
    // and numbers them again (as on the drawing): one table of 48 corners, made by the first row, named by the second.
    let sink = done.record.sink.as_ref().expect("where it went");
    assert_eq!(sink.tables.len(), 1);
    assert_eq!(
        (sink.tables[0].rows, sink.tables[0].row.as_deref()),
        (48, Some("rows[0]"))
    );
    assert!(done.rows[1].ran.added.iter().all(|u| !done.rows[0].ran.added.contains(u)));
    assert_eq!(doc.revision(), before);
    let _ = std::fs::remove_dir_all(&dir);
}

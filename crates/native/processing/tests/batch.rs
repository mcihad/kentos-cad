//! Batch runs and run records (docs/adr/0529): what the shared command cases cannot say because they cannot stop a
//! run between rows or look at the drawing's layers: a stop between rows takes back the rows before it, a batch is one
//! undo step, a model that stops in the middle leaves no layer behind, a record names the persistent ids and the seed,
//! and running a record again is told apart (same revision, same data, changed data).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use kentos_commands::{BatchRow, DocumentSnapshotV1, ProcessingBatch, ProcessingRun};
use kentos_domain::{Document, Slot};
use kentos_native_application::processing_host::{BatchEnd, BatchFlow, tools};
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
}

impl Flow {
    fn new(stop_at: u32) -> Self {
        Flow {
            stop_at,
            asked: Cell::new(0),
            progress: RefCell::new(Vec::new()),
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
        .batch(&mut doc, &batch, &flow)
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
        .batch(&mut doc, &batch, &flow)
        .expect("not refused");
    let BatchEnd::Ran(ran) = end else {
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
    let BatchEnd::Ran(ran) = tools()
        .expect("registered")
        .batch(&mut doc, &batch, &Flow::new(u32::MAX))
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
        tool: "model:builtin.parcelSheet".into(),
        parameters: BTreeMap::new(),
        selection: vec![uid(&doc, 1), uid(&doc, 2)],
        view: None,
        sink: None,
    };
    let error = tools()
        .expect("registered")
        .run(&mut doc, &run, true)
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
        tool: "sample.random".into(),
        parameters: values(&[
            ("input", json!({ "scope": "uids", "uids": [one.clone()] })),
            ("count", json!("3")),
            ("seed", json!(7)),
        ]),
        selection: Vec::new(),
        view: None,
        sink: None,
    };
    let (ran, _) = tools()
        .expect("registered")
        .run(&mut doc, &run, true)
        .expect("runs");
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
        tool: "selection.byExpression".into(),
        parameters: values(&[("input", json!({ "scope": "layer", "layerId": "parsel" }))]),
        selection: Vec::new(),
        view: None,
        sink: None,
    };
    let (ran, _) = tools()
        .expect("registered")
        .run(&mut doc, &run, true)
        .expect("runs");
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
    };
    let clock = std::time::Instant::now();
    let BatchEnd::Ran(ran) = tools()
        .expect("registered")
        .batch(&mut doc, &batch, &Flow::new(u32::MAX))
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
    };
    let clock = std::time::Instant::now();
    let BatchEnd::Ran(_) = tools()
        .expect("registered")
        .batch(&mut doc, &choose, &Flow::new(u32::MAX))
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
    };
    let (ran, _) = tools()
        .expect("registered")
        .run(&mut doc, &run, true)
        .expect("runs");
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

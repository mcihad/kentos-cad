//! The window's run and `cad.processing.run`'s are one run (plan/gis.md CBS-41, docs/adr/0548): every case of the
//! vector tools and models is played as the İşlemler window plays it and again through the command, with the same
//! values in the command's words (`command_values`: objects by persistent ids, the selection and the box given
//! explicitly). The drawings after the two runs, the new layers, the summaries and the notes are the same; a value the
//! window refuses the command refuses with the window's words. So what a case shows of a tool, Python and MCP get too.

use super::*;
use kentos_commands::ProcessingRun;
use kentos_native_application::processing_host::tools;
use kentos_processing::command::{command_values, register};
use kentos_processing::model_runner::MODEL_PREFIX;

/// The case files of the tools that run on the drawing alone.
const FILES: [&str; 16] = [
    "cases.json",
    "queries.json",
    "proximity.json",
    "layers.json",
    "convert.json",
    "table-points.json",
    "lrs.json",
    "generate.json",
    "generalize.json",
    "sampling.json",
    "transfer.json",
    "address.json",
    "geometry.json",
    "spatial-stats.json",
    "spatial-inference.json",
    "network.json",
];

/// The tree's layers by their order, ids left out: a layer a run makes takes a new id each time.
fn tree_shape(doc: &Document) -> Value {
    let mut v = serde_json::to_value(doc.layers().nodes()).expect("the tree writes");
    fn strip(v: &mut Value) {
        match v {
            Value::Object(o) => {
                o.remove("id");
                // A derivation record's time is the run's own (docs/adr/0633 §2): two runs are never at one moment.
                if o.contains_key("engine") {
                    o.remove("at");
                }
                o.values_mut().for_each(strip);
            }
            Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    strip(&mut v);
    v
}

/// The case files of the raster tools: the window's run reads the cases' rasters from memory and keeps what it
/// writes (cases/surface.rs); the command's reads them beside the drawing's file in a folder of its own and writes
/// its results there, as Python and MCP do (`drawing`, docs/adr/0548 §3).
const RASTER_FILES: [&str; 10] = [
    "surface.json",
    "interpolation.json",
    "raster-ops.json",
    "raster-vector.json",
    "hydrology.json",
    "distance.json",
    "suitability.json",
    "remote.json",
    "multidim.json",
    "pointcloud.json",
];

/// Where a raster case's command reads and writes: a folder of its own with the case's rasters in it.
struct Place {
    folder: PathBuf,
    rasters: BTreeMap<String, Vec<u8>>,
    written: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl Place {
    fn new(rasters: &BTreeMap<String, Vec<u8>>, id: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("kentos-equivalence-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("the folder");
        for (name, bytes) in rasters {
            std::fs::write(folder.join(name), bytes).expect("a raster writes");
        }
        Place {
            folder,
            rasters: rasters.clone(),
            written: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// What the command wrote: the files in the folder that were not there before, by name.
    fn made(&self) -> BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(&self.folder)
            .expect("the folder reads")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| !self.rasters.contains_key(n))
            .map(|n| {
                let bytes = std::fs::read(self.folder.join(&n)).expect("a file reads");
                (n, bytes)
            })
            .collect()
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// The drawing's objects in their order, a new layer's id put as its name, a path in `folder` as its name in it.
fn objects_in(doc: &Document, folder: Option<&std::path::Path>) -> Vec<Value> {
    let prefix = folder.map(|f| format!("{}/", f.display()));
    objects(doc)
        .into_iter()
        .map(|v| match &prefix {
            Some(p) => {
                serde_json::from_str(&v.to_string().replace(p.as_str(), "")).expect("an object")
            }
            None => v,
        })
        .collect()
}

/// The drawing's objects in their order, a new layer's id put as its name.
fn objects(doc: &Document) -> Vec<Value> {
    let names: BTreeMap<String, String> = doc
        .layers()
        .leaves()
        .into_iter()
        .map(|l| (l.id.clone(), l.name.clone()))
        .collect();
    doc.entities()
        .map(|e| {
            let mut v = plain(e);
            if let Some(id) = v["layerId"].as_str()
                && let Some(name) = names.get(id)
            {
                v["layerId"] = json!(name);
            }
            v
        })
        .collect()
}

/// A file parameter as Python and MCP give it: its path, read by the command as the dialog reads a chosen file (a
/// window's record keeps the file's name, not its content).
fn with_paths(
    mut parameters: BTreeMap<String, Value>,
    tool: &Tool,
    c: &Value,
    files: &Value,
) -> BTreeMap<String, Value> {
    for p in &tool.parameters {
        if let (ParamKind::File { .. }, Some(Value::String(name))) =
            (&p.kind, c["values"].get(&p.name))
        {
            let path = folder().join(files[name.as_str()].as_str().expect("the file"));
            parameters.insert(p.name.clone(), json!({ "path": path.to_string_lossy() }));
        }
    }
    parameters
}

/// One case both ways: what differs, if anything.
fn both_ways(
    c: &Value,
    registry: &Registry,
    files: &Value,
    place: Option<&Place>,
) -> Option<String> {
    let id = c["id"].as_str().unwrap_or("?");
    let (tool_id, tool) = match c["run"].get("tool").and_then(Value::as_str) {
        Some(t) => (t.to_owned(), registry.tool(t).expect("the tool")),
        None => {
            let m = c["run"]["model"].as_str().expect("a tool or a model");
            let lookup = |id: &str| registry.tool(id);
            let model = registry.model(m).expect("the model");
            (format!("{MODEL_PREFIX}{m}"), model_as_tool(model, &lookup))
        }
    };
    let window = match place {
        None => play(c, registry, false, files),
        Some(p) => {
            let files: Arc<dyn Files> = Arc::new(super::surface::FixtureFiles {
                rasters: p.rasters.clone(),
                written: p.written.clone(),
            });
            play_with(c, registry, true, &json!({}), Some(files))
        }
    };

    let mut doc = load(c["document"].as_str().expect("a document"));
    let values = with_values(&tool, &doc, c.get("values"), files);
    let selection = slots(c.get("selection"))
        .into_iter()
        .filter_map(|s| doc.uid(s).map(|u| u.to_string()))
        .collect();
    let view = c.get("view").and_then(Value::as_array).map(|v| {
        let n = |i: usize| v[i].as_f64().expect("a number");
        [n(0), n(1), n(2), n(3)]
    });
    let input = ProcessingRun {
        environment: None,
        tool: tool_id,
        parameters: with_paths(command_values(&doc, &tool, &values), &tool, c, files),
        selection,
        view,
        sink: None,
        drawing: place.map(|p| p.folder.join("cizim.kcad").display().to_string()),
    };
    struct Go;
    impl kentos_native_application::processing_host::BatchFlow for Go {
        fn stopped(&self) -> bool {
            false
        }
        fn progress(&self, _: u64, _: u64) {}
    }
    let ran = tools()
        .expect("the tools")
        .run(&mut doc, &input, true, &Go)
        .map(|end| match end {
            kentos_native_application::processing_host::RunEnd::Ran(ran, warnings) => (ran, warnings),
            kentos_native_application::processing_host::RunEnd::Cancelled => panic!("it was not stopped"),
        });

    match (&window.outcome, ran) {
        (Outcome::Ok { record, .. }, Ok((ran, _))) => {
            let mut why = Vec::new();
            // A file the command wrote is named with its folder; the window's test host names it alone.
            let prefix = place.map(|p| format!("{}/", p.folder.display()));
            let bare = |t: &str| match &prefix {
                Some(p) => t.replace(p.as_str(), ""),
                None => t.to_owned(),
            };
            let summary = bare(&ran.summary);
            if summary != record.summary {
                why.push(format!("özet: {:?} ≠ {:?}", record.summary, summary));
            }
            let notes: Vec<(String, String)> = window
                .log
                .iter()
                .map(|l| {
                    let level = if l.level == Level::Info {
                        "info"
                    } else {
                        "warn"
                    };
                    (level.to_owned(), l.text.clone())
                })
                .collect();
            let said: Vec<(String, String)> = ran
                .notes
                .iter()
                .map(|n| (n.level.clone(), bare(&n.text)))
                .collect();
            if notes != said {
                why.push(format!("notlar: {notes:?} ≠ {said:?}"));
            }
            if tree_shape(&window.host.doc) != tree_shape(&doc) {
                why.push("katman ağacı farklı".into());
            }
            let folder = place.map(|p| p.folder.as_path());
            let (a, b) = (objects(&window.host.doc), objects_in(&doc, folder));
            if a.len() != b.len() {
                why.push(format!("nesne sayısı {} ≠ {}", a.len(), b.len()));
            } else if let Some(i) = (0..a.len()).find(|&i| !same(&a[i], &b[i])) {
                why.push(format!("{i}. nesne: {} ≠ {}", a[i], b[i]));
            }
            if let Some(p) = place {
                let kept = p
                    .written
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                let made = p.made();
                if kept.keys().ne(made.keys()) {
                    why.push(format!(
                        "dosyalar {:?} ≠ {:?}",
                        kept.keys().collect::<Vec<_>>(),
                        made.keys().collect::<Vec<_>>()
                    ));
                } else if let Some(n) = kept.keys().find(|n| kept[*n] != made[*n]) {
                    why.push(format!("“{n}” bayt bayt farklı"));
                }
            }
            (!why.is_empty()).then(|| format!("{id}: {}", why.join("; ")))
        }
        (Outcome::Invalid { issues }, Err(e)) => {
            let want = issues.first().map(|i| i.message.as_str()).unwrap_or("");
            (e.message != want).then(|| format!("{id}: ret “{want}” ≠ “{}”", e.message))
        }
        (Outcome::Stopped { message, .. }, Err(e)) => {
            (e.message != *message).then(|| format!("{id}: durma “{message}” ≠ “{}”", e.message))
        }
        (_, Err(e)) => Some(format!(
            "{id}: pencere çalıştı ama komut reddetti: {} ({})",
            e.message, e.code
        )),
        (_, Ok(_)) => Some(format!("{id}: pencere reddetti ama komut çalıştı")),
    }
}

#[test]
fn the_window_and_the_command_run_the_same() {
    register();
    let registry = Registry::builtin();
    let mut played = 0;
    let mut problems = Vec::new();
    for name in FILES {
        let file = case_file(name);
        for c in file["cases"].as_array().expect("cases") {
            played += 1;
            if let Some(p) = both_ways(c, &registry, &file["files"], None) {
                problems.push(format!("{name} {p}"));
            }
        }
    }
    for name in RASTER_FILES {
        let file = case_file(name);
        let rasters: BTreeMap<String, Vec<u8>> = file["rasters"]
            .as_object()
            .map(|m| {
                m.iter()
                    .map(|(n, path)| {
                        let bytes = super::surface::surface_fixture(path.as_str().expect("a path"));
                        (n.clone(), bytes)
                    })
                    .collect()
            })
            .unwrap_or_default();
        for c in file["cases"].as_array().expect("cases") {
            played += 1;
            let place = Place::new(&rasters, c["id"].as_str().unwrap_or("?"));
            if let Some(p) = both_ways(c, &registry, &json!({}), Some(&place)) {
                problems.push(format!("{name} {p}"));
            }
        }
    }
    assert!(played > 400, "{played} durum");
    assert!(
        problems.is_empty(),
        "{} durumdan {} farklı:\n{}",
        played,
        problems.len(),
        problems.join("\n")
    );
}

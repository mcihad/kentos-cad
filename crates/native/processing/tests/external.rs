//! An extension's tool in İşlemler (plan/eklenti.md ADIM 19; docs/adr/8057 §7): its descriptor read into a tool
//! without its code, its run through a provider standing in for the extension's process, its answer checked whole.
//! The window's way (the runner) and the commands' way (`cad.processing.run`, what Python and MCP call) run it alike.

use std::sync::{Arc, Mutex, OnceLock};

use kentos_commands::{DocumentSnapshotV1, Entity, ProcessingRun};
use kentos_domain::{Document, Slot};
use kentos_processing::external::{self, ExternalTool, Failure, Provider};
use kentos_processing::runner::{Outcome, Runner, Status};
use kentos_processing::types::{Feedback, Target, Values};
use kentos_processing::{Bounds, Host, Registry, Scene};
use serde_json::{Value, json};

fn descriptor() -> Value {
    json!({
        "format": "kentos.processingTool",
        "schemaVersion": 1,
        "id": "olc",
        "label": "Parsel alanları",
        "description": "Her parselin alanını hesaplar ve parselleri yeni katmana yazar.",
        "modelDescription": "Use it to get each parcel's area in square metres; parcels over the limit are selected.",
        "keywords": ["alan"],
        "parameters": [
            { "name": "parcels", "label": "Parseller", "type": "features", "kinds": ["polygon"],
              "scopes": ["layer", "selection", "all"] },
            { "name": "limit", "label": "Büyük parsel sınırı", "type": "number", "unit": "m²", "default": 500 },
            { "name": "output", "label": "Çıktı katmanı", "type": "layer", "default": { "newName": "Alanlar" } }
        ],
        "outputs": [
            { "name": "parcels", "label": "Ölçülen parseller", "type": "features", "sinkParameter": "output" },
            { "name": "table", "label": "Alan tablosu", "type": "table" },
            { "name": "total", "label": "Toplam alan", "type": "number" }
        ]
    })
}

/// What the provider was given, and what it answers.
#[derive(Default)]
struct Seen {
    inputs: Vec<Value>,
    answer: Option<Result<Value, Failure>>,
}

static SEEN: OnceLock<Mutex<Seen>> = OnceLock::new();

fn seen() -> std::sync::MutexGuard<'static, Seen> {
    SEEN.get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn shoelace(pts: &[Value]) -> f64 {
    let xy: Vec<(f64, f64)> = pts
        .iter()
        .map(|p| (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap()))
        .collect();
    let mut twice = 0.0;
    for i in 0..xy.len() {
        let (x0, y0) = xy[i];
        let (x1, y1) = xy[(i + 1) % xy.len()];
        twice += x0 * y1 - x1 * y0;
    }
    twice.abs() / 2.0
}

/// Stands in for the extension's process: measures each parcel as its Python would, or gives the answer set.
struct Stand;

impl Provider for Stand {
    fn run(
        &self,
        tool: &ExternalTool,
        input: Value,
        feedback: &mut dyn Feedback,
    ) -> Result<Value, Failure> {
        assert_eq!(tool.local, "olc");
        feedback.progress(0.5, "ölçülüyor");
        let mut seen = seen();
        seen.inputs.push(input.clone());
        if let Some(answer) = seen.answer.clone() {
            return answer;
        }
        let limit = input["params"]["limit"].as_f64().unwrap();
        let features = input["inputs"]["parcels"]["features"].as_array().unwrap();
        let mut out = Vec::new();
        let mut rows = Vec::new();
        let mut big = Vec::new();
        let mut total = 0.0;
        for f in features {
            let area = shoelace(f["entity"]["pts"].as_array().unwrap());
            total += area;
            out.push(json!({ "source": f["uid"], "attrs": { "alan": area } }));
            rows.push(json!({ "parsel": f["entity"]["attrs"]["Parsel"], "alan": area }));
            if area > limit {
                big.push(f["uid"].clone());
            }
        }
        // Its rows as the extension's process sends them (ext_host.table): their columns in order.
        let table = json!({
            "columns": ["parsel", "alan"],
            "rows": rows.iter().map(|r| json!([r["parsel"], r["alan"]])).collect::<Vec<_>>(),
        });
        Ok(json!({
            "kind": "outputs",
            "outputs": { "parcels": out, "table": table, "total": total },
            "select": big,
            "summary": format!("{} parsel ölçüldü.", features.len()),
        }))
    }
}

fn drawing(withheld: bool) -> Document {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/processing/v1/parcels.kcad");
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    if withheld {
        // “Nitelik” is kept in the drawing (docs/adr/0638 §5): a third party's code reads it cut.
        for layer in v["layers"].as_array_mut().unwrap() {
            if layer["id"] == "parsel" {
                layer["fields"] = json!([{ "name": "Nitelik", "kind": "text", "withheld": true }]);
            }
        }
    }
    Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).unwrap()).unwrap()
}

struct Bare(Document, Vec<Slot>);

impl Scene for Bare {
    fn doc(&self) -> &Document {
        &self.0
    }

    fn selected(&self) -> Vec<Slot> {
        Vec::new()
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        None
    }
}

impl Host for Bare {
    fn doc_mut(&mut self) -> &mut Document {
        &mut self.0
    }

    fn select(&mut self, ids: &[Slot]) {
        self.1 = ids.to_vec();
    }
}

fn values() -> Values {
    json!({
        "parcels": { "scope": "layer", "layerId": "parsel" },
        "limit": 500,
        "output": { "newName": "Alanlar" },
    })
    .as_object()
    .unwrap()
    .clone()
}

/// One test at a time: they share the process's shelf, its provider and what it was given.
fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    static TURN: Mutex<()> = Mutex::new(());
    TURN.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Every test's shelf: the tool registered, the provider standing in. The tests share one process and its shelf.
/// The extension's package digest the host gives (yapay).
const DIGEST: &str = "sha256:abababababababababababababababababababababababababababababababab";

fn setup() -> ExternalTool {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let mut tool =
            external::from_descriptor("tr.ornek.deneme", "Deneme", "1.0.0", "olc", &descriptor())
                .unwrap();
        // The host gives the package's digest (docs/adr/8057 §13).
        tool.digest = DIGEST.into();
        external::set_tools(vec![tool]);
        external::register_provider(Arc::new(Stand));
        kentos_processing::command::register();
    });
    external::find("ext.tr.ornek.deneme.olc").unwrap()
}

/// The new layer's objects: their attributes and areas, in order.
fn written(doc: &Document) -> Vec<(String, String)> {
    let layer = doc
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == "Alanlar")
        .map(|l| l.id.clone())
        .expect("the new layer");
    doc.entities()
        .filter(|e| e.base().layer_id == layer)
        .map(|e| {
            (
                e.base().attrs.get("Parsel").cloned().unwrap_or_default(),
                e.base().attrs.get("alan").cloned().unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn a_descriptor_is_read_into_a_tool_without_its_code() {
    let _turn = one_at_a_time();
    let tool = setup();
    assert_eq!(tool.tool.id, "ext.tr.ornek.deneme.olc");
    assert_eq!(tool.tool.category, external::CATEGORY);
    assert_eq!(
        tool.tool.targets,
        vec![Target::Worker],
        "the window never waits for it"
    );
    let names: Vec<&str> = tool
        .tool
        .parameters
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, ["parcels", "limit", "output"]);
    let help = tool.tool.help.as_deref().unwrap();
    assert!(help.contains("Deneme eklentisinin aracı (tr.ornek.deneme 1.0.0)"));
    assert!(
        help.contains("ayrı süreç bir korumalı alan değildir"),
        "{help}"
    );
    assert_eq!(
        external::split(&tool.tool.id),
        Some(("tr.ornek.deneme", "olc"))
    );
    // The commands list it with the built-in tools.
    let registry = Registry::current();
    assert!(registry.get("ext.tr.ornek.deneme.olc").is_some());
    assert_eq!(registry.category_path(&tool.tool.category), "Eklentiler");
}

#[test]
fn a_descriptor_that_does_not_fit_is_refused_with_its_place() {
    let refused = |change: &dyn Fn(&mut Value)| {
        let mut d = descriptor();
        change(&mut d);
        external::from_descriptor("tr.ornek.deneme", "Deneme", "1.0.0", "olc", &d)
            .err()
            .expect("refused")
    };
    assert!(refused(&|d| d["id"] = json!("baska")).contains("“olc”"));
    assert!(refused(&|d| d["schemaVersion"] = json!(2)).contains("sürüm 1"));
    assert!(
        refused(&|d| d["parameters"][1]["type"] = json!("rasterValues"))
            .contains("parameters[1]: “rasterValues” türü")
    );
    assert!(
        refused(&|d| d["parameters"][1]["name"] = json!("class"))
            .contains("Python'da bir ad olamaz")
    );
    assert!(refused(&|d| d["parameters"][1]["name"] = json!("parcels")).contains("iki parametre"));
    assert!(
        refused(&|d| d["outputs"][0]["sinkParameter"] = json!("limit"))
            .contains("bir katman (layer) parametresi değil")
    );
    assert!(refused(&|d| d["outputs"][2]["type"] = json!("raster")).contains("çıktı türü"));
    assert!(
        refused(&|d| d["capabilities"] = json!({ "runtimes": ["browser"] }))
            .contains("desktop-python")
    );
}

#[test]
fn the_window_and_the_commands_run_an_extensions_tool_alike() {
    let _turn = one_at_a_time();
    let _tool = setup();
    seen().answer = None;
    // The window's way: the runner prepares, computes and applies in one undo step.
    let mut host = Bare(drawing(false), Vec::new());
    let tool = Registry::current().get("ext.tr.ornek.deneme.olc").unwrap();
    let mut runner = Runner::new();
    let mut log = Vec::new();
    let outcome = runner.run(&mut host, &tool, &values(), false, &mut log);
    let Outcome::Ok { result, added, .. } = outcome else {
        panic!("ran: {outcome:?}");
    };
    assert_eq!(added.len(), 3);
    assert_eq!(result.summary.as_deref(), Some("3 parsel ölçüldü."));
    assert_eq!(result.outputs["total"], json!(1800.0));
    let table = &result.outputs["table"];
    assert_eq!(table["columns"], json!(["parsel", "alan"]));
    assert_eq!(table["rows"][0], json!(["1", "600"]));
    let window = written(&host.0);
    assert_eq!(
        window,
        [
            ("1".into(), "600".into()),
            ("2".into(), "750".into()),
            ("3".into(), "450".into())
        ]
    );
    assert_eq!(host.1.len(), 2, "the parcels over the limit are selected");
    // One undo step takes back the layer and its objects.
    let objects = host.0.entities().count();
    assert_eq!(host.0.undo().as_deref(), Some("Parsel alanları"));
    assert_eq!(host.0.entities().count(), objects - 3);
    assert!(host.0.layers().leaves().iter().all(|l| l.name != "Alanlar"));

    // The commands' way (Python, MCP): the same tool, values and answer.
    let mut doc = drawing(false);
    let hosted = kentos_native_application::processing_host::tools().expect("registered");
    let run: ProcessingRun = serde_json::from_value(json!({
        "tool": "ext.tr.ornek.deneme.olc",
        "parameters": values(),
    }))
    .unwrap();
    struct Never;
    impl kentos_native_application::processing_host::BatchFlow for Never {
        fn stopped(&self) -> bool {
            false
        }
        fn progress(&self, _: u64, _: u64) {}
    }
    hosted.run(&mut doc, &run, true, &Never).expect("runs");
    assert_eq!(
        written(&doc),
        window,
        "the same objects, the same attributes"
    );
}

#[test]
fn the_inputs_go_as_data_with_persistent_ids_and_hidden_fields_cut() {
    let _turn = one_at_a_time();
    let _tool = setup();
    seen().answer = None;
    let mut host = Bare(drawing(true), Vec::new());
    let tool = Registry::current().get("ext.tr.ornek.deneme.olc").unwrap();
    let outcome = Runner::new().run(&mut host, &tool, &values(), false, &mut Vec::new());
    assert!(matches!(outcome, Outcome::Ok { .. }), "{outcome:?}");
    let input = seen().inputs.last().cloned().unwrap();
    let features = input["inputs"]["parcels"]["features"].as_array().unwrap();
    assert_eq!(features.len(), 3);
    let first = &features[0];
    let uid = first["uid"].as_str().unwrap();
    assert_eq!(uid.len(), 36, "a persistent id, UUID text: {uid}");
    assert_eq!(first["layer"], "Parsel", "its layer's name");
    assert_eq!(
        first["entity"]["id"], 0,
        "a slot is not an id outside the program"
    );
    assert_eq!(first["entity"]["attrs"]["Nitelik"], "‹gizli›");
    assert_eq!(first["entity"]["attrs"]["Ada"], "101");
    assert!(
        input["params"].get("output").is_none(),
        "where it writes is the host's"
    );
    assert_eq!(input["params"]["limit"], 500);
    assert!(
        input["inputs"]["parcels"]["description"]
            .as_str()
            .unwrap()
            .contains("kapalı alan")
    );
}

#[test]
fn an_answer_that_does_not_fit_writes_nothing() {
    let _turn = one_at_a_time();
    let _tool = setup();
    let cases = [
        (
            json!({ "outputs": { "nope": 1 } }),
            "“nope” araç tanımında bir çıktı değil",
        ),
        (
            json!({ "outputs": { "parcels": [{ "source": "00000000-0000-0000-0000-000000000000" }] } }),
            "araca verilen nesnelerden biri değil",
        ),
        (
            json!({ "outputs": { "parcels": [{ "geometry": { "type": "Polygon", "coordinates": [[[0, 0], [1, 0], [0, 0]]] } }] } }),
            "en az 3 köşe",
        ),
        (
            json!({ "outputs": { "parcels": [{ "geometry": { "type": "Point", "coordinates": [0, null] } }] } }),
            "iki sonlu sayı",
        ),
        (
            json!({ "outputs": { "parcels": [{ "geometry": { "type": "GeometryCollection", "geometries": [] } }] } }),
            "“GeometryCollection” şekli yazılamaz",
        ),
        (
            json!({ "outputs": { "parcels": [{ "geometry": { "type": "Point", "coordinates": [0, 0] }, "attrs": { "a": [1] } }] } }),
            "tek bir değer olmalı",
        ),
        (
            json!({ "outputs": { "total": "çok" } }),
            "“total” bir sayı olmalı",
        ),
        (json!({ "outputs": { "table": [1, 2] } }), "bir satır"),
    ];
    for (answer, words) in cases {
        seen().answer = Some(Ok(answer));
        let mut host = Bare(drawing(false), Vec::new());
        let before = host.0.entities().count();
        let tool = Registry::current().get("ext.tr.ornek.deneme.olc").unwrap();
        let outcome = Runner::new().run(&mut host, &tool, &values(), false, &mut Vec::new());
        let Outcome::Stopped {
            status, message, ..
        } = outcome
        else {
            panic!("refused: {outcome:?}");
        };
        assert_eq!(status, Status::Error);
        assert!(message.contains(words), "{message}");
        assert!(
            message.starts_with("“Parsel alanları” tipli sonucu geçersiz"),
            "{message}"
        );
        assert_eq!(host.0.entities().count(), before, "nothing written");
        assert!(host.0.undo().is_none(), "no undo step");
    }
    // The code failed: said as it is.
    seen().answer = Some(Err(Failure::Failed(
        "“Parsel alanları” hata verdi: ZeroDivisionError".into(),
    )));
    let mut host = Bare(drawing(false), Vec::new());
    let tool = Registry::current().get("ext.tr.ornek.deneme.olc").unwrap();
    let outcome = Runner::new().run(&mut host, &tool, &values(), false, &mut Vec::new());
    assert!(
        matches!(&outcome, Outcome::Stopped { status: Status::Error, message, .. } if message.contains("ZeroDivisionError")),
        "{outcome:?}"
    );
    seen().answer = None;
}

#[test]
fn a_new_shape_of_the_tools_own_goes_to_the_output_layer_as_geojson_gives_it() {
    let _turn = one_at_a_time();
    let _tool = setup();
    // What `ext.polygon`, shapely's and GeoPandas' `__geo_interface__` give: GeoJSON, rings closed, a hole, parts.
    seen().answer = Some(Ok(json!({
        "outputs": { "parcels": [
            { "geometry": { "type": "Polygon", "coordinates": [
                [[487000, 4420000], [487010, 4420000], [487010, 4420010], [487000, 4420000]],
                [[487006, 4420001], [487009, 4420001], [487009, 4420004], [487006, 4420001]]
              ] },
              "attrs": { "ad": "üçgen", "n": 3, "tamam": true, "boş": null }, "label": "Ü" },
            { "geometry": { "type": "Point", "coordinates": [487005, 4420005, 1102.5] } },
            { "geometry": { "type": "LineString", "coordinates": [[487000, 4420000], [487020, 4420030]] } },
            { "geometry": { "type": "MultiPolygon", "coordinates": [
                [[[0, 0], [1, 0], [1, 1], [0, 0]]], [[[5, 5], [6, 5], [6, 6], [5, 5]]]
              ] } },
            { "type": "Feature", "geometry": { "type": "Point", "coordinates": [1, 2] },
              "properties": { "kaynak": "geopandas" } }
        ] }
    })));
    let mut host = Bare(drawing(false), Vec::new());
    let tool = Registry::current().get("ext.tr.ornek.deneme.olc").unwrap();
    let outcome = Runner::new().run(&mut host, &tool, &values(), false, &mut Vec::new());
    let Outcome::Ok { added, .. } = outcome else {
        panic!("{outcome:?}");
    };
    let objects: Vec<&Entity> = added.iter().map(|s| host.0.get(*s).unwrap()).collect();
    let Entity::Polygon(triangle) = objects[0] else {
        panic!("an area");
    };
    assert_eq!(
        triangle.pts.len(),
        3,
        "the ring's closing position is left out"
    );
    assert_eq!(triangle.holes.as_ref().map(Vec::len), Some(1));
    let first = &triangle.base;
    assert_eq!(first.attrs.get("ad").map(String::as_str), Some("üçgen"));
    assert_eq!(first.attrs.get("n").map(String::as_str), Some("3"));
    assert_eq!(first.attrs.get("tamam").map(String::as_str), Some("true"));
    assert!(
        !first.attrs.contains_key("boş"),
        "an empty value is left out"
    );
    assert_eq!(first.label.as_deref(), Some("Ü"));
    assert!(matches!(objects[1], Entity::Point(p) if p.p.x == 487005.0));
    assert!(matches!(objects[2], Entity::Polyline(l) if l.pts.len() == 2));
    let Entity::Polygon(two) = objects[3] else {
        panic!("an area of two parts");
    };
    assert_eq!(two.parts.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        objects[4].base().attrs.get("kaynak").map(String::as_str),
        Some("geopandas")
    );
    seen().answer = None;
}

#[test]
fn an_input_goes_with_its_shape_as_geojson_its_arcs_tessellated() {
    let _turn = one_at_a_time();
    let _tool = setup();
    seen().answer = None;
    // The first parcel's top edge an arc (a cadastral parcel's curve).
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/processing/v1/parcels.kcad");
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    v["entities"][0]["bulges"] = json!([0.0, 0.0, 0.4, 0.0]);
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&v.to_string()).unwrap()).unwrap();
    let mut host = Bare(doc, Vec::new());
    let tool = Registry::current().get("ext.tr.ornek.deneme.olc").unwrap();
    let outcome = Runner::new().run(&mut host, &tool, &values(), false, &mut Vec::new());
    assert!(matches!(outcome, Outcome::Ok { .. }), "{outcome:?}");
    let input = seen().inputs.last().cloned().unwrap();
    let features = input["inputs"]["parcels"]["features"].as_array().unwrap();
    let curved = &features[0]["geometry"];
    assert_eq!(curved["type"], "Polygon");
    let ring = curved["coordinates"][0].as_array().unwrap();
    assert!(
        ring.len() > 10,
        "the arc tessellated: {} positions",
        ring.len()
    );
    assert_eq!(ring.first(), ring.last(), "a GeoJSON ring is closed");
    let straight = &features[1]["geometry"];
    assert_eq!(straight["coordinates"][0].as_array().unwrap().len(), 5);
    // Its area from the chords is the arc's within a few hundredths of a square metre: 20 × 30 and a circular segment
    // of chord 20 m, sagitta 0.4 · 10 = 4 m (r = 14.5 m, θ = 4 atan 0.4).
    let xy: Vec<(f64, f64)> = ring
        .iter()
        .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
        .collect();
    let twice: f64 = xy
        .windows(2)
        .map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1)
        .sum();
    let theta = 4.0 * 0.4f64.atan();
    let r = (10.0f64.powi(2) + 4.0f64.powi(2)) / (2.0 * 4.0);
    let exact = 600.0 + r * r / 2.0 * (theta - theta.sin());
    assert!(
        (twice.abs() / 2.0 - exact).abs() < 0.05,
        "{} m² for {exact} m²",
        twice.abs() / 2.0
    );
}

#[test]
fn an_extensions_tool_chains_with_a_built_in_one_in_a_model() {
    // P12: the extension's new objects are the next step's input, as a built-in tool's would be; the model is one undo
    // step as a whole.
    let _turn = one_at_a_time();
    let _tool = setup();
    seen().answer = None;
    let model = kentos_processing::Model::from_text(&json!({
        "id": "olc-tampon",
        "label": "Ölç ve tampon",
        "category": "analysis",
        "description": "Parselleri ölçer, ölçülenlerin çevresine 2 m tampon çizer.",
        "inputs": [{ "name": "parseller", "label": "Parseller", "type": "features", "kinds": ["polygon"] }],
        "steps": [
            { "id": "olc", "tool": "ext.tr.ornek.deneme.olc", "values": {
                "parcels": { "kind": "input", "name": "parseller" },
                "limit": { "kind": "value", "value": 500 },
                "output": { "kind": "value", "value": { "newName": "Alanlar" } } } },
            { "id": "tampon", "tool": "geometry.buffer", "values": {
                "input": { "kind": "output", "step": "olc", "output": "parcels" },
                "distance": { "kind": "value", "value": 2 },
                "layer": { "kind": "value", "value": { "newName": "Tampon" } } } }
        ],
        "outputs": []
    })
    .to_string())
    .expect("a model");
    let mut host = Bare(drawing(false), Vec::new());
    let before = host.0.entities().count();
    let registry = Registry::current();
    let lookup = |id: &str| registry.tool(id);
    let inputs = json!({ "parseller": { "scope": "layer", "layerId": "parsel" } })
        .as_object()
        .unwrap()
        .clone();
    let outcome = kentos_processing::model_runner::run_model(
        &model,
        &inputs,
        &mut Runner::new(),
        &mut host,
        &lookup,
        &mut Vec::new(),
    );
    assert!(matches!(outcome, Outcome::Ok { .. }), "{outcome:?}");
    let on = |name: &str| {
        let layer = host
            .0
            .layers()
            .leaves()
            .into_iter()
            .find(|l| l.name == name)
            .map(|l| l.id.clone())
            .unwrap_or_default();
        host.0
            .entities()
            .filter(|e| e.base().layer_id == layer)
            .count()
    };
    assert_eq!(on("Alanlar"), 3, "the extension's step wrote its objects");
    assert_eq!(on("Tampon"), 3, "the built-in step buffered them");
    assert_eq!(host.0.entities().count(), before + 6);
    assert!(host.0.undo().is_some());
    assert_eq!(
        host.0.entities().count(),
        before,
        "the whole model is one undo step"
    );
}

/// Stands in for an extension's expression function: `ada-parsel(ada, parsel)` → "101/1"; counts its batches.
#[derive(Default)]
struct AdaParsel {
    batches: Mutex<Vec<usize>>,
}

impl kentos_expression::external::ExternalCalls for AdaParsel {
    fn call(
        &self,
        name: &str,
        rows: &[Vec<kentos_expression::external::Arg>],
    ) -> Vec<kentos_expression::external::Arg> {
        use kentos_expression::external::Arg;
        assert_eq!(name, "ext.tr.ornek.deneme.ada-parsel");
        self.batches.lock().unwrap().push(rows.len());
        rows.iter()
            .map(|r| match (r.first(), r.get(1)) {
                (Some(Arg::Text(a)), Some(Arg::Text(p))) => Arg::Text(format!("{a}/{p}")),
                _ => Arg::Null,
            })
            .collect()
    }
}

#[test]
fn an_extensions_expression_function_answers_a_processing_expression_a_batch_at_a_time() {
    let _turn = one_at_a_time();
    let _tool = setup();
    let answers = Arc::new(AdaParsel::default());
    external::set_expressions(vec![kentos_expression::external::ExternalDef {
        name: "ext.tr.ornek.deneme.ada-parsel".into(),
        description: "Ada/parsel".into(),
    }]);
    external::register_expression_calls(answers.clone());
    let tool = Registry::current().get("attributes.calculate").unwrap();
    let values = json!({
        "input": { "scope": "layer", "layerId": "parsel" },
        "field": "etiket",
        "value": "ext.tr.ornek.deneme.ada-parsel(Ada, Parsel)",
        "empty": "keep",
        "label": false,
    })
    .as_object()
    .unwrap()
    .clone();
    // The window checks the value without the extension's code: the name is known.
    let doc = drawing(false);
    let issues = kentos_processing::parameters::validate_values(&tool, &values, doc.layers());
    assert!(issues.is_empty(), "{issues:?}");
    let mut host = Bare(drawing(false), Vec::new());
    let outcome = Runner::new().run(&mut host, &tool, &values, false, &mut Vec::new());
    assert!(matches!(outcome, Outcome::Ok { .. }), "{outcome:?}");
    let labels: Vec<String> = host
        .0
        .entities()
        .filter(|e| e.base().layer_id == "parsel")
        .map(|e| e.base().attrs.get("etiket").cloned().unwrap_or_default())
        .collect();
    assert_eq!(labels, ["101/1", "101/2", "101/3"]);
    assert_eq!(
        *answers.batches.lock().unwrap(),
        [3],
        "one call for the three parcels"
    );
    // A function no extension offers is refused before the run.
    let mut wrong = values.clone();
    wrong.insert("value".into(), json!("ext.tr.ornek.yok.islev(Ada)"));
    let issues = kentos_processing::parameters::validate_values(&tool, &wrong, doc.layers());
    assert!(
        issues
            .iter()
            .any(|i| i.message.contains("ext.tr.ornek.yok.islev")),
        "{issues:?}"
    );
    external::set_expressions(Vec::new());
}

#[test]
fn a_models_step_pins_the_extension_its_tool_came_from() {
    // docs/adr/8057 §13, PY-TOOL-20: a step added for an extension's tool records the extension, its version and its
    // package digest; the JSON keeps it; a model opened without the extension says which one it was made with, and
    // nothing is installed for it.
    use kentos_processing::model::{Model, check_model};
    use kentos_processing::model_edit::{add_step, new_model};
    setup();
    let registry = Registry::current();
    let lookup = |id: &str| registry.tool(id);
    let mut model = new_model("eklentili".into());
    let step = add_step(&mut model, "ext.tr.ornek.deneme.olc", &lookup, None, None);
    let plain = add_step(&mut model, "attributes.calculate", &lookup, None, None);
    let pinned = |id: &str| model.steps.iter().find(|s| s.id == id).unwrap().pin.clone();
    assert_eq!(
        pinned(&step),
        Some(kentos_commands::ExtensionPin {
            extension: "tr.ornek.deneme".into(),
            version: "1.0.0".into(),
            digest: DIGEST.into(),
        })
    );
    assert_eq!(pinned(&plain), None, "a built-in tool has no pin");
    let text = serde_json::to_string(&model).unwrap();
    let back = Model::from_text(&text).expect("it reads back");
    assert_eq!(back.steps[0].pin, pinned(&step));
    assert_eq!(model.to_json()["steps"][0]["pin"]["version"], "1.0.0");
    assert!(model.to_json()["steps"][1].get("pin").is_none());
    // Opened where the extension is not: the step's problem names the extension and version.
    let none = |id: &str| {
        (!id.starts_with("ext."))
            .then(|| registry.tool(id))
            .flatten()
    };
    let issues = check_model(&back, &none);
    assert!(
        issues
            .iter()
            .any(|i| i.step.as_deref() == Some(step.as_str())
                && i.message
                    .contains("model tr.ornek.deneme 1.0.0 ile yapıldı")),
        "{issues:?}"
    );
}

#[test]
fn the_tool_list_gives_an_ai_model_the_descriptors_words_and_the_extension() {
    // docs/adr/8057 §14 (plan D18): `cad.processing.tools` lists an extension's tool with its descriptor's
    // `modelDescription` and its extension as installed, read without the extension's code; a built-in tool has
    // neither. A description too long is refused with the descriptor.
    setup();
    let hosted = kentos_native_application::processing_host::tools().expect("registered");
    let listed = hosted
        .list(
            &drawing(false),
            &kentos_commands::ProcessingTools::default(),
        )
        .expect("listed");
    let ours = listed
        .tools
        .iter()
        .find(|t| t.id == "ext.tr.ornek.deneme.olc")
        .expect("the extension's tool");
    assert_eq!(
        ours.model_description.as_deref(),
        Some(
            "Use it to get each parcel's area in square metres; parcels over the limit are selected."
        )
    );
    let pin = ours.extension.as_ref().expect("its extension");
    assert_eq!(
        (
            pin.extension.as_str(),
            pin.version.as_str(),
            pin.digest.as_str()
        ),
        ("tr.ornek.deneme", "1.0.0", DIGEST)
    );
    let json = serde_json::to_value(ours).unwrap();
    assert_eq!(
        json["modelDescription"],
        ours.model_description.clone().unwrap().as_str()
    );
    let builtin = listed
        .tools
        .iter()
        .find(|t| t.id == "attributes.calculate")
        .expect("a built-in tool");
    assert!(builtin.model_description.is_none() && builtin.extension.is_none());
    assert!(
        serde_json::to_value(builtin)
            .unwrap()
            .get("modelDescription")
            .is_none()
    );
    let mut long = descriptor();
    long["modelDescription"] = json!("a".repeat(external::MOST_MODEL_DESCRIPTION + 1));
    let Err(refused) =
        external::from_descriptor("tr.ornek.deneme", "Deneme", "1.0.0", "olc", &long)
    else {
        panic!("a description too long is refused");
    };
    assert!(refused.contains("modelDescription en çok"), "{refused}");
}

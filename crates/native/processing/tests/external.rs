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
fn setup() -> ExternalTool {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let tool =
            external::from_descriptor("tr.ornek.deneme", "Deneme", "1.0.0", "olc", &descriptor())
                .unwrap();
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

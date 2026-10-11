//! The shared processing cases (fixtures/processing/v1, the format in
//! fixtures/processing/README.md): each case runs a built-in tool or model
//! on a drawing through the native runner, and what the run did is compared
//! with what the case says. The web plays the same files
//! (apps/web/src/processing/cases.test.ts): cases.json, queries.json
//! (docs/adr/0200), whose file values name files beside it, read through the
//! formats core as the dialog reads a chosen file, geometry.json
//! (docs/adr/0201), whose new objects are measured (`addedShapes`), and
//! surface.json (docs/adr/0231), interpolation.json (docs/adr/0232) and
//! raster-ops.json (docs/adr/0233; cases/surface.rs), the raster tools run
//! with files that read the cases' rasters.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use kentos_contracts::{DocumentSnapshotV1, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geometry::Bounds;
use kentos_processing::files::Files;
use kentos_processing::model_runner::{model_as_tool, record_model, replay_model, run_model};
use kentos_processing::parameters::default_values;
use kentos_processing::types::ParamKind;
use kentos_processing::{
    Defaults, Feedback, Host, Level, LogLine, Outcome, Prepared, Registry, Runner, Scene, Tool,
    Values,
};
use serde_json::{Map, Value, json};

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/processing/v1")
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(folder().join(name))
        .unwrap_or_else(|e| panic!("fixtures/processing/v1/{name}: {e}"))
}

fn load(name: &str) -> Document {
    let snapshot = DocumentSnapshotV1::from_json(&fixture(name)).expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("the drawing opens")
}

struct TestHost {
    doc: Document,
    selection: Vec<Slot>,
    view: Option<Bounds>,
}

impl Scene for TestHost {
    fn doc(&self) -> &Document {
        &self.doc
    }

    fn selected(&self) -> Vec<Slot> {
        self.selection.clone()
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        self.view
    }
}

impl Host for TestHost {
    fn doc_mut(&mut self) -> &mut Document {
        &mut self.doc
    }

    fn select(&mut self, ids: &[Slot]) {
        self.selection = ids.to_vec();
    }
}

/// An object as the cases write it: every field but the id.
fn plain(e: &Entity) -> Value {
    let mut v = serde_json::to_value(e).expect("an object writes");
    if let Some(o) = v.as_object_mut() {
        o.remove("id");
    }
    v
}

/// An object's geometry as the cases write it: without its kind and what is
/// not geometry (Veri aktar's Geometriyi güncelle, docs/adr/0216 §4).
fn geometry_part(e: &Value) -> Value {
    let mut g = e.clone();
    if let Some(o) = g.as_object_mut() {
        for k in [
            "kind",
            "layerId",
            "attrs",
            "color",
            "label",
            "symbol",
            "lineWeight",
            "labelPins",
        ] {
            o.remove(k);
        }
    }
    g
}

/// The drawing's objects (by id) and layer tree, to compare before and after.
#[derive(Debug, PartialEq)]
struct State {
    objects: BTreeMap<u32, Value>,
    layers: Value,
}

fn state(doc: &Document) -> State {
    State {
        objects: doc.entities().map(|e| (e.base().id, plain(e))).collect(),
        layers: serde_json::to_value(doc.layers().nodes()).expect("the tree writes"),
    }
}

/// JSON equality with numbers compared as numbers (1000 and 1000.0 are one).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// Whether two values are equal, numbers within `tol`.
fn close(a: &Value, b: &Value, tol: f64) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) => (x - y).abs() <= tol,
            _ => false,
        },
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| close(a, b, tol))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| close(v, w, tol)))
        }
        _ => a == b,
    }
}

/// Numbers other than coordinates are exact: only geometry fields (and a
/// file's `toleranceFields`, `more`) get the tolerance.
fn same_object(have: &Value, want: &Value, tol: f64, more: &[String]) -> bool {
    const GEOMETRY: [&str; 9] = ["p", "a", "b", "c", "pts", "holes", "r", "major", "ratio"];
    let (Some(h), Some(w)) = (have.as_object(), want.as_object()) else {
        return false;
    };
    let keys = |o: &Map<String, Value>| o.keys().cloned().collect::<BTreeSet<_>>();
    keys(h) == keys(w)
        && w.iter().all(|(k, v)| {
            let got = &h[k];
            if GEOMETRY.contains(&k.as_str()) || more.contains(k) {
                close(got, v, tol)
            } else {
                same(got, v)
            }
        })
}

/// The fields a geometry tool's new object may have: its kind, layer and
/// attributes, and its geometry's.
const SHAPE_FIELDS: [&str; 12] = [
    "kind", "layerId", "attrs", "pts", "bulges", "holes", "parts", "p", "a", "b", "c", "r",
];

/// An object's vertices in order: an area's ring then its holes, a path's, a point's and its points'.
fn vertices_of(e: &Value) -> Value {
    let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
    if e["kind"] == "point" {
        let mut out = vec![e["p"].clone()];
        out.extend(list(&e["parts"]).iter().map(|q| q["p"].clone()));
        return Value::Array(out);
    }
    let mut out = list(&e["pts"]);
    for h in list(&e["holes"]) {
        out.extend(list(&h["pts"]));
    }
    Value::Array(out)
}

/// A geometry tool's new object against the case (`addedShapes`, docs/adr/0201):
/// its kind, layer and attributes exactly, no other fields, and its measures
/// (the core's `measured`: parts, holes, area, length, points) and vertices
/// within the tolerances; why not, when it is not.
fn same_shape(have: &Value, want: &Value, tol: f64, measure_tol: f64) -> Option<String> {
    use kentos_geometry_core::api::json::{FromJson, Json};
    use kentos_geometry_core::ops::geoprocess::calls::measured;
    if let Some(extra) = have
        .as_object()
        .and_then(|o| o.keys().find(|k| !SHAPE_FIELDS.contains(&k.as_str())))
    {
        return Some(format!("fazla alan “{extra}”"));
    }
    for key in ["kind", "layerId", "attrs"] {
        if !same(&have[key], &want[key]) {
            return Some(key.to_owned());
        }
    }
    let json = Json::parse(&have.to_string()).ok()?;
    let e = kentos_geometry_core::entity::Entity::from_json(&json).ok()?;
    let m = measured(&e.shape);
    if json!(m.parts) != want["parts"] || json!(m.holes) != want["holes"] {
        return Some(format!("parça {}, delik {}", m.parts, m.holes));
    }
    for (key, got) in [("area", m.area), ("length", m.length)] {
        if let Some(w) = want[key].as_f64()
            && got.is_none_or(|g| (g - w).abs() > measure_tol)
        {
            return Some(format!("{key} {got:?}"));
        }
    }
    if !want["points"].is_null() {
        let points: Vec<Value> = m
            .points
            .unwrap_or_default()
            .iter()
            .map(|p| json!({ "x": p.x, "y": p.y }))
            .collect();
        if !close(&Value::Array(points), &want["points"], tol) {
            return Some("noktalar".to_owned());
        }
    }
    if !want["vertices"].is_null() && !close(&vertices_of(have), &want["vertices"], tol) {
        return Some("köşeler".to_owned());
    }
    None
}

struct Seen {
    outcome: Outcome,
    log: Vec<LogLine>,
    host: TestHost,
    before: State,
}

fn slots(v: Option<&Value>) -> Vec<Slot> {
    v.and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_u64)
                .map(|id| Slot(id as u32))
                .collect()
        })
        .unwrap_or_default()
}

fn with_values(tool: &Tool, doc: &Document, over: Option<&Value>, files: &Value) -> Values {
    let mut values = default_values(tool, &Defaults::of(doc));
    if let Some(Value::Object(over)) = over {
        for (k, v) in over {
            values.insert(k.clone(), v.clone());
        }
    }
    // A file parameter's value names a file beside the cases: the dialog's value, its name and first sheet's rows.
    for p in &tool.parameters {
        if let (ParamKind::File { .. }, Some(Value::String(name))) = (&p.kind, values.get(&p.name))
        {
            let path = files[name.as_str()]
                .as_str()
                .unwrap_or_else(|| panic!("dosya yok: {name}"));
            let bytes = std::fs::read(folder().join(path)).expect("the file reads");
            // As the dialog reads a chosen file: İşlemler's limits (docs/adr/0223 §1).
            let read = kentos_formats::table_file::read_with(
                &bytes,
                kentos_formats::table_file::Limits::PROCESSING,
            );
            let rows = read
                .sheets
                .first()
                .map(|s| s.rows.clone())
                .unwrap_or_default();
            let value = json!({ "name": name, "rows": rows });
            values.insert(p.name.clone(), value);
        }
    }
    values
}

/// A tool's messages when it is computed apart from its host (the desktop's background runs), and the host's files.
struct Apart<'a> {
    log: &'a mut Vec<LogLine>,
    files: Option<Arc<dyn Files>>,
}

impl Feedback for Apart<'_> {
    fn files(&self) -> Option<Arc<dyn Files>> {
        self.files.clone()
    }

    fn progress(&mut self, _fraction: f64, _label: &str) {}

    fn info(&mut self, message: String) {
        self.log.push(LogLine {
            level: Level::Info,
            text: message,
        });
    }

    fn warn(&mut self, message: String) {
        self.log.push(LogLine::warn(message));
    }

    fn canceled(&self) -> bool {
        false
    }
}

/// Plays a case. `on_copy`: as the desktop runs apart from the drawing on
/// another thread, a tool is prepared on the host, computed on the
/// drawing's reading copy and finished on the host; a model runs on the
/// copy, recorded, and is replayed on the host.
fn play(c: &Value, registry: &Registry, on_copy: bool, files: &Value) -> Seen {
    play_with(c, registry, on_copy, files, None)
}

/// [`play`] with the host's files (the raster tools', docs/adr/0231): given to the run computed apart.
fn play_with(
    c: &Value,
    registry: &Registry,
    on_copy: bool,
    files: &Value,
    host_files: Option<Arc<dyn Files>>,
) -> Seen {
    let doc = load(c["document"].as_str().expect("a document"));
    let view = c.get("view").and_then(Value::as_array).map(|v| {
        let n = |i: usize| v[i].as_f64().expect("a number");
        Bounds {
            min_x: n(0),
            min_y: n(1),
            max_x: n(2),
            max_y: n(3),
        }
    });
    let mut host = TestHost {
        doc,
        selection: slots(c.get("selection")),
        view,
    };
    let before = state(&host.doc);
    let mut runner = Runner::new();
    let mut log = Vec::new();
    let lookup = |id: &str| registry.tool(id);
    let outcome = if let Some(id) = c["run"].get("tool").and_then(Value::as_str) {
        let tool = registry
            .tool(id)
            .unwrap_or_else(|| panic!("araç yok: {id}"));
        let values = with_values(&tool, &host.doc, c.get("values"), files);
        if on_copy {
            match runner.prepare(&host, &tool, &values, false, &mut log) {
                Prepared::Ready(job) => {
                    let copy = host.doc.reading_copy();
                    let result = Runner::compute(
                        &job,
                        &copy,
                        &mut Apart {
                            log: &mut log,
                            files: host_files.clone(),
                        },
                    );
                    runner.finish(&mut host, job, result, false, &mut log)
                }
                Prepared::Done(outcome) => outcome,
            }
        } else {
            runner.run(&mut host, &tool, &values, false, &mut log)
        }
    } else {
        let id = c["run"]["model"].as_str().expect("a tool or a model");
        let model = registry
            .model(id)
            .unwrap_or_else(|| panic!("model yok: {id}"));
        let as_tool = model_as_tool(model, &lookup);
        let values = with_values(&as_tool, &host.doc, c.get("values"), files);
        if on_copy {
            let mut copy = TestHost {
                doc: host.doc.reading_copy(),
                selection: host.selection.clone(),
                view: host.view,
            };
            let mut unheard = Vec::new();
            let steps = record_model(
                model,
                &values,
                &mut copy,
                &lookup,
                &mut Apart {
                    log: &mut unheard,
                    files: None,
                },
                &mut log,
            );
            assert!(unheard.is_empty(), "the steps' messages are the log's");
            // The copy's messages are the run's; the replay's repeat them.
            let mut replayed = Vec::new();
            replay_model(
                model,
                &values,
                &mut runner,
                &mut host,
                &lookup,
                steps,
                &mut replayed,
            )
        } else {
            run_model(model, &values, &mut runner, &mut host, &lookup, &mut log)
        }
    };
    Seen {
        outcome,
        log,
        host,
        before,
    }
}

/// What a run did, in the cases' terms.
fn observed(s: &Seen) -> Value {
    match &s.outcome {
        Outcome::Invalid { issues } => json!({
            "status": "invalid",
            "issues": issues
                .iter()
                .map(|i| match &i.param {
                    Some(p) => json!({ "param": p, "message": i.message }),
                    None => json!({ "message": i.message }),
                })
                .collect::<Vec<_>>(),
        }),
        Outcome::Stopped {
            status, message, ..
        } => json!({ "status": format!("{status:?}").to_lowercase(), "message": message }),
        Outcome::Ok { result, record, .. } => {
            let doc = &s.host.doc;
            let mut known = BTreeSet::new();
            fn walk(nodes: &Value, known: &mut BTreeSet<String>) {
                for n in nodes.as_array().into_iter().flatten() {
                    if let Some(id) = n["id"].as_str() {
                        known.insert(id.to_owned());
                    }
                    walk(&n["children"], known);
                }
            }
            walk(&s.before.layers, &mut known);
            // The groups a run made, and its new layers with their fields and, in a group it made, that
            // group (docs/adr/0217 §7).
            let mut groups = Vec::new();
            fn walk_now(
                nodes: &[kentos_contracts::LayerNode],
                known: &BTreeSet<String>,
                groups: &mut Vec<Value>,
            ) {
                for n in nodes {
                    if n.kind == kentos_contracts::LayerNodeType::Group && !known.contains(&n.id) {
                        groups.push(json!({ "id": n.id, "name": n.name }));
                    }
                    walk_now(&n.children, known, groups);
                }
            }
            walk_now(doc.layers().nodes(), &known, &mut groups);
            let made: BTreeSet<&str> = groups.iter().filter_map(|g| g["id"].as_str()).collect();
            let layers: Vec<Value> = doc
                .layers()
                .leaves()
                .into_iter()
                .filter(|l| !known.contains(&l.id))
                .map(|l| {
                    let mut v = json!({ "id": l.id, "name": l.name, "style": l.style });
                    if !l.fields.is_empty() {
                        v["fields"] = json!(l.fields);
                    }
                    if let Some(parent) = doc
                        .layers()
                        .parent(&l.id)
                        .filter(|p| made.contains(p.id.as_str()))
                    {
                        v["parent"] = json!(parent.id);
                    }
                    v
                })
                .collect();
            let mut added = Vec::new();
            let mut updated = Vec::new();
            let now = state(doc);
            for (id, e) in &now.objects {
                match s.before.objects.get(id) {
                    None => added.push(e.clone()),
                    Some(old) if old != e => {
                        let mut u = json!({ "id": id, "attrs": e["attrs"] });
                        if let Some(label) = e.get("label") {
                            u["label"] = label.clone();
                        }
                        // A changed geometry is written too (Veri aktar's Geometriyi güncelle).
                        let now = geometry_part(e);
                        if geometry_part(old) != now {
                            u["geometry"] = now;
                        }
                        // A layer changed too (Özniteliğe göre böl's and Seçilenleri çıkar's Taşı, docs/adr/0217).
                        if old["layerId"] != e["layerId"] {
                            u["layerId"] = e["layerId"].clone();
                        }
                        updated.push(u);
                    }
                    Some(_) => {}
                }
            }
            let removed: Vec<u32> = s
                .before
                .objects
                .keys()
                .filter(|id| !now.objects.contains_key(id))
                .copied()
                .collect();
            let log: Vec<Value> = s
                .log
                .iter()
                .map(|l| {
                    let level = match l.level {
                        Level::Info => "info",
                        Level::Warn => "warn",
                    };
                    json!({ "level": level, "text": l.text })
                })
                .collect();
            json!({
                "status": "ok",
                "summary": record.summary,
                "log": log,
                "layers": layers,
                "groups": groups,
                "added": added,
                "updated": updated,
                "removed": removed,
                "selection": s.host.selection.iter().map(|s| s.0).collect::<Vec<_>>(),
                "outputs": result.outputs,
            })
        }
    }
}

fn check(c: &Value, s: Seen, tol: f64) -> Vec<String> {
    check_measured(c, s, tol, tol)
}

fn check_measured(c: &Value, s: Seen, tol: f64, measure_tol: f64) -> Vec<String> {
    check_full(c, s, tol, measure_tol, &[])
}

fn check_full(c: &Value, mut s: Seen, tol: f64, measure_tol: f64, more: &[String]) -> Vec<String> {
    let id = c["id"].as_str().unwrap_or("?");
    let got = observed(&s);
    let want = &c["expect"];
    let mut problems = Vec::new();
    let mut expect = |ok: bool, what: &str, got: &Value, want: &Value| {
        if !ok {
            problems.push(format!(
                "{id}: {what}\n    beklenen: {want}\n    gelen:    {got}"
            ));
        }
    };
    expect(
        got["status"] == want["status"],
        "durum",
        &got["status"],
        &want["status"],
    );
    if got["status"] != want["status"] {
        return problems;
    }
    match want["status"].as_str() {
        Some("invalid") => {
            expect(
                same(&got["issues"], &want["issues"]),
                "sorunlar",
                &got["issues"],
                &want["issues"],
            );
            return problems;
        }
        Some("ok") => {}
        _ => {
            expect(
                got["message"] == want["message"],
                "ileti",
                &got["message"],
                &want["message"],
            );
            return problems;
        }
    }
    let or_empty = |v: &Value| if v.is_null() { json!([]) } else { v.clone() };
    expect(
        got["summary"] == want["summary"],
        "özet",
        &got["summary"],
        &want["summary"],
    );
    expect(
        same(&got["log"], &or_empty(&want["log"])),
        "iletiler",
        &got["log"],
        &want["log"],
    );
    expect(
        same(&got["layers"], &or_empty(&want["layers"])),
        "katmanlar",
        &got["layers"],
        &want["layers"],
    );
    expect(
        same(&got["groups"], &or_empty(&want["groups"])),
        "gruplar",
        &got["groups"],
        &want["groups"],
    );
    let shapes = !want["addedShapes"].is_null();
    let (have, wanted) = (
        or_empty(&got["added"]),
        or_empty(&want[if shapes { "addedShapes" } else { "added" }]),
    );
    let (have, wanted) = (
        have.as_array().cloned().unwrap_or_default(),
        wanted.as_array().cloned().unwrap_or_default(),
    );
    if have.len() != wanted.len() {
        expect(
            false,
            "eklenen sayısı",
            &json!(have.len()),
            &json!(wanted.len()),
        );
    } else {
        for (i, (h, w)) in have.iter().zip(&wanted).enumerate() {
            if shapes {
                let why = same_shape(h, w, tol, measure_tol);
                let what = format!("addedShapes[{i}]: {}", why.as_deref().unwrap_or(""));
                expect(why.is_none(), &what, h, w);
            } else {
                expect(same_object(h, w, tol, more), &format!("added[{i}]"), h, w);
            }
        }
    }
    // A changed geometry is compared when the case writes one for an update (Veri aktar's Geometriyi
    // güncelle); the elevations a run writes are compared below.
    let wanted_updates = or_empty(&want["updated"]);
    let shapes = wanted_updates
        .as_array()
        .is_some_and(|u| u.iter().any(|u| u.get("geometry").is_some()));
    let mut got_updates = got["updated"].clone();
    if !shapes {
        for u in got_updates.as_array_mut().into_iter().flatten() {
            if let Some(o) = u.as_object_mut() {
                o.remove("geometry");
            }
        }
    }
    expect(
        same(&got_updates, &wanted_updates),
        "değişen",
        &got_updates,
        &want["updated"],
    );
    // The vertices' elevations of the objects a run changed (Eğrilere kot ver), each path's in `elevation::paths`' order.
    for (key, zs) in want["elevations"].as_object().cloned().unwrap_or_default() {
        let got_zs = key
            .parse::<u32>()
            .ok()
            .and_then(|k| s.host.doc.get(Slot(k)))
            .map(|e| {
                kentos_native_application::elevation::paths(e)
                    .iter()
                    .map(|p| p.zs.clone())
                    .collect::<Vec<_>>()
            });
        let want_zs: Option<Vec<Vec<Option<f64>>>> = serde_json::from_value(zs.clone()).ok();
        expect(
            got_zs.is_some() && got_zs == want_zs,
            &format!("kotlar {key}"),
            &json!(got_zs),
            &zs,
        );
    }
    expect(
        same(&got["removed"], &or_empty(&want["removed"])),
        "silinen",
        &got["removed"],
        &want["removed"],
    );
    if !want["selection"].is_null() {
        expect(
            same(&got["selection"], &want["selection"]),
            "seçim",
            &got["selection"],
            &want["selection"],
        );
    }
    if let Some(outputs) = want["outputs"].as_object() {
        for (k, v) in outputs {
            expect(
                same(&got["outputs"][k], v),
                &format!("outputs.{k}"),
                &got["outputs"][k],
                v,
            );
        }
    }
    // One undo step takes the whole run back, and redo brings it again; a
    // run that edits nothing leaves no step.
    let doc = &mut s.host.doc;
    if want["undo"].is_null() {
        expect(
            !doc.can_undo(),
            "geri alınacak adım olmamalı",
            &json!(doc.can_undo()),
            &json!(false),
        );
        return problems;
    }
    let after = state(doc);
    let undone = doc.undo();
    expect(
        undone.as_deref() == want["undo"].as_str(),
        "geri alma adımı",
        &json!(undone),
        &want["undo"],
    );
    expect(
        state(doc) == s.before,
        "geri alınınca çizim eski hâline dönmeli",
        &json!(null),
        &json!(null),
    );
    expect(
        !doc.can_undo(),
        "tek adım",
        &json!(doc.can_undo()),
        &json!(false),
    );
    doc.redo();
    expect(
        state(doc) == after,
        "yinelenince sonuç geri gelmeli",
        &json!(null),
        &json!(null),
    );
    problems
}

fn case_file(name: &str) -> Value {
    let file: Value = serde_json::from_str(&fixture(name)).expect("the case file reads");
    assert_eq!(
        (file["format"].as_str(), file["version"].as_u64()),
        (Some("kentos.processing-cases"), Some(1)),
        "{name}: a v1 case file"
    );
    file
}

fn cases() -> Value {
    case_file("cases.json")
}

/// The query tools' cases (docs/adr/0200): the same run here and on the
/// drawing's reading copy as the background runs it.
#[test]
fn the_query_cases_do_what_they_say() {
    let file = case_file("queries.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check(c, play(c, &registry, false, &file["files"]), tol);
        found.extend(check(c, play(c, &registry, true, &file["files"]), tol));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 20, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// The proximity tools' cases (docs/adr/0215): written by the independent
/// reference (scripts/fixtures/proximity_cases.py); the same run here and on
/// the drawing's reading copy as the background runs it.
#[test]
fn the_proximity_cases_do_what_they_say() {
    let file = case_file("proximity.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check(c, play(c, &registry, false, &file["files"]), tol);
        found.extend(check(c, play(c, &registry, true, &file["files"]), tol));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 9, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// The Katman tools' cases (docs/adr/0217): written by the independent
/// reference (scripts/fixtures/layer_ops_cases.py); the same run here and on
/// the drawing's reading copy as the background runs it.
#[test]
fn the_layer_tool_cases_do_what_they_say() {
    let file = case_file("layers.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check(c, play(c, &registry, false, &file["files"]), tol);
        found.extend(check(c, play(c, &registry, true, &file["files"]), tol));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 15, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Geometri dönüştürme's cases (docs/adr/0218): written by the independent
/// reference (scripts/fixtures/convert_cases.py); the same run here and on
/// the drawing's reading copy as the background runs it; computed bulges,
/// elevations, angles and parts within the tolerance (`toleranceFields`).
#[test]
fn the_conversion_cases_do_what_they_say() {
    let file = case_file("convert.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let more: Vec<String> = file["toleranceFields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check_full(
            c,
            play(c, &registry, false, &file["files"]),
            tol,
            tol,
            &more,
        );
        found.extend(check_full(
            c,
            play(c, &registry, true, &file["files"]),
            tol,
            tol,
            &more,
        ));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 23, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Tablodan nokta's cases (docs/adr/0223 §5): written by the independent
/// reference (scripts/fixtures/table_points_cases.py: the tables read by the
/// table reader's rule, the points moved with PROJ); the same run here and on
/// the drawing's reading copy as the background runs it.
#[test]
fn the_table_point_cases_do_what_they_say() {
    let file = case_file("table-points.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let more: Vec<String> = file["toleranceFields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check_full(
            c,
            play(c, &registry, false, &file["files"]),
            tol,
            tol,
            &more,
        );
        found.extend(check_full(
            c,
            play(c, &registry, true, &file["files"]),
            tol,
            tol,
            &more,
        ));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 4, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Doğrusal referans (docs/adr/0224 §5) against the independent
/// reference (scripts/fixtures/lrs_processing_cases.py on lrs_cases.py's
/// rules); the same run here and on the drawing's reading copy as the
/// background runs it.
#[test]
fn the_linear_referencing_cases_do_what_they_say() {
    let file = case_file("lrs.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let more: Vec<String> = file["toleranceFields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check_full(
            c,
            play(c, &registry, false, &file["files"]),
            tol,
            tol,
            &more,
        );
        found.extend(check_full(
            c,
            play(c, &registry, true, &file["files"]),
            tol,
            tol,
            &more,
        ));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 14, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Geometri üretimi's cases (docs/adr/0219): written by the independent
/// reference (scripts/fixtures/generate_cases.py: exact cells and triangles,
/// GEOS beside them); the same run here and on the drawing's reading copy as
/// the background runs it; radii and elevations within the tolerance.
#[test]
fn the_generation_cases_do_what_they_say() {
    let file = case_file("generate.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let more: Vec<String> = file["toleranceFields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check_full(
            c,
            play(c, &registry, false, &file["files"]),
            tol,
            tol,
            &more,
        );
        found.extend(check_full(
            c,
            play(c, &registry, true, &file["files"]),
            tol,
            tol,
            &more,
        ));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 20, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Genelleştirme's cases (docs/adr/0221): written by the independent reference
/// (scripts/fixtures/generalize_cases.py: the ADR's rules in the same order,
/// corners' triangles on fractions, the closings' areas worked by hand beside
/// GEOS's, GEOS's unions for the joinings); the same run here and on the
/// drawing's reading copy; elevations and the closings' areas within the
/// tolerance.
#[test]
fn the_generalization_cases_do_what_they_say() {
    let file = case_file("generalize.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let more: Vec<String> = file["toleranceFields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check_full(
            c,
            play(c, &registry, false, &file["files"]),
            tol,
            tol,
            &more,
        );
        found.extend(check_full(
            c,
            play(c, &registry, true, &file["files"]),
            tol,
            tol,
            &more,
        ));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 22, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Örnekleme's cases (docs/adr/0220): written by the independent reference
/// (scripts/fixtures/sampling_cases.py: the ADR's lattices and seeded
/// numbers, points in areas decided on fractions, GEOS's distance for
/// Nesnelere değenler and its intersection for the pieces); the same run
/// here and on the drawing's reading copy; elevations within the tolerance.
#[test]
fn the_sampling_cases_do_what_they_say() {
    let file = case_file("sampling.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let more: Vec<String> = file["toleranceFields"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check_full(
            c,
            play(c, &registry, false, &file["files"]),
            tol,
            tol,
            &more,
        );
        found.extend(check_full(
            c,
            play(c, &registry, true, &file["files"]),
            tol,
            tol,
            &more,
        ));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 29, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Veri aktar's cases (docs/adr/0216): written by the independent reference
/// (scripts/fixtures/transfer_cases.py); the same run here and on the
/// drawing's reading copy as the background runs it.
#[test]
fn the_transfer_cases_do_what_they_say() {
    let file = case_file("transfer.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check(c, play(c, &registry, false, &file["files"]), tol);
        found.extend(check(c, play(c, &registry, true, &file["files"]), tol));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 7, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// The scene of Veri aktar's timing (docs/adr/0216 §7), the web's too
/// (apps/web/scripts/perf/transfer.test.ts): 10 000 parcels on a layer with
/// typed fields, and a deed list of 10 000 rows, 9 900 of them the parcels'
/// and 100 new ones with their east and north.
pub fn transfer_scene() -> (Document, Values) {
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let mut entities = Vec::with_capacity(10_000);
    for i in 0..10_000usize {
        let (x, y) = (e0 + (i % 100) as f64 * 20.0, n0 + (i / 100) as f64 * 20.0);
        entities.push(format!(
            r#"{{"kind":"polygon","id":{},"layerId":"parsel","attrs":{{"Parsel":"{}","Ada":"101","Mahalle":"Merkez"}},"pts":[{{"x":{x},"y":{y}}},{{"x":{},"y":{y}}},{{"x":{},"y":{}}},{{"x":{x},"y":{}}}]}}"#,
            i + 1,
            i + 1,
            x + 20.0,
            x + 20.0,
            y + 20.0,
            y + 20.0
        ));
    }
    let fields = r#"[{"name":"Parsel","kind":"integer","required":true},{"name":"Ada","kind":"integer","required":true},
        {"name":"Malik","kind":"text","length":40},{"name":"Hisse","kind":"text"},{"name":"Alan","kind":"decimal","scale":2},
        {"name":"Tescil","kind":"date"},{"name":"Durum","kind":"text","default":"Beyanlı"},
        {"name":"Mahalle","kind":"text","required":true,"default":"Merkez"}]"#;
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Tapu","settings":{{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
            "origin":{{"x":{e0},"y":{n0}}},"layers":[{{"id":"parsel","name":"Parsel","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"fields":{fields},"children":[]}}],"activeLayer":"parsel","entities":[{}],"styles":{{"items":[],"categories":[]}}}}"#,
        entities.join(",")
    );
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("the drawing reads"))
            .expect("the drawing opens");
    let mut rows = vec![json!([
        "PARSEL",
        "ADA",
        "MALIK_ADI",
        "HISSE",
        "YUZOLCUM",
        "TESCIL_TARIHI",
        "Y",
        "X"
    ])];
    for i in 1..=10_000usize {
        let key = if i <= 9_900 { i } else { 10_000 + i - 9_900 };
        rows.push(json!([
            key.to_string(),
            "101",
            format!("Malik {i}"),
            "1/2",
            format!("{},25", 400 + i % 100),
            "03.05.2021",
            format!("{}", e0 + 2_100.0 + (i % 10) as f64),
            format!("{}", n0 + (i / 10) as f64)
        ]));
    }
    let values: Values = [
        ("target", json!({ "scope": "layer", "layerId": "parsel" })),
        ("targetKey", json!("Parsel")),
        ("sourceKind", json!("file")),
        ("file", json!({ "name": "tapu.csv", "rows": rows })),
        ("sourceKey", json!("PARSEL")),
        (
            "map",
            json!([
                { "target": "Malik", "source": "MALIK_ADI", "default": "" },
                { "target": "Hisse", "source": "HISSE", "default": "" },
                { "target": "Alan", "source": "YUZOLCUM", "default": "" },
                { "target": "Tescil", "source": "TESCIL_TARIHI", "default": "" },
                { "target": "Ada", "source": "ADA", "default": "" },
            ]),
        ),
        ("existing", json!("overwrite")),
        ("geometry", json!(false)),
        ("append", json!(true)),
        ("east", json!("Y")),
        ("north", json!("X")),
        ("reportOnly", json!(false)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    (doc, values)
}

/// Veri aktar's whole run (docs/adr/0216 §7) on the scene above: the
/// runner's check, the tool and its one undo step, after a warm-up, each run
/// undone before the next; p50 and p95 against the ADR's budget. Run on purpose:
/// `cargo test --release -p kentos-processing --test cases transfer_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timing, run by hand in release"]
fn transfer_timing() {
    use std::time::Instant;
    let (doc, values) = transfer_scene();
    let tool = Registry::builtin()
        .tool("attributes.transfer")
        .expect("Veri aktar");
    let mut host = TestHost {
        doc,
        selection: Vec::new(),
        view: None,
    };
    let mut runner = Runner::new();
    let mut ms = Vec::new();
    let mut summary = String::new();
    for run in 0..12 {
        let mut log = Vec::new();
        let t = Instant::now();
        let outcome = runner.run(&mut host, &tool, &values, true, &mut log);
        let took = t.elapsed().as_secs_f64() * 1e3;
        let Outcome::Ok { record, .. } = outcome else {
            panic!("the run: {log:?}");
        };
        summary = record.summary;
        assert!(host.doc.undo().is_some(), "one step");
        if run >= 2 {
            ms.push(took);
        }
    }
    ms.sort_by(f64::total_cmp);
    let (p50, p95) = (
        ms[ms.len() / 2],
        ms[(ms.len() * 95 / 100).min(ms.len() - 1)],
    );
    println!(
        "Veri aktar, 10 000 parsel ve 10 000 satır (beş alan, 100 ekleme): p50 {p50:.1} ms, p95 {p95:.1} ms, bütçe 120 ms\n  {summary}"
    );
    // Its parts, once: the check and the inputs, the tool, the one undo step.
    let mut log = Vec::new();
    let t = Instant::now();
    let Prepared::Ready(job) = runner.prepare(&host, &tool, &values, true, &mut log) else {
        panic!("prepared: {log:?}");
    };
    let prepared = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let result = Runner::compute(
        &job,
        &host.doc,
        &mut Apart {
            log: &mut log,
            files: None,
        },
    );
    let computed = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    runner.finish(&mut host, job, result, false, &mut log);
    println!(
        "  hazırlık {prepared:.1} ms, araç {computed:.1} ms, uygulama {:.1} ms",
        t.elapsed().as_secs_f64() * 1e3
    );
    assert!(p50 <= 120.0, "p50 {p50:.1} ms > 120 ms");
}

/// Katman işlemleri's timing scene (docs/adr/0217 §9): 100 000 parcels on a
/// 20 m grid in two layers of 50 000, Pa and Pb; Mahalle of 50 values, Ada of
/// 1000, Parsel their own; every hundredth parcel a copy of the one before it
/// (values and shape), so 1000 duplicates, and every Mahalle and Ada pair
/// comes twice in a hundred, so every object's shape is weighed.
pub fn layer_scene() -> Document {
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let mut entities = Vec::with_capacity(100_000);
    for i in 0..100_000usize {
        let k = if i % 100 == 99 { i - 1 } else { i };
        let (x, y) = (e0 + (k % 316) as f64 * 20.0, n0 + (k / 316) as f64 * 20.0);
        let size = 10.0 + (k % 7) as f64;
        entities.push(format!(
            r#"{{"kind":"polygon","id":{},"layerId":"{}","attrs":{{"Parsel":"{}","Ada":"{}","Mahalle":"Mahalle {}"}},"pts":[{{"x":{x},"y":{y}}},{{"x":{},"y":{y}}},{{"x":{},"y":{}}},{{"x":{x},"y":{}}}]}}"#,
            i + 1,
            if i < 50_000 { "pa" } else { "pb" },
            k + 1,
            (k / 100) % 1000,
            k % 50 + 1,
            x + size,
            x + size,
            y + size,
            y + size
        ));
    }
    let layer = |id: &str, name: &str| {
        format!(
            r#"{{"id":"{id}","name":"{name}","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"children":[]}}"#
        )
    };
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Katmanlar","settings":{{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
            "origin":{{"x":{e0},"y":{n0}}},"layers":[{},{}],"activeLayer":"pa","entities":[{}],"styles":{{"items":[],"categories":[]}}}}"#,
        layer("pa", "Pa"),
        layer("pb", "Pb"),
        entities.join(",")
    );
    Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("the drawing reads"))
        .expect("the drawing opens")
}

/// Katman işlemleri's whole runs (docs/adr/0217 §9) on the scene above: the
/// runner's check, the tool and its one undo step, after a warm-up, each run
/// undone before the next; p50 and p95 against the ADR's budgets. Run on purpose:
/// `cargo test --release -p kentos-processing --test cases layer_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn layer_timing() {
    use std::time::Instant;
    let registry = Registry::builtin();
    let all = json!({ "scope": "all" });
    let runs: [(&str, &str, Values, f64); 4] = [
        (
            "layers.splitByAttribute",
            "Özniteliğe göre böl, 100 000 parsel, 50 değer, kopya",
            [
                ("input", all.clone()),
                ("fields", json!("Mahalle")),
                ("mode", json!("copy")),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
            300.0,
        ),
        (
            "layers.deleteDuplicates",
            "Yinelenenleri sil, 100 000 parsel, iki alan ve geometri",
            [
                ("input", all.clone()),
                ("fields", json!("Mahalle, Ada")),
                ("geometry", json!(true)),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
            200.0,
        ),
        (
            "layers.orderBy",
            "İfadeye göre sırala, 100 000 parsel, $alan, kopya",
            [
                ("input", all.clone()),
                ("expression", json!("$alan")),
                ("direction", json!("desc")),
                ("nulls", json!("last")),
                ("orderField", json!("Sıra")),
                ("output", json!({ "newName": "Sıralı" })),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
            300.0,
        ),
        (
            "layers.merge",
            "Vektör katmanlarını birleştir, iki 50 000'lik katman",
            [
                ("layers", json!(["pa", "pb"])),
                ("map", json!([])),
                ("sourceField", json!("Kaynak katman")),
                ("output", json!({ "newName": "Birleşik" })),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
            300.0,
        ),
    ];
    let mut host = TestHost {
        doc: layer_scene(),
        selection: Vec::new(),
        view: None,
    };
    let mut over = Vec::new();
    for (id, title, values, budget) in runs {
        let tool = registry.tool(id).expect("the tool");
        let mut runner = Runner::new();
        let mut ms = Vec::new();
        let mut summary = String::new();
        for run in 0..8 {
            let mut log = Vec::new();
            let t = Instant::now();
            let outcome = runner.run(&mut host, &tool, &values, true, &mut log);
            let took = t.elapsed().as_secs_f64() * 1e3;
            let Outcome::Ok { record, .. } = outcome else {
                panic!("{id}: {outcome:?} {log:?}");
            };
            summary = record.summary;
            assert!(host.doc.undo().is_some(), "{id}: one step");
            if run >= 2 {
                ms.push(took);
            }
        }
        ms.sort_by(f64::total_cmp);
        let (p50, p95) = (
            ms[ms.len() / 2],
            ms[(ms.len() * 95 / 100).min(ms.len() - 1)],
        );
        println!("{title}: p50 {p50:.1} ms, p95 {p95:.1} ms, bütçe {budget} ms\n  {summary}");
        // Its parts, once: the check and the inputs, the tool, the one undo step.
        let mut log = Vec::new();
        let t = Instant::now();
        let Prepared::Ready(job) = runner.prepare(&host, &tool, &values, true, &mut log) else {
            panic!("{id} prepared: {log:?}");
        };
        let prepared = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let result = Runner::compute(
            &job,
            &host.doc,
            &mut Apart {
                log: &mut log,
                files: None,
            },
        );
        let computed = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        runner.finish(&mut host, job, result, false, &mut log);
        println!(
            "  hazırlık {prepared:.1} ms, araç {computed:.1} ms, uygulama {:.1} ms",
            t.elapsed().as_secs_f64() * 1e3
        );
        assert!(host.doc.undo().is_some(), "{id}: one step");
        if p50 > budget {
            over.push(format!("{title}: p50 {p50:.1} ms > {budget} ms"));
        }
    }
    assert!(over.is_empty(), "{over:#?}");
}

/// Geometri dönüştürme's points scene (docs/adr/0218 §13): 100 000 points
/// on a grid in one layer, Hat of 1000 values, Sıra counting down within
/// each hat (so ordering moves every point), each with an elevation.
pub fn route_points_scene() -> Document {
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let mut entities = Vec::with_capacity(100_000);
    for i in 0..100_000usize {
        let (x, y) = (e0 + (i % 400) as f64 * 5.0, n0 + (i / 400) as f64 * 5.0);
        entities.push(format!(
            r#"{{"kind":"point","id":{},"layerId":"nokta","attrs":{{"Hat":"H{}","Sıra":"{}"}},"p":{{"x":{x},"y":{y}}},"z":{}}}"#,
            i + 1,
            i % 1000,
            100 - i / 1000,
            (i % 97) as f64 / 4.0
        ));
    }
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Noktalar","settings":{{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
            "origin":{{"x":{e0},"y":{n0}}},"layers":[{{"id":"nokta","name":"Nokta","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"children":[]}}],"activeLayer":"nokta","entities":[{}],"styles":{{"items":[],"categories":[]}}}}"#,
        entities.join(",")
    );
    Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("the drawing reads"))
        .expect("the drawing opens")
}

/// A table of 100 000 rows of east, north, bearing and length (a decimal
/// comma in every tenth), as a chosen file's value holds it.
pub fn bearing_table() -> Value {
    let mut rows = vec![json!(["Ad", "Doğu", "Kuzey", "Semt", "Uzunluk"])];
    for i in 0..100_000usize {
        let semt = if i % 10 == 0 {
            format!("{},5", i % 400)
        } else {
            format!("{}", (i * 7) % 400)
        };
        rows.push(json!([
            format!("S{i}"),
            format!("{}", 487_000 + (i % 400) * 5),
            format!("{}", 4_420_000 + (i / 400) * 5),
            semt,
            format!("{}", 5 + i % 20)
        ]));
    }
    json!({ "name": "semt.csv", "rows": rows })
}

/// Geometri dönüştürme's whole runs (docs/adr/0218 §13): the runner's check,
/// the tool and its one undo step, after a warm-up, each run undone before
/// the next and on its own (a pause lets the run before free what it let go
/// of off its thread); p50 and p95 against the ADR's budgets. Run on purpose:
/// `cargo test --release -p kentos-processing --test cases convert_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn convert_timing() {
    use std::time::Instant;
    let registry = Registry::builtin();
    let all = json!({ "scope": "all" });
    let values = |pairs: Vec<(&str, Value)>| -> Values {
        pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
    };
    let parcels = || layer_scene();
    let points = || route_points_scene();
    // A run: the tool, its title, its drawing, its values, its budget (ms).
    type Run = (&'static str, &'static str, fn() -> Document, Values, f64);
    let runs: Vec<Run> = vec![
        (
            "convert.toPoints",
            "Nesneden nokta, 100 000 parsel, iç nokta",
            parcels as fn() -> Document,
            values(vec![
                ("input", all.clone()),
                ("location", json!("inside")),
                ("parts", json!(false)),
                ("layer", json!({ "newName": "Merkezler" })),
            ]),
            300.0,
        ),
        (
            "convert.vertices",
            "Köşeleri noktaya çevir, 100 000 parsel, bütün köşeler (400 000 nokta)",
            parcels,
            values(vec![
                ("input", all.clone()),
                ("which", json!("all")),
                ("layer", json!({ "newName": "Köşeler" })),
            ]),
            800.0,
        ),
        (
            "convert.densify",
            "Köşe ekle, 100 000 parsel, 5 m aralık",
            parcels,
            values(vec![
                ("input", all.clone()),
                ("method", json!("interval")),
                ("interval", json!(5)),
                ("count", json!(1)),
                ("layer", json!({ "newName": "Sıklaştırılan" })),
            ]),
            800.0,
        ),
        (
            "convert.pointsToPath",
            "Noktalardan çizgi, 100 000 nokta, 1000 hat, sıralı",
            points,
            values(vec![
                ("input", all.clone()),
                ("order", json!("Sıra")),
                ("group", json!("Hat")),
                ("close", json!(false)),
                ("layer", json!({ "newName": "Yollar" })),
            ]),
            300.0,
        ),
        (
            "convert.bearingLine",
            "Semt ve uzunluktan çizgi, 100 000 satır",
            parcels,
            values(vec![
                ("sourceKind", json!("file")),
                ("file", bearing_table()),
                ("east", json!("Doğu")),
                ("north", json!("Kuzey")),
                ("bearing", json!("Semt")),
                ("length", json!("Uzunluk")),
                ("unit", json!("grad")),
                ("layer", json!({ "newName": "Semt çizgileri" })),
            ]),
            300.0,
        ),
    ];
    let mut over = Vec::new();
    for (id, title, scene, values, budget) in runs {
        let tool = registry.tool(id).expect("the tool");
        let mut host = TestHost {
            doc: scene(),
            selection: Vec::new(),
            view: None,
        };
        let mut runner = Runner::new();
        let mut ms = Vec::new();
        let mut summary = String::new();
        for run in 0..8 {
            let mut log = Vec::new();
            let t = Instant::now();
            let outcome = runner.run(&mut host, &tool, &values, true, &mut log);
            let took = t.elapsed().as_secs_f64() * 1e3;
            let Outcome::Ok { record, .. } = outcome else {
                panic!("{id}: {log:?}");
            };
            summary = record.summary;
            assert!(host.doc.undo().is_some(), "{id}: one step");
            // A run lets a big table's values go off its thread; the next starts when they are gone.
            std::thread::sleep(std::time::Duration::from_millis(400));
            if run >= 2 {
                ms.push(took);
            }
        }
        ms.sort_by(f64::total_cmp);
        let (p50, p95) = (
            ms[ms.len() / 2],
            ms[(ms.len() * 95 / 100).min(ms.len() - 1)],
        );
        println!("{title}: p50 {p50:.1} ms, p95 {p95:.1} ms, bütçe {budget} ms\n  {summary}");
        // Its parts, once: the check and the inputs, the tool, the one undo step.
        let mut log = Vec::new();
        let t = Instant::now();
        let Prepared::Ready(job) = runner.prepare(&host, &tool, &values, true, &mut log) else {
            panic!("{id} prepared: {log:?}");
        };
        let prepared = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let result = Runner::compute(
            &job,
            &host.doc,
            &mut Apart {
                log: &mut log,
                files: None,
            },
        );
        let computed = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        runner.finish(&mut host, job, result, false, &mut log);
        println!(
            "  hazırlık {prepared:.1} ms, araç {computed:.1} ms, uygulama {:.1} ms",
            t.elapsed().as_secs_f64() * 1e3
        );
        if p50 > budget {
            over.push(format!("{title}: p50 {p50:.1} ms > {budget} ms"));
        }
    }
    assert!(over.is_empty(), "{over:#?}");
}

/// A table of 100 000 surveyed points (name, east, north, elevation, code;
/// a decimal comma in every tenth) in the project's plane, or as longitudes
/// and latitudes in WGS 84 (degrees and fractions, every tenth in degrees,
/// minutes and seconds), as a chosen file's value holds it.
pub fn point_table(geographic: bool) -> Value {
    let mut rows = vec![json!(["Ad", "Doğu", "Kuzey", "Kot", "Kod"])];
    for i in 0..100_000usize {
        let (e, n) = if geographic {
            let (lon, lat) = (
                35.95 + (i % 400) as f64 * 0.0002,
                39.85 + (i / 400) as f64 * 0.0002,
            );
            if i % 10 == 0 {
                let dms = |v: f64| {
                    let d = v.trunc();
                    let m = ((v - d) * 60.0).trunc();
                    format!("{d} {m} {:.3}", ((v - d) * 60.0 - m) * 60.0)
                };
                (dms(lon), dms(lat))
            } else {
                (format!("{lon:.7}"), format!("{lat:.7}"))
            }
        } else {
            let (e, n) = (
                487_000.0 + (i % 400) as f64 * 5.0 + 0.125,
                4_420_000.0 + (i / 400) as f64 * 5.0 + 0.5,
            );
            if i % 10 == 0 {
                (
                    format!("{e}").replace('.', ","),
                    format!("{n}").replace('.', ","),
                )
            } else {
                (format!("{e}"), format!("{n}"))
            }
        };
        let code = ["SN", "YL", "BN"][i % 3];
        rows.push(json!([
            format!("N{i}"),
            e,
            n,
            format!("{}", 800 + i % 97),
            code
        ]));
    }
    json!({ "name": if geographic { "wgs84.csv" } else { "duzlem.csv" }, "rows": rows })
}

/// Tablodan nokta's whole runs (docs/adr/0223 §8): 100 000 rows in the
/// project's plane and from WGS 84, the runner's check, the tool and its one
/// undo step, after a warm-up, each run undone before the next; p50 and p95
/// against the ADR's budgets. Run on purpose:
/// `cargo test --release -p kentos-processing --test cases table_points_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn table_points_timing() {
    use std::time::Instant;
    let registry = Registry::builtin();
    let tool = registry.tool("convert.tablePoints").expect("the tool");
    let values = |file: Value, system: &str| -> Values {
        [
            ("sourceKind", json!("file")),
            ("file", file),
            ("east", json!("Doğu")),
            ("north", json!("Kuzey")),
            ("elevation", json!("Kot")),
            ("name", json!("Ad")),
            ("system", json!(system)),
            ("layer", json!({ "newName": "Noktalar" })),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect()
    };
    let runs = [
        (
            "Tablodan nokta, 100 000 satır, projenin düzleminde",
            values(point_table(false), "project"),
            300.0,
        ),
        (
            "Tablodan nokta, 100 000 satır, WGS 84'ten projenin sistemine",
            values(point_table(true), "4326"),
            400.0,
        ),
    ];
    let mut over = Vec::new();
    for (title, values, budget) in runs {
        let mut host = TestHost {
            doc: layer_scene(),
            selection: Vec::new(),
            view: None,
        };
        let mut runner = Runner::new();
        let mut ms = Vec::new();
        let mut summary = String::new();
        for run in 0..8 {
            let mut log = Vec::new();
            let t = Instant::now();
            let outcome = runner.run(&mut host, &tool, &values, true, &mut log);
            let took = t.elapsed().as_secs_f64() * 1e3;
            let Outcome::Ok { record, .. } = outcome else {
                panic!("{title}: {log:?}");
            };
            summary = record.summary;
            assert!(host.doc.undo().is_some(), "{title}: one step");
            // A run lets a big table's values go off its thread; the next starts when they are gone.
            std::thread::sleep(std::time::Duration::from_millis(400));
            if run >= 2 {
                ms.push(took);
            }
        }
        ms.sort_by(f64::total_cmp);
        let (p50, p95) = (
            ms[ms.len() / 2],
            ms[(ms.len() * 95 / 100).min(ms.len() - 1)],
        );
        println!("{title}: p50 {p50:.1} ms, p95 {p95:.1} ms, bütçe {budget} ms\n  {summary}");
        if p50 > budget {
            over.push(format!("{title}: p50 {p50:.1} ms > {budget} ms"));
        }
    }
    assert!(over.is_empty(), "{over:#?}");
}

/// 100 000 survey points scattered over 2 km by 2 km to the centimetre (a
/// 32-bit congruential sequence, the web's `scatter` value for value).
pub fn scatter_scene() -> Document {
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let mut x = 20_261_011u32;
    let mut next = || {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(x % 200_000) / 100.0
    };
    let mut entities = Vec::with_capacity(100_000);
    for i in 0..100_000usize {
        let (dx, dy) = (next(), next());
        entities.push(format!(
            r#"{{"kind":"point","id":{},"layerId":"nokta","attrs":{{"No":"P{}"}},"p":{{"x":{},"y":{}}}}}"#,
            i + 1,
            i + 1,
            e0 + dx,
            n0 + dy
        ));
    }
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Noktalar","settings":{{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
            "origin":{{"x":{e0},"y":{n0}}},"layers":[{{"id":"nokta","name":"Nokta","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"children":[]}}],"activeLayer":"nokta","entities":[{}],"styles":{{"items":[],"categories":[]}}}}"#,
        entities.join(",")
    );
    Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("the drawing reads"))
        .expect("the drawing opens")
}

/// Geometri üretimi's whole runs (docs/adr/0219 §8): 100 000 scattered
/// points' Voronoi cells, Delaunay triangles and concave hull, 100 000
/// parcels' rotated boxes and poles; the runner's check, the tool and its one
/// undo step, after a warm-up, each run undone before the next (a pause lets
/// the run before free what it let go of off its thread); p50 and p95
/// against the ADR's budgets. Run on purpose:
/// `cargo test --release -p kentos-processing --test cases generate_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn generate_timing() {
    use std::time::Instant;
    let registry = Registry::builtin();
    let all = json!({ "scope": "all" });
    let values = |pairs: Vec<(&str, Value)>| -> Values {
        pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
    };
    let parcels = || layer_scene();
    let points = || scatter_scene();
    // A run: the tool, its title, its drawing, its values, its budget (ms).
    type Run = (&'static str, &'static str, fn() -> Document, Values, f64);
    let runs: Vec<Run> = vec![
        (
            "generate.voronoi",
            "Voronoi alanları, 100 000 nokta",
            points as fn() -> Document,
            values(vec![
                ("input", all.clone()),
                ("margin", json!(10)),
                ("layer", json!({ "newName": "Voronoi" })),
            ]),
            500.0,
        ),
        (
            "generate.delaunay",
            "Delaunay üçgenleri, 100 000 nokta (200 000 üçgen)",
            points,
            values(vec![
                ("input", all.clone()),
                ("name", json!("No")),
                ("layer", json!({ "newName": "Üçgenler" })),
            ]),
            800.0,
        ),
        (
            "generate.concaveHull",
            "İçbükey zarf, 100 000 nokta, oran 0,3",
            points,
            values(vec![
                ("input", all.clone()),
                ("ratio", json!(0.3)),
                ("holes", json!(false)),
                ("group", json!("all")),
                ("layer", json!({ "newName": "İçbükey zarf" })),
            ]),
            500.0,
        ),
        (
            "generate.bounding",
            "Sınırlayıcı geometri, 100 000 parsel, döndürülmüş kutu",
            parcels,
            values(vec![
                ("input", all.clone()),
                ("kind", json!("rotated")),
                ("group", json!("each")),
                ("layer", json!({ "newName": "Sınırlayıcı" })),
            ]),
            400.0,
        ),
        (
            "generate.pole",
            "Ulaşılmazlık kutbu, 100 000 parsel, 0,1 m",
            parcels,
            values(vec![
                ("input", all.clone()),
                ("precision", json!(0.1)),
                ("layer", json!({ "newName": "Kutuplar" })),
            ]),
            1000.0,
        ),
    ];
    let mut over = Vec::new();
    for (id, title, scene, values, budget) in runs {
        let tool = registry.tool(id).expect("the tool");
        let mut host = TestHost {
            doc: scene(),
            selection: Vec::new(),
            view: None,
        };
        let mut runner = Runner::new();
        let mut ms = Vec::new();
        let mut summary = String::new();
        for run in 0..8 {
            let mut log = Vec::new();
            let t = Instant::now();
            let outcome = runner.run(&mut host, &tool, &values, true, &mut log);
            let took = t.elapsed().as_secs_f64() * 1e3;
            let Outcome::Ok { record, .. } = outcome else {
                panic!("{id}: {log:?}");
            };
            summary = record.summary;
            assert!(host.doc.undo().is_some(), "{id}: one step");
            std::thread::sleep(std::time::Duration::from_millis(400));
            if run >= 2 {
                ms.push(took);
            }
        }
        ms.sort_by(f64::total_cmp);
        let (p50, p95) = (
            ms[ms.len() / 2],
            ms[(ms.len() * 95 / 100).min(ms.len() - 1)],
        );
        println!("{title}: p50 {p50:.1} ms, p95 {p95:.1} ms, bütçe {budget} ms\n  {summary}");
        // Its parts, once: the check and the inputs, the tool, the one undo step.
        let mut log = Vec::new();
        let t = Instant::now();
        let Prepared::Ready(job) = runner.prepare(&host, &tool, &values, true, &mut log) else {
            panic!("{id} prepared: {log:?}");
        };
        let prepared = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let result = Runner::compute(
            &job,
            &host.doc,
            &mut Apart {
                log: &mut log,
                files: None,
            },
        );
        let computed = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        runner.finish(&mut host, job, result, false, &mut log);
        println!(
            "  hazırlık {prepared:.1} ms, araç {computed:.1} ms, uygulama {:.1} ms",
            t.elapsed().as_secs_f64() * 1e3
        );
        if p50 > budget {
            over.push(format!("{title}: p50 {p50:.1} ms > {budget} ms"));
        }
    }
    assert!(over.is_empty(), "{over:#?}");
}

/// The geometry tools' cases (docs/adr/0201): the same run here and on the
/// drawing's reading copy as the background runs it; new objects measured.
#[test]
fn the_geometry_cases_do_what_they_say() {
    let file = case_file("geometry.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let measure_tol = file["measureTolerance"]
        .as_f64()
        .expect("a measure tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check_measured(c, play(c, &registry, false, &json!({})), tol, measure_tol);
        found.extend(check_measured(
            c,
            play(c, &registry, true, &json!({})),
            tol,
            measure_tol,
        ));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 19, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Mekânsal istatistik's cases (docs/adr/0238): the same run here and on the
/// drawing's reading copy; centres, circles, ellipses and copies.
#[test]
fn the_spatial_stats_cases_do_what_they_say() {
    let file = case_file("spatial-stats.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = Vec::new();
        for on_copy in [false, true] {
            let seen = play(c, &registry, on_copy, &json!({}));
            // Each new layer right above its input's (its group, the place before it).
            for (new, at) in c["expect"]["layerAbove"]
                .as_object()
                .cloned()
                .unwrap_or_default()
            {
                let layers = seen.host.doc.layers();
                let (a, b) = (
                    layers.place_of(&new),
                    layers.place_of(at.as_str().unwrap_or("")),
                );
                if !matches!((&a, &b), (Some((g, i)), Some((h, j))) if g == h && i + 1 == *j) {
                    found.push(format!("{}: “{new}” is at {a:?}, “{at}” at {b:?}", c["id"]));
                }
            }
            found.extend(check(c, seen, tol));
        }
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 18, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Ağ analizi's tools' cases (docs/adr/0209 §6, §7): the same run here and on the drawing's reading copy; new
/// objects measured (the areas within the file's tolerance: shapely's against the core's exact arcs).
#[test]
fn the_network_cases_do_what_they_say() {
    let file = case_file("network.json");
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let measure_tol = file["measureTolerance"]
        .as_f64()
        .expect("a measure tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let mut found = check_measured(c, play(c, &registry, false, &json!({})), tol, measure_tol);
        found.extend(check_measured(
            c,
            play(c, &registry, true, &json!({})),
            tol,
            measure_tol,
        ));
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(report.len() >= 9, "{} cases", report.len());
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

/// Each drawing's defaults and each tool's default values on it, as the web reads them.
#[test]
fn the_defaults_the_tools_take_from_the_drawing() {
    let (
        file,
        geometry,
        network,
        surface,
        interpolation,
        raster_ops,
        raster_vector,
        hydrology,
        distance,
        suitability,
        spatial_stats,
        remote,
        multidim,
    ) = (
        cases(),
        case_file("geometry.json"),
        case_file("network.json"),
        case_file("surface.json"),
        case_file("interpolation.json"),
        case_file("raster-ops.json"),
        case_file("raster-vector.json"),
        case_file("hydrology.json"),
        case_file("distance.json"),
        case_file("suitability.json"),
        case_file("spatial-stats.json"),
        case_file("remote.json"),
        case_file("multidim.json"),
    );
    let registry = Registry::builtin();
    let lookup = |id: &str| registry.tool(id);
    let documents = file["documents"]
        .as_object()
        .expect("documents")
        .iter()
        .chain(geometry["documents"].as_object().expect("documents"))
        .chain(network["documents"].as_object().expect("documents"))
        .chain(surface["documents"].as_object().expect("documents"))
        .chain(interpolation["documents"].as_object().expect("documents"))
        .chain(raster_ops["documents"].as_object().expect("documents"))
        .chain(raster_vector["documents"].as_object().expect("documents"))
        .chain(hydrology["documents"].as_object().expect("documents"))
        .chain(distance["documents"].as_object().expect("documents"))
        .chain(suitability["documents"].as_object().expect("documents"))
        .chain(spatial_stats["documents"].as_object().expect("documents"))
        .chain(remote["documents"].as_object().expect("documents"))
        .chain(multidim["documents"].as_object().expect("documents"));
    for (name, d) in documents {
        let doc = load(name);
        let defaults = Defaults::of(&doc);
        assert!(
            same(&defaults.to_json(), &d["defaults"]),
            "{name}: {} ≠ {}",
            defaults.to_json(),
            d["defaults"]
        );
        for (id, values) in d["tools"].as_object().expect("tools") {
            let tool = registry
                .tool(id)
                .or_else(|| registry.model(id).map(|m| model_as_tool(m, &lookup)))
                .unwrap_or_else(|| panic!("{id} yok"));
            let got = Value::Object(default_values(&tool, &defaults));
            assert!(same(&got, values), "{id}: {got} ≠ {values}");
        }
    }
}

/// The desktop computes a large job on another thread, on the drawing's
/// reading copy, and applies the result on the drawing it shows; a model
/// runs whole on the copy and is replayed on the drawing: every case ends
/// the same that way.
#[test]
fn every_case_does_the_same_computed_on_a_copy_of_the_drawing() {
    fn sent<T: Send>() {}
    sent::<kentos_processing::Job>();
    sent::<Document>();
    sent::<kentos_processing::RunResult>();
    sent::<kentos_processing::Model>();
    sent::<kentos_processing::model_runner::RecordedStep>();
    let file = cases();
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let (mut tools, mut models) = (0, 0);
    for c in file["cases"].as_array().expect("cases") {
        if c["run"].get("tool").is_some() {
            tools += 1;
        } else {
            models += 1;
        }
        problems.extend(check(c, play(c, &registry, true, &file["files"]), tol));
    }
    assert!(
        tools > 10 && models > 0,
        "{tools} tool cases, {models} model cases"
    );
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn every_case_does_what_it_says() {
    let file = cases();
    let tol = file["tolerance"].as_f64().expect("a tolerance");
    let registry = Registry::builtin();
    let mut problems = Vec::new();
    let mut report = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let found = check(c, play(c, &registry, false, &file["files"]), tol);
        report.push(format!(
            "{} {}: {}",
            if found.is_empty() { "✓" } else { "✗" },
            c["id"].as_str().unwrap_or("?"),
            c["title"].as_str().unwrap_or("")
        ));
        problems.extend(found);
    }
    println!("{}", report.join("\n"));
    assert!(
        problems.is_empty(),
        "\n{}\n\n{}",
        report.join("\n"),
        problems.join("\n")
    );
}

// Yüzey analizi's cases (docs/adr/0231): the raster tools over the host's files.
#[path = "cases/surface.rs"]
mod surface;

/// A network built from a drawing of a synthetic grid city (225 × 225 crossings 50 m apart, every block its own
/// polyline with its speed and a tenth one way): what reading the drawing costs (the definition's layers and
/// expressions; the desktop does this part on its interface's thread, docs/adr/0209 §12) and the whole build as the
/// İşlemler tools and Python do it. Release, by hand:
/// `cargo test --release -p kentos-processing --test cases network_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn network_timing() {
    use kentos_processing::network::{Source, from_document, network_input};
    use std::time::Instant;
    const N: usize = 225;
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let mut entities = Vec::new();
    let mut street = |a: (f64, f64), b: (f64, f64), k: usize| {
        let one_way = if k.is_multiple_of(20) {
            r#","yon":"FT""#
        } else if k % 20 == 1 {
            r#","yon":"TF""#
        } else {
            ""
        };
        entities.push(format!(
            r#"{{"kind":"polyline","id":{},"layerId":"yol","attrs":{{"hiz":"{}"{one_way}}},"pts":[{{"x":{},"y":{}}},{{"x":{},"y":{}}}]}}"#,
            entities.len() + 1,
            [30, 50, 70][k % 3],
            a.0,
            a.1,
            b.0,
            b.1
        ));
    };
    let at = |i: usize, j: usize| (e0 + i as f64 * 50.0, n0 + j as f64 * 50.0);
    let mut k = 0;
    for i in 0..N {
        for j in 0..N {
            if i + 1 < N {
                street(at(i, j), at(i + 1, j), k);
                k += 1;
            }
            if j + 1 < N {
                street(at(i, j), at(i, j + 1), k);
                k += 1;
            }
        }
    }
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Izgara","settings":{{"srid":5254,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow",
            "networks":[{{"id":"yollar","name":"Yollar","kind":"road","edges":[{{"layer":"yol"}}],"connect":"ends","tolerance":0.01,
            "direction":{{"kind":"field","field":"yon","forward":["FT"],"backward":["TF"],"closed":["N"]}},
            "costs":[{{"name":"Süre","kind":"speed","field":"hiz","speed":50}}],"closed":"durum = 'kapalı'"}}]}},
            "origin":{{"x":{e0},"y":{n0}}},"layers":[{{"id":"yol","name":"Yol","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"children":[]}}],"activeLayer":"yol","entities":[{}],"styles":{{"items":[],"categories":[]}}}}"#,
        entities.join(",")
    );
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("the drawing reads"))
            .expect("the drawing opens");
    let def = doc.settings().networks[0].clone();
    let by = |l: &str| doc.by_layer(l).collect::<Vec<_>>();
    let name = |l: &str| l.to_owned();
    let measures = |_: &[&Entity]| Vec::new();
    let t = Instant::now();
    let input = network_input(
        &def,
        &Source {
            by_layer: &by,
            layer_name: &name,
            measures: &measures,
        },
    );
    println!(
        "çizimi okuma ({} kenar, ifadeler ve alanlar)   {:.1} ms (masaüstünde arayüzün iş parçacığında)",
        input.edges.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    let mut store = kentos_geometry_core::store::Store::new();
    store.put_many(doc.by_layer("yol").map(kentos_processing::geometry::record));
    let t = Instant::now();
    let (edges, _, _) = kentos_geometry_core::ops::network::input::from_store(
        &store,
        &input.edges,
        &input.junctions,
    );
    println!(
        "kenarların yolları depodan ({})   {:.1} ms (masaüstünde arayüzün iş parçacığında)",
        edges.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    let t = Instant::now();
    let built = from_document(&doc, "yollar").expect("the network builds");
    println!(
        "çizimden bütün kurma (depo, okuma, graf; {} parça)   {:.1} ms (İşlemler ve Python)",
        built.session.graph.pieces.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
}

/// A layer's filter leaves its objects out of the calls to other layers
/// (docs/adr/0214 §3, docs/adr/0211 §1) as it does of the inputs: the trees
/// inside each parcel of the query drawing, all of them and then the pines only.
#[test]
fn a_layer_filter_leaves_its_objects_out_of_the_calls_to_other_layers() {
    use kentos_expression::rows::As;
    use kentos_expression::world::{World, WorldLayer};
    use kentos_expression::{Schema, Value as V};
    use kentos_processing::expression::{EntityObjects, Evaluation, evaluate_in};
    use kentos_processing::runner::world_layers;

    let counts = |doc: &Document| -> Vec<V<'static>> {
        let schema = Schema {
            world: true,
            ..Schema::default()
        };
        let expr = kentos_expression::compile_with("kesişen_sayısı('Ağaç')", &schema)
            .expect("the expression compiles");
        let parcels: Vec<&Entity> = doc.by_layer("parsel").collect();
        let layer_name = |id: &str| {
            doc.layers()
                .get(id)
                .map_or_else(|| id.to_owned(), |l| l.name.clone())
        };
        let store = kentos_processing::geometry::RunGeometry::of(doc.entities());
        let looked_at = world_layers([&expr], &parcels, doc);
        let objects: Vec<(String, EntityObjects)> = looked_at
            .iter()
            .map(|(name, l)| {
                (
                    name.clone(),
                    EntityObjects::new(l, &layer_name, Some(store.store())),
                )
            })
            .collect();
        let world = World {
            layers: objects
                .iter()
                .map(|(name, o)| WorldLayer {
                    name: name.clone(),
                    ids: o.ids(),
                    objects: o,
                })
                .collect(),
            store: Some(store.store()),
        };
        let how = Evaluation {
            layer_name: &layer_name,
            store: Some(store.store()),
            world: Some(&world),
        };
        evaluate_in(&expr, &parcels, &how, As::Value)
    };
    let mut doc = load("queries.kcad");
    let all = counts(&doc);
    doc.set_layer_filter(
        "agac",
        Some(kentos_contracts::LayerFilter {
            expression: Some("Tür = 'Çam'".into()),
            objects: Vec::new(),
        }),
        "Süzgeç",
    )
    .expect("the filter is set");
    let pines = counts(&doc);
    let n = |v: &[V<'static>]| -> Vec<f64> {
        v.iter()
            .map(|x| match x {
                V::Num(n) => *n,
                _ => f64::NAN,
            })
            .collect()
    };
    // As queries.json's `expr-tree-count` says: the tree on the boundary in two parcels, the one in the hole in none.
    assert_eq!(n(&all), [2.0, 2.0, 1.0, 1.0]);
    // The pines: 10 in the first parcel, 12 in the second, 14 in the fourth's hole.
    assert_eq!(n(&pines), [1.0, 1.0, 0.0, 0.0]);
}

/// Ten star-shaped areas of 10 000 vertices each (100 000 in all), for Alt
/// bölümlere ayır's timing (docs/adr/0220 §7); the web's `sampling.test.ts`
/// builds the same.
pub fn stars_scene() -> Document {
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let mut entities = Vec::with_capacity(10);
    for s in 0..10usize {
        let (cx, cy) = (e0 + 1000.0 * s as f64, n0);
        let pts: Vec<String> = (0..10_000usize)
            .map(|i| {
                let t = std::f64::consts::PI * i as f64 / 5000.0;
                let r = if i % 2 == 0 {
                    400.0
                } else {
                    300.0 + (i % 7) as f64 * 10.0
                };
                format!(r#"{{"x":{},"y":{}}}"#, cx + r * t.cos(), cy + r * t.sin())
            })
            .collect();
        entities.push(format!(
            r#"{{"kind":"polygon","id":{},"layerId":"y","attrs":{{"Ad":"Yıldız {}"}},"pts":[{}]}}"#,
            s + 1,
            s + 1,
            pts.join(",")
        ));
    }
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"Yıldızlar","settings":{{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
            "origin":{{"x":{e0},"y":{n0}}},"layers":[{{"id":"y","name":"Yıldız","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"children":[]}}],"activeLayer":"y","entities":[{}],"styles":{{"items":[],"categories":[]}}}}"#,
        entities.join(",")
    );
    Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("the drawing reads"))
        .expect("the drawing opens")
}

/// Örnekleme's whole runs (docs/adr/0220 §7) on the parcels of `layer_scene`
/// and the stars above: the runner's check, the tool and its one undo step,
/// after a warm-up, each run undone before the next; p50 and p95 against the
/// ADR's budgets. Run on purpose:
/// `cargo test --release -p kentos-processing --test cases sampling_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn sampling_timing() {
    use std::time::Instant;
    let registry = Registry::builtin();
    let all = json!({ "scope": "all" });
    let values = |pairs: Vec<(&str, Value)>| -> Values {
        pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
    };
    let parcels = || layer_scene();
    let stars = || stars_scene();
    // A run: the tool, its title, its drawing, its values, its budget (ms).
    type Run = (&'static str, &'static str, fn() -> Document, Values, f64);
    let runs: Vec<Run> = vec![
        (
            "sample.grid",
            "Ağ oluştur, 100 000 parselin kutusunda 12 m'lik altıgenler, Nesnelere değenler",
            parcels as fn() -> Document,
            values(vec![
                ("input", all.clone()),
                ("shape", json!("hexagon")),
                ("side", json!(12)),
                ("snap", json!(true)),
                ("keep", json!("meeting")),
                ("layer", json!({ "newName": "Ağ" })),
            ]),
            800.0,
        ),
        (
            "sample.regular",
            "Düzenli noktalar, 100 000 parselde 12 m aralık",
            parcels,
            values(vec![
                ("input", all.clone()),
                ("dx", json!(12)),
                ("dy", json!(12)),
                ("snap", json!(true)),
                ("shift", json!(0)),
                ("layer", json!({ "newName": "Düzenli noktalar" })),
            ]),
            700.0,
        ),
        (
            "sample.random",
            "Rastgele noktalar, 100 000 parselde birer, en az 2 m arayla",
            parcels,
            values(vec![
                ("input", all.clone()),
                ("count", json!("1")),
                ("min", json!(2)),
                ("tries", json!(10)),
                ("seed", json!(1)),
                ("layer", json!({ "newName": "Rastgele noktalar" })),
            ]),
            600.0,
        ),
        (
            "sample.alongLines",
            "Hat boyunca nokta, 100 000 parselin sınırında 25 m'de bir",
            parcels,
            values(vec![
                ("input", all.clone()),
                ("mode", json!("interval")),
                ("step", json!(25)),
                ("start", json!(0)),
                ("end", json!(0)),
                ("ends", json!(true)),
                ("layer", json!({ "newName": "Hat noktaları" })),
            ]),
            900.0,
        ),
        (
            "sample.subdivide",
            "Alt bölümlere ayır, 10 alanda 100 000 köşe, en çok 256",
            stars as fn() -> Document,
            values(vec![
                ("input", all.clone()),
                ("max", json!(256)),
                ("layer", json!({ "newName": "Alt bölümler" })),
            ]),
            1200.0,
        ),
    ];
    let mut over = Vec::new();
    for (id, title, scene, values, budget) in runs {
        let tool = registry.tool(id).expect("the tool");
        let mut host = TestHost {
            doc: scene(),
            selection: Vec::new(),
            view: None,
        };
        let mut runner = Runner::new();
        let mut ms = Vec::new();
        let mut summary = String::new();
        for run in 0..8 {
            let mut log = Vec::new();
            let t = Instant::now();
            let outcome = runner.run(&mut host, &tool, &values, true, &mut log);
            let took = t.elapsed().as_secs_f64() * 1e3;
            let Outcome::Ok { record, .. } = outcome else {
                panic!("{id}: {log:?}");
            };
            summary = record.summary;
            assert!(host.doc.undo().is_some(), "{id}: one step");
            std::thread::sleep(std::time::Duration::from_millis(400));
            if run >= 2 {
                ms.push(took);
            }
        }
        ms.sort_by(f64::total_cmp);
        let (p50, p95) = (
            ms[ms.len() / 2],
            ms[(ms.len() * 95 / 100).min(ms.len() - 1)],
        );
        println!("{title}: p50 {p50:.1} ms, p95 {p95:.1} ms, bütçe {budget} ms\n  {summary}");
        let mut log = Vec::new();
        let t = Instant::now();
        let Prepared::Ready(job) = runner.prepare(&host, &tool, &values, true, &mut log) else {
            panic!("{id} prepared: {log:?}");
        };
        let prepared = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let result = Runner::compute(
            &job,
            &host.doc,
            &mut Apart {
                log: &mut log,
                files: None,
            },
        );
        let computed = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        runner.finish(&mut host, job, result, false, &mut log);
        println!(
            "  hazırlık {prepared:.1} ms, araç {computed:.1} ms, uygulama {:.1} ms",
            t.elapsed().as_secs_f64() * 1e3
        );
        if p50 > budget {
            over.push(format!("{title}: p50 {p50:.1} ms > {budget} ms"));
        }
    }
    assert!(over.is_empty(), "{over:#?}");
}

/// A polygon of the timings' scenes: its id, attributes JSON and ring.
type Polygon = (usize, String, Vec<(f64, f64)>);

/// A drawing of `polygons` on one layer, for the timings.
fn polygons_scene(name: &str, polygons: Vec<Polygon>) -> Document {
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let entities: Vec<String> = polygons
        .into_iter()
        .map(|(id, attrs, ring)| {
            let pts: Vec<String> = ring
                .iter()
                .map(|(x, y)| format!(r#"{{"x":{},"y":{}}}"#, e0 + x, n0 + y))
                .collect();
            format!(
                r#"{{"kind":"polygon","id":{id},"layerId":"g","attrs":{attrs},"pts":[{}]}}"#,
                pts.join(",")
            )
        })
        .collect();
    let json = format!(
        r#"{{"format":"kentos.document","version":1,"name":"{name}","settings":{{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis","drawingFont":"barlow"}},
            "origin":{{"x":{e0},"y":{n0}}},"layers":[{{"id":"g","name":"Genelleştirme","type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{{"color":"fg","lineType":"continuous","lineWeight":0.25}},"children":[]}}],"activeLayer":"g","entities":[{}],"styles":{{"items":[],"categories":[]}}}}"#,
        entities.join(",")
    );
    Document::from_snapshot(DocumentSnapshotV1::from_json(&json).expect("the drawing reads"))
        .expect("the drawing opens")
}

/// A wobble of a grid edge's inner vertex `k` (±0.08 m), the same for both parcels that share the edge.
fn wobble(a: usize, b: usize, k: usize) -> f64 {
    (((a * 31 + b * 17 + k * 7) % 9) as f64 - 4.0) * 0.02
}

/// A coverage of `cols` × `rows` parcels of 20 m, each side with three inner
/// vertices that wobble (16 vertices a parcel), neighbours sharing them
/// exactly (docs/adr/0221 §10); the web's `generalize.test.ts` builds the same.
pub fn coverage_scene(cols: usize, rows: usize) -> Document {
    let mut polygons = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        for c in 0..cols {
            let (x0, y0) = (c as f64 * 20.0, r as f64 * 20.0);
            let (x1, y1) = (x0 + 20.0, y0 + 20.0);
            let mut ring = vec![(x0, y0)];
            // Bottom: the horizontal edge (c, r); right: the vertical edge (c + 1, r); top and left reversed.
            for k in 1..4 {
                ring.push((x0 + 5.0 * k as f64, y0 + wobble(c, 2 * r, k)));
            }
            ring.push((x1, y0));
            for k in 1..4 {
                ring.push((x1 + wobble(c + 1, 2 * r + 1, k), y0 + 5.0 * k as f64));
            }
            ring.push((x1, y1));
            for k in (1..4).rev() {
                ring.push((x0 + 5.0 * k as f64, y1 + wobble(c, 2 * (r + 1), k)));
            }
            ring.push((x0, y1));
            for k in (1..4).rev() {
                ring.push((x0 + wobble(c, 2 * r + 1, k), y0 + 5.0 * k as f64));
            }
            let id = r * cols + c + 1;
            polygons.push((id, format!(r#"{{"Parsel":"{id}"}}"#), ring));
        }
    }
    polygons_scene("Kaplama", polygons)
}

/// `blocks` blocks of four buildings 3 m apart, 40 m from the next block:
/// each a 12 × 8 m building with a 0.8 m step on its top side, its corners
/// off by up to 0.15 m and the whole turned up to 1.2° (docs/adr/0221 §10);
/// the web's `generalize.test.ts` builds the same.
pub fn buildings_scene(blocks: usize) -> Document {
    let shape = [
        (0.0, 0.0),
        (12.0, 0.0),
        (12.0, 8.0),
        (6.0, 8.0),
        (6.0, 8.8),
        (0.0, 8.8),
    ];
    let per_row = 50;
    let mut polygons = Vec::with_capacity(blocks * 4);
    let mut id = 0;
    for b in 0..blocks {
        let (bx, by) = ((b % per_row) as f64 * 70.0, (b / per_row) as f64 * 60.0);
        for (ox, oy) in [(0.0, 0.0), (15.0, 0.0), (0.0, 11.8), (15.0, 11.8)] {
            id += 1;
            let turn = ((id % 7) as f64 - 3.0) * 0.4 * std::f64::consts::PI / 180.0;
            let (sin, cos) = turn.sin_cos();
            let ring: Vec<(f64, f64)> = shape
                .iter()
                .enumerate()
                .map(|(i, &(x, y))| {
                    let (nx, ny) = (
                        ((id * 7 + i * 3) % 7) as f64 * 0.05 - 0.15,
                        ((id * 5 + i * 11) % 7) as f64 * 0.05 - 0.15,
                    );
                    let (x, y) = (x - 6.0 + nx, y - 4.4 + ny);
                    (
                        bx + ox + 6.0 + x * cos - y * sin,
                        by + oy + 4.4 + x * sin + y * cos,
                    )
                })
                .collect();
            polygons.push((id, format!(r#"{{"Bina":"{id}","Blok":"{}"}}"#, b + 1), ring));
        }
    }
    polygons_scene("Binalar", polygons)
}

/// Rows of parcels 20 m deep, nineteen 20 m wide then a 2 m sliver (40 m²,
/// 5 % of them), neighbours sharing their sides (docs/adr/0221 §10); the
/// web's `generalize.test.ts` builds the same.
pub fn slivers_scene(cols: usize, rows: usize) -> Document {
    let mut polygons = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        let mut x = 0.0;
        let y0 = r as f64 * 20.0;
        for c in 0..cols {
            let w = if c % 20 == 19 { 2.0 } else { 20.0 };
            let id = r * cols + c + 1;
            polygons.push((
                id,
                format!(r#"{{"Parsel":"{id}"}}"#),
                vec![(x, y0), (x + w, y0), (x + w, y0 + 20.0), (x, y0 + 20.0)],
            ));
            x += w;
        }
    }
    polygons_scene("Şeritler", polygons)
}

/// Genelleştirme's whole runs (docs/adr/0221 §10): the runner's check, the
/// tool and its one undo step, after a warm-up, each run undone before the
/// next; p50 and p95 against the ADR's budgets. Run on purpose:
/// `cargo test --release -p kentos-processing --test cases generalize_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run on purpose in release"]
fn generalize_timing() {
    use std::time::Instant;
    let registry = Registry::builtin();
    let all = json!({ "scope": "all" });
    let values = |pairs: Vec<(&str, Value)>| -> Values {
        pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
    };
    fn coverage() -> Document {
        coverage_scene(320, 313)
    }
    fn small_coverage() -> Document {
        coverage_scene(100, 100)
    }
    fn buildings() -> Document {
        buildings_scene(2500)
    }
    fn slivers() -> Document {
        slivers_scene(400, 250)
    }
    // A run: the tool, its title, its drawing, its values, its budget (ms).
    type Run = (&'static str, &'static str, fn() -> Document, Values, f64);
    let runs: Vec<Run> = vec![
        (
            "generalize.smooth",
            "Yumuşat, 10 000 parselde ortak kenarlarla, üç yineleme",
            small_coverage,
            values(vec![
                ("input", all.clone()),
                ("iterations", json!(3)),
                ("offset", json!(0.25)),
                ("maxAngle", json!(180)),
                ("shared", json!(true)),
                ("layer", json!({ "newName": "Yumuşatılan" })),
            ]),
            500.0,
        ),
        (
            "generalize.orthogonalize",
            "Dik açılandır, 10 000 eğik bina",
            buildings,
            values(vec![
                ("input", all.clone()),
                ("threshold", json!(15)),
                ("iterations", json!(1000)),
                ("layer", json!({ "newName": "Dik açılandırılan" })),
            ]),
            700.0,
        ),
        (
            "generalize.building",
            "Bina sadeleştir, 10 000 basamaklı bina, 3 m",
            buildings,
            values(vec![
                ("input", all.clone()),
                ("minEdge", json!(3)),
                ("parallel", json!(20)),
                ("orthogonal", json!(20)),
                ("layer", json!({ "newName": "Sadeleştirilen binalar" })),
            ]),
            300.0,
        ),
        (
            "generalize.aggregate",
            "Alanları kümele, 10 000 bina dörtlü bloklarda, 5 m",
            buildings,
            values(vec![
                ("input", all.clone()),
                ("distance", json!(5)),
                ("minArea", json!(0)),
                ("minHole", json!(0)),
                ("fillNotches", json!(false)),
                ("layer", json!({ "newName": "Kümelenen alanlar" })),
            ]),
            5000.0,
        ),
        (
            "generalize.dropParts",
            "Küçük parçaları ayıkla, 100 000 parsel",
            coverage,
            values(vec![
                ("input", all.clone()),
                ("minArea", json!(10)),
                ("percent", json!(0)),
                ("parts", json!(true)),
                ("holes", json!(true)),
                ("layer", json!({ "newName": "Ayıklanan" })),
            ]),
            500.0,
        ),
        (
            "generalize.eliminate",
            "Küçükleri komşuya kat, 100 000 parselin 5 000 şeridi",
            slivers,
            values(vec![
                ("input", all.clone()),
                ("minArea", json!(100)),
                ("into", json!("longest")),
                ("layer", json!({ "newName": "Katılan" })),
            ]),
            800.0,
        ),
        (
            "generalize.coverageSimplify",
            "Ortak kenarları koruyarak sadeleştir, 100 000 parsel, 1,6 milyon köşe, 1 m",
            coverage,
            values(vec![
                ("input", all.clone()),
                ("tolerance", json!(1)),
                ("keepBoundary", json!(false)),
                ("layer", json!({ "newName": "Sadeleştirilen kaplama" })),
            ]),
            2000.0,
        ),
    ];
    let only = std::env::var("KENTOS_TIMING_ONLY").ok();
    let mut over = Vec::new();
    for (id, title, scene, values, budget) in runs {
        if only
            .as_deref()
            .is_some_and(|o| !o.split(',').any(|x| id.ends_with(x)))
        {
            continue;
        }
        let tool = registry.tool(id).expect("the tool");
        let mut host = TestHost {
            doc: scene(),
            selection: Vec::new(),
            view: None,
        };
        let mut runner = Runner::new();
        let mut ms = Vec::new();
        let mut summary = String::new();
        for run in 0..6 {
            let mut log = Vec::new();
            let t = Instant::now();
            let outcome = runner.run(&mut host, &tool, &values, true, &mut log);
            let took = t.elapsed().as_secs_f64() * 1e3;
            let Outcome::Ok { record, .. } = outcome else {
                panic!("{id}: {log:?}");
            };
            summary = record.summary;
            assert!(host.doc.undo().is_some(), "{id}: one step");
            std::thread::sleep(std::time::Duration::from_millis(300));
            if run >= 2 {
                ms.push(took);
            }
        }
        ms.sort_by(f64::total_cmp);
        let (p50, p95) = (
            ms[ms.len() / 2],
            ms[(ms.len() * 95 / 100).min(ms.len() - 1)],
        );
        println!("{title}: p50 {p50:.1} ms, p95 {p95:.1} ms, bütçe {budget} ms\n  {summary}");
        let mut log = Vec::new();
        let t = Instant::now();
        let Prepared::Ready(job) = runner.prepare(&host, &tool, &values, true, &mut log) else {
            panic!("{id} prepared: {log:?}");
        };
        let prepared = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let result = Runner::compute(
            &job,
            &host.doc,
            &mut Apart {
                log: &mut log,
                files: None,
            },
        );
        let computed = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        runner.finish(&mut host, job, result, false, &mut log);
        println!(
            "  hazırlık {prepared:.1} ms, araç {computed:.1} ms, uygulama {:.1} ms",
            t.elapsed().as_secs_f64() * 1e3
        );
        if p50 > budget {
            over.push(format!("{title}: p50 {p50:.1} ms > {budget} ms"));
        }
    }
    assert!(over.is_empty(), "{over:#?}");
}

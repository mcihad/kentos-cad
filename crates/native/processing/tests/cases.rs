//! The shared processing cases (fixtures/processing/v1, the format in
//! fixtures/processing/README.md): each case runs a built-in tool or model
//! on a drawing through the native runner, and what the run did is compared
//! with what the case says. The web plays the same file
//! (apps/web/src/processing/cases.test.ts).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use kentos_contracts::{DocumentSnapshotV1, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geometry::Bounds;
use kentos_processing::model_runner::{model_as_tool, record_model, replay_model, run_model};
use kentos_processing::parameters::default_values;
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

/// Numbers other than coordinates are exact: only geometry fields get the tolerance.
fn same_object(have: &Value, want: &Value, tol: f64) -> bool {
    const GEOMETRY: [&str; 6] = ["p", "a", "b", "c", "pts", "holes"];
    let (Some(h), Some(w)) = (have.as_object(), want.as_object()) else {
        return false;
    };
    let keys = |o: &Map<String, Value>| o.keys().cloned().collect::<BTreeSet<_>>();
    keys(h) == keys(w)
        && w.iter().all(|(k, v)| {
            let got = &h[k];
            if GEOMETRY.contains(&k.as_str()) {
                close(got, v, tol)
            } else {
                same(got, v)
            }
        })
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

fn with_values(tool: &Tool, doc: &Document, over: Option<&Value>) -> Values {
    let mut values = default_values(tool, &Defaults::of(doc));
    if let Some(Value::Object(over)) = over {
        for (k, v) in over {
            values.insert(k.clone(), v.clone());
        }
    }
    values
}

/// A tool's messages when it is computed apart from its host (the desktop's background runs).
struct Apart<'a> {
    log: &'a mut Vec<LogLine>,
}

impl Feedback for Apart<'_> {
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
fn play(c: &Value, registry: &Registry, on_copy: bool) -> Seen {
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
        let values = with_values(&tool, &host.doc, c.get("values"));
        if on_copy {
            match runner.prepare(&host, &tool, &values, false, &mut log) {
                Prepared::Ready(job) => {
                    let copy = host.doc.reading_copy();
                    let result = Runner::compute(&job, &copy, &mut Apart { log: &mut log });
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
        let values = with_values(&as_tool, &host.doc, c.get("values"));
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
                &mut Apart { log: &mut unheard },
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
            let layers: Vec<Value> = doc
                .layers()
                .leaves()
                .into_iter()
                .filter(|l| !known.contains(&l.id))
                .map(|l| json!({ "id": l.id, "name": l.name, "style": l.style }))
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
                "added": added,
                "updated": updated,
                "removed": removed,
                "selection": s.host.selection.iter().map(|s| s.0).collect::<Vec<_>>(),
                "outputs": result.outputs,
            })
        }
    }
}

fn check(c: &Value, mut s: Seen, tol: f64) -> Vec<String> {
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
    let (have, wanted) = (or_empty(&got["added"]), or_empty(&want["added"]));
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
            expect(same_object(h, w, tol), &format!("added[{i}]"), h, w);
        }
    }
    expect(
        same(&got["updated"], &or_empty(&want["updated"])),
        "değişen",
        &got["updated"],
        &want["updated"],
    );
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

fn cases() -> Value {
    let file: Value = serde_json::from_str(&fixture("cases.json")).expect("cases.json reads");
    assert_eq!(
        (file["format"].as_str(), file["version"].as_u64()),
        (Some("kentos.processing-cases"), Some(1)),
        "a v1 case file"
    );
    file
}

/// Each drawing's defaults and each tool's default values on it, as the web reads them.
#[test]
fn the_defaults_the_tools_take_from_the_drawing() {
    let file = cases();
    let registry = Registry::builtin();
    let lookup = |id: &str| registry.tool(id);
    for (name, d) in file["documents"].as_object().expect("documents") {
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
        problems.extend(check(c, play(c, &registry, true), tol));
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
        let found = check(c, play(c, &registry, false), tol);
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

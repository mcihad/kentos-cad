//! The acceptance corpus's flows on the desktop (the 2+1 apartment and, docs/adr/0798, the corpus's other samples,
//! `CORPUS`). The 2+1 apartment's (plan/architecture-advanced.md ADV-07, docs/adr/0776): the
//! shared scenario (fixtures/acceptance/v1/flat.json, worked out by scripts/fixtures/acceptance_flat.py without
//! KentOS code) replayed in the app through the agent link an MCP client attaches to (`desktop.attach`), undone and
//! redone as Ctrl+Z and Ctrl+Y do it, saved and opened as the app's own; every bound object the same by its id after
//! the reopening. With `KENTOS_ACCEPTANCE_OUT` the drawing is saved there as `desktop.kcad` for the hosts'
//! comparison (`acceptance_flat.py --compare`). The mouse-and-keyboard trace draws the same apartment
//! (fixtures/interaction/v1/acceptance-flat.json); `perf::flat` measures it, copied.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::acceptance_tests::{bind, differs};
use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

pub(crate) const SCENARIO: &str = include_str!("../../../fixtures/acceptance/v1/flat.json");
/// The acceptance corpus's other samples (docs/adr/0798), by id.
pub(crate) const CORPUS: [(&str, &str); 4] = [
    ("change", include_str!("../../../fixtures/acceptance/v1/change.json")),
    ("house", include_str!("../../../fixtures/acceptance/v1/house.json")),
    ("curved", include_str!("../../../fixtures/acceptance/v1/curved.json")),
    ("renovation", include_str!("../../../fixtures/acceptance/v1/renovation.json")),
];

fn ask(app: &mut App, method: &str, params: Value) -> Value {
    app.agents_answer(&json!({ "id": 1, "method": method, "params": params }))
        .unwrap_or_else(|(code, message)| panic!("{method}: {code}: {message}"))
}

fn open(app: &mut App, path: &std::path::Path) {
    let doc = Document::read(path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(app, task);
}

fn count(app: &App) -> usize {
    app.document.as_ref().map_or(0, |d| d.model.entities().count())
}

/// Every bound object as the agent reads it.
fn shown(app: &mut App, names: &BTreeMap<String, String>) -> BTreeMap<String, Value> {
    names
        .iter()
        // The layer and the scenario's files (absolute paths) are not objects.
        .filter(|(k, u)| *k != "layer" && !u.contains('/'))
        .map(|(k, u)| (k.clone(), ask(app, "entity", json!({ "uid": u }))["entity"].clone()))
        .collect()
}

fn bound(output: &Value, key: &Value, names: &mut BTreeMap<String, String>) {
    let text = |v: &Value| v.as_str().expect("an id").to_owned();
    match key {
        Value::String(name) => {
            let uid = [&output["uid"], &output["set"], &output["uids"][0]]
                .into_iter()
                .find(|v| v.is_string())
                .expect("an id");
            names.insert(name.clone(), text(uid));
        }
        Value::Array(keys) => {
            let ids = output["created"].as_array().or(output["uids"].as_array()).expect("ids");
            for (k, u) in keys.iter().zip(ids) {
                names.insert(text(k), text(u));
            }
        }
        _ => {}
    }
}

/// The app with the scenario's empty project open and the agent link up; the folder its files go to.
pub(crate) fn opened(dir_name: &str) -> (App, std::path::PathBuf) {
    opened_with(SCENARIO, dir_name)
}

/// [`opened`] with the scenario `text`'s empty project.
pub(crate) fn opened_with(text: &str, dir_name: &str) -> (App, std::path::PathBuf) {
    let scenario: Value = serde_json::from_str(text).expect("the scenario");
    let dir = scratch(dir_name);
    let choices = kentos_headless::new_drawing(&scenario["project"]).expect("the choices");
    let made = kentos_headless::Session::new(&choices).expect("a new project");
    let path = dir.join("daire.kcad");
    std::fs::write(&path, made.to_bytes().expect("its bytes")).expect("written");
    let (mut app, _) = App::boot(None);
    open(&mut app, &path);
    let (link, _task) = crate::python::link::Link::open(&dir.join("m.sock"), |heard| {
        Message::Python(crate::python::Event::Agent(heard))
    })
    .expect("the agent link");
    app.python.link = Some(link);
    (app, dir)
}

/// The scenario's commands in the open drawing, no checks: what the performance measures copy; its bound ids.
#[cfg(unix)]
pub(crate) fn drawn(app: &mut App) -> BTreeMap<String, String> {
    drawn_with(app, SCENARIO)
}

/// [`drawn`] for the scenario `text`: its commands and its queries' bindings, refusals and locks left out.
#[cfg(unix)]
pub(crate) fn drawn_with(app: &mut App, text: &str) -> BTreeMap<String, String> {
    let scenario: Value = serde_json::from_str(text).expect("the scenario");
    let mut names = first_names(app, &scenario);
    for step in scenario["steps"].as_array().expect("steps") {
        if let Some(query) = step["query"].as_str() {
            let answer = ask(app, "run", json!({ "command": query, "op": "execute", "input": bind(&step["input"], &names) }));
            bound(&answer["output"], &step["as"], &mut names);
            continue;
        }
        let Some(command) = step["command"].as_str() else { continue };
        if step.get("refuse").is_some() || command == "cad.layers.set" {
            continue;
        }
        let answer = ask(app, "run", json!({ "command": command, "op": "execute", "input": bind(&step["input"], &names) }));
        assert_eq!(answer["status"], "completed", "{}: {answer}", step["name"]);
        bound(&answer["output"], &step["as"], &mut names);
    }
    names
}

/// The open drawing's active layer and the scenario's files (absolute paths) by their names.
fn first_names(app: &App, scenario: &Value) -> BTreeMap<String, String> {
    let mut names = BTreeMap::new();
    names.insert(
        "layer".to_owned(),
        app.document.as_ref().map(|d| d.model.layers().active().to_owned()).expect("a layer"),
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (k, path) in scenario["files"].as_object().into_iter().flatten() {
        let file = root.join(path.as_str().expect("a path")).canonicalize().expect("the scenario's file");
        names.insert(k.clone(), file.to_string_lossy().into_owned());
    }
    names
}

#[cfg(unix)]
#[test]
fn the_apartment_is_drawn_changed_scheduled_and_reopened_as_the_scenario_says() {
    replay(SCENARIO, "desktop.kcad");
}

/// The acceptance corpus's other samples (docs/adr/0798), each saved as `<id>/desktop.kcad`.
#[cfg(unix)]
#[test]
fn every_corpus_sample_is_drawn_changed_and_reopened_as_its_scenario_says() {
    for (id, text) in CORPUS {
        replay(text, &format!("{id}/desktop.kcad"));
    }
}

/// Replays a `kentos.acceptance` scenario through the agent link; with `KENTOS_ACCEPTANCE_OUT` the drawing is
/// saved there as `out_name`.
#[cfg(unix)]
fn replay(text: &str, out_name: &str) {
    let scenario: Value = serde_json::from_str(text).expect("the scenario");
    let tolerance = scenario["tolerance"].as_f64().expect("a tolerance");
    let (mut app, dir) = opened_with(text, &format!("kabul-{}", scenario["id"].as_str().unwrap_or("daire")));
    let mut names = first_names(&app, &scenario);
    let mut undone = 0;
    for (n, step) in scenario["steps"].as_array().expect("steps").iter().enumerate() {
        let name = step["name"].as_str().unwrap_or("");
        let mut output = Value::Null;
        if let Some(command) = step["command"].as_str() {
            let before = count(&app);
            let answer = ask(
                &mut app,
                "run",
                json!({ "command": command, "op": "execute", "input": bind(&step["input"], &names) }),
            );
            app.follow_sheets();
            if let Some(code) = step["refuse"].as_str() {
                assert_eq!(answer["status"], "failed", "{name}: {answer}");
                assert_eq!(answer["error"]["code"], code, "{name}: {answer}");
                assert_eq!(count(&app), before, "{name}: a refusal writes nothing");
            } else {
                assert_eq!(answer["status"], "completed", "{name}: {answer}");
                output = answer["output"].clone();
                bound(&output, &step["as"], &mut names);
                if let Some(label) = step["label"].as_str() {
                    // Ctrl+Z and Ctrl+Y: the app's own undo and redo, each saying the step.
                    let after = count(&app);
                    let objects = shown(&mut app, &names);
                    let said = app.log.lines().count();
                    app.undo();
                    assert!(
                        app.log.lines().skip(said).any(|l| l.text.contains(label)),
                        "{name}: the undone step “{label}” is said"
                    );
                    assert_eq!(count(&app), before, "{name}: undone");
                    app.step_history(false);
                    assert_eq!(count(&app), after, "{name}: redone");
                    assert_eq!(shown(&mut app, &names), objects, "{name}: redone the same objects");
                    undone += 1;
                }
            }
        } else if let Some(query) = step["query"].as_str() {
            let answer = ask(&mut app, "run", json!({ "command": query, "op": "execute", "input": bind(&step["input"], &names) }));
            assert_eq!(answer["status"], "completed", "{name}: {answer}");
            output = answer["output"].clone();
            bound(&output, &step["as"], &mut names);
        } else if step["reopen"] == true {
            let objects = shown(&mut app, &names);
            let file = dir.join(format!("daire-{n}.kcad"));
            let snapshot = app.document.as_ref().expect("a drawing").model.to_snapshot_v2();
            crate::document::write(&snapshot, &file).expect("saved");
            open(&mut app, &file);
            assert_eq!(shown(&mut app, &names), objects, "{name}: every bound object by its id");
        }
        let expect = &step["expect"];
        let mut problems = Vec::new();
        for (key, want) in expect["objects"].as_object().into_iter().flatten() {
            let got = ask(&mut app, "entity", json!({ "uid": names[key] }));
            differs(&got["entity"], &bind(want, &names), tolerance, key, &mut problems);
        }
        for (key, want) in expect["measures"].as_object().into_iter().flatten() {
            let got = ask(&mut app, "measure", json!({ "uid": names[key] }));
            differs(&got, want, tolerance, &format!("{key} ölçü"), &mut problems);
        }
        if let Some(want) = expect.get("count") {
            differs(&json!(count(&app)), want, 0.0, "nesne sayısı", &mut problems);
        }
        for (kind, want) in expect["kinds"].as_object().into_iter().flatten() {
            let got = app
                .document
                .as_ref()
                .map_or(0, |d| d.model.entities().filter(|e| e.kind() == kind.as_str()).count());
            differs(&json!(got), want, 0.0, &format!("tür {kind}"), &mut problems);
        }
        if let Some(want) = expect.get("output") {
            differs(&output, &bind(want, &names), 1e-9, "cevap", &mut problems);
        }
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
    assert!(undone > scenario["undone"].as_u64().unwrap_or(10), "{undone} steps undone");
    if let Some(out) = std::env::var_os("KENTOS_ACCEPTANCE_OUT") {
        let file = std::path::PathBuf::from(out).join(out_name);
        std::fs::create_dir_all(file.parent().expect("a folder")).expect("the output folder");
        let snapshot = app.document.as_ref().expect("a drawing").model.to_snapshot_v2();
        crate::document::write(&snapshot, &file).expect("saved");
    }
}

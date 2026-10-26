//! The first GIS acceptance project on the desktop (plan/gis.md CBS-44, docs/adr/0554, 0655): the shared scenario
//! (fixtures/acceptance/v1/parcels.json, worked out by scripts/fixtures/acceptance_parcels.py without KentOS code)
//! replayed in the app, its commands through the agent link an MCP client attaches to, undo as Ctrl+Z does it, the
//! reopening as the app's own save (`saving::write_shared`) and open, with a save stopped half way that leaves the file
//! as it was; five formats written and the GeoPackage opened in a new project with nothing lost. Python and the MCP
//! server replay the same file.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

const SCENARIO: &str = include_str!("../../../fixtures/acceptance/v1/parcels.json");

/// Where `actual` differs from `expected`: only the fields `expected` names; numbers to `tolerance`.
fn differs(actual: &Value, expected: &Value, tolerance: f64, at: &str, out: &mut Vec<String>) {
    match expected {
        Value::Object(fields) => {
            for (k, v) in fields {
                differs(&actual[k.as_str()], v, tolerance, &format!("{at}.{k}"), out);
            }
        }
        Value::Number(n) => {
            let ok = actual
                .as_f64()
                .zip(n.as_f64())
                .is_some_and(|(a, e)| (a - e).abs() <= tolerance);
            if !ok {
                out.push(format!("{at}: {actual}, beklenen {expected}"));
            }
        }
        _ => {
            if actual != expected {
                out.push(format!("{at}: {actual}, beklenen {expected}"));
            }
        }
    }
}

fn bind(value: &Value, names: &BTreeMap<String, String>, out: &std::path::Path) -> Value {
    match value {
        Value::String(s) if s.starts_with("$out:") => {
            json!(out.join(&s[5..]).display().to_string())
        }
        Value::String(s) if s.starts_with('$') => json!(names[&s[1..]]),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), bind(v, names, out)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|v| bind(v, names, out)).collect()),
        other => other.clone(),
    }
}

fn layer_named(nodes: &Value, name: &str) -> Option<String> {
    for n in nodes.as_array().into_iter().flatten() {
        if n["name"] == name && n["type"] == "layer" {
            return n["id"].as_str().map(str::to_owned);
        }
        if let Some(id) = layer_named(&n["children"], name) {
            return Some(id);
        }
    }
    None
}

/// An agent's request to the open drawing, answered as the link answers it.
fn ask(app: &mut App, method: &str, params: Value) -> Value {
    app.agents_answer(&json!({ "id": 1, "method": method, "params": params }))
        .unwrap_or_else(|(code, message)| panic!("{method}: {code}: {message}"))
}

fn open(app: &mut App, path: &std::path::Path) {
    let doc = Document::read(path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(app, task);
}

/// A new project as the wizard makes it, opened in the app.
fn fresh(app: &mut App, scenario: &Value, path: &std::path::Path) {
    let choices = kentos_headless::new_drawing(&scenario["project"]).expect("the choices");
    let made = kentos_headless::Session::new(&choices).expect("a new project");
    std::fs::write(path, made.to_bytes().expect("its bytes")).expect("written");
    open(app, path);
}

/// The app's own save of its drawing to `path`; `stopped` asks it to stop before the file is replaced.
fn save(app: &App, path: &std::path::Path, stopped: bool) -> Result<(), crate::saving::SaveError> {
    let model = app.document.as_ref().expect("a drawing").model.clone();
    let (head, objects) = model.to_snapshot_v2_shared();
    crate::saving::write_shared(
        &head,
        &objects,
        path,
        &AtomicBool::new(stopped),
        &mut |_| {},
        &crate::saving::Faults::NONE,
    )
}

#[cfg(unix)]
#[test]
fn the_parcels_are_made_checked_written_and_opened_elsewhere_as_the_scenario_says() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let tolerance = scenario["tolerance"].as_f64().expect("a tolerance");
    let dir = scratch("kabul-ada");
    let (mut app, _) = App::boot(None);
    fresh(&mut app, &scenario, &dir.join("ada.kcad"));
    let (link, _task) = crate::python::link::Link::open(&dir.join("m.sock"), |heard| {
        Message::Python(crate::python::Event::Agent(heard))
    })
    .expect("the agent link");
    app.python.link = Some(link);
    let mut names = BTreeMap::new();
    let mut rows: Vec<Value> = Vec::new();
    for (n, step) in scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .enumerate()
    {
        let name = step["name"].as_str().unwrap_or("");
        if step["fresh"] == true {
            rows.extend(
                ask(&mut app, "journal", json!({}))["calls"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
            );
            fresh(&mut app, &scenario, &dir.join(format!("yeni-{n}.kcad")));
        }
        let mut selected = Value::Null;
        if let Some(command) = step["command"].as_str() {
            let answer = ask(
                &mut app,
                "run",
                json!({ "command": command, "op": "execute", "input": bind(&step["input"], &names, &dir) }),
            );
            assert_eq!(answer["status"], "completed", "{name}: {answer}");
            if let Some(key) = step["as"].as_str() {
                names.insert(
                    key.to_owned(),
                    answer["output"]["uid"].as_str().expect("uid").to_owned(),
                );
            }
            for (key, field) in step["bind"].as_object().into_iter().flatten() {
                let value = answer["output"][field.as_str().expect("a field")]
                    .as_str()
                    .expect("bound");
                names.insert(key.clone(), value.to_owned());
            }
            selected = answer["output"]["selected"].clone();
        } else if let Some(label) = step["undo"].as_str() {
            // As Ctrl+Z: the app's own undo, which says the step it took back.
            app.undo();
            assert!(
                app.log.said(kentos_interaction::Level::Info, label)
                    || app.log.lines().any(|l| l.text.contains(label)),
                "{name}: the step “{label}” is said"
            );
        } else if step["reopen"] == true {
            rows.extend(
                ask(&mut app, "journal", json!({}))["calls"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
            );
            let file = dir.join(format!("ada-{n}.kcad"));
            save(&app, &file, false).expect("saved");
            let kept = std::fs::read(&file).expect("the file");
            // A save stopped before it replaces the file leaves the file as it was and no temporary file beside it.
            assert!(
                matches!(
                    save(&app, &file, true),
                    Err(crate::saving::SaveError::Stopped)
                ),
                "{name}: the stopped save"
            );
            assert_eq!(std::fs::read(&file).expect("the file"), kept, "{name}");
            let beside = std::fs::read_dir(&dir)
                .expect("the folder")
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().ends_with(".yaziliyor"))
                .count();
            assert_eq!(beside, 0, "{name}: no temporary file left");
            open(&mut app, &file);
        }
        let expect = &step["expect"];
        let mut problems = Vec::new();
        for (key, want) in expect["objects"].as_object().into_iter().flatten() {
            let got = ask(&mut app, "entity", json!({ "uid": names[key] }));
            differs(&got["entity"], want, tolerance, key, &mut problems);
        }
        for (key, want) in expect["measures"].as_object().into_iter().flatten() {
            let got = ask(&mut app, "measure", json!({ "uid": names[key] }));
            differs(&got, want, tolerance, &format!("{key} ölçü"), &mut problems);
        }
        if let Some(count) = expect.get("count") {
            let summary = ask(&mut app, "summary", json!({}));
            differs(
                &summary["objects"],
                count,
                0.0,
                "nesne sayısı",
                &mut problems,
            );
        }
        if let Some(want) = expect["selected"].as_array() {
            let mut want: Vec<String> = want
                .iter()
                .filter_map(|k| k.as_str())
                .map(|k| names[k].clone())
                .collect();
            let mut got: Vec<String> = selected
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|u| u.as_str().map(str::to_owned))
                .collect();
            want.sort();
            got.sort();
            if got != want {
                problems.push(format!("seçilenler {got:?}, beklenen {want:?}"));
            }
        }
        for (layer, spec) in expect["layers"].as_object().into_iter().flatten() {
            let tree = ask(&mut app, "layers", json!({}));
            let Some(id) = layer_named(&tree, layer) else {
                problems.push(format!("{layer} katmanı yok"));
                continue;
            };
            let page = ask(&mut app, "entities", json!({ "layer": id }));
            let key = spec["key"].as_str().expect("a key");
            let items = page["items"].as_array().cloned().unwrap_or_default();
            let rows_of = spec["rows"].as_object().expect("rows");
            if items.len() != rows_of.len() {
                problems.push(format!(
                    "{layer}: {} nesne, beklenen {}",
                    items.len(),
                    rows_of.len()
                ));
            }
            for item in &items {
                let attrs = &item["entity"]["attrs"];
                let Some(k) = attrs[key].as_str() else {
                    problems.push(format!("{layer}: {key} yok"));
                    continue;
                };
                differs(
                    attrs,
                    &rows_of[k],
                    0.0,
                    &format!("{layer}/{k}"),
                    &mut problems,
                );
                let m = ask(&mut app, "measure", json!({ "uid": item["uid"] }));
                differs(
                    &m["area"],
                    &spec["areas"][k],
                    tolerance,
                    &format!("{layer}/{k} alan"),
                    &mut problems,
                );
            }
        }
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
    rows.extend(
        ask(&mut app, "journal", json!({}))["calls"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
    );
    assert!(
        rows.iter().all(|r| r["kind"] == "command"),
        "every write went through a command: {rows:#?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

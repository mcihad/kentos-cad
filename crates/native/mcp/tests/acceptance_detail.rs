//! CAD-ADV-20 acceptance (docs/adr/1004) through the MCP server: the parametric plate of
//! fixtures/acceptance/v1/detail.json (written by scripts/fixtures/acceptance_detail.py without KentOS code) replayed
//! as a model would, tool call after tool call. Python replays the same file (python/tests/test_acceptance_detail.py):
//! the same points, lengths, areas, refusals, undo, request replay, save–open, a failed half-way script, a cancelled
//! job and a detail sheet (docs/adr/1011).

use std::collections::BTreeMap;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

const SCENARIO: &str = include_str!("../../../../fixtures/acceptance/v1/detail.json");

fn tool(server: &mut Server, name: &str, arguments: Value) -> Value {
    let request = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": name, "arguments": arguments,
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": MODERN,
                "io.modelcontextprotocol/clientInfo": { "name": "kabul", "version": "1" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    server.handle(&request).expect("an answer")["result"].clone()
}

fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
    let result = tool(server, name, arguments);
    assert_eq!(result["isError"], false, "{name}: {result}");
    result["structuredContent"].clone()
}

fn bind(value: &Value, names: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(s) if s.starts_with('$') && names.contains_key(&s[1..]) => json!(names[&s[1..]]),
        Value::Array(items) => Value::Array(items.iter().map(|v| bind(v, names)).collect()),
        Value::Object(map) => Value::Object(map.iter().map(|(k, v)| (k.clone(), bind(v, names))).collect()),
        other => other.clone(),
    }
}

/// The value at a dotted path (`sheets.0.maps.0.scale`).
fn at_path<'a>(value: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(value, |v, key| match key.parse::<usize>() {
        Ok(i) => &v[i],
        Err(_) => &v[key],
    })
}

/// The command's own result: at the top, or under `result` when the call went by the protocol.
fn result_of(answer: &Value) -> &Value {
    if answer.get("status").is_some() { answer } else { &answer["result"] }
}

#[test]
fn the_plate_follows_its_parameters_and_refusals_write_nothing_as_the_scenario_says() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let tol = scenario["tolerance"].as_f64().expect("a tolerance");
    let dir = std::env::temp_dir().join(format!("kentos-mcp-detay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let setup = dir.join("plaka.json");
    std::fs::write(&setup, scenario["setup"].to_string()).expect("the drawing");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({ "path": setup }));
    let mut drawing = opened["drawing"].as_str().expect("a handle").to_owned();
    let items = call(&mut server, "drawing.entities", json!({ "drawing": drawing }))["items"].clone();
    let mut names: BTreeMap<String, String> = scenario["names"]
        .as_array()
        .expect("names")
        .iter()
        .zip(items.as_array().expect("items"))
        .map(|(n, e)| (n.as_str().unwrap().to_owned(), e["uid"].as_str().unwrap().to_owned()))
        .collect();
    let revision = |server: &mut Server, drawing: &str| {
        call(server, "drawing.summary", json!({ "drawing": drawing }))["revision"].clone()
    };
    for (n, step) in scenario["steps"].as_array().expect("steps").iter().enumerate() {
        let name = step["name"].as_str().unwrap_or("");
        let before = revision(&mut server, &drawing);
        if let Some(command) = step["command"].as_str() {
            let mut arguments = bind(&step["input"], &names);
            arguments["drawing"] = json!(drawing);
            if let Some(id) = step.get("requestId") {
                arguments["requestId"] = id.clone();
            }
            if let Some(expected) = step.get("expected") {
                arguments["expected"] = expected.clone();
            }
            let answer = tool(&mut server, command, arguments.clone());
            let content = answer["structuredContent"].clone();
            let result = result_of(&content);
            for (path, want) in step["output"].as_object().into_iter().flatten() {
                assert_eq!(at_path(&result["output"], path), want, "{name}: output.{path}: {}", content);
            }
            if let Some(code) = step["refuse"].as_str() {
                assert!(matches!(result["status"].as_str(), Some("failed" | "conflict")), "{name}: {answer}");
                assert_eq!(result["error"]["code"], code, "{name}: {answer}");
                assert_eq!(revision(&mut server, &drawing), before, "{name}: a refusal writes nothing");
            } else {
                assert_eq!(result["status"], "completed", "{name}: {answer}");
                if let Some(keys) = step["as"].as_array() {
                    for (key, uid) in keys.iter().zip(result["output"]["created"].as_array().expect("created")) {
                        names.insert(key.as_str().unwrap().to_owned(), uid.as_str().unwrap().to_owned());
                    }
                }
            }
            if step["twice"] == true {
                let after = revision(&mut server, &drawing);
                let again = call(&mut server, command, arguments);
                assert_eq!(again["replayed"], true, "{name}: answered from before: {again}");
                assert_eq!(revision(&mut server, &drawing), after, "{name}: applied once");
            }
        } else if let Some(crash) = step.get("crash") {
            // A model runs no script: the steps a script would take are one change set, its last step the one that
            // fails. Nothing of it is written.
            let steps = bind(&crash["steps"], &names);
            let answer = tool(
                &mut server,
                "cad.changeset.apply",
                json!({ "drawing": drawing, "label": crash["label"], "steps": steps }),
            );
            let content = answer["structuredContent"].clone();
            let result = result_of(&content);
            assert!(matches!(result["status"].as_str(), Some("failed" | "conflict")), "{name}: {answer}");
            assert_eq!(revision(&mut server, &drawing), before, "{name}: nothing of it is written");
        } else if let Some(job) = step.get("job") {
            let started = call(
                &mut server,
                "drawing.job.start",
                json!({ "drawing": drawing, "command": job["command"], "input": bind(&job["input"], &names) }),
            );
            let id = started["id"].clone();
            if step["cancel"] == true {
                call(&mut server, "drawing.job.cancel", json!({ "drawing": drawing, "id": id }));
            }
            let over = call(&mut server, "drawing.job.wait", json!({ "drawing": drawing, "id": id }));
            assert_eq!(over["phase"], "cancelled", "{name}: {over}");
            assert_eq!(revision(&mut server, &drawing), before, "{name}: nothing is written");
        } else if let Some(label) = step["undo"].as_str() {
            let undone = call(&mut server, "drawing.undo", json!({ "drawing": drawing }));
            assert_eq!(undone["step"], label, "{name}");
        } else if step["reopen"] == true {
            let path = dir.join(format!("plaka-{n}.kcad"));
            call(&mut server, "drawing.save", json!({ "drawing": drawing, "path": path }));
            let opened = call(&mut server, "drawing.open", json!({ "path": path }));
            drawing = opened["drawing"].as_str().expect("a handle").to_owned();
        }
        let expect = &step["expect"];
        for (key, want) in expect["points"].as_object().into_iter().flatten() {
            let got = call(&mut server, "drawing.entity", json!({ "drawing": drawing, "uid": names[key] }));
            for end in ["a", "b"] {
                for (axis, i) in [("x", 0), ("y", 1)] {
                    let g = got["entity"][end][axis].as_f64().expect("a coordinate");
                    let w = want[end][i].as_f64().expect("a number");
                    assert!((g - w).abs() <= tol, "{name}: {key}.{end}.{axis} {g} ≠ {w}");
                }
            }
        }
        for (key, want) in expect["areas"].as_object().into_iter().flatten() {
            let got = call(&mut server, "drawing.measure", json!({ "drawing": drawing, "uid": names[key] }));
            let g = got["area"].as_f64().expect("an area");
            assert!((g - want.as_f64().unwrap()).abs() <= 1e-6, "{name}: {key} area {g} ≠ {want}");
        }
        for (key, want) in expect["lengths"].as_object().into_iter().flatten() {
            let got = call(&mut server, "drawing.measure", json!({ "drawing": drawing, "uid": names[key] }));
            let g = got["length"].as_f64().expect("a length");
            assert!((g - want.as_f64().unwrap()).abs() <= 1e-6, "{name}: {key} {g} ≠ {want}");
        }
    }
}

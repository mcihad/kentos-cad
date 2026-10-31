//! The 2+1 apartment's acceptance flow through the MCP server (plan/architecture-advanced.md ADV-07,
//! docs/adr/0776): the shared scenario (fixtures/acceptance/v1/flat.json, worked out by
//! scripts/fixtures/acceptance_flat.py without KentOS code) replayed as a model would, tool call after tool
//! call: every write step undone and redone (one step, its name, the drawing back), every refusal's code the
//! scenario's, every bound object the same by the same id after the drawing is saved and opened. With
//! `KENTOS_ACCEPTANCE_OUT` the drawing is saved there as `mcp.kcad` for the hosts' comparison
//! (`acceptance_flat.py --compare`). Python replays the same file.

use std::collections::BTreeMap;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

const SCENARIO: &str = include_str!("../../../../fixtures/acceptance/v1/flat.json");

fn tool_result(server: &mut Server, name: &str, arguments: Value) -> Value {
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
    let answer = server.handle(&request).expect("an answer");
    answer["result"].clone()
}

fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
    let result = tool_result(server, name, arguments);
    assert_eq!(result["isError"], false, "{name}: {result}");
    result["structuredContent"].clone()
}

/// Where `actual` differs from `expected`: only the fields `expected` names; a point `[x, y]` is `{"x", "y"}`;
/// numbers to `tolerance`.
fn differs(actual: &Value, expected: &Value, tolerance: f64, at: &str, out: &mut Vec<String>) {
    if let (Some(e), Some(a)) = (expected.as_array(), actual.as_object())
        && e.len() == 2
        && e.iter().all(Value::is_number)
        && a.contains_key("x")
    {
        return differs(&json!([a["x"], a["y"]]), expected, tolerance, at, out);
    }
    match expected {
        Value::Object(fields) => {
            for (k, v) in fields {
                differs(&actual[k.as_str()], v, tolerance, &format!("{at}.{k}"), out);
            }
        }
        Value::Array(items) => match actual.as_array() {
            Some(a) if a.len() == items.len() => {
                for (i, (x, e)) in a.iter().zip(items).enumerate() {
                    differs(x, e, tolerance, &format!("{at}[{i}]"), out);
                }
            }
            _ => out.push(format!("{at}: {actual}, beklenen {expected}")),
        },
        Value::Number(n) => {
            let ok = actual.as_f64().zip(n.as_f64()).is_some_and(|(a, e)| (a - e).abs() <= tolerance);
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

fn bind(value: &Value, names: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(s) if s.starts_with('$') => json!(names[&s[1..]]),
        Value::Object(fields) => Value::Object(fields.iter().map(|(k, v)| (k.clone(), bind(v, names))).collect()),
        Value::Array(items) => Value::Array(items.iter().map(|v| bind(v, names)).collect()),
        other => other.clone(),
    }
}

/// The step's `as`: a name the output's `uid`, `set` or first `uids`; a list its `created` or `uids`.
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

struct Run {
    server: Server,
    drawing: String,
}

impl Run {
    fn on(&mut self, tool: &str, mut arguments: Value) -> Value {
        arguments["drawing"] = json!(self.drawing);
        call(&mut self.server, tool, arguments)
    }
    fn count(&mut self) -> u64 {
        self.on("drawing.summary", json!({}))["objects"].as_u64().expect("a count")
    }
    fn entities(&mut self, names: &BTreeMap<String, String>) -> BTreeMap<String, Value> {
        names
            .iter()
            .filter(|(k, _)| *k != "layer")
            .map(|(k, u)| (k.clone(), self.on("drawing.entity", json!({ "uid": u }))["entity"].clone()))
            .collect()
    }
}

#[test]
fn the_apartment_is_drawn_changed_scheduled_and_reopened_as_the_scenario_says() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let tolerance = scenario["tolerance"].as_f64().expect("a tolerance");
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", scenario["project"].clone());
    let drawing = made["drawing"].as_str().expect("a handle").to_owned();
    let mut names = BTreeMap::new();
    names.insert("layer".to_owned(), made["summary"]["activeLayer"].as_str().expect("a layer").to_owned());
    let mut run = Run { server, drawing };
    let dir = std::env::temp_dir().join(format!("kentos-mcp-daire-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let mut undone = 0;
    for (n, step) in scenario["steps"].as_array().expect("steps").iter().enumerate() {
        let name = step["name"].as_str().unwrap_or("");
        let mut output = Value::Null;
        if let Some(command) = step["command"].as_str() {
            let before = run.count();
            let mut arguments = bind(&step["input"], &names);
            arguments["drawing"] = json!(run.drawing);
            if let Some(code) = step["refuse"].as_str() {
                let result = tool_result(&mut run.server, command, arguments);
                assert_eq!(result["isError"], true, "{name}: {result}");
                assert_eq!(result["structuredContent"]["error"]["code"], code, "{name}: {result}");
                assert_eq!(run.count(), before, "{name}: a refusal writes nothing");
            } else {
                let answer = call(&mut run.server, command, arguments);
                assert_eq!(answer["status"], "completed", "{name}: {answer}");
                output = answer["output"].clone();
                bound(&output, &step["as"], &mut names);
                if let Some(label) = step["label"].as_str() {
                    let after = run.count();
                    let shown = run.entities(&names);
                    assert_eq!(run.on("drawing.undo", json!({}))["step"], label, "{name}");
                    assert_eq!(run.count(), before, "{name}: undone");
                    assert_eq!(run.on("drawing.redo", json!({}))["step"], label, "{name}");
                    assert_eq!(run.count(), after, "{name}: redone");
                    assert_eq!(run.entities(&names), shown, "{name}: redone the same objects");
                    undone += 1;
                }
            }
        } else if let Some(query) = step["query"].as_str() {
            let answer = run.on(query, bind(&step["input"], &names));
            assert_eq!(answer["status"], "completed", "{name}: {answer}");
            output = answer["output"].clone();
        } else if step["reopen"] == true {
            let shown = run.entities(&names);
            let path = dir.join(format!("daire-{n}.kcad"));
            run.on("drawing.save", json!({ "path": path }));
            let opened = call(&mut run.server, "drawing.open", json!({ "path": path }));
            run.drawing = opened["drawing"].as_str().expect("a handle").to_owned();
            assert_eq!(run.entities(&names), shown, "{name}: every bound object by its id");
        }
        let expect = &step["expect"];
        let mut problems = Vec::new();
        for (key, want) in expect["objects"].as_object().into_iter().flatten() {
            let got = run.on("drawing.entity", json!({ "uid": names[key] }));
            differs(&got["entity"], &bind(want, &names), tolerance, key, &mut problems);
        }
        for (key, want) in expect["measures"].as_object().into_iter().flatten() {
            let got = run.on("drawing.measure", json!({ "uid": names[key] }));
            differs(&got, want, tolerance, &format!("{key} ölçü"), &mut problems);
        }
        if let Some(count) = expect.get("count") {
            differs(&json!(run.count()), count, 0.0, "nesne sayısı", &mut problems);
        }
        for (kind, want) in expect["kinds"].as_object().into_iter().flatten() {
            let page = run.on("drawing.entities", json!({ "kinds": [kind], "limit": 1000 }));
            let got = page["items"].as_array().map_or(0, Vec::len);
            differs(&json!(got), want, 0.0, &format!("tür {kind}"), &mut problems);
        }
        if let Some(want) = expect.get("output") {
            differs(&output, want, 1e-9, "cevap", &mut problems);
        }
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
    assert!(undone > 10);
    if let Some(out) = std::env::var_os("KENTOS_ACCEPTANCE_OUT") {
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).expect("the output folder");
        run.on("drawing.save", json!({ "path": out.join("mcp.kcad") }));
    }
    let _ = std::fs::remove_dir_all(dir);
}

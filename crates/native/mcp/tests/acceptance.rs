//! The first end-to-end acceptance drawing through the MCP server (TODOS.md
//! CADP-12, plan/cad.md §23.1 K01–K04, K11, K12): the shared scenario
//! (fixtures/acceptance/v1/room.json, worked out by
//! scripts/fixtures/acceptance_room.py without KentOS code) replayed as a
//! model would replay it, tool call after tool call, every step's objects,
//! measures and sheets held to it, its DXF and PDF read back without KentOS
//! code (the plotter's tests/support/files.rs). Python and the desktop
//! replay the same file.

use std::collections::BTreeMap;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

#[path = "../../plot/tests/support/files.rs"]
mod files;

const SCENARIO: &str = include_str!("../../../../fixtures/acceptance/v1/room.json");

fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
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
    let result = answer["result"].clone();
    assert_eq!(result["isError"], false, "{name}: {answer}");
    result["structuredContent"].clone()
}

/// Where `actual` differs from `expected`: only the fields `expected` names;
/// a point `[x, y]` is `{"x", "y"}` in a drawing; numbers to `tolerance`.
fn differs(actual: &Value, expected: &Value, tolerance: f64, at: &str, out: &mut Vec<String>) {
    if let (Some(e), Some(a)) = (expected.as_array(), actual.as_object())
        && e.len() == 2
        && e.iter().all(Value::is_number)
        && a.contains_key("x")
    {
        let point = json!([a["x"], a["y"]]);
        return differs(&point, expected, tolerance, at, out);
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

fn bind(value: &Value, names: &BTreeMap<String, String>) -> Value {
    match value {
        // `$out:` paths are the caller's to place.
        Value::String(s) if s.starts_with('$') && !s.starts_with("$out:") => {
            json!(names[&s[1..]])
        }
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), bind(v, names)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|v| bind(v, names)).collect()),
        other => other.clone(),
    }
}

#[test]
fn the_room_is_drawn_corrected_dimensioned_and_hatched_as_the_scenario_says() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let tolerance = scenario["tolerance"].as_f64().expect("a tolerance");
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", scenario["project"].clone());
    let mut drawing = made["drawing"].as_str().expect("a handle").to_owned();
    let mut names = BTreeMap::new();
    names.insert(
        "layer".to_owned(),
        made["summary"]["activeLayer"]
            .as_str()
            .expect("a layer")
            .to_owned(),
    );
    // The journal is the session's (docs/adr/0253 §3): taken before every reopening.
    let mut rows: Vec<Value> = Vec::new();
    let dir = std::env::temp_dir().join(format!("kentos-mcp-kabul-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    for (n, step) in scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .enumerate()
    {
        let name = step["name"].as_str().unwrap_or("");
        if let Some(command) = step["command"].as_str() {
            let mut arguments = bind(&step["input"], &names);
            arguments["drawing"] = json!(drawing);
            let answer = call(&mut server, command, arguments);
            assert_eq!(answer["status"], "completed", "{name}: {answer}");
            match &step["as"] {
                Value::String(key) => {
                    names.insert(
                        key.clone(),
                        answer["output"]["uid"].as_str().expect("uid").to_owned(),
                    );
                }
                Value::Array(keys) => {
                    for (key, uid) in keys
                        .iter()
                        .zip(answer["output"]["created"].as_array().expect("created"))
                    {
                        names.insert(
                            key.as_str().expect("a name").to_owned(),
                            uid.as_str().expect("uid").to_owned(),
                        );
                    }
                }
                _ => {}
            }
        } else if let Some(label) = step["undo"].as_str() {
            let undone = call(&mut server, "drawing.undo", json!({ "drawing": drawing }));
            assert_eq!(undone["step"], label, "{name}");
        } else if step["reopen"] == true {
            // Saved to a file and opened as a model would: a new handle on the same drawing.
            let journal = call(
                &mut server,
                "drawing.journal",
                json!({ "drawing": drawing }),
            );
            rows.extend(journal["calls"].as_array().cloned().unwrap_or_default());
            let path = dir.join(format!("oda-{n}.kcad"));
            call(
                &mut server,
                "drawing.save",
                json!({ "drawing": drawing, "path": path }),
            );
            let opened = call(&mut server, "drawing.open", json!({ "path": path }));
            drawing = opened["drawing"].as_str().expect("a handle").to_owned();
        }
        let expect = &step["expect"];
        let mut problems = Vec::new();
        match step["export"].as_str() {
            Some("dxf") => {
                let path = dir.join(format!("oda-{n}.dxf"));
                call(
                    &mut server,
                    "drawing.exportDxf",
                    json!({ "drawing": drawing, "path": path }),
                );
                let bytes = std::fs::read(&path).expect("the DXF");
                problems.extend(files::dxf_problems(&bytes, &expect["dxf"]));
            }
            Some("pdf") => {
                let path = dir.join(format!("oda-{n}.pdf"));
                call(
                    &mut server,
                    "drawing.exportPdf",
                    json!({ "drawing": drawing, "path": path }),
                );
                let bytes = std::fs::read(&path).expect("the PDF");
                problems.extend(files::pdf_problems(&bytes, &expect["pdf"]));
                match files::pdf_text_problems(&path, &expect["pdf"]) {
                    Some(texts) => problems.extend(texts),
                    None => {
                        eprintln!("pdftotext yok: PDF'in yazıları denetlenmedi (poppler kurun).")
                    }
                }
            }
            _ => {}
        }
        if let Some(sheets) = expect.get("sheets") {
            let summary = call(
                &mut server,
                "drawing.summary",
                json!({ "drawing": drawing }),
            );
            differs(
                &summary["sheets"],
                sheets,
                tolerance,
                "paftalar",
                &mut problems,
            );
        }
        for (key, want) in expect["objects"].as_object().into_iter().flatten() {
            let got = call(
                &mut server,
                "drawing.entity",
                json!({ "drawing": drawing, "uid": names[key] }),
            );
            differs(&got["entity"], want, tolerance, key, &mut problems);
        }
        for (key, want) in expect["measures"].as_object().into_iter().flatten() {
            let got = call(
                &mut server,
                "drawing.measure",
                json!({ "drawing": drawing, "uid": names[key] }),
            );
            differs(&got, want, tolerance, &format!("{key} ölçü"), &mut problems);
        }
        if let Some(count) = expect.get("count") {
            let summary = call(
                &mut server,
                "drawing.summary",
                json!({ "drawing": drawing }),
            );
            differs(
                &summary["objects"],
                count,
                0.0,
                "nesne sayısı",
                &mut problems,
            );
        }
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
    // The record says what ran, in order, and replays (docs/adr/0253 §3, §10): on a new drawing
    // of the same project every command row done is the same, the undone ones are not run, the
    // files are planned again (not written) and are the same files.
    let mut journal = call(
        &mut server,
        "drawing.journal",
        json!({ "drawing": drawing }),
    );
    rows.extend(journal["calls"].as_array().cloned().unwrap_or_default());
    let ran: Vec<&str> = scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter_map(|s| s["command"].as_str())
        .collect();
    let recorded: Vec<&str> = rows
        .iter()
        .filter(|r| r["kind"] == "command" && r.get("effects").is_none())
        .filter_map(|r| r["command"].as_str())
        .collect();
    assert_eq!(
        recorded, ran,
        "the record's commands are the scenario's, in order"
    );
    assert!(
        rows.iter().all(|r| r["kind"] == "command"),
        "every write went through a command"
    );
    let done = rows
        .iter()
        .filter(|r| r["kind"] == "command" && r["status"] == "done" && r.get("effects").is_none())
        .count();
    let written = rows.iter().filter(|r| r.get("effects").is_some()).count();
    assert_eq!(written, 2, "the DXF and the PDF");
    journal["calls"] = Value::Array(rows);
    let fresh = call(&mut server, "drawing.new", scenario["project"].clone());
    let fresh = fresh["drawing"].as_str().expect("a handle").to_owned();
    let before: Vec<_> = std::fs::read_dir(&dir)
        .expect("lists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    let report = call(
        &mut server,
        "drawing.replay",
        json!({ "drawing": fresh, "journal": journal }),
    );
    let wrong: Vec<&Value> = report["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter(|s| s["outcome"] == "differs" || s["outcome"] == "failed")
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
    assert_eq!(report["same"], json!(done + written), "{report}");
    let after: Vec<_> = std::fs::read_dir(&dir)
        .expect("lists")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    assert_eq!(before, after, "a replay writes no file");
    let mine = call(
        &mut server,
        "drawing.summary",
        json!({ "drawing": drawing }),
    );
    let theirs = call(&mut server, "drawing.summary", json!({ "drawing": fresh }));
    assert_eq!(theirs["objects"], mine["objects"]);
    let _ = std::fs::remove_dir_all(dir);
}

const GIS_SCENARIO: &str = include_str!("../../../../fixtures/acceptance/v1/parcels.json");

/// A layer's node of the tree by its name.
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

/// The first GIS acceptance project through the MCP server (plan/gis.md CBS-44, docs/adr/0554): the shared scenario
/// (fixtures/acceptance/v1/parcels.json, worked out by scripts/fixtures/acceptance_parcels.py without KentOS code)
/// replayed tool call after tool call: typed layers, an undone value, Alan hesapla, Konuma göre seç, a reopening, five
/// formats written and the GeoPackage opened in a new drawing with nothing lost. Every write is its command's row.
#[test]
fn the_parcels_are_made_checked_written_and_opened_elsewhere_as_the_scenario_says() {
    let scenario: Value = serde_json::from_str(GIS_SCENARIO).expect("the scenario");
    let tolerance = scenario["tolerance"].as_f64().expect("a tolerance");
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", scenario["project"].clone());
    let mut drawing = made["drawing"].as_str().expect("a handle").to_owned();
    let mut names = BTreeMap::new();
    let dir = std::env::temp_dir().join(format!("kentos-mcp-ada-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let out = dir.display().to_string();
    let mut rows: Vec<Value> = Vec::new();
    for (n, step) in scenario["steps"].as_array().expect("steps").iter().enumerate() {
        let name = step["name"].as_str().unwrap_or("");
        if step["fresh"] == true {
            let journal = call(&mut server, "drawing.journal", json!({ "drawing": drawing }));
            rows.extend(journal["calls"].as_array().cloned().unwrap_or_default());
            let fresh = call(&mut server, "drawing.new", scenario["project"].clone());
            drawing = fresh["drawing"].as_str().expect("a handle").to_owned();
        }
        let mut selected = Value::Null;
        if let Some(command) = step["command"].as_str() {
            let text = bind(&step["input"], &names).to_string().replace("$out:", &format!("{out}/"));
            let mut arguments: Value = serde_json::from_str(&text).expect("json");
            arguments["drawing"] = json!(drawing);
            let answer = call(&mut server, command, arguments);
            assert_eq!(answer["status"], "completed", "{name}: {answer}");
            if let Some(key) = step["as"].as_str() {
                names.insert(key.to_owned(), answer["output"]["uid"].as_str().expect("uid").to_owned());
            }
            for (key, field) in step["bind"].as_object().into_iter().flatten() {
                let value = answer["output"][field.as_str().expect("a field")].as_str().expect("bound");
                names.insert(key.clone(), value.to_owned());
            }
            selected = answer["output"]["selected"].clone();
        } else if let Some(label) = step["undo"].as_str() {
            let undone = call(&mut server, "drawing.undo", json!({ "drawing": drawing }));
            assert_eq!(undone["step"], label, "{name}");
        } else if step["reopen"] == true {
            let journal = call(&mut server, "drawing.journal", json!({ "drawing": drawing }));
            rows.extend(journal["calls"].as_array().cloned().unwrap_or_default());
            let path = dir.join(format!("ada-{n}.kcad"));
            call(&mut server, "drawing.save", json!({ "drawing": drawing, "path": path }));
            let opened = call(&mut server, "drawing.open", json!({ "path": path }));
            drawing = opened["drawing"].as_str().expect("a handle").to_owned();
        }
        let expect = &step["expect"];
        let mut problems = Vec::new();
        for (key, want) in expect["objects"].as_object().into_iter().flatten() {
            let got = call(&mut server, "drawing.entity", json!({ "drawing": drawing, "uid": names[key] }));
            differs(&got["entity"], want, tolerance, key, &mut problems);
        }
        for (key, want) in expect["measures"].as_object().into_iter().flatten() {
            let got = call(&mut server, "drawing.measure", json!({ "drawing": drawing, "uid": names[key] }));
            differs(&got, want, tolerance, &format!("{key} ölçü"), &mut problems);
        }
        if let Some(count) = expect.get("count") {
            let summary = call(&mut server, "drawing.summary", json!({ "drawing": drawing }));
            differs(&summary["objects"], count, 0.0, "nesne sayısı", &mut problems);
        }
        if let Some(want) = expect["selected"].as_array() {
            let mut want: Vec<String> = want.iter().filter_map(|k| k.as_str()).map(|k| names[k].clone()).collect();
            let mut got: Vec<String> =
                selected.as_array().into_iter().flatten().filter_map(|u| u.as_str().map(str::to_owned)).collect();
            want.sort();
            got.sort();
            if got != want {
                problems.push(format!("seçilenler {got:?}, beklenen {want:?}"));
            }
        }
        for (layer, spec) in expect["layers"].as_object().into_iter().flatten() {
            let tree = call(&mut server, "drawing.layers", json!({ "drawing": drawing }));
            let Some(id) = layer_named(&tree["layers"], layer).or_else(|| layer_named(&tree, layer)) else {
                problems.push(format!("{layer} katmanı yok"));
                continue;
            };
            let page = call(&mut server, "drawing.entities", json!({ "drawing": drawing, "layer": id }));
            let key = spec["key"].as_str().expect("a key");
            let items = page["items"].as_array().cloned().unwrap_or_default();
            let rows_of = spec["rows"].as_object().expect("rows");
            if items.len() != rows_of.len() {
                problems.push(format!("{layer}: {} nesne, beklenen {}", items.len(), rows_of.len()));
            }
            for item in &items {
                let attrs = &item["entity"]["attrs"];
                let Some(k) = attrs[key].as_str() else {
                    problems.push(format!("{layer}: {key} yok"));
                    continue;
                };
                differs(attrs, &rows_of[k], 0.0, &format!("{layer}/{k}"), &mut problems);
                let m = call(&mut server, "drawing.measure", json!({ "drawing": drawing, "uid": item["uid"] }));
                differs(&m["area"], &spec["areas"][k], tolerance, &format!("{layer}/{k} alan"), &mut problems);
            }
        }
        assert!(problems.is_empty(), "{name}: {problems:#?}");
    }
    let journal = call(&mut server, "drawing.journal", json!({ "drawing": drawing }));
    rows.extend(journal["calls"].as_array().cloned().unwrap_or_default());
    assert!(
        rows.iter().all(|r| r["kind"] == "command"),
        "every write went through a command: {rows:#?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

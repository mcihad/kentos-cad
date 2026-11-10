//! Eğimle kot ver's high point through a real AI host (plan/map-cad.md MAP-06, docs/adr/3380): the shared cases'
//! drawing opened as a .kcad file over MCP; the plan of 3 % ahead and 2 % back on the polyline gives the case's point
//! (on the arc, added, high) and writes nothing; execute adds the vertex, and the saved file holds it with the case's
//! place, elevations and the arc's two halves.
//! Docs/adr/4980: the range on a further part's hole.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"grade-peak-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn close(a: &Value, b: &Value, within: f64) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() <= within,
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q, within)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w, within))),
        _ => a == b,
    }
}

#[test]
fn the_high_point_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.grade.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Yüksek nokta yaylı"))).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-grade-peak-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let want = &step["result"]["output"];
    // Plan: the case's answer, nothing written.
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    args["op"] = json!("plan");
    let r = call(&mut server, "cad.entities.grade", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"];
    assert_eq!((&got["point"]["index"], &got["point"]["inserted"], &got["point"]["kind"]), (&json!(3), &json!(true), &json!("high")));
    for key in ["length", "start", "end", "grade", "back", "zs", "point"] {
        assert!(close(&got[key], &want[key], 1e-6), "{key}: {} != {}", got[key], want[key]);
    }
    // Execute, then the saved file: the polyline as the case leaves it.
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.entities.grade", args);
    assert_eq!(r["isError"], false, "{r}");
    let saved = std::env::temp_dir().join(format!("kentos-grade-peak-saved-{}.kcad", std::process::id()));
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let back = serde_json::to_value(kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap()).unwrap();
    let polyline = back["entities"].as_array().unwrap().iter().find(|e| e["id"] == json!(1)).unwrap();
    let fields = &step["expect"]["fields"]["1"];
    for key in ["pts", "bulges", "zs"] {
        assert!(close(&polyline[key], &fields[key], 1e-6), "{key}: {} != {}", polyline[key], fields[key]);
    }
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(saved);
}

/// Docs/adr/4980: the range on the two-part area's second part's hole (410, 10), (430, 10), (430, 20), (410, 20) over
/// MCP: vertices 1 to 3 from 101 m to 101.6 m along 30 m (2 %), the first vertex and the other rings left; a hole the
/// part has not is refused at `ring`.
#[test]
fn a_range_on_a_further_parts_hole_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.grade.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let uid = cases["setup"]["uids"][5].as_str().unwrap().to_owned();
    let path = std::env::temp_dir().join(format!("kentos-grade-part-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let r = call(
        &mut server,
        "cad.entities.grade",
        json!({"drawing": drawing, "uid": uid, "from": 1, "to": 3, "part": 1, "ring": 1, "start": 101.0, "end": 101.6}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert!(close(&out["grade"], &json!(0.02), 1e-12) && close(&out["length"], &json!(30.0), 1e-9), "{out}");
    let area = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid}));
    let e = &area["structuredContent"]["entity"];
    assert!(close(&e["parts"][0]["holes"][0]["zs"], &json!([null, 101.0, 101.2, 101.6]), 1e-9), "{e}");
    assert!(e["zs"].is_null() && e["parts"][0]["zs"].is_null() && e["holes"][0]["zs"].is_null(), "{e}");
    let r = call(
        &mut server,
        "cad.entities.grade",
        json!({"drawing": drawing, "uid": uid, "from": 0, "to": 1, "part": 1, "ring": 2, "start": 100.0, "grade": 0.01}),
    );
    assert_eq!(r["structuredContent"]["error"]["path"], "ring", "{r}");
}

//! A scenario's links to its nearest infrastructure written as lines through a real AI host (plan/map-cad.md MAP-10,
//! docs/adr/3860): the processing case's drawing (cad.scenarios.impact's case with the sewer line 40 m) opened as a
//! .kcad file over MCP. `cad.processing.run` of Altyapı bağlarını çiz answers the case's summary and step, one new
//! layer and eleven lines with the case's fields and ends; one undo step takes them away.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"scenario-links-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn count(v: &Value) -> usize {
    v.as_array().map_or(0, Vec::len)
}

#[test]
fn the_links_go_to_their_layer_over_mcp_as_the_case_says() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/processing/v1/scenario-links.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["id"] == "scenario-links").unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../fixtures/processing/v1/scenario-links.kcad")).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-scenario-links-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "scenarios.links", "parameters": case["values"]}));
    assert_eq!(ran["isError"], false, "{ran}");
    let out = &ran["structuredContent"]["output"];
    let want = &case["expect"];
    assert_eq!(out["summary"], want["summary"]);
    assert_eq!(out["step"], want["undo"]);
    assert_eq!(count(&out["layers"]), 1, "{out}");
    let added = out["added"].as_array().unwrap().clone();
    let shapes = want["addedShapes"].as_array().unwrap();
    assert_eq!(added.len(), shapes.len());
    for (uid, shape) in added.iter().zip(shapes) {
        let got = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid}));
        let e = &got["structuredContent"]["entity"];
        assert_eq!(e["kind"], "line", "{e}");
        assert_eq!(e["attrs"], shape["attrs"], "{e}");
        for (end, k) in [("a", 0), ("b", 1)] {
            for axis in ["x", "y"] {
                let (a, b) = (e[end][axis].as_f64().unwrap(), shape["vertices"][k][axis].as_f64().unwrap());
                assert!((a - b).abs() < 1e-6, "{end}.{axis}: {a} != {b}");
            }
        }
    }
    let undone = call(&mut server, "drawing.undo", json!({"drawing": drawing}));
    assert_eq!(undone["isError"], false, "{undone}");
    let left = call(&mut server, "drawing.entities", json!({"drawing": drawing}));
    assert_eq!(count(&left["structuredContent"]["items"]), snapshot.entities.len(), "{left}");
    let _ = std::fs::remove_file(path);
}

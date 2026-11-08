//! The topology findings written to an error layer through a real AI host (plan/map-cad.md MAP-07, docs/adr/2200):
//! the processing case's drawing opened as a .kcad file over MCP. `cad.processing.run` of Topolojiyi kurallarla
//! denetle without Bulguları hata katmanına yaz selects the findings' objects and adds nothing. With it, the case's
//! summary, one new layer and three new points carrying the case's fields (the objects' lasting ids, the findings'
//! keys) at the case's places. One undo step takes them away.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"topology-errors-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// A list's length; an empty list is left out of the answer.
fn count(v: &Value) -> usize {
    v.as_array().map_or(0, Vec::len)
}

#[test]
fn the_findings_go_to_the_error_layer_over_mcp_as_the_case_says() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/processing/v1/geometry.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["id"] == "topology-check-errors").unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../fixtures/processing/v1/topology.kcad")).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-topology-errors-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    // Without the box: the findings' objects chosen, nothing written.
    let read = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "topology.check", "parameters": {}}));
    assert_eq!(read["isError"], false, "{read}");
    let out = &read["structuredContent"]["output"];
    assert_eq!(count(&out["added"]), 0, "{out}");
    assert_eq!(count(&out["selected"]), 5, "{out}");

    // With it: the case's summary, its layer and its points.
    let args = json!({"drawing": drawing, "tool": "topology.check", "parameters": case["values"]});
    let ran = call(&mut server, "cad.processing.run", args);
    assert_eq!(ran["isError"], false, "{ran}");
    let out = &ran["structuredContent"]["output"];
    let want = &case["expect"];
    assert_eq!(out["summary"], want["summary"]);
    assert_eq!(out["step"], want["undo"]);
    assert_eq!(count(&out["layers"]), 1, "{out}");
    assert_eq!(count(&out["selected"]), 0, "{out}");
    let added = out["added"].as_array().unwrap().clone();
    let shapes = want["addedShapes"].as_array().unwrap();
    assert_eq!(added.len(), shapes.len());
    for (uid, shape) in added.iter().zip(shapes) {
        let got = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid}));
        let e = &got["structuredContent"]["entity"];
        assert_eq!(e["kind"], "point", "{e}");
        assert_eq!(e["attrs"], shape["attrs"], "{e}");
        for key in ["x", "y"] {
            let (a, b) = (e["p"][key].as_f64().unwrap(), shape["points"][0][key].as_f64().unwrap());
            assert!((a - b).abs() < 1e-6, "{key}: {a} != {b}");
        }
    }

    // One undo step.
    let undone = call(&mut server, "drawing.undo", json!({"drawing": drawing}));
    assert_eq!(undone["isError"], false, "{undone}");
    let left = call(&mut server, "drawing.entities", json!({"drawing": drawing}));
    assert_eq!(left["structuredContent"]["items"].as_array().unwrap().len(), 5, "{left}");
    let _ = std::fs::remove_file(path);
}

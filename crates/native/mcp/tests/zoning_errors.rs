//! The zoning check's road regions written to an error layer through a real AI host (plan/map-cad.md MAP-09,
//! docs/adr/3840): the processing case's drawing (cad.zoning.check's case with four decisions and three parcels in a
//! wide road) opened as a .kcad file over MCP. `cad.processing.run` of İmar planını denetle without Bulguları hata
//! katmanına yaz selects the findings' objects and adds nothing; with it, the case's summary and step, one new layer
//! and seven areas carrying the case's fields. One undo step takes them away.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"zoning-errors-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn count(v: &Value) -> usize {
    v.as_array().map_or(0, Vec::len)
}

#[test]
fn the_road_regions_go_to_the_error_layer_over_mcp_as_the_case_says() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/processing/v1/zoning.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["id"] == "zoning-check-errors").unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../fixtures/processing/v1/zoning-yollar.kcad")).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-zoning-errors-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    // Without the box: the findings' objects chosen, nothing written.
    let read = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "zoning.check", "parameters": {}}));
    assert_eq!(read["isError"], false, "{read}");
    let out = &read["structuredContent"]["output"];
    assert_eq!(count(&out["added"]), 0, "{out}");
    assert_eq!(count(&out["selected"]), 8, "{out}");

    // With it: the case's summary, its layer and its areas.
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "zoning.check", "parameters": case["values"]}));
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
        assert_eq!(e["kind"], "polygon", "{e}");
        assert_eq!(e["attrs"], shape["attrs"], "{e}");
    }
    // One undo step: the drawing's objects as they were.
    let undone = call(&mut server, "drawing.undo", json!({"drawing": drawing}));
    assert_eq!(undone["isError"], false, "{undone}");
    let left = call(&mut server, "drawing.entities", json!({"drawing": drawing}));
    assert_eq!(count(&left["structuredContent"]["items"]), snapshot.entities.len(), "{left}");
    let _ = std::fs::remove_file(path);
}

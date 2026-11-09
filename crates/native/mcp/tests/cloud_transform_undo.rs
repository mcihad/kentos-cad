//! A cloud's last transform taken back through a real AI host (plan/map-cad.md MAP-13, docs/adr/3900): the processing
//! cases' drawing and clouds in a scratch folder over MCP. `pointcloud.transform` with the case's rotated matrix writes
//! a cloud with one step of history; Son dönüşümü geri al on it writes a cloud with the first file and no history left,
//! as the shared case of the one-step history says; on a cloud without a history it is refused with the case's words.
use std::path::PathBuf;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"cloud-transform-undo-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/processing/v1")
}

#[test]
fn the_last_step_is_taken_back_and_leaves_the_history() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/processing/v1/pointcloud.json")).unwrap();
    let case = |id: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["id"] == id).unwrap().clone();
    let dir = std::env::temp_dir().join(format!("kentos-cloud-undo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixtures().join("pointcloud.kcad"), dir.join("pointcloud.kcad")).unwrap();
    for (name, rel) in cases["rasters"].as_object().unwrap() {
        std::fs::copy(fixtures().join(rel.as_str().unwrap()), dir.join(name)).unwrap();
    }
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": dir.join("pointcloud.kcad")}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let entity = |s: &mut Server, uid: &Value| call(s, "drawing.entity", json!({"drawing": drawing, "uid": uid}))["structuredContent"]["entity"].clone();

    // Moved: one step of history.
    let rotated = case("transform-rotated");
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": rotated["values"]}));
    assert_eq!(ran["isError"], false, "{ran}");
    let moved = entity(&mut server, &ran["structuredContent"]["output"]["added"][0]);

    // Taken back: the first file, no history left; the summary and the count the one-step case's.
    let undo = case("transform-undo-last");
    let ran = call(
        &mut server,
        "cad.processing.run",
        json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": {"input": {"scope": "layer", "layerId": moved["layerId"]}, "undoLast": true, "output": "geri-alinan"}}),
    );
    assert_eq!(ran["isError"], false, "{ran}");
    let out = &ran["structuredContent"]["output"];
    assert_eq!(out["outputs"]["count"], undo["expect"]["outputs"]["count"], "{out}");
    let back = entity(&mut server, &out["added"][0]);
    assert_eq!(back["attrs"], json!({"Kaynak": "koy.laz"}), "{back}");

    // No history: refused with the case's words.
    let none = case("transform-undo-none");
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": none["values"]}));
    let said = ran.to_string();
    assert!(said.contains(none["expect"]["message"].as_str().unwrap()), "{said}");
    let _ = std::fs::remove_dir_all(dir);
}

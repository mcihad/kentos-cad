//! Dönüşüm uygula through a real AI host (plan/map-cad.md MAP-13, docs/adr/3100): the processing cases' drawing and
//! clouds copied into a scratch folder and opened over MCP. `cad.processing.run` of `pointcloud.transform` with the
//! shared case's matrix writes the moved cloud: the case's summary, outputs and notes, one undo step, one new layer and
//! the new cloud object with the case's box; a matrix that takes the cloud off its grid is refused and writes nothing.
use std::path::PathBuf;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"cloud-transform-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/processing/v1")
}

#[test]
fn the_moved_cloud_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/processing/v1/pointcloud.json")).unwrap();
    let case = |id: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["id"] == id).unwrap().clone();
    let dir = std::env::temp_dir().join(format!("kentos-cloud-transform-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixtures().join("pointcloud.kcad"), dir.join("pointcloud.kcad")).unwrap();
    for (name, rel) in cases["rasters"].as_object().unwrap() {
        std::fs::copy(fixtures().join(rel.as_str().unwrap()), dir.join(name)).unwrap();
    }
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": dir.join("pointcloud.kcad")}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    // Off the grid: refused, nothing written.
    let far = case("transform-off-grid");
    let refused = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": far["values"]}));
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(refused.to_string().contains(far["expect"]["message"].as_str().unwrap()), "{refused}");
    assert!(!dir.join("koy-donusmus.laz").exists(), "nothing written");

    // The rotated and shifted matrix: the case's file, summary, outputs, notes and object.
    let rotated = case("transform-rotated");
    let want = &rotated["expect"];
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": rotated["values"]}));
    assert_eq!(ran["isError"], false, "{ran}");
    let out = &ran["structuredContent"]["output"];
    let file = dir.join("koy-donusmus.laz");
    let written = file.to_string_lossy().into_owned();
    assert_eq!(out["summary"].as_str().unwrap().replace(&written, "koy-donusmus.laz"), want["summary"]);
    assert_eq!(out["outputs"], want["outputs"]);
    assert_eq!(out["notes"], want["log"]);
    assert_eq!(out["step"], want["undo"]);
    assert_eq!(out["layers"].as_array().map(Vec::len), Some(1));
    let added = out["added"].as_array().unwrap().clone();
    assert_eq!(added.len(), 1);
    let got = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": added[0]}));
    let e = &got["structuredContent"]["entity"];
    assert_eq!(e["kind"], "pointcloud", "{e}");
    assert_eq!(e["count"], want["added"][0]["count"], "{e}");
    for (a, b) in e["bounds"].as_array().unwrap().iter().zip(want["added"][0]["bounds"].as_array().unwrap()) {
        assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-9, "{a} != {b}");
    }
    assert!(std::fs::metadata(&file).unwrap().len() > 1000, "the LAZ is there");
    let _ = std::fs::remove_dir_all(dir);
}

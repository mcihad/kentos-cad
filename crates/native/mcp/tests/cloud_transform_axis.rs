//! A cloud turned by an axis and an angle through a real AI host (plan/map-cad.md MAP-13, docs/adr/4300): the
//! processing cases' drawing and clouds in a scratch folder over MCP. `pointcloud.transform` with Eksen ve açı (Z, 0.5°,
//! the cloud's middle, a move) writes the case's cloud: the count and the summary are the case's and the history keeps
//! the matrix the case's reference made; the axis of no length is refused with the case's words.
use std::path::PathBuf;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"cloud-transform-axis-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/processing/v1")
}

#[test]
fn an_axis_and_an_angle_move_the_cloud_as_the_case_does() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/processing/v1/pointcloud.json")).unwrap();
    let case = |id: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["id"] == id).unwrap().clone();
    let dir = std::env::temp_dir().join(format!("kentos-cloud-axis-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixtures().join("pointcloud.kcad"), dir.join("pointcloud.kcad")).unwrap();
    for (name, rel) in cases["rasters"].as_object().unwrap() {
        std::fs::copy(fixtures().join(rel.as_str().unwrap()), dir.join(name)).unwrap();
    }
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": dir.join("pointcloud.kcad")}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let turned = case("transform-axis-z");
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": turned["values"]}));
    assert_eq!(ran["isError"], false, "{ran}");
    let out = &ran["structuredContent"]["output"];
    assert_eq!(out["outputs"]["count"], turned["expect"]["outputs"]["count"], "{out}");
    // The summary's words up to the file, whose path here is the scratch folder's.
    let head = |s: &Value| s.as_str().unwrap().split('“').next().unwrap().to_owned();
    assert_eq!(head(&out["summary"]), head(&turned["expect"]["summary"]), "{out}");
    let added = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": out["added"][0]}))["structuredContent"]["entity"].clone();
    assert_eq!(added["attrs"], turned["expect"]["added"][0]["attrs"], "{added}");

    let zero = case("transform-axis-zero");
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": zero["values"]}));
    let said = ran.to_string();
    assert!(said.contains(zero["expect"]["message"].as_str().unwrap()), "{said}");
    let _ = std::fs::remove_dir_all(dir);
}

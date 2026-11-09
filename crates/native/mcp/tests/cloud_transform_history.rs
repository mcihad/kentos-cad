//! A cloud moved and moved back through a real AI host (plan/map-cad.md MAP-13, docs/adr/3500): the processing cases'
//! drawing and clouds in a scratch folder over MCP. `pointcloud.transform` with the case's rotated matrix writes a cloud
//! whose history is the case's (Kaynak koy.laz, Dönüşümler the matrix); the same matrix with Tersini uygula on that
//! cloud writes a third one whose history keeps the first file and both matrices, the inverse last (its numbers the
//! shared case's, which checks the inverse's points bit for bit).
use std::path::PathBuf;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"cloud-transform-history-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/processing/v1")
}

#[test]
fn moved_and_moved_back_the_history_grows() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/processing/v1/pointcloud.json")).unwrap();
    let case = |id: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["id"] == id).unwrap().clone();
    let dir = std::env::temp_dir().join(format!("kentos-cloud-history-{}", std::process::id()));
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

    // Moved: the case's history.
    let rotated = case("transform-rotated");
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": rotated["values"]}));
    assert_eq!(ran["isError"], false, "{ran}");
    let moved_uid = ran["structuredContent"]["output"]["added"][0].clone();
    let moved = entity(&mut server, &moved_uid);
    assert_eq!(moved["attrs"], rotated["expect"]["added"][0]["attrs"], "{moved}");
    let first = moved["attrs"]["Dönüşümler"].as_str().unwrap().to_owned();

    // Moved back from the moved cloud: the first file kept, both matrices, the inverse's as the case writes it.
    let inverse = case("transform-inverse");
    let mut back = rotated["values"].clone();
    back["input"] = json!({"scope": "layer", "layerId": moved["layerId"]});
    back["invert"] = json!(true);
    back["output"] = json!("geri-donmus");
    let ran = call(&mut server, "cad.processing.run", json!({"drawing": drawing, "tool": "pointcloud.transform", "parameters": back}));
    assert_eq!(ran["isError"], false, "{ran}");
    let again = entity(&mut server, &ran["structuredContent"]["output"]["added"][0]);
    assert_eq!(again["attrs"]["Kaynak"], json!("koy.laz"), "{again}");
    let undone = inverse["expect"]["added"][0]["attrs"]["Dönüşümler"].as_str().unwrap();
    assert_eq!(again["attrs"]["Dönüşümler"], json!(format!("{first} | {undone}")), "{again}");
    let _ = std::fs::remove_dir_all(dir);
}

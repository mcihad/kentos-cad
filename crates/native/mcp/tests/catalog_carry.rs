//! Kataloğa taşı through a real AI host (plan/map-cad.md MAP-04, docs/adr/5340): a drawing with the map assets
//! catalog's layers made over MCP (cad.profiles.apply), a control point on its layer, then carried to an institution's
//! catalog whose GNSS points' class takes the KN code: the point goes to the new class's layer, made with its fields; the
//! same catalog on both sides has nothing to carry.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"catalog-carry-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_control_point_carried_to_the_new_catalog_over_mcp() {
    let mut server = Server::new();
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/drawing.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "cad.profiles.apply", json!({"drawing": drawing, "name": "harita-varliklari"}));
    assert_eq!(r["isError"], false, "{r}");
    let layer = r["structuredContent"]["output"]["layers"].as_array().unwrap().iter().find_map(|l| l.as_str()).map(str::to_owned);
    assert!(layer.is_some(), "{r}");
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.profiles.carry.json")).expect("cases");
    let catalog = cases["cases"][0]["steps"][2]["input"]["toProfile"].clone();
    // A control point on the control points' layer (the catalog's first class).
    let r = call(&mut server, "cad.profiles.check", json!({"drawing": drawing, "name": "harita-varliklari"}));
    let kn = r["structuredContent"]["output"]["classes"][0]["layer"].as_str().expect("the layer").to_owned();
    let r = call(&mut server, "cad.entities.create", json!({"drawing": drawing, "layerId": kn,
        "objects": [{"geometry": {"kind": "point", "p": {"x": 500001.0, "y": 4400001.0}}, "attrs": {"Kod": "KN"}}]}));
    assert_eq!(r["isError"], false, "{r}");
    let r = call(&mut server, "cad.profiles.carry", json!({"drawing": drawing, "from": "harita-varliklari", "toProfile": catalog}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["classes"][0]["moved"], 1, "{out}");
    assert!(!out["layers"].as_array().expect("made").is_empty(), "{out}");
    let r = call(&mut server, "cad.profiles.carry", json!({"drawing": drawing, "from": "harita-varliklari", "to": "harita-varliklari"}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("nothing_to_set"), "{r}");
}

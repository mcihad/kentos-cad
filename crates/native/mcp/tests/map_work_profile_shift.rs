//! A map work item's typed layers' profile and how far a changed object moved, through a real AI host (plan/map-cad.md
//! MAP-01, docs/adr/4880): the `.kcad` schema 211 sample (fixtures/kcad/v2/map-work-approved-parts.kcad) opened over
//! MCP. The design takes the map assets profile's control points: the class's layer is added and joins the item, the
//! query names the profile; then approved again and its line moved 0.3 m east and 0.4 m north: both ends moved 0.5 m.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"map-work-profile-shift-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const LINE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13";

fn opened(server: &mut Server) -> Value {
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/map-work-approved-parts.kcad");
    let opened = call(server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    opened["structuredContent"]["drawing"].clone()
}

fn design(server: &mut Server, drawing: &Value) -> Value {
    let r = call(server, "map.work.query", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    r["structuredContent"]["output"]["items"][2].clone()
}

#[test]
fn an_item_s_profile_and_a_moved_line_s_shift_over_mcp() {
    let mut server = Server::new();
    let drawing = opened(&mut server);
    let r = call(
        &mut server,
        "map.work.edit",
        json!({"drawing": drawing, "operation": {"op": "profile", "item": "is-3", "name": "harita-varliklari", "classes": ["kontrol-noktasi"]}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let added = r["structuredContent"]["output"]["layers"].as_array().cloned().unwrap_or_default();
    assert_eq!(added.len(), 2, "the group and the control points' layer: {r}");
    let row = design(&mut server, &drawing);
    assert_eq!(row["profile"]["name"], "harita-varliklari", "{row}");
    assert_eq!(row["profile"]["found"], true, "{row}");
    assert!(row["layers"].as_array().unwrap().contains(&added[1]), "the class's layer joined the item: {row}");

    let r = call(&mut server, "map.work.edit", json!({"drawing": drawing, "operation": {"op": "set", "item": "is-3", "status": "approved"}}));
    assert_eq!(r["isError"], false, "{r}");
    let r = call(
        &mut server,
        "cad.entities.transform",
        json!({"drawing": drawing, "uids": [LINE], "transform": {"kind": "move", "dx": 0.3, "dy": 0.4}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let row = design(&mut server, &drawing);
    let changed = row["changes"]["objects"].as_array().unwrap().iter().find(|o| o["uid"] == LINE).cloned().expect("moved");
    let shift = &changed["shift"];
    assert_eq!(shift["paired"], true, "{changed}");
    assert!((shift["largest"]["d"].as_f64().unwrap() - 0.5).abs() < 1e-9, "{changed}");
    assert!((shift["rms"].as_f64().unwrap() - 0.5).abs() < 1e-9, "{changed}");
}

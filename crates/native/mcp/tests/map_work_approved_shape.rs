//! A changed geometry's object as approved through a real AI host (plan/map-cad.md MAP-01, docs/adr/4480): the `.kcad`
//! schema 211 sample (fixtures/kcad/v2/map-work-approved-parts.kcad) opened over MCP, its design approved again (so the
//! approval keeps its objects' values). The line moved, `map.work.query` gives it as approved: its ends where they were
//! before the move; its height set alone, no shape.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"map-work-approved-shape-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const LINE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13";

fn opened(server: &mut Server) -> Value {
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/map-work-approved-parts.kcad");
    let opened = call(server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let r = call(server, "map.work.edit", json!({"drawing": drawing, "operation": {"op": "set", "item": "is-3", "status": "approved"}}));
    assert_eq!(r["isError"], false, "{r}");
    drawing
}

fn line_change(server: &mut Server, drawing: &Value) -> Value {
    let r = call(server, "map.work.query", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    let objects = r["structuredContent"]["output"]["items"][2]["changes"]["objects"].clone();
    objects.as_array().unwrap().iter().find(|o| o["uid"] == LINE).cloned().expect("the line changed")
}

#[test]
fn a_moved_line_s_approved_shape_over_mcp() {
    // The line's ends as the sample keeps them.
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/map-work-approved-parts.kcad");
    let read = serde_json::to_value(kentos_kcad::decode(&std::fs::read(sample).unwrap()).unwrap()).unwrap();
    let at = read["uids"].as_array().unwrap().iter().position(|u| u == LINE).expect("the line");
    let before = read["entities"][at].clone();
    let mut server = Server::new();
    let drawing = opened(&mut server);
    let r = call(
        &mut server,
        "cad.entities.transform",
        json!({"drawing": drawing, "uids": [LINE], "transform": {"kind": "move", "dx": 0.25, "dy": -0.1}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let changed = line_change(&mut server, &drawing);
    assert_eq!(changed["parts"], json!(["geometry"]), "{changed}");
    let approved = &changed["approved"];
    assert_eq!(approved["kind"], "line", "{changed}");
    for end in ["a", "b"] {
        assert_eq!(approved[end], before[end], "{end} as before the move");
    }

    // Its height alone: no shape.
    let mut server = Server::new();
    let drawing = opened(&mut server);
    let r = call(&mut server, "cad.entities.set", json!({"drawing": drawing, "uids": [LINE], "attrs": {"Kot": "1060"}, "operation": "attributes"}));
    assert_eq!(r["isError"], false, "{r}");
    let changed = line_change(&mut server, &drawing);
    assert_eq!(changed["parts"], json!(["attributes"]), "{changed}");
    assert!(changed.get("approved").is_none(), "{changed}");
}

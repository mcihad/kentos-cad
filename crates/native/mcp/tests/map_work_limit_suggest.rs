//! A changed object's shift against a limit and the profiles suggested for an item's kind, through a real AI host
//! (plan/map-cad.md MAP-01, docs/adr/5280): the `.kcad` schema 211 sample opened over MCP. The design, without a profile,
//! is offered the zoning profile; then approved again and its line moved 0.3 m east and 0.4 m north (both ends 0.5 m):
//! beyond the table's sayısal 1/1000 (0.09 m), both ends; within the drawing's ±0.2 mm at 1/5000 (1 m). A cell the
//! table leaves empty is refused at its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"map-work-limit-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const LINE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13";

fn query(server: &mut Server, args: Value) -> Value {
    let r = call(server, "map.work.query", args);
    assert_eq!(r["isError"], false, "{r}");
    r["structuredContent"]["output"].clone()
}

#[test]
fn a_limit_and_the_suggested_profile_over_mcp() {
    let mut server = Server::new();
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/map-work-approved-parts.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let tree = query(&mut server, json!({"drawing": drawing, "suggest": true}));
    let design = &tree["items"][2];
    assert_eq!(design["kind"], "design", "{design}");
    assert_eq!(design["suggested"][0]["name"], "imar-plani", "{design}");

    let r = call(&mut server, "map.work.edit", json!({"drawing": drawing, "operation": {"op": "set", "item": "is-3", "status": "approved"}}));
    assert_eq!(r["isError"], false, "{r}");
    let r = call(
        &mut server,
        "cad.entities.transform",
        json!({"drawing": drawing, "uids": [LINE], "transform": {"kind": "move", "dx": 0.3, "dy": 0.4}}),
    );
    assert_eq!(r["isError"], false, "{r}");

    let tree = query(&mut server, json!({"drawing": drawing, "limit": {"basis": "method", "method": "sayisal", "scale": 1000}}));
    assert_eq!(tree["limit"]["metres"], 0.09, "{tree}");
    let row = &tree["items"][2];
    let shift = row["changes"]["objects"].as_array().unwrap().iter().find(|o| o["uid"] == LINE).expect("moved")["shift"].clone();
    assert_eq!((shift["beyond"].clone(), shift["over"].clone()), (json!(true), json!(2)), "{shift}");
    assert_eq!(row["changes"]["beyond"], 1, "{row}");

    let tree = query(&mut server, json!({"drawing": drawing, "limit": {"basis": "drawing", "scale": 5000}}));
    let row = &tree["items"][2];
    assert_eq!(row["changes"]["beyond"], 0, "{row}");

    let r = call(&mut server, "map.work.query", json!({"drawing": drawing, "limit": {"basis": "method", "method": "fotoplan", "scale": 500}}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("limit.scale"), "{r}");
}

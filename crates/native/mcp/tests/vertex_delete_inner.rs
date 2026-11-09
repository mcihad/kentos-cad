//! Köşe sil with Topolojik düzenleme through a real AI host (plan/map-cad.md MAP-07, docs/adr/4200): the shared cases'
//! drawing (fixtures/commands/v1/cad.entities.vertex.inner.json) opened as a .kcad file over MCP. Removing the road R's
//! middle vertex joins its two edges (40 m and 20 m); the branch T2's end, 20 m along them, goes to 20/60 of the joined
//! edge (worked out here); without Topolojik düzenleme it stays.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"vertex-delete-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn opened(server: &mut Server, tag: &str) -> (Value, Vec<String>) {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.vertex.inner.json")).unwrap();
    let mut setup = cases["setup"].clone();
    let count = setup["entities"].as_array().unwrap().len();
    let uids: Vec<String> = (1..=count).map(|i| format!("0192f5c0-4200-7000-8000-{i:012x}")).collect();
    setup["version"] = json!(2);
    setup["uids"] = json!(uids);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-vertex-delete-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let open = call(server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    (open["structuredContent"]["drawing"].clone(), uids)
}

fn branch_end(server: &mut Server, drawing: &Value, uid: &str) -> (f64, f64) {
    let e = call(server, "drawing.entity", json!({"drawing": drawing, "uid": uid}));
    let b = &e["structuredContent"]["entity"]["b"];
    (b["x"].as_f64().unwrap() - 487_000.0, b["y"].as_f64().unwrap() - 4_420_000.0)
}

#[test]
fn a_lost_corner_s_edges_carry_the_branch_over_mcp() {
    let mut server = Server::new();
    let place = json!({"kind": "vertex", "part": 0, "ring": 0, "index": 1});
    let (drawing, uids) = opened(&mut server, "acik");
    let r = call(&mut server, "cad.entities.vertex", json!({"drawing": drawing, "uid": uids[14], "action": "remove", "place": place, "topology": {}}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["output"]["ids"], json!([15, 16]), "{r}");
    // (0, −40) to (40, −60), 20 of the 60 m: (40/3, −40 − 20/3).
    let (x, y) = branch_end(&mut server, &drawing, &uids[15]);
    assert!((x - 40.0 / 3.0).abs() < 1e-6 && (y - (-40.0 - 20.0 / 3.0)).abs() < 1e-6, "{x} {y}");

    let (drawing, uids) = opened(&mut server, "kapali");
    let r = call(&mut server, "cad.entities.vertex", json!({"drawing": drawing, "uid": uids[14], "action": "remove", "place": place}));
    assert_eq!(r["isError"], false, "{r}");
    let (x, y) = branch_end(&mut server, &drawing, &uids[15]);
    assert!((x - 20.0).abs() < 1e-9 && (y + 40.0).abs() < 1e-9, "stays: {x} {y}");
}

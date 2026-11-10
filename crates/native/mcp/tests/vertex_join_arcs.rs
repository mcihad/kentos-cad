//! Köşeyi sil joining two arcs of one circle through a real AI host (plan/map-cad.md MAP-07, docs/adr/5000): the shared
//! cases' drawing (fixtures/commands/v1/cad.entities.vertex.json) opened as a .kcad file over MCP. The polyline 19 runs
//! by two arcs of one circle, of bulges 1/2 and 1/4; with `joinArcs` its middle vertex goes and one arc of both sweeps
//! remains, tan((4·atan ½ + 4·atan ¼)/4) worked out here; the neighbour 23 along the same arcs the other way takes its
//! negative. The polyline 20's second arc is of another circle: the edge is straight, with `arcs_not_joined`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"vertex-join-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn opened(server: &mut Server, tag: &str) -> (Value, Vec<String>) {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.vertex.json")).unwrap();
    let mut setup = cases["setup"].clone();
    let count = setup["entities"].as_array().unwrap().len();
    let uids: Vec<String> = (1..=count).map(|i| format!("0192f5c0-5000-7000-8000-{i:012x}")).collect();
    setup["version"] = json!(2);
    setup["uids"] = json!(uids);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-vertex-join-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let open = call(server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    (open["structuredContent"]["drawing"].clone(), uids)
}

fn bulges(server: &mut Server, drawing: &Value, uid: &str) -> Vec<f64> {
    let e = call(server, "drawing.entity", json!({"drawing": drawing, "uid": uid}));
    e["structuredContent"]["entity"]["bulges"]
        .as_array()
        .map(|b| b.iter().map(|v| v.as_f64().unwrap()).collect())
        .unwrap_or_default()
}

#[test]
fn two_arcs_of_one_circle_become_one_over_mcp() {
    let mut server = Server::new();
    let place = json!({"kind": "vertex", "part": 0, "ring": 0, "index": 1});
    let want = ((4.0 * 0.5f64.atan() + 4.0 * 0.25f64.atan()) / 4.0).tan();
    let (drawing, uids) = opened(&mut server, "bir");
    let r = call(
        &mut server,
        "cad.entities.vertex",
        json!({"drawing": drawing, "uid": uids[18], "action": "remove", "place": place, "joinArcs": 0.001, "topology": {}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["output"]["ids"], json!([19, 23]), "{r}");
    let own = bulges(&mut server, &drawing, &uids[18]);
    assert!((own[0] - want).abs() < 1e-12, "{own:?} {want}");
    let theirs = bulges(&mut server, &drawing, &uids[22]);
    assert!((theirs[0] + want).abs() < 1e-12, "{theirs:?}");

    let (drawing, uids) = opened(&mut server, "ayri");
    let r = call(
        &mut server,
        "cad.entities.vertex",
        json!({"drawing": drawing, "uid": uids[19], "action": "remove", "place": place, "joinArcs": 0.001}),
    );
    assert_eq!(r["isError"], false, "{r}");
    assert!(r.to_string().contains("arcs_not_joined"), "{r}");
    assert!(bulges(&mut server, &drawing, &uids[19]).iter().all(|b| *b == 0.0));
}

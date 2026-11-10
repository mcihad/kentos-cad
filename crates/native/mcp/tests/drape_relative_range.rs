//! Yüzeyden kot ver's height over the surface and range through a real AI host (plan/map-cad.md MAP-06,
//! docs/adr/4580): the shared cases' drawing (fixtures/commands/v1/cad.entities.drape.json: the surface “Zemin”, its
//! diagonal AC) opened as a .kcad file over MCP. The point 0.15 m over its triangle's plane (worked out here from ABC);
//! the polyline's range 0–1 with break points takes its two vertices and the vertex on AC, its last vertex none; a range
//! for two objects is refused at its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"drape-relative-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn drape_over_and_within_a_range_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.drape.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let uid = |slot: usize| cases["setup"]["uids"][slot - 1].as_str().unwrap().to_owned();
    let path = std::env::temp_dir().join(format!("kentos-drape-relative-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();

    let r = call(&mut server, "cad.entities.drape", json!({"drawing": drawing, "uids": [uid(5)], "surface": "Zemin", "offset": 0.15}));
    assert_eq!(r["isError"], false, "{r}");
    // ABC: z = 100 + 0.04 x + 0.1 y at (60, 20), 0.15 m over it.
    let point = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid(5)}));
    let z = point["structuredContent"]["entity"]["z"].as_f64().unwrap();
    assert!((z - (100.0 + 0.04 * 60.0 + 0.1 * 20.0 + 0.15)).abs() < 1e-9, "{z}");

    let r = call(
        &mut server,
        "cad.entities.drape",
        json!({"drawing": drawing, "uids": [uid(8)], "surface": "t-1", "breaks": true, "range": {"from": 0, "to": 1}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["set"].as_u64(), out["inserted"].as_u64()), (Some(2), Some(1)), "{out}");
    let line = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid(8)}));
    let zs = line["structuredContent"]["entity"]["zs"].as_array().unwrap().clone();
    assert_eq!(zs.len(), 4);
    assert!(zs[3].is_null(), "the last vertex is out of the range: {zs:?}");

    let r = call(
        &mut server,
        "cad.entities.drape",
        json!({"drawing": drawing, "uids": [uid(8), uid(9)], "surface": "t-1", "range": {"from": 0, "to": 1}}),
    );
    assert_eq!(r["structuredContent"]["error"]["path"], "range", "{r}");
}

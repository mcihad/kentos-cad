//! Parsel kayıtları through a real AI host (plan/map-cad.md MAP-08, docs/adr/4220): the shared cases' drawing
//! (fixtures/commands/v1/cad.parcels.record.json: Ada 101's areas) opened as a .kcad file over MCP. Kadastro 1985
//! records 101/7 and 101/8, which share one boundary by its number; 2026/145 ifraz closes 101/7 and makes 101/13 and
//! 101/14 from its halves. The register read back (with its history) names the parcels before and after through the
//! documents, worked out here; one undo takes the ifraz back.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"parcel-register-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn parcel_register_over_mcp() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.record.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let uid = |slot: usize| cases["setup"]["uids"][slot - 1].as_str().unwrap().to_owned();
    let path = std::env::temp_dir().join(format!("kentos-parcel-register-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();

    // A1 (101/7) and A2 (101/8): 4 + 3 boundaries, the edge between them once.
    let first = call(
        &mut server,
        "cad.parcels.record",
        json!({"drawing": drawing, "document": {"name": "Kadastro 1985"}, "create": [{"uid": uid(1)}, {"uid": uid(2)}]}),
    );
    assert_eq!(first["isError"], false, "{first}");
    let out = &first["structuredContent"]["output"];
    assert_eq!((out["document"].as_u64(), out["boundariesCreated"].as_u64()), (Some(1), Some(7)), "{out}");

    // The ifraz: A11 and A12 (slots 11, 12) from 101/7's halves; its west and east boundaries go on.
    let ifraz = call(
        &mut server,
        "cad.parcels.record",
        json!({"drawing": drawing, "document": {"name": "2026/145 ifraz"}, "create": [{"uid": uid(11)}, {"uid": uid(12)}], "close": [1]}),
    );
    assert_eq!(ifraz["isError"], false, "{ifraz}");
    let out = &ifraz["structuredContent"]["output"];
    assert_eq!(out["created"], json!([3, 4]), "{out}");
    assert_eq!(out["boundariesShared"].as_u64(), Some(2), "{out}");

    let listed = call(&mut server, "cad.parcels.records", json!({"drawing": drawing, "history": true}));
    assert_eq!(listed["isError"], false, "{listed}");
    let parcels = listed["structuredContent"]["output"]["parcels"].as_array().unwrap().clone();
    let row = |n: u64| parcels.iter().find(|p| p["number"].as_u64() == Some(n)).unwrap().clone();
    assert_eq!((row(1)["state"].as_str(), row(1)["after"].clone()), (Some("closed"), json!([3, 4])));
    assert_eq!(row(3)["before"], json!([1]));
    // 101/14 (4) shares the east boundary with 101/8 (2) and the middle one with 101/13 (3).
    assert_eq!(row(4)["neighbours"], json!([2, 3]));
    assert!((row(3)["recordedArea"].as_f64().unwrap() - 100.0).abs() < 1e-9);

    let undo = call(&mut server, "drawing.undo", json!({"drawing": drawing}));
    assert_eq!(undo["isError"], false, "{undo}");
    let listed = call(&mut server, "cad.parcels.records", json!({"drawing": drawing}));
    let docs = listed["structuredContent"]["output"]["documents"].as_array().unwrap().len();
    assert_eq!(docs, 1, "{listed}");
}

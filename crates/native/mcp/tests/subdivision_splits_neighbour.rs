//! İfrazda komşunun sınırının bölünmesi through a real AI host (plan/map-cad.md MAP-08, docs/adr/4620): the shared
//! case's drawing (fixtures/commands/v1/cad.parcels.record.json, “İfraz: …”: Kadastro 1985's 101/7, 101/8 and 101/9)
//! opened as a .kcad file over MCP. The subdivision of 101/7 splits the boundary 101/9 shares with it at the new corner
//! (10, 10); 101/9's neighbours are then the two new parcels.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"subdivision-split-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_subdivision_splits_the_neighbour_s_boundary_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.record.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("İfraz: 101/7")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-subdivision-split-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.parcels.record", input);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["boundariesSplit"].as_u64(), out["neighbours"].clone()), (Some(1), json!([3])), "{out}");
    let listed = call(&mut server, "cad.parcels.records", json!({"drawing": drawing}));
    assert_eq!(listed["isError"], false, "{listed}");
    let parcels = listed["structuredContent"]["output"]["parcels"].as_array().unwrap().clone();
    let p3 = parcels.iter().find(|p| p["number"] == 3).expect("101/9");
    assert_eq!(p3["neighbours"], json!([4, 5]), "{p3}");
}

//! An arc against its chord in the boundary graph through a real AI host (plan/map-cad.md MAP-08, docs/adr/2620): the
//! shared case's three mismatches opened as a .kcad file over MCP; `cad.parcels.boundaries` names each with its two
//! boundaries, the sliver's area and the gap at the middles, as the case does.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"arc-chord-mismatch-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_mismatches_over_mcp_are_the_case_s() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.boundaries.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().unwrap().starts_with("uyuşmayan üç sınır"))
        .unwrap()
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-arc-chord-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let answer = call(&mut server, "cad.parcels.boundaries", json!({"drawing": drawing}));
    assert_eq!(answer["isError"], false, "{answer}");
    let got = answer["structuredContent"]["output"]["problems"].as_array().unwrap().clone();
    let want = case["steps"][0]["result"]["output"]["problems"].as_array().unwrap().clone();
    assert_eq!(got.len(), 3);
    for (g, w) in got.iter().zip(&want) {
        assert_eq!((&g["kind"], &g["boundaries"], &g["parcels"]), (&w["kind"], &w["boundaries"], &w["parcels"]));
        for key in ["area", "gap"] {
            let (a, b) = (g[key].as_f64().unwrap(), w[key].as_f64().unwrap());
            assert!((a - b).abs() < 1e-9, "{key}: {a} vs {b}");
        }
    }
    let _ = std::fs::remove_file(path);
}

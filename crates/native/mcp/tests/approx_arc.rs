//! An arc drawn with chords through a real AI host (plan/map-cad.md MAP-08, docs/adr/3420): the shared case's drawing
//! (A1's arc; the neighbour A3 draws it with three corners rounded to the centimetre) opened as a .kcad file over MCP.
//! Without a distance the graph has only the corner at the arc's top as a missing corner; with `arcTolerance` 0.01 it
//! has the case's one `approxArc` problem: its boundaries, strip and widest place.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"approx-arc-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn an_arc_drawn_with_chords_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.boundaries.json")).unwrap();
    let case = |start: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start))).unwrap().clone();
    let with = case("santimetreye yuvarlanmış köşeler 1 cm");
    let without = case("santimetreye yuvarlanmış köşeler uzaklık");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(with["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-approx-arc-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    for (c, kind) in [(&without, "missingVertex"), (&with, "approxArc")] {
        let step = &c["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "cad.parcels.boundaries", args);
        assert_eq!(r["isError"], false, "{r}");
        let got = r["structuredContent"]["output"]["problems"].as_array().unwrap().clone();
        let want = step["result"]["output"]["problems"].as_array().unwrap().clone();
        assert_eq!(got.len(), 1, "{r}");
        assert_eq!((got[0]["kind"].as_str(), &got[0]["boundaries"]), (Some(kind), &want[0]["boundaries"]));
        for key in ["area", "gap"] {
            if let (Some(a), Some(b)) = (got[0][key].as_f64(), want[0][key].as_f64()) {
                assert!((a - b).abs() < 1e-9, "{key}: {a} != {b}");
            }
        }
    }
    let _ = std::fs::remove_file(path);
}

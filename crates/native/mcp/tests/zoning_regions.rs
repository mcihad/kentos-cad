//! İmar planı denetimi's road regions through a real AI host (plan/map-cad.md MAP-09, docs/adr/3440): the shared
//! case's drawing (both roads wide, a parcel in two roads) opened as a .kcad file over MCP; the answer's findings carry
//! the case's regions, a road width's and a road parcel's, and no other finding carries any.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"zoning-regions-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_road_regions_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.zoning.check.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("iki yol da geniş"))).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-zoning-regions-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "cad.zoning.check", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = r["structuredContent"]["output"]["findings"].as_array().unwrap().clone();
    let want = step["result"]["output"]["findings"].as_array().unwrap().clone();
    assert_eq!(got.len(), want.len());
    let mut roads = 0;
    for (g, w) in got.iter().zip(&want) {
        assert_eq!(g["regions"], w["regions"], "{}", g["kind"]);
        if matches!(g["kind"].as_str(), Some("roadWidth" | "roadParcel")) {
            roads += 1;
            assert!(g["regions"].as_array().is_some_and(|r| !r.is_empty()), "{g}");
        }
    }
    assert!(roads >= 5, "{roads}");
    let _ = std::fs::remove_file(path);
}

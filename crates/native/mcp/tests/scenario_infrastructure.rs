//! The area near an infrastructure layer through a real AI host (plan/map-cad.md MAP-10, docs/adr/2660): the shared
//! case's scenario with a sewer line opened as a .kcad file over MCP; `cad.scenarios.impact` with the line's layer and
//! 40 m gives the case's measures on both sides.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"scenario-infrastructure-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_area_near_the_sewer_over_mcp_is_the_case_s() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "altyapıya yakınlık: kanalizasyon hattına 40 m")
        .unwrap()
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-scenario-infrastructure-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let mut args = case["steps"][0]["input"].clone();
    args["drawing"] = opened["structuredContent"]["drawing"].clone();
    let answer = call(&mut server, "cad.scenarios.impact", args);
    assert_eq!(answer["isError"], false, "{answer}");
    let got = answer["structuredContent"]["output"]["measures"].as_array().unwrap().clone();
    let want = case["steps"][0]["result"]["output"]["measures"].as_array().unwrap().clone();
    for id in ["infrastructure", "infrastructureShare"] {
        let g = got.iter().find(|m| m["id"] == id).unwrap();
        let w = want.iter().find(|m| m["id"] == id).unwrap();
        assert_eq!((&g["label"], &g["method"]), (&w["label"], &w["method"]));
        for side in ["base", "alternative"] {
            assert!((g[side].as_f64().unwrap() - w[side].as_f64().unwrap()).abs() < 1e-9, "{id} {side}");
        }
    }
    let _ = std::fs::remove_file(path);
}

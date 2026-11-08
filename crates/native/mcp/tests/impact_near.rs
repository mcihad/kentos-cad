//! The infrastructure measure's nearness through a real AI host (plan/map-cad.md MAP-10, docs/adr/3060): the shared
//! case's drawing (a sewer line, the base and the scenario's areas) opened as a .kcad file over MCP; `cad.scenarios.impact`
//! with the infrastructure layer at 40 m answers the case's `near`: each side's areas near and beyond, by persistent id,
//! and the layer's objects.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"impact-near-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_nearness_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "altyapıya yakınlık: kanalizasyon hattına 40 m")
        .unwrap()
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-impact-near-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "cad.scenarios.impact", input);
    assert_eq!(r["isError"], false, "{r}");
    let want = &case["steps"][0]["result"]["output"]["near"];
    assert!(want["alternativeLines"].as_array().is_some_and(|l| !l.is_empty()));
    assert_eq!(&r["structuredContent"]["output"]["near"], want);
    let _ = std::fs::remove_file(path);
}

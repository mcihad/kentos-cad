//! Which areas the access reached through a real AI host (plan/map-cad.md MAP-10, docs/adr/2260): the shared case's
//! drawing opened as a .kcad file over MCP. `cad.scenarios.impact` with the case's access question names the same
//! reached, unreached and target areas on both sides as the case; without the question there is no `reach`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"scenario-reach-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_reached_areas_over_mcp_are_the_case_s() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "erişim: konuttan parka ve yeşil alana 40 m").unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-scenario-reach-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.scenarios.impact", args);
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["output"]["reach"], step["result"]["output"]["reach"]);
    let plain = call(&mut server, "cad.scenarios.impact", json!({"drawing": drawing, "scenario": "alt-a"}));
    assert_eq!(plain["isError"], false, "{plain}");
    assert!(plain["structuredContent"]["output"]["reach"].is_null(), "{plain}");
    let _ = std::fs::remove_file(path);
}

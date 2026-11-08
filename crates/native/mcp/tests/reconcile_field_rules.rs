//! Detay eşleştirmede alan kuralları through a real AI host (plan/map-cad.md MAP-04, docs/adr/2540): the shared case's drawing
//! whose class layers carry the profile's fields, opened as a .kcad file over MCP. The plan says the accepted manhole's
//! network is not on the list, as the case does; executed with the network chosen from the list and the door's number,
//! both are written, the network as the list's code.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"reconcile-field-rules-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_field_rules_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.features.reconcile.json")).unwrap();
    let case = |start: &str| {
        cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with(start)).unwrap().clone()
    };
    let rules = case("Alan kuralları:");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(rules["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-reconcile-rules-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    // The plan: the groups' invalid and writes as the case says.
    let mut args = rules["steps"][0]["input"].clone();
    args["drawing"] = drawing.clone();
    args["op"] = json!("plan");
    let plan = call(&mut server, "map.features.reconcile", args);
    assert_eq!(plan["isError"], false, "{plan}");
    let got = plan["structuredContent"]["output"]["groups"].as_array().unwrap().clone();
    let want = rules["steps"][0]["result"]["output"]["groups"].as_array().unwrap().clone();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        assert_eq!((&g["status"], &g["writes"], &g["invalid"], &g["missing"]), (&w["status"], &w["writes"], &w["invalid"], &w["missing"]));
    }

    // Executed with the values given: the manhole's network written as the list's code, the door's number.
    let given = case("Pencereden alan:");
    let mut args = given["steps"][0]["input"].clone();
    args["drawing"] = drawing;
    let done = call(&mut server, "map.features.reconcile", args);
    assert_eq!(done["isError"], false, "{done}");
    let written = done["structuredContent"]["output"]["written"].as_array().map(Vec::len);
    let want_written = given["steps"][0]["result"]["output"]["written"].as_array().map(Vec::len);
    assert_eq!(written, want_written);
    let _ = std::fs::remove_file(path);
}

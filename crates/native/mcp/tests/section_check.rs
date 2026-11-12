//! Kesit kontrolü through a real AI host (plan/map-cad.md MAP-14, docs/adr/5120; BÖHHBÜY m. 93(1)): the shared cases'
//! drawing (fixtures/commands/v1/road.terrains.sectioncheck.json) opened as a .kcad file over MCP. The first case's
//! section answers as the case says; one point too few leaves gaps past 20 m, said as a warning.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"section-check-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_section_is_checked_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.sectioncheck.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-section-check-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    let first = &cases["cases"][0]["steps"][0];
    let mut input = first["input"].clone();
    input["drawing"] = drawing.clone();
    let r = call(&mut server, "road.terrains.sectioncheck", input);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let want = &first["result"]["output"];
    assert_eq!((out["compared"].clone(), out["within"].clone()), (want["compared"].clone(), json!(true)), "{out}");
    assert!((out["rms"].as_f64().unwrap() - want["rms"].as_f64().unwrap()).abs() < 1e-9);
    let gaps = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("İki nokta eksik")).unwrap();
    let mut input = gaps["steps"][0]["input"].clone();
    input["drawing"] = drawing;
    let r = call(&mut server, "road.terrains.sectioncheck", input);
    assert_eq!(r["structuredContent"]["output"]["gaps"].as_array().map(Vec::len), Some(2), "{r}");
}

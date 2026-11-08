//! `road.project.rebuild` through a real AI host (plan/map-cad.md MAP-20, docs/adr/2440): the shared cases' road project
//! opened as a .kcad file over MCP, its corridor and guardrail drawn and their template renamed. The rebuild draws the
//! two stale drawings again, each taking away the objects the shared case says (`removed`), as the desktop's job and
//! Python's do; the tree is current after.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"rebuild-equivalence-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_rebuild_over_mcp_is_the_shared_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.rebuild.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "İki eskiyen çizim birlikte, tek adımda").unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-rebuild-equivalence-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let steps = case["steps"].as_array().unwrap();
    for step in &steps[..3] {
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, step["command"].as_str().unwrap(), args);
        assert_eq!(r["isError"], false, "{r}");
    }
    let r = call(&mut server, "road.project.rebuild", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    let got: Vec<(Value, Value)> = r["structuredContent"]["output"]["rebuilt"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (x["drawing"].clone(), x["removed"].clone()))
        .collect();
    let want: Vec<(Value, Value)> = steps[3]["result"]["output"]["rebuilt"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (x["drawing"].clone(), x["removed"].clone()))
        .collect();
    assert_eq!(got, want);
    let tree = call(&mut server, "road.project.query", json!({"drawing": drawing}));
    assert_eq!(tree["isError"], false, "{tree}");
    assert_eq!(tree["structuredContent"]["output"]["rebuildable"].as_u64().unwrap_or(0), 0);
    let _ = std::fs::remove_file(path);
}

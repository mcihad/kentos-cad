//! One object's topology findings through a real AI host (plan/map-cad.md MAP-07, docs/adr/3800): the shared cases'
//! drawing opened as a .kcad file over MCP; `cad.topology.check` with A3's and A5's ids answers the case's findings
//! (A3–A4's overlap, A5's short edge) and counts, the whole check's rules and objects; an id not in the drawing is
//! refused with its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"topology-object-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn an_object_s_findings_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.topology.check.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "uids: A3 ve A5'in bulguları").unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-topology-object-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let want = &step["result"]["output"];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.topology.check", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"];
    let keys = |v: &Value| v["findings"].as_array().unwrap().iter().map(|f| f["key"].clone()).collect::<Vec<_>>();
    assert_eq!(keys(got), keys(want));
    for key in ["total", "open", "exceptions", "rules", "objects", "complete"] {
        assert_eq!(got[key], want[key], "{key}");
    }
    let uids = cases["setup"]["uids"].as_array().unwrap();
    assert!(got["findings"].as_array().unwrap().iter().all(|f| {
        f["uids"].as_array().unwrap().iter().any(|u| u == &uids[2] || u == &uids[4])
    }));
    // An id not in the drawing: refused, its place said.
    let r = call(&mut server, "cad.topology.check", json!({"drawing": drawing, "uids": [uids[0], "0192f5a9-1414-7000-8000-000000000009"]}));
    assert_eq!(r["isError"], true, "{r}");
    let e = &r["structuredContent"]["error"];
    assert_eq!((&e["code"], &e["path"]), (&json!("entity_not_found"), &json!("uids[1]")), "{r}");
    let _ = std::fs::remove_file(path);
}

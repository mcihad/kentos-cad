//! Senaryo ölçüleri's access along a network and its table written in the drawing through a real AI host
//! (plan/map-cad.md MAP-10, docs/adr/5060): the shared cases' drawings (fixtures/commands/v1/cad.scenarios.impact.json,
//! “ağ üzerinden erişim …”; cad.scenarios.record.json) opened as .kcad files over MCP. The routes along the road network
//! are the case's; Ölçüleri çizime yaz writes the table with the case's cells and the next comparison reads it back as
//! current; a compared layer is refused at its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"scenario-network-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn opened(setup: &Value, tag: &str) -> (Server, Value) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup.clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-scenario-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    (server, drawing)
}

#[test]
fn the_access_along_the_network_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("ağ üzerinden erişim: konuttan")).unwrap();
    let (mut server, drawing) = opened(&case["setup"], "network");
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing;
    let r = call(&mut server, "cad.scenarios.impact", input);
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"]["reach"], &case["steps"][0]["result"]["output"]["reach"]);
    for key in ["alternativeReached", "alternativeUnreached", "baseUnreached"] {
        assert_eq!(got[key], want[key], "{key}");
    }
    let (g, w) = (got["alternativeRoutes"].as_array().unwrap(), want["alternativeRoutes"].as_array().unwrap());
    assert_eq!(g.len(), w.len());
    for (g, w) in g.iter().zip(w) {
        assert_eq!(g["target"], w["target"]);
        assert!((g["distance"].as_f64().unwrap() - w["distance"].as_f64().unwrap()).abs() < 1e-9, "{g} vs {w}");
        assert_eq!(g["path"].as_array().unwrap().len(), w["path"].as_array().unwrap().len());
    }
}

#[test]
fn the_table_written_over_mcp_is_read_back_as_current() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.record.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("yaz: tablo 0 katmanında")).unwrap();
    let (mut server, drawing) = opened(&case["setup"], "record");
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.scenarios.record", input);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["cells"], case["steps"][0]["result"]["output"]["cells"]);
    let table = out["table"].clone();
    let r = call(&mut server, "cad.scenarios.impact", json!({"drawing": drawing, "scenario": "alt-a"}));
    let records = &r["structuredContent"]["output"]["records"];
    assert_eq!(records[0]["table"], table, "{r}");
    assert_eq!((records[0]["baseChanged"].clone(), records[0]["scenarioChanged"].clone()), (json!(false), json!(false)));
    let r = call(&mut server, "cad.scenarios.record", json!({"drawing": drawing, "impact": {"scenario": "alt-a"}, "layerId": "karar"}));
    assert_eq!(r["structuredContent"]["error"]["path"], "layerId", "{r}");
}

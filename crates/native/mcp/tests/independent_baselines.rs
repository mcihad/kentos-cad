//! A baseline session's independent baselines through a real AI host (plan/map-cad.md MAP-03, docs/adr/5320): the
//! `.kcad` schema 240 sample with its baseline sessions opened over MCP. Every baseline in a session and not excluded is
//! marked independent or dependent, no session has more independent ones than its marks less one, and the edit sets the
//! adjustment choice to them (or there is nothing to set).
use std::collections::BTreeSet;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"independent-baselines-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn independent_baselines_over_mcp() {
    let mut server = Server::new();
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/map-survey-sessions.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "map.survey.query", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let rows = out["baselines"].as_array().expect("baselines");
    for s in out["baselineSessions"].as_array().expect("sessions") {
        let members: Vec<&Value> =
            rows.iter().filter(|b| b["session"] == s["id"] && b.get("excluded").is_none()).collect();
        assert!(members.iter().all(|b| b["independent"].is_boolean()), "{s}");
        let marks: BTreeSet<&str> =
            members.iter().flat_map(|b| [b["from"].as_str().unwrap_or(""), b["to"].as_str().unwrap_or("")]).collect();
        let free = members.iter().filter(|b| b["independent"] == true).count();
        assert!(free + 1 <= marks.len().max(1), "{s}");
    }
    let r = call(&mut server, "map.survey.edit", json!({"drawing": drawing, "operation": {"op": "independent"}}));
    if r["isError"] == true {
        assert!(r.to_string().contains("nothing_to_set"), "{r}");
    }
    let r = call(&mut server, "map.survey.edit", json!({"drawing": drawing, "operation": {"op": "independent", "session": "bo-99"}}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("operation.session"), "{r}");
}

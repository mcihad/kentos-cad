//! Senaryo ölçüleri's slope classes and aspect through a real AI host (plan/map-cad.md MAP-10, docs/adr/4660): the
//! shared case's drawing (fixtures/commands/v1/cad.scenarios.impact.json, “eğim sınıfları ve bakı …”) opened as a .kcad
//! file over MCP. Its measures by class and by aspect are the case's (the triangle falling south, the one falling
//! south-west); breaks that do not rise are refused at their path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"slope-classes-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_slope_classes_and_aspects_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("eğim sınıfları ve bakı")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-slope-classes-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.scenarios.impact", input);
    assert_eq!(r["isError"], false, "{r}");
    let got = r["structuredContent"]["output"]["measures"].as_array().unwrap().clone();
    let want = case["steps"][0]["result"]["output"]["measures"].as_array().unwrap().clone();
    for w in want.iter().filter(|m| m["id"].as_str().is_some_and(|id| id.starts_with("slopeClass") || id.starts_with("aspect"))) {
        let g = got.iter().find(|m| m["id"] == w["id"]).unwrap_or_else(|| panic!("{} missing", w["id"]));
        for side in ["base", "alternative"] {
            let (a, b) = (g[side].as_f64().unwrap(), w[side].as_f64().unwrap());
            assert!((a - b).abs() < 1e-6, "{} {side}: {a} vs {b}", w["id"]);
        }
    }
    let r = call(
        &mut server,
        "cad.scenarios.impact",
        json!({"drawing": drawing, "scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 10.0, "classes": [15.0, 5.0]}}),
    );
    assert_eq!(r["structuredContent"]["error"]["path"], "slope.classes", "{r}");
}

//! The network's confidence regions through a real AI host (plan/map-cad.md MAP-05, docs/adr/2560): the shared case
//! whose baselines' deviations are oblong and turned 30°, opened as a .kcad file over MCP; every new mark's error
//! ellipse (semi-axes, direction, height's interval, confidence) is the case's.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"network-region-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_confidence_regions_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.network.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Bazların sapmaları 1,5 ve 4 mm")).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-network-region-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let r = call(&mut server, "map.survey.network", json!({"drawing": opened["structuredContent"]["drawing"]}));
    assert_eq!(r["isError"], false, "{r}");
    let got = r["structuredContent"]["output"]["stations"].as_array().unwrap().clone();
    let want = case["steps"][0]["result"]["output"]["stations"].as_array().unwrap().clone();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        match (g.get("ellipse"), w.get("ellipse")) {
            (None, None) => {}
            (Some(g), Some(w)) => {
                for key in ["confidence", "major", "minor", "azimuth", "up"] {
                    let (a, b) = (g[key].as_f64().unwrap(), w[key].as_f64().unwrap());
                    assert!((a - b).abs() < 1e-6, "{key}: {a} ≠ {b}");
                }
            }
            other => panic!("{other:?}"),
        }
    }
    let _ = std::fs::remove_file(path);
}

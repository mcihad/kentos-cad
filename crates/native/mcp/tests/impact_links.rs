//! Scenario areas joined to their nearest infrastructure line through a real AI host (plan/map-cad.md MAP-10,
//! docs/adr/3460): the shared case with the roads as the infrastructure (the scenario has two roads) opened as a .kcad
//! file over MCP; each side's links are the case's: the area, the nearest road (the first by the drawing's order when
//! two are equally near), the centroid, the road's nearest place and the distance.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"impact-links-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_links_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("senaryonun yerine geçtiği katman altyapı"))).unwrap().clone();
    let setup = case.get("setup").cloned().unwrap_or_else(|| cases["setup"].clone());
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-impact-links-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "cad.scenarios.impact", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"]["near"];
    let want = &step["result"]["output"]["near"];
    for key in ["baseLinks", "alternativeLinks"] {
        let (g, w) = (got[key].as_array().unwrap(), want[key].as_array().unwrap());
        assert_eq!(g.len(), w.len(), "{key}");
        assert!(!g.is_empty(), "{key}");
        for (a, b) in g.iter().zip(w) {
            assert_eq!((&a["area"], &a["line"]), (&b["area"], &b["line"]), "{key}");
            for k in ["from", "to"] {
                for i in 0..2 {
                    assert!((a[k][i].as_f64().unwrap() - b[k][i].as_f64().unwrap()).abs() < 1e-6, "{key} {k}");
                }
            }
            assert!((a["distance"].as_f64().unwrap() - b["distance"].as_f64().unwrap()).abs() < 1e-6, "{key}");
        }
    }
    let _ = std::fs::remove_file(path);
}

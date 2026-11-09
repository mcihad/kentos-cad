//! Senaryo ölçüleri's terrain slope through a real AI host (plan/map-cad.md MAP-10, docs/adr/4260): the shared case's
//! drawing (fixtures/commands/v1/cad.scenarios.impact.json, “eğim: Zemin yüzeyinde …”) opened as a .kcad file over MCP.
//! The answer carries the four slope measures as the case does; Alternatif A's konut east of x = 125 is half off the
//! surface (50 m²), a gap of the scenario only; nothing is written.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"scenario-slope-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn slope_over_mcp() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.scenarios.impact.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("eğim: Zemin yüzeyinde")).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-scenario-slope-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();

    let r = call(&mut server, "cad.scenarios.impact", json!({"drawing": drawing, "scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 10.0}}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let want = &case["steps"][0]["result"]["output"];
    let by = |o: &Value, id: &str| o["measures"].as_array().unwrap().iter().find(|m| m["id"] == id).unwrap().clone();
    for id in ["slopeMean", "slopeSteep", "slopeSteepShare", "slopeOff"] {
        let (got, wanted) = (by(out, id), by(want, id));
        for side in ["base", "alternative"] {
            let (g, w) = (got[side].as_f64().unwrap(), wanted[side].as_f64().unwrap());
            assert!((g - w).abs() < 1e-6, "{id} {side}: {g} != {w}");
        }
    }
    assert!((by(out, "slopeOff")["alternative"].as_f64().unwrap() - 50.0).abs() < 1e-6);
    let gaps: Vec<&str> = out["gaps"].as_array().unwrap().iter().filter_map(Value::as_str).filter(|g| g.contains("yüzeyinin dışında")).collect();
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert!(gaps[0].contains("senaryoda"));
}

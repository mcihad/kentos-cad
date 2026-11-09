//! Kerbs with arcs reconciled through a real AI host (plan/map-cad.md MAP-04, docs/adr/3340): the shared case's drawing
//! opened as a .kcad file over MCP gives the case's groups on a plan (the arc and its chord apart, the arc and its copy
//! drawn by its vertices agreed, their spread), and execute writes the kerb with its arc.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"reconcile-arcs-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_arcs_plan_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.features.reconcile.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Yaylı bordür"))).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-reconcile-arcs-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    args["op"] = json!("plan");
    let r = call(&mut server, "map.features.reconcile", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = r["structuredContent"]["output"]["groups"].as_array().unwrap().clone();
    let want = step["result"]["output"]["groups"].as_array().unwrap().clone();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        assert_eq!((&g["status"], &g["candidates"]), (&w["status"], &w["candidates"]));
        let (a, b) = (g["spread"].as_f64().unwrap(), w["spread"].as_f64().unwrap());
        assert!((a - b).abs() < 1e-9, "spread {a} != {b}");
    }
    // Execute: the agreed kerb written with its arc.
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "map.features.reconcile", args);
    assert_eq!(r["isError"], false, "{r}");
    // Saved: the two candidates with arcs and the written kerb, its arc kept.
    let saved = std::env::temp_dir().join(format!("kentos-reconcile-arcs-saved-{}.kcad", std::process::id()));
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let back = serde_json::to_value(kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap()).unwrap();
    let arcs = back["entities"].as_array().unwrap().iter().filter(|e| e.get("bulges").is_some()).count();
    assert_eq!(arcs, 3, "two candidates and the kerb written with its arc");
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(saved);
}

//! Area candidates reconciled through a real AI host (plan/map-cad.md MAP-04, docs/adr/2140): the shared case's drawing
//! of building footprints and green areas opened as a .kcad file over MCP gives the case's groups (statuses, overlap
//! ratios, centroids) on a plan, and an overlap ratio outside 0–1 is refused with the case's code, place and words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"reconcile-areas-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const PLAN: &str = "Alanlar (plan): bina izi iki kaynakta örtüşür (yineleneniyle), 0,25 örtüşen iki iz ayrı tekler, var \
                    olan binanın üstündeki aday, yeşil alan/bina çelişkisi, nokta çizilmiş bina, alan çizilmiş direk";

#[test]
fn the_footprints_plan_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.features.reconcile.json")).unwrap();
    let find = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
    let case = find(PLAN);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-reconcile-areas-{}.kcad", std::process::id()));
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
        for key in ["x", "y", "spread", "overlap"] {
            match (g[key].as_f64(), w[key].as_f64()) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{key}: {a} != {b}"),
                (a, b) => assert_eq!(a, b, "{key}"),
            }
        }
    }
    // An overlap ratio of 1: refused as the case says.
    let refusal = find("Örtüşme 1");
    let mut args = refusal["steps"][0]["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "map.features.reconcile", args);
    assert_eq!(r["isError"], true, "{r}");
    let text = r.to_string();
    let error = &refusal["steps"][0]["result"]["error"];
    for key in ["code", "message", "path"] {
        assert!(text.contains(error[key].as_str().unwrap()), "{key}: {text}");
    }
    let _ = std::fs::remove_file(path);
}

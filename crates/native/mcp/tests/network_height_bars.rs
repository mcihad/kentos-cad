//! The height bars an accepted GNSS network draws beside its error ellipses, through a real AI host (plan/map-cad.md
//! MAP-05, docs/adr/3360): the shared case's archive opened as a .kcad file over MCP; the network accepted with ellipses
//! and `heights` answers the case's three ellipses and three bars, and each bar saved and read back is the case's (the
//! half-interval magnified, its ticks a fifth of it).
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"network-height-bars-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_accepted_network_s_height_bars_are_the_case_s_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.edit.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().contains("kot çubuklarıyla")).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-network-bars-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ag.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "map.survey.edit", args);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["ellipses"].as_u64(), out["bars"].as_u64()), (Some(3), Some(3)), "{out}");
    let saved = dir.join("kabul.kcad");
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let back = serde_json::to_value(kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap()).unwrap();
    let bars: Vec<&Value> = back["entities"].as_array().unwrap().iter().filter(|e| e["attrs"].get("Kot yarı aralığı (m)").is_some()).collect();
    assert_eq!(bars.len(), 3);
    let want: Vec<&Value> = step["expect"]["fields"].as_object().unwrap().values().filter(|f| f["attrs"].get("Kot yarı aralığı (m)").is_some()).collect();
    assert_eq!(want.len(), 3);
    for b in &bars {
        // The case's bar of the same mark (its fields are keyed by slot).
        let w = want.iter().find(|w| w["attrs"]["İşaret"] == b["attrs"]["İşaret"]).expect("the case's bar");
        assert_eq!(b["attrs"], w["attrs"]);
        for (p, q) in b["pts"].as_array().unwrap().iter().zip(w["pts"].as_array().unwrap()) {
            for k in ["x", "y"] {
                assert!((p[k].as_f64().unwrap() - q[k].as_f64().unwrap()).abs() < 1e-6, "{p} {q}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

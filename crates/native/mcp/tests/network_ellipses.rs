//! The error ellipses an accepted GNSS network draws, through a real AI host (plan/map-cad.md MAP-05, docs/adr/2960):
//! the shared case's archive with its baselines' deviations turned 30° opened as a .kcad file over MCP; the network
//! accepted with ellipses magnified 2000 times answers three ellipses, the drawing saved and read back holds them on
//! the layer with their marks and the adjustment, their major axes as the case says, and `map.survey.query` lists the
//! use that records them, up to date.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"network-ellipses-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_accepted_network_s_ellipses_are_the_case_s_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.edit.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().unwrap().contains("sapmalar 30° dönük: elipslerin"))
        .unwrap()
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-network-ellipses-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ag.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let r = call(&mut server, "map.survey.edit", json!({"drawing": drawing, "operation": step["input"]["operation"]}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["output"]["ellipses"], 3);
    let saved = dir.join("ag-kabul.kcad");
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let back = serde_json::to_value(kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap()).unwrap();
    let ellipses: Vec<&Value> = back["entities"].as_array().unwrap().iter().filter(|e| e["kind"] == "ellipse").collect();
    assert_eq!(ellipses.len(), 3);
    let want: Vec<&Value> = step["expect"]["fields"].as_object().unwrap().values().filter(|f| f.get("major").is_some()).collect();
    for (got, want) in ellipses.iter().zip(&want) {
        assert_eq!((&got["layerId"], &got["attrs"]), (&want["layerId"], &want["attrs"]));
        for axis in ["x", "y"] {
            let (a, b) = (got["major"][axis].as_f64().unwrap(), want["major"][axis].as_f64().unwrap());
            assert!((a - b).abs() < 1e-6, "{axis}: {a} ≠ {b}");
        }
    }
    let q = call(&mut server, "map.survey.query", json!({"drawing": drawing}));
    let uses = q["structuredContent"]["output"]["uses"].as_array().unwrap().clone();
    let used = uses.iter().find(|u| u["operation"] == "Hata elipsleri (da-1)").expect("the use");
    assert_eq!((used["state"].as_str(), used["objects"].as_u64()), (Some("current"), Some(3)), "{used}");
    let _ = std::fs::remove_dir_all(dir);
}

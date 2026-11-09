//! Yüzeye en yakın uzaklık through a real AI host (plan/map-cad.md MAP-19, docs/adr/4020): the shared cases' drawing of
//! `cad.asbuilt.check` opened as a .kcad file over MCP. Held to the nearest distance with the case's limit, every point's
//! nearest distance and place and P3's flags are the shared case's.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"asbuilt-nearest-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_nearest_distances_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-asbuilt-nearest-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().unwrap().starts_with("yüzeye en yakın uzaklık: aynı sınır"))
        .unwrap()["steps"][0]
        .clone();
    let mut args = step["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "cad.asbuilt.check", args);
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"]["points"], &step["result"]["output"]["points"]);
    for (g, w) in got.as_array().unwrap().iter().zip(want.as_array().unwrap()) {
        assert_eq!(g["name"], w["name"]);
        match (g["nearest"].as_f64(), w["nearest"].as_f64()) {
            (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{}: {a} {b}", g["name"]),
            (a, b) => assert_eq!(a, b, "{}", g["name"]),
        }
        if let (Some(a), Some(b)) = (g["nearestAt"].as_array(), w["nearestAt"].as_array()) {
            for (x, y) in a.iter().zip(b) {
                assert!((x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 1e-6, "{}", g["name"]);
            }
        }
        assert_eq!(g["flags"], w["flags"], "{}", g["name"]);
    }
    let _ = std::fs::remove_file(path);
}

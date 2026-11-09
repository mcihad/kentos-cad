//! Koordinat dönüştür's decision point by point through a real AI host (plan/map-cad.md MAP-02, docs/adr/3700): the
//! shared cases' project of `cad.crs.transform` opened as a .kcad file over MCP. The case whose three points go from
//! TUREF / TM30 to WGS 84 / UTM 35N gives the case's notes (one within every area, one beyond the target's, one beyond
//! the source's and the target's) and the warning that counts them; every point is converted.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"crs-point-notes-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn each_point_s_areas_of_use_are_said_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.crs.transform.json")).unwrap();
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-crs-point-notes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("donustur.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Noktaların ayrı kararları"))).unwrap();
    let want = &case["steps"][0]["result"];
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing;
    let r = call(&mut server, "cad.crs.transform", input);
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"];
    assert_eq!(got["output"]["flagged"], want["output"]["flagged"]);
    assert_eq!(
        got["output"]["flagged"],
        json!([{"index": 1, "notes": ["outside_target_area"]}, {"index": 2, "notes": ["outside_source_area", "outside_target_area"]}])
    );
    assert!(got["output"]["points"].as_array().unwrap().iter().all(|p| !p.is_null()), "every point converted");
    assert!(r.to_string().contains("outside_area"), "{r}");
    let _ = std::fs::remove_dir_all(dir);
}

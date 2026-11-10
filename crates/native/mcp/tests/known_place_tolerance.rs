//! The user's tolerance of a known point's place against its mark's accepted point through a real AI host
//! (plan/map-cad.md MAP-03, docs/adr/4520): the shared case's drawing of `map.survey.query` (N1 both known and
//! accepted) opened as a .kcad file over MCP. With a 5 mm horizontal limit N1 is beyond, with 1 cm within; BN1 and BN2
//! are not judged; a limit of 0 is refused at its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"known-place-tolerance-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_known_point_s_place_against_the_user_s_limits_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.query.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Kabul farkı sınırı: yatay 5 mm")))
        .cloned()
        .expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-known-place-tolerance-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(&path);
    let drawing = opened["structuredContent"]["drawing"].clone();
    let places = |server: &mut Server, tolerance: Value| {
        let r = call(server, "map.survey.query", json!({"drawing": drawing, "places": true, "tolerance": tolerance}));
        assert_eq!(r["isError"], false, "{r}");
        r["structuredContent"]["output"]["knownPlaces"].clone()
    };
    let beyond_of = |list: &Value, mark: &str| list.as_array().unwrap().iter().find(|p| p["mark"] == mark).unwrap()["beyond"].clone();
    let tight = places(&mut server, json!({"horizontal": 0.005}));
    assert_eq!(beyond_of(&tight, "N1"), json!(true), "{tight}");
    assert_eq!(beyond_of(&tight, "BN1"), Value::Null, "not judged");
    let wide = places(&mut server, json!({"horizontal": 0.01}));
    assert_eq!(beyond_of(&wide, "N1"), json!(false), "{wide}");
    let r = call(&mut server, "map.survey.query", json!({"drawing": drawing, "places": true, "tolerance": {"horizontal": 0.0}}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("tolerance.horizontal"), "{r}");
}

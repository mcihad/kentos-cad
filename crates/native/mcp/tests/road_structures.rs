//! KentOS Yol's structures through MCP (plan/road.md Y11, docs/adr/0909): the culverts' shared cases' drawing (corridors
//! whose crosses are in fill, a guardrail and two culverts) is written to a file and opened; the guardrails and a culvert
//! are asked, the culvert is turned and drawn, and the other is removed.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"road-structures-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities":{}})
}

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":meta()}}))
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

#[test]
fn culverts_are_asked_changed_drawn_and_removed() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.culverts.query")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("road.culverts.edit")["annotations"]["destructiveHint"], true);
    assert_eq!(tool("road.guardrails.edit")["annotations"]["destructiveHint"], true);

    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.culverts.edit.json")).expect("cases");
    let snapshot = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let path = std::env::temp_dir().join(format!("kentos-road-structures-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).expect("encodes")).expect("writes");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let rails = call(&mut server, "road.guardrails.query", json!({"drawing":drawing}));
    assert_eq!(rails["isError"], false, "{rails}");
    let asked = call(&mut server, "road.culverts.query", json!({"drawing":drawing,"culvert":"m-1"}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(asked["isError"], false, "{asked}");
    let first = asked["structuredContent"]["output"]["culverts"][0]["length"].as_f64().expect("a length");

    let turned = call(&mut server, "road.culverts.edit", json!({"drawing":drawing,"operation":"set","culvert":"m-1","skew":25.0}));
    assert_eq!(turned["isError"], false, "{turned}");
    let longer = turned["structuredContent"]["output"]["culvert"]["length"].as_f64().expect("a length");
    assert!(longer > first, "{first} → {longer}");

    let drawn = call(&mut server, "road.culverts.draw", json!({"drawing":drawing,"culvert":"m-1","layerId":"korkuluk","labels":true}));
    assert_eq!(drawn["isError"], false, "{drawn}");
    assert_eq!(drawn["structuredContent"]["output"]["uids"].as_array().map(Vec::len), Some(3), "{drawn}");

    let gone = call(&mut server, "road.culverts.edit", json!({"drawing":drawing,"operation":"remove","culvert":"Yan menfez"}));
    assert_eq!(gone["isError"], false, "{gone}");
    let missing = call(&mut server, "road.culverts.query", json!({"drawing":drawing,"culvert":"m-2"}));
    assert_eq!(missing["isError"], true, "{missing}");
    assert!(missing.to_string().contains("unknown_culvert"), "{missing}");
}

#[test]
fn retaining_walls_are_asked_changed_and_drawn() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.walls.query")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("road.walls.edit")["annotations"]["destructiveHint"], true);

    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.walls.edit.json")).expect("cases");
    let snapshot = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let path = std::env::temp_dir().join(format!("kentos-road-walls-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).expect("encodes")).expect("writes");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let asked = call(&mut server, "road.walls.query", json!({"drawing":drawing,"wall":"d-1"}));
    assert_eq!(asked["isError"], false, "{asked}");
    let wall = &asked["structuredContent"]["output"]["walls"][0];
    assert_eq!(wall["rows"].as_array().map(Vec::len), Some(5), "{asked}");
    let concrete = wall["concrete"].as_f64().expect("concrete");

    let wider = call(&mut server, "road.walls.edit", json!({"drawing":drawing,"operation":"set","wall":"d-1",
        "shape":{"top":0.3,"batter":0.1,"footing":3.2,"base":0.5,"toe":0.8,"cover":0.6,"above":0.9}}));
    assert_eq!(wider["isError"], false, "{wider}");
    assert!(wider["structuredContent"]["output"]["wall"]["concrete"].as_f64().expect("concrete") > concrete, "{wider}");

    let drawn = call(&mut server, "road.walls.draw", json!({"drawing":drawing,"wall":"d-1","layerId":"korkuluk","labels":true}));
    assert_eq!(drawn["isError"], false, "{drawn}");
    assert_eq!(drawn["structuredContent"]["output"]["uids"].as_array().map(Vec::len), Some(3), "{drawn}");
}

#[test]
fn the_pay_book_gives_its_items_quantities() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.pay.query")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("road.pay.edit")["annotations"]["destructiveHint"], true);

    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.pay.query.json")).expect("cases");
    let snapshot = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let path = std::env::temp_dir().join(format!("kentos-road-pay-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).expect("encodes")).expect("writes");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let asked = call(&mut server, "road.pay.query", json!({"drawing":drawing,"item":"P-3"}));
    assert_eq!(asked["isError"], false, "{asked}");
    let length = asked["structuredContent"]["output"]["rows"][0]["quantity"].as_f64().expect("a quantity");
    assert!(length > 0.0, "{asked}");

    let priced = call(&mut server, "road.pay.edit", json!({"drawing":drawing,"operation":"set","items":[
        {"code":"P-3","name":"Otokorkuluk","unit":"m","price":40.0}],
        "rules":[{"measure":"guardrailLength","item":"P-3","factor":1.0}]}));
    assert_eq!(priced["isError"], false, "{priced}");
    let total = call(&mut server, "road.pay.query", json!({"drawing":drawing}));
    let out = &total["structuredContent"]["output"];
    assert!((out["total"].as_f64().expect("a total") - 40.0 * length).abs() < 1e-6, "{total}");
    assert_eq!(out["unpriced"], 0, "{total}");
}

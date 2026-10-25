//! Contours are semantic input, including source holes; no client offset math.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn request(server: &mut Server, method: &str, mut params: Value) -> Value {
    params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"architecture-contour-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
    server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .unwrap()["result"]
        .clone()
}
fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let r = request(server, "tools/call", json!({"name":name,"arguments":args}));
    assert_eq!(r["resultType"], "complete", "{r}");
    r
}
#[test]
fn contour_schema_plan_holes_one_undo_and_duplicate_refusal_use_the_real_host() {
    let mut server = Server::new();
    let listed = request(&mut server, "tools/list", json!({}));
    let tool = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "cad.walls.convert")
        .unwrap();
    assert!(
        tool["inputSchema"]["$defs"]["WallsFromMode"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["const"] == "contours")
    );
    let new = call(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = new["structuredContent"]["drawing"].clone();
    let layer = new["structuredContent"]["summary"]["activeLayer"].clone();
    let made = call(
        &mut server,
        "cad.polygon.create",
        json!({"drawing":drawing,"layerId":layer,"pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":8},{"x":0,"y":8}],"holes":[{"pts":[{"x":4,"y":3},{"x":6,"y":3},{"x":6,"y":5},{"x":4,"y":5}]}],"attrs":{"office":"A"}}),
    );
    assert_eq!(made["isError"], false, "{made}");
    let uid = made["structuredContent"]["output"]["uid"].clone();
    let mut input = json!({"drawing":drawing,"uids":[uid],"mode":"contours","justify":"left","keepSources":true,"op":"plan"});
    let plan = call(&mut server, "cad.walls.convert", input.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(
        plan["structuredContent"]["output"]["walls"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        1
    );
    input["op"] = json!("execute");
    input["expectedRevision"] = plan["structuredContent"]["output"]["revision"].clone();
    let written = call(&mut server, "cad.walls.convert", input.clone());
    assert_eq!(written["isError"], false, "{written}");
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        3
    );
    call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        1
    );
    call(&mut server, "drawing.redo", json!({"drawing":drawing}));
    input.as_object_mut().unwrap().remove("expectedRevision");
    let refused = call(&mut server, "cad.walls.convert", input);
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_wall"
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        3
    );
}

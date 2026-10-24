//! The new semantic inputs pass through MCP discovery and the real headless host.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn request(server: &mut Server, method: &str, mut params: Value) -> Value {
    params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"architecture-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
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
fn precision_placement_is_discoverable_and_an_agent_gets_the_same_geometry_and_atomic_refusal() {
    let mut s = Server::new();
    let listed = request(&mut s, "tools/list", json!({}));
    let tool = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "cad.openings.create")
        .unwrap();
    let fields = &tool["inputSchema"]["$defs"]["NewOpening"]["properties"];
    assert!(fields["span"].is_object() && fields["distanceReference"].is_object());
    let d = call(&mut s, "drawing.new", json!({"workspace":"cad"}));
    let drawing = d["structuredContent"]["drawing"].clone();
    let layer = d["structuredContent"]["summary"]["activeLayer"].clone();
    let wall = call(
        &mut s,
        "cad.walls.create",
        json!({"drawing":drawing,"layerId":layer,"walls":[{"pts":[{"x":0,"y":0},{"x":10,"y":0}],"width":0.2,"closed":false}]}),
    );
    let uid = wall["structuredContent"]["output"]["uids"][0].clone();
    let mut input = json!({"drawing":drawing,"layerId":layer,"op":"plan","openings":[{"host":uid,"point":{"x":4,"y":0.2},"width":1,"distance":2,"from":"end","distanceReference":"hinge","hand":"start"}]});
    let plan = call(&mut s, "cad.openings.create", input.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(
        plan["structuredContent"]["output"]["openings"][0]["at"],
        8.5
    );
    let summary = call(&mut s, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(summary["structuredContent"]["objects"], 1);
    input["op"] = json!("execute");
    let made = call(&mut s, "cad.openings.create", input);
    assert_eq!(made["isError"], false, "{made}");
    let mut input = json!({"drawing":drawing,"layerId":layer,"op":"plan","openings":[{"host":uid,"form":"window","span":[{"x":2,"y":0.2},{"x":3.2,"y":0.2}]}]});
    let plan = call(&mut s, "cad.openings.create", input.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert!(
        (plan["structuredContent"]["output"]["openings"][0]["width"]
            .as_f64()
            .unwrap()
            - 1.2)
            .abs()
            < 1e-12
    );
    input["op"] = json!("execute");
    let made = call(&mut s, "cad.openings.create", input.clone());
    assert_eq!(made["isError"], false, "{made}");
    call(&mut s, "drawing.undo", json!({"drawing":drawing}));
    let summary = call(&mut s, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(summary["structuredContent"]["objects"], 2);
    call(&mut s, "drawing.redo", json!({"drawing":drawing}));
    input["openings"][0]["width"] = json!(1);
    let refused = call(&mut s, "cad.openings.create", input);
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_opening"
    );
    let summary = call(&mut s, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(summary["structuredContent"]["objects"], 3);
}

//! KentOS Yol's terrain surfaces through MCP (plan/road.md Y2.3): a road project's sample drawing, a surface made of
//! its points, its heights and its section asked, and a map project refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"road-terrain-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities":{}}}}),
        )
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

fn opened(server: &mut Server) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn a_surface_is_made_asked_and_refused_outside_a_road_project() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{
            "io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"road-terrain-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.terrains.query")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("road.terrains.edit")["annotations"]["destructiveHint"], true);
    assert_eq!(tool("road.terrains.create")["annotations"]["destructiveHint"], false);

    let drawing = opened(&mut server);
    let made = call(
        &mut server,
        "road.terrains.create",
        json!({"drawing":drawing,"name":"Zemin 2","points":{"layers":["zemin"]},
               "breaklines":{"uids":["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e16"]}}),
    );
    assert_eq!(made["isError"], false, "{made}");
    let report = &made["structuredContent"]["output"]["surface"];
    assert_eq!(report["id"], "t-1");
    assert_eq!(report["state"], "current");
    assert_eq!(report["figures"]["triangles"], 6);
    let asked = call(
        &mut server,
        "road.terrains.query",
        json!({"drawing":drawing,"surface":"zemin 2","at":[[20.0,15.0],[60.0,0.0]],"along":[[0.0,15.0],[40.0,15.0]]}),
    );
    assert_eq!(asked["isError"], false, "{asked}");
    let out = &asked["structuredContent"]["output"];
    assert_eq!(out["heights"][0]["z"], 101.75);
    assert_eq!(out["heights"][1]["place"], "outside");
    let section = out["section"].as_array().expect("section");
    assert_eq!(section.first().unwrap()["station"], 0.0);
    assert_eq!(section.last().unwrap()["station"], 40.0);

    let map = call(&mut server, "drawing.new", json!({"workspace":"cad","cadWork":"map","province":6}));
    assert_eq!(map["isError"], false, "{map}");
    let map = map["structuredContent"]["drawing"].clone();
    let refused = call(&mut server, "road.terrains.query", json!({"drawing":map}));
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(refused["structuredContent"]["error"]["code"], "not_a_road_project", "{refused}");
}

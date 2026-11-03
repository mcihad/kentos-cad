//! KentOS Yol's alignments through MCP (plan/road.md Y3, docs/adr/0901): the road project's sample drawing holds an
//! alignment and its parallel axis; another is made from PIs, a point's station and offset asked, the edit refused
//! while a parallel axis follows.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"road-alignment-test","version":"1"},
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
fn an_alignment_is_made_asked_and_kept_while_followed() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.alignments.query")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("road.alignments.edit")["annotations"]["destructiveHint"], true);
    assert_eq!(tool("road.alignments.create")["annotations"]["destructiveHint"], false);

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let made = call(
        &mut server,
        "road.alignments.create",
        json!({"drawing":drawing,"name":"Yan yol","pis":[{"id":"a","x":0.0,"y":-200.0},
            {"id":"p1","x":300.0,"y":-200.0,"radius":250.0,"spiralIn":50.0,"spiralOut":50.0},
            {"id":"b","x":500.0,"y":0.0}]}),
    );
    assert_eq!(made["isError"], false, "{made}");
    let report = &made["structuredContent"]["output"]["alignment"];
    assert_eq!(report["id"], "g-4");
    assert_eq!(report["elements"].as_array().map(Vec::len), Some(5));

    let asked = call(
        &mut server,
        "road.alignments.query",
        json!({"drawing":drawing,"alignment":"ana aks","points":[[520.0,30.0]],"kms":[{"km":2550.0,"region":2}]}),
    );
    assert_eq!(asked["isError"], false, "{asked}");
    let out = &asked["structuredContent"]["output"];
    let foot = &out["points"][0]["nearest"];
    assert_eq!(foot["at"]["element"], "p1/kurp");
    assert!(foot["offset"].as_f64().unwrap() < 0.0, "left of the alignment: {foot}");
    assert_eq!(out["kms"][0]["places"][0]["region"], 2);

    let refused = call(&mut server, "road.alignments.edit", json!({"drawing":drawing,"alignment":"g-1","operation":"remove"}));
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(refused.to_string().contains("alignment_in_use"), "{refused}");
}

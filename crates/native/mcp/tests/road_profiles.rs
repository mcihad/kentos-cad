//! KentOS Yol's profiles through MCP (plan/road.md Y4, docs/adr/0902): the road project's sample drawing holds a
//! profile on its alignment; another is made, the red elevations asked, the alignment's removal refused while a profile
//! lies on it.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"road-profile-test","version":"1"},
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
fn a_profile_is_made_asked_and_keeps_its_alignment() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.profiles.query")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("road.profiles.edit")["annotations"]["destructiveHint"], true);
    assert_eq!(tool("road.profiles.create")["annotations"]["destructiveHint"], false);

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let made = call(
        &mut server,
        "road.profiles.create",
        json!({"drawing":drawing,"name":"Alternatif","alignment":"ana aks","pvis":[
            {"id":"a","station":0.0,"elevation":100.0},
            {"id":"v1","station":500.0,"elevation":96.0,"radius":5000.0},
            {"id":"b","station":1000.0,"elevation":101.0}]}),
    );
    assert_eq!(made["isError"], false, "{made}");
    let report = &made["structuredContent"]["output"]["profile"];
    assert_eq!(report["id"], "b-2");
    assert_eq!(report["curves"][0]["kind"], "circle");
    assert_eq!(report["curves"][0]["crest"], false);

    let asked = call(&mut server, "road.profiles.query", json!({"drawing":drawing,"profile":"b-2","stations":[0.0,500.0]}));
    assert_eq!(asked["isError"], false, "{asked}");
    let at = &asked["structuredContent"]["output"]["at"];
    assert!((at[0]["red"].as_f64().unwrap() - 100.0).abs() < 1e-9, "{at}");
    // At the sag's PVI the circle lies above it.
    assert!(at[1]["red"].as_f64().unwrap() > 96.0, "{at}");

    let refused = call(&mut server, "road.alignments.edit", json!({"drawing":drawing,"alignment":"g-1","operation":"remove"}));
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(refused.to_string().contains("alignment_in_use"), "{refused}");
}

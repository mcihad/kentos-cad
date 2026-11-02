//! KentOS Yol's templates and sample line sets through MCP (plan/road.md Y5, docs/adr/0903): the road project's sample
//! drawing holds two templates and three sets; a template and a set are made, the set's sections asked, a profile and a
//! template a set uses are kept.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"road-section-test","version":"1"},
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
fn a_set_is_made_asked_and_keeps_what_it_uses() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.sections.query")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("road.sections.edit")["annotations"]["destructiveHint"], true);
    assert_eq!(tool("road.templates.edit")["annotations"]["destructiveHint"], true);

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let made = call(
        &mut server,
        "road.templates.edit",
        json!({"drawing":drawing,"operation":"set","name":"Dar","left":{
            "components":[{"kind":"lane","width":3.0,"slope":-2.0}],"cut":1.0,"fill":1.5}}),
    );
    assert_eq!(made["isError"], false, "{made}");
    assert_eq!(made["structuredContent"]["output"]["template"]["id"], "tk-3");

    let set = call(
        &mut server,
        "road.sections.edit",
        json!({"drawing":drawing,"operation":"set","name":"Dar kesitler","alignment":"ana aks","profile":"Proje kotu",
            "template":"dar","left":20.0,"right":20.0,"stations":{"method":"stations","stations":[200.0,100.0]}}),
    );
    assert_eq!(set["isError"], false, "{set}");
    let report = &set["structuredContent"]["output"]["set"];
    assert_eq!(report["id"], "k-4");
    assert_eq!(report["stations"], json!([100.0, 200.0]));

    let asked = call(&mut server, "road.sections.query", json!({"drawing":drawing,"set":"k-4"}));
    assert_eq!(asked["isError"], false, "{asked}");
    let rows = asked["structuredContent"]["output"]["sections"].as_array().expect("sections").clone();
    assert_eq!(rows.len(), 2, "{asked}");
    assert!(rows[0]["axis"].as_f64().is_some(), "{asked}");

    let kept = call(&mut server, "road.profiles.edit", json!({"drawing":drawing,"profile":"b-1","operation":"remove"}));
    assert_eq!(kept["isError"], true, "{kept}");
    assert!(kept.to_string().contains("profile_in_use"), "{kept}");
    let kept = call(&mut server, "road.templates.edit", json!({"drawing":drawing,"template":"Dar","operation":"remove"}));
    assert_eq!(kept["isError"], true, "{kept}");
    assert!(kept.to_string().contains("road_template_in_use"), "{kept}");
}

//! An office standard's block and sheet template through MCP (docs/adr/1100): an agent binds a standard naming a
//! block publication of `kentos-ofis` and an approved template, asks the install's plan, installs and finds the block
//! and the approved revision in the drawing; a reference no library holds stays an `unavailable` row. The tool is
//! marked destructive: its replace writes over a definition.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"office-standard-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities":{}})
}

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":meta()}}),
        )
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

#[test]
fn an_agent_installs_an_office_block_and_template_and_sees_what_no_library_holds() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = tools
        .iter()
        .find(|t| t["name"] == "cad.standards.install")
        .expect("the tool");
    assert_eq!(tool["annotations"]["destructiveHint"], true);

    let doc = json!({"format":"kentos.document","version":1,"name":"Ofis",
        "settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50.0,
                    "workspace":"cad"},
        "origin":{"x":0.0,"y":0.0},
        "layers":[{"id":"plan","name":"Plan","type":"layer","visible":true,"locked":false,"expanded":true,
                   "style":{"color":"fg","lineType":"continuous","lineWeight":0.25},"children":[]}],
        "activeLayer":"plan","entities":[],"styles":{"items":[],"categories":[]}});
    let dir =
        std::env::temp_dir().join(format!("kentos-mcp-office-standard-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("ofis.json");
    std::fs::write(&source, doc.to_string()).unwrap();
    let opened = call(&mut server, "drawing.open", json!({"path":source}));
    let drawing = opened["structuredContent"]["drawing"].clone();

    let package = json!({"format":"kentos.drawing-standard","version":2,"id":"ofis","revision":"2026.4",
        "standard":{"name":"Ofis","layers":[]},
        "resources":{"blocks":[
            {"name":"KAPI","publication":{"familyId":"0192f5b1-0000-7000-8000-0000000f0001",
                "typeId":"0192f5b1-0000-7000-8000-0000000e0001","typeName":"Tek kanatlı kapı 90","release":2}},
            {"name":"ESKI","publication":{"familyId":"0192f5b1-0000-7000-8000-0000000f0001",
                "typeId":"0192f5b1-0000-7000-8000-0000000e0001","typeName":"Tek kanatlı kapı 90","release":7}}],
            "sheetTemplates":[{"templateId":"sys:cad-mimari","revision":1}]}});
    let bound = call(
        &mut server,
        "cad.settings.set",
        json!({"drawing":drawing,"standardDocs":[{"text":package.to_string()}]}),
    );
    assert_eq!(bound["isError"], false, "{bound}");

    let plan = call(
        &mut server,
        "cad.standards.install",
        json!({"drawing":drawing,"op":"plan"}),
    );
    assert_eq!(plan["isError"], false, "{plan}");
    let rows = plan["structuredContent"]["output"]["rows"].clone();
    assert_eq!(rows[0]["status"], "missing");
    assert_eq!(rows[0]["library"], "kentos-ofis");
    assert_eq!(rows[1]["status"], "unavailable");
    assert_eq!(rows[1]["fields"], json!(["release"]));
    assert_eq!(rows[2]["kind"], "sheetTemplate");

    let done = call(
        &mut server,
        "cad.standards.install",
        json!({"drawing":drawing}),
    );
    assert_eq!(done["isError"], false, "{done}");
    let out = &done["structuredContent"]["output"];
    assert_eq!(out["rows"], rows, "the plan is the result");
    assert_eq!(
        (out["added"].as_u64(), out["unavailable"].as_u64()),
        (Some(2), Some(1))
    );

    let again = call(
        &mut server,
        "cad.standards.install",
        json!({"drawing":drawing,"op":"plan"}),
    );
    let rows = &again["structuredContent"]["output"]["rows"];
    assert_eq!(rows[0]["status"], "same");
    assert_eq!(rows[2]["status"], "same");
}

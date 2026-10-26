//! The real MCP host exposes the same preview, refusal, write and single undo.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn call(server: &mut Server, name: &str, args: Value) -> Value {
    server.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"room-number-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}
#[test]
fn numbering_plan_keeps_ids_and_moved_tag_then_one_undo_restores_everything() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.spaces.number.json"
    ))
    .unwrap();
    let path = std::env::temp_dir().join(format!("kentos-room-number-{}.json", std::process::id()));
    std::fs::write(&path, fixture["setup"].to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let items=call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let id = |slot: u64| {
        items
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["entity"]["id"].as_u64() == Some(slot))
            .unwrap()["uid"]
            .clone()
    };
    let tags = call(
        &mut server,
        "cad.tags.create",
        json!({"drawing":drawing,"layerId":"duvar","tags":[{"of":id(1),"at":{"x":-2.,"y":5.},"content":"{number}\n{floorElevation}/{ceilingElevation}","leader":true}]}),
    );
    assert_eq!(tags["isError"], false, "{tags}");
    let before=call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let args = json!({"drawing":drawing,"uids":[id(4),id(2),id(3),id(1)],"first":"Z-009","order":{"kind":"direction","from":{"x":0.,"y":0.},"to":{"x":1.,"y":0.}}});
    let mut preview = args.clone();
    preview["op"] = json!("plan");
    let plan = call(&mut server, "cad.spaces.number", preview);
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(
        plan["structuredContent"]["output"]["kept"],
        json!([id(1), id(3), id(2), id(4)])
    );
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        before
    );
    let made = call(&mut server, "cad.spaces.number", args);
    assert_eq!(made["isError"], false, "{made}");
    assert_eq!(
        made["structuredContent"]["output"]["spaces"],
        plan["structuredContent"]["output"]["spaces"]
    );
    let after=call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let tag = after
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["entity"]["kind"] == "tag")
        .unwrap();
    assert_eq!(tag["entity"]["text"], "Z-009\n0.000/3.000");
    assert_eq!(tag["entity"]["at"], json!({"x":-2.,"y":5.}));
    assert_eq!(tag["entity"]["of"], id(1));
    let refused = call(
        &mut server,
        "cad.spaces.number",
        json!({"drawing":drawing,"uids":[id(1)],"first":"Z-010"}),
    );
    assert_eq!(refused["isError"], true);
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "space_number_conflict"
    );
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        after
    );
    call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        before
    );
    call(&mut server, "drawing.redo", json!({"drawing":drawing}));
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        after
    );
    std::fs::remove_file(path).unwrap();
}

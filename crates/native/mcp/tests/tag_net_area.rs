//! A real MCP plan/write/undo uses the same inner-face area, preserving the gross source.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn call(server: &mut Server, name: &str, args: Value) -> Value {
    server.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"net-tag-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}
#[test]
fn net_tag_plan_matches_write_and_wall_change_is_one_undo() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.tags.create.json"
    ))
    .unwrap();
    let setup = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["name"]
                .as_str()
                .unwrap()
                .starts_with("Net alan gross/.2/reverse=False")
        })
        .unwrap()["setup"]
        .clone();
    let path = std::env::temp_dir().join(format!("kentos-net-tag-{}.json", std::process::id()));
    std::fs::write(&path, setup.to_string()).unwrap();
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
    let args = json!({"drawing":drawing,"layerId":"etiket","tags":[{"of":id(18),"at":{"x":10.,"y":5.},"content":"{area}/{netArea}","leader":true}]});
    let mut preview = args.clone();
    preview["op"] = json!("plan");
    let planned = call(&mut server, "cad.tags.create", preview);
    assert_eq!(planned["isError"], false, "{planned}");
    assert_eq!(
        planned["structuredContent"]["output"]["tags"][0]["text"],
        "26.04 m²/22.04 m²"
    );
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        items
    );
    let made = call(&mut server, "cad.tags.create", args);
    assert_eq!(made["isError"], false, "{made}");
    let before=call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let changed = call(
        &mut server,
        "cad.walls.edit",
        json!({"drawing":drawing,"uids":[id(17)],"operation":{"kind":"set","width":0.4}}),
    );
    assert_eq!(changed["isError"], false, "{changed}");
    let after=call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let tag = after
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["entity"]["kind"] == "tag")
        .unwrap();
    assert_eq!(tag["entity"]["text"], "28.16 m²/20.16 m²");
    assert!((tag["entity"]["at"]["x"].as_f64().unwrap() - 10.).abs() < 1e-9);
    assert!((tag["entity"]["at"]["y"].as_f64().unwrap() - 5.).abs() < 1e-9);
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

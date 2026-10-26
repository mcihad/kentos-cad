//! The actual MCP host reviews room identities through the existing query command.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    server.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"room-check-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn room_number_review_is_read_only_and_repair_keeps_the_existing_single_undo() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.tags.check.json"
    ))
    .unwrap();
    let setup = &fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["name"]
                .as_str()
                .unwrap()
                .starts_with("Numarasız mahal opt-in")
        })
        .unwrap()["setup"];
    let path = std::env::temp_dir().join(format!("kentos-room-check-{}.json", std::process::id()));
    std::fs::write(&path, setup.to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let before = call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let uids: Vec<_> = before
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uid"].clone())
        .collect();
    let mut first = Value::Null;
    for op in ["plan", "execute"] {
        let answer = call(
            &mut server,
            "cad.tags.check",
            json!({"drawing":drawing,"op":op,"missingSpaceNumbers":true}),
        );
        assert_eq!(answer["isError"], false, "{answer}");
        let output = answer["structuredContent"]["output"].clone();
        let rows: Vec<_> = output["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["code"] == "tag_missing_space_number")
            .collect();
        assert_eq!(rows.len(), 3);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row["uids"], json!([uids[i]]));
        }
        if first.is_null() {
            first = output;
        } else {
            assert_eq!(output, first);
        }
    }
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        before
    );
    let repaired = call(
        &mut server,
        "cad.spaces.edit",
        json!({"drawing":drawing,"uids":[uids[0]],"operation":{"kind":"set","number":"N-001"}}),
    );
    assert_eq!(repaired["isError"], false, "{repaired}");
    let reviewed = call(
        &mut server,
        "cad.tags.check",
        json!({"drawing":drawing,"missingSpaceNumbers":true}),
    );
    assert_eq!(
        reviewed["structuredContent"]["output"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["code"] == "tag_missing_space_number")
            .count(),
        2
    );
    let undone = call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(undone["isError"], false, "{undone}");
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        before
    );
    std::fs::remove_file(path).unwrap();
}

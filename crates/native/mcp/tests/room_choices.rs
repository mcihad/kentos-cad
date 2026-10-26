//! The actual AI host shares advisory project settings and existing room edit/history.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    server.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"room-choices-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn project_lists_remain_advisory_and_survive_room_undo_and_real_reopen() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.tags.create.json"
    ))
    .unwrap();
    let path =
        std::env::temp_dir().join(format!("kentos-room-choices-{}.json", std::process::id()));
    let saved = path.with_extension("kcad");
    std::fs::write(&path, fixture["setup"].to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let entities = |s: &mut Server| {
        call(s, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"]
            .clone()
    };
    let before = entities(&mut server);
    let room = before
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["entity"]["kind"] == "space")
        .unwrap()["uid"]
        .clone();
    // arch is a whole replacement: copy existing levels and every other option.
    let mut arch = fixture["setup"]["settings"]["arch"].clone();
    arch["spaceNames"] = json!(["Mutfak", "mutfak", "Çalışma"]);
    arch["spaceUsages"] = json!(["Konut", "Ofis"]);
    arch["spaceNumberPrefixes"] = json!({"zemin":"G-","kat-1":"01-"});
    for op in ["plan", "execute"] {
        let result = call(
            &mut server,
            "cad.settings.set",
            json!({"drawing":drawing,"op":op,"arch":arch}),
        );
        assert_eq!(result["isError"], false, "{result}");
        assert_eq!(entities(&mut server), before);
        assert_eq!(
            call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["canUndo"],
            false
        );
    }
    let edited = call(
        &mut server,
        "cad.spaces.edit",
        json!({"drawing":drawing,"uids":[room],"operation":{"kind":"set","name":"Serbest atölye","usage":"Özel","number":"G-001"}}),
    );
    assert_eq!(edited["isError"], false, "{edited}");
    assert_ne!(entities(&mut server), before);
    let undone = call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(undone["isError"], false, "{undone}");
    assert_eq!(entities(&mut server), before);
    let summary = call(&mut server, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(summary["structuredContent"]["settings"]["arch"], arch);
    assert_eq!(summary["structuredContent"]["canUndo"], false);
    assert_eq!(summary["structuredContent"]["canRedo"], true);
    let mut invalid = arch.clone();
    invalid["spaceNames"] = json!(["Mutfak", "Mutfak"]);
    let refused = call(
        &mut server,
        "cad.settings.set",
        json!({"drawing":drawing,"arch":invalid}),
    );
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_arch"
    );
    assert_eq!(entities(&mut server), before);
    let result = call(
        &mut server,
        "drawing.save",
        json!({"drawing":drawing,"path":saved}),
    );
    assert_eq!(result["isError"], false, "{result}");
    // Read the real saved drawing rather than trusting a write response.
    let reopened = call(&mut server, "drawing.open", json!({"path":saved}));
    assert_eq!(reopened["isError"], false, "{reopened}");
    let back = reopened["structuredContent"]["drawing"].clone();
    let summary = call(&mut server, "drawing.summary", json!({"drawing":back}));
    assert_eq!(
        summary["structuredContent"]["settings"]["arch"], arch,
        "{summary}"
    );
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":back}))["structuredContent"]["items"],
        before
    );
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(saved).unwrap();
}

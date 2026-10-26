//! Real AI host: a combined create/refresh has one history step and persistent ids.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"tag-refresh-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}
// A document stamp is a host token, not a semantic entity field. Compare full
// UUID/entity models independently of those tokens, including after reopening.
fn model(mut value: Value) -> Value {
    for row in value.as_array_mut().unwrap() {
        row.as_object_mut().unwrap().remove("stamp");
    }
    value
}
#[test]
fn planned_refresh_keeps_manual_place_and_single_undo_with_new_tag_and_reopen() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.tags.create.json"
    ))
    .unwrap();
    let path = std::env::temp_dir().join(format!("kentos-tag-refresh-{}.json", std::process::id()));
    let saved = path.with_extension("kcad");
    std::fs::write(&path, fixture["setup"].to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let entities = |s: &mut Server| {
        model(
            call(s, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"]
                .clone(),
        )
    };
    let initial = entities(&mut server);
    let room = initial
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["entity"]["kind"] == "space")
        .unwrap()["uid"]
        .clone();
    let added = call(
        &mut server,
        "cad.tags.create",
        json!({"drawing":drawing,"layerId":"etiket","tags":[{"of":room,"at":{"x":-4.,"y":8.},"content":"{name}","size":4.,"leader":true}]}),
    );
    assert_eq!(added["isError"], false, "{added}");
    let uid = added["structuredContent"]["output"]["uids"][0].clone();
    let before = entities(&mut server);
    let mut args = json!({"drawing":drawing,"layerId":"etiket","all":{"kinds":["space"],"refresh":true,"content":"{number} / {name}"},"op":"plan"});
    let plan = call(&mut server, "cad.tags.create", args.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(entities(&mut server), before);
    let planned = &plan["structuredContent"]["output"];
    assert_eq!(planned["tags"].as_array().unwrap().len(), 1);
    assert_eq!(planned["refreshed"].as_array().unwrap().len(), 1);
    args["op"] = json!("execute");
    args["expectedRevision"] = planned["revision"].clone();
    let result = call(&mut server, "cad.tags.create", args.clone());
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(
        result["structuredContent"]["output"]["refreshed"],
        json!([uid])
    );
    let after = entities(&mut server);
    let mut expected = before.clone();
    let changed = expected
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["uid"] == uid)
        .unwrap();
    changed["entity"]["content"] = json!("{number} / {name}");
    changed["entity"]["text"] = json!("101 / Salon");
    let new_uid = result["structuredContent"]["output"]["uids"][0].clone();
    let id = result["structuredContent"]["output"]["ids"][0].clone();
    let second_room = initial
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["entity"]["kind"] == "space")
        .nth(1)
        .unwrap()["uid"]
        .clone();
    // Compare complete entities, including unrelated sources and original tag placement/style.
    let new = after
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["uid"] == new_uid)
        .unwrap();
    assert_eq!(
        new["entity"],
        json!({"kind":"tag","id":id,"layerId":"etiket","attrs":{},"of":second_room,"at":{"x":26.5,"y":1.},"anchor":{"x":26.5,"y":1.},"content":"{number} / {name}","text":"102 / Mutfak","shape":"rectangle"})
    );
    expected.as_array_mut().unwrap().push(new.clone());
    assert_eq!(after, expected);
    assert_eq!(
        call(&mut server, "drawing.undo", json!({"drawing":drawing}))["isError"],
        false
    );
    assert_eq!(entities(&mut server), before);
    assert_eq!(
        call(&mut server, "drawing.redo", json!({"drawing":drawing}))["isError"],
        false
    );
    assert_eq!(entities(&mut server), after);
    assert_eq!(
        call(
            &mut server,
            "drawing.save",
            json!({"drawing":drawing,"path":saved})
        )["isError"],
        false
    );
    let reopened = call(&mut server, "drawing.open", json!({"path":saved}));
    assert_eq!(reopened["isError"], false, "{reopened}");
    let back = reopened["structuredContent"]["drawing"].clone();
    let reloaded = model(call(&mut server,"drawing.entities",json!({"drawing":back}))["structuredContent"]["items"].clone());
    assert_eq!(reloaded, after);
    args["op"] = json!("execute");
    args.as_object_mut().unwrap().remove("expectedRevision");
    let noop = call(&mut server, "cad.tags.create", args);
    assert_eq!(noop["isError"], true);
    assert_eq!(noop["structuredContent"]["error"]["code"], "no_objects");
    assert_eq!(entities(&mut server), after);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(saved).unwrap();
}

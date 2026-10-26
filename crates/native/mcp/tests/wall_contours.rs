//! Contours are semantic input, including source holes; no client offset math.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn request(server: &mut Server, method: &str, mut params: Value) -> Value {
    params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"architecture-contour-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
    server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .unwrap()["result"]
        .clone()
}
fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let r = request(server, "tools/call", json!({"name":name,"arguments":args}));
    assert_eq!(r["resultType"], "complete", "{r}");
    r
}
#[test]
fn contour_schema_plan_holes_one_undo_and_duplicate_refusal_use_the_real_host() {
    let mut server = Server::new();
    let listed = request(&mut server, "tools/list", json!({}));
    let tool = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "cad.walls.convert")
        .unwrap();
    assert!(
        tool["inputSchema"]["$defs"]["WallsFromMode"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["const"] == "contours")
    );
    let new = call(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = new["structuredContent"]["drawing"].clone();
    let layer = new["structuredContent"]["summary"]["activeLayer"].clone();
    let made = call(
        &mut server,
        "cad.polygon.create",
        json!({"drawing":drawing,"layerId":layer,"pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":8},{"x":0,"y":8}],"holes":[{"pts":[{"x":4,"y":3},{"x":6,"y":3},{"x":6,"y":5},{"x":4,"y":5}]}],"attrs":{"office":"A"}}),
    );
    assert_eq!(made["isError"], false, "{made}");
    let uid = made["structuredContent"]["output"]["uid"].clone();
    let mut input = json!({"drawing":drawing,"uids":[uid],"mode":"contours","justify":"left","keepSources":true,"op":"plan"});
    let plan = call(&mut server, "cad.walls.convert", input.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(
        plan["structuredContent"]["output"]["walls"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        1
    );
    input["op"] = json!("execute");
    input["expectedRevision"] = plan["structuredContent"]["output"]["revision"].clone();
    let written = call(&mut server, "cad.walls.convert", input.clone());
    assert_eq!(written["isError"], false, "{written}");
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        3
    );
    call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        1
    );
    call(&mut server, "drawing.redo", json!({"drawing":drawing}));
    input.as_object_mut().unwrap().remove("expectedRevision");
    let refused = call(&mut server, "cad.walls.convert", input);
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_wall"
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        3
    );
}

#[test]
fn shared_axes_use_the_same_headless_plan_and_one_undo() {
    let mut server = Server::new();
    let new = call(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = new["structuredContent"]["drawing"].clone();
    let layer = new["structuredContent"]["summary"]["activeLayer"].clone();
    let mut uids = Vec::new();
    for x in [0, 4] {
        let made = call(
            &mut server,
            "cad.polygon.create",
            json!({"drawing":drawing,"layerId":layer,"pts":[{"x":x,"y":0},{"x":x+4,"y":0},{"x":x+4,"y":3},{"x":x,"y":3}],"attrs":{"room":x.to_string()}}),
        );
        uids.push(made["structuredContent"]["output"]["uid"].clone());
    }
    let mut input = json!({"drawing":drawing,"uids":uids,"mode":"contours","justify":"center","keepSources":true,"op":"plan"});
    let plan = call(&mut server, "cad.walls.convert", input.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(
        plan["structuredContent"]["output"]["walls"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        2
    );
    input["op"] = json!("execute");
    input["expectedRevision"] = plan["structuredContent"]["output"]["revision"].clone();
    let result = call(&mut server, "cad.walls.convert", input.clone());
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        5
    );
    call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        2
    );
    call(&mut server, "drawing.redo", json!({"drawing":drawing}));
    input.as_object_mut().unwrap().remove("expectedRevision");
    assert_eq!(
        call(&mut server, "cad.walls.convert", input)["isError"],
        true
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        5
    );
}

#[test]
fn explicit_conversion_levels_share_the_native_schema_validation_and_duplicate_scope() {
    let mut server = Server::new();
    let new = call(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = new["structuredContent"]["drawing"].clone();
    let layer = new["structuredContent"]["summary"]["activeLayer"].clone();
    let settings = call(
        &mut server,
        "cad.settings.set",
        json!({"drawing":drawing,"arch":{"levels":[
            {"id":"zemin","name":"Zemin kat","elevation":0,"height":3},
            {"id":"kat-1","name":"1. kat","elevation":0,"height":3}
        ]}}),
    );
    assert_eq!(settings["isError"], false, "{settings}");
    let source = call(
        &mut server,
        "cad.polygon.create",
        json!({"drawing":drawing,"layerId":layer,"pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":3},{"x":0,"y":3}],"attrs":{"level":"kat-1"}}),
    );
    let uid = source["structuredContent"]["output"]["uid"].clone();
    for (i, level) in [None, Some("zemin"), Some("kat-1")].into_iter().enumerate() {
        let mut args = json!({"drawing":drawing,"uids":[uid],"mode":"contours","justify":"center","keepSources":true,"op":"plan"});
        if let Some(id) = level {
            args["arch"] = json!({"level":id});
        }
        let plan = call(&mut server, "cad.walls.convert", args.clone());
        assert_eq!(plan["isError"], false, "{plan}");
        assert_eq!(
            plan["structuredContent"]["output"]["walls"][0]["arch"]["level"],
            json!(level)
        );
        assert_eq!(
            call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
            i + 1
        );
        args["op"] = json!("execute");
        let done = call(&mut server, "cad.walls.convert", args.clone());
        assert_eq!(done["isError"], false, "{done}");
        let duplicate = call(&mut server, "cad.walls.convert", args);
        assert_eq!(
            duplicate["structuredContent"]["error"]["code"],
            "invalid_wall"
        );
    }
    let refused = call(
        &mut server,
        "cad.walls.convert",
        json!({"drawing":drawing,"uids":[uid],"mode":"contours","keepSources":false,"arch":{"level":"missing-floor"}}),
    );
    assert_eq!(
        refused["structuredContent"]["error"]["code"], "level_not_found",
        "{refused}"
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        4
    );
}

#[test]
fn measured_mixed_sources_use_the_same_plan_and_atomic_history_through_mcp() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/walls/v1/mixed-contours.json"
    ))
    .unwrap();
    let mut setup: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.walls.convert.json"
    ))
    .unwrap();
    setup["setup"]["entities"] = fixture["sources"].clone();
    let path = std::env::temp_dir().join(format!(
        "kentos-mcp-mixed-contours-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, setup["setup"].to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let page = call(&mut server, "drawing.entities", json!({"drawing":drawing}));
    let originals = page["structuredContent"]["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(originals.len(), 7);
    let uids = originals
        .iter()
        .map(|r| r["uid"].clone())
        .collect::<Vec<_>>();
    let mut args = json!({"drawing":drawing,"uids":uids,"mode":"contours","justify":"center","keepSources":true,"width":0.2,"op":"plan"});
    let plan = call(&mut server, "cad.walls.convert", args.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    let walls = plan["structuredContent"]["output"]["walls"]
        .as_array()
        .unwrap();
    assert_eq!(walls.len(), 9);
    assert_eq!(walls.iter().filter(|w| w["closed"] == true).count(), 6);
    assert_eq!(
        walls
            .iter()
            .filter(|w| w.get("bulges").is_some_and(|b| b.is_array()))
            .count(),
        2
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        7
    );
    args["op"] = json!("execute");
    args["expectedRevision"] = plan["structuredContent"]["output"]["revision"].clone();
    let done = call(&mut server, "cad.walls.convert", args.clone());
    assert_eq!(done["isError"], false, "{done}");
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        16
    );
    call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    let restored = call(&mut server, "drawing.entities", json!({"drawing":drawing}));
    assert_eq!(restored["structuredContent"]["items"], json!(originals));
    call(&mut server, "drawing.redo", json!({"drawing":drawing}));
    args.as_object_mut().unwrap().remove("expectedRevision");
    let refused = call(&mut server, "cad.walls.convert", args);
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_wall"
    );
    assert_eq!(
        call(&mut server, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        16
    );
    std::fs::remove_file(path).unwrap();
}

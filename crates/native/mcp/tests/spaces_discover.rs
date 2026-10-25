//! Real MCP host: explicit semantic levels, plan/execute, quantities and undo.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"space-level-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities":{}}}}),
        )
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}
#[test]
fn discover_is_available_through_real_mcp_with_preview_atomic_write_repeat_and_history() {
    let mut setup: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.walls.convert.json"
    ))
    .unwrap();
    setup["setup"]["entities"] = json!([
        {"kind":"wall","id":1,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"arch":{"level":"zemin"},"pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":8},{"x":0,"y":8}]},
        {"kind":"wall","id":2,"layerId":"cizim","attrs":{},"width":0.2,"arch":{"level":"zemin"},"pts":[{"x":5,"y":0},{"x":5,"y":8}]}]);
    let path =
        std::env::temp_dir().join(format!("kentos-mcp-discover-{}.json", std::process::id()));
    std::fs::write(&path, setup["setup"].to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let original=call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let args = json!({"drawing":drawing,"layerId":"cizim","level":"zemin"});
    let mut planned = args.clone();
    planned["op"] = json!("plan");
    let plan = call(&mut server, "cad.spaces.discover", planned.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(
        plan["structuredContent"]["output"]["proposals"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        original
    );
    let done = call(&mut server, "cad.spaces.discover", args.clone());
    assert_eq!(done["isError"], false, "{done}");
    assert_eq!(
        done["structuredContent"]["output"]["uids"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let repeat = call(&mut server, "cad.spaces.discover", planned);
    assert!(
        repeat["structuredContent"]["output"]["proposals"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["existing"].as_array().unwrap().len() == 1)
    );
    let empty = call(&mut server, "cad.spaces.discover", args);
    assert_eq!(empty["isError"], false, "{empty}");
    assert!(
        empty["structuredContent"]["output"]["uids"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        original
    );
    call(&mut server, "drawing.redo", json!({"drawing":drawing}));
    let q = call(
        &mut server,
        "cad.spaces.quantities",
        json!({"drawing":drawing}),
    );
    assert!(
        (q["structuredContent"]["output"]["totals"]["spaceArea"]
            .as_f64()
            .unwrap()
            - 2. * 4.8 * 7.8)
            .abs()
            < 1e-7
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn persistent_column_policy_reaches_real_mcp_plan_quantities_edit_and_one_undo() {
    let setup: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.spaces.columns.json"
    )).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-mcp-columns-{}.json", std::process::id()));
    std::fs::write(&path, setup["setup"].to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let original = call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let args = json!({"drawing":drawing,"layerId":"mahal","level":"zemin","columns":"net"});
    let mut preview = args.clone();
    preview["op"] = json!("plan");
    let planned = call(&mut server,"cad.spaces.discover",preview);
    assert_eq!(planned["isError"],false,"{planned}");
    assert_eq!(planned["structuredContent"]["output"]["proposals"][0]["space"]["columns"],"net");
    assert_eq!(call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"],original);
    let made = call(&mut server,"cad.spaces.discover",args);
    assert_eq!(made["isError"],false,"{made}");
    let uids = made["structuredContent"]["output"]["uids"].clone();
    assert_eq!(uids.as_array().unwrap().len(),1);
    let q = call(&mut server,"cad.spaces.quantities",json!({"drawing":drawing}));
    let row = &q["structuredContent"]["output"]["spaces"][0];
    assert_eq!(row["columns"],"net");
    assert!((row["area"].as_f64().unwrap()-70.44).abs()<1e-7);
    let changed = call(&mut server,"cad.spaces.edit",json!({"drawing":drawing,"uids":uids,"operation":{"kind":"set","columns":"all","method":"gross"}}));
    assert_eq!(changed["isError"],false,"{changed}");
    let q = call(&mut server,"cad.spaces.quantities",json!({"drawing":drawing}));
    let row = &q["structuredContent"]["output"]["spaces"][0];
    assert_eq!(row["columns"],"all");
    assert!((row["area"].as_f64().unwrap()-77.64).abs()<1e-7);
    call(&mut server,"drawing.undo",json!({"drawing":drawing}));
    let q = call(&mut server,"cad.spaces.quantities",json!({"drawing":drawing}));
    assert_eq!(q["structuredContent"]["output"]["spaces"][0]["uid"],uids[0]);
    assert_eq!(q["structuredContent"]["output"]["spaces"][0]["columns"],"net");
    call(&mut server,"drawing.undo",json!({"drawing":drawing}));
    assert_eq!(call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"],original);
    std::fs::remove_file(path).unwrap();
}

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
fn room_levels_isolate_overlapping_rooms_plan_and_persist_one_history_step() {
    let mut setup: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.walls.convert.json"
    ))
    .unwrap();
    for l in setup["setup"]["settings"]["arch"]["levels"]
        .as_array_mut()
        .unwrap()
    {
        l["elevation"] = json!(0);
    }
    setup["setup"]["entities"] = json!([
        {"kind":"wall","id":1,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"arch":{"level":"zemin"},
            "pts":[{"x":0,"y":0},{"x":6,"y":0},{"x":6,"y":4},{"x":0,"y":4}]},
        {"kind":"wall","id":2,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"arch":{"level":"kat-1"},
            "pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":8},{"x":0,"y":8}]}]);
    let path = std::env::temp_dir().join(format!(
        "kentos-mcp-space-levels-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, setup["setup"].to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let original=call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let args = json!({"drawing":drawing,"layerId":"cizim","spaces":[{"seed":{"x":2,"y":2},"arch":{"level":"kat-1"}}]});
    let mut plan_args = args.clone();
    plan_args["op"] = json!("plan");
    let planned = call(&mut server, "cad.spaces.create", plan_args);
    assert_eq!(planned["isError"], false, "{planned}");
    assert_eq!(
        planned["structuredContent"]["output"]["spaces"][0]["arch"]["level"],
        "kat-1"
    );
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        original
    );
    let done = call(&mut server, "cad.spaces.create", args);
    assert_eq!(done["isError"], false, "{done}");
    let quantities = call(
        &mut server,
        "cad.spaces.quantities",
        json!({"drawing":drawing}),
    );
    assert!(
        (quantities["structuredContent"]["output"]["spaces"][0]["area"]
            .as_f64()
            .unwrap()
            - 9.8 * 7.8)
            .abs()
            < 1e-8
    );
    call(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"],
        original
    );
    call(&mut server, "drawing.redo", json!({"drawing":drawing}));
    let mut bad = json!({"drawing":drawing,"layerId":"cizim","spaces":[{"seed":{"x":8,"y":6},"arch":{"level":"zemin"}}]});
    let before=call(&mut server,"drawing.entities",json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    for op in ["plan", "execute"] {
        bad["op"] = json!(op);
        let r = call(&mut server, "cad.spaces.create", bad.clone());
        assert_eq!(
            r["structuredContent"]["error"]["code"], "space_not_enclosed",
            "{r}"
        );
        assert_eq!(
            call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]
                ["items"],
            before
        );
    }
    std::fs::remove_file(path).unwrap();
}

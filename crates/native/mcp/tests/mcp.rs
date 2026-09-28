//! The MCP server end to end (docs/adr/0133): a modern client's requests
//! with their `_meta`, a legacy client's handshake, the tools on a drawing
//! (a polygon planned, written, measured, saved and read back), the
//! refusals a model can act on, the resources, and the real process on stdio.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use kentos_mcp::{LEGACY, MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientInfo": { "name": "test", "version": "1" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

/// A modern request's answer.
fn ask(server: &mut Server, id: u64, method: &str, mut params: Value) -> Value {
    params["_meta"] = meta();
    server
        .handle(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
        .expect("an answer")
}

/// A tool call's result.
fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
    let answer = ask(
        server,
        7,
        "tools/call",
        json!({ "name": name, "arguments": arguments }),
    );
    assert_eq!(answer["result"]["resultType"], "complete", "{answer}");
    answer["result"].clone()
}

fn square() -> Value {
    json!([
        {"x": 423500.0, "y": 4512300.0},
        {"x": 423520.0, "y": 4512300.0},
        {"x": 423520.0, "y": 4512312.5},
        {"x": 423500.0, "y": 4512312.5}
    ])
}

#[test]
fn a_modern_client_discovers_lists_and_draws_a_measured_polygon_it_saves_and_reads_back() {
    let mut server = Server::new();
    let found = ask(&mut server, 1, "server/discover", json!({}));
    assert_eq!(found["result"]["supportedVersions"], json!([MODERN]));
    assert_eq!(found["result"]["capabilities"]["tools"], json!({}));
    assert_eq!(
        found["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "kentos-mcp"
    );
    assert!(
        found["result"]["instructions"]
            .as_str()
            .is_some_and(|t| t.contains("drawing"))
    );

    let listed = ask(&mut server, 2, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for name in [
        "drawing.new",
        "drawing.open",
        "drawing.entities",
        "cad.polygon.create",
        "cad.entities.set",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    assert!(
        !names.iter().any(|n| n.starts_with("project.")),
        "the server's commands are not here"
    );
    let polygon = tools
        .iter()
        .find(|t| t["name"] == "cad.polygon.create")
        .expect("the command");
    assert_eq!(polygon["inputSchema"]["required"][0], "drawing");
    assert_eq!(
        polygon["inputSchema"]["properties"]["op"]["enum"],
        json!(["execute", "plan", "validate"])
    );
    assert!(polygon["inputSchema"]["$defs"].is_object());
    // Again the same list, in the same order.
    assert_eq!(
        ask(&mut server, 3, "tools/list", json!({}))["result"]["tools"],
        listed["result"]["tools"]
    );

    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Ada 101", "srid": 5256 }),
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let layer = made["structuredContent"]["summary"]["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();

    let plan = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "op": "plan", "layerId": layer, "pts": square() }),
    );
    assert_eq!(plan["isError"], false);
    assert_eq!(plan["structuredContent"]["status"], "completed");
    assert_eq!(
        plan["structuredContent"]["output"]["entity"]["kind"],
        "polygon"
    );
    let summary = call(
        &mut server,
        "drawing.summary",
        json!({ "drawing": drawing }),
    );
    assert_eq!(
        summary["structuredContent"]["objects"], 0,
        "a plan writes nothing"
    );

    let written = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "layerId": layer, "pts": square() }),
    );
    let uid = written["structuredContent"]["output"]["uid"]
        .as_str()
        .expect("its id")
        .to_owned();
    // The text is the same JSON, for clients that read only text.
    let text: Value =
        serde_json::from_str(written["content"][0]["text"].as_str().expect("text")).expect("JSON");
    assert_eq!(text, written["structuredContent"]);

    let measured = call(
        &mut server,
        "drawing.measure",
        json!({ "drawing": drawing, "uid": uid }),
    );
    assert_eq!(measured["structuredContent"]["area"], 250.0);
    let page = call(
        &mut server,
        "drawing.entities",
        json!({ "drawing": drawing, "kinds": ["polygon"], "bbox": [423499, 4512299, 423501, 4512301] }),
    );
    assert_eq!(page["structuredContent"]["items"][0]["uid"], uid.as_str());

    let dir = std::env::temp_dir().join(format!("kentos-mcp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("ada.kcad");
    let saved = call(
        &mut server,
        "drawing.save",
        json!({ "drawing": drawing, "path": path }),
    );
    assert_eq!(saved["isError"], false, "{saved}");
    let closed = call(&mut server, "drawing.close", json!({ "drawing": drawing }));
    assert_eq!(closed["isError"], false, "saved, so it closes");

    let again = call(&mut server, "drawing.open", json!({ "path": path }));
    let reopened = again["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    assert_ne!(reopened, drawing, "a new handle");
    let measured = call(
        &mut server,
        "drawing.measure",
        json!({ "drawing": reopened, "uid": uid }),
    );
    assert_eq!(
        measured["structuredContent"]["area"], 250.0,
        "the same id, the same area"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn refusals_a_model_can_act_on_are_results_and_malformed_requests_are_errors() {
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", json!({ "srid": 5256 }));
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();

    let refused = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "layerId": "yok", "pts": square() }),
    );
    assert_eq!(refused["isError"], true);
    assert_eq!(refused["structuredContent"]["status"], "failed");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "layer_not_found"
    );

    let shape = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "layerId": 3 }),
    );
    assert_eq!(shape["isError"], true);
    assert_eq!(shape["structuredContent"]["error"]["code"], "invalid_input");
    let unknown = call(&mut server, "drawing.summary", json!({ "drawing": "yok" }));
    assert_eq!(
        unknown["structuredContent"]["error"]["code"],
        "unknown_drawing"
    );
    let no_crs = call(&mut server, "drawing.new", json!({ "name": "CRS'siz" }));
    assert_eq!(
        no_crs["structuredContent"]["error"]["code"],
        "invalid_input"
    );

    // Unsaved work is not thrown away unasked.
    call(
        &mut server,
        "cad.point.create",
        json!({ "drawing": drawing, "layerId": made["structuredContent"]["summary"]["activeLayer"], "p": {"x": 1.0, "y": 2.0} }),
    );
    let kept = call(&mut server, "drawing.close", json!({ "drawing": drawing }));
    assert_eq!(kept["structuredContent"]["error"]["code"], "unsaved");
    let gone = call(
        &mut server,
        "drawing.close",
        json!({ "drawing": drawing, "discard": true }),
    );
    assert_eq!(gone["isError"], false);

    let no_tool = ask(
        &mut server,
        9,
        "tools/call",
        json!({ "name": "yok", "arguments": {} }),
    );
    assert_eq!(no_tool["error"]["code"], -32602);
    let no_method = ask(&mut server, 10, "yok/yok", json!({}));
    assert_eq!(no_method["error"]["code"], -32601);
    // No version, no handshake: malformed.
    let bare = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 11, "method": "tools/list" }))
        .expect("an answer");
    assert_eq!(bare["error"]["code"], -32602);
    // A version this server does not speak: which it does.
    let old = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 12, "method": "tools/list",
            "params": { "_meta": { "io.modelcontextprotocol/protocolVersion": "1900-01-01",
                                   "io.modelcontextprotocol/clientCapabilities": {} } } }))
        .expect("an answer");
    assert_eq!(old["error"]["code"], -32022);
    assert_eq!(
        old["error"]["data"],
        json!({ "supported": [MODERN], "requested": "1900-01-01" })
    );
    let no_capabilities = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 13, "method": "tools/list",
            "params": { "_meta": { "io.modelcontextprotocol/protocolVersion": MODERN } } }))
        .expect("an answer");
    assert_eq!(no_capabilities["error"]["code"], -32602);
}

#[test]
fn a_legacy_client_shakes_hands_and_is_served_without_meta() {
    let mut server = Server::new();
    let hello = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "eski", "version": "1" } } }))
        .expect("an answer");
    assert_eq!(hello["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(hello["result"]["serverInfo"]["name"], "kentos-mcp");
    assert!(hello["result"]["capabilities"]["tools"].is_object());
    // A version it does not know gets the newest legacy one.
    let mut other = Server::new();
    let newest = other
        .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2024-01-01" } }))
        .expect("an answer");
    assert_eq!(newest["result"]["protocolVersion"], LEGACY[0]);

    assert!(
        server
            .handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .is_none(),
        "a notification has no answer"
    );
    let listed = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))
        .expect("an answer");
    assert!(
        listed["result"]["tools"]
            .as_array()
            .is_some_and(|t| !t.is_empty())
    );
    assert!(
        listed["result"].get("resultType").is_none(),
        "legacy results as legacy clients know them"
    );
    let pong = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 3, "method": "ping" }))
        .expect("an answer");
    assert_eq!(pong["result"], json!({}));
}

#[test]
fn the_catalog_and_the_open_drawings_are_resources() {
    let mut server = Server::new();
    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Kaynak", "srid": 5256 }),
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let listed = ask(&mut server, 1, "resources/list", json!({}));
    let uris: Vec<&str> = listed["result"]["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .filter_map(|r| r["uri"].as_str())
        .collect();
    assert_eq!(
        uris,
        [
            "kentos://catalog".to_owned(),
            format!("kentos://drawing/{drawing}")
        ]
    );
    let catalog = ask(
        &mut server,
        2,
        "resources/read",
        json!({ "uri": "kentos://catalog" }),
    );
    let text = catalog["result"]["contents"][0]["text"]
        .as_str()
        .expect("text");
    let parsed: Value = serde_json::from_str(text).expect("JSON");
    assert_eq!(parsed["format"], "kentos.commands");
    let one = ask(
        &mut server,
        3,
        "resources/read",
        json!({ "uri": format!("kentos://drawing/{drawing}") }),
    );
    assert!(
        one["result"]["contents"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("Kaynak"))
    );
    let none = ask(
        &mut server,
        4,
        "resources/read",
        json!({ "uri": "kentos://yok" }),
    );
    assert_eq!(none["error"]["code"], -32602);
}

#[test]
fn the_process_speaks_json_lines_on_stdio() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kentos-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the server starts");
    let mut stdin = child.stdin.take().expect("its input");
    let mut lines = BufReader::new(child.stdout.take().expect("its output")).lines();
    let mut send = |message: Value| {
        writeln!(stdin, "{message}").expect("written");
        stdin.flush().expect("flushed");
    };
    send(
        json!({ "jsonrpc": "2.0", "id": 1, "method": "server/discover", "params": { "_meta": meta() } }),
    );
    send(json!({ "jsonrpc": "2.0", "method": "notifications/cancelled", "params": {} }));
    send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "_meta": meta(), "name": "drawing.new", "arguments": { "srid": 5256 } } }));
    let mut read = || -> Value {
        serde_json::from_str(&lines.next().expect("a line").expect("read")).expect("JSON")
    };
    let first = read();
    assert_eq!(first["id"], 1);
    assert_eq!(first["result"]["supportedVersions"][0], MODERN);
    let second = read();
    assert_eq!(second["id"], 2, "the notification had no answer");
    assert_eq!(second["result"]["isError"], false);
    send(json!("bozuk"));
    let broken: Value =
        serde_json::from_str(&lines.next().expect("a line").expect("read")).expect("JSON");
    assert_eq!(broken["error"]["code"], -32600);
    child.kill().expect("stopped");
    let _ = child.wait();
}

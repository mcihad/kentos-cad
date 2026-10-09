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
        "cad.layers.service",
        "cad.network.define",
        "cad.layers.time",
        "cad.scenarios.edit",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    // A map service layer's command removes a layer too (docs/adr/0208 §15), a network's its definition (0209 §11),
    // a layer's time its time and a scenario's apply its layers (0210 §11).
    for name in [
        "cad.layers.service",
        "cad.network.define",
        "cad.layers.time",
        "cad.scenarios.edit",
    ] {
        let tool = tools
            .iter()
            .find(|t| t["name"] == name)
            .expect("the command");
        assert_eq!(tool["annotations"]["destructiveHint"], true, "{name}");
    }
    for name in [
        "project.list",
        "project.open",
        "project.share",
        "project.create",
        "project.rename",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    let share = tools
        .iter()
        .find(|t| t["name"] == "project.share")
        .expect("the command");
    assert_eq!(share["inputSchema"]["required"][0], "tenant");
    assert_eq!(share["inputSchema"]["required"][1], "project");
    assert_eq!(share["annotations"]["openWorldHint"], true);
    let create = tools
        .iter()
        .find(|t| t["name"] == "project.create")
        .expect("the command");
    assert!(
        create["inputSchema"]["properties"].get("project").is_none(),
        "a new project has none yet"
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
fn a_layers_time_and_a_scenario_go_through_their_tools() {
    // The traces' scene (docs/adr/0210): parcels, buildings and breakdowns in
    // time, a road and a hidden scenario widening it.
    let scene = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/interaction/v1/temporal.kcad"
    );
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({ "path": scene }));
    let drawing = opened["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();

    let timed = call(
        &mut server,
        "cad.layers.time",
        json!({ "drawing": drawing, "layer": "yol", "time": { "start": "acilis_tarihi", "cumulative": true } }),
    );
    assert_eq!(timed["isError"], false, "{timed}");
    assert_eq!(timed["structuredContent"]["output"]["changed"], true);
    let layers = call(&mut server, "drawing.layers", json!({ "drawing": drawing }));
    let road = layers["structuredContent"]["layers"]
        .as_array()
        .and_then(|l| l.iter().find(|n| n["id"] == "yol"))
        .expect("the road")
        .clone();
    assert_eq!(
        road["time"],
        json!({ "start": "acilis_tarihi", "cumulative": true })
    );
    let group = call(
        &mut server,
        "cad.layers.time",
        json!({ "drawing": drawing, "layer": "alt-a", "time": { "start": "tarih" } }),
    );
    assert_eq!(group["isError"], true);
    assert_eq!(group["structuredContent"]["error"]["code"], "not_a_layer");

    let made = call(
        &mut server,
        "cad.scenarios.edit",
        json!({ "drawing": drawing, "operation": "create", "name": "Öneri B", "layers": ["parsel"] }),
    );
    assert_eq!(made["isError"], false, "{made}");
    let output = &made["structuredContent"]["output"];
    assert_eq!(output["objects"], 6);
    assert_eq!(output["layers"][0]["base"], "parsel");
    let scenario = output["scenario"].as_str().expect("its group").to_owned();
    let applied = call(
        &mut server,
        "cad.scenarios.edit",
        json!({ "drawing": drawing, "operation": "apply", "scenario": scenario }),
    );
    assert_eq!(applied["isError"], false, "{applied}");
    assert_eq!(applied["structuredContent"]["output"]["removed"], 6);
    let undone = call(&mut server, "drawing.undo", json!({ "drawing": drawing }));
    assert_eq!(undone["isError"], false, "{undone}");
    assert_eq!(undone["structuredContent"]["step"], "Senaryoyu uygula");
}

#[test]
fn a_layers_filter_goes_through_its_tool_and_leaves_the_objects() {
    // The traces' scene (docs/adr/0211): five parcels and two buildings with attributes.
    let scene = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/interaction/v1/feature-table.kcad"
    );
    let mut server = Server::new();
    let listed = ask(&mut server, 2, "tools/list", json!({}));
    let tool = listed["result"]["tools"]
        .as_array()
        .and_then(|t| t.iter().find(|t| t["name"] == "cad.layers.filter"))
        .expect("the command")
        .clone();
    // A filter is the layer's view: it takes nothing away.
    assert_eq!(tool["annotations"]["destructiveHint"], false);
    let opened = call(&mut server, "drawing.open", json!({ "path": scene }));
    let drawing = opened["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let filtered = call(
        &mut server,
        "cad.layers.filter",
        json!({ "drawing": drawing, "layer": "parsel", "filter": { "expression": "Ada = '101'" } }),
    );
    assert_eq!(filtered["isError"], false, "{filtered}");
    let output = &filtered["structuredContent"]["output"];
    assert_eq!(
        (output["passed"].clone(), output["total"].clone()),
        (json!(2), json!(5))
    );
    let layers = call(&mut server, "drawing.layers", json!({ "drawing": drawing }));
    let parcels = layers["structuredContent"]["layers"]
        .as_array()
        .and_then(|l| l.iter().find(|n| n["id"] == "kadastro"))
        .and_then(|g| g["children"].as_array())
        .and_then(|c| c.iter().find(|n| n["id"] == "parsel"))
        .expect("the parcels")
        .clone();
    assert_eq!(parcels["filter"], json!({ "expression": "Ada = '101'" }));
    // The commands and queries see every object.
    let page = call(
        &mut server,
        "drawing.entities",
        json!({ "drawing": drawing, "layer": "parsel" }),
    );
    assert_eq!(
        page["structuredContent"]["items"].as_array().map(Vec::len),
        Some(5)
    );
    let refused = call(
        &mut server,
        "cad.layers.filter",
        json!({ "drawing": drawing, "layer": "parsel", "filter": { "expression": "2 >= $sıra" } }),
    );
    assert_eq!(refused["isError"], true);
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_expression"
    );
    let undone = call(&mut server, "drawing.undo", json!({ "drawing": drawing }));
    assert_eq!(undone["structuredContent"]["step"], "Katman süzgeci");
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

#[test]
fn the_servers_tools_need_the_account_the_environment_names() {
    if std::env::var_os("KENTOS_URL").is_some() {
        return; // A developer's own server is named: this test is about none.
    }
    let mut server = Server::new();
    let listed = call(&mut server, "project.list", json!({}));
    assert_eq!(listed["isError"], true);
    assert_eq!(
        listed["structuredContent"]["error"]["code"],
        "not_configured"
    );
    let shared = call(
        &mut server,
        "project.share",
        json!({ "tenant": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "project": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                "userId": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "role": "viewer" }),
    );
    assert_eq!(
        shared["structuredContent"]["error"]["code"],
        "not_configured"
    );
}

/// A stand-in for the desktop's agents' link: a Unix socket answering on a
/// headless drawing as the desktop answers on its open one.
#[cfg(unix)]
fn stand_in_desktop(path: &std::path::Path) -> std::thread::JoinHandle<()> {
    use kentos_headless::{NewDrawing, Op, Session};
    use std::os::unix::net::UnixListener;
    let listener = UnixListener::bind(path).expect("the socket");
    std::thread::spawn(move || {
        let mut session = Session::new(&NewDrawing {
            name: "Masaüstü".into(),
            srid: 5256,
            plot_scale: 1000.0,
            workspace: kentos_contracts::Workspace::Cad,
            drawing_font: kentos_contracts::DrawingFont::Barlow,
            province: None,
            drawing_unit: None,
        })
        .expect("a drawing");
        let (stream, _) = listener.accept().expect("a connection");
        let mut writer = stream.try_clone().expect("its writer");
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { break };
            let request: Value = serde_json::from_str(&line).expect("JSON");
            let params = &request["params"];
            let result = match request["method"].as_str().unwrap_or_default() {
                "hello" => Ok(json!({ "name": "Masaüstü", "objects": session.document().len() })),
                "summary" => Ok(session.summary()),
                "layers" => Ok(serde_json::to_value(session.layers()).expect("layers")),
                "measure" => session
                    .measure(params["uid"].as_str().unwrap_or_default())
                    .map(|m| serde_json::to_value(m).expect("a measure")),
                "run" => session.run(
                    params["command"].as_str().unwrap_or_default(),
                    None,
                    Op::parse(params["op"].as_str().unwrap_or("execute")).expect("op"),
                    params["input"].clone(),
                ),
                other => panic!("{other}"),
            };
            let answer = match result {
                Ok(r) => json!({ "id": request["id"], "ok": true, "result": r }),
                Err(e) => {
                    json!({ "id": request["id"], "ok": false, "code": e.code, "message": e.message })
                }
            };
            writeln!(writer, "{answer}").expect("answered");
        }
    })
}

#[cfg(unix)]
#[test]
fn a_desktop_handle_sends_the_tools_to_the_drawing_on_the_screen() {
    let dir = std::env::temp_dir().join(format!("kentos-mcp-masaustu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("m.sock");
    let desktop = stand_in_desktop(&path);
    let mut server = Server::new();

    let attached = call(&mut server, "desktop.attach", json!({ "path": path }));
    assert_eq!(attached["isError"], false, "{attached}");
    assert_eq!(attached["structuredContent"]["desktop"]["name"], "Masaüstü");
    let handle = attached["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    assert!(handle.starts_with("desktop-"));

    let summary = call(&mut server, "drawing.summary", json!({ "drawing": handle }));
    let layer = summary["structuredContent"]["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let written = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": handle, "layerId": layer, "pts": square() }),
    );
    assert_eq!(
        written["structuredContent"]["status"], "completed",
        "{written}"
    );
    let uid = written["structuredContent"]["output"]["uid"]
        .as_str()
        .expect("its id")
        .to_owned();
    let measured = call(
        &mut server,
        "drawing.measure",
        json!({ "drawing": handle, "uid": uid }),
    );
    assert_eq!(measured["structuredContent"]["area"], 250.0);
    let refused = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": handle, "layerId": "yok", "pts": square() }),
    );
    assert_eq!(refused["isError"], true, "the desktop's command refused");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "layer_not_found"
    );

    let saved = call(&mut server, "drawing.save", json!({ "drawing": handle }));
    assert_eq!(
        saved["structuredContent"]["error"]["code"], "desktop_user",
        "the user saves"
    );
    let listed = call(&mut server, "drawing.list", json!({}));
    assert_eq!(listed["structuredContent"]["drawings"][0]["desktop"], true);
    let closed = call(&mut server, "drawing.close", json!({ "drawing": handle }));
    assert_eq!(closed["structuredContent"]["detached"], true);
    let gone = call(&mut server, "drawing.summary", json!({ "drawing": handle }));
    assert_eq!(
        gone["structuredContent"]["error"]["code"],
        "unknown_drawing"
    );
    desktop.join().expect("the stand-in ended");

    let none = call(
        &mut server,
        "desktop.attach",
        json!({ "path": dir.join("yok.sock") }),
    );
    assert_eq!(none["structuredContent"]["error"]["code"], "no_desktop");
    let _ = std::fs::remove_dir_all(dir);
}

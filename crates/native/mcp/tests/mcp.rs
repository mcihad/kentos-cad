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

/// A command the catalog says only reads (`effect: query`) is read-only to a client, a file command that writes a file
/// is not, and no command that changes the drawing is (docs/adr/0622 §8).
#[test]
fn the_read_only_hint_follows_the_catalogs_effect() {
    let mut server = Server::new();
    let listed = ask(&mut server, 1, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let catalog = serde_json::to_value(kentos_commands::catalog()).expect("the catalog");
    let mut reads = 0;
    for c in catalog["commands"].as_array().expect("commands") {
        let id = c["id"].as_str().expect("an id");
        let Some(tool) = tools.iter().find(|t| t["name"] == id) else {
            continue;
        };
        // A file command writes its file, but the print check; a query that writes a file of its own is destructive.
        let writes_file = (id.starts_with("file.") && id != "file.preflight.sheets")
            || tool["annotations"]["destructiveHint"] == true;
        let read_only = c["effect"] == "query" && !writes_file;
        assert_eq!(tool["annotations"]["readOnlyHint"], read_only, "{id}");
        if read_only {
            reads += 1;
            assert_eq!(tool["annotations"]["destructiveHint"], false, "{id}");
        }
    }
    // The audit's 41 commands without the hint are among them (cad.topology.check, cad.texts.check …).
    assert!(reads >= 50, "{reads}");
    for id in ["cad.topology.check", "cad.texts.check", "cad.sources.query", "cad.time.query"] {
        let tool = tools.iter().find(|t| t["name"] == id).expect("the command");
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{id}");
    }
    for id in ["file.export.dxf", "file.package", "cad.layers.edit", "cad.profiles.import", "cad.rasters.gpkgwrite"] {
        let tool = tools.iter().find(|t| t["name"] == id).expect("the command");
        assert_eq!(tool["annotations"]["readOnlyHint"], false, "{id}");
    }
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
        "cad.entities.offset",
        "cad.layers.service",
        "cad.network.define",
        "cad.linearref.define",
        "cad.addresses.define",
        "cad.layers.time",
        "cad.scenarios.edit",
        "cad.layers.labels",
        "cad.labels.pin",
        "cad.layers.renderer",
        "cad.layers.edit",
        "cad.layers.set",
        "cad.layers.states",
        "cad.styles.edit",
        "cad.settings.set",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    // A map service layer's command removes a layer too (docs/adr/0208 §15), a network's its definition (0209 §11),
    // a layer's time its time and a scenario's apply its layers (0210 §11), a labelling or a pin with null (0212 §5),
    // a renderer with null (0213 §5), address data's definition (0229 §14), Ötele's deleteSource its sources (0246).
    for name in [
        "cad.entities.offset",
        "cad.layers.service",
        "cad.network.define",
        "cad.linearref.define",
        "cad.addresses.define",
        "cad.layers.time",
        "cad.scenarios.edit",
        "cad.layers.labels",
        "cad.labels.pin",
        "cad.layers.renderer",
        "cad.layers.edit",
        "cad.layers.set",
        "cad.layers.states",
        "cad.styles.edit",
        "cad.settings.set",
        "cad.views.edit",
        "cad.forms.edit",
        "cad.forms.submit",
        "cad.forms.fill",
        "cad.relations.edit",
        "cad.rasters.gpkgread",
        "cad.rasters.gpkgwrite",
        "cad.profiles.import",
        "cad.profiles.migrate",
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
        "jobs.start",
        "jobs.status",
        "jobs.cancel",
        "jobs.result",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    // Asking for a job may write the project (a commit); reading one and taking its result do not.
    for (name, read_only) in [
        ("jobs.start", false),
        ("jobs.status", true),
        ("jobs.cancel", false),
        ("jobs.result", true),
    ] {
        let tool = tools.iter().find(|t| t["name"] == name).expect("the tool");
        assert_eq!(tool["annotations"]["readOnlyHint"], read_only, "{name}");
        assert_eq!(tool["annotations"]["openWorldHint"], true, "{name}");
        assert_eq!(tool["inputSchema"]["required"][0], "tenant");
        assert_eq!(tool["inputSchema"]["required"][1], "project");
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

/// A model sets a drawing up as the desktop would (docs/adr/0248): a layer
/// added, renamed and locked, a text style, the plot scale; a user's setting
/// is refused by the schema, not quietly dropped.
#[test]
fn layers_styles_and_settings_go_through_their_tools() {
    let mut server = Server::new();
    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Ofis", "srid": 5256 }),
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let added = call(
        &mut server,
        "cad.layers.edit",
        json!({ "drawing": drawing, "operation": "add", "name": "Duvar", "color": "#FF0000", "lineWeight": 0.5, "activate": true }),
    );
    assert_eq!(added["structuredContent"]["status"], "completed", "{added}");
    let layer = added["structuredContent"]["output"]["layer"]
        .as_str()
        .expect("its id")
        .to_owned();
    let renamed = call(
        &mut server,
        "cad.layers.set",
        json!({ "drawing": drawing, "operation": "rename", "layer": layer, "name": "Duvarlar" }),
    );
    assert_eq!(renamed["structuredContent"]["output"]["changed"], true);
    let locked = call(
        &mut server,
        "cad.layers.set",
        json!({ "drawing": drawing, "operation": "locked", "layer": layer, "value": true }),
    );
    assert_eq!(
        locked["structuredContent"]["output"]["active"],
        layer.as_str()
    );
    let tree = call(&mut server, "drawing.layers", json!({ "drawing": drawing }));
    let node = tree["structuredContent"]["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|n| n["id"] == layer.as_str())
        .expect("the layer");
    assert_eq!(node["name"], "Duvarlar");
    assert_eq!(node["locked"], true);
    assert_eq!(node["style"]["color"], "#FF0000");

    let style = call(
        &mut server,
        "cad.styles.edit",
        json!({ "drawing": drawing, "kind": "text", "operation": "create",
                "text": { "id": "", "name": "Oda adı", "font": "arimo", "height": 2.5 } }),
    );
    assert_eq!(style["structuredContent"]["status"], "completed", "{style}");
    let scaled = call(
        &mut server,
        "cad.settings.set",
        json!({ "drawing": drawing, "plotScale": 100 }),
    );
    assert_eq!(
        scaled["structuredContent"]["output"]["changed"],
        json!(["plotScale"])
    );
    let summary = call(
        &mut server,
        "drawing.summary",
        json!({ "drawing": drawing }),
    );
    assert_eq!(summary["structuredContent"]["settings"]["plotScale"], 100.0);
    assert_eq!(
        summary["structuredContent"]["settings"]["textStyles"][0]["name"],
        "Oda adı"
    );
    // A theme is the user's, not the drawing's: the schema has no such key.
    let refused = ask(
        &mut server,
        9,
        "tools/call",
        json!({ "name": "cad.settings.set", "arguments": { "drawing": drawing, "theme": "dark" } }),
    );
    let text = refused.to_string();
    assert!(text.contains("theme"), "{refused}");
    let summary = call(
        &mut server,
        "drawing.summary",
        json!({ "drawing": drawing }),
    );
    assert_eq!(summary["structuredContent"]["settings"]["plotScale"], 100.0);
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

/// Ötele by its meaning (docs/adr/0246): the line 2 of the traces' drawing
/// moved 0.2 m west is the copy the desktop's Ötele tool writes for the same
/// request (crates/native/interaction, tests/all/edits.rs) and Python's
/// (python/tests/test_offset.py): x = 486979.8 from y = 4420006 to 4420018.
#[test]
fn an_offset_by_its_meaning_gives_the_tools_copy() {
    let scene = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/interaction/v1/edits.kcad"
    );
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({ "path": scene }));
    let drawing = opened["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let page = call(
        &mut server,
        "drawing.entities",
        json!({ "drawing": drawing, "kinds": ["line"], "bbox": [486979, 4420005, 486981, 4420019] }),
    );
    let line = page["structuredContent"]["items"]
        .as_array()
        .and_then(|items| items.iter().find(|i| i["entity"]["id"] == 2))
        .and_then(|i| i["uid"].as_str())
        .expect("the line 2")
        .to_owned();
    let ask_offset = |op: &str| {
        json!({
            "drawing": drawing,
            "op": op,
            "uids": [line],
            "by": { "kind": "distance", "distance": 0.2, "side": { "x": 486977.0, "y": 4420010.0 } }
        })
    };
    let planned = call(&mut server, "cad.entities.offset", ask_offset("plan"));
    assert_eq!(planned["isError"], false, "{planned}");
    let copy = planned["structuredContent"]["output"]["entities"][0].clone();
    let written = call(&mut server, "cad.entities.offset", ask_offset("execute"));
    assert_eq!(written["isError"], false, "{written}");
    let output = &written["structuredContent"]["output"];
    assert_eq!(output["distances"], json!([0.2]));
    let made = output["created"][0].as_str().expect("a copy").to_owned();
    let page = call(
        &mut server,
        "drawing.entities",
        json!({ "drawing": drawing, "kinds": ["line"], "bbox": [486979.7, 4420005, 486979.9, 4420019] }),
    );
    let written_copy = page["structuredContent"]["items"]
        .as_array()
        .and_then(|items| items.iter().find(|i| i["uid"] == made.as_str()))
        .map(|i| i["entity"].clone())
        .expect("the copy");
    for (end, y) in [("a", 4420006.0), ("b", 4420018.0)] {
        assert_eq!(written_copy[end], json!({ "x": 486979.8, "y": y }), "{end}");
        assert_eq!(copy[end], written_copy[end], "the plan is what was written");
    }
    let refused = call(
        &mut server,
        "cad.entities.offset",
        json!({ "drawing": drawing, "uids": [line], "by": { "kind": "distance", "distance": 0.0, "side": { "x": 0.0, "y": 0.0 } } }),
    );
    assert_eq!(refused["isError"], true);
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_distance"
    );
    assert_eq!(refused["structuredContent"]["error"]["path"], "by.distance");
    let undone = call(&mut server, "drawing.undo", json!({ "drawing": drawing }));
    assert_eq!(undone["structuredContent"]["step"], "Ötele");
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
    let job = call(
        &mut server,
        "jobs.status",
        json!({ "tenant": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "project": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                "job": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f" }),
    );
    assert_eq!(job["structuredContent"]["error"]["code"], "not_configured");
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
            workspace: kentos_commands::Workspace::Cad,
            drawing_font: kentos_commands::DrawingFont::Barlow,
            province: None,
            drawing_unit: None,
            annotation: None,
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
                // The protocol's call (docs/adr/0253), as the desktop's agent link answers it.
                "call" => session.call(params.clone()),
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

/// The label engine's commands as tools (docs/adr/0212 §5): a layer's labelling written in rules, an
/// object's label pinned by hand, a pin against the rules refused with its reason.
#[test]
fn a_layer_is_labelled_by_rules_and_a_label_pinned() {
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", json!({ "srid": 5256 }));
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let layer = made["structuredContent"]["summary"]["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let written = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "layerId": layer, "pts": square() }),
    );
    let uid = written["structuredContent"]["output"]["uid"]
        .as_str()
        .expect("the object's id")
        .to_owned();
    let labelled = call(
        &mut server,
        "cad.layers.labels",
        json!({ "drawing": drawing, "layer": layer, "labels": { "mode": "rules", "classes": [
            { "name": "No", "style": { "placement": "center", "size": 10, "text": "$etiket" } }
        ] } }),
    );
    assert_eq!(labelled["isError"], false, "{labelled}");
    assert_eq!(labelled["structuredContent"]["output"]["changed"], true);
    let pinned = call(
        &mut server,
        "cad.labels.pin",
        json!({ "drawing": drawing, "pins": [{ "uid": uid, "class": "No", "pin": { "at": { "x": 3, "y": -2 }, "rotation": 15 } }] }),
    );
    assert_eq!(pinned["isError"], false, "{pinned}");
    assert_eq!(pinned["structuredContent"]["output"]["changed"], 1);
    let refused = call(
        &mut server,
        "cad.labels.pin",
        json!({ "drawing": drawing, "pins": [{ "uid": uid, "pin": { "rotation": 15 } }] }),
    );
    assert_eq!(refused["isError"], true);
    assert_eq!(refused["structuredContent"]["status"], "failed");
}

/// Katman stili's command as a tool (docs/adr/0213 §5): a layer's heat map written, then taken
/// away; a renderer the style core's rules refuse says why.
#[test]
fn a_layer_takes_a_heat_map_and_gives_it_back() {
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", json!({ "srid": 5256 }));
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let layer = made["structuredContent"]["summary"]["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let heat =
        json!({ "type": "heatmap", "radius": 20, "unit": "px", "ramp": ["#2B83BA00", "#D7191C"] });
    let written = call(
        &mut server,
        "cad.layers.renderer",
        json!({ "drawing": drawing, "layer": layer, "renderer": heat }),
    );
    assert_eq!(written["isError"], false, "{written}");
    assert_eq!(written["structuredContent"]["output"]["changed"], true);
    let again = call(
        &mut server,
        "cad.layers.renderer",
        json!({ "drawing": drawing, "layer": layer, "renderer": heat }),
    );
    assert_eq!(again["structuredContent"]["output"]["changed"], false);
    let taken = call(
        &mut server,
        "cad.layers.renderer",
        json!({ "drawing": drawing, "layer": layer, "renderer": null }),
    );
    assert_eq!(taken["structuredContent"]["output"]["changed"], true);
    let refused = call(
        &mut server,
        "cad.layers.renderer",
        json!({ "drawing": drawing, "layer": layer, "renderer": { "type": "heatmap", "radius": 600, "ramp": ["#000000", "#FFFFFF"] } }),
    );
    assert_eq!(refused["isError"], true);
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_renderer"
    );
    assert!(
        refused["structuredContent"]["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("“radius”")),
        "{refused}"
    );
}

/// A CAD project in millimetres (docs/adr/0245): a 4000 × 3000 mm room is
/// written in metres, measures 12 m², and the project says mm is its unit.
#[test]
fn a_millimetre_cad_project_takes_metres_and_says_its_unit() {
    let mut server = Server::new();
    let listed = ask(&mut server, 1, "tools/list", json!({}));
    let new = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|t| t["name"] == "drawing.new")
        .expect("drawing.new")
        .clone();
    assert_eq!(new["inputSchema"]["required"], json!([]));
    assert_eq!(
        new["inputSchema"]["properties"]["drawingUnit"]["enum"],
        json!(["mm", "cm", "m", "in", "ft"])
    );

    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Oda", "workspace": "cad", "drawingUnit": "mm" }),
    );
    assert_eq!(made["isError"], false, "{made}");
    let summary = &made["structuredContent"]["summary"];
    assert_eq!(summary["settings"]["srid"], 0);
    assert_eq!(summary["settings"]["workspace"], "cad");
    assert_eq!(summary["settings"]["drawingUnit"], "mm");
    assert_eq!(summary["settings"]["plotScale"], 1.0);
    assert_eq!(
        summary["units"],
        json!({ "length": "m", "area": "m2", "drawing": "mm" })
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let layer = summary["activeLayer"].as_str().expect("a layer").to_owned();
    let room = json!([{"x": 0.0, "y": 0.0}, {"x": 4.0, "y": 0.0}, {"x": 4.0, "y": 3.0}, {"x": 0.0, "y": 3.0}]);
    let written = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "layerId": layer, "pts": room }),
    );
    let uid = written["structuredContent"]["output"]["uid"]
        .as_str()
        .expect("its id")
        .to_owned();
    let measured = call(
        &mut server,
        "drawing.measure",
        json!({ "drawing": drawing, "uid": uid }),
    );
    assert_eq!(measured["structuredContent"]["area"], 12.0);
    assert_eq!(measured["structuredContent"]["length"], 14.0);
    assert_eq!(
        measured["structuredContent"]["units"],
        json!({ "length": "m", "area": "m2", "drawing": "mm" })
    );

    // What the wizard cannot make is refused in its words.
    let refused = call(
        &mut server,
        "drawing.new",
        json!({ "srid": 5256, "drawingUnit": "mm" }),
    );
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_input"
    );
    let unnamed = call(
        &mut server,
        "drawing.new",
        json!({ "srid": 5256, "name": "" }),
    );
    assert_eq!(
        unnamed["structuredContent"]["error"]["message"],
        "Proje adı boş olamaz."
    );
    let unknown = call(&mut server, "drawing.new", json!({ "srid": 4 }));
    assert_eq!(
        unknown["structuredContent"]["error"]["code"],
        "unknown_system"
    );
}

#[test]
fn external_cad_opens_through_mcp_without_creating_a_project_first() {
    let mut server = Server::new();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/dxf-units/mm.dxf");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let content = &opened["structuredContent"];
    assert_eq!(content["summary"]["settings"]["workspace"], "cad");
    assert_eq!(content["summary"]["settings"]["drawingUnit"], "mm");
    assert!(content["summary"]["openedCad"]["report"].is_object());
    // The wizard's first work, a building in metres at 1:100 (docs/adr/0626 §1).
    let made = call(&mut server, "drawing.new", json!({"workspace":"cad"}));
    assert_eq!(made["structuredContent"]["summary"]["units"]["drawing"], "m");
    assert_eq!(made["structuredContent"]["summary"]["settings"]["plotScale"], 100.0);
}

/// The protocol through MCP (docs/adr/0253): a command's summary and its
/// stamps, a request sent again, what a call expects, a job, the journal.
#[test]
fn a_model_writes_by_the_protocol_with_requests_stamps_jobs_and_a_journal() {
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", json!({ "srid": 5256 }));
    let handle = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let layer = made["structuredContent"]["summary"]["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let args =
        json!({ "drawing": handle, "layerId": layer, "pts": square(), "requestId": "model:1" });
    let first = call(&mut server, "cad.polygon.create", args.clone());
    assert_eq!(first["isError"], false, "{first}");
    let answer = &first["structuredContent"];
    let uid = answer["output"]["uid"].as_str().expect("a uid").to_owned();
    assert_eq!(answer["summary"]["added"], json!([uid.clone()]));
    let stamp = answer["summary"]["stamps"]["objects"][&uid]
        .as_str()
        .expect("a stamp")
        .to_owned();
    // Sent again: the same answer, written once.
    let again = call(&mut server, "cad.polygon.create", args);
    assert_eq!(again["structuredContent"]["replayed"], true);
    let summary = call(&mut server, "drawing.summary", json!({ "drawing": handle }));
    assert_eq!(summary["structuredContent"]["objects"], 1);
    // A stale stamp writes nothing and names its object.
    let moved = call(
        &mut server,
        "cad.entities.transform",
        json!({ "drawing": handle, "uids": [uid], "transform": { "kind": "move", "dx": 1.0, "dy": 0.0 },
                "expected": { "objects": { uid.clone(): stamp.clone() } } }),
    );
    assert_eq!(moved["isError"], false, "{moved}");
    let stale = call(
        &mut server,
        "cad.entities.delete",
        json!({ "drawing": handle, "uids": [uid], "expected": { "objects": { uid.clone(): stamp } } }),
    );
    assert_eq!(stale["isError"], true);
    assert_eq!(stale["structuredContent"]["error"]["code"], "stale_state");
    assert_eq!(
        stale["structuredContent"]["error"]["uids"],
        json!([uid.clone()])
    );
    assert_eq!(stale["structuredContent"]["error"]["retryable"], true);
    // A job.
    let started = call(
        &mut server,
        "drawing.job.start",
        json!({ "drawing": handle, "command": "cad.entities.transform",
                "input": { "uids": [uid], "transform": { "kind": "move", "dx": 1.0, "dy": 0.0 } } }),
    );
    let id = started["structuredContent"]["id"]
        .as_str()
        .expect("a job")
        .to_owned();
    let done = call(
        &mut server,
        "drawing.job.wait",
        json!({ "drawing": handle, "id": id }),
    );
    assert_eq!(done["structuredContent"]["phase"], "done", "{done}");
    let late = call(
        &mut server,
        "drawing.job.cancel",
        json!({ "drawing": handle, "id": id }),
    );
    assert_eq!(
        late["structuredContent"]["error"]["code"],
        "already_committed"
    );
    // The journal, replayed on a new drawing.
    let journal = call(&mut server, "drawing.journal", json!({ "drawing": handle }));
    let calls = journal["structuredContent"]["calls"]
        .as_array()
        .expect("rows")
        .len();
    assert!(calls >= 3, "{journal}");
    let other = call(&mut server, "drawing.new", json!({ "srid": 5256 }));
    let other = other["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let report = call(
        &mut server,
        "drawing.replay",
        json!({ "drawing": other, "journal": journal["structuredContent"] }),
    );
    assert_eq!(report["structuredContent"]["failed"], 0, "{report}");
    assert!(
        report["structuredContent"]["same"]
            .as_u64()
            .is_some_and(|n| n >= 3)
    );
}

/// docs/adr/0256: an agent changes a drawing's unit keeping the numbers,
/// measures many objects at once with the basis said once, reads the
/// conventions, and finds every number's unit in the tools' schemas.
#[test]
fn an_agent_changes_the_unit_and_measures_in_a_batch() {
    let mut server = Server::new();
    let listed = ask(&mut server, 1, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let change = tools
        .iter()
        .find(|t| t["name"] == "cad.units.change")
        .expect("cad.units.change");
    assert_eq!(change["annotations"]["destructiveHint"], true);
    let line = tools
        .iter()
        .find(|t| t["name"] == "cad.arc.create")
        .expect("cad.arc.create");
    assert_eq!(line["inputSchema"]["properties"]["a0"]["x-unit"], "rad");
    assert_eq!(line["inputSchema"]["properties"]["r"]["x-unit"], "length");

    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Oda", "workspace": "cad", "drawingUnit": "m" }),
    );
    let summary = &made["structuredContent"]["summary"];
    assert_eq!(
        summary["conventions"]["labels"],
        json!({ "x": "X", "y": "Y" })
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let layer = summary["activeLayer"].as_str().expect("a layer").to_owned();
    // A millimetre drawing taken for metres: 4000 × 3000 “m”.
    let room = json!([{"x": 0.0, "y": 0.0}, {"x": 4000.0, "y": 0.0}, {"x": 4000.0, "y": 3000.0}, {"x": 0.0, "y": 3000.0}]);
    let written = call(
        &mut server,
        "cad.polygon.create",
        json!({ "drawing": drawing, "layerId": layer, "pts": room }),
    );
    let uid = written["structuredContent"]["output"]["uid"]
        .as_str()
        .expect("its id")
        .to_owned();
    let changed = call(
        &mut server,
        "cad.units.change",
        json!({ "drawing": drawing, "from": "m", "to": "mm", "geometry": "scale" }),
    );
    assert_eq!(changed["isError"], false, "{changed}");
    assert_eq!(changed["structuredContent"]["output"]["factor"], 0.001);
    let many = call(
        &mut server,
        "drawing.measure",
        json!({ "drawing": drawing, "uids": [uid] }),
    );
    let m = &many["structuredContent"];
    assert_eq!(m["totals"]["area"], 12.0, "{m}");
    assert_eq!(m["units"]["drawing"], "mm");
    assert_eq!(m["basis"]["method"], "plane");
    let no_system = call(
        &mut server,
        "drawing.measure",
        json!({ "drawing": drawing, "uid": uid, "method": "ellipsoid" }),
    );
    assert_eq!(no_system["isError"], true);
    assert_eq!(
        no_system["structuredContent"]["error"]["code"],
        "method_unavailable"
    );
}

/// An agent takes a DXF in (docs/adr/0257 §3): the tool is the catalog's, not
/// destructive (it only adds); the plan says the unit the file declares, the
/// write takes the millimetre room in as metres; a file declaring none is
/// read in the project's unit and says so.
#[test]
fn an_agent_imports_a_dxf_in_its_unit() {
    let mut server = Server::new();
    let listed = ask(&mut server, 1, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let import = tools
        .iter()
        .find(|t| t["name"] == "file.import.dxf")
        .expect("file.import.dxf");
    assert_eq!(import["annotations"]["destructiveHint"], false);
    assert!(
        import["inputSchema"]["properties"]
            .get("fileUnit")
            .is_some(),
        "{import}"
    );
    let files = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1/dxf-units");
    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Oda", "workspace": "cad", "drawingUnit": "m" }),
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let path = files.join("mm.dxf").display().to_string();
    let planned = call(
        &mut server,
        "file.import.dxf",
        json!({ "drawing": drawing, "op": "plan", "path": path }),
    );
    assert_eq!(planned["isError"], false, "{planned}");
    let unit = &planned["structuredContent"]["output"]["unit"];
    assert_eq!(
        (&unit["declared"], &unit["source"], &unit["factor"]),
        (&json!(4), &json!("declared"), &json!(0.001))
    );
    let done = call(
        &mut server,
        "file.import.dxf",
        json!({ "drawing": drawing, "path": path }),
    );
    assert_eq!(done["isError"], false, "{done}");
    assert_eq!(done["structuredContent"]["output"]["objects"], 5);
    let unitless = call(
        &mut server,
        "file.import.dxf",
        json!({ "drawing": drawing, "op": "plan", "path": files.join("unitless.dxf").display().to_string() }),
    );
    let warnings = unitless["structuredContent"]["warnings"]
        .as_array()
        .expect("warnings");
    assert_eq!(warnings[0]["code"], "unit_assumed", "{unitless}");
}

#[test]
fn an_agent_imports_a_dwg_through_the_pinned_codec() {
    let mut server = Server::new();
    let listed = ask(&mut server, 1, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let import = tools
        .iter()
        .find(|t| t["name"] == "file.import.dwg")
        .expect("file.import.dwg");
    assert_eq!(import["annotations"]["destructiveHint"], false);
    assert!(
        import["inputSchema"]["properties"]
            .get("fileUnit")
            .is_some(),
        "{import}"
    );
    let files = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1/dwg");
    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Oda", "workspace": "cad", "drawingUnit": "m" }),
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let path = files.join("mm.dwg").display().to_string();
    let planned = call(
        &mut server,
        "file.import.dwg",
        json!({ "drawing": drawing, "op": "plan", "path": path }),
    );
    assert_eq!(planned["isError"], false, "{planned}");
    let unit = &planned["structuredContent"]["output"]["unit"];
    assert_eq!(
        (&unit["declared"], &unit["source"], &unit["factor"]),
        (&json!(4), &json!("declared"), &json!(0.001))
    );
    let done = call(
        &mut server,
        "file.import.dwg",
        json!({ "drawing": drawing, "path": path }),
    );
    assert_eq!(done["isError"], false, "{done}");
    assert_eq!(done["structuredContent"]["output"]["objects"], 5);
}

#[test]
fn an_agent_keeps_and_renames_a_named_view() {
    // docs/adr/0285 §1: the project's named views through the catalog's command.
    let mut server = Server::new();
    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Ada", "workspace": "cad", "drawingUnit": "m" }),
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let saved = call(
        &mut server,
        "cad.views.edit",
        json!({ "drawing": drawing, "edit": { "kind": "save", "name": "Ada 112", "center": { "x": 120.0, "y": 80.0 }, "width": 60.0, "height": 40.0, "twist": 7.0 } }),
    );
    assert_eq!(saved["isError"], false, "{saved}");
    let view = &saved["structuredContent"]["output"]["view"];
    assert_eq!(view["name"], "Ada 112");
    assert_eq!(view["twist"], 7.0 - 2.0 * std::f64::consts::PI);
    let renamed = call(
        &mut server,
        "cad.views.edit",
        json!({ "drawing": drawing, "edit": { "kind": "rename", "name": "Ada 112", "to": "Kuzey" } }),
    );
    assert_eq!(
        renamed["structuredContent"]["output"]["views"][0]["name"],
        "Kuzey"
    );
    let refused = call(
        &mut server,
        "cad.views.edit",
        json!({ "drawing": drawing, "edit": { "kind": "delete", "name": "Yok" } }),
    );
    assert_eq!(refused["isError"], true, "{refused}");
}

#[test]
fn an_agent_sets_a_ucs_and_turns_its_points_explicitly() {
    let mut server = Server::new();
    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Bina", "workspace": "cad", "drawingUnit": "m" }),
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    assert!(made["structuredContent"]["summary"]["conventions"]["ucs"].is_null());
    // The building's wall from (100, 200) going north: the UCS's x axis.
    let set = call(
        &mut server,
        "cad.ucs.set",
        json!({ "drawing": drawing, "to": { "kind": "twoPoints", "origin": { "x": 100.0, "y": 200.0 }, "xPoint": { "x": 100.0, "y": 210.0 } } }),
    );
    assert_eq!(set["isError"], false, "{set}");
    let ucs = &set["structuredContent"]["output"]["ucs"];
    assert_eq!(ucs["angle"], std::f64::consts::FRAC_PI_2);
    let saved = call(
        &mut server,
        "cad.ucs.set",
        json!({ "drawing": drawing, "to": { "kind": "save", "name": "Duvar" } }),
    );
    assert_eq!(
        saved["structuredContent"]["output"]["named"],
        json!(["Duvar"])
    );
    let summary = call(
        &mut server,
        "drawing.summary",
        json!({ "drawing": drawing }),
    );
    assert_eq!(
        summary["structuredContent"]["conventions"]["ucs"]["name"],
        "Duvar"
    );
    // 10 along the UCS's x is 10 north in the world; and back.
    let world = call(
        &mut server,
        "drawing.toWorld",
        json!({ "drawing": drawing, "points": [{ "x": 10.0, "y": 0.0 }] }),
    );
    let p = &world["structuredContent"]["points"][0];
    assert!((p["x"].as_f64().unwrap() - 100.0).abs() < 1e-9, "{p}");
    assert!((p["y"].as_f64().unwrap() - 210.0).abs() < 1e-9, "{p}");
    let back = call(
        &mut server,
        "drawing.toUcs",
        json!({ "drawing": drawing, "points": [{ "x": 100.0, "y": 210.0 }], "name": "Duvar" }),
    );
    let q = &back["structuredContent"]["points"][0];
    assert!((q["x"].as_f64().unwrap() - 10.0).abs() < 1e-9, "{q}");
    assert!(q["y"].as_f64().unwrap().abs() < 1e-9, "{q}");
    // The wire is the world: a line given in world points is where they say.
    let layer = made["structuredContent"]["summary"]["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let line = call(
        &mut server,
        "cad.line.create",
        json!({ "drawing": drawing, "layerId": layer, "a": { "x": 100.0, "y": 200.0 }, "b": { "x": 100.0, "y": 210.0 } }),
    );
    assert_eq!(line["isError"], false, "{line}");
}

#[test]
fn an_agent_makes_publishes_and_uses_a_form() {
    // docs/adr/0401 §9: a layer's form through the catalog's commands, as the desktop's window and Python.
    let mut server = Server::new();
    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Mahalle", "workspace": "gis", "srid": 5256 }),
    );
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let added = call(
        &mut server,
        "cad.layers.edit",
        json!({ "drawing": drawing, "operation": "add", "name": "Bina", "activate": true }),
    );
    assert_eq!(added["isError"], false, "{added}");
    let layer = added["structuredContent"]["output"]["layer"]
        .as_str()
        .expect("the new layer")
        .to_owned();
    let fields = call(
        &mut server,
        "cad.layers.edit",
        json!({ "drawing": drawing, "operation": "fields", "layer": layer, "fields": [
            { "name": "Durum", "kind": "text", "required": true },
            { "name": "Kat", "kind": "integer", "min": "1" }
        ] }),
    );
    assert_eq!(fields["isError"], false, "{fields}");
    let form = call(
        &mut server,
        "cad.forms.edit",
        json!({ "drawing": drawing, "edit": { "kind": "create", "name": "Bina denetimi", "layer": layer } }),
    );
    assert_eq!(form["isError"], false, "{form}");
    assert_eq!(
        form["structuredContent"]["output"]["form"]["id"],
        "bina-denetimi"
    );
    let published = call(
        &mut server,
        "cad.forms.edit",
        json!({ "drawing": drawing, "edit": { "kind": "publish", "id": "bina-denetimi" } }),
    );
    assert_eq!(
        published["structuredContent"]["output"]["forms"][0]["version"],
        1
    );
    let asked = call(
        &mut server,
        "cad.forms.validate",
        json!({ "drawing": drawing, "form": "bina-denetimi" }),
    );
    assert_eq!(
        asked["structuredContent"]["output"]["valid"], true,
        "{asked}"
    );
    let refused = call(
        &mut server,
        "cad.forms.submit",
        json!({ "drawing": drawing, "form": "yok", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6dffff" }),
    );
    assert_eq!(refused["isError"], true, "{refused}");
}

/// STAC (docs/adr/0528): an agent prepares an item's assets as rasters by address, in one step, and gets the record;
/// the search is a read-only tool whose bad request is a refusal it can act on, before any network.
#[test]
fn an_agent_prepares_a_stac_item_and_a_bad_search_is_a_refusal() {
    let mut server = Server::new();
    let tools = ask(&mut server, 2, "tools/list", json!({}));
    let search = tools["result"]["tools"]
        .as_array()
        .and_then(|t| t.iter().find(|t| t["name"] == "cad.stac.search"))
        .expect("the search tool");
    assert_eq!(search["annotations"]["readOnlyHint"], true);
    let made = call(&mut server, "drawing.new", json!({ "srid": 32637 }));
    let drawing = made["structuredContent"]["drawing"]
        .as_str()
        .expect("a handle")
        .to_owned();
    let refused = call(
        &mut server,
        "cad.stac.search",
        json!({ "drawing": drawing, "endpoint": "ftp://x.example/v1" }),
    );
    assert_eq!(refused["isError"], true);
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_stac_endpoint"
    );
    let asset = |key: &str, common: &str| {
        json!({
            "key": key, "href": format!("https://h.example/{key}.tif"), "roles": ["data", "reflectance"],
            "bands": [{ "commonName": common }],
            "rasterBands": [{ "dataType": "uint16", "nodata": 0, "scale": 0.0001, "offset": -0.1 }],
            "epsg": 32637, "shape": [100, 200], "transform": [10, 0, 300000, 0, -10, 4400040]
        })
    };
    let item = json!({
        "catalog": "https://earth-search.aws.element84.com/v1", "collection": "sentinel-2-l2a", "id": "S2A_TEST",
        "datetime": "2024-07-13T08:29:15Z", "cloudCover": 3.1,
        "assets": [asset("nir", "nir"), asset("red", "red")]
    });
    let planned = call(
        &mut server,
        "cad.stac.prepare",
        json!({ "drawing": drawing, "op": "plan", "item": item, "assets": ["red", "nir"] }),
    );
    assert_eq!(planned["isError"], false, "{planned}");
    let written = call(
        &mut server,
        "cad.stac.prepare",
        json!({ "drawing": drawing, "op": "execute", "item": item, "assets": ["red", "nir"] }),
    );
    assert_eq!(written["isError"], false, "{written}");
    let record = &written["structuredContent"]["output"]["preparation"];
    assert_eq!(record["format"], "kentos.stac-preparation");
    assert_eq!(record["bands"][0]["asset"], "red");
    assert_eq!(record["bands"][0]["scale"], 0.0001);
    assert_eq!(
        record["reflectance"],
        json!({ "scale": 0.0001, "offset": -0.1 })
    );
    let page = call(
        &mut server,
        "drawing.entities",
        json!({ "drawing": drawing, "kinds": ["raster"] }),
    );
    assert_eq!(
        page["structuredContent"]["items"].as_array().map(Vec::len),
        Some(2),
        "{page}"
    );
}

#[test]
fn an_agent_must_explicitly_accept_a_reported_polyline_loss() {
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", json!({"name":"Bütünlük", "workspace":"cad", "drawingUnit":"m"}));
    let drawing = made["structuredContent"]["drawing"].as_str().unwrap().to_owned();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/cad-io/ocs-0-width.dxf");
    let refused = call(&mut server,"file.import.dxf",json!({"drawing":drawing,"path":source,"op":"plan"}));
    assert_eq!(refused["isError"],true,"{refused}");
    assert_eq!(refused["structuredContent"]["error"]["code"],"polyline_geometry_loss","{refused}");
    let accepted = call(&mut server,"file.import.dxf",json!({"drawing":drawing,"path":source,"op":"plan","allowPolylineLoss":true}));
    assert_eq!(accepted["isError"],false,"{accepted}");
    assert!(accepted["structuredContent"]["warnings"].as_array().unwrap().iter().any(|w| w["code"] == "import_losses"));
}

#[test]
fn spatial_sources_keep_xyz_through_the_agent_import_and_history_tools() {
    fn subset(actual: &Value, expected: &Value) {
        match expected {
            Value::Object(fields) => for (key,value) in fields { subset(&actual[key],value); },
            Value::Array(values) => {
                assert_eq!(actual.as_array().map(Vec::len),Some(values.len()),"{actual}");
                for (a,b) in actual.as_array().unwrap().iter().zip(values) { subset(a,b); }
            },
            Value::Number(value) => {
                let expected = value.as_f64().unwrap();
                let actual = actual.as_f64().expect("finite XYZ");
                assert!(actual.is_finite() && (actual-expected).abs() <= expected.abs()*32.*f64::EPSILON+1e-12,
                    "{actual} != {expected}");
            },
            _ => assert_eq!(actual,expected),
        }
    }
    let data: Value = serde_json::from_str(include_str!("../../../../fixtures/formats/v1/cad-spatial/expected.json")).unwrap();
    let mut server = Server::new();
    for case in data["cases"].as_array().unwrap() {
        let made = call(&mut server,"drawing.new",json!({"workspace":"cad","drawingUnit":"mm"}));
        let drawing = made["structuredContent"]["drawing"].clone();
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(case["source"].as_str().unwrap());
        let imported = call(&mut server,"file.import.dxf",json!({"drawing":drawing,"path":source,"explodeBlocks":true}));
        assert_eq!(imported["isError"],false,"{}: {imported}",case["name"]);
        for phase in ["import","redo"] {
            let page = call(&mut server,"drawing.entities",json!({"drawing":drawing}));
            let actual = page["structuredContent"]["items"].as_array().unwrap().iter()
                .map(|r| r["entity"].clone()).collect::<Vec<_>>();
            subset(&json!(actual),&case["expected"]);
            if phase == "import" {
                let undone = call(&mut server,"drawing.undo",json!({"drawing":drawing}));
                assert_eq!(undone["isError"],false,"{undone}");
                let empty = call(&mut server,"drawing.entities",json!({"drawing":drawing}));
                assert_eq!(empty["structuredContent"]["items"],json!([]));
                let redone = call(&mut server,"drawing.redo",json!({"drawing":drawing}));
                assert_eq!(redone["isError"],false,"{redone}");
            }
        }
        let closed = call(&mut server,"drawing.close",json!({"drawing":drawing,"discard":true}));
        assert_eq!(closed["isError"],false,"{closed}");
    }
}

#[test]
fn new_cad_defaults_follow_the_wizard_for_every_unit() {
    let mut server = Server::new();
    // A local building in metres is drawn at 1:100 (docs/adr/0626 §1).
    for (unit, scale) in [("m", 100.0), ("cm", 50.0), ("mm", 1.0), ("in", 1.0), ("ft", 1.0)] {
        let made = call(&mut server, "drawing.new", json!({"workspace":"cad","drawingUnit":unit}));
        assert_eq!(made["isError"], false, "{made}");
        assert_eq!(made["structuredContent"]["summary"]["settings"]["plotScale"], scale, "{unit}");
    }
    let listed = ask(&mut server, 1, "tools/list", json!({}));
    let new = listed["result"]["tools"].as_array().expect("tools").iter().find(|t| t["name"] == "drawing.new").expect("new");
    assert_eq!(new["inputSchema"]["properties"]["drawingUnit"]["enum"], json!(["mm","cm","m","in","ft"]));
}

#[test]
fn selection_and_donut_use_the_same_native_commands_as_python() {
    let mut server = Server::new();
    let made = call(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = made["structuredContent"]["drawing"].clone();
    let summary = made["structuredContent"]["summary"].clone();
    let layer = summary["activeLayer"].clone();
    let locked = call(&mut server, "cad.settings.set", json!({"drawing":drawing,"projectType":"gis","plotScale":500}));
    assert_eq!(locked["structuredContent"]["error"]["code"], "project_type_locked", "{locked}");
    let after = call(&mut server, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(after["structuredContent"]["settings"], summary["settings"], "whole input refused");
    let ring = call(&mut server, "cad.donut.create", json!({"drawing":drawing,"layerId":layer,
        "center":{"x":0.0,"y":0.0},"inner":2.0,"outer":6.0}));
    assert_eq!(ring["structuredContent"]["status"], "completed", "{ring}");
    let uid = ring["structuredContent"]["output"]["created"][0].clone();
    let selected = call(&mut server, "drawing.select", json!({"drawing":drawing,"uids":[uid,uid]}));
    assert_eq!(selected["structuredContent"]["selected"], 1, "{selected}");
    let selection = call(&mut server, "drawing.selection", json!({"drawing":drawing}));
    assert_eq!(selection["structuredContent"]["uids"], json!([uid]));
    for bad in [json!(null), json!("not-a-list"), json!([uid,17]), json!([uid,"missing"])] {
        let failed = call(&mut server, "drawing.select", json!({"drawing":drawing,"uids":bad}));
        assert_eq!(failed["isError"], true, "{failed}");
        let selection = call(&mut server, "drawing.selection", json!({"drawing":drawing}));
        assert_eq!(selection["structuredContent"]["uids"], json!([uid]));
    }
    let after = call(&mut server, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(after["structuredContent"]["revision"], ring["structuredContent"]["output"]["revision"]);
    let _ = call(&mut server, "cad.entities.delete", json!({"drawing":drawing,"uids":[uid]}));
    let selection = call(&mut server, "drawing.selection", json!({"drawing":drawing}));
    assert_eq!(selection["structuredContent"]["uids"], json!([]));
}

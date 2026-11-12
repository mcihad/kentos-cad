//! GNSS baseline sessions, new points' velocities and the kept place limits through a real AI host (plan/map-cad.md
//! MAP-03, docs/adr/4920): the shared cases' drawings of `map.survey.network` opened as .kcad files over MCP. With
//! `correlations: session` both sessions go in and the answer says so; without, they are said to be left out; a session
//! missing a block is refused; the new marks' velocities come from the campaign's grid with its name; the place limits
//! written by `map.survey.edit` stay in the project and judge the known points' places.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"gnss-sessions-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// A shared case's drawing opened over MCP: its handle.
fn open(server: &mut Server, file: &str, starts: &str) -> Value {
    let cases: Value = serde_json::from_str(file).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(starts)))
        .cloned()
        .expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-gnss-sessions-{}-{}.kcad", std::process::id(), starts.len()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(&path);
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn sessions_velocities_and_kept_limits_over_mcp() {
    let network = include_str!("../../../../fixtures/commands/v1/map.survey.network.json");
    // The campaign's velocity grid in this device's library, where every host reads it (KENTOS_GRID_DIR); this
    // binary's one test.
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let library = std::env::temp_dir().join(format!("kentos-gnss-sessions-grids-{}", std::process::id()));
    std::fs::create_dir_all(&library).unwrap();
    let grid = std::fs::read(root.join("fixtures/geoid/v1/hiz.tif")).unwrap();
    std::fs::write(library.join(format!("{}.tif", kentos_commands::encoding::sha256_hex(&grid))), &grid).unwrap();
    unsafe { std::env::set_var("KENTOS_GRID_DIR", &library) };
    let mut server = Server::new();
    let drawing = open(&mut server, network, "İki baz oturumu");
    let r = call(&mut server, "map.survey.network", json!({"drawing": drawing, "correlations": "session"}));
    assert_eq!(r["isError"], false, "{r}");
    let used = &r["structuredContent"]["output"]["correlations"];
    assert_eq!(used["used"], "session", "{used}");
    assert_eq!(used["sessions"], json!(["bo-1", "bo-2"]), "{used}");
    let r = call(&mut server, "map.survey.network", json!({"drawing": drawing}));
    assert_eq!(r["structuredContent"]["output"]["correlations"]["unused"], json!(["bo-1", "bo-2"]), "{r}");
    assert!(r.to_string().contains("session_correlations_unused"), "{r}");
    // The limits kept with their source; the known points' places judged by them.
    let r = call(
        &mut server,
        "map.survey.edit",
        json!({"drawing": drawing, "operation": {"op": "tolerance", "horizontal": 0.005, "source": "İdarenin şartnamesi"}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let r = call(&mut server, "map.survey.query", json!({"drawing": drawing, "places": true}));
    assert_eq!(r["structuredContent"]["output"]["tolerance"]["source"], "İdarenin şartnamesi", "{r}");

    let missing = open(&mut server, network, "Oturumun bir bloğu eksik");
    let r = call(&mut server, "map.survey.network", json!({"drawing": missing, "correlations": "session"}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("session_blocks_missing"), "{r}");

    let moving = open(&mut server, network, "Yeni noktaların hızı");
    let r = call(&mut server, "map.survey.network", json!({"drawing": moving, "velocities": true}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["velocityGrid"]["name"], "Hız alanı (sentetik)", "{out}");
    let n1 = out["stations"].as_array().unwrap().iter().find(|s| s["mark"] == "N1").unwrap();
    let east = n1["velocity"][0].as_f64().unwrap();
    assert!((-0.03..-0.02).contains(&east), "N1 moves about 2.4 cm a year west: {n1}");
}

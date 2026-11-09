//! The crossing pipes' outer walls in a pipe run profile through a real AI host (plan/map-cad.md MAP-16, docs/adr/4360):
//! the walls case's drawing opened as a .kcad file over MCP. `cad.pipes.profile` with the water network's wall asked
//! answers the case's crossings (each wall as read) and draws as many objects (the outer walls and their filled rings
//! with them); asked without walls in the drawing, no wall is answered.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"pipe-profile-walls-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn run_case(name: &str) {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.profile.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-pipe-profile-walls-{}-{}.kcad", std::process::id(), name.len()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "cad.pipes.profile", args);
    assert_eq!(r["isError"], false, "{name}: {r}");
    let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
    let walls = |v: &Value| v["crossings"].as_array().unwrap().iter().map(|c| c.get("wall").cloned()).collect::<Vec<_>>();
    assert_eq!(walls(got), walls(want), "{name}");
    assert_eq!(got["uids"].as_array().map(Vec::len), want["uids"].as_array().map(Vec::len), "{name}: as many objects");
    assert_eq!((&got["datum"], &got["top"]), (&want["datum"], &want["top"]), "{name}");
    let _ = std::fs::remove_file(path);
}

#[test]
fn the_crossing_pipes_outer_walls_over_mcp_are_the_cases() {
    run_case("Kesişen borunun dış duvarı");
    run_case("Et sorulmuş, borularda yok");
}

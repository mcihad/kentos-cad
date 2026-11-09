//! Writing only what changed through a real AI host (plan/map-cad.md MAP-14, docs/adr/4320): the regional cases' Harita
//! project opened as a .kcad file over MCP. Its ground's contour lines drawn and a hill's point raised, as the first
//! case; `road.project.rebuild` with `regional` answers the case's row (removed, kept, region); the lines that stayed
//! keep their ids; saved and opened again, the drawing is up to date and drawn again by name writes nothing.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"regional-rebuild-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn only_what_changed_is_written_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.rebuild.regional.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let uids = cases["setup"]["uids"].as_array().unwrap().clone();
    let path = std::env::temp_dir().join(format!("kentos-regional-rebuild-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let steps = cases["cases"][0]["steps"].as_array().unwrap();
    let mut before = Vec::new();
    for step in &steps[..2] {
        let mut args = step["input"].clone();
        if let Some(uid) = args.pointer_mut("/changes/0/uid") {
            let slot: usize = uid.as_str().unwrap().trim_start_matches("$uidOf:").parse().unwrap();
            *uid = uids[slot - 1].clone();
        }
        args["drawing"] = drawing.clone();
        let r = call(&mut server, step["command"].as_str().unwrap(), args);
        assert_eq!(r["isError"], false, "{r}");
        if step["command"] == "road.terrains.contours" {
            before = r["structuredContent"]["output"]["uids"].as_array().unwrap().clone();
        }
    }
    let expected = &steps[2]["result"]["output"]["rebuilt"][0];
    let again = call(&mut server, "road.project.rebuild", json!({"drawing": drawing, "regional": true}));
    assert_eq!(again["isError"], false, "{again}");
    let row = &again["structuredContent"]["output"]["rebuilt"][0];
    assert_eq!((&row["removed"], &row["kept"]), (&expected["removed"], &expected["kept"]), "{row}");
    for k in 0..4 {
        let (got, want) = (row["region"][k].as_f64().unwrap(), expected["region"][k].as_f64().unwrap());
        assert!((got - want).abs() < 1e-6, "region[{k}]: {got} ≠ {want}");
    }
    let after = row["uids"].as_array().unwrap();
    let stayed = after.iter().filter(|u| before.contains(u)).count();
    assert_eq!(stayed as u64, expected["kept"].as_u64().unwrap(), "the lines alike keep their ids");

    // Saved and opened again: up to date; drawn again by name, nothing is written.
    let saved = std::env::temp_dir().join(format!("kentos-regional-rebuild-saved-{}.kcad", std::process::id()));
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let reopened = call(&mut server, "drawing.open", json!({"path": saved}));
    assert_eq!(reopened["isError"], false, "{reopened}");
    let back = reopened["structuredContent"]["drawing"].clone();
    let named = call(&mut server, "road.project.rebuild", json!({"drawing": back, "drawings": ["cz-1"], "regional": true}));
    assert_eq!(named["isError"], false, "{named}");
    let row = &named["structuredContent"]["output"]["rebuilt"][0];
    assert_eq!((&row["removed"], &row["kept"]), (&json!(0), &json!(after.len())), "{row}");
    assert_eq!(row["uids"].as_array().unwrap(), after);
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(saved);
}

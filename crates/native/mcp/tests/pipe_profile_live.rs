//! A pipe run profile kept with its network through a real AI host (plan/map-cad.md MAP-16, docs/adr/3560): the shared
//! cases' Harita project opened as a .kcad file over MCP. `cad.pipes.profile` draws the sewer's run from P1; the
//! project's tree has it under the network, up to date; P1's material written makes it stale, the network named;
//! `road.project.rebuild` draws it again (as many objects); saved and opened again the tree is still up to date (the
//! objects' stamp leaves out their numbers in the document, which a file read again renumbers).
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"pipe-profile-live-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const P1: &str = "0192f640-1640-7000-8000-0000000000f1";

/// The profile's row in the tree: its state and why.
fn profile_row(s: &mut Server, drawing: &Value) -> (String, Vec<String>) {
    let r = call(s, "road.project.query", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    let nodes = r["structuredContent"]["output"]["nodes"].as_array().unwrap().clone();
    let network = nodes.iter().position(|n| n["kind"] == "network").expect("the network");
    assert_eq!(nodes[network]["name"], "Atıksu");
    let row = nodes.iter().find(|n| n["kind"] == "drawing").expect("the profile").clone();
    assert_eq!((&row["name"], &row["parent"]), (&json!("Hat profili · Profil"), &json!(network)));
    let why = row["why"].as_array().map(|w| w.iter().map(|x| x.as_str().unwrap().to_owned()).collect()).unwrap_or_default();
    (row["state"].as_str().unwrap().to_owned(), why)
}

#[test]
fn the_profile_follows_its_network_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.profile.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-pipe-profile-live-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let input = json!({"drawing": drawing, "network": "atiksu", "start": P1,
                       "inverts": {"from": "fields", "start": "Akar baş", "end": "Akar son"},
                       "diameter": {"field": "Çap", "unit": "mm"}, "ground": {"from": "surface", "surface": "Zemin"},
                       "layerId": "profil", "at": [487000.0, 4419700.0]});
    let drawn = call(&mut server, "cad.pipes.profile", input);
    assert_eq!(drawn["isError"], false, "{drawn}");
    assert_eq!(drawn["structuredContent"]["output"]["uids"].as_array().map(Vec::len), Some(112));
    assert_eq!(profile_row(&mut server, &drawing), ("current".to_owned(), Vec::new()));
    let set = call(&mut server, "cad.entities.set", json!({"drawing": drawing, "uids": [P1], "attrs": {"Malzeme": "PVC"}, "operation": "attributes"}));
    assert_eq!(set["isError"], false, "{set}");
    assert_eq!(profile_row(&mut server, &drawing), ("stale".to_owned(), vec!["“Atıksu” ağı çizildiğinden beri değişti.".to_owned()]));
    let again = call(&mut server, "road.project.rebuild", json!({"drawing": drawing}));
    assert_eq!(again["isError"], false, "{again}");
    let rebuilt = &again["structuredContent"]["output"]["rebuilt"][0];
    assert_eq!((&rebuilt["command"], &rebuilt["removed"]), (&json!("cad.pipes.profile"), &json!(112)));
    assert_eq!(profile_row(&mut server, &drawing).0, "current");
    // Saved and opened again: still up to date.
    let saved = std::env::temp_dir().join(format!("kentos-pipe-profile-live-saved-{}.kcad", std::process::id()));
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let reopened = call(&mut server, "drawing.open", json!({"path": saved}));
    assert_eq!(reopened["isError"], false, "{reopened}");
    let again = reopened["structuredContent"]["drawing"].clone();
    assert_eq!(profile_row(&mut server, &again), ("current".to_owned(), Vec::new()));
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(saved);
}

//! A drawing's record keeps the version that drew it and the revision its write made, through a real AI host
//! (plan/map-cad.md MAP-20, docs/adr/2840): the graph cases' map project opened as a .kcad file over MCP, its zero line
//! drawn. İş grafiği's recipe says version 1 and the revision the command answered; the drawing saved and read back
//! is schema 209 and its record keeps both.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"drawing-provenance-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_zero_line_s_record_keeps_its_version_and_revision_in_the_file() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.graph.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-drawing-provenance-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("harita.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let drawn = call(
        &mut server,
        "road.terrains.zeroline",
        json!({"drawing": drawing, "base": "Düz", "comparison": "Tümsek", "layerId": "sifir"}),
    );
    assert_eq!(drawn["isError"], false, "{drawn}");
    let revision = drawn["structuredContent"]["output"]["revision"].as_str().unwrap().to_owned();
    let g = call(&mut server, "road.project.graph", json!({"drawing": drawing}));
    assert_eq!(g["isError"], false, "{g}");
    let recipe = g["structuredContent"]["output"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["kind"] == "drawing")
        .unwrap()["recipe"]
        .clone();
    assert_eq!((&recipe["made"], &recipe["revision"]), (&json!(1), &json!(revision)));
    let saved = dir.join("harita-cizildi.kcad");
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let bytes = std::fs::read(&saved).unwrap();
    assert!(bytes.windows(10).any(|w| w == b"\x67version\x18\xd1"), "the file is schema 209");
    let back = serde_json::to_value(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    let record = &back["settings"]["road"]["drawings"][0];
    assert_eq!((&record["version"], &record["revision"]), (&json!(1), &json!(revision)));
    let _ = std::fs::remove_dir_all(dir);
}

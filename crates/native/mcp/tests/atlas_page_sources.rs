//! Each atlas page's sources through a real AI host (plan/map-cad.md MAP-18, docs/adr/3600): the shared cases' atlas
//! drawing with the atlas map alone (`plain`), opened as a .kcad file over MCP, its sheets added as a script adds them.
//! `file.export.atlas`'s plan lists on every page its own parcel first and as many objects as the case's ground test
//! finds, and gives a digest for every object a page lists.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"atlas-page-sources-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn every_atlas_page_lists_its_sources_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/atlas/v1/cases.json")).unwrap();
    let setup = &cases["setups"]["plain"];
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(setup["drawing"].clone()).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-atlas-page-sources-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("atlas.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    for sheet in setup["book"]["sheets"].as_array().unwrap() {
        let r = call(&mut server, "cad.sheets.edit", json!({"drawing": drawing, "ops": [{"op": "addSheet", "sheet": sheet}]}));
        assert_eq!(r["isError"], false, "{r}");
    }
    let r = call(&mut server, "file.export.atlas", json!({"drawing": drawing, "op": "plan", "path": dir.join("atlas.pdf"), "sheet": "atlas-paftasi"}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Plan: haritası tek olan sayfa"))).unwrap();
    let want = case["steps"][0]["result"]["output"]["pages"].as_array().unwrap();
    let pages = out["pages"].as_array().unwrap();
    assert_eq!(pages.len(), want.len());
    for (got, w) in pages.iter().zip(want) {
        let sources = got["sources"].as_array().unwrap();
        assert_eq!(sources.len(), w["sources"].as_array().unwrap().len(), "{}", got["name"]);
        assert_eq!(sources[0], got["feature"], "{}: its own object first", got["name"]);
        for u in sources {
            assert!(out["objects"][u.as_str().unwrap()].as_str().is_some_and(|d| d.len() == 64), "{u}");
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

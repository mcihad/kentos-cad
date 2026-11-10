//! Atlas sayfasının görünüşü through a real AI host (plan/map-cad.md MAP-18, docs/adr/4820): the shared cases' atlas
//! drawing with the atlas map alone (`plain`), opened as a .kcad file over MCP. `file.export.atlas` writes the record
//! with each page's look; an attribute no page prints (`Sır`) changes: against the record, the pages listing the parcel
//! changed and look the same; its `Ada`, which its own page prints, changes: that page looks other, its neighbours the
//! same.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"atlas-look-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// The changed pages' looks: (is it the parcel's own page, its look).
fn looks(answer: &Value, parcel: &Value) -> Vec<(bool, String)> {
    answer["structuredContent"]["output"]["changes"]["pages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["state"] == "changed")
        .map(|p| (p["feature"] == *parcel, p["look"].as_str().unwrap_or("").to_owned()))
        .collect()
}

#[test]
fn a_page_looks_other_only_where_it_draws_the_change() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/atlas/v1/cases.json")).unwrap();
    let setup = &cases["setups"]["plain"];
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(setup["drawing"].clone()).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-atlas-look-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("atlas.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    for sheet in setup["book"]["sheets"].as_array().unwrap() {
        let r = call(&mut server, "cad.sheets.edit", json!({"drawing": drawing, "ops": [{"op": "addSheet", "sheet": sheet}]}));
        assert_eq!(r["isError"], false, "{r}");
    }
    let written = call(&mut server, "file.export.atlas", json!({"drawing": drawing, "path": dir.join("atlas.pdf"), "sheet": "atlas-paftasi"}));
    assert_eq!(written["isError"], false, "{written}");
    let record = dir.join("atlas.kaynaklar.json");
    let kept: Value = serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
    assert!(kept["pages"].as_array().unwrap().iter().all(|p| p["look"].as_str().is_some_and(|l| l.len() == 64)), "each page's look");

    let items = call(&mut server, "drawing.entities", json!({"drawing": drawing}))["structuredContent"]["items"].clone();
    let parcel = items.as_array().unwrap().iter().find(|r| r["entity"]["id"] == 2).expect("parcel 2")["uid"].clone();
    let again = |server: &mut Server, attrs: Value, name: &str| {
        let set = call(server, "cad.entities.set", json!({"drawing": drawing, "uids": [parcel], "operation": "attributes", "attrs": attrs}));
        assert_eq!(set["isError"], false, "{set}");
        let made = call(server, "file.export.atlas", json!({"drawing": drawing, "path": dir.join(name), "sheet": "atlas-paftasi", "previous": record}));
        assert_eq!(made["isError"], false, "{made}");
        made
    };
    // Sır, which no page prints: every changed page looks the same.
    let hidden = looks(&again(&mut server, json!({"Sır": "yeni-not"}), "atlas-2.pdf"), &parcel);
    assert!(hidden.len() > 1 && hidden.iter().all(|(_, l)| l == "same"), "{hidden:?}");
    // Its Ada, which its own page prints: that page looks other, its neighbours the same.
    let ada = looks(&again(&mut server, json!({"Ada": "A9"}), "atlas-3.pdf"), &parcel);
    assert!(ada.iter().all(|(own, l)| l == if *own { "changed" } else { "same" }), "{ada:?}");
    assert_eq!(ada.iter().filter(|(own, _)| *own).count(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

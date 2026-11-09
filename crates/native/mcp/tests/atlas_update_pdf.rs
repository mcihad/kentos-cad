//! Atlasın güncelleme PDF'i through a real AI host (plan/map-cad.md MAP-18, docs/adr/4400): the shared cases' atlas
//! drawing with the atlas map alone (`plain`), opened as a .kcad file over MCP. The whole atlas is written with its
//! record; an attribute of parcel 2 changes; `file.export.atlas` with `changedOnly` writes the update's PDF of the
//! pages the shared case names (`written`), its record the whole atlas's with them; with nothing changed since, it is
//! refused (`atlas_unchanged`).
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"atlas-update-pdf-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_update_pdf_over_mcp_holds_the_case_s_pages() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/atlas/v1/cases.json")).unwrap();
    let setup = &cases["setups"]["plain"];
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(setup["drawing"].clone()).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-atlas-update-pdf-{}", std::process::id()));
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
    let written = call(&mut server, "file.export.atlas", json!({"drawing": drawing, "path": dir.join("atlas.pdf"), "sheet": "atlas-paftasi"}));
    assert_eq!(written["isError"], false, "{written}");
    let record = dir.join("atlas.kaynaklar.json");
    // Nothing changed since: no update.
    let none = call(&mut server, "file.export.atlas", json!({"drawing": drawing, "path": dir.join("bos.pdf"), "sheet": "atlas-paftasi", "previous": record, "changedOnly": true}));
    assert_eq!(none["isError"], true, "{none}");
    assert_eq!(none["structuredContent"]["error"]["code"], "atlas_unchanged", "{none}");
    assert!(!dir.join("bos.pdf").exists());

    // Parcel 2 by its persistent id; another attribute than the owner (hidden from agents) changes its digest.
    let items = call(&mut server, "drawing.entities", json!({"drawing": drawing}))["structuredContent"]["items"].clone();
    let parcel = items.as_array().unwrap().iter().find(|r| r["entity"]["id"] == 2).expect("parcel 2")["uid"].clone();
    let set = call(&mut server, "cad.entities.set", json!({"drawing": drawing, "uids": [parcel], "operation": "attributes", "attrs": {"Sır": "yeni-not"}}));
    assert_eq!(set["isError"], false, "{set}");
    let update = dir.join("guncelleme.pdf");
    let r = call(&mut server, "file.export.atlas", json!({"drawing": drawing, "path": update, "sheet": "atlas-paftasi", "previous": record, "changedOnly": true}));
    assert_eq!(r["isError"], false, "{r}");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Güncelleme PDF'i: yalnız")).unwrap();
    let want = &case["steps"][2]["result"]["output"]["written"];
    assert_eq!(&r["structuredContent"]["output"]["written"], want, "{r}");
    assert!(update.exists());
    let again: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("guncelleme.kaynaklar.json")).unwrap()).unwrap();
    assert_eq!(again["pages"].as_array().map(Vec::len), Some(100), "the whole atlas's pages");
    assert_eq!(&again["written"], want);
    let _ = std::fs::remove_dir_all(dir);
}

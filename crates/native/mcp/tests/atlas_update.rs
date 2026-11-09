//! Atlasın güncelleme yayını through a real AI host (plan/map-cad.md MAP-18, docs/adr/4000): the shared cases' atlas
//! drawing with the atlas map alone (`plain`), opened as a .kcad file over MCP. `file.export.atlas` writes the record
//! beside the PDF; an attribute of parcel 2 changes (`cad.entities.set`; its owner is hidden from agents); the plan
//! against the record says each page as the shared case does (the pages listing parcel 2 changed, the others the
//! same); a missing record is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"atlas-update-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_pages_against_the_previous_atlas_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/atlas/v1/cases.json")).unwrap();
    let setup = &cases["setups"]["plain"];
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(setup["drawing"].clone()).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-atlas-update-{}", std::process::id()));
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
    assert_eq!(written["structuredContent"]["output"]["record"], json!(record), "{written}");
    assert!(record.exists());

    // Parcel 2 (the drawing's second object) by its persistent id.
    let items = call(&mut server, "drawing.entities", json!({"drawing": drawing}))["structuredContent"]["items"].clone();
    let parcel = items.as_array().unwrap().iter().find(|r| r["entity"]["id"] == 2).expect("parcel 2")["uid"].clone();
    // The owner is hidden from agents (the form's redact): another attribute of parcel 2 changes its digest as well.
    let set = call(&mut server, "cad.entities.set", json!({"drawing": drawing, "uids": [parcel], "operation": "attributes", "attrs": {"Sır": "yeni-not"}}));
    assert_eq!(set["isError"], false, "{set}");
    let planned = call(&mut server, "file.export.atlas", json!({"drawing": drawing, "op": "plan", "path": dir.join("atlas-2.pdf"), "sheet": "atlas-paftasi", "previous": record}));
    assert_eq!(planned["isError"], false, "{planned}");
    let got: Vec<(u64, String)> = planned["structuredContent"]["output"]["changes"]["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["index"].as_u64().unwrap(), p["state"].as_str().unwrap().to_owned()))
        .collect();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Güncelleme: kayıt PDF'in yanına")).unwrap();
    let want: Vec<(u64, String)> = case["steps"][2]["result"]["output"]["changes"]["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["index"].as_u64().unwrap(), p["state"].as_str().unwrap().to_owned()))
        .collect();
    assert_eq!(got, want);

    let refused = call(&mut server, "file.export.atlas", json!({"drawing": drawing, "op": "plan", "path": dir.join("atlas-3.pdf"), "sheet": "atlas-paftasi", "previous": dir.join("yok.kaynaklar.json")}));
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(refused["structuredContent"]["error"]["code"], "no_previous", "{refused}");
    let _ = std::fs::remove_dir_all(dir);
}

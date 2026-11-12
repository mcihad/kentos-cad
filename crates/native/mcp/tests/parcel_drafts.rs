//! Belge taslağı ve onayı through a real AI host (plan/map-cad.md MAP-08, docs/adr/5020): the shared case's drawing
//! (fixtures/commands/v1/cad.parcels.record.json, “Taslak, kenar ölçüsüz: …”: Kadastro 1985's 101/7, 101/8 and 101/9)
//! opened as a .kcad file over MCP. The ifraz is written as a draft (nothing current changes), its approval as given
//! writes it as document 2, and the register then lists the approval; a second draft is withdrawn.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"parcel-drafts-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_draft_is_approved_and_another_withdrawn_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.record.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Taslak, kenar ölçüsüz")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-parcel-drafts-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.parcels.record", input);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["draft"].clone(), out["document"].clone()), (json!(true), json!(1)), "{out}");
    let listed = call(&mut server, "cad.parcels.records", json!({"drawing": drawing}));
    let o = &listed["structuredContent"]["output"];
    assert_eq!((o["current"].clone(), o["drafts"][0]["name"].clone()), (json!(3), json!("2026/145 ifraz")), "{o}");
    let approved = call(
        &mut server,
        "cad.parcels.approve",
        json!({"drawing": drawing, "draft": 1, "approval": {"reference": "2026/145-K", "date": "2026-03-20"}}),
    );
    assert_eq!(approved["isError"], false, "{approved}");
    assert_eq!(approved["structuredContent"]["output"]["document"], 2, "{approved}");
    let terkin = call(
        &mut server,
        "cad.parcels.record",
        json!({"drawing": drawing, "document": {"name": "2026/9 terkin"}, "close": [2], "draft": true}),
    );
    assert_eq!(terkin["isError"], false, "{terkin}");
    let withdrawn = call(&mut server, "cad.parcels.withdraw", json!({"drawing": drawing, "draft": 1}));
    assert_eq!(withdrawn["structuredContent"]["output"]["name"], "2026/9 terkin", "{withdrawn}");
    let listed = call(&mut server, "cad.parcels.records", json!({"drawing": drawing}));
    let o = &listed["structuredContent"]["output"];
    assert!(o.get("drafts").is_none(), "{o}");
    let docs = o["documents"].as_array().unwrap();
    assert_eq!(docs[1]["approval"], json!({"reference": "2026/145-K", "date": "2026-03-20"}), "{o}");
}

//! Detay eşleştirme's decisions as review notes through a real AI host (plan/map-cad.md MAP-04, docs/adr/4140): the
//! shared cases' drawing (fixtures/commands/v1/map.features.reconcile.json, with its own note 3) opened as a .kcad file
//! over MCP. The four decisions are the notes 4–7 that `cad.review.notes.list` gives back, resolved, by the reviewer,
//! each with its decision; the asset written carries no `İnceleme`; a time that is no short label is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"reconcile-notes-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_decisions_are_review_notes_over_mcp() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.features.reconcile.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Kararlar inceleme notu")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-reconcile-notes-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing.clone();

    // A time that is no short label.
    let mut long = input.clone();
    long["reviewedAt"] = json!("2".repeat(65));
    let r = call(&mut server, "map.features.reconcile", long);
    assert_eq!(r["structuredContent"]["error"]["path"], "reviewedAt", "{r}");

    let r = call(&mut server, "map.features.reconcile", input.clone());
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["notes"], json!([4, 5, 6, 7]), "{out}");
    let listed = call(&mut server, "cad.review.notes.list", json!({"drawing": drawing}));
    assert_eq!(listed["isError"], false, "{listed}");
    let notes = listed["structuredContent"]["output"]["notes"].as_array().unwrap().clone();
    assert_eq!(notes.len(), 5);
    let decisions = input["decisions"].as_array().unwrap();
    for (n, given) in notes[1..].iter().zip(decisions) {
        let note = &n["note"];
        assert_eq!((note["status"].as_str(), note["author"].as_str()), (Some("resolved"), Some("Ayşe Demir")), "{note}");
        assert_eq!(note["decision"]["candidate"], given["candidate"]);
        for key in ["accept", "class", "fields", "reject"] {
            assert_eq!(note["decision"].get(key), given.get(key), "{key}: {note}");
        }
        assert_eq!(n["stale"], false, "{n}");
    }
    // The asset written carries no review attributes.
    let asset = notes[1]["note"]["decision"]["asset"].clone();
    let e = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": asset}));
    assert_eq!(e["isError"], false, "{e}");
    let attrs = &e["structuredContent"]["entity"]["attrs"];
    assert!(attrs.get("İnceleme").is_none() && attrs.get("Gözden geçiren").is_none(), "{attrs}");
}

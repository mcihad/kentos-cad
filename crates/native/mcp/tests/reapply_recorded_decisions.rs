//! Detay eşleştirme's recorded decisions taken again by a model's new run through a real AI host (plan/map-cad.md
//! MAP-04, docs/adr/4540): the shared case's drawing after a reviewed run and a new one (fixtures/commands/v1/
//! map.features.reconcile.json, “Yeniden çıkarım”) opened as a .kcad file over MCP. Without `reapply` nothing is taken;
//! with it the four new candidates near decided ones take the notes 2, 4, 3 and 1, the far one none; the notes Eşleştir
//! writes name them.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"reapply-decisions-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_recorded_decisions_are_taken_again_over_mcp() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.features.reconcile.json")).unwrap();
    let case =
        cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Yeniden çıkarım, plan")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-reapply-decisions-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing.clone();

    // Without reapply: nothing taken, and nothing to mark but the existing ones.
    let mut plain = input.clone();
    plain["reapply"] = json!(false);
    plain["op"] = json!("plan");
    let r = call(&mut server, "map.features.reconcile", plain);
    assert_eq!(r["isError"], false, "{r}");
    assert!(r["structuredContent"]["output"].get("reapplied").is_none(), "{r}");

    input["reviewer"] = json!("Ayşe Demir");
    let r = call(&mut server, "map.features.reconcile", input);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let taken: Vec<u64> = out["reapplied"].as_array().unwrap().iter().map(|t| t["note"].as_u64().unwrap()).collect();
    assert_eq!(taken, [2, 4, 3, 1], "{out}");
    // The far one (the drawing's 22nd object, past the radius) takes none.
    let far = case["setup"]["uids"][21].clone();
    assert!(out["reapplied"].as_array().unwrap().iter().all(|t| t["candidate"] != far), "{out}");
    assert_eq!(out["notes"], json!([5, 6, 7, 8]), "{out}");
    let listed = call(&mut server, "cad.review.notes.list", json!({"drawing": drawing}));
    let notes = listed["structuredContent"]["output"]["notes"].as_array().unwrap().clone();
    for (n, earlier) in notes[4..].iter().zip(taken) {
        let text = n["note"]["text"].as_str().unwrap();
        assert!(text.ends_with(&format!("Önceki karar (not {earlier}) yeniden uygulandı.")), "{text}");
    }
}

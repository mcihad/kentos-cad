//! Plan değişikliği taslağı and the kept symbol choices through a real AI host (plan/map-cad.md MAP-09,
//! docs/adr/5040): the shared cases' drawing (fixtures/commands/v1/cad.zoning.draft.json: plans UİP-1 and UİP-2,
//! decision areas) opened as a .kcad file over MCP. The tool is listed as destructive; a draft turning the park into
//! trade keeps the park as it is in force, says the park's area under m. 26(3), and the zoning check is clean; a kept
//! symbol choice is read on the next run; one undo takes the draft out.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"zoning-draft-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}})
}

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":meta()}})).unwrap()["result"].clone()
}

#[test]
fn a_plan_change_draft_over_mcp() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.zoning.draft.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let entities = cases["setup"]["entities"].as_array().unwrap().clone();
    let uid_of = |decision: &str| {
        let at = entities.iter().position(|e| e["attrs"]["KararKimlik"] == decision).unwrap();
        cases["setup"]["uids"][at].as_str().unwrap().to_owned()
    };
    let path = std::env::temp_dir().join(format!("kentos-zoning-draft-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    let listed = server.handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}})).unwrap();
    let tool = listed["result"]["tools"].as_array().unwrap().iter().find(|t| t["name"] == "cad.zoning.draft").cloned();
    assert_eq!(tool.expect("listed")["annotations"]["destructiveHint"], true);
    let entity = |server: &mut Server, decision: &str| {
        call(server, "drawing.entity", json!({"drawing": drawing, "uid": uid_of(decision)}))["structuredContent"]["entity"].clone()
    };

    let park = entity(&mut server, "K1");
    let r = call(
        &mut server,
        "cad.zoning.draft",
        json!({"drawing": drawing, "id": "T-1", "plan": "UİP-1", "reason": "Parkın yeri değişiyor.",
               "changes": [{"uid": uid_of("K1"), "field": "Kullanim", "value": "ticaret"}]}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["draft"]["changes"][0]["before"], "park", "{out}");
    assert_eq!(out["findings"][0]["kind"], "infrastructureLess", "{out}");
    assert_eq!(out["findings"][0]["area"].as_f64(), Some(400.0), "{out}");
    // The plan in force is as it was.
    assert_eq!(entity(&mut server, "K1"), park);
    let checked = call(&mut server, "cad.zoning.check", json!({"drawing": drawing}));
    let stale = checked["structuredContent"]["output"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == "draftStale")
        .count();
    assert_eq!(stale, 0, "{checked}");

    // A kept choice is read on the next run.
    let pick = json!([{"level": "uip", "use": "konut", "symbol": "mpyy.uip.konut.gelisme-konut-alani"}]);
    let r = call(&mut server, "cad.zoning.symbolize", json!({"drawing": drawing, "symbols": pick, "keep": true}));
    assert_eq!(r["isError"], false, "{r}");
    let r = call(&mut server, "cad.zoning.symbolize", json!({"drawing": drawing}));
    let konut = r["structuredContent"]["output"]["areas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["uid"] == uid_of("K3").as_str())
        .cloned()
        .unwrap();
    assert_eq!((konut["from"].as_str(), konut["state"].as_str()), (Some("project"), Some("same")), "{konut}");

    // Two undos: the kept choice, then the draft.
    for _ in 0..2 {
        let undo = call(&mut server, "drawing.undo", json!({"drawing": drawing}));
        assert_eq!(undo["isError"], false, "{undo}");
    }
    let r = call(&mut server, "cad.zoning.draft", json!({"drawing": drawing, "id": "T-1", "remove": true}));
    assert_eq!(r["isError"], true, "the draft is undone: {r}");
}

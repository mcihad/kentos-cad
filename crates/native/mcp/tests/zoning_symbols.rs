//! Sembolü karardan ver through a real AI host (plan/map-cad.md MAP-09, docs/adr/4240): the shared cases' drawing
//! (fixtures/commands/v1/cad.zoning.symbolize.json: plans UİP-1 and NİP-1, decision areas) opened as a .kcad file over
//! MCP. The park area takes the UİP park symbol, the area whose symbol said Park but whose decision is ticaret takes the
//! ticaret symbol, the konut area is left open; the user's pick fills it; the decisions (attributes) never change; one
//! undo takes a step back.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"zoning-symbols-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn symbols_from_decisions_over_mcp() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.zoning.symbolize.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let entities = cases["setup"]["entities"].as_array().unwrap().clone();
    let uid_of = |decision: &str| {
        let at = entities.iter().position(|e| e["attrs"]["KararKimlik"] == decision).unwrap();
        cases["setup"]["uids"][at].as_str().unwrap().to_owned()
    };
    let path = std::env::temp_dir().join(format!("kentos-zoning-symbols-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    // The tool writes over the areas' symbols: it is listed as destructive.
    let listed = server.handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"zoning-symbols-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap();
    let tool = listed["result"]["tools"].as_array().unwrap().iter().find(|t| t["name"] == "cad.zoning.symbolize").cloned();
    assert_eq!(tool.expect("listed")["annotations"]["destructiveHint"], true);
    let entity = |server: &mut Server, decision: &str| {
        call(server, "drawing.entity", json!({"drawing": drawing, "uid": uid_of(decision)}))["structuredContent"]["entity"].clone()
    };

    let r = call(&mut server, "cad.zoning.symbolize", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["set"].as_u64(), out["unmapped"].as_u64()), (Some(3), Some(3)), "{out}");
    assert_eq!(entity(&mut server, "K1")["symbol"], "mpyy.uip.acik-yesil.park");
    let wrong = entity(&mut server, "K4");
    assert_eq!((wrong["symbol"].as_str(), wrong["attrs"]["Kullanim"].as_str()), (Some("mpyy.uip.calisma.ticaret-alani"), Some("ticaret")));
    assert!(entity(&mut server, "K3").get("symbol").is_none());

    // The user's pick for konut fills the open one; its decision stays.
    let pick = json!([{"level": "uip", "use": "konut", "symbol": "mpyy.uip.konut.gelisme-konut-alani"}]);
    let r = call(&mut server, "cad.zoning.symbolize", json!({"drawing": drawing, "symbols": pick}));
    assert_eq!(r["isError"], false, "{r}");
    let konut = entity(&mut server, "K3");
    assert_eq!((konut["symbol"].as_str(), konut["attrs"]["Kullanim"].as_str()), (Some("mpyy.uip.konut.gelisme-konut-alani"), Some("konut")));

    let undo = call(&mut server, "drawing.undo", json!({"drawing": drawing}));
    assert_eq!(undo["isError"], false, "{undo}");
    assert!(entity(&mut server, "K3").get("symbol").is_none());
    assert_eq!(entity(&mut server, "K1")["symbol"], "mpyy.uip.acik-yesil.park");
}

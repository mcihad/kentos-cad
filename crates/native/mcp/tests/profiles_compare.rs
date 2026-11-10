//! Katalog geçiş raporu through a real AI host (plan/map-cad.md MAP-04, docs/adr/4940): the shared cases' drawing, with
//! the institution's new catalog carried inside it (docs/adr/0735), opened as a .kcad file over MCP.
//! `cad.profiles.compare` gives the first case's report with the new catalog by itself and by its name; an unknown new
//! catalog is refused at `to`; the drawing is not changed.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"profiles-compare-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_catalog_report_over_mcp_is_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.profiles.compare.json")).unwrap();
    let step = &cases["cases"][0]["steps"][1];
    let catalog = step["input"]["toProfile"].clone();
    let mut want = step["result"]["output"].clone();
    let mut setup = cases["setup"].clone();
    setup["settings"]["profileFiles"] = json!([{ "kind": "profile", "name": "belediye-harita", "text": catalog.to_string() }]);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-profiles-compare-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let by_itself = call(&mut server, "cad.profiles.compare", json!({"drawing": drawing, "from": "harita-varliklari", "toProfile": catalog}));
    assert_eq!(by_itself["isError"], false, "{by_itself}");
    let got = by_itself["structuredContent"]["output"].clone();
    want["revision"] = got["revision"].clone();
    assert_eq!(got, want, "the new catalog given by itself");
    let by_name = call(&mut server, "cad.profiles.compare", json!({"drawing": drawing, "from": "harita-varliklari", "to": "belediye-harita"}));
    assert_eq!(by_name["isError"], false, "{by_name}");
    assert_eq!(by_name["structuredContent"]["output"], want, "the drawing's own catalog by its name");
    let unknown = call(&mut server, "cad.profiles.compare", json!({"drawing": drawing, "from": "harita-varliklari", "to": "yok-boyle"}));
    assert_eq!(unknown["isError"], true, "{unknown}");
    assert!(unknown.to_string().contains("unknown_profile"), "{unknown}");
    let again = call(&mut server, "cad.profiles.compare", json!({"drawing": drawing, "from": "harita-varliklari", "to": "belediye-harita"}));
    assert_eq!(again["structuredContent"]["output"]["revision"], got["revision"], "nothing written");
    let _ = std::fs::remove_file(&path);
}

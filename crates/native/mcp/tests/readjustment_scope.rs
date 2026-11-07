//! Düzenleme kapsamı through a real AI host (plan/map-cad.md MAP-11, docs/adr/2060): the shared cases' drawing of
//! `cad.readjustment.scope` opened as a .kcad file over MCP gives the main case's statuses, bases and counts, and a
//! boundary that is no uid is refused with the case's code, place and words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"readjustment-scope-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_scope_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.scope.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-readjustment-scope-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let main = &cases["cases"][0]["steps"][0];
    let mut args = main["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.readjustment.scope", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"];
    let want = &main["result"]["output"];
    for key in ["basis", "whole", "partial", "ready", "review", "rule"] {
        assert_eq!(got[key], want[key], "{key}");
    }
    let pick = |v: &Value, key: &str| -> Vec<Value> { v["parcels"].as_array().unwrap().iter().map(|p| p[key].clone()).collect() };
    for key in ["parcel", "kind", "status", "basis", "outsideRecorded", "pieces", "slivers"] {
        assert_eq!(pick(got, key), pick(want, key), "{key}");
    }
    // A boundary that is no uid: the refusal's code, place and words, as the case says.
    let refusal = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "sınırın kimliği geçersiz").unwrap();
    let mut args = refusal["steps"][0]["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "cad.readjustment.scope", args);
    assert_eq!(r["isError"], true, "{r}");
    let text = r.to_string();
    let error = &refusal["steps"][0]["result"]["error"];
    for key in ["code", "message", "path"] {
        assert!(text.contains(error[key].as_str().unwrap()), "{key}: {text}");
    }
    let _ = std::fs::remove_file(path);
}

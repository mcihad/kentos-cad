//! İzleme incelemesi through a real AI host (plan/map-cad.md MAP-19, docs/adr/5220): the review cases' drawing opened
//! as a .kcad file over MCP. `cad.asbuilt.review` keeps the monitoring job with its thresholds' source and the
//! reviewer's statement as the shared case does (its digest the case's), lists it as current, and a review asked
//! without the thresholds' source is refused with the case's words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"asbuilt-review-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_review_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.review.json")).unwrap();
    let case = |prefix: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with(prefix)).unwrap().clone();
    let mut server = Server::new();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-asbuilt-review-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let step = case("üç dönemli izleme")["steps"][0].clone();
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.asbuilt.review", args);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let want = &step["result"]["output"];
    assert_eq!(out["kept"]["digest"], want["kept"]["digest"]);
    assert_eq!(out["kept"]["job"], want["kept"]["job"]);
    assert_eq!(out["kept"]["reviewer"], want["kept"]["reviewer"]);
    let states = |v: &Value| v["kept"]["marks"].as_array().unwrap().iter().map(|m| (m["name"].clone(), m["state"].clone())).collect::<Vec<_>>();
    assert_eq!(states(out), states(want));
    assert_eq!(out["reviews"], json!([{ "name": "Ekim incelemesi", "reviewer": "A. Yılmaz", "date": "2026-10-20",
        "phrase": "checked", "alarm": want["reviews"][0]["alarm"], "warning": want["reviews"][0]["warning"], "current": true }]));

    let step = case("eşiğin kaynağı yok")["steps"][0].clone();
    let mut args = step["input"].clone();
    args["drawing"] = drawing;
    let said = call(&mut server, "cad.asbuilt.review", args).to_string();
    assert!(said.contains(step["result"]["error"]["message"].as_str().unwrap()), "{said}");
    let _ = std::fs::remove_file(path);
}

//! Each map's own survey quality through a real AI host (plan/map-cad.md MAP-18, docs/adr/2800): the drawing of
//! fixtures/sheet/v1/survey-scope.json (scripts/fixtures/sheet_survey_scope_cases.py, no KentOS code) opened as a .kcad
//! file over MCP, its sheet of three maps added with `cad.sheets.edit`. The print check (`file.preflight.sheets`) says
//! only the text tied to the map over no accepted point, naming the placeholder it would print.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"map-survey-scope-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_print_check_over_mcp_says_the_map_over_no_point() {
    let case: Value = serde_json::from_str(include_str!("../../../../fixtures/sheet/v1/survey-scope.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["drawing"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-map-survey-scope-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let edited = call(&mut server, "cad.sheets.edit", json!({"drawing": drawing, "ops": [{"op": "addSheet", "sheet": case["sheet"]}]}));
    assert_eq!(edited["isError"], false, "{edited}");
    let checked = call(&mut server, "file.preflight.sheets", json!({"drawing": drawing}));
    assert_eq!(checked["isError"], false, "{checked}");
    let _ = std::fs::remove_file(path);
    let said: Vec<String> = checked["structuredContent"]["output"]["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["code"] == "survey_quality_missing")
        .map(|i| i["message"].as_str().unwrap_or("").to_owned())
        .collect();
    assert_eq!(said.len(), case["findings"].as_array().map(Vec::len).unwrap_or(0), "{said:?}");
    assert!(said[0].starts_with("“c_konum” yazısı"), "{}", said[0]);
    assert!(said[0].contains("haritanın içinde GNSS ile kabul edilmiş nokta yok; yerine ‹konum_dogrulugu?› basılır"), "{}", said[0]);
}

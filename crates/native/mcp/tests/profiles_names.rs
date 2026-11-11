//! Kurumdaki alan adları through a real AI host (plan/map-cad.md MAP-17, docs/adr/5180): the shared cases' empty drawing
//! opened as a .kcad file over MCP. `cad.profiles.names` gives the cases' answer for BÜYÜK_HARF in ASCII; a mapping that
//! is the form alone is read by `cad.profiles.check`, one whose form puts two fields on one name is refused; an unknown
//! mapping is refused at `mappingName`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"profiles-names-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_names_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.profiles.names.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("BÜYÜK_HARF, ASCII")))
        .expect("the case");
    let step = &case["steps"][1];
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-profiles-names-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let got = call(&mut server, "cad.profiles.names", args);
    assert_eq!(got["isError"], false, "{got}");
    assert_eq!(got["structuredContent"]["output"], step["result"]["output"], "the cases' answer");
    let alone = json!({"format": "kentos.profile-mapping", "version": 1, "name": "buyuk-harf", "title": "Büyük harfli adlar",
                       "profile": "tucbs-temel", "names": {"case": "upperSnakeCase", "letters": "ascii"}, "classes": []});
    let checked = call(&mut server, "cad.profiles.check", json!({"drawing": drawing, "name": "tucbs-temel", "mapping": alone}));
    assert_eq!(checked["isError"], false, "{checked}");
    assert_eq!(checked["structuredContent"]["output"]["mapping"], "buyuk-harf", "{checked}");
    // A form that puts two fields of a class on one name: the mapping does not hold.
    let clashing = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["steps"].as_array().unwrap().iter())
        .find(|s| s["input"]["profile"]["name"] == "cakisan")
        .map(|s| s["input"]["profile"].clone())
        .expect("the clashing profile");
    let bad = json!({"format": "kentos.profile-mapping", "version": 1, "name": "cakisan-bicim", "title": "Çakışan biçim",
                     "profile": "cakisan", "names": {"case": "upperSnakeCase", "letters": "ascii"}, "classes": []});
    let refused = call(&mut server, "cad.profiles.check", json!({"drawing": drawing, "profile": clashing, "mapping": bad}));
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(refused.to_string().contains("invalid_profile_mapping"), "{refused}");
    let unknown = call(&mut server, "cad.profiles.names", json!({"drawing": drawing, "name": "tucbs-temel", "mappingName": "yok-boyle"}));
    assert_eq!(unknown["isError"], true, "{unknown}");
    assert!(unknown.to_string().contains("unknown_mapping"), "{unknown}");
    let _ = std::fs::remove_file(&path);
}

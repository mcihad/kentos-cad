//! The survey quality on a sheet through a real AI host (plan/map-cad.md MAP-18, docs/adr/2400): the map project of
//! `fixtures/kcad/v2/map-survey-network.json` opened as a .kcad file over MCP, a sheet added whose texts read
//! `@konum_dogrulugu` and `@olcme_kalitesi`. The print check (`file.preflight.sheets`) finds both values there; the
//! same project without its GNSS archive leaves both empty, and the check says so for each text
//! (`survey_quality_missing`).
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"sheet-survey-quality-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn text(id: &str, top: i64, content: &str) -> Value {
    json!({"id": id, "name": id, "frame": {"left": 20000, "top": top, "width": 380000, "height": 12000},
           "kind": {"type": "text", "content": content}})
}

/// The texts the print check says read a quality the project lacks, with the project's archive or without it.
fn empty_texts(archive: bool) -> Vec<String> {
    let mut v: Value =
        serde_json::from_str(include_str!("../../../../fixtures/kcad/v2/map-survey-network.json")).unwrap();
    if !archive {
        v["settings"].as_object_mut().unwrap().remove("mapSurvey");
    }
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(v).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-sheet-quality-{}-{archive}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let sheet = json!({"id": "pafta", "name": "Pafta",
        "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                 "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
        "items": [text("konum", 20000, "Konum: [% @konum_dogrulugu %]"), text("not", 40000, "[% @olcme_kalitesi %]")]});
    let edited = call(&mut server, "cad.sheets.edit", json!({"drawing": drawing, "ops": [{"op": "addSheet", "sheet": sheet}]}));
    assert_eq!(edited["isError"], false, "{edited}");
    let checked = call(&mut server, "file.preflight.sheets", json!({"drawing": drawing}));
    assert_eq!(checked["isError"], false, "{checked}");
    let _ = std::fs::remove_file(path);
    let mut found: Vec<String> = checked["structuredContent"]["output"]["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["code"] == "survey_quality_missing" && i["error"] == false)
        // The text's name, as the message quotes it first (“konum” yazısı …).
        .map(|i| i["message"].as_str().unwrap_or("").split('”').next().unwrap_or("").trim_start_matches('“').to_owned())
        .collect();
    found.sort();
    found
}

#[test]
fn the_print_check_over_mcp_finds_the_archive_s_quality() {
    assert!(empty_texts(true).is_empty(), "the archive gives both values");
    assert_eq!(empty_texts(false), ["konum", "not"], "no archive, no quality: each text said");
}

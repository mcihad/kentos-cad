//! Separation sheets through a real AI host (plan/map-cad.md MAP-11, docs/adr/2680): the shared case's drawing opened as
//! a .kcad file over MCP; `cad.readjustment.sheets` for the three parcels partly inside makes the case's sheets (names,
//! scales, centres, the tables' rows, the points' count), and asked again it says the first sheet is there already.
//! The parts outside take the parcel numbers they are registered with (m. 13/3, docs/adr/3080): with them the table of
//! parts is the case's, one short it is refused and nothing is written.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"separation-sheets-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.sheets.json")).unwrap()
}

fn case(cases: &Value, name: &str) -> Value {
    cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone()
}

/// The case's drawing opened over MCP from a .kcad file named by `tag`: the server, the drawing's handle, the file.
fn opened(cases: &Value, tag: &str) -> (Server, Value, std::path::PathBuf) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-separation-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    (server, drawing, path)
}

#[test]
fn the_separation_sheets_over_mcp_are_the_case_s() {
    let cases = cases();
    let case = case(&cases, "kısmen giren üç parsel: üç ayırma çapı paftası, noktalar parsellerin katmanlarında");
    let (mut server, drawing, path) = opened(&cases, "sheets");
    let mut args = case["steps"][0]["input"].clone();
    args["drawing"] = drawing.clone();
    let answer = call(&mut server, "cad.readjustment.sheets", args.clone());
    assert_eq!(answer["isError"], false, "{answer}");
    let got = answer["structuredContent"]["output"]["sheets"].as_array().unwrap().clone();
    let want = case["steps"][0]["result"]["output"]["sheets"].as_array().unwrap().clone();
    assert_eq!(got.len(), 3);
    for (g, w) in got.iter().zip(&want) {
        for key in ["sheet", "name", "uid", "parcel", "scale", "points", "areas", "parts"] {
            assert_eq!(g[key], w[key], "{key}");
        }
        for i in 0..2 {
            assert!((g["center"][i].as_f64().unwrap() - w["center"][i].as_f64().unwrap()).abs() < 1e-6);
        }
        assert!(g["layer"].as_str().is_some_and(|l| !l.is_empty()), "the points' layer");
    }
    let again = call(&mut server, "cad.readjustment.sheets", args);
    assert_eq!(again["isError"], true, "{again}");
    assert!(again.to_string().contains("separation_sheet_exists"), "{again}");
    let _ = std::fs::remove_file(path);
}

#[test]
fn the_parts_outside_take_their_parcel_numbers_over_mcp() {
    let cases = cases();
    let (mut server, drawing, path) = opened(&cases, "numbers");
    // One number for two parts outside: refused at the parcel's numbers, nothing written.
    let short = case(&cases, "parsel numaraları: parça sayısı tutmuyor");
    let mut args = short["steps"][0]["input"].clone();
    args["drawing"] = drawing.clone();
    let refused = call(&mut server, "cad.readjustment.sheets", args);
    assert_eq!(refused["isError"], true, "{refused}");
    let want = &short["steps"][0]["result"]["error"];
    let text = refused.to_string();
    assert!(text.contains(want["message"].as_str().unwrap()), "{refused}");
    assert!(text.contains(want["path"].as_str().unwrap()), "{refused}");
    // Both numbers (the sheet is not there yet: the refusal wrote nothing): the table of parts names the parts outside
    // with them, as the case's.
    let named = case(&cases, "dışarıda kalan iki parçanın parsel numaraları yazılınca Kısımlar'da adlarıyla (m. 13/3)");
    let mut args = named["steps"][0]["input"].clone();
    args["drawing"] = drawing;
    let answer = call(&mut server, "cad.readjustment.sheets", args);
    assert_eq!(answer["isError"], false, "{answer}");
    let got = &answer["structuredContent"]["output"]["sheets"];
    let want = &named["steps"][0]["result"]["output"]["sheets"];
    assert_eq!(got.as_array().map(Vec::len), Some(1));
    assert_eq!(got[0]["parts"], want[0]["parts"]);
    assert_eq!(got[0]["parts"][1][0], "Girmeyen 1 · parsel 101/24");
    assert_eq!(got[0]["parts"][2][0], "Girmeyen 2 · parsel 101/25", "the number as written, trimmed");
    let _ = std::fs::remove_file(path);
}

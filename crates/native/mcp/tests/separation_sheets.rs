//! Separation sheets through a real AI host (plan/map-cad.md MAP-11, docs/adr/2680): the shared case's drawing opened as
//! a .kcad file over MCP; `cad.readjustment.sheets` for the three parcels partly inside makes the case's sheets (names,
//! scales, centres, the tables' rows, the points' count), and asked again it says the first sheet is there already.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"separation-sheets-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_separation_sheets_over_mcp_are_the_case_s() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.sheets.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "kısmen giren üç parsel: üç ayırma çapı paftası, noktalar parsellerin katmanlarında")
        .unwrap()
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-separation-sheets-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
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

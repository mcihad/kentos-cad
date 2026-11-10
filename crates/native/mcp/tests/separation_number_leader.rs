//! The outside pieces' numbers on leaders when they do not fit, through a real AI host (plan/map-cad.md MAP-11,
//! docs/adr/4680): the shared case's drawing opened as a .kcad file over MCP; `cad.readjustment.sheets` with the numbers,
//! 6 mm and Kılavuz answers the case's labels (the 4 m pieces' numbers outside at the full height, each with its
//! leader's tip) and writes those numbers as leaders, not texts; a leader without a height is refused at its path.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"separation-number-leader-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_numbers_on_leaders_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.sheets.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("numara yazısı 6 mm, kılavuzla")))
        .unwrap()
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-separation-number-leader-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.readjustment.sheets", args);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let want = step["result"]["output"]["sheets"].as_array().unwrap();
    let mut leaders = 0;
    for (got, wanted) in out["sheets"].as_array().unwrap().iter().zip(want) {
        for (g, w) in got["labels"].as_array().unwrap().iter().zip(wanted["labels"].as_array().unwrap()) {
            assert_eq!(g["number"], w["number"]);
            assert_eq!(g["fits"], w["fits"], "{g}");
            assert_eq!(g.get("leader").is_some(), w.get("leader").is_some(), "{g}");
            for k in 0..2 {
                assert!((g["at"][k].as_f64().unwrap() - w["at"][k].as_f64().unwrap()).abs() < 1e-6, "{g}");
            }
            if let Some(tip) = w.get("leader") {
                leaders += 1;
                for k in 0..2 {
                    assert!((g["leader"][k].as_f64().unwrap() - tip[k].as_f64().unwrap()).abs() < 1e-6, "{g}");
                }
            }
        }
    }
    assert_eq!(leaders, 2, "101/10's two 4 m pieces");
    let r = call(
        &mut server,
        "cad.readjustment.sheets",
        json!({"drawing": drawing, "scope": step["input"]["scope"], "numbers": step["input"]["numbers"], "numberLeader": true}),
    );
    assert_eq!(r["structuredContent"]["error"]["path"], "numberLeader", "{r}");
}

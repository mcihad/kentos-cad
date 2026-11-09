//! The outside pieces' numbers fitted inside them through a real AI host (plan/map-cad.md MAP-11, docs/adr/4280): the
//! shared case's drawing opened as a .kcad file over MCP; `cad.readjustment.sheets` with the numbers and a height on
//! paper answers the case's labels (place, height, turn, share, fit) and writes each number as a text, not a point, at
//! that place, height and turn.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"separation-number-fit-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_fitted_numbers_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.sheets.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("numara yazısı 6 mm"))).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-separation-number-fit-{}.kcad", std::process::id()));
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
    let mut labels = Vec::new();
    for (g, w) in out["sheets"].as_array().unwrap().iter().zip(want) {
        for (a, b) in g["labels"].as_array().unwrap().iter().zip(w["labels"].as_array().unwrap()) {
            assert_eq!((&a["number"], &a["fits"], &a["share"]), (&b["number"], &b["fits"], &b["share"]), "{a}");
            for key in ["height", "rotation"] {
                assert!((a[key].as_f64().unwrap() - b[key].as_f64().unwrap()).abs() < 1e-6, "{key}: {a} {b}");
            }
            for i in 0..2 {
                assert!((a["at"][i].as_f64().unwrap() - b["at"][i].as_f64().unwrap()).abs() < 1e-6, "{a} {b}");
            }
            labels.push(a.clone());
        }
    }
    // Each number is a text at its label's place, height and turn; no labelled point.
    let uids: Vec<String> = out["uids"].as_array().unwrap().iter().map(|u| u.as_str().unwrap().to_owned()).collect();
    let mut texts = 0;
    for uid in uids {
        let e = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid}))["structuredContent"]["entity"].clone();
        if e["kind"] == "point" {
            assert!(e.get("label").is_none_or(|l| !l.as_str().unwrap_or("").starts_with("101/")), "{e}");
        }
        if e["kind"] == "text" {
            let label = labels.iter().find(|l| l["number"] == e["text"]).expect("its label");
            assert!((e["height"].as_f64().unwrap() - label["height"].as_f64().unwrap()).abs() < 1e-9);
            assert!((e["rotation"].as_f64().unwrap() - label["rotation"].as_f64().unwrap()).abs() < 1e-9);
            texts += 1;
        }
    }
    assert_eq!(texts, labels.len());
}

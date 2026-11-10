//! Pafta eğrileri through a real AI host (plan/map-cad.md MAP-14, docs/adr/4740): the shared cases' drawing opened as a
//! .kcad file over MCP. `road.terrains.sheetcontours` gives the cases' m. 63(2) heights and their m. 93(1)(c) check with
//! its warning; drawn, the drawing is kept and `road.project.edit` chooses drawing again whole.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"sheet-contour-drawing-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// Two numbers' lists or numbers alike within the cases' 10⁻⁶.
fn near(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (k, (x, y)) in a.iter().zip(b).enumerate() {
                near(x, y, &format!("{at}[{k}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                near(a.get(k).unwrap_or(&Value::Null), v, &format!("{at}.{k}"));
            }
        }
        _ => match (got.as_f64(), want.as_f64()) {
            (Some(x), Some(y)) => assert!((x - y).abs() < 1e-6, "{at}: {x} {y}"),
            _ => assert_eq!(got, want, "{at}"),
        },
    }
}

#[test]
fn the_sheet_s_heights_check_and_kept_drawing_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.sheetcontours.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-sheet-contour-drawing-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = |start: &str| -> Value {
        cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start))).unwrap()["steps"][0].clone()
    };
    // m. 63(2): the road's and the built-up area's heights.
    let s = step("m. 63(2) planı yazmaz");
    let mut args = s["input"].clone();
    args["drawing"] = drawing.clone();
    args["op"] = json!("plan");
    let r = call(&mut server, "road.terrains.sheetcontours", args);
    assert_eq!(r["isError"], false, "{r}");
    near(&r["structuredContent"]["output"]["spots"], &s["result"]["output"]["spots"], "spots");
    // m. 93(1)(c): over the limit, said as a warning.
    let s = step("m. 93(1)(c): farkların");
    let mut args = s["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "road.terrains.sheetcontours", args);
    assert_eq!(r["isError"], false, "{r}");
    near(&r["structuredContent"]["output"]["check"], &s["result"]["output"]["check"], "check");
    assert!(r.to_string().contains("section_check_exceeded"), "{r}");
    // Drawn above, the drawing is kept; drawing again whole is chosen.
    let r = call(&mut server, "road.project.edit", json!({"drawing": drawing.clone(), "regional": false}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["output"]["regional"], false, "{r}");
    let r = call(&mut server, "road.project.query", json!({"drawing": drawing.clone()}));
    assert_eq!(r["structuredContent"]["output"]["regional"], false, "{r}");
    let nodes = r["structuredContent"]["output"]["nodes"].as_array().unwrap();
    assert!(nodes.iter().any(|n| n["kind"] == "drawing" && n["command"] == "road.terrains.sheetcontours"), "{r}");
    let _ = std::fs::remove_file(&path);
}

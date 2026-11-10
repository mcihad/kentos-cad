//! Sections along a drawn axis through a real AI host (plan/map-cad.md MAP-15, docs/adr/4760): the shared cases'
//! drawings opened as .kcad files over MCP. `road.terrains.compare` along the axis with a quarter circle gives the case's
//! sections and their ends; `road.terrains.cutfill` planned with sections from km 1+000 and the materials gives the
//! cases' kilometres, words and tables.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"axis-sections-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// Two JSON values alike, numbers within the cases' 10⁻⁶ (the expected's keys read).
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

/// The file `name`'s cases, its drawing opened over MCP, and the case starting `start`.
fn opened(server: &mut Server, cases: &Value, tag: &str) -> Value {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-axis-sections-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(&path);
    opened["structuredContent"]["drawing"].clone()
}

fn step(cases: &Value, start: &str) -> Value {
    cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start))).unwrap()["steps"][0].clone()
}

#[test]
fn sections_along_an_axis_over_mcp_are_the_cases() {
    let mut server = Server::new();
    let sections: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.compare.sections.json")).unwrap();
    let drawing = opened(&mut server, &sections, "compare");
    let s = step(&sections, "Yaylı eksen: kurpta");
    let mut args = s["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "road.terrains.compare", args);
    assert_eq!(r["isError"], false, "{r}");
    near(&r["structuredContent"]["output"]["sections"], &s["result"]["output"]["sections"], "sections");
    let cutfill: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.cutfill.json")).unwrap();
    let drawing = opened(&mut server, &cutfill, "cutfill");
    for start in ["Kesitler eksen boyunca", "Malzeme bölgeleri"] {
        let s = step(&cutfill, start);
        let mut args = s["input"].clone();
        args["drawing"] = drawing.clone();
        args["op"] = json!("plan");
        let r = call(&mut server, "road.terrains.cutfill", args);
        assert_eq!(r["isError"], false, "{start}: {r}");
        let got = &r["structuredContent"]["output"];
        for key in ["sections", "sectionLabels", "sectionTable", "materials", "materialLabels", "materialTable"] {
            if let Some(want) = s["result"]["output"].get(key) {
                near(&got[key], want, &format!("{start}.{key}"));
            }
        }
    }
}

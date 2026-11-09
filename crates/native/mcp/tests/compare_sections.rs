//! A surface comparison by sections through a real AI host (plan/map-cad.md MAP-15, docs/adr/4340): the section cases'
//! drawing opened as a .kcad file over MCP. `road.terrains.compare` by sections north–south, prismoidal, and across a
//! slanted axis answers the volumes, the exact TIN beside and every section's areas as the cases do; an azimuth of 360°
//! is refused with the case's words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"compare-sections-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// Every number in `got` within 1e-6 of `want`'s, the words the same.
fn same(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-6, "{at}: {a} ≠ {b}");
        }
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                same(&a[k], v, &format!("{at}.{k}"));
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(x, y, &format!("{at}[{i}]"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

#[test]
fn the_sections_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.compare.sections.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-compare-sections-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let case = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap()["steps"][0].clone();
    for name in ["Kesitler kuzey–güney (semt 90°) her 2,5 m", "Prizmoidal: ortadaki kesitle", "Eğik eksen: semt 30°"] {
        let step = case(name);
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "road.terrains.compare", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        let got = &r["structuredContent"]["output"];
        let want = &step["result"]["output"];
        for key in ["cut", "fill", "net", "exact", "interval", "azimuth", "formula", "sections"] {
            same(&got[key], &want[key], &format!("{name} {key}"));
        }
    }
    let step = case("Semt 360");
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let said = call(&mut server, "road.terrains.compare", args).to_string();
    assert!(said.contains(step["result"]["error"]["message"].as_str().unwrap()), "{said}");
    let _ = std::fs::remove_file(path);
}

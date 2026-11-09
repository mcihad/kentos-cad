//! Material zones in a surface comparison through a real AI host (plan/map-cad.md MAP-15, docs/adr/3540): the shared
//! cases' drawing opened as a .kcad file over MCP. `road.terrains.compare` with rock and earth over the terrace answers
//! each zone's part, the rest and the quantity by material as the case does; a zone half off the base answers the
//! case's warning at its boundary.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"compare-material-zones-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// Every number in `got` within 1e-9 of `want`'s, the words the same.
fn same(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-9, "{at}: {a} ≠ {b}");
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{at}");
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
fn the_material_zones_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.compare.zones.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-compare-zones-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    for name in ["İki malzeme bölgesi", "Yarısı tabanın dışında bölge"] {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).unwrap().clone();
        let step = &case["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "road.terrains.compare", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
        for key in ["zones", "rest", "zoned"] {
            same(&got[key], &want[key], &format!("{name} {key}"));
        }
        let warned: Vec<&Value> = r["structuredContent"]["warnings"].as_array().unwrap().iter().filter(|w| w["path"].as_str().is_some_and(|p| p.starts_with("zones"))).collect();
        let wanted: Vec<&Value> = step["result"]["warnings"].as_array().unwrap().iter().filter(|w| w["path"].as_str().is_some_and(|p| p.starts_with("zones"))).collect();
        assert_eq!(warned, wanted, "{name}");
    }
    let _ = std::fs::remove_file(path);
}

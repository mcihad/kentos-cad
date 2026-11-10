//! Tarama ağı dengelemesi through a real AI host (plan/map-cad.md MAP-13, docs/adr/4720): the shared cases' drawing
//! (seven synthetic scans of the fixtures, their paths made absolute) opened as a .kcad file over MCP.
//! `cad.pointclouds.network` gives the cases' scans, links and loops; two scans are refused at `uids`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"pointcloud-network-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// A text with its `$fixture:` paths made absolute, as the shared runners make them.
fn fixtures(v: &Value) -> Value {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/");
    serde_json::from_str(&v.to_string().replace("$fixture:", &root.display().to_string())).unwrap()
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
fn the_network_over_mcp_is_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pointclouds.network.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(fixtures(&cases["setup"])).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-pointcloud-network-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    for name in [
        "üç tarama, A sabit, Huber ağırlığıyla: üç bağ, bir döngü; bilinen kaymalar 1 cm içinde",
        "bağlar verilir: zincir A–B, B–C, döngü yok, artık yok",
        "iki tarama",
    ] {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
        let step = &case["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "cad.pointclouds.network", args);
        if step["result"]["status"] == "failed" {
            assert_eq!(r["isError"], true, "{name}: {r}");
            assert!(r.to_string().contains("en az üç taramayla"), "{name}: {r}");
            continue;
        }
        assert_eq!(r["isError"], false, "{name}: {r}");
        near(&r["structuredContent"]["output"], &step["result"]["output"], name);
    }
    let _ = std::fs::remove_file(&path);
}

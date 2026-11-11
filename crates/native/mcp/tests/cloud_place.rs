//! Bulutu konumla through a real AI host (plan/map-cad.md MAP-13, docs/adr/5100): the shared cases' drawing (a text
//! cloud of the fixtures, its paths made absolute) opened as a .kcad file over MCP. `cad.pointclouds.place` with Euler
//! angles gives the case's places; `cad.pointclouds.query` then finds the point where the drawing shows it; a matrix
//! that scales is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"cloud-place-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixtures(v: &Value) -> Value {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/");
    serde_json::from_str(&v.to_string().replace("$fixture:", &root.display().to_string())).unwrap()
}

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
                if v.as_str().is_some_and(|s| s.starts_with('$')) {
                    continue;
                }
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
fn placing_a_cloud_over_mcp_is_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pointclouds.place.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(fixtures(&cases["setup"])).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-cloud-place-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let uid = snapshot.uids[0].clone();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Euler: üç açı")).unwrap().clone();
    let steps = case["steps"].as_array().unwrap();
    let mut args = steps[0]["input"].clone();
    args["uid"] = json!(uid);
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.pointclouds.place", args);
    assert_eq!(r["isError"], false, "{r}");
    near(&r["structuredContent"]["output"], &steps[0]["result"]["output"], "place");
    // Nokta sorgula then reads the file's point where the drawing shows it.
    let mut query = steps[1]["input"].clone();
    query["uids"] = json!([uid]);
    query["drawing"] = drawing.clone();
    let q = call(&mut server, "cad.pointclouds.query", query);
    assert_eq!(q["isError"], false, "{q}");
    near(&q["structuredContent"]["output"]["point"], &steps[1]["result"]["output"]["point"], "query");
    // A matrix that scales is refused.
    let scaled = call(
        &mut server,
        "cad.pointclouds.place",
        json!({"drawing": drawing, "uid": uid, "place": {"kind": "matrix", "matrix": [1.001, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]}}),
    );
    assert_eq!(scaled["isError"], true, "{scaled}");
    assert!(scaled.to_string().contains("katı dönüşüm"), "{scaled}");
    let _ = std::fs::remove_file(&path);
}

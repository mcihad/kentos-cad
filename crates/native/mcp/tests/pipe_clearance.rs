//! The clearances to another network's pipes through a real AI host (plan/map-cad.md MAP-16, docs/adr/2360): the
//! shared cases' drawing opened as a .kcad file over MCP. `cad.pipes.check` with `clearance` gives each case's
//! crossings (inverts, clearance, the pipe above), near passes and findings: the sewer under the water network with
//! the least vertical clearance and the water above, with the least horizontal one, and the storm network's arc.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"pipe-clearance-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASES: [&str; 3] = [
    "açıklık: en az düşey açıklık ve suyun üstte olması",
    "açıklık: en az yatay açıklık; kesişmeden yakın geçen boru",
    "açıklık: yağmursuyunun yaylı borusunda kesişim ve yanından geçiş",
];

/// Two lists of crossings or near passes alike: the same pipes and side, the measures within 10⁻⁹ (an answer leaves
/// an empty list out).
fn alike(got: &Value, want: &Value, what: &str) {
    let (g, w) = (got.as_array().cloned().unwrap_or_default(), want.as_array().cloned().unwrap_or_default());
    assert_eq!(g.len(), w.len(), "{what}: {got}");
    for (g, w) in g.iter().zip(&w) {
        for key in ["pipe", "other", "above"] {
            assert_eq!(g[key], w[key], "{what} {key}");
        }
        for key in ["invert", "otherInvert", "clearance", "distance"] {
            match (g[key].as_f64(), w[key].as_f64()) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{what} {key}: {a} {b}"),
                (a, b) => assert_eq!(a.is_some(), b.is_some(), "{what} {key}"),
            }
        }
        for key in ["at", "otherAt"] {
            for i in 0..2 {
                match (g[key][i].as_f64(), w[key][i].as_f64()) {
                    (Some(a), Some(b)) => assert!((a - b).abs() < 1e-6, "{what} {key}[{i}]: {a} {b}"),
                    (a, b) => assert_eq!(a.is_some(), b.is_some(), "{what} {key}[{i}]"),
                }
            }
        }
    }
}

#[test]
fn the_clearances_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.check.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-pipe-clearance-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    for name in CASES {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
        let step = &case["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "cad.pipes.check", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
        alike(&got["crossings"], &want["crossings"], name);
        alike(&got["near"], &want["near"], name);
        assert_eq!(got["counts"], want["counts"], "{name}");
        let kinds = |v: &Value| -> Vec<(Value, Value)> {
            v["findings"].as_array().unwrap().iter().map(|f| (f["uid"].clone(), f["kind"].clone())).collect()
        };
        assert_eq!(kinds(got), kinds(want), "{name}");
    }
    // The checked network as the other: refused at its place.
    let mut args = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == CASES[0]).unwrap()["steps"][0]["input"].clone();
    args["drawing"] = drawing;
    args["clearance"]["network"] = json!("atiksu");
    let r = call(&mut server, "cad.pipes.check", args);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("clearance.network"), "{r}");
    let _ = std::fs::remove_file(path);
}

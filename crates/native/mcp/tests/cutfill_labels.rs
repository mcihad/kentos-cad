//! The cut and fill zones' labels and quantity table through a real AI host (plan/map-cad.md MAP-15, docs/adr/2340): the
//! shared cases' drawing opened as a .kcad file over MCP. `road.terrains.cutfill` planned with `labels` and `table`
//! gives each case's labels (words and places) and table cells, at the test lattice and at TM coordinates.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"cutfill-labels-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASES: [&str; 3] = [
    "Yazı ve tablo birlikte, plan yazmaz",
    "Bantlı miktar tablosu: alt ve üst sınırlarıyla",
    "Proje koordinatlarında yazılar",
];

#[test]
fn the_labels_and_table_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.cutfill.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-cutfill-labels-{}.kcad", std::process::id()));
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
        args["op"] = json!("plan");
        let r = call(&mut server, "road.terrains.cutfill", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
        assert_eq!(got["table"], want["table"], "{name}");
        let (gl, wl) = (got["labels"].as_array().cloned().unwrap_or_default(), want["labels"].as_array().cloned().unwrap_or_default());
        assert_eq!(gl.len(), wl.len(), "{name}");
        for (g, w) in gl.iter().zip(&wl) {
            assert_eq!(g["text"], w["text"], "{name}");
            for i in 0..2 {
                assert!((g["at"][i].as_f64().unwrap() - w["at"][i].as_f64().unwrap()).abs() < 1e-9, "{name} {i}");
            }
        }
    }
    let _ = std::fs::remove_file(path);
}

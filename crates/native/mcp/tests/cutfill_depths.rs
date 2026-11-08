//! The cut and fill zones' deepest places through a real AI host (plan/map-cad.md MAP-15, docs/adr/2740): the shared
//! cases' drawing opened as a .kcad file over MCP. `road.terrains.cutfill` planned with `depths` gives each case's
//! zones' deepest places and depths and the words written there, at the test lattice, banded, on the bank's ridge (equal
//! depths: the westmost, then the southmost) and at TM coordinates.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"cutfill-depths-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASES: [&str; 4] = [
    "En derin yer: kazının derinliği, dolgunun yüksekliği, noktası ve yazısıyla",
    "Bantlı bölgelerin en derin yerleri",
    "Setin sırtında eşit derinlikler: en batıdaki, sonra en güneydeki",
    "Proje koordinatlarında en derin yer",
];

#[test]
fn the_deepest_places_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.cutfill.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-cutfill-depths-{}.kcad", std::process::id()));
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
        let (gz, wz) = (got["zones"].as_array().unwrap(), want["zones"].as_array().unwrap());
        assert_eq!(gz.len(), wz.len(), "{name}");
        for (g, w) in gz.iter().zip(wz) {
            for i in 0..2 {
                assert!((g["deepest"][i].as_f64().unwrap() - w["deepest"][i].as_f64().unwrap()).abs() < 1e-6, "{name}");
            }
            assert!((g["depth"].as_f64().unwrap() - w["depth"].as_f64().unwrap()).abs() < 1e-9, "{name}");
        }
        let words = |v: &Value| v["depths"].as_array().unwrap().iter().map(|d| d["text"].clone()).collect::<Vec<_>>();
        assert_eq!(words(got), words(want), "{name}");
    }
    let _ = std::fs::remove_file(path);
}

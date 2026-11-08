//! The separation sheet's parts through a real AI host (plan/map-cad.md MAP-11, docs/adr/2280): the shared cases'
//! drawings opened as .kcad files over MCP. `cad.readjustment.scope` gives each partial parcel's parts as the cases
//! do: their sides, areas and rings (the L of two cells, the ring with a hole, the parcel with two parts outside).
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"separation-sheet-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASES: [&str; 3] = [
    "köşesi sınırın dışında kalan parsel: girmeyen kısım L biçiminde tek parça",
    "sınır parselin içinde: girmeyen kısım delikli tek parça",
    "iki parçası dışarıda kalan parsel: iki parça, hata ikisinde birlikte",
];

/// Whether two JSON values agree, numbers within 1e-6.
fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 1e-6,
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w))),
        _ => a == b,
    }
}

#[test]
fn the_parts_over_mcp_are_the_cases() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.scope.json")).unwrap();
    let mut server = Server::new();
    for (n, name) in CASES.iter().enumerate() {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == *name).unwrap().clone();
        let setup = if case["setup"].is_null() { cases["setup"].clone() } else { case["setup"].clone() };
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
        let path = std::env::temp_dir().join(format!("kentos-separation-sheet-{}-{n}.kcad", std::process::id()));
        std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
        let opened = call(&mut server, "drawing.open", json!({"path": path}));
        assert_eq!(opened["isError"], false, "{opened}");
        let mut args = case["steps"][0]["input"].clone();
        args["drawing"] = opened["structuredContent"]["drawing"].clone();
        let r = call(&mut server, "cad.readjustment.scope", args);
        assert_eq!(r["isError"], false, "{r}");
        let got = r["structuredContent"]["output"]["parcels"].as_array().cloned().unwrap_or_default();
        let want = case["steps"][0]["result"]["output"]["parcels"].as_array().cloned().unwrap_or_default();
        assert_eq!(got.len(), want.len(), "{name}");
        for (g, w) in got.iter().zip(&want) {
            assert!(close(&g["parts"], &w["parts"]), "{name}: {} != {}", g["parts"], w["parts"]);
        }
        let _ = std::fs::remove_file(path);
    }
}

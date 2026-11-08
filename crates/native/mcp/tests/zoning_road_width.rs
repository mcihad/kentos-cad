//! Decisions inside a road's planned width through a real AI host (plan/map-cad.md MAP-09, docs/adr/2240): the shared
//! cases' drawings opened as .kcad files over MCP. `cad.zoning.check` gives each case's findings: their kinds, objects,
//! areas and words, for the road wider than drawn and for the road that turns; the clean plan gives none.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"zoning-road-width-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASES: [&str; 3] = [
    "yollar planda yazılan genişlikte: yol kararları ve nazım planın kararı sayılmaz",
    "yol genişliği çizilenden geniş: iki yandaki dört karar yola giriyor",
    "dönen yol: iki kolunda da genişlik; dönüşün yuvarlak dış köşesi kararlara değmiyor",
];

#[test]
fn the_road_width_findings_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.zoning.check.json")).unwrap();
    let mut server = Server::new();
    for (n, name) in CASES.iter().enumerate() {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == *name).unwrap().clone();
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
        let path = std::env::temp_dir().join(format!("kentos-zoning-road-width-{}-{n}.kcad", std::process::id()));
        std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
        let opened = call(&mut server, "drawing.open", json!({"path": path}));
        assert_eq!(opened["isError"], false, "{opened}");
        let mut args = case["steps"][0]["input"].clone();
        args["drawing"] = opened["structuredContent"]["drawing"].clone();
        let r = call(&mut server, "cad.zoning.check", args);
        assert_eq!(r["isError"], false, "{r}");
        let got = r["structuredContent"]["output"]["findings"].as_array().cloned().unwrap_or_default();
        let want = case["steps"][0]["result"]["output"]["findings"].as_array().cloned().unwrap_or_default();
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (g, w) in got.iter().zip(&want) {
            for key in ["kind", "uids", "message"] {
                assert_eq!(g[key], w[key], "{name}: {key}");
            }
            let (a, b) = (g["area"].as_f64().unwrap(), w["area"].as_f64().unwrap());
            assert!((a - b).abs() < 1e-9, "{name}: {a} != {b}");
        }
        let _ = std::fs::remove_file(path);
    }
}

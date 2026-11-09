//! A stockpile over its corners triangulated through a real AI host (plan/map-cad.md MAP-15, docs/adr/3940): the
//! shared cases' drawing opened as a .kcad file over MCP. `road.terrains.stockpile` with `triangulated` answers the
//! pentagon's and the L's volumes and the base's triangles as the shared cases do, with no plane; corners on a line are
//! refused with the case's words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"stockpile-triangulated-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const READS: [&str; 2] = [
    "Beşgen, köşelerden üçgenlenmiş taban: üç üçgen, her biri kendi düzlemi",
    "L (iç bükey), üçgenlenmiş taban: girintide üçgen yok",
];

#[test]
fn the_triangulated_bases_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.stockpile.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-stockpile-triangulated-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let case = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap()["steps"][0].clone();
    for name in READS {
        let step = case(name);
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "road.terrains.stockpile", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
        for key in ["cut", "fill", "total", "area", "covered", "cutArea", "fillArea"] {
            assert!((got[key].as_f64().unwrap() - want[key].as_f64().unwrap()).abs() < 1e-6, "{name} {key}: {} vs {}", got[key], want[key]);
        }
        assert_eq!(got["plane"], Value::Null, "{name}: no one plane");
        let (g, w) = (got["triangles"].as_array().unwrap(), want["triangles"].as_array().unwrap());
        assert_eq!(g.len(), w.len(), "{name}");
        for (a, b) in g.iter().zip(w) {
            for k in 0..3 {
                for i in 0..3 {
                    assert!((a[k][i].as_f64().unwrap() - b[k][i].as_f64().unwrap()).abs() < 1e-9, "{name}: {a} vs {b}");
                }
            }
        }
    }
    let line = case("Köşeleri bir doğru üstünde: üçgenlenmiş taban kurulamaz");
    let mut args = line["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "road.terrains.stockpile", args);
    assert_eq!(r["isError"], true, "{r}");
    assert_eq!(r["structuredContent"]["error"]["code"], "invalid_stockpile", "{r}");
    assert_eq!(r["structuredContent"]["error"]["message"], line["result"]["error"]["message"], "{r}");
    let _ = std::fs::remove_file(path);
}

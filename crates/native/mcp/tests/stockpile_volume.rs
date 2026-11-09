//! A stockpile's volume through a real AI host (plan/map-cad.md MAP-15, docs/adr/3140): the shared cases' drawing opened
//! as a .kcad file over MCP. `road.terrains.stockpile` answers each case's volumes, base plane and corner heights (the
//! pentagon with the fitted plane and with a typed height, the holed area), warns where the surface does not reach,
//! and refuses a corner off the surface; nothing is written.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"stockpile-volume-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const READS: [&str; 4] = [
    "Beşgen, köşelere oturan eğimli düzlem (en küçük kareler)",
    "Beşgen, verilen kot 101,5",
    "Delikli alan, köşelerin ortalaması: deliğin hacmi sayılmaz",
    "Yüzeyi aşan sınır, verilen kot: örtülmeyen yer uyarıyla, köşe kotları yok",
];

#[test]
fn the_stockpile_volumes_over_mcp_are_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.stockpile.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-stockpile-volume-{}.kcad", std::process::id()));
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
        for key in ["cut", "fill", "total", "area", "covered", "uncovered", "cutArea", "fillArea"] {
            assert!((got[key].as_f64().unwrap() - want[key].as_f64().unwrap()).abs() < 1e-6, "{name} {key}: {} vs {}", got[key], want[key]);
        }
        // a and b to 10⁻⁹; c, the plane's height at the project's origin millions of metres away, to the cases' 10⁻⁶.
        for (i, within) in [(0, 1e-9), (1, 1e-9), (2, 1e-6)] {
            assert!((got["plane"][i].as_f64().unwrap() - want["plane"][i].as_f64().unwrap()).abs() < within, "{name} plane {i}");
        }
        assert_eq!(got["corners"].as_array().map(Vec::len), want["corners"].as_array().map(Vec::len), "{name}");
        let warned = r.to_string().contains("terrain_no_data");
        assert_eq!(warned, !step["result"]["warnings"].as_array().unwrap().is_empty(), "{name}: {r}");
    }
    let off = case("Yüzeyi aşan sınır, köşeden taban: dışarıdaki köşe ret");
    let mut args = off["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "road.terrains.stockpile", args);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("invalid_stockpile"), "{r}");
    let _ = std::fs::remove_file(path);
}

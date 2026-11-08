//! The summits' and pits' heights through a real AI host (plan/map-cad.md MAP-14, docs/adr/2320): the shared cases'
//! drawing opened as a .kcad file over MCP. `road.terrains.sheetcontours` with `spots` plans each case's
//! characteristic points: the hill's summit, the summit outside the building, the bowl's pit; with `slopes`, the
//! corners of the slopes' top and bottom lines with the surface's heights, the summit first (docs/adr/3120).
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"sheet-spot-heights-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASES: [&str; 5] = [
    "Tepe kotu planı yazmaz",
    "Tepe kotu detaylarla: binanın içindeki köşe sayılmaz",
    "Çukur kotu: çukurun en içteki kapalı eğrisinde en alçak köşe",
    "Şev kotları planı yazmaz (1/5000)",
    "Tepe ve şev kotları birlikte: tepe önce, şevler sonra (1/5000)",
];

#[test]
fn the_spot_heights_over_mcp_are_the_cases() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.sheetcontours.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-sheet-spot-heights-{}.kcad", std::process::id()));
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
        let r = call(&mut server, "road.terrains.sheetcontours", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        let (got, want) = (&r["structuredContent"]["output"]["spots"], &step["result"]["output"]["spots"]);
        assert_eq!(got.as_array().map(Vec::len), want.as_array().map(Vec::len), "{name}");
        for (g, w) in got.as_array().unwrap().iter().zip(want.as_array().unwrap()) {
            assert_eq!((&g["kind"], &g["text"]), (&w["kind"], &w["text"]), "{name}");
            for key in ["x", "y", "z"] {
                assert!((g[key].as_f64().unwrap() - w[key].as_f64().unwrap()).abs() < 1e-9, "{name} {key}");
            }
        }
    }
    let _ = std::fs::remove_file(path);
}

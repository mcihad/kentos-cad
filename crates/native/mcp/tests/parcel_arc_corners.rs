//! A missing corner on an arc boundary through a real AI host (plan/map-cad.md MAP-08, docs/adr/2220): the shared
//! case's drawing opened as a .kcad file over MCP. `cad.parcels.boundaries` on the arc and the two parcels above it
//! gives the case's problem: the corner at the arc's top, its parcels, its place. The corner across the chord on the
//! arc's circle gives none, as the case says.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"parcel-arc-corners-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const ON_ARC: &str = "yaylı sınırda eksik köşe: üstteki iki parselin köşesi alttakinin yayının tepesinde";
const ACROSS: &str = "yayın çemberinde ama kirişin öbür yanındaki köşe yayın üstünde değildir";

#[test]
fn the_arc_s_missing_corner_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.parcels.boundaries.json")).unwrap();
    let find = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
    let case = find(ON_ARC);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-parcel-arc-corners-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    for name in [ON_ARC, ACROSS] {
        let step = &find(name)["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "cad.parcels.boundaries", args);
        assert_eq!(r["isError"], false, "{r}");
        let got = r["structuredContent"]["output"]["problems"].as_array().cloned().unwrap_or_default();
        let want = step["result"]["output"]["problems"].as_array().cloned().unwrap_or_default();
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (g, w) in got.iter().zip(&want) {
            assert_eq!((&g["kind"], &g["parcels"]), (&w["kind"], &w["parcels"]), "{name}");
            for i in 0..2 {
                let (a, b) = (g["at"][i].as_f64().unwrap(), w["at"][i].as_f64().unwrap());
                assert!((a - b).abs() < 1e-9, "{name}: {a} != {b}");
            }
        }
    }
    let _ = std::fs::remove_file(path);
}

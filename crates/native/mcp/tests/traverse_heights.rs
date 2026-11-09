//! A traverse's heights by trigonometric heighting through a real AI host (plan/map-cad.md MAP-02, docs/adr/4100): the
//! shared cases' drawing of `cad.survey.points` (fixtures/commands/v1/cad.survey.points.json) opened as a .kcad file over
//! MCP. The closed traverse with a 12 mm height error writes its points with the cases' heights and says the
//! misclosure; the one whose start point carries its own height takes it; a traverse without a start height is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"traverse-heights-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn case(cases: &Value, name: &str) -> Value {
    cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone()
}

/// The case's drawing opened over MCP, and its first step's input with the drawing.
fn opened(server: &mut Server, cases: &Value, c: &Value, tag: &str) -> Value {
    let setup = c.get("setup").unwrap_or(&cases["setup"]).clone();
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(setup).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-traverse-heights-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let open = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(open["isError"], false, "{open}");
    let _ = std::fs::remove_file(path);
    let mut input = c["steps"][0]["input"].clone();
    input["drawing"] = open["structuredContent"]["drawing"].clone();
    input
}

#[test]
fn a_traverse_s_heights_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.survey.points.json")).unwrap();
    let mut server = Server::new();
    for (name, tag) in [
        ("Poligon hesabı kotlarla: kapalı, 12 mm kot kapanması kenarlara dağıtılır", "kapali"),
        ("Poligon hesabı kotlarla: başlangıç kotu A noktasının kendisinden", "kendi"),
    ] {
        let c = case(&cases, name);
        let input = opened(&mut server, &cases, &c, tag);
        let r = call(&mut server, "cad.survey.points", input.clone());
        assert_eq!(r["isError"], false, "{name}: {r}");
        let out = &r["structuredContent"]["output"];
        let want = &c["steps"][0]["result"]["output"];
        let f = (out["heightMisclosure"].as_f64().unwrap() - want["heightMisclosure"].as_f64().unwrap()).abs();
        assert!(f < 1e-9, "{name}: {out}");
        // Each written point's height is the case's.
        let fields = c["steps"][0]["expect"]["fields"].as_object().unwrap();
        let uids = out["uids"].as_array().unwrap();
        for (uid, (_, w)) in uids.iter().zip(fields) {
            let got = call(&mut server, "drawing.entity", json!({"drawing": input["drawing"], "uid": uid}));
            assert_eq!(got["isError"], false, "{got}");
            let z = got["structuredContent"]["entity"]["z"].as_f64().unwrap();
            assert!((z - w["z"].as_f64().unwrap()).abs() < 1e-6, "{name}: {z} {w}");
        }
    }
    let c = case(&cases, "Poligon hesabı kotlarla: başlangıç kotu yok");
    let input = opened(&mut server, &cases, &c, "kotsuz");
    let r = call(&mut server, "cad.survey.points", input);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("heights.start"), "{r}");
}

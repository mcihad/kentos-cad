//! A traverse's reciprocal zenith angles through a real AI host (plan/map-cad.md MAP-02, docs/adr/4500): the shared
//! cases' drawing of `cad.survey.points` opened as a .kcad file over MCP. The case whose legs are all sighted back (the
//! air's k 0.2, the project's 0.13) gives the case's answer: no height misclosure and each pair's refraction
//! coefficient; a back sight list one short is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"traverse-reciprocal-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn opened(server: &mut Server, setup: &Value) -> Value {
    let mut setup = setup.clone();
    // The cases' setup is a version 1 snapshot: as version 2 it takes persistent ids.
    let n = setup["entities"].as_array().unwrap().len();
    setup["version"] = json!(2);
    setup["uids"] = json!((1..=n).map(|i| format!("0192f5c0-4500-7000-8000-{i:012x}")).collect::<Vec<_>>());
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-traverse-reciprocal-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(path);
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn reciprocal_zenith_angles_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.survey.points.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Poligon hesabı karşılıklı başucu açılarıyla: havanın")))
        .cloned()
        .expect("the case");
    let mut server = Server::new();
    let drawing = opened(&mut server, &cases["setup"]);
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.survey.points", args.clone());
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
    assert!(got["heightMisclosure"].as_f64().unwrap().abs() < 1e-9, "{got}");
    let (g, w) = (got["refractions"].as_array().unwrap(), want["refractions"].as_array().unwrap());
    assert_eq!(g.len(), w.len());
    for (a, b) in g.iter().zip(w) {
        assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-9, "{a} {b}");
    }

    // One back sight short: refused.
    let mut short = args;
    short["work"]["heights"]["back"].as_array_mut().unwrap().pop();
    let r = call(&mut server, "cad.survey.points", short);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("geri nişanlar 3 kenar için olmalı"), "{r}");
}

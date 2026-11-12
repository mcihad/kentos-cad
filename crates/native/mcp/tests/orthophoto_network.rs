//! Birleşme çizgisi ağı and Ayak izi through a real AI host (plan/map-cad.md MAP-12, docs/adr/5260): the shared cases'
//! drawings opened as .kcad files over MCP. Three orthophotos' network has the case's junction and elements' areas; a
//! raster's footprint from its NoData has the case's parts and area; a single image is refused with the case's words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"orthophoto-network-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn opened(server: &mut Server, setup: &Value, tag: &str) -> Value {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    let text = setup.to_string().replace("$fixture:", &format!("{}/", fixtures.display()));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(&text).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-ortho-network-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(path);
    opened["structuredContent"]["drawing"].clone()
}

fn case(cases: &Value, prefix: &str) -> Value {
    cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with(prefix)).unwrap().clone()
}

fn close(a: &Value, b: &Value) {
    assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-6, "{a} ≠ {b}");
}

#[test]
fn the_network_and_the_footprint_over_mcp_are_the_cases() {
    let mut server = Server::new();
    let net: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.orthophoto.network.json")).unwrap();
    let c = case(&net, "üç görüntü");
    let drawing = opened(&mut server, &c["setup"], "net");
    let mut args = c["steps"][0]["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.orthophoto.network", args);
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"], &c["steps"][0]["result"]["output"]);
    close(&got["junctions"][0]["at"][0], &want["junctions"][0]["at"][0]);
    close(&got["junctions"][0]["at"][1], &want["junctions"][0]["at"][1]);
    for k in 0..3 {
        close(&got["elements"][k]["area"], &want["elements"][k]["area"]);
    }
    let lone = case(&net, "tek görüntü");
    let mut args = lone["steps"][0]["input"].clone();
    args["drawing"] = drawing;
    let said = call(&mut server, "cad.orthophoto.network", args).to_string();
    assert!(said.contains(lone["steps"][0]["result"]["error"]["message"].as_str().unwrap()), "{said}");

    let foot: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.orthophoto.footprint.json")).unwrap();
    let c = case(&foot, "NoData'dan ayak izi");
    let drawing = opened(&mut server, &c["setup"], "foot");
    let mut args = c["steps"][0]["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "cad.orthophoto.footprint", args);
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"], &c["steps"][0]["result"]["output"]);
    close(&got["area"], &want["area"]);
    assert_eq!(got["parts"].as_array().unwrap().len(), want["parts"].as_array().unwrap().len());
    assert_eq!(got["valid"], want["valid"]);
}

//! EPSG's area of use polygons and the project's refraction coefficient through a real AI host (plan/map-cad.md MAP-02,
//! docs/adr/4900): a TUREF / TM30 drawing opened over MCP. A point in the Sea of Marmara (in TM30's box, out of its
//! polygon) is noted out of the source's area, one on Kütahya's plain is not; `cad.settings.set` writes a measured k
//! and refuses one outside [−1, 1].
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"crs-area-polygons-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_marmara_point_and_a_measured_k_over_mcp() {
    let mut server = Server::new();
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/drawing.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    // TM30 coordinates of 39.5°N 29.5°E (Kütahya's plain) and 40.75°N 28.9°E (the Sea of Marmara), as the shared case.
    let r = call(
        &mut server,
        "cad.crs.transform",
        json!({"drawing": drawing, "from": {"srid": 5254}, "to": {"srid": 4326},
               "points": [{"x": 420496.9775717948, "y": 4450222.180035507}, {"x": 407101.63387104846, "y": 4513392.561839832}]}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let flagged = &r["structuredContent"]["output"]["flagged"];
    assert_eq!(flagged, &json!([{"index": 1, "notes": ["outside_source_area"]}]), "{r}");

    let r = call(&mut server, "cad.settings.set", json!({"drawing": drawing, "refraction": 0.1625}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["output"]["changed"], json!(["refraction"]), "{r}");
    let r = call(&mut server, "cad.settings.set", json!({"drawing": drawing, "refraction": 1.5}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("invalid_refraction"), "{r}");
}

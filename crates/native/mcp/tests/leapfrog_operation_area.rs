//! Leap-frog trigonometric levelling and the datum transformation's area polygon through a real AI host
//! (plan/map-cad.md MAP-02, docs/adr/5300): a drawing opened over MCP. A leap-frog setup gives N1 R1's height plus its
//! difference; geometric levelling refuses it. Burgas written in WGS 84 / UTM 35N is in the datum operations' box but
//! out of extent 1237's polygon: noted out of the operation's area, the record naming the extent.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"leapfrog-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_leapfrog_setup_and_burgas_over_mcp() {
    let mut server = Server::new();
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/drawing.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let setup = json!({"from": "R1", "to": "N1",
        "back": {"distance": 62.415, "zenith": 101.2345, "target": 1.5},
        "fore": {"distance": 58.902, "zenith": 97.1234, "target": 1.5}});
    let work = |method: &str| json!({"kind": "level", "method": method, "known": [{"name": "R1", "h": 100.0}], "rows": [], "leapfrog": [setup]});
    let r = call(&mut server, "cad.survey.adjust", json!({"drawing": drawing, "work": work("trigonometric"), "unit": "grad"}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let dh = out["leapfrog"][0]["dh"].as_f64().expect("a difference");
    let n1 = out["points"].as_array().unwrap().iter().find(|p| p["name"] == "N1").expect("N1")["h"].as_f64().unwrap();
    assert!((n1 - (100.0 + dh)).abs() < 1e-9, "{out}");
    let r = call(&mut server, "cad.survey.adjust", json!({"drawing": drawing, "work": work("geometric")}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("work.method"), "{r}");

    let r = call(
        &mut server,
        "cad.crs.transform",
        json!({"drawing": drawing, "from": {"srid": 32635}, "to": {"srid": 5252},
               "points": [{"x": 538618.1207398332, "y": 4705400.089902891}]}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["operation"]["area"]["extent"], 1237, "{out}");
    assert!(out["flagged"][0]["notes"].as_array().unwrap().contains(&json!("outside_operation_area")), "{out}");
}

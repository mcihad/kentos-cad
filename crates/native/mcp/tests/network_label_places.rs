//! An accepted GNSS network's ellipse labels kept clear through a real AI host (plan/map-cad.md MAP-05, docs/adr/4160):
//! the shared cases' network with N6 15 m east of N1 (fixtures/commands/v1/map.survey.edit.json) opened as a .kcad file
//! over MCP. Accepted with labels, the answer says one label moved and none crowded; N1's label stands at the top right
//! of its ellipse (its bottom left (a + h)/√2 up and right of the point), the others east.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"label-places-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_crowded_label_moves_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.edit.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Ağı kabul et, etiketler: N1")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-label-places-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing.clone();
    let r = call(&mut server, "map.survey.edit", input);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["labels"].as_u64(), out["labelsMoved"].as_u64()), (Some(4), Some(1)), "{out}");
    assert!(out.get("labelsCrowded").is_none(), "{out}");
    // The labels and ellipses as written, by their marks.
    let fields = case["steps"][0]["expect"]["fields"].as_object().unwrap();
    let ellipse = fields.values().find(|v| v.get("major").is_some() && v["attrs"]["İşaret"] == "N1").unwrap();
    let label = fields.values().find(|v| v.get("text").is_some() && v["attrs"]["İşaret"] == "N1").unwrap();
    assert_eq!(label["align"], "bottomLeft");
    let a = ellipse["major"]["x"].as_f64().unwrap().hypot(ellipse["major"]["y"].as_f64().unwrap());
    let d = (a + label["height"].as_f64().unwrap()) / 2f64.sqrt();
    let near = |got: &Value, want: f64| assert!((got.as_f64().unwrap() - want).abs() < 1e-9, "{got} ≠ {want}");
    near(&label["p"]["x"], ellipse["c"]["x"].as_f64().unwrap() + d);
    near(&label["p"]["y"], ellipse["c"]["y"].as_f64().unwrap() + d);
    // The drawing holds that text, read back over MCP: N1's at the top right, the others east.
    let listed = call(&mut server, "drawing.entities", json!({"drawing": drawing, "layer": "gnss", "kinds": ["text"]}));
    assert_eq!(listed["isError"], false, "{listed}");
    let texts = listed["structuredContent"]["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(texts.len(), 4, "{listed}");
    let aligns: Vec<(&str, &str)> = texts
        .iter()
        .map(|t| (t["entity"]["attrs"]["İşaret"].as_str().unwrap_or(""), t["entity"]["align"].as_str().unwrap_or("")))
        .collect();
    assert_eq!(aligns, [("N1", "bottomLeft"), ("N4", "middleLeft"), ("N5", "middleLeft"), ("N6", "middleLeft")]);
}

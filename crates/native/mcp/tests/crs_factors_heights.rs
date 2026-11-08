//! The scale factors at each point's own height and along lines through a real AI host (plan/map-cad.md MAP-02,
//! docs/adr/2900): the shared cases' TM36 project, its heights orthometric with the TG-20 geoid, opened as a .kcad file
//! over MCP. Before the geoid's grid is on the device `cad.crs.factors` with the drawing's heights is refused
//! (`geoid_missing`, never a zero N); with it the answer is the shared case's: each point's ellipsoidal height and N,
//! each line's scale, height factor and height.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"crs-factors-heights-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn near(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{at}: {a} ≠ {b}");
        }
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                if k != "revision" {
                    near(&a[k], v, &format!("{at}.{k}"));
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                near(x, y, &format!("{at}[{i}]"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

#[test]
fn the_drawing_s_heights_go_through_the_geoid_over_mcp() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let cases: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("fixtures/commands/v1/cad.crs.factors.json")).unwrap())
            .unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"].as_str().unwrap().starts_with("Ortometrik kotlar ve TG-20 geoidi"))
        .unwrap()
        .clone();
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(case["setup"].clone()).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-crs-factors-heights-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("harita.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let mut input = case["steps"][0]["input"].clone();
    input["drawing"] = drawing;
    // Not on this device yet: refused, the model's name said.
    let r = call(&mut server, "cad.crs.factors", input.clone());
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("geoid_missing") && r.to_string().contains("TG-20 (sentetik)"), "{r}");
    // The grid in the device's library under its SHA-256, as Proje ayarları › Izgaralar puts it.
    let bytes = std::fs::read(root.join("fixtures/geoid/v1/tg.gtx")).unwrap();
    let id = kentos_native_application::crs_models::sha256(&bytes);
    std::fs::write(dir.join(format!("{id}.gtx")), &bytes).unwrap();
    kentos_native_application::crs_models::load(&id, kentos_native_application::crs_models::ModelKind::Geoid, Some(&dir))
        .unwrap();
    let r = call(&mut server, "cad.crs.factors", input);
    assert_eq!(r["isError"], false, "{r}");
    let want = &case["steps"][0]["result"]["output"];
    near(&r["structuredContent"]["output"], want, "output");
    assert_eq!(want["lines"].as_array().map(Vec::len), Some(2));
    assert!(want["points"][0]["undulation"].as_f64().is_some());
    let _ = std::fs::remove_dir_all(dir);
}

//! Yeniden izdüşür's preview through a real AI host (plan/map-cad.md MAP-02, docs/adr/2500): the drawing of
//! `cad.layers.reproject`'s shared cases opened as a .kcad file over MCP. The plan of the layer with a point far north
//! of Türkiye gives the case's shifts (that point first, with its three notes), the shifts summed up, the chain's
//! verdict and the warnings, and writes nothing; executed, it moves the objects as the plan said. The layer of a point,
//! a text and a circle read as WGS 84 is refused with each reason counted, as the case says.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"layer-reproject-preview-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// The same JSON, numbers within a millimetre (the cases' `within`), persistent ids and `$any` taking anything.
fn close(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (_, Value::String(s)) if s == "$any" || s.starts_with("$uidOf:") || s == "$current" => {}
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!((a - b).abs() <= 1e-3, "{at}: {a} ≠ {b}");
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                close(x, y, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                close(a.get(k).unwrap_or(&Value::Null), b.get(k).unwrap_or(&Value::Null), &format!("{at}.{k}"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

#[test]
fn the_preview_over_mcp_is_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.layers.reproject.json")).unwrap();
    let case = |start: &str| {
        cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with(start)).unwrap().clone()
    };
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-layer-reproject-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    // The plan: the case's preview, nothing written.
    let c = case("önizleme: kullanım alanlarının dışındaki nokta");
    let want = &c["steps"][0]["result"];
    let plan = call(&mut server, "cad.layers.reproject", json!({"drawing": drawing, "layer": "uzak", "from": 2322, "op": "plan"}));
    assert_eq!(plan["isError"], false, "{plan}");
    let got = &plan["structuredContent"];
    close(&got["output"], &want["output"], "plan");
    let codes = |w: &Value| w.as_array().unwrap().iter().map(|w| w["code"].clone()).collect::<Vec<_>>();
    assert_eq!(codes(&got["warnings"]), codes(&want["warnings"]));
    let far = &got["output"]["shifts"][0];
    assert_eq!((far["before"]["y"].as_f64(), far["notes"].as_array().map(Vec::len)), (Some(4_900_000.0), Some(3)));

    // Executed: the move the plan said.
    let done = call(&mut server, "cad.layers.reproject", json!({"drawing": drawing, "layer": "uzak", "from": 2322}));
    assert_eq!(done["isError"], false, "{done}");
    close(&done["structuredContent"]["output"], &c["steps"][1]["result"]["output"], "execute");

    // Refused, each reason counted.
    let r = case("ret: nedenler karışıkken");
    let refused = call(&mut server, "cad.layers.reproject", json!({"drawing": drawing, "layer": "yazilar", "from": 4326}));
    let error = &refused["structuredContent"]["error"];
    let want = &r["steps"][0]["result"]["error"];
    assert_eq!((&error["code"], &error["message"]), (&want["code"], &want["message"]), "{refused}");
    assert_eq!(error["uids"].as_array().map(Vec::len), Some(3));
    let _ = std::fs::remove_file(path);
}

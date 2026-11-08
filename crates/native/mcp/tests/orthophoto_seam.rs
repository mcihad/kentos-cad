//! The seam measured in the images through a real AI host (plan/map-cad.md MAP-12, docs/adr/2460): the shared seam
//! case's drawing (two synthetic GeoTIFFs of the fixtures, their paths made absolute) opened as a .kcad file over MCP.
//! `cad.orthophoto.compare` with `seam` gives the case's places, counts and shift; a narrow search leaves every peak
//! on its edge.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"orthophoto-seam-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

/// A text with its `$fixture:` paths made absolute, as the shared runners make them.
fn fixtures(v: &Value) -> Value {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/");
    serde_json::from_str(&v.to_string().replace("$fixture:", &root.display().to_string())).unwrap()
}

/// Two places' lists alike: states and correlations, shifts within 10⁻⁹.
fn alike(got: &Value, want: &Value, name: &str) {
    let (g, w) = (got["patches"].as_array().unwrap(), want["patches"].as_array().unwrap());
    assert_eq!(g.len(), w.len(), "{name}");
    for (a, b) in g.iter().zip(w) {
        assert_eq!(a["state"], b["state"], "{name}");
        for key in ["dx", "dy", "correlation"] {
            match (a[key].as_f64(), b[key].as_f64()) {
                (Some(x), Some(y)) => assert!((x - y).abs() < 1e-9, "{name} {key}: {x} {y}"),
                (x, y) => assert_eq!(x.is_some(), y.is_some(), "{name} {key}"),
            }
        }
    }
    for key in ["ok", "low", "edge", "flat", "passed"] {
        assert_eq!(got[key], want[key], "{name} {key}");
    }
}

#[test]
fn the_seam_over_mcp_is_the_cases() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.orthophoto.compare.json")).unwrap();
    for name in ["dikiş: örtüşmede ölçülen kayma bilinen sentetik kaymadır; dokusuz ve değişen yerler ayrı", "dikiş: arama dar, tepe sınırda kalır"] {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(fixtures(&case["setup"])).unwrap();
        let path = std::env::temp_dir().join(format!("kentos-orthophoto-seam-{}.kcad", std::process::id()));
        std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
        let mut server = Server::new();
        let opened = call(&mut server, "drawing.open", json!({"path": path}));
        assert_eq!(opened["isError"], false, "{opened}");
        let step = &case["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = opened["structuredContent"]["drawing"].clone();
        let r = call(&mut server, "cad.orthophoto.compare", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        alike(&r["structuredContent"]["output"]["seam"], &step["result"]["output"]["seam"], name);
        let _ = std::fs::remove_file(path);
    }
}

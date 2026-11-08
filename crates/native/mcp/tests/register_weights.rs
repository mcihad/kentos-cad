//! Robust weights of the scan registration through a real AI host (plan/map-cad.md MAP-13, docs/adr/2300): the shared
//! cases' drawing (its clouds' files from the fixtures) opened as a .kcad file over MCP. `cad.pointclouds.register` of
//! the scan with the raised patch gives each case's transform and weights' report, plain, with Huber's weights and
//! trimmed; a trimmed fit without its share is refused with the case's words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"register-weights-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASES: [&str; 3] = [
    "kaba hatalı eşleşmeler: 0,5 m yüksekteki yama düz ICP'yi aşağı çeker",
    "Huber ağırlıkları (k 1,345) kaba hatayı bastırır",
    "kırpılmış ICP: eşleşmelerin en iyi %90'ı",
];

#[test]
fn the_weighted_fits_over_mcp_are_the_cases() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pointclouds.register.json")).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    let text = cases["setup"].to_string().replace("$fixture:", &format!("{}/", root.display()));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(&text).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-register-weights-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let find = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
    for name in CASES {
        let step = &find(name)["steps"][0];
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        let r = call(&mut server, "cad.pointclouds.register", args);
        assert_eq!(r["isError"], false, "{r}");
        let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
        for i in 0..3 {
            let (a, b) = (got["translation"][i].as_f64().unwrap(), want["translation"][i].as_f64().unwrap());
            assert!((a - b).abs() < 1e-7, "{name} {i}: {a} != {b}");
        }
        assert_eq!(got["robust"]["lighter"], want["robust"]["lighter"], "{name}");
        assert_eq!(got["robust"]["kind"], want["robust"]["kind"], "{name}");
    }
    let refusal = &find("kırpılmış ICP payı verilmemiş")["steps"][0];
    let mut args = refusal["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "cad.pointclouds.register", args);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("robust.ratio"), "{r}");
    let _ = std::fs::remove_file(path);
}

//! Rejections of the scan registration through a real AI host (plan/map-cad.md MAP-13, docs/adr/2700): the shared cases'
//! drawing (its clouds' files from the fixtures) opened as a .kcad file over MCP. `cad.pointclouds.register` of the wall
//! only the second scan saw and of the curb's shadow gives each case's transform and rejections' counts, plainly, with
//! the normals' angle and with the edges left out; a rejection asking nothing is refused with the case's words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"register-rejections-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const CASES: [&str; 4] = [
    "yalnız ikinci taramada duvar: ayağı zeminle eşleşir, düz ICP kayar",
    "normal açısı 30°: duvarın eşleşmeleri ayıklanır",
    "bordürün arkası sabit taramada gölge: kaldırım yolun kenarıyla eşleşir, düz ICP kayar",
    "kenardaki eşleşmeler ayıklanınca bordür kaydırmaz",
];

#[test]
fn the_rejecting_fits_over_mcp_are_the_cases() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pointclouds.register.json")).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    let text = cases["setup"].to_string().replace("$fixture:", &format!("{}/", root.display()));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(&text).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-register-rejections-{}.kcad", std::process::id()));
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
        assert_eq!((&got["pairs"], &got["rejected"]), (&want["pairs"], &want["rejected"]), "{name}");
    }
    let refusal = &find("ayıklama bir şey istemiyor")["steps"][0];
    let mut args = refusal["input"].clone();
    args["drawing"] = drawing;
    let r = call(&mut server, "cad.pointclouds.register", args);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("Ayıklama için kenardaki eşleşmeleri"), "{r}");
    let _ = std::fs::remove_file(path);
}

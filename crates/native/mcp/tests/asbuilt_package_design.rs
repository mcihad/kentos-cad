//! The returned record checked with its package's own design through a real AI host (plan/map-cad.md MAP-19,
//! docs/adr/2820): the shared cases' drawing of `cad.asbuilt.check` opened as a .kcad file over MCP. With the older
//! design's package and `packageDesign`, the answer is the shared case's (the package's digest, the drawing's change
//! said, P5 and P6 not measured); the package whose design files are not its digest's is refused at `package`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"asbuilt-package-design-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

#[test]
fn the_record_with_its_package_s_own_design_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-asbuilt-package-design-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "paketin tasarımı: eski tasarım; kayıt sahaya verilenle, çizimdeki tasarım değişti")
        .unwrap()["steps"][0]
        .clone();
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    args["package"] = json!(fixture("asbuilt/v1/files/eski-tasarim-paketi.zip"));
    let r = call(&mut server, "cad.asbuilt.check", args.clone());
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
    for key in ["designDigest", "designChanged", "within", "outside", "unmatched", "ambiguous", "missing"] {
        assert_eq!(got[key], want[key], "{key}");
    }
    assert_eq!(got["package"]["design"], true);
    args["package"] = json!(fixture("asbuilt/v1/files/bozuk-tasarim-paketi.zip"));
    let r = call(&mut server, "cad.asbuilt.check", args);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("özetiyle tutmuyor"), "{r}");
    let _ = std::fs::remove_file(path);
}

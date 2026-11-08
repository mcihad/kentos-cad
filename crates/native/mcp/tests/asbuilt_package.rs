//! The returned record checked with its field package through a real AI host (plan/map-cad.md MAP-19, docs/adr/2420):
//! the shared cases' drawing of `cad.asbuilt.check` opened as a .kcad file over MCP, the fixtures' packages given by
//! their paths. The current package gives the digest and the limits the input leaves empty, as the shared case says;
//! the older one says the design changed and its other system, epoch and heights; a zip that is not a package is
//! refused at `package`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"asbuilt-package-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

#[test]
fn the_record_s_package_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-asbuilt-package-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let case = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap()["steps"][0].clone();
    for (name, file) in [
        ("paket: tasarım özeti ve kabul sınırları paketten", "asbuilt/v1/files/saha-paketi.zip"),
        ("paket: eski tasarımın paketi, başka sistem, epok ve kot", "asbuilt/v1/files/eski-paket.zip"),
    ] {
        let step = case(name);
        let mut args = step["input"].clone();
        args["drawing"] = drawing.clone();
        args["package"] = json!(fixture(file));
        let r = call(&mut server, "cad.asbuilt.check", args);
        assert_eq!(r["isError"], false, "{name}: {r}");
        let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
        for key in ["within", "outside", "unmatched", "ambiguous", "lowQuality", "designChanged", "designDigest"] {
            assert_eq!(got[key], want[key], "{name} {key}");
        }
        for key in ["taken", "sameSystem", "sameEpoch", "sameHeights", "digest", "stamp", "revision", "epsg"] {
            assert_eq!(got["package"][key], want["package"][key], "{name} package.{key}");
        }
    }
    let mut args = case("paket: tasarım özeti ve kabul sınırları paketten")["input"].clone();
    args["drawing"] = drawing;
    args["package"] = json!(fixture("asbuilt/v1/files/paket-degil.zip"));
    let r = call(&mut server, "cad.asbuilt.check", args);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("invalid_asbuilt_check") && r.to_string().contains("saha-paketi.json"), "{r}");
    let _ = std::fs::remove_file(path);
}

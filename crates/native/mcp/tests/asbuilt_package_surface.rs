//! The field package's own surface through a real AI host (plan/map-cad.md MAP-19, docs/adr/3220): the shared cases'
//! drawing of `cad.asbuilt.check` opened as a .kcad file over MCP. `cad.field.package` writes the design surface into
//! the package as `yuzey.xml`; the record checked with the package's design reads it back (the answer says the surface
//! was the package's, and with the drawing's own surface nothing changed). The fixture package that carries the plane
//! 5 cm lower over the south half gives the shared case's surface differences and off-surface points, and says the
//! drawing's surface changed since; its copy with a changed `yuzey.xml` is refused at `package`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"asbuilt-package-surface-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

#[test]
fn the_package_s_surface_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.asbuilt.check.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-asbuilt-surface-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("uygulama.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    // The package written here carries the drawing's surface.
    let zip = dir.join("saha.zip");
    let written = call(
        &mut server,
        "cad.field.package",
        json!({"drawing": drawing, "design": "tasarim", "lines": "bordur", "surface": "Tasarım", "vertical": 0.03,
               "path": zip, "stamp": "2026-11-09T08:00:00"}),
    );
    assert_eq!(written["isError"], false, "{written}");
    let surface = &written["structuredContent"]["output"]["surface"];
    assert_eq!((surface["file"].as_str(), surface["vertices"].as_u64(), surface["triangles"].as_u64()), (Some("yuzey.xml"), Some(121), Some(200)));
    let base = json!({"drawing": drawing, "measured": "olcum", "surface": "Tasarım", "search": 0.5,
                      "qualityField": "Hassasiyet", "packageDesign": true});
    let mut args = base.clone();
    args["package"] = json!(zip);
    let r = call(&mut server, "cad.asbuilt.check", args);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["package"]["surface"], true, "{out}");
    assert!(out.get("surfaceChanged").is_none(), "the drawing's own surface: {out}");

    // The fixture package's surface: the shared case's differences.
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "paketin yüzeyi: çizimdeki yüzey paketinkinden sonra değişmiş").unwrap();
    let want = &case["steps"][0]["result"]["output"];
    let mut args = case["steps"][0]["input"].clone();
    args["drawing"] = drawing.clone();
    args["package"] = json!(fixture("asbuilt/v1/files/yuzeyli-paket.zip"));
    let r = call(&mut server, "cad.asbuilt.check", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"];
    assert_eq!(got["surfaceChanged"], true);
    for key in ["within", "outside", "unmatched", "ambiguous"] {
        assert_eq!(got[key], want[key], "{key}");
    }
    for (g, w) in got["points"].as_array().unwrap().iter().zip(want["points"].as_array().unwrap()) {
        assert_eq!(g["flags"], w["flags"], "{}", w["name"]);
        match (g["surface"].as_f64(), w["surface"].as_f64()) {
            (Some(a), Some(b)) => assert!((a - b).abs() < 1e-6, "{}: {a} {b}", w["name"]),
            (a, b) => assert_eq!(a, b, "{}", w["name"]),
        }
    }

    // A yuzey.xml changed by hand.
    let mut args = base;
    args["package"] = json!(fixture("asbuilt/v1/files/bozuk-yuzey-paketi.zip"));
    let r = call(&mut server, "cad.asbuilt.check", args);
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("yuzey.xml") && r.to_string().contains("özetiyle tutmuyor"), "{r}");
    let _ = std::fs::remove_dir_all(dir);
}

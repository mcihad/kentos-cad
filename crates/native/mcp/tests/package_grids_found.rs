//! A project opened from its delivery package finds its grids there, through a real AI host (plan/map-cad.md MAP-17,
//! docs/adr/3580). The scale factors' TM36 project, its heights orthometric on a geoid model of its own (TG-20 with its
//! last node moved a millimetre, so no other run holds it) and a datum choice shifting by the Türkiye NTv2 grid, is
//! packed over MCP with both grids in the device's library. With the library empty: the project file alone names
//! both missing and `cad.crs.factors` is refused (`geoid_missing`); the package's file finds both in `izgaralar/`
//! (its SHA-256 checked) and the command answers, the library left unwritten.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"package-grids-found-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn hex(bytes: &[u8]) -> String {
    kentos_commands::encoding::sha256_hex(bytes)
}

/// The places opening found the drawing's grids, by kind.
fn places(opened: &Value) -> Vec<(String, String)> {
    opened["structuredContent"]["summary"]["grids"]
        .as_array()
        .map(|a| a.iter().map(|g| (g["kind"].as_str().unwrap().to_owned(), g["place"].as_str().unwrap().to_owned())).collect())
        .unwrap_or_default()
}

#[test]
fn the_package_s_grids_are_found_when_it_is_opened_over_mcp() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = std::env::temp_dir().join(format!("kentos-package-grids-found-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (library, empty) = (dir.join("izgara"), dir.join("bos"));
    std::fs::create_dir_all(&library).unwrap();
    std::fs::create_dir_all(&empty).unwrap();
    let mut geoid_bytes = std::fs::read(root.join("fixtures/geoid/v1/tg.gtx")).unwrap();
    let n = geoid_bytes.len();
    let last = f32::from_be_bytes(geoid_bytes[n - 4..].try_into().unwrap()) + 0.001;
    geoid_bytes[n - 4..].copy_from_slice(&last.to_be_bytes());
    let ntv2_bytes = std::fs::read(root.join("fixtures/geodesy/v1/ntv2/tr.gsb")).unwrap();
    let (geoid, ntv2) = (hex(&geoid_bytes), hex(&ntv2_bytes));
    std::fs::write(library.join(format!("{geoid}.gtx")), &geoid_bytes).unwrap();
    std::fs::write(library.join(format!("{ntv2}.gsb")), &ntv2_bytes).unwrap();
    // This binary's one test: the library is read where every host reads it, KENTOS_GRID_DIR.
    unsafe { std::env::set_var("KENTOS_GRID_DIR", &library) };
    let cases: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("fixtures/commands/v1/cad.crs.factors.json")).unwrap()).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Ortometrik kotlar ve TG-20 geoidi")).unwrap().clone();
    let mut v1 = case["setup"].clone();
    v1["settings"]["heights"]["geoid"] = json!({"name": "TG-20 (paket)", "sha256": geoid});
    v1["settings"]["datumTransforms"] =
        json!([{ "from": "ED50", "to": "TUREF", "name": "Bölge 7", "grid": { "id": ntv2, "file": "tr.gsb", "size": ntv2_bytes.len() } }]);
    let v1: kentos_commands::DocumentSnapshotV1 = serde_json::from_value(v1).unwrap();
    let snapshot = kentos_commands::migrate_v1(v1).unwrap();
    let path = dir.join("harita.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    assert_eq!(places(&opened), [("ntv2".into(), "library".into()), ("geoid".into(), "library".into())]);
    let out = dir.join("teslim");
    let r = call(&mut server, "file.package", json!({"drawing": opened["structuredContent"]["drawing"], "path": out, "name": "harita"}));
    assert_eq!(r["isError"], false, "{r}");
    // Another machine: its library holds neither.
    unsafe { std::env::set_var("KENTOS_GRID_DIR", &empty) };
    let mut input = case["steps"][0]["input"].clone();
    // The project file alone: both missing, the command refused.
    let alone = dir.join("tek");
    std::fs::create_dir_all(&alone).unwrap();
    std::fs::copy(out.join("harita.kcad"), alone.join("harita.kcad")).unwrap();
    let opened = call(&mut server, "drawing.open", json!({"path": alone.join("harita.kcad")}));
    assert_eq!(places(&opened), [("ntv2".into(), "missing".into()), ("geoid".into(), "missing".into())]);
    input["drawing"] = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "cad.crs.factors", input.clone());
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("geoid_missing"), "{r}");
    // The package's file: both found beside it, the command answers.
    let opened = call(&mut server, "drawing.open", json!({"path": out.join("harita.kcad")}));
    assert_eq!(opened["isError"], false, "{opened}");
    assert_eq!(places(&opened), [("ntv2".into(), "package".into()), ("geoid".into(), "package".into())]);
    input["drawing"] = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "cad.crs.factors", input);
    assert_eq!(r["isError"], false, "{r}");
    assert!(r["structuredContent"]["output"]["points"][0]["undulation"].as_f64().is_some(), "{r}");
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0, "the library is not written");
    let _ = std::fs::remove_dir_all(dir);
}

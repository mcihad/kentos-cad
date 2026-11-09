//! The delivery package's grids through a real AI host (plan/map-cad.md MAP-17, docs/adr/3180): the open-files sample
//! (fixtures/exchange/v1/devir.json) with its heights resting on a geoid model and a datum choice shifting by an NTv2
//! grid, opened as a .kcad file over MCP and packed with the device's grid library in a scratch folder. The package
//! holds both grids under their SHA-256 in `izgaralar/`, as the answer and `paket.json` list them; a velocity grid the
//! library lacks is left out at its campaign's setting.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"package-grids-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn hex(bytes: &[u8]) -> String {
    kentos_commands::encoding::sha256_hex(bytes)
}

#[test]
fn the_package_carries_the_grids_the_project_names_over_mcp() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = std::env::temp_dir().join(format!("kentos-package-grids-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let library = dir.join("izgara");
    std::fs::create_dir_all(&library).unwrap();
    let geoid_bytes = std::fs::read(root.join("fixtures/geoid/v1/tg.gtx")).unwrap();
    let ntv2_bytes = std::fs::read(root.join("fixtures/geodesy/v1/ntv2/tr.gsb")).unwrap();
    let (geoid, ntv2) = (hex(&geoid_bytes), hex(&ntv2_bytes));
    std::fs::write(library.join(format!("{geoid}.gtx")), &geoid_bytes).unwrap();
    std::fs::write(library.join(format!("{ntv2}.gsb")), &ntv2_bytes).unwrap();
    // This binary's one test: the library is read where every host reads it, KENTOS_GRID_DIR.
    unsafe { std::env::set_var("KENTOS_GRID_DIR", &library) };
    let mut drawing: Value = serde_json::from_str(include_str!("../../../../fixtures/exchange/v1/devir.json")).unwrap();
    drawing["settings"]["heights"] = json!({ "kind": "orthometric", "verticalSrid": 5775, "geoid": { "name": "TG-20", "sha256": geoid } });
    drawing["settings"]["datumTransforms"] =
        json!([{ "from": "ED50", "to": "TUREF", "name": "Bölge 7", "grid": { "id": ntv2, "file": "tr.gsb", "size": ntv2_bytes.len() } }]);
    drawing["settings"]["mapSurvey"] =
        json!({ "campaigns": [{ "id": "kp-1", "name": "Bir", "velocity": { "name": "Hız ızgarası", "sha256": "a".repeat(64) } }] });
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(drawing).unwrap();
    let path = dir.join("ada.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let out = dir.join("teslim");
    let r = call(&mut server, "file.package", json!({"drawing": opened["structuredContent"]["drawing"], "path": out, "name": "ada"}));
    assert_eq!(r["isError"], false, "{r}");
    let files = r["structuredContent"]["output"]["files"].as_array().unwrap().clone();
    let grids: Vec<(String, String)> = files
        .iter()
        .filter(|f| f["kind"] == "grid")
        .map(|f| (f["path"].as_str().unwrap().to_owned(), f["sha256"].as_str().unwrap().to_owned()))
        .collect();
    assert_eq!(grids, [(format!("izgaralar/{ntv2}.gsb"), ntv2.clone()), (format!("izgaralar/{geoid}.gtx"), geoid.clone())]);
    for (file, digest) in &grids {
        assert_eq!(&hex(&std::fs::read(out.join(file)).unwrap()), digest, "{file}");
    }
    let left = &r["structuredContent"]["output"]["left"];
    assert!(left.to_string().contains("mapSurvey.campaigns[0].velocity"), "{left}");
    let manifest: Value = serde_json::from_slice(&std::fs::read(out.join("paket.json")).unwrap()).unwrap();
    assert_eq!(manifest["files"].as_array().unwrap().iter().filter(|f| f["kind"] == "grid").count(), 2);
    let _ = std::fs::remove_dir_all(dir);
}

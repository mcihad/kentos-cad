//! The delivery package's open files through a real AI host (plan/map-cad.md MAP-17, docs/adr/2780): the blocks sample
//! (fixtures/exchange/v1/devir-blok.json) opened as a .kcad file over MCP and packed with GeoPackage and DXF. The
//! package holds both files and the report under `devir/`; its matrix keeps the block and the table in DXF alone and the
//! relation in GeoPackage alone (ADR 2380's table), as `paket.json` says too.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"package-exchange-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_package_carries_the_open_files_and_their_matrix_over_mcp() {
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../fixtures/exchange/v1/devir-blok.json")).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-package-exchange-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("blok.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let out = dir.join("teslim");
    let r = call(
        &mut server,
        "file.package",
        json!({"drawing": drawing, "path": out, "name": "blok", "exchange": {"formats": ["gpkg", "dxf"]}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    for file in ["devir/blok.gpkg", "devir/blok.dxf", "devir/devir-raporu.csv"] {
        assert!(out.join(file).is_file(), "{file}");
    }
    let rows = r["structuredContent"]["output"]["exchange"]["rows"].as_array().unwrap().clone();
    let state = |aspect: &str, i: usize| rows.iter().find(|x| x["aspect"] == aspect).unwrap()["cells"][i]["state"].clone();
    assert_eq!((state("blocks", 0), state("blocks", 1)), (json!("lost"), json!("kept")));
    assert_eq!((state("tables", 0), state("tables", 1)), (json!("lost"), json!("kept")));
    assert_eq!((state("relations", 0), state("relations", 1)), (json!("kept"), json!("lost")));
    let manifest: Value = serde_json::from_slice(&std::fs::read(out.join("paket.json")).unwrap()).unwrap();
    assert_eq!(manifest["exchange"]["rows"], json!(rows));
    let _ = std::fs::remove_dir_all(dir);
}

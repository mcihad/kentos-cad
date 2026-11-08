//! The blocks, the tables and the project's relations of the exchange round trip through a real AI host
//! (plan/map-cad.md MAP-17, docs/adr/2380): fixtures/exchange/v1/devir-blok.json opened as a .kcad file over MCP.
//! `file.exchange.matrix` keeps the block and the table in DXF alone and the relation in GeoPackage alone;
//! `file.exchange.check` on DXF names the relation lost.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"exchange-parts-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn blocks_tables_and_relations_over_mcp() {
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../fixtures/exchange/v1/devir-blok.json")).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-exchange-parts-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "file.exchange.matrix", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    let rows = r["structuredContent"]["output"]["rows"].as_array().unwrap().clone();
    let state = |aspect: &str, format: &str| -> String {
        let row = rows.iter().find(|x| x["aspect"] == aspect).unwrap();
        row["cells"].as_array().unwrap().iter().find(|c| c["format"] == format).unwrap()["state"].as_str().unwrap().to_owned()
    };
    for f in ["gpkg", "parquet", "fgb", "gml", "shp", "dxf"] {
        assert_eq!(state("blocks", f), if f == "dxf" { "kept" } else { "lost" }, "{f}");
        assert_eq!(state("tables", f), if f == "dxf" { "kept" } else { "lost" }, "{f}");
        assert_eq!(state("relations", f), if f == "gpkg" { "kept" } else { "lost" }, "{f}");
    }
    let c = call(&mut server, "file.exchange.check", json!({"drawing": drawing, "format": "dxf"}));
    assert_eq!(c["isError"], false, "{c}");
    let losses = c["structuredContent"]["output"]["losses"].as_array().unwrap().clone();
    let lost = losses.iter().find(|l| l["kind"] == "relationLost").unwrap();
    assert_eq!(lost["detail"], "Parselin malikleri");
    assert!(losses.iter().all(|l| l["kind"] != "blockChanged" && l["kind"] != "tableChanged"), "{losses:?}");
    let _ = std::fs::remove_file(path);
}

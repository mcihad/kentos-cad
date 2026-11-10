//! İmar denetiminin sembol uyuşmazlığı and its error layer written to a GeoPackage through a real AI host (plan/map-cad.md
//! MAP-09, docs/adr/4640). `cad.zoning.check` on the shared case's drawing (a trade area drawn with the park's symbol)
//! finds that one area; `cad.processing.run` of İmar planını denetle with Bulguları hata katmanına yaz and a sink writes
//! the seven regions to a GeoPackage table and leaves the drawing as it was.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"zoning-symbol-gpkg-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn open(server: &mut Server, snapshot: &kentos_commands::DocumentSnapshotV2, name: &str) -> Value {
    let path = std::env::temp_dir().join(format!("kentos-{name}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(opened["isError"], false, "{opened}");
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn the_symbol_against_the_decision_and_the_error_layer_to_a_file_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.zoning.check.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("sembol kararı göstermiyor")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let mut server = Server::new();
    let drawing = open(&mut server, &snapshot, "zoning-symbol");
    let r = call(&mut server, "cad.zoning.check", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    let found: Vec<&Value> =
        r["structuredContent"]["output"]["findings"].as_array().unwrap().iter().filter(|f| f["kind"] == "symbolMismatch").collect();
    assert_eq!(found.len(), 1, "{r}");
    assert_eq!(found[0]["uids"], json!([snapshot.uids[1].to_string()]), "S1, the trade area");

    // The error layer to a GeoPackage: the drawing stays.
    let roads: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(include_str!("../../../../fixtures/processing/v1/zoning-yollar.kcad")).unwrap();
    let drawing = open(&mut server, &roads, "zoning-gpkg");
    let gpkg = std::env::temp_dir().join(format!("kentos-zoning-errors-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&gpkg);
    let ran = call(
        &mut server,
        "cad.processing.run",
        json!({"drawing": drawing, "tool": "zoning.check", "parameters": {"write": true, "layer": {"newName": "İmar bulguları"}},
               "sink": {"path": gpkg}}),
    );
    assert_eq!(ran["isError"], false, "{ran}");
    let sink = &ran["structuredContent"]["output"]["sink"];
    assert_eq!(sink["tables"][0]["table"], "İmar bulguları", "{sink}");
    assert_eq!((sink["tables"][0]["rows"].as_u64(), sink["tables"][0]["geometry"].as_str()), (Some(7), Some("POLYGON")), "{sink}");
    assert!(gpkg.exists());
    // The seven areas were made on a copy and went to the file; the drawing has its own objects only.
    let items = call(&mut server, "drawing.entities", json!({"drawing": drawing, "limit": 1000}))["structuredContent"]["items"].clone();
    assert_eq!(items.as_array().map(Vec::len), Some(roads.entities.len()), "the drawing stays");
    let _ = std::fs::remove_file(&gpkg);
}

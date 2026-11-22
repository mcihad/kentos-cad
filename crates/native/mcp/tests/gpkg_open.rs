//! Real MCP host, a GeoPackage opened as a project (docs/adr/9116): an agent opens a file of no project, which becomes
//! a new project of its tables written into the file; drawing.save without a path writes there again; a file of
//! several projects is refused with their names until `project` names one. The files are fixtures/data/v1/gpkg-open's.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"gpkg-open-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}}))
        .unwrap()["result"]
        .clone()
}

fn copy(file: &str, tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-mcp-gpkg-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let to = dir.join(file);
    std::fs::copy(
        format!("{}/../../../fixtures/data/v1/gpkg-open/{file}", env!("CARGO_MANIFEST_DIR")),
        &to,
    )
    .expect("the copy");
    to
}

#[test]
fn an_agent_opens_a_geopackage_of_no_project_and_saves_into_it() {
    let path = copy("katmanlar.gpkg", "made");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let out = &opened["structuredContent"];
    let said = &out["summary"]["openedGeoPackage"];
    assert_eq!(said["made"], true, "{opened}");
    assert_eq!(said["written"], true, "{opened}");
    assert_eq!(said["project"], "katmanlar");
    assert_eq!(out["summary"]["project"]["name"], "katmanlar", "{opened}");
    assert_eq!(out["summary"]["settings"]["srid"], 5256);
    let drawing = out["drawing"].clone();

    // drawing.save without a path: into the file again.
    let saved = call(&mut server, "drawing.save", json!({"drawing": drawing}));
    assert_eq!(saved["isError"], false, "{saved}");
    assert_eq!(saved["structuredContent"]["name"], "katmanlar", "{saved}");

    // Opened again: the project the file keeps now.
    let again = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(again["isError"], false, "{again}");
    assert_eq!(again["structuredContent"]["summary"]["openedGeoPackage"]["made"], false, "{again}");
}

#[test]
fn a_file_of_several_projects_is_named_and_one_is_opened_by_project() {
    let path = copy("iki-projeli.gpkg", "several");
    let mut server = Server::new();
    let refused = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(refused["isError"], true, "{refused}");
    let words = refused.to_string();
    assert!(words.contains("several_projects") && words.contains("“Çamlık”, “Zeytinlik”"), "{refused}");
    let opened = call(&mut server, "drawing.open", json!({"path": path, "project": "Çamlık"}));
    assert_eq!(opened["isError"], false, "{opened}");
    assert_eq!(opened["structuredContent"]["summary"]["project"]["name"], "Çamlık", "{opened}");
}

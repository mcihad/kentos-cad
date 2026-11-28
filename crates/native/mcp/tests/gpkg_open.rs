//! Real MCP host, a GeoPackage opened as a project (docs/adr/9116): an agent opens a file of no project, which becomes
//! a new project of its tables made in memory, the file unchanged (docs/adr/9122); drawing.save without a path keeps
//! it in the file; a file of
//! several projects is refused with their names until `project` names one. Its tile tables open as a raster layer under
//! the tables, and drawing.save gives a changed look to its table's default style, once (docs/adr/9117). The files are
//! fixtures/data/v1/gpkg-open's.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    server
        .handle(
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"gpkg-open-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}}),
        )
        .unwrap()["result"]
        .clone()
}

fn copy(file: &str, tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-mcp-gpkg-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let to = dir.join(file);
    std::fs::copy(
        format!(
            "{}/../../../fixtures/data/v1/gpkg-open/{file}",
            env!("CARGO_MANIFEST_DIR")
        ),
        &to,
    )
    .expect("the copy");
    to
}

#[test]
fn an_agent_opens_a_geopackage_of_no_project_and_saves_into_it() {
    let path = copy("katmanlar.gpkg", "made");
    let before = std::fs::read(&path).expect("the file");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let out = &opened["structuredContent"];
    let said = &out["summary"]["openedGeoPackage"];
    assert_eq!(said["made"], true, "{opened}");
    assert_eq!(said["kept"], false, "{opened}");
    assert_eq!(out["summary"]["project"]["kept"], false, "{opened}");
    assert!(
        std::fs::read(&path).expect("the file") == before,
        "opening wrote nothing into the file"
    );
    assert_eq!(said["project"], "katmanlar");
    assert_eq!(out["summary"]["project"]["name"], "katmanlar", "{opened}");
    assert_eq!(out["summary"]["settings"]["srid"], 5256);
    let drawing = out["drawing"].clone();

    // drawing.save without a path: the project kept in the file now.
    let saved = call(&mut server, "drawing.save", json!({"drawing": drawing}));
    assert_eq!(saved["isError"], false, "{saved}");
    assert_eq!(saved["structuredContent"]["name"], "katmanlar", "{saved}");

    // Opened again: the project the file keeps now.
    let again = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(again["isError"], false, "{again}");
    assert_eq!(
        again["structuredContent"]["summary"]["openedGeoPackage"]["made"], false,
        "{again}"
    );
}

#[test]
fn a_file_of_several_projects_is_named_and_one_is_opened_by_project() {
    let path = copy("iki-projeli.gpkg", "several");
    let mut server = Server::new();
    let refused = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(refused["isError"], true, "{refused}");
    let words = refused.to_string();
    assert!(
        words.contains("several_projects") && words.contains("“Çamlık”, “Zeytinlik”"),
        "{refused}"
    );
    let opened = call(
        &mut server,
        "drawing.open",
        json!({"path": path, "project": "Çamlık"}),
    );
    assert_eq!(opened["isError"], false, "{opened}");
    assert_eq!(
        opened["structuredContent"]["summary"]["project"]["name"], "Çamlık",
        "{opened}"
    );
}

/// The drawing's layers by their names, in the tree's order.
fn layers(server: &mut Server, drawing: &Value) -> Vec<(String, String)> {
    let told = call(server, "drawing.layers", json!({"drawing": drawing}));
    told["structuredContent"]["layers"]
        .as_array()
        .expect("the layers")
        .iter()
        .map(|n| {
            (
                n["name"].as_str().unwrap_or("").to_owned(),
                n["id"].as_str().unwrap_or("").to_owned(),
            )
        })
        .collect()
}

#[test]
fn an_agent_sees_the_tiles_as_a_raster_and_its_save_writes_a_changed_look_once() {
    let mut server = Server::new();
    let path = copy("ortofotolu.gpkg", "tiles");
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let names: Vec<String> = layers(&mut server, &drawing)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(names, ["Taslak", "parseller", "ortofoto"]);

    let path = copy("katmanlar.gpkg", "looks");
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let binalar = layers(&mut server, &drawing)
        .into_iter()
        .find(|(n, _)| n == "binalar")
        .map(|(_, id)| id)
        .expect("binalar");
    let renderer = json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
        { "id": "f", "type": "simpleFill", "color": "#2a6fdb" },
        { "id": "o", "type": "simpleLine", "color": "#123456", "width": 0.5 }
    ] } } });
    let done = call(
        &mut server,
        "cad.layers.renderer",
        json!({"drawing": drawing, "layer": binalar, "renderer": renderer}),
    );
    assert_eq!(done["isError"], false, "{done}");
    let saved = call(&mut server, "drawing.save", json!({"drawing": drawing}));
    assert_eq!(
        saved["structuredContent"]["looks"],
        json!(["binalar"]),
        "{saved}"
    );
    let again = call(&mut server, "drawing.save", json!({"drawing": drawing}));
    assert_eq!(
        again["structuredContent"]["looks"],
        json!([]),
        "unchanged: {again}"
    );
}

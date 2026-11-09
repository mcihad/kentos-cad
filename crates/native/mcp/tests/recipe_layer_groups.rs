//! A recipe layer's groups, whole style and line type through a real AI host (plan/map-cad.md MAP-20, docs/adr/4040):
//! the shared cases' Yol project of `road.project.produce` opened as a .kcad file over MCP. The fixture recipe whose
//! layer “Kazı sınırı” is in the groups YOL › Toprak işleri with a style naming the line type SINIR adds the layer in
//! the project's group Yol (its name folded), in a Toprak işleri added there, with the style, and adds SINIR; the
//! graph's recipes then keep those groups, that style and SINIR's definition, so a recipe saved here carries them. A
//! recipe whose style's colour is not its definition's is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"recipe-layer-groups-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

fn opened(server: &mut Server, name: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.produce.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-recipe-groups-{name}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(path);
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn a_recipe_layer_s_groups_style_and_line_type_over_mcp() {
    let mut server = Server::new();
    let drawing = opened(&mut server, "grup");
    let recipe = fixture("recipes/v1/grup-stilli.json");
    let r = call(&mut server, "road.project.produce", json!({"drawing": drawing, "path": recipe}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let went = &out["layers"][0];
    assert_eq!(went["how"], "created", "{out}");
    assert_eq!(went["group"], json!(["Yol", "Toprak işleri"]));
    assert_eq!(went["groupsAdded"], json!(["Toprak işleri"]));
    assert_eq!(went["lineTypeAdded"], "SINIR");
    assert_eq!(out["produced"].as_array().unwrap().len(), 2);
    let said = r["structuredContent"].to_string();
    assert!(said.contains("“Yol › Toprak işleri” grubuna eklendi"), "{said}");

    // The graph's recipes keep the groups, the style as the recipe gave it and SINIR as the recipe defined it.
    let theirs: Value = serde_json::from_str(&std::fs::read_to_string(&recipe).unwrap()).unwrap();
    let def = theirs["nodes"].as_array().unwrap().iter().find_map(|n| n.get("recipe")).unwrap()["layer"].clone();
    let graph = call(&mut server, "road.project.graph", json!({"drawing": drawing}));
    assert_eq!(graph["isError"], false, "{graph}");
    let layers: Vec<Value> = graph["structuredContent"]["output"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n.get("recipe").map(|r| r["layer"].clone()))
        .collect();
    assert_eq!(layers.len(), 2);
    for l in &layers {
        assert_eq!(l["group"], json!(["Yol", "Toprak işleri"]));
        assert_eq!(l["style"], def["style"]);
        assert_eq!(l["lineTypeDef"], def["lineTypeDef"]);
    }

    // A style whose colour is not its definition's: refused, nothing written.
    let other = opened(&mut server, "tutmayan");
    let r = call(&mut server, "road.project.produce", json!({"drawing": other, "path": fixture("recipes/v1/stili-tutmayan.json")}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("invalid_project_recipe"), "{r}");
}

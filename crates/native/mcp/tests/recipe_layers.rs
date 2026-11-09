//! A recipe's layers through a real AI host (plan/map-cad.md MAP-20, docs/adr/3640): the shared cases' Yol project of
//! `road.project.produce` opened as a .kcad file over MCP. The fixture recipe whose layer “Kazı sınırı” the project
//! lacks adds it with the recipe's look (the warning says so) and both zero lines go on it; the graph's recipes then
//! keep that layer's definition; a second call finds it by its name and leaves both. The recipe whose layer “Sıfır
//! hattı” the project has under another id draws on that layer; given in `layers`, on the one given.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"recipe-layers-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

fn opened(server: &mut Server, name: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.produce.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-recipe-layers-{name}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(path);
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn a_recipe_s_layers_over_mcp() {
    let mut server = Server::new();
    let drawing = opened(&mut server, "yeni");
    let recipe = fixture("recipes/v1/yeni-katmanli.json");
    let r = call(&mut server, "road.project.produce", json!({"drawing": drawing, "path": recipe}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"];
    let layers = out["output"]["layers"].as_array().unwrap();
    assert_eq!(layers.len(), 1, "{out}");
    assert_eq!((layers[0]["how"].as_str(), layers[0]["toName"].as_str()), (Some("created"), Some("Kazı sınırı")));
    let added = layers[0]["to"].as_str().unwrap().to_owned();
    assert!(out.to_string().contains("recipe_layer_created"), "{out}");
    assert_eq!(out["output"]["produced"].as_array().unwrap().len(), 2);

    // The graph's recipes keep the added layer's definition: the recipe's look.
    let graph = call(&mut server, "road.project.graph", json!({"drawing": drawing}));
    assert_eq!(graph["isError"], false, "{graph}");
    let defs: Vec<Value> = graph["structuredContent"]["output"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n.get("recipe").map(|r| r["layer"].clone()))
        .collect();
    // Docs/adr/4040: with its whole style too (the recipe from before gave colour, line type and weight).
    let style = json!({"color": "#2A9D8F", "lineType": "dashed", "lineWeight": 0.35});
    let want = json!({"id": added, "name": "Kazı sınırı", "color": "#2A9D8F", "lineType": "dashed", "lineWeight": 0.35, "style": style});
    assert_eq!(defs, vec![want.clone(), want]);

    // Again: found by its name, both left.
    let again = call(&mut server, "road.project.produce", json!({"drawing": drawing, "path": recipe}));
    assert_eq!(again["isError"], false, "{again}");
    let left: Vec<&str> = again["structuredContent"]["output"]["left"].as_array().unwrap().iter().filter_map(|l| l["why"].as_str()).collect();
    assert_eq!(left, ["exists", "exists"]);

    // By name under another id, and the one given.
    let other = opened(&mut server, "adli");
    let named = fixture("recipes/v1/katman-adli.json");
    let r = call(&mut server, "road.project.produce", json!({"drawing": other, "path": named}));
    assert_eq!(r["isError"], false, "{r}");
    let went = &r["structuredContent"]["output"]["layers"][0];
    assert_eq!((went["from"].as_str(), went["to"].as_str(), went["how"].as_str()), (Some("k-7"), Some("sifir"), Some("name")));
    let third = opened(&mut server, "verilen");
    let r = call(&mut server, "road.project.produce", json!({"drawing": third, "path": named, "layers": {"k-7": "drenaj"}}));
    assert_eq!(r["isError"], false, "{r}");
    let went = &r["structuredContent"]["output"]["layers"][0];
    assert_eq!((went["to"].as_str(), went["how"].as_str()), (Some("drenaj"), Some("given")));
}

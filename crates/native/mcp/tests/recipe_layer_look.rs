//! A recipe layer's transparency, blending mode, type and Yazdır through a real AI host (plan/map-cad.md MAP-20,
//! docs/adr/4440): the shared cases' Yol project of `road.project.produce` opened as a .kcad file over MCP. The fixture
//! recipe whose layer “Kazı sınırı” is 40 % transparent in the multiply mode, a line layer not printed, adds the layer
//! so and says it; the graph's recipes then keep them, so a recipe saved here carries them. A recipe whose point layer
//! would take the zero line's lines is refused (the plan too), and so is one whose composition keeps a mask.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"recipe-layer-look-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

fn opened(server: &mut Server, name: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.produce.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-recipe-look-{name}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(path);
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn a_recipe_layer_s_transparency_type_and_print_over_mcp() {
    let mut server = Server::new();
    let drawing = opened(&mut server, "saydam");
    let recipe = fixture("recipes/v1/saydam-katmanli.json");
    let r = call(&mut server, "road.project.produce", json!({"drawing": drawing, "path": recipe}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["layers"][0]["how"], "created", "{out}");
    let said = r["structuredContent"].to_string();
    assert!(said.contains("Tarifteki gibi saydamlığı %40, karışım kipi Çoğalt, türü Çizgi ve yazdırılmaz."), "{said}");

    // The graph's recipes keep them as the recipe gave them.
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
        assert_eq!(l["compose"], def["compose"]);
        assert_eq!(l["geometry"], "line");
        assert_eq!(l["plot"], false);
    }

    // A point layer for the zero line's lines: refused, the plan too; nothing written.
    let other = opened(&mut server, "nokta");
    let point = fixture("recipes/v1/nokta-katmanli.json");
    for op in ["plan", "execute"] {
        let r = call(&mut server, "road.project.produce", json!({"drawing": other, "path": point, "op": op}));
        assert_eq!(r["isError"], true, "{op}: {r}");
        assert!(r.to_string().contains("layer_geometry_mismatch"), "{op}: {r}");
    }

    // A composition with a mask: refused.
    let r = call(&mut server, "road.project.produce", json!({"drawing": other, "path": fixture("recipes/v1/maskeli.json")}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("invalid_project_recipe"), "{r}");
}

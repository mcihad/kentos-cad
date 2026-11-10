//! A recipe layer's fields, the value lists they link and its subtypes through a real AI host (plan/map-cad.md MAP-20,
//! docs/adr/4860): the shared cases' Yol project of `road.project.produce` opened as a .kcad file over MCP. The fixture
//! recipe whose layer “Kazı sınırı” holds three fields, one linked to the list “Sınır türü”, adds the layer with them and
//! the list, and says it; the graph's recipes then keep them, so a recipe saved here carries them. A recipe whose
//! field keeps a bound table's column is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"recipe-layer-fields-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

fn opened(server: &mut Server, name: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.produce.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-recipe-fields-{name}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(path);
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn a_recipe_layer_s_fields_lists_and_subtypes_over_mcp() {
    let mut server = Server::new();
    let drawing = opened(&mut server, "alanli");
    let recipe = fixture("recipes/v1/alanli-katmanli.json");
    let r = call(&mut server, "road.project.produce", json!({"drawing": drawing, "path": recipe}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["layers"][0]["how"], "created", "{out}");
    assert_eq!(out["layers"][0]["domainsAdded"], json!(["Sınır türü"]), "{out}");
    let said = r["structuredContent"].to_string();
    assert!(said.contains("Alanları (3) ve alt türleri tarifteki gibi. “Sınır türü” değer listesi de tarifteki tanımıyla eklendi."), "{said}");

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
        assert_eq!(l["fields"], def["fields"]);
        assert_eq!(l["domains"], def["domains"]);
        assert_eq!(l["subtypes"], def["subtypes"]);
    }

    // A field keeping a bound table's column: refused, nothing written.
    let other = opened(&mut server, "sutun");
    let r = call(&mut server, "road.project.produce", json!({"drawing": other, "path": fixture("recipes/v1/bagli-sutunlu.json")}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("invalid_project_recipe"), "{r}");
    assert!(r.to_string().contains("bağlı tablonun sütununu taşıyor"), "{r}");
}

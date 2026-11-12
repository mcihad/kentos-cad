//! A recipe layer's value combinations, record rules, indexes and forms, and `addFields`, through a real AI host
//! (plan/map-cad.md MAP-20, docs/adr/5240): the shared cases' Yol project of `road.project.produce` opened as a .kcad
//! file over MCP. The fixture recipe whose layer “Kazı sınırı” keeps one of each adds the layer with them and its form
//! as a draft, and the graph's recipes then keep them; the recipe of the zero line's name with `addFields` gives the
//! layer found here the fields it lacks; a recipe whose form binds a field the layer lacks is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"recipe-layer-rules-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

/// The produce cases' setup, or a case's, opened.
fn opened(server: &mut Server, case: Option<&str>, tag: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.produce.json")).unwrap();
    let setup = match case {
        Some(prefix) => cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with(prefix)).unwrap()["setup"].clone(),
        None => cases["setup"].clone(),
    };
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-recipe-rules-{tag}-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let _ = std::fs::remove_file(path);
    opened["structuredContent"]["drawing"].clone()
}

#[test]
fn a_recipe_layer_s_combinations_rules_indexes_and_forms_over_mcp() {
    let mut server = Server::new();
    let drawing = opened(&mut server, None, "tanimli");
    let recipe = fixture("recipes/v1/tanimli-katmanli.json");
    let r = call(&mut server, "road.project.produce", json!({"drawing": drawing, "path": recipe}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!(out["layers"][0]["formsAdded"], json!(["Sınır formu"]), "{out}");
    let said = r["structuredContent"].to_string();
    assert!(said.contains("1 değer bileşimi, 1 kayıt kuralı ve 1 dizin de tarifteki gibi."), "{said}");
    assert!(said.contains("“Sınır formu” formu taslak olarak eklendi"), "{said}");
    let theirs: Value = serde_json::from_str(&std::fs::read_to_string(&recipe).unwrap()).unwrap();
    let def = theirs["nodes"].as_array().unwrap().iter().find_map(|n| n.get("recipe")).unwrap()["layer"].clone();
    let graph = call(&mut server, "road.project.graph", json!({"drawing": drawing}));
    for l in graph["structuredContent"]["output"]["nodes"].as_array().unwrap().iter().filter_map(|n| n.get("recipe").map(|r| r["layer"].clone())) {
        for key in ["contingents", "rules", "indexes", "forms"] {
            assert_eq!(l[key], def[key], "{key}");
        }
    }

    // addFields: the zero line's layer here takes the fields it lacks.
    let found = opened(&mut server, Some("Eksik alanları ekle: adıyla bulunan"), "eksik");
    let r = call(&mut server, "road.project.produce", json!({"drawing": found, "path": fixture("recipes/v1/tanimli-adli.json"), "addFields": true}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["output"]["layers"][0]["fieldsAdded"], json!(["Tür", "Zemin"]), "{r}");

    // A form bound to a field the layer lacks: refused.
    let other = opened(&mut server, None, "form");
    let r = call(&mut server, "road.project.produce", json!({"drawing": other, "path": fixture("recipes/v1/formu-tutmayan.json")}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("katmanın alanlarında olmayan “Yok” alanına bağlı"), "{r}");
}

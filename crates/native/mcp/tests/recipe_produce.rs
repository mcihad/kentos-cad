//! Tariften üret through a real AI host (plan/map-cad.md MAP-20, docs/adr/3240): the shared cases' Yol project of
//! `road.project.produce` opened as a .kcad file over MCP, the fixture recipe of two zero lines produced. The answer is
//! the shared case's (the sources found by name, the lines' counts); `road.project.graph` then holds both drawings with
//! their recipes, current; a second call leaves both (`exists`); a recipe naming a surface the project lacks, its
//! drawing asked by name, is refused with the drawing named and nothing written.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"recipe-produce-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn fixture(rel: &str) -> String {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures").join(rel).display().to_string()
}

#[test]
fn a_recipe_is_produced_in_another_project_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.produce.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-recipe-produce-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let recipe = fixture("recipes/v1/sifir-hatlari.json");
    let r = call(&mut server, "road.project.produce", json!({"drawing": drawing, "path": recipe}));
    assert_eq!(r["isError"], false, "{r}");
    let got = &r["structuredContent"]["output"];
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Tarifin iki sıfır hattı bu projede: kaynaklar adlarıyla").unwrap();
    let want = &case["steps"][0]["result"]["output"];
    for (g, w) in got["produced"].as_array().unwrap().iter().zip(want["produced"].as_array().unwrap()) {
        for key in ["from", "drawing", "command", "source"] {
            assert_eq!(g[key], w[key], "{key}");
        }
        assert_eq!(g["uids"].as_array().unwrap().len(), w["uids"].as_array().unwrap().len(), "the lines");
    }
    let graph = call(&mut server, "road.project.graph", json!({"drawing": drawing}));
    assert_eq!(graph["isError"], false, "{graph}");
    let g = &graph["structuredContent"]["output"];
    assert_eq!((g["drawings"].as_u64(), g["outdated"].as_u64()), (Some(2), Some(0)));
    let inputs: Vec<&Value> = g["nodes"].as_array().unwrap().iter().filter(|n| n["kind"] == "drawing").map(|n| &n["recipe"]["input"]).collect();
    assert_eq!(inputs[0], &json!({"base": "t-5", "comparison": "Tümsek", "layerId": "sifir"}), "kept as this project's");
    let again = call(&mut server, "road.project.produce", json!({"drawing": drawing, "path": recipe}));
    let left: Vec<&str> = again["structuredContent"]["output"]["left"].as_array().unwrap().iter().map(|l| l["why"].as_str().unwrap()).collect();
    assert_eq!(left, ["exists", "exists"]);
    let refused = call(
        &mut server,
        "road.project.produce",
        json!({"drawing": drawing, "path": fixture("recipes/v1/eksik-kaynakli.json"), "drawings": ["cz-3"]}),
    );
    assert_eq!(refused["isError"], true, "{refused}");
    assert!(refused.to_string().contains("Tarifin “cz-3” çizimi (Sıfır hattı)"), "{refused}");
    let after = call(&mut server, "road.project.graph", json!({"drawing": drawing}));
    assert_eq!(after["structuredContent"]["output"]["drawings"].as_u64(), Some(2), "nothing written");
    let _ = std::fs::remove_file(path);
}

//! Esnet with the vertices inside a changed edge through a real AI host (plan/map-cad.md MAP-07, docs/adr/3400): the
//! shared cases' drawing opened as a .kcad file over MCP. A window over A's north-east corner without a selection
//! stretches A and B2 (both have the corner); with Topolojik düzenleme the T junction B1 and B2 share on A's north edge
//! goes to the same fraction of the turned edge in both (the cases' answer: stretched A and B2, followed B1 and the
//! line J), and the saved file has the junction and J's end on A's new edge.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"topology-inner-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_t_junction_two_stretched_objects_share_goes_with_the_edge_over_mcp() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.stretch.inner.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with("Seçimsiz")).unwrap().clone();
    // The cases' drawing is written without ids; here each object gets one.
    let mut setup = cases["setup"].clone();
    let count = setup["entities"].as_array().unwrap().len();
    let uids: Vec<String> = (1..=count).map(|i| format!("0192f5c0-3400-7000-8000-{i:012x}")).collect();
    setup["version"] = json!(2);
    setup["uids"] = json!(uids);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-topology-inner-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let done = call(&mut server, "cad.entities.stretch", args);
    assert_eq!(done["isError"], false, "{done}");
    // The cases' answer, each `$uidOf:N` the N-th object's id.
    let ids = |v: &Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let n: usize = s.as_str().unwrap().trim_start_matches("$uidOf:").parse().unwrap();
                uids[n - 1].clone()
            })
            .collect()
    };
    let output = &done["structuredContent"]["output"];
    let want = &step["result"]["output"];
    for key in ["stretched", "followed"] {
        let got: Vec<String> = output[key].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_owned()).collect();
        assert_eq!(got, ids(&want[key]), "{key}: {done}");
    }
    // The saved file: the junction at (22, 31) in B1 and B2, J's end at (33, 31.5), on A's north edge (0, 30)–(44, 32).
    let saved = std::env::temp_dir().join(format!("kentos-topology-inner-saved-{}.kcad", std::process::id()));
    let written = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(written["isError"], false, "{written}");
    let after = kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap();
    let (e0, n0) = (487_000.0, 4_420_000.0);
    let has = |pts: &[kentos_commands::Vec2], x: f64, y: f64| pts.iter().any(|p| (p.x - e0 - x).abs() < 1e-6 && (p.y - n0 - y).abs() < 1e-6);
    for k in [1, 2] {
        let kentos_commands::Entity::Polygon(b) = &after.entities[k] else { panic!("B{k}") };
        assert!(has(&b.pts, 22.0, 31.0), "B{k}: {:?}", b.pts);
    }
    let kentos_commands::Entity::Line(j) = &after.entities[10] else { panic!("J") };
    assert!((j.b.x - e0 - 33.0).abs() < 1e-6 && (j.b.y - n0 - 31.5).abs() < 1e-6, "{:?}", j.b);
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(saved);
}

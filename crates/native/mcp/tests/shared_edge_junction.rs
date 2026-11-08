//! Paralel kaydır's shared edge with the vertices inside it through a real AI host (plan/map-cad.md MAP-07,
//! docs/adr/2600): the shared cases' drawing opened as a .kcad file over MCP. A's east edge 3 m east with Topolojik
//! düzenleme carries B1's and B2's shared corner on it (one place) and the saved file has it at the moved edge; the
//! shortened trapeze's edge that would leave a shared vertex off it is refused, as the cases say.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"shared-edge-junction-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_vertices_inside_a_shared_edge_go_with_it_over_mcp() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.shapes.edit.junction.json")).unwrap();
    let case = |start: &str| {
        cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().unwrap().starts_with(start)).unwrap().clone()
    };
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-shared-edge-junction-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let uid = |i: usize| snapshot.uids[i - 1].to_text();

    // The T junction: one place carried, B1 and B2 changed with A.
    let junction = case("Üçlü birleşim:");
    let mut args = junction["steps"][0]["input"].clone();
    for (k, v) in [("id", uid(1))] {
        args["work"][k] = json!(v);
    }
    args["drawing"] = drawing.clone();
    let done = call(&mut server, "cad.shapes.edit", args);
    assert_eq!(done["isError"], false, "{done}");
    let output = &done["structuredContent"]["output"];
    assert_eq!(output["carried"], 1, "{done}");
    assert_eq!(output["neighbours"].as_array().map(Vec::len), Some(2), "{done}");

    // The saved file: B1's corner on the edge is at the moved edge, x 13 m from the origin's east.
    let saved = std::env::temp_dir().join(format!("kentos-shared-edge-junction-saved-{}.kcad", std::process::id()));
    let written = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(written["isError"], false, "{written}");
    let after = kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap();
    let kentos_commands::Entity::Polygon(b1) = &after.entities[1] else { panic!("B1") };
    assert!(b1.pts.iter().any(|p| (p.x - 487_013.0).abs() < 1e-6 && (p.y - 4_420_010.0).abs() < 1e-6), "{:?}", b1.pts);
    assert!(b1.pts.iter().all(|p| (p.x - 487_010.0).abs() > 1e-6), "nothing left on the old edge: {:?}", b1.pts);

    // The shortened edge that would leave the shared vertex off it: refused, the cases' words.
    let off = case("Kısalan kenar:");
    let mut args = off["steps"][0]["input"].clone();
    args["work"]["id"] = json!(uid(10));
    args["drawing"] = drawing;
    let refused = call(&mut server, "cad.shapes.edit", args);
    assert_eq!(refused["isError"], true, "{refused}");
    let want = off["steps"][0]["result"]["error"]["message"].as_str().unwrap();
    assert!(refused.to_string().contains(want), "{refused}");
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(saved);
}

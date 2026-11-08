//! The pipes crossing a pipe run in its profile through a real AI host (plan/map-cad.md MAP-16, docs/adr/2760): the
//! shared case's drawing (the sewer and the water network) opened as a .kcad file over MCP. `cad.pipes.profile` with
//! the water network as `crossing` lists the water pipes crossing the run as the case does and draws an ellipse for
//! each one with an invert there; the drawing saved and read back as a .kcad file keeps them on the profile's layer.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"profile-crossings-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_water_pipes_crossing_the_sewer_are_drawn_in_its_profile() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.profile.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Kesişen borular: içme suyu boruları atıksu hattını kesiyor")
        .unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-profile-crossings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("altyapi.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let mut input = step["input"].clone();
    input["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.pipes.profile", input);
    assert_eq!(r["isError"], false, "{r}");
    let got = r["structuredContent"]["output"]["crossings"].as_array().unwrap().clone();
    let want = step["result"]["output"]["crossings"].as_array().unwrap().clone();
    assert_eq!(got.len(), want.len(), "{got:?}");
    for (g, w) in got.iter().zip(&want) {
        assert_eq!((&g["uid"], &g["name"]), (&w["uid"], &w["name"]));
        for key in ["station", "invert", "diameter"] {
            match (g[key].as_f64(), w[key].as_f64()) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{key}: {a} {b}"),
                (a, b) => assert_eq!(a.is_some(), b.is_some(), "{key} of {}", w["name"]),
            }
        }
    }
    let drawn = want.iter().filter(|c| c["invert"].is_number()).count();
    let saved = dir.join("altyapi-profil.kcad");
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let back = serde_json::to_value(kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap()).unwrap();
    let ellipses = back["entities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] == "ellipse" && e["layerId"] == "profil")
        .count();
    assert_eq!(ellipses, drawn, "an ellipse for each crossing pipe with an invert");
    let _ = std::fs::remove_dir_all(dir);
}

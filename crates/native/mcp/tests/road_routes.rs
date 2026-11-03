//! KentOS Yol's preliminary routes through MCP (plan/road.md Y12.1, docs/adr/0910 §1): the routes' shared cases'
//! drawing is written to a file and opened; a route is found avoiding an area, drawn, and fitted as an alignment.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"road-route-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities":{}})
}

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":meta()}}))
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

#[test]
fn a_route_is_found_drawn_and_fitted() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.routes.find")["annotations"]["readOnlyHint"], true);

    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.routes.find.json")).expect("cases");
    let snapshot = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let path = std::env::temp_dir().join(format!("kentos-road-routes-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).expect("encodes")).expect("writes");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let ask = json!({"drawing":drawing,"surface":"Mevcut zemin","from":{"x":70.0,"y":80.0},"to":{"x":520.0,"y":280.0},
        "cell":10.0,"margin":30.0,"grade":8.0,"climb":10.0});
    let open = call(&mut server, "road.routes.find", ask.clone());
    assert_eq!(open["isError"], false, "{open}");
    let mut avoiding = ask.clone();
    avoiding["avoid"] = json!(["yasak"]);
    avoiding["tolerance"] = json!(3.0);
    let around = call(&mut server, "road.routes.find", avoiding.clone());
    let (a, b) = (&open["structuredContent"]["output"], &around["structuredContent"]["output"]);
    assert!(b["length"].as_f64() > a["length"].as_f64(), "{a} {b}");

    avoiding["layerId"] = json!("guzergah");
    let drawn = call(&mut server, "road.routes.draw", avoiding);
    assert_eq!(drawn["isError"], false, "{drawn}");
    let uid = drawn["structuredContent"]["output"]["uids"][0].clone();
    let fitted = call(&mut server, "road.alignments.fit", json!({"drawing":drawing,"uid":uid,"method":"bestFit",
        "name":"Ön güzergâh aksı","maxRadius":200.0}));
    assert_eq!(fitted["isError"], false, "{fitted}");
}

/// The road project's sample drawing's junction (docs/adr/0910 §2): asked, widened and drawn.
#[test]
fn the_sample_junction_is_asked_widened_and_drawn() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let asked = call(&mut server, "road.junctions.query", json!({"drawing":drawing,"junction":"j-1"}));
    assert_eq!(asked["isError"], false, "{asked}");
    let j = &asked["structuredContent"]["output"]["junctions"][0];
    assert_eq!(j["returns"].as_array().map(Vec::len), Some(4), "{asked}");
    assert_eq!(j["connected"], true);
    let first = j["returns"][0]["length"].as_f64().expect("a length");
    let wider = call(&mut server, "road.junctions.edit", json!({"drawing":drawing,"operation":"set","junction":"j-1","radius":25.0}));
    assert_eq!(wider["isError"], false, "{wider}");
    let longer = wider["structuredContent"]["output"]["junction"]["returns"][0]["length"].as_f64().expect("a length");
    assert!(longer > first, "{first} → {longer}");
    let drawn = call(&mut server, "road.junctions.draw", json!({"drawing":drawing,"junction":"j-1","layerId":"0","labels":true}));
    assert_eq!(drawn["isError"], false, "{drawn}");
    assert_eq!(drawn["structuredContent"]["output"]["uids"].as_array().map(Vec::len), Some(5), "{drawn}");
    let separated = call(&mut server, "road.junctions.edit", json!({"drawing":drawing,"operation":"set","junction":"j-1","kind":"separated"}));
    assert_eq!(separated["structuredContent"]["output"]["junction"]["connected"], false, "{separated}");
}

/// KentOS Yol's 3D scene (plan/road.md Y12.3, docs/adr/0910 §3): the scene's shared cases' drawing written as glTF.
#[test]
fn the_scene_goes_out_as_gltf() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = tools.iter().find(|t| t["name"] == "file.export.gltf").cloned().unwrap_or_default();
    assert_eq!(tool["annotations"]["destructiveHint"], true, "{tool}");
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/file.export.gltf.json")).expect("cases");
    let snapshot = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let path = std::env::temp_dir().join(format!("kentos-road-scene-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).expect("encodes")).expect("writes");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let _ = std::fs::remove_file(&path);
    let drawing = opened["structuredContent"]["drawing"].clone();
    let glb = std::env::temp_dir().join(format!("kentos-road-scene-{}.glb", std::process::id()));
    let written = call(&mut server, "file.export.gltf", json!({"drawing":drawing,"path":glb,"overwrite":true}));
    assert_eq!(written["isError"], false, "{written}");
    let out = &written["structuredContent"]["output"];
    assert_eq!(out["meshes"].as_array().map(Vec::len), Some(3), "{out}");
    let bytes = std::fs::read(&glb).expect("written");
    let _ = std::fs::remove_file(&glb);
    assert_eq!(&bytes[0..4], b"glTF");
    assert_eq!(bytes.len() as u64, out["bytes"].as_u64().unwrap());
}

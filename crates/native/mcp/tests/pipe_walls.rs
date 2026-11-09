//! Et kalınlığı through a real AI host (plan/map-cad.md MAP-16, docs/adr/3960): the shared cases' walls drawing opened
//! as a .kcad file over MCP. `cad.pipes.check` with `diameter.wall` measures the cover to the pipes' top outside and
//! the clearances to the water network outside to outside, as the shared case says; an empty wall attribute is refused
//! at `diameter.wall`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"pipe-walls-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_walls_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.check.json")).unwrap();
    let case = |name: &str| cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap().clone();
    let walled = case("et kalınlığı: açıklıklar dıştan dışa");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(walled["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-pipe-walls-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let step = &walled["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.pipes.check", args);
    assert_eq!(r["isError"], false, "{r}");
    let (got, want) = (&r["structuredContent"]["output"], &step["result"]["output"]);
    assert_eq!(got["coverTo"], "outside");
    for (g, w) in got["pipes"].as_array().unwrap().iter().zip(want["pipes"].as_array().unwrap()) {
        assert_eq!(g["uid"], w["uid"]);
        for key in ["wall", "minCover", "maxCover"] {
            match (g[key].as_f64(), w[key].as_f64()) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-6, "{key}: {a} vs {b}"),
                (a, b) => assert_eq!(a, b, "{key} of {}", g["uid"]),
            }
        }
        assert_eq!(g["findings"], w["findings"], "{}", g["uid"]);
    }
    for (g, w) in got["crossings"].as_array().unwrap().iter().zip(want["crossings"].as_array().unwrap()) {
        match (g["clearance"].as_f64(), w["clearance"].as_f64()) {
            (Some(a), Some(b)) => assert!((a - b).abs() < 1e-6, "clearance: {a} vs {b}"),
            (a, b) => assert_eq!(a, b),
        }
    }

    let refused = case("et kalınlığı: boş öznitelik");
    let mut args = refused["steps"][0]["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.pipes.check", args);
    assert_eq!(r["isError"], true, "{r}");
    assert_eq!(r["structuredContent"]["error"]["code"], "invalid_pipe_check", "{r}");
    assert_eq!(r["structuredContent"]["error"]["path"], "diameter.wall", "{r}");
    let _ = std::fs::remove_file(path);
}

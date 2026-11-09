//! Yüzeyden kot ver through a real AI host (plan/map-cad.md MAP-06, docs/adr/4180): the shared cases' drawing
//! (fixtures/commands/v1/cad.entities.drape.json: the surface “Zemin” of four corners, its diagonal AC) opened as a
//! .kcad file over MCP. The point inside takes its plane's height (worked out here from ABC's corners), the polyline
//! with break points gets its vertex on AC; the tool is destructive (it writes over elevations) and one undo puts the
//! heights back.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"drape-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn drape_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.drape.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let uid = |slot: usize| cases["setup"]["uids"][slot - 1].as_str().unwrap().to_owned();
    let path = std::env::temp_dir().join(format!("kentos-drape-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();

    // The tool is listed as destructive.
    let listed = server.handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"drape-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap();
    let tools = listed["result"]["tools"].as_array().unwrap();
    let drape = tools.iter().find(|t| t["name"] == "cad.entities.drape").expect("listed");
    assert_eq!(drape["annotations"]["destructiveHint"], true, "{drape}");

    let r = call(&mut server, "cad.entities.drape", json!({"drawing": drawing, "uids": [uid(5), uid(8)], "surface": "Zemin", "breaks": true}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["set"].as_u64(), out["inserted"].as_u64()), (Some(4), Some(1)), "{out}");
    // ABC: z = 100 + 0.04 x + 0.1 y at (60, 20).
    let point = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid(5)}));
    let z = point["structuredContent"]["entity"]["z"].as_f64().unwrap();
    assert!((z - (100.0 + 0.04 * 60.0 + 0.1 * 20.0)).abs() < 1e-9, "{z}");
    let line = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid(8)}));
    let pts = line["structuredContent"]["entity"]["pts"].as_array().unwrap().clone();
    assert_eq!(pts.len(), 4);
    assert!((pts[1]["x"].as_f64().unwrap() - 500_045.0).abs() < 1e-6 && (pts[1]["y"].as_f64().unwrap() - 4_400_036.0).abs() < 1e-6);
    let undo = call(&mut server, "drawing.undo", json!({"drawing": drawing}));
    assert_eq!(undo["isError"], false, "{undo}");
    let point = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid(5)}));
    assert!(point["structuredContent"]["entity"].get("z").is_none(), "{point}");
}

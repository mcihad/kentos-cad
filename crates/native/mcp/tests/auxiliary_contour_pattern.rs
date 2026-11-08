//! The auxiliary contours' pattern through a real AI host (plan/map-cad.md MAP-14, docs/adr/2720): the shared cases'
//! drawing opened as a .kcad file over MCP. `road.terrains.sheetcontours` draws the lines; the auxiliary ones name the
//! package's line type, which the project's types now hold (BÖHHBÜY Ek-3 0003: 5 mm dash, 1 mm gap, on paper). The
//! drawing saved and read back as a .kcad file keeps the type and every auxiliary line's pattern.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"auxiliary-contour-pattern-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_auxiliary_lines_keep_the_package_s_pattern_in_the_file() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.sheetcontours.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-auxiliary-pattern-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("egriler.kcad");
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let r = call(&mut server, "road.terrains.sheetcontours", json!({"drawing": drawing, "surface": "t-1", "layerId": "esyukselti"}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let name = out["pattern"].as_str().unwrap().to_owned();
    assert_eq!(name, "Yardımcı eğri (bohhbuy-2018-m78)");
    let auxiliary = out["lines"].as_array().unwrap().iter().filter(|l| l["kind"] == "auxiliary").count();
    assert!(auxiliary > 0);
    let saved = dir.join("egriler-cizildi.kcad");
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let back = kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap();
    let value = serde_json::to_value(&back).unwrap();
    let types = value["settings"]["lineTypes"].as_array().unwrap();
    let t = types.iter().find(|t| t["name"] == name).unwrap();
    assert_eq!((&t["elements"], &t["unit"]), (&json!([{"length": 5.0}, {"length": -1.0}]), &json!("paper")));
    let patterned = value["entities"].as_array().unwrap().iter().filter(|e| e["linePattern"] == name.as_str()).count();
    assert_eq!(patterned, auxiliary, "every auxiliary line names the type");
    let _ = std::fs::remove_dir_all(dir);
}

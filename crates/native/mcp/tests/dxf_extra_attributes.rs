//! Tanımı olmayan öznitelikler DXF'te through a real AI host (plan/map-cad.md MAP-17, docs/adr/3980, 4380): a block
//! without attribute definitions inserted with attributes of its own, written by `file.export.dxf`. The attribute whose
//! name is a DXF tag as it is goes out as an invisible ATTRIB of the INSERT; the one with a space too, its tag with “_”
//! and its name in KentOS's data, and the export's report says so; the file opened again has every attribute as it was.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"dxf-extra-attributes-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn ok(s: &mut Server, name: &str, args: Value) -> Value {
    let r = call(s, name, args);
    assert_eq!(r["isError"], false, "{name}: {r}");
    r["structuredContent"].clone()
}

#[test]
fn an_attribute_without_a_definition_goes_out_as_an_invisible_attrib() {
    let dir = std::env::temp_dir().join(format!("kentos-dxf-extra-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut server = Server::new();
    let new = ok(&mut server, "drawing.new", json!({"workspace": "cad"}));
    let (drawing, layer) = (new["drawing"].clone(), new["summary"]["activeLayer"].clone());
    let line = ok(&mut server, "cad.line.create", json!({"drawing": drawing, "layerId": layer, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 1.0, "y": 0.0}}));
    let uid = line["output"]["uid"].clone();
    let block = ok(&mut server, "cad.blocks.define", json!({"drawing": drawing, "name": "Rögar", "base": {"x": 0.0, "y": 0.0}, "uids": [uid], "units": "m"}));
    let attrs = json!({"Malzeme": "Çelik", "Bakım notu": "2026"});
    ok(&mut server, "cad.blocks.insert", json!({"drawing": drawing, "block": block["output"]["block"], "at": {"x": 10.0, "y": 20.0}, "layerId": layer, "attrs": attrs}));
    let path = dir.join("ogeler.dxf");
    let written = ok(&mut server, "file.export.dxf", json!({"drawing": drawing, "path": path}));
    assert_eq!(written["status"], "completed", "{written}");
    let notes = written["output"]["report"]["notes"].as_array().unwrap();
    assert!(notes.iter().any(|n| n["what"] == "Öznitelik etiketi" && n["reason"].as_str().unwrap().contains("“Bakım_notu” etiketiyle")), "{notes:?}");
    let text = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
    assert!(text.contains("AcDbAttribute\r\n  2\r\nMalzeme\r\n 70\r\n1\r\n"), "Malzeme's invisible ATTRIB");
    assert!(text.contains("AcDbAttribute\r\n  2\r\nBakım_notu\r\n 70\r\n1\r\n"), "Bakım notu's, its tag with “_”");
    assert!(!text.contains("  2\r\nBakım notu\r\n"), "no tag with a space");
    let back = ok(&mut server, "drawing.open", json!({"path": path}));
    let records = ok(&mut server, "drawing.entities", json!({"drawing": back["drawing"]}));
    let insert = records["items"].as_array().unwrap().iter().find(|r| r["entity"]["kind"] == "insert").expect("the insert").clone();
    assert_eq!(insert["entity"]["attrs"], attrs);
    std::fs::remove_dir_all(dir).unwrap();
}

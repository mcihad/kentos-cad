//! Tanımlı özniteliğin boşluklu etiketinin geri adı through a real AI host (plan/map-cad.md MAP-17, docs/adr/4800): a
//! block whose attribute definition is named “Kol boyu”, inserted with its value, written by `file.export.dxf`. The
//! ATTDEF and the ATTRIB go out as “Kol_boyu”, the ATTDEF holding the name in KentOS's data, and the report says the
//! name comes back; the file opened again has the definition's tag and the insert's value under “Kol boyu”.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"dxf-defined-names-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn ok(s: &mut Server, name: &str, args: Value) -> Value {
    let r = call(s, name, args);
    assert_eq!(r["isError"], false, "{name}: {r}");
    r["structuredContent"].clone()
}

#[test]
fn a_definition_named_with_a_space_comes_back_by_its_name() {
    let dir = std::env::temp_dir().join(format!("kentos-dxf-defined-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut server = Server::new();
    let new = ok(&mut server, "drawing.new", json!({"workspace": "cad"}));
    let (drawing, layer) = (new["drawing"].clone(), new["summary"]["activeLayer"].clone());
    let line = ok(&mut server, "cad.line.create", json!({"drawing": drawing, "layerId": layer, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 0.0, "y": 6.0}}));
    let block = ok(&mut server, "cad.blocks.define", json!({"drawing": drawing, "name": "Direk", "base": {"x": 0.0, "y": 0.0}, "uids": [line["output"]["uid"]], "units": "m"}));
    let id = block["output"]["block"].clone();
    ok(&mut server, "cad.blocks.edit", json!({"drawing": drawing, "operation": "attributes", "block": id, "attributes": [
        {"tag": "No", "p": {"x": 0.5, "y": 5.0}, "height": 0.4, "rotation": 0.0},
        {"tag": "Kol boyu", "p": {"x": 0.5, "y": 3.5}, "height": 0.25, "rotation": 0.0, "value": "2.5 m"}
    ]}));
    ok(&mut server, "cad.blocks.insert", json!({"drawing": drawing, "block": id, "at": {"x": 10.0, "y": 20.0}, "layerId": layer, "attributes": {"No": "7", "Kol boyu": "3 m"}}));
    let path = dir.join("direk.dxf");
    let written = ok(&mut server, "file.export.dxf", json!({"drawing": drawing, "path": path}));
    let notes = written["output"]["report"]["notes"].as_array().unwrap();
    assert!(
        notes.iter().any(|n| n["what"] == "Öznitelik etiketi" && n["reason"].as_str().unwrap().contains("adı ATTDEF'in KentOS verisinde")),
        "{notes:?}"
    );
    let text = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
    assert!(text.contains("AcDbAttributeDefinition\r\n  3\r\n\r\n  2\r\nKol_boyu\r\n"), "Kol boyu's ATTDEF, its tag with “_”");
    assert!(text.contains("1000\r\ntagname\r\n1000\r\nKol_boyu\r\n1000\r\nKol boyu\r\n"), "its name in KentOS's data");
    assert!(!text.contains("  2\r\nKol boyu\r\n"), "no tag with a space");
    let back = ok(&mut server, "drawing.open", json!({"path": path}));
    let records = ok(&mut server, "drawing.entities", json!({"drawing": back["drawing"]}));
    let insert = records["items"].as_array().unwrap().iter().find(|r| r["entity"]["kind"] == "insert").expect("the insert").clone();
    assert_eq!(insert["entity"]["attrs"], json!({"No": "7", "Kol boyu": "3 m"}));
    std::fs::remove_dir_all(dir).unwrap();
}

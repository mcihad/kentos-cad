//! Public MCP discovery and query must match the desktop's read-only diagnostic contract.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn tool(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":name,"arguments":args,"_meta":{
            "io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"inspection-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone();
    assert_eq!(result["isError"], false, "{result}");
    result["structuredContent"].clone()
}

#[test]
fn inspection_through_mcp_keeps_uuid_geometry_and_reports_physical_gap() {
    let mut server = Server::new();
    let new = tool(&mut server,"drawing.new",json!({"workspace":"cad"}));
    let drawing = &new["drawing"];
    let layer = &new["summary"]["activeLayer"];
    for (a,b) in [(500000.,500001.),(500001.004,500002.)] {
        let made = tool(&mut server,"cad.line.create",json!({"drawing":drawing,"layerId":layer,
            "a":{"x":a,"y":4500000.},"b":{"x":b,"y":4500000.}}));
        assert_eq!(made["status"],"completed","{made}");
    }
    let before = tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    let found = tool(&mut server,"cad.drawing.inspect",json!({"drawing":drawing,"maxGap":0.005}));
    assert_eq!(found["status"],"completed","{found}");
    let findings = found["output"]["findings"].as_array().unwrap();
    let gap = findings.iter().find(|f|f["code"]=="boundary_gap").expect("4 mm gap");
    assert!((gap["distance"].as_f64().unwrap()-0.004).abs()<1e-8);
    assert_eq!(gap["uids"].as_array().unwrap().len(),2);
    assert_eq!(tool(&mut server,"drawing.entities",json!({"drawing":drawing})),before);
    let empty = tool(&mut server,"cad.drawing.inspect",json!({"drawing":drawing,"uids":[]}));
    assert_eq!(empty["output"]["examined"],0);
}

#[test]
fn hatch_origin_edit_through_mcp_keeps_all_other_hatch_data_and_undoes_once() {
    let mut server = Server::new();
    let new = tool(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = &new["drawing"];
    let layer = &new["summary"]["activeLayer"];
    let circle = tool(&mut server, "cad.circle.create", json!({"drawing":drawing,"layerId":layer,"c":{"x":0.,"y":0.},"r":5.}));
    assert_eq!(circle["status"], "completed", "{circle}");
    let made = tool(&mut server,"cad.entities.hatch",json!({"drawing":drawing,"layerId":layer,
        "region":{"kind":"point","at":{"x":0.,"y":0.},"associative":true},
        "pattern":{"name":"ANSI31","scale":2.,"angle":31.}}));
    assert_eq!(made["status"], "completed", "{made}");
    let before = tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    let hatch = before["items"].as_array().unwrap().iter().find(|r|r["entity"]["kind"]=="hatch").unwrap();
    let done = tool(&mut server,"cad.hatches.edit",json!({"drawing":drawing,"edit":{"kind":"origin","uids":[hatch["uid"]],"origin":{"kind":"point","at":{"x":0.123,"y":0.456}}}}));
    assert_eq!(done["status"], "completed", "{done}");
    let after = tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    let mut expected = hatch["entity"].clone();
    expected["pattern"]["origin"] = json!({"x":0.123,"y":0.456});
    assert_eq!(after["items"].as_array().unwrap().iter().find(|r|r["uid"]==hatch["uid"]).unwrap()["entity"],expected);
    tool(&mut server,"drawing.undo",json!({"drawing":drawing}));
    let undone = tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    assert_eq!(undone["items"].as_array().unwrap().iter().map(|r|&r["entity"]).collect::<Vec<_>>(), before["items"].as_array().unwrap().iter().map(|r|&r["entity"]).collect::<Vec<_>>());
}

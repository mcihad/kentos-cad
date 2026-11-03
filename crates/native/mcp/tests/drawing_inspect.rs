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

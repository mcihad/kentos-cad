//! A parcel inside a road's planned width through a real AI host (plan/map-cad.md MAP-09, docs/adr/2640): the shared
//! case's plan with three parcels opened as a .kcad file over MCP; `cad.zoning.check` names the three parcels Y1's
//! 12 m reach into, with the areas and words of the case.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"zoning-road-parcel-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_parcels_inside_a_road_s_width_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.zoning.check.json")).unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "yol genişliği çizilenden geniş: yola giren kararlar ve parseller")
        .unwrap()
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-zoning-road-parcel-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let answer = call(&mut server, "cad.zoning.check", json!({"drawing": drawing}));
    assert_eq!(answer["isError"], false, "{answer}");
    let kind = |f: &&Value| f["kind"] == "roadParcel";
    let got: Vec<Value> = answer["structuredContent"]["output"]["findings"].as_array().unwrap().iter().filter(kind).cloned().collect();
    let want: Vec<Value> =
        case["steps"][0]["result"]["output"]["findings"].as_array().unwrap().iter().filter(kind).cloned().collect();
    assert_eq!(got.len(), 3);
    assert_eq!(got, want);
    let _ = std::fs::remove_file(path);
}

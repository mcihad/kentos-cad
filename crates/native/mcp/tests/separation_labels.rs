//! The numbered pieces outside on the drawing through a real AI host (plan/map-cad.md MAP-11, docs/adr/3480): the
//! shared case's drawing opened as a .kcad file over MCP; `cad.readjustment.sheets` with the three parcels' outside
//! pieces numbered answers the case's labels, and the saved file has each piece as an area with its number and a
//! point inside it labelled with the number, on the parcel's layer of numbered pieces.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"separation-labels-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_numbered_pieces_over_mcp_are_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.readjustment.sheets.json")).unwrap();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("üç parselin dış parçaları"))).unwrap().clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let path = std::env::temp_dir().join(format!("kentos-separation-labels-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let step = &case["steps"][0];
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.readjustment.sheets", args);
    assert_eq!(r["isError"], false, "{r}");
    let got = r["structuredContent"]["output"]["sheets"].as_array().unwrap().clone();
    let want = step["result"]["output"]["sheets"].as_array().unwrap().clone();
    let mut numbers = Vec::new();
    for (g, w) in got.iter().zip(&want) {
        let (gl, wl) = (g["labels"].as_array().unwrap(), w["labels"].as_array().unwrap());
        assert_eq!(gl.len(), wl.len());
        for (a, b) in gl.iter().zip(wl) {
            assert_eq!((&a["part"], &a["number"]), (&b["part"], &b["number"]));
            for i in 0..2 {
                assert!((a["at"][i].as_f64().unwrap() - b["at"][i].as_f64().unwrap()).abs() < 1e-9);
            }
            numbers.push(a["number"].as_str().unwrap().to_owned());
        }
    }
    assert_eq!(numbers, ["101/30", "101/31", "101/24", "101/25"]);
    // The saved file: each number as an area's attribute and as a point's label.
    let saved = std::env::temp_dir().join(format!("kentos-separation-labels-saved-{}.kcad", std::process::id()));
    let s = call(&mut server, "drawing.save", json!({"drawing": drawing, "path": saved}));
    assert_eq!(s["isError"], false, "{s}");
    let back = serde_json::to_value(kentos_kcad::decode(&std::fs::read(&saved).unwrap()).unwrap()).unwrap();
    for n in &numbers {
        let areas = back["entities"].as_array().unwrap().iter().filter(|e| e["kind"] == "polygon" && e["attrs"]["ParselNo"] == json!(n)).count();
        let points = back["entities"].as_array().unwrap().iter().filter(|e| e["kind"] == "point" && e["label"] == json!(n)).count();
        assert_eq!((areas, points), (1, 1), "{n}");
    }
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(saved);
}

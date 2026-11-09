//! Yüzey yapıştır through a real AI host (plan/map-cad.md MAP-14, docs/adr/3920): the independent writer's paste
//! drawing (`fixtures/kcad/v2/surface-pastes.kcad`) over MCP. Son zemin with Mevcut zemin pasted alone is Mevcut
//! zemin's triangles: the same heights inside them and its taken-out triangle made again from the pasted corners;
//! Mevcut zemin pasting it back is said (`cycle`) and pastes nothing; a surface pasting itself is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"surface-paste-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn a_pasted_surface_is_the_ground_where_it_lies_and_a_paste_back_is_said() {
    let drawing_json: Value = serde_json::from_str(include_str!("../../../../fixtures/kcad/v2/surface-pastes.json")).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/surface-pastes.kcad");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let surfaces = drawing_json["settings"]["road"]["surfaces"].as_array().unwrap().clone();

    // Son zemin with the paste alone.
    let mut son = surfaces[1].clone();
    son["edits"] = json!([{"op": "paste", "terrain": "t-zemin"}]);
    let set = call(&mut server, "road.terrains.edit", json!({"drawing": drawing, "operation": "set", "surface": son}));
    assert_eq!(set["isError"], false, "{set}");
    assert_eq!(set["structuredContent"]["output"]["surface"]["left"], Value::Null, "{set}");
    let inside = json!([[10.0, 10.0], [5.0, 25.0], [30.0, 5.0], [12.0, 20.0], [20.0, 20.0], [35.0, 27.0]]);
    let ask = |s: &mut Server, id: &str| {
        let asked = call(s, "road.terrains.query", json!({"drawing": drawing, "surface": id, "at": inside}));
        assert_eq!(asked["isError"], false, "{asked}");
        asked["structuredContent"]["output"].clone()
    };
    let (zemin, pasted) = (ask(&mut server, "t-zemin"), ask(&mut server, "t-son"));
    let figures = |o: &Value| o["surfaces"][0]["figures"].clone();
    assert_eq!(figures(&pasted)["triangles"].as_u64(), figures(&zemin)["triangles"].as_u64().map(|n| n + 1));
    assert_eq!((figures(&zemin)["area"].as_f64(), figures(&pasted)["area"].as_f64()), (Some(975.0), Some(1200.0)));
    for k in 0..4 {
        let (a, b) = (&zemin["heights"][k], &pasted["heights"][k]);
        let (za, zb) = (a["z"].as_f64().unwrap(), b["z"].as_f64().unwrap());
        assert!((za - zb).abs() < 1e-9, "{a} {b}");
    }
    // Mevcut zemin's taken-out triangle: no height there; Son zemin has one, from the pasted corners.
    for k in 4..6 {
        assert_eq!(zemin["heights"][k]["place"], "outside", "{zemin}");
        assert_eq!(pasted["heights"][k]["place"], "inside", "{pasted}");
    }

    // Mevcut zemin pasting Son zemin back: said, nothing pasted.
    let mut back = surfaces[0].clone();
    back["edits"].as_array_mut().unwrap().push(json!({"op": "paste", "terrain": "t-son"}));
    let set = call(&mut server, "road.terrains.edit", json!({"drawing": drawing, "operation": "set", "surface": back}));
    assert_eq!(set["isError"], false, "{set}");
    let report = &set["structuredContent"]["output"]["surface"];
    let said: Vec<(String, String)> = report["left"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (l["id"].as_str().unwrap().to_owned(), l["why"].as_str().unwrap().to_owned()))
        .collect();
    assert!(said.contains(&("edits[4]".into(), "cycle".into())), "{report}");
    assert_eq!(report["figures"]["triangles"], figures(&zemin)["triangles"], "pastes nothing");

    // Itself: refused, nothing written.
    son["edits"] = json!([{"op": "paste", "terrain": "t-son"}]);
    let refused = call(&mut server, "road.terrains.edit", json!({"drawing": drawing, "operation": "set", "surface": son}));
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(refused["structuredContent"]["error"]["code"], "invalid_terrain", "{refused}");
    assert_eq!(refused["structuredContent"]["error"]["path"], "surface.edits[0]", "{refused}");
}

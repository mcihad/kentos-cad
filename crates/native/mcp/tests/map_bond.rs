//! A map bond through a real AI host (plan/map-cad.md MAP-06, docs/adr/2580): the schema 208 sample (a parcel's front
//! edge kept 5 m from a road's edge) opened as a .kcad file over MCP; the road moved 2 m south, the bonded edge follows
//! and the saved file still holds the bond. Then Paralel kaydır with a kept reference writes a second bond, the
//! setback's distance, as the shared cases do.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"map-bond-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const PARCEL: &str = "0192f5a6-2580-7000-8000-000000000001";
const ROAD: &str = "0192f5a6-2580-7000-8000-000000000002";

fn saved(server: &mut Server, drawing: &Value, name: &str) -> kentos_commands::DocumentSnapshotV2 {
    let path = std::env::temp_dir().join(format!("kentos-map-bond-{name}-{}.kcad", std::process::id()));
    let written = call(server, "drawing.save", json!({"drawing": drawing, "path": path}));
    assert_eq!(written["isError"], false, "{written}");
    let snapshot = kentos_kcad::decode(&std::fs::read(&path).unwrap()).unwrap();
    let _ = std::fs::remove_file(path);
    snapshot
}

/// The parcel's corners.
fn parcel(s: &kentos_commands::DocumentSnapshotV2) -> Vec<kentos_commands::Vec2> {
    let at = s.uids.iter().position(|u| u.to_text() == PARCEL).unwrap();
    match &s.entities[at] {
        kentos_commands::Entity::Polygon(p) => p.pts.clone(),
        other => panic!("{other:?}"),
    }
}

/// The parcel's front edge's north.
fn front(s: &kentos_commands::DocumentSnapshotV2) -> f64 {
    parcel(s)[0].y
}

#[test]
fn the_bond_over_mcp_follows_the_road_and_survives_the_file() {
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/map-bond.kcad");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    // The road 2 m south: the bonded edge follows, from 5 m north of 4420005 to 5 m north of 4420003.
    let moved = call(&mut server, "cad.entities.transform", json!({
        "drawing": drawing, "uids": [ROAD], "transform": {"kind": "move", "dx": 0.0, "dy": -2.0}}));
    assert_eq!(moved["isError"], false, "{moved}");
    let after = saved(&mut server, &drawing, "moved");
    assert!((front(&after) - 4_420_008.0).abs() < 1e-6, "{}", front(&after));
    assert_eq!(after.constraints.constraints.len(), 1, "the bond is kept");
    assert!(kentos_commands::rule_is_map_bond(&after.constraints.constraints[0].rule));

    // Turned, the road would turn the edge: refused, nothing written.
    let turned = call(&mut server, "cad.entities.transform", json!({
        "drawing": drawing, "uids": [ROAD],
        "transform": {"kind": "rotate", "center": {"x": 487190.0, "y": 4420003.0}, "angle": 0.01}}));
    assert_eq!(turned["isError"], true, "{turned}");
    assert!(turned.to_string().contains("constraint_conflict"), "{turned}");
    assert!((front(&saved(&mut server, &drawing, "turned")) - 4_420_008.0).abs() < 1e-6);

    // Paralel kaydır with a kept reference: the parcel's back edge 25 m from its front edge, a second bond (d2).
    let kept = call(&mut server, "cad.shapes.edit", json!({
        "drawing": drawing,
        "work": {"kind": "edgeShift", "id": PARCEL, "at": {"x": 487210.0, "y": 4420040.0}, "distance": 25.0,
                 "reference": {"id": PARCEL, "at": {"x": 487210.0, "y": 4420008.0}, "keep": true}}}));
    assert_eq!(kept["isError"], false, "{kept}");
    assert_eq!(kept["structuredContent"]["output"]["bonds"].as_array().map(Vec::len), Some(1), "{kept}");
    let both = saved(&mut server, &drawing, "kept");
    assert_eq!(both.constraints.constraints.len(), 2);
    let names: Vec<&str> = both.constraints.parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["d1", "d2"]);
    assert_eq!(both.constraints.parameters[1].expression, "25");

    // The road again: the front edge follows the road, the back edge its front edge (a chain).
    let again = call(&mut server, "cad.entities.transform", json!({
        "drawing": drawing, "uids": [ROAD], "transform": {"kind": "move", "dx": 0.0, "dy": -1.0}}));
    assert_eq!(again["isError"], false, "{again}");
    let chained = saved(&mut server, &drawing, "chained");
    assert!((front(&chained) - 4_420_007.0).abs() < 1e-6, "{}", front(&chained));
    let back = parcel(&chained)[2].y;
    assert!((back - 4_420_032.0).abs() < 1e-6, "the back edge 25 m behind: {back}");
}

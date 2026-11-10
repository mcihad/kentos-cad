//! Yüzeyden kot ver's height over the surface and range through a real AI host (plan/map-cad.md MAP-06,
//! docs/adr/4580): the shared cases' drawing (fixtures/commands/v1/cad.entities.drape.json: the surface “Zemin”, its
//! diagonal AC) opened as a .kcad file over MCP. The point 0.15 m over its triangle's plane (worked out here from ABC);
//! the polyline's range 0–1 with break points takes its two vertices and the vertex on AC, its last vertex none; a range
//! for two objects is refused at its path. Docs/adr/4980: the two-part area's second part's hole takes its range 1–3
//! 0.15 m over the surface, with the break point where AC crosses it; the first part is left; a part it has not is
//! refused at `range.part`.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"drape-relative-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn drape_over_and_within_a_range_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.drape.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let uid = |slot: usize| cases["setup"]["uids"][slot - 1].as_str().unwrap().to_owned();
    let path = std::env::temp_dir().join(format!("kentos-drape-relative-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();

    let r = call(&mut server, "cad.entities.drape", json!({"drawing": drawing, "uids": [uid(5)], "surface": "Zemin", "offset": 0.15}));
    assert_eq!(r["isError"], false, "{r}");
    // ABC: z = 100 + 0.04 x + 0.1 y at (60, 20), 0.15 m over it.
    let point = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid(5)}));
    let z = point["structuredContent"]["entity"]["z"].as_f64().unwrap();
    assert!((z - (100.0 + 0.04 * 60.0 + 0.1 * 20.0 + 0.15)).abs() < 1e-9, "{z}");

    let r = call(
        &mut server,
        "cad.entities.drape",
        json!({"drawing": drawing, "uids": [uid(8)], "surface": "t-1", "breaks": true, "range": {"from": 0, "to": 1}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["set"].as_u64(), out["inserted"].as_u64()), (Some(2), Some(1)), "{out}");
    let line = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid(8)}));
    let zs = line["structuredContent"]["entity"]["zs"].as_array().unwrap().clone();
    assert_eq!(zs.len(), 4);
    assert!(zs[3].is_null(), "the last vertex is out of the range: {zs:?}");

    let r = call(
        &mut server,
        "cad.entities.drape",
        json!({"drawing": drawing, "uids": [uid(8), uid(9)], "surface": "t-1", "range": {"from": 0, "to": 1}}),
    );
    assert_eq!(r["structuredContent"]["error"]["path"], "range", "{r}");
}

#[test]
fn drape_a_range_on_a_further_parts_hole_over_mcp() {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.drape.json")).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).unwrap();
    let uid = |slot: usize| cases["setup"]["uids"][slot - 1].as_str().unwrap().to_owned();
    let path = std::env::temp_dir().join(format!("kentos-drape-part-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let mut server = Server::new();
    let open = call(&mut server, "drawing.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    assert_eq!(open["isError"], false, "{open}");
    let drawing = open["structuredContent"]["drawing"].clone();

    // The area (slot 18): its second part's hole (65, 35), (85, 35), (85, 65), (65, 65), vertices 1 to 3.
    let r = call(
        &mut server,
        "cad.entities.drape",
        json!({"drawing": drawing, "uids": [uid(18)], "surface": "Zemin", "breaks": true, "offset": 0.15,
               "range": {"from": 1, "to": 3, "part": 1, "ring": 1}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["set"].as_u64(), out["inserted"].as_u64()), (Some(3), Some(1)), "{out}");
    let area = call(&mut server, "drawing.entity", json!({"drawing": drawing, "uid": uid(18)}));
    let e = &area["structuredContent"]["entity"];
    let zs: Vec<Option<f64>> = e["parts"][0]["holes"][0]["zs"].as_array().unwrap().iter().map(Value::as_f64).collect();
    // ABC (z = 100 + 0.04 x + 0.1 y) holds (85, 35) and (85, 65); ACD (z = 100 + 0.072 x + 0.06 y) holds (65, 65); AC
    // (y = 0.8 x) crosses the edge y = 65 at x = 81.25.
    let abc = |x: f64, y: f64| 100.0 + 0.04 * x + 0.1 * y + 0.15;
    let acd = |x: f64, y: f64| 100.0 + 0.072 * x + 0.06 * y + 0.15;
    let want = [None, Some(abc(85.0, 35.0)), Some(abc(85.0, 65.0)), Some(abc(81.25, 65.0)), Some(acd(65.0, 65.0))];
    assert_eq!(zs.len(), want.len(), "{zs:?}");
    for (got, want) in zs.iter().zip(want) {
        match (got, want) {
            (Some(g), Some(w)) => assert!((g - w).abs() < 1e-9, "{zs:?}"),
            (g, w) => assert_eq!(*g, w, "{zs:?}"),
        }
    }
    assert!(e["zs"].is_null() && e["parts"][0]["zs"].is_null(), "the other rings are left: {e}");

    let r = call(
        &mut server,
        "cad.entities.drape",
        json!({"drawing": drawing, "uids": [uid(18)], "surface": "t-1", "range": {"from": 0, "to": 1, "part": 2}}),
    );
    assert_eq!(r["structuredContent"]["error"]["path"], "range.part", "{r}");
}

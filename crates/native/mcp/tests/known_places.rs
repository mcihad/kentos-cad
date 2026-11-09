//! Each known point's own place in the project through a real AI host (plan/map-cad.md MAP-03, docs/adr/4120): the
//! `.kcad` schema 207 sample (fixtures/kcad/v2/map-survey-velocity.kcad: ANKR published at 2005.0 with its velocity, N1's
//! old C3 coordinates at 2026.84 without one, the campaign deciding 2005.0 without a velocity grid) opened over MCP.
//! ANKR's place is `cad.crs.transform`'s for its own coordinates; N1's has none and says why; a known point added on N3
//! at 2026.84 with its velocity goes back by V·Δt (its east, north, up worked out here) and is compared with N3's
//! accepted point. Without `places`, nothing of it.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"known-places-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_known_points_places_over_mcp() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": root.join("map-survey-velocity.kcad")}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let places = |server: &mut Server| {
        let r = call(server, "map.survey.query", json!({"drawing": drawing, "places": true}));
        assert_eq!(r["isError"], false, "{r}");
        r["structuredContent"]["output"]["knownPlaces"].clone()
    };
    let rows = places(&mut server);
    let (ankr, n1) = (&rows[0], &rows[1]);
    assert_eq!((ankr["mark"].as_str(), ankr["epoch"].as_f64()), (Some("ANKR"), Some(2005.0)), "{rows}");
    assert!(ankr.get("moved").is_none(), "at the decision: {ankr}");
    // The chain's own answer for ANKR's coordinates, asked as a separate call.
    let t = call(
        &mut server,
        "cad.crs.transform",
        json!({"drawing": drawing, "points": [{"x": 32.758565, "y": 39.88745, "z": 976.012}],
               "from": {"srid": 5252, "epoch": 2005.0, "heights": {"kind": "ellipsoidal"}}, "to": {"epoch": 2005.0}}),
    );
    assert_eq!(t["isError"], false, "{t}");
    let p = &t["structuredContent"]["output"]["points"][0];
    for k in ["x", "y", "z"] {
        let (a, b) = (ankr[k].as_f64().unwrap(), p[k].as_f64().unwrap());
        assert!((a - b).abs() < 1e-9, "{k}: {a} ≠ {b}");
    }
    // N1's old coordinates at 2026.84: no velocity, no grid.
    assert!(n1.get("x").is_none(), "{n1}");
    assert_eq!(n1["point"], "kn-1");
    assert!(n1["why"].as_str().unwrap().contains("2026,84"), "{n1}");

    // A known point on N3 at 2026.84 with its own velocity goes back to 2005.0 by V·Δt.
    let (lat, lon, v) = (39.79, 30.61, [-0.0105, 0.0098, 0.0112]);
    let added = call(
        &mut server,
        "map.survey.edit",
        json!({"drawing": drawing, "operation": {"op": "known", "mark": "N3", "lat": lat, "lon": lon, "height": 850.0,
               "srid": 5252, "epoch": 2026.84, "velocity": v}}),
    );
    assert_eq!(added["isError"], false, "{added}");
    let rows = places(&mut server);
    let n3 = rows.as_array().unwrap().iter().find(|r| r["mark"] == "N3").unwrap();
    let moved = &n3["moved"];
    assert_eq!(moved["by"], "velocity", "{n3}");
    let years = 2005.0 - 2026.84;
    let (f, l) = (f64::to_radians(lat), f64::to_radians(lon));
    let rows_enu = [
        [-l.sin(), l.cos(), 0.0],
        [-f.sin() * l.cos(), -f.sin() * l.sin(), f.cos()],
        [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()],
    ];
    for (i, row) in rows_enu.iter().enumerate() {
        let want: f64 = (0..3).map(|j| row[j] * v[j] * years).sum();
        let got = moved["shift"][i].as_f64().unwrap();
        assert!((got - want).abs() < 1e-6, "shift[{i}]: {got} ≠ {want}");
    }
    assert_eq!(n3["point"], "kn-2");
    assert!(n3["dx"].is_number() && n3["dy"].is_number(), "compared with N3's accepted point: {n3}");

    // Without `places`, nothing of it.
    let r = call(&mut server, "map.survey.query", json!({"drawing": drawing}));
    assert!(r["structuredContent"]["output"].get("knownPlaces").is_none(), "{r}");
}

//! A known point's own velocity through a real AI host (plan/map-cad.md MAP-03, docs/adr/2520): the `.kcad` schema 207
//! sample (fixtures/kcad/v2/map-survey-velocity.kcad: the network's held TUSAGA-Aktif point at 2005.0 with its
//! Earth-centred velocity, the campaign without a velocity grid) opened over MCP. Adjusted at a measurement epoch, the
//! point goes there by its velocity: `moved` says `velocity` and the shift is V·Δt in its east, north, up (worked out
//! here from the file's numbers); without the epoch nothing moves. A known point added with a velocity and no epoch is
//! refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"known-velocity-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_schema_207_sample_moves_its_known_point_by_its_velocity_over_mcp() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2");
    let sample: Value = serde_json::from_str(&std::fs::read_to_string(root.join("map-survey-velocity.json")).unwrap()).unwrap();
    let k = &sample["settings"]["mapSurvey"]["known"][0];
    let (lat, lon) = (k["lat"].as_f64().unwrap().to_radians(), k["lon"].as_f64().unwrap().to_radians());
    let v: Vec<f64> = k["velocity"].as_array().unwrap().iter().map(|c| c.as_f64().unwrap()).collect();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": root.join("map-survey-velocity.kcad")}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let r = call(&mut server, "map.survey.network", json!({"drawing": drawing, "epoch": 2026.84}));
    assert_eq!(r["isError"], false, "{r}");
    let moved = &r["structuredContent"]["output"]["moved"];
    assert_eq!(moved.as_array().map(Vec::len), Some(1), "{r}");
    let m = &moved[0];
    assert_eq!((m["id"].as_str(), m["by"].as_str()), (Some("bn-1"), Some("velocity")));
    let years = 2026.84 - 2005.0;
    assert!((m["years"].as_f64().unwrap() - years).abs() < 1e-12);
    // V·Δt in the point's east, north, up.
    let d: Vec<f64> = v.iter().map(|c| c * years).collect();
    let rows = [
        [-lon.sin(), lon.cos(), 0.0],
        [-lat.sin() * lon.cos(), -lat.sin() * lon.sin(), lat.cos()],
        [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()],
    ];
    for (i, row) in rows.iter().enumerate() {
        let want: f64 = (0..3).map(|j| row[j] * d[j]).sum();
        let got = m["shift"][i].as_f64().unwrap();
        assert!((got - want).abs() < 1e-6, "shift[{i}]: {got} ≠ {want}");
    }

    // Without a measurement epoch the baselines' times decide; one baseline has none, so nothing moves.
    let r = call(&mut server, "map.survey.network", json!({"drawing": drawing}));
    assert_eq!(r["isError"], false, "{r}");
    assert!(r["structuredContent"]["output"].get("moved").is_none(), "{r}");

    // A velocity without an epoch is refused.
    let r = call(
        &mut server,
        "map.survey.edit",
        json!({"drawing": drawing, "operation": {"op": "known", "mark": "N3", "lat": 39.8, "lon": 30.6, "height": 850.0,
               "srid": 5252, "velocity": [-0.0105, 0.0098, 0.0112]}}),
    );
    let error = &r["structuredContent"]["error"];
    assert_eq!((error["code"].as_str(), error["path"].as_str()), (Some("invalid_survey_value"), Some("operation.velocity")), "{r}");
}

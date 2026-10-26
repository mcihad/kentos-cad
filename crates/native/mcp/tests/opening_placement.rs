//! The new semantic inputs pass through MCP discovery and the real headless host.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn request(server: &mut Server, method: &str, mut params: Value) -> Value {
    params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"architecture-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
    server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .unwrap()["result"]
        .clone()
}
fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let r = request(server, "tools/call", json!({"name":name,"arguments":args}));
    assert_eq!(r["resultType"], "complete", "{r}");
    r
}

#[test]
fn opening_jamb_dimension_plan_follow_history_and_file_use_the_same_agent_commands() {
    let mut s = Server::new();
    let new = call(&mut s, "drawing.new", json!({"workspace":"cad"}));
    let drawing = new["structuredContent"]["drawing"].clone();
    let layer = new["structuredContent"]["summary"]["activeLayer"].clone();
    let wall = call(
        &mut s,
        "cad.walls.create",
        json!({"drawing":drawing,"layerId":layer,"walls":[{"pts":[{"x":0,"y":0},{"x":10,"y":0}],"width":0.2,"closed":false}]}),
    );
    assert_eq!(wall["isError"], false, "{wall}");
    let host = wall["structuredContent"]["output"]["uids"][0].clone();
    let opening = call(
        &mut s,
        "cad.openings.create",
        json!({"drawing":drawing,"layerId":layer,"openings":[{"host":host,"form":"window","at":4,"width":1}]}),
    );
    assert_eq!(opening["isError"], false, "{opening}");
    let uid = opening["structuredContent"]["output"]["uids"][0].clone();
    let model = |s: &mut Server, drawing: &Value| {
        let mut rows = call(s, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]
            ["items"]
            .clone();
        for row in rows.as_array_mut().unwrap() {
            // Stamps are host concurrency tokens; UUID and every entity field stay.
            row.as_object_mut().unwrap().remove("stamp");
        }
        rows
    };
    let before = model(&mut s, &drawing);
    for (index, x, y) in [(0, 3.5, 0.1), (1, 3.5, -0.1), (2, 4.5, 0.1), (3, 4.5, -0.1)] {
        for all in [false, true] {
            let found = call(
                &mut s,
                "cad.snap.find",
                json!({"drawing":drawing,"at":{"x":x,"y":y},"aperture":0.0001,"kinds":["endpoint"],"all":all}),
            );
            assert_eq!(found["isError"], false, "{found}");
            let hit = &found["structuredContent"]["output"]["snaps"][0];
            assert_eq!(hit["uid"], uid);
            assert_eq!(hit["point"], json!({"x":x,"y":y}));
            assert_eq!(
                hit["place"],
                json!({"kind":"vertex","part":0,"ring":0,"index":index})
            );
        }
    }
    assert_eq!(model(&mut s, &drawing), before);
    let mut args = json!({"drawing":drawing,"layerId":layer,"op":"plan","at":{"x":4,"y":-1},"measure":{"kind":"aligned","a":{"kind":"place","uid":uid,"place":{"kind":"vertex","part":0,"ring":0,"index":0}},"b":{"kind":"place","uid":uid,"place":{"kind":"vertex","part":0,"ring":0,"index":2}}}});
    let plan = call(&mut s, "cad.dimensions.create", args.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(model(&mut s, &drawing), before);
    assert_eq!(
        plan["structuredContent"]["output"]["entities"][0]["a"],
        json!({"x":3.5,"y":0.1})
    );
    args["op"] = json!("execute");
    args["expectedRevision"] = plan["structuredContent"]["output"]["revision"].clone();
    let made = call(&mut s, "cad.dimensions.create", args.clone());
    assert_eq!(made["isError"], false, "{made}");
    let dim = made["structuredContent"]["output"]["created"][0].clone();
    let created = model(&mut s, &drawing);
    assert_eq!(
        &created.as_array().unwrap()[..2],
        before.as_array().unwrap()
    );
    let dimension = |rows: &Value| {
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["uid"] == dim)
            .unwrap()["entity"]
            .clone()
    };
    assert_eq!(dimension(&created)["b"], json!({"x":4.5,"y":0.1}));
    for key in ["a", "b"] {
        assert_eq!(dimension(&created)["refs"][key]["uid"], uid);
    }
    call(&mut s, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(model(&mut s, &drawing), before);
    call(&mut s, "drawing.redo", json!({"drawing":drawing}));
    assert_eq!(model(&mut s, &drawing), created);
    let changed = call(
        &mut s,
        "cad.openings.edit",
        json!({"drawing":drawing,"uids":[uid],"operation":{"kind":"set","width":2}}),
    );
    assert_eq!(changed["isError"], false, "{changed}");
    let wide = model(&mut s, &drawing);
    assert_eq!(wide[0], before[0]);
    let mut expected_dim = dimension(&created);
    expected_dim["a"] = json!({"x":3.0,"y":0.1});
    expected_dim["b"] = json!({"x":5.0,"y":0.1});
    assert_eq!(dimension(&wide), expected_dim);
    call(&mut s, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(model(&mut s, &drawing), created);
    call(&mut s, "drawing.redo", json!({"drawing":drawing}));
    assert_eq!(model(&mut s, &drawing), wide);
    let path = std::env::temp_dir().join(format!("kentos-opening-dim-{}.kcad", std::process::id()));
    assert_eq!(
        call(
            &mut s,
            "drawing.save",
            json!({"drawing":drawing,"path":path})
        )["isError"],
        false
    );
    let back =
        call(&mut s, "drawing.open", json!({"path":path}))["structuredContent"]["drawing"].clone();
    assert_eq!(model(&mut s, &back), wide);
    let deleted = call(
        &mut s,
        "cad.entities.delete",
        json!({"drawing":drawing,"uids":[host]}),
    );
    assert_eq!(deleted["isError"], false, "{deleted}");
    let broken = model(&mut s, &drawing);
    for key in ["a", "b"] {
        assert_eq!(dimension(&broken)["refs"][key]["broken"], "no_such_place");
    }
    args.as_object_mut().unwrap().remove("expectedRevision");
    let refused = call(&mut s, "cad.dimensions.create", args);
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "no_such_place"
    );
    assert_eq!(model(&mut s, &drawing), broken);
    call(&mut s, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(model(&mut s, &drawing), wide);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn precision_placement_is_discoverable_and_an_agent_gets_the_same_geometry_and_atomic_refusal() {
    let mut s = Server::new();
    let listed = request(&mut s, "tools/list", json!({}));
    let tool = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "cad.openings.create")
        .unwrap();
    let fields = &tool["inputSchema"]["$defs"]["NewOpening"]["properties"];
    assert!(fields["span"].is_object() && fields["distanceReference"].is_object());
    let d = call(&mut s, "drawing.new", json!({"workspace":"cad"}));
    let drawing = d["structuredContent"]["drawing"].clone();
    let layer = d["structuredContent"]["summary"]["activeLayer"].clone();
    let wall = call(
        &mut s,
        "cad.walls.create",
        json!({"drawing":drawing,"layerId":layer,"walls":[{"pts":[{"x":0,"y":0},{"x":10,"y":0}],"width":0.2,"closed":false}]}),
    );
    let uid = wall["structuredContent"]["output"]["uids"][0].clone();
    let mut input = json!({"drawing":drawing,"layerId":layer,"op":"plan","openings":[{"host":uid,"point":{"x":4,"y":0.2},"width":1,"distance":2,"from":"end","distanceReference":"hinge","hand":"start"}]});
    let plan = call(&mut s, "cad.openings.create", input.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert_eq!(
        plan["structuredContent"]["output"]["openings"][0]["at"],
        8.5
    );
    let summary = call(&mut s, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(summary["structuredContent"]["objects"], 1);
    input["op"] = json!("execute");
    let made = call(&mut s, "cad.openings.create", input);
    assert_eq!(made["isError"], false, "{made}");
    let mut input = json!({"drawing":drawing,"layerId":layer,"op":"plan","openings":[{"host":uid,"form":"window","span":[{"x":2,"y":0.2},{"x":3.2,"y":0.2}]}]});
    let plan = call(&mut s, "cad.openings.create", input.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    assert!(
        (plan["structuredContent"]["output"]["openings"][0]["width"]
            .as_f64()
            .unwrap()
            - 1.2)
            .abs()
            < 1e-12
    );
    input["op"] = json!("execute");
    let made = call(&mut s, "cad.openings.create", input.clone());
    assert_eq!(made["isError"], false, "{made}");
    call(&mut s, "drawing.undo", json!({"drawing":drawing}));
    let summary = call(&mut s, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(summary["structuredContent"]["objects"], 2);
    call(&mut s, "drawing.redo", json!({"drawing":drawing}));
    input["openings"][0]["width"] = json!(1);
    let refused = call(&mut s, "cad.openings.create", input);
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_opening"
    );
    let summary = call(&mut s, "drawing.summary", json!({"drawing":drawing}));
    assert_eq!(summary["structuredContent"]["objects"], 3);
}

//! Real MCP host: measured source recipes, stable children, atomic history and persistence.

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn request(server: &mut Server, method: &str, mut params: Value) -> Value {
    params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"dimension-set-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities":{}});
    server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .unwrap()["result"]
        .clone()
}
fn tool(server: &mut Server, name: &str, args: Value) -> Value {
    let result = request(server, "tools/call", json!({"name":name,"arguments":args}));
    assert_eq!(result["resultType"], "complete", "{name}: {result}");
    assert_eq!(result["isError"], false, "{name}: {result}");
    result["structuredContent"].clone()
}
fn output(server: &mut Server, name: &str, args: Value) -> Value {
    tool(server, name, args)["output"].clone()
}
fn items(server: &mut Server, drawing: &Value) -> Value {
    let mut rows = tool(server, "drawing.entities", json!({"drawing":drawing}))["items"].clone();
    for row in rows.as_array_mut().unwrap() {
        row.as_object_mut().unwrap().remove("stamp");
        // KCAD compacts file-local slots; persistent UUID and every meaningful
        // entity field must survive. A local slot is not a cloud/object identity.
        row["entity"].as_object_mut().unwrap().remove("id");
    }
    rows
}
fn sets(server: &mut Server, drawing: &Value) -> Value {
    output(
        server,
        "cad.dimensionsets.query",
        json!({"drawing":drawing}),
    )["sets"]
        .clone()
}
fn spans(server: &mut Server, drawing: &Value) -> Vec<f64> {
    let mut lengths: Vec<_> = items(server, drawing)
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| {
            let e = &r["entity"];
            (e["kind"] == "dimension").then(|| {
                let dx = e["b"]["x"].as_f64().unwrap() - e["a"]["x"].as_f64().unwrap();
                let dy = e["b"]["y"].as_f64().unwrap() - e["a"]["y"].as_f64().unwrap();
                dx.hypot(dy)
            })
        })
        .collect();
    lengths.sort_by(f64::total_cmp);
    lengths
}
/// Independent 10 m source, 2 m opening at 4 m, left face y=.1.
fn assert_saved_source_geometry(
    server: &mut Server,
    drawing: &Value,
    wall: &Value,
    opening: &Value,
) {
    let rows = items(server, drawing);
    let rows = rows.as_array().unwrap();
    let entity = |uid: &Value| &rows.iter().find(|row| row["uid"] == *uid).unwrap()["entity"];
    assert_eq!(
        entity(wall)["pts"],
        json!([{"x":0.,"y":0.},{"x":10.,"y":0.}])
    );
    assert_eq!(entity(wall)["width"].as_f64(), Some(0.2));
    assert_eq!(entity(opening)["host"], *wall);
    assert_eq!(entity(opening)["at"].as_f64(), Some(4.));
    assert_eq!(entity(opening)["width"].as_f64(), Some(2.));
    assert_eq!(
        entity(opening)["place"],
        json!([{"x":3.,"y":0.1},{"x":3.,"y":-0.1},
               {"x":5.,"y":0.1},{"x":5.,"y":-0.1}])
    );
    let recipes = sets(server, drawing);
    for (member, (a, b, source_a, source_b)) in
        recipes[0]["members"].as_array().unwrap().iter().zip([
            (0., 3., wall, opening),
            (3., 5., opening, opening),
            (5., 10., opening, wall),
            (0., 10., wall, wall),
        ])
    {
        let dimension = entity(&member["uid"]);
        assert_eq!(dimension["a"], json!({"x":a,"y":0.1}));
        assert_eq!(dimension["b"], json!({"x":b,"y":0.1}));
        assert_eq!(member["a"]["uid"], *source_a);
        assert_eq!(member["b"]["uid"], *source_b);
        assert_eq!(dimension["refs"]["a"], member["a"]);
        assert_eq!(dimension["refs"]["b"], member["b"]);
    }
}

fn member_map(set: &Value) -> BTreeMap<String, Value> {
    set["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                serde_json::to_string(&json!([m["row"], m["a"], m["b"]])).unwrap(),
                m["uid"].clone(),
            )
        })
        .collect()
}
fn setup(server: &mut Server) -> (Value, Value, Value, Value) {
    let new = tool(server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = new["drawing"].clone();
    let layer = new["summary"]["activeLayer"].clone();
    let wall = output(
        server,
        "cad.walls.create",
        json!({"drawing":drawing,"layerId":layer,
        "walls":[{"pts":[{"x":0,"y":0},{"x":10,"y":0}],"width":0.2,"closed":false}]}),
    )["uids"][0]
        .clone();
    let opening = output(
        server,
        "cad.openings.create",
        json!({"drawing":drawing,"layerId":layer,
        "openings":[{"host":wall,"at":4,"width":1}]}),
    )["uids"][0]
        .clone();
    (drawing, layer, wall, opening)
}
fn create(server: &mut Server, drawing: &Value, layer: &Value, wall: &Value) -> Value {
    output(
        server,
        "cad.dimensionsets.create",
        json!({"drawing":drawing,"layerId":layer,
        "wall":wall,"face":"left","at":{"x":4,"y":1},"gapMm":7.5}),
    )
}

#[test]
fn discovery_readonly_plan_numeric_sources_stable_members_and_save_open() {
    let mut server = Server::new();
    let list = request(&mut server, "tools/list", json!({}));
    for name in [
        "cad.dimensionsets.create",
        "cad.dimensionsets.edit",
        "cad.dimensionsets.query",
    ] {
        let schema = &list["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap()["inputSchema"];
        assert!(schema.is_object(), "{name}: {schema}");
    }
    let (drawing, layer, wall, opening) = setup(&mut server);
    let before = items(&mut server, &drawing);
    let summary = tool(&mut server, "drawing.summary", json!({"drawing":drawing}));
    let mut args =
        json!({"drawing":drawing,"layerId":layer,"wall":wall,"at":{"x":4,"y":1},"op":"plan"});
    let plan = output(&mut server, "cad.dimensionsets.create", args.clone());
    assert_eq!(plan["entities"].as_array().unwrap().len(), 4);
    assert_eq!(
        plan["uids"],
        json!([]),
        "a plan allocates no persistent children"
    );
    assert_eq!(items(&mut server, &drawing), before);
    assert_eq!(
        tool(&mut server, "drawing.summary", json!({"drawing":drawing})),
        summary
    );
    assert_eq!(sets(&mut server, &drawing), json!([]));
    args["op"] = json!("execute");
    args["expectedRevision"] = plan["revision"].clone();
    let made = output(&mut server, "cad.dimensionsets.create", args);
    assert_eq!(made["created"].as_array().unwrap().len(), 4);
    assert_eq!(spans(&mut server, &drawing), vec![1., 3.5, 5.5, 10.]);
    let original = sets(&mut server, &drawing);
    let original_items = items(&mut server, &drawing);
    let initial = member_map(&original[0]);
    assert_eq!(
        output(
            &mut server,
            "cad.dimensionsets.query",
            json!({"drawing":drawing,"wall":wall})
        )["sets"],
        original
    );
    assert_eq!(
        output(
            &mut server,
            "cad.dimensionsets.query",
            json!({"drawing":drawing,"ids":[made["set"]]})
        )["sets"],
        original
    );
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(sets(&mut server, &drawing), json!([]));
    assert_eq!(items(&mut server, &drawing), before);
    tool(&mut server, "drawing.redo", json!({"drawing":drawing}));
    assert_eq!(items(&mut server, &drawing), original_items);
    assert_eq!(sets(&mut server, &drawing), original);
    let before_width = items(&mut server, &drawing);
    let width_plan = output(
        &mut server,
        "cad.openings.edit",
        json!({"drawing":drawing,"uids":[opening],"op":"plan","operation":{"kind":"set","width":2}}),
    );
    assert!(
        width_plan["followers"]["updated"]
            .as_array()
            .is_some_and(|v| !v.is_empty()),
        "{width_plan}"
    );
    assert_eq!(items(&mut server, &drawing), before_width);
    output(
        &mut server,
        "cad.openings.edit",
        json!({"drawing":drawing,"uids":[opening],"operation":{"kind":"set","width":2}}),
    );
    assert_eq!(spans(&mut server, &drawing), vec![2., 3., 5., 10.]);
    assert_eq!(member_map(&sets(&mut server, &drawing)[0]), initial);
    let width_items = items(&mut server, &drawing);
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(items(&mut server, &drawing), original_items);
    tool(&mut server, "drawing.redo", json!({"drawing":drawing}));
    assert_eq!(items(&mut server, &drawing), width_items);
    let added = output(
        &mut server,
        "cad.openings.create",
        json!({"drawing":drawing,"layerId":layer,
        "openings":[{"host":wall,"at":7,"width":1}]}),
    )["uids"][0]
        .clone();
    assert_eq!(
        spans(&mut server, &drawing),
        vec![1., 1.5, 2., 2.5, 3., 10.]
    );
    let expanded = sets(&mut server, &drawing);
    let new_ids = member_map(&expanded[0]);
    for (segment, uid) in &initial {
        if let Some(current) = new_ids.get(segment) {
            assert_eq!(current, uid);
        }
    }
    output(
        &mut server,
        "cad.entities.delete",
        json!({"drawing":drawing,"uids":[added]}),
    );
    assert_eq!(spans(&mut server, &drawing), vec![2., 3., 5., 10.]);
    let saved_sets = sets(&mut server, &drawing);
    let saved_items = items(&mut server, &drawing);
    let dir = std::env::temp_dir().join(format!(
        "kentos-mcp-dimension-{}-{}",
        std::process::id(),
        made["set"].as_str().unwrap()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dimensions.kcad");
    tool(
        &mut server,
        "drawing.save",
        json!({"drawing":drawing,"path":path}),
    );
    let reopened = tool(&mut server, "drawing.open", json!({"path":path}))["drawing"].clone();
    assert_eq!(sets(&mut server, &reopened), saved_sets);
    assert_eq!(items(&mut server, &reopened), saved_items);
    assert_saved_source_geometry(&mut server, &reopened, &wall, &opening);
    // Source deletion retains both the recipe warning and its last measurable geometry.
    output(
        &mut server,
        "cad.entities.delete",
        json!({"drawing":drawing,"uids":[wall]}),
    );
    assert_eq!(sets(&mut server, &drawing)[0]["broken"], "hostGone");
    assert_eq!(spans(&mut server, &drawing), vec![2., 3., 5., 10.]);
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(sets(&mut server, &drawing), saved_sets);
    assert_eq!(items(&mut server, &drawing), saved_items);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn row_placement_text_reset_suppression_detach_and_delete_are_undoable() {
    let mut server = Server::new();
    let (drawing, layer, wall, _) = setup(&mut server);
    let made = create(&mut server, &drawing, &layer, &wall);
    let id = made["set"].clone();
    let original = sets(&mut server, &drawing);
    let initial_ids = member_map(&original[0]);
    let member = original[0]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["row"] == "overall")
        .unwrap()["uid"]
        .clone();
    let baseline = items(&mut server, &drawing);
    output(
        &mut server,
        "cad.dimensionsets.edit",
        json!({"drawing":drawing,"id":id,"operation":{"kind":"move","row":"overall","at":{"x":4,"y":3}}}),
    );
    assert_eq!(
        sets(&mut server, &drawing)[0]["rowPlaces"][0],
        json!({"row":"overall","at":{"x":4.,"y":3.}})
    );
    assert_eq!(member_map(&sets(&mut server, &drawing)[0]), initial_ids);
    output(
        &mut server,
        "cad.dimensionsets.edit",
        json!({"drawing":drawing,"id":id,"operation":{"kind":"resetRows"}}),
    );
    assert!(sets(&mut server, &drawing)[0]["rowPlaces"].is_null());
    assert_eq!(items(&mut server, &drawing), baseline);
    output(
        &mut server,
        "cad.dimensions.edit",
        json!({"drawing":drawing,"edit":{"kind":"moveText","uid":member,"along":0.3,"across":0.2}}),
    );
    let shifted = items(&mut server, &drawing);
    assert!(shifted.as_array().unwrap().iter().any(
        |r| r["uid"] == member && r["entity"]["textShift"] == json!({"along":0.3,"across":0.2})
    ));
    output(
        &mut server,
        "cad.dimensionsets.edit",
        json!({"drawing":drawing,"id":id,"operation":{"kind":"resetText","row":"overall"}}),
    );
    assert_eq!(items(&mut server, &drawing), baseline);
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(items(&mut server, &drawing), shifted);
    tool(&mut server, "drawing.redo", json!({"drawing":drawing}));
    assert_eq!(items(&mut server, &drawing), baseline);
    let hidden = original[0]["members"][0]["uid"].clone();
    output(
        &mut server,
        "cad.entities.delete",
        json!({"drawing":drawing,"uids":[hidden]}),
    );
    let suppressed = sets(&mut server, &drawing);
    assert_eq!(suppressed[0]["members"].as_array().unwrap().len(), 3);
    assert_eq!(suppressed[0]["suppressed"].as_array().unwrap().len(), 1);
    output(
        &mut server,
        "cad.dimensionsets.edit",
        json!({"drawing":drawing,"id":id,"operation":{"kind":"restoreHidden"}}),
    );
    assert_eq!(
        sets(&mut server, &drawing)[0]["members"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let complete = items(&mut server, &drawing);
    let complete_sets = sets(&mut server, &drawing);
    output(
        &mut server,
        "cad.dimensionsets.edit",
        json!({"drawing":drawing,"id":id,"operation":{"kind":"detach"}}),
    );
    assert_eq!(sets(&mut server, &drawing), json!([]));
    assert_eq!(items(&mut server, &drawing), complete);
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(sets(&mut server, &drawing), complete_sets);
    let deleted = output(
        &mut server,
        "cad.dimensionsets.edit",
        json!({"drawing":drawing,"id":id,"operation":{"kind":"delete"}}),
    );
    assert_eq!(deleted["removed"].as_array().unwrap().len(), 4);
    assert_eq!(sets(&mut server, &drawing), json!([]));
    assert_eq!(items(&mut server, &drawing).as_array().unwrap().len(), 2);
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(sets(&mut server, &drawing), complete_sets);
    assert_eq!(items(&mut server, &drawing), complete);
}

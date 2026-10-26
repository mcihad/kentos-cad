//! Array count/spacing is exposed as semantic input; agents never synthesize geometry.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};
fn request(server: &mut Server, method: &str, mut params: Value) -> Value {
    params["_meta"] = json!({"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"architecture-array-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}});
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
fn array_schema_plan_write_undo_and_number_conflict_use_the_real_host() {
    let mut s = Server::new();
    let listed = request(&mut s, "tools/list", json!({}));
    let tool = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "cad.openings.edit")
        .unwrap();
    assert!(tool["inputSchema"]["$defs"]["OpeningArrayLayout"].is_object());
    assert!(tool["inputSchema"]["$defs"]["OpeningArrayNumbering"].is_object());
    let d = call(&mut s, "drawing.new", json!({"workspace":"cad"}));
    let drawing = d["structuredContent"]["drawing"].clone();
    let layer = d["structuredContent"]["summary"]["activeLayer"].clone();
    let host = call(&mut s,"cad.walls.create",json!({"drawing":drawing,"layerId":layer,"walls":[{"pts":[{"x":0,"y":0},{"x":20,"y":0}],"width":0.2,"closed":false}]}))["structuredContent"]["output"]["uids"][0].clone();
    let source = call(&mut s,"cad.openings.create",json!({"drawing":drawing,"layerId":layer,"openings":[{"host":host,"at":1,"width":1,"number":"K1"}]}))["structuredContent"]["output"]["uids"][0].clone();
    let mut args = json!({"drawing":drawing,"uids":[source],"op":"plan","operation":{"kind":"array","layout":{"kind":"spacing","count":3,"distance":1,"reference":"clear","direction":"end"},"numbering":{"prefix":"K","start":2,"step":1}}});
    let plan = call(&mut s, "cad.openings.edit", args.clone());
    assert_eq!(plan["isError"], false, "{plan}");
    let output = &plan["structuredContent"]["output"];
    let added = output["added"].as_array().unwrap();
    assert_eq!(added.len(), 3);
    for (i, o) in added.iter().enumerate() {
        assert_eq!(o["at"], json!(3. + i as f64 * 2.));
        assert_eq!(o["number"], json!(format!("K{}", i + 2)));
    }
    assert_eq!(
        call(&mut s, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        2
    );
    args["op"] = json!("execute");
    args["expectedRevision"] = output["revision"].clone();
    let made = call(&mut s, "cad.openings.edit", args.clone());
    assert_eq!(made["isError"], false, "{made}");
    assert_eq!(
        call(&mut s, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        5
    );
    call(&mut s, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        call(&mut s, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        2
    );
    call(&mut s, "drawing.redo", json!({"drawing":drawing}));
    args.as_object_mut().unwrap().remove("expectedRevision");
    args["operation"]["layout"] = json!({"kind":"fit","count":1,"startMargin":10,"endMargin":0});
    let refused = call(&mut s, "cad.openings.edit", args);
    assert_eq!(refused["isError"], true, "{refused}");
    assert_eq!(
        refused["structuredContent"]["error"]["code"],
        "invalid_opening"
    );
    assert_eq!(
        call(&mut s, "drawing.summary", json!({"drawing":drawing}))["structuredContent"]["objects"],
        5
    );
}

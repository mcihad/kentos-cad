//! Real MCP host (docs/adr/0720): an agent gives every wall its level explicitly (an unknown one refused), the level
//! a wall names stays with the objects' ids said, and the level and view a drawing opens with go into its file.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"active-level-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}}))
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

#[test]
fn levels_are_explicit_kept_while_named_and_the_opening_level_is_saved() {
    let levels = json!([{"id":"zemin","name":"Zemin kat","elevation":0.0,"height":3.0},
                        {"id":"kat-1","name":"1. kat","elevation":3.0,"height":3.0}]);
    let doc = json!({"format":"kentos.document","version":1,"name":"Katlar",
        "settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50.0,
                    "workspace":"cad","arch":{"levels":levels}},
        "origin":{"x":0.0,"y":0.0},
        "layers":[{"id":"duvar","name":"Duvar","type":"layer","visible":true,"locked":false,"expanded":true,
                   "style":{"color":"fg","lineType":"continuous","lineWeight":0.25},"children":[]}],
        "activeLayer":"duvar","entities":[],"styles":{"items":[],"categories":[]}});
    let dir = std::env::temp_dir().join(format!("kentos-mcp-active-level-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("katlar.json");
    std::fs::write(&source, doc.to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":source}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    // The wall's level is the input's, nothing implied.
    let wall = |level: &str| json!({"drawing":drawing,"layerId":"duvar","walls":[{"pts":[{"x":0,"y":0},{"x":5,"y":0}],
        "width":0.2,"arch":{"level":level}}]});
    let made = call(&mut server, "cad.walls.create", wall("kat-1"));
    assert_eq!(made["isError"], false, "{made}");
    let items = call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    assert_eq!(items[0]["entity"]["arch"]["level"], "kat-1", "{items}");
    let uid = items[0]["uid"].clone();
    let unknown = call(&mut server, "cad.walls.create", wall("kat-9"));
    assert_eq!(unknown["structuredContent"]["error"]["code"], "level_not_found", "{unknown}");
    // 1. kat taken away while the wall names it: kept, the wall's id said.
    let gone = call(&mut server, "cad.settings.set", json!({"drawing":drawing,"arch":{"levels":[levels[0]]}}));
    let error = &gone["structuredContent"]["error"];
    assert_eq!(error["code"], "level_in_use", "{gone}");
    assert!(error["message"].as_str().unwrap().starts_with("“1. kat” katını 1 duvar anıyor; kaldırılmadı."), "{gone}");
    assert_eq!(error["uids"], json!([uid]));
    // The level and view the drawing opens with: into the file, read back.
    let set = call(&mut server, "cad.settings.set", json!({"drawing":drawing,
        "arch":{"levels":levels,"activeLevel":"kat-1","levelView":"below"}}));
    assert_eq!(set["isError"], false, "{set}");
    let saved = dir.join("katlar.kcad");
    let written = call(&mut server, "drawing.save", json!({"drawing":drawing,"path":saved}));
    assert_eq!(written["isError"], false, "{written}");
    let again = call(&mut server, "drawing.open", json!({"path":saved}));
    let other = again["structuredContent"]["drawing"].clone();
    let summary = call(&mut server, "drawing.summary", json!({"drawing":other}));
    let arch = &summary["structuredContent"]["settings"]["arch"];
    assert_eq!((arch["activeLevel"].clone(), arch["levelView"].clone()), (json!("kat-1"), json!("below")), "{summary}");
    let _ = std::fs::remove_dir_all(dir);
}

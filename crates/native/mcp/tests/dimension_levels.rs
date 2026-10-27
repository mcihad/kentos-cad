//! Real MCP host (docs/adr/0721 §2): an agent binds a Kot to a level; its value is the level's elevation plus its
//! offset, it follows the level the project's settings raise, and the level it names stays.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"dimension-levels-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}}))
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

#[test]
fn a_kot_bound_to_a_level_follows_it_and_keeps_it() {
    let levels = json!([{"id":"zemin","name":"Zemin kat","elevation":0.0},
                        {"id":"k1","name":"1. kat","elevation":3.25}]);
    let doc = json!({"format":"kentos.document","version":1,"name":"Kotlar",
        "settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50.0,
                    "workspace":"cad","arch":{"levels":levels}},
        "origin":{"x":0.0,"y":0.0},
        "layers":[{"id":"olcu","name":"Ölçü","type":"layer","visible":true,"locked":false,"expanded":true,
                   "style":{"color":"fg","lineType":"continuous","lineWeight":0.25},"children":[]}],
        "activeLayer":"olcu","entities":[],"styles":{"items":[],"categories":[]}});
    let dir = std::env::temp_dir().join(format!("kentos-mcp-dimension-levels-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("kotlar.json");
    std::fs::write(&source, doc.to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":source}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let made = call(&mut server, "cad.dimensions.create", json!({"drawing":drawing,"layerId":"olcu","at":{"x":5,"y":4},
        "measure":{"kind":"elevation","a":{"kind":"at","at":{"x":4,"y":3}},"level":{"id":"k1","offset":-0.02}}}));
    assert_eq!(made["isError"], false, "{made}");
    let kot = |server: &mut Server| {
        call(server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"][0].clone()
    };
    let item = kot(&mut server);
    assert_eq!(item["entity"]["za"], json!(3.25 + -0.02), "{item}");
    assert_eq!(item["entity"]["level"], json!({"id":"k1","offset":-0.02}));
    // 1. kat raised: the Kot follows.
    let raised = json!([levels[0], {"id":"k1","name":"1. kat","elevation":6.5}]);
    let set = call(&mut server, "cad.settings.set", json!({"drawing":drawing,"arch":{"levels":raised}}));
    assert_eq!(set["isError"], false, "{set}");
    assert_eq!(kot(&mut server)["entity"]["za"], json!(6.5 + -0.02));
    // Taken away while the Kot names it: kept, its id said.
    let gone = call(&mut server, "cad.settings.set", json!({"drawing":drawing,"arch":{"levels":[levels[0]]}}));
    let error = &gone["structuredContent"]["error"];
    assert_eq!(error["code"], "level_in_use", "{gone}");
    assert!(error["message"].as_str().unwrap().starts_with("“1. kat” katını 1 kot işareti anıyor"), "{gone}");
    assert_eq!(error["uids"], json!([item["uid"]]));
    let _ = std::fs::remove_dir_all(dir);
}

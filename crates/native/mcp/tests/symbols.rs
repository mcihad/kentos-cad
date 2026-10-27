//! Real MCP host (docs/adr/0721 §3–§5): an agent places the north arrow, marks a revision with its date and
//! description, and writes the material legend of the walls; the library's blocks come with them, a local project's
//! true north is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"symbols-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}}))
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

#[test]
fn an_agent_places_the_north_a_revision_and_the_material_legend() {
    let doc = json!({"format":"kentos.document","version":1,"name":"İşaretler",
        "settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100.0,
                    "workspace":"cad"},
        "origin":{"x":0.0,"y":0.0},
        "layers":[{"id":"plan","name":"Plan","type":"layer","visible":true,"locked":false,"expanded":true,
                   "style":{"color":"fg","lineType":"continuous","lineWeight":0.25},"children":[]}],
        "activeLayer":"plan","entities":[],"styles":{"items":[],"categories":[]}});
    let dir = std::env::temp_dir().join(format!("kentos-mcp-symbols-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("isaretler.json");
    std::fs::write(&source, doc.to_string()).unwrap();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path":source}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let walls = call(&mut server, "cad.walls.create", json!({"drawing":drawing,"layerId":"plan",
        "walls":[{"pts":[{"x":0,"y":0},{"x":8,"y":0}],"style":"İç duvar 15"}]}));
    assert_eq!(walls["isError"], false, "{walls}");
    let north = call(&mut server, "cad.north.create", json!({"drawing":drawing,"at":{"x":9,"y":4},"layerId":"plan"}));
    assert_eq!(north["isError"], false, "{north}");
    assert_eq!(north["structuredContent"]["output"]["added"], true, "{north}");
    let refused = call(&mut server, "cad.north.create",
        json!({"drawing":drawing,"at":{"x":9,"y":6},"layerId":"plan","north":{"kind":"true"}}));
    assert_eq!(refused["structuredContent"]["error"]["code"], "no_true_north", "{refused}");
    let marked = call(&mut server, "cad.revisions.mark", json!({"drawing":drawing,"layerId":"plan",
        "frame":{"kind":"rectangle","a":{"x":2,"y":-2},"b":{"x":5,"y":-0.5}},"arc":0.3,"number":"A",
        "date":"2026-10-27","description":"Duvar uzatıldı"}));
    assert_eq!(marked["isError"], false, "{marked}");
    let tag = marked["structuredContent"]["output"]["tag"].clone();
    let legend = call(&mut server, "cad.legends.materials", json!({"drawing":drawing,"at":{"x":-3,"y":4},"layerId":"plan"}));
    assert_eq!(legend["isError"], false, "{legend}");
    let names: Vec<&str> = legend["structuredContent"]["output"]["materials"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["name"].as_str())
        .collect();
    assert_eq!(names, ["Sıva", "Tuğla"]);
    let items = call(&mut server, "drawing.entities", json!({"drawing":drawing}))["structuredContent"]["items"].clone();
    let delta = items.as_array().unwrap().iter().find(|i| i["uid"] == tag).expect("the delta").clone();
    assert_eq!(delta["entity"]["attrs"], json!({"REV":"A","Tarih":"2026-10-27","Açıklama":"Duvar uzatıldı"}));
    let _ = std::fs::remove_dir_all(dir);
}

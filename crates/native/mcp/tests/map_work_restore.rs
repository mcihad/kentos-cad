//! An approved item's objects put back as approved through a real AI host (plan/map-cad.md MAP-01, docs/adr/4080): the
//! `.kcad` schema 213 sample (fixtures/kcad/v2/map-work-approved-values.kcad: the approved design keeps its line's
//! value) opened over MCP. The line given a height is set back to its value; deleted, it is made again with its
//! persistent id, `map.work.query` then says the design is up to date; a second restore finds nothing to put back; the
//! approved delivery package without objects is refused.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"map-work-restore-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const LINE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13";

#[test]
fn the_design_s_line_is_put_back_as_approved_over_mcp() {
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/kcad/v2/map-work-approved-values.kcad");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let design = |server: &mut Server| {
        let r = call(server, "map.work.query", json!({"drawing": drawing}));
        assert_eq!(r["isError"], false, "{r}");
        r["structuredContent"]["output"]["items"][2].clone()
    };
    let set = call(
        &mut server,
        "cad.entities.set",
        json!({"drawing": drawing, "uids": [LINE], "attrs": {"Kot": "1060"}, "operation": "attributes"}),
    );
    assert_eq!(set["isError"], false, "{set}");
    let r = call(&mut server, "map.work.restore", json!({"drawing": drawing, "item": "is-3"}));
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    assert_eq!((out["restored"].clone(), out["reverted"].clone(), out["asApproved"].clone()), (json!([]), json!([LINE]), json!(true)), "{out}");
    assert_eq!(design(&mut server)["state"], "current");
    // Deleted, made again with its id.
    let gone = call(&mut server, "cad.entities.delete", json!({"drawing": drawing, "uids": [LINE]}));
    assert_eq!(gone["isError"], false, "{gone}");
    assert_eq!(design(&mut server)["changes"]["removed"], 1);
    let r = call(&mut server, "map.work.restore", json!({"drawing": drawing, "item": "is-3"}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(r["structuredContent"]["output"]["restored"], json!([LINE]));
    let d = design(&mut server);
    assert_eq!((d["state"].as_str(), d.get("changes")), (Some("current"), None), "{d}");
    // Nothing left to put back; a package without objects is refused.
    let again = call(&mut server, "map.work.restore", json!({"drawing": drawing, "item": "is-3"}));
    assert_eq!(again["isError"], true, "{again}");
    assert!(again.to_string().contains("nothing_to_restore"), "{again}");
    let package = call(&mut server, "map.work.restore", json!({"drawing": drawing, "item": "is-4"}));
    assert_eq!(package["isError"], true, "{package}");
    assert!(package.to_string().contains("no_approved_values"), "{package}");
}

//! Harita işi's stamps through a real AI host (plan/map-cad.md MAP-01, docs/adr/2080): the `.kcad` schema 206 sample
//! (fixtures/kcad/v2/map-work-stamps.kcad: an approved design and an approved delivery with their stamps, a line made
//! from the design) opened over MCP reads both approvals and the record up to date, since the stamps are the content
//! revisions of what the file holds; taken back to review the design loses its stamp, approved again it gets the same
//! one.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"map-work-stamps-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

#[test]
fn the_schema_206_sample_reads_its_stamps_up_to_date_over_mcp() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/map-work-stamps.kcad");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let query = |server: &mut Server| {
        let r = call(server, "map.work.query", json!({"drawing": drawing}));
        assert_eq!(r["isError"], false, "{r}");
        r["structuredContent"]["output"].clone()
    };
    let t = query(&mut server);
    let design = &t["items"][2];
    assert_eq!((design["status"].as_str(), design["state"].as_str()), (Some("approved"), Some("current")), "{t}");
    assert_eq!(design["approved"], design["content"], "the stamp is what the file holds");
    // The delivery holds no objects: its content revision is the digest of [].
    assert_eq!(t["items"][3]["approved"], "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945");
    assert_eq!(t["items"][3]["state"], "current");
    let made = t["derivations"].as_array().unwrap().iter().find(|d| d["source"] == "is-3").expect("the line's record").clone();
    assert_eq!((made["state"].as_str(), made["revision"].clone()), (Some("current"), design["content"].clone()));
    // The two problems are the campaign's and the orthophoto's files, which are not on this machine.
    assert_eq!((t["stale"].as_u64(), t["problems"].as_u64()), (Some(0), Some(2)));
    assert_eq!((t["items"][0]["state"].as_str(), t["items"][1]["state"].as_str()), (Some("missing"), Some("missing")));
    let stamp = design["approved"].clone();
    // Back to review: the stamp goes; approved again: the same stamp.
    let edit = |server: &mut Server, status: &str| {
        let r = call(server, "map.work.edit", json!({"drawing": drawing, "operation": {"op": "set", "item": "is-3", "status": status}}));
        assert_eq!(r["isError"], false, "{r}");
        r["structuredContent"]["output"].clone()
    };
    let out = edit(&mut server, "review");
    assert!(out.get("approved").is_none(), "{out}");
    assert!(query(&mut server)["items"][2].get("approved").is_none());
    let out = edit(&mut server, "approved");
    assert_eq!((out["approved"].clone(), out["content"].clone()), (stamp.clone(), stamp));
}

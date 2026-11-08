//! Harita işi's objects changed since an approval through a real AI host (plan/map-cad.md MAP-01, docs/adr/2880): the
//! `.kcad` schema 210 sample (fixtures/kcad/v2/map-work-approved-objects.kcad: the approved design keeps its one line's
//! digest) opened over MCP reads the design up to date with nothing changed; its line given a height and a line drawn
//! on its layer, `map.work.query` counts one changed and one added and lists both ids. Saved, the file is schema 210 and
//! keeps the approval's one pair; approved again, the design is up to date and the saved file keeps both lines' pairs.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"map-work-changes-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

const LINE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13";

#[test]
fn the_design_s_changed_and_added_lines_are_listed_and_saved_over_mcp() {
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/kcad/v2/map-work-approved-objects.kcad");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let design = |server: &mut Server| {
        let r = call(server, "map.work.query", json!({"drawing": drawing}));
        assert_eq!(r["isError"], false, "{r}");
        r["structuredContent"]["output"]["items"][2].clone()
    };
    let d = design(&mut server);
    assert_eq!((d["state"].as_str(), d.get("changes")), (Some("current"), None), "{d}");
    let set = call(
        &mut server,
        "cad.entities.set",
        json!({"drawing": drawing, "uids": [LINE], "attrs": {"Kot": "1060"}, "operation": "attributes"}),
    );
    assert_eq!(set["isError"], false, "{set}");
    let made = call(
        &mut server,
        "cad.entities.create",
        json!({"drawing": drawing, "layerId": "tasarim", "objects": [{"geometry": {
            "kind": "line", "a": {"x": 500000.0, "y": 4400005.0}, "b": {"x": 500010.0, "y": 4400005.0}}}]}),
    );
    assert_eq!(made["isError"], false, "{made}");
    let new = made["structuredContent"]["output"]["created"][0].as_str().unwrap().to_owned();
    let d = design(&mut server);
    assert_eq!(d["state"], "revised", "{d}");
    let c = &d["changes"];
    assert_eq!((c["added"].as_u64(), c["removed"].as_u64(), c["changed"].as_u64()), (Some(1), Some(0), Some(1)), "{d}");
    let mut listed: Vec<&str> = c["uids"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    listed.sort_unstable();
    let mut both = vec![LINE, new.as_str()];
    both.sort_unstable();
    assert_eq!(listed, both);
    let saved_pairs = |server: &mut Server, name: &str| {
        let dir = std::env::temp_dir().join(format!("kentos-map-work-changes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let s = call(server, "drawing.save", json!({"drawing": drawing, "path": path}));
        assert_eq!(s["isError"], false, "{s}");
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.windows(10).any(|w| w == b"\x67version\x18\xd2"), "the file is schema 210");
        let back = serde_json::to_value(kentos_kcad::decode(&bytes).unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(dir);
        back["settings"]["mapWork"]["items"][2]["approvedObjects"].clone()
    };
    // Saved before a new approval: the approval's one pair, the line's digest then.
    let pairs = saved_pairs(&mut server, "degisti.kcad");
    assert_eq!(pairs.as_array().map(Vec::len), Some(1), "{pairs}");
    assert_eq!(pairs[0][0], LINE);
    // Approved again: up to date, both lines kept with their digests now.
    let r = call(
        &mut server,
        "map.work.edit",
        json!({"drawing": drawing, "operation": {"op": "set", "item": "is-3", "status": "approved"}}),
    );
    assert_eq!(r["isError"], false, "{r}");
    let d = design(&mut server);
    assert_eq!((d["state"].as_str(), d.get("changes")), (Some("current"), None), "{d}");
    let pairs = saved_pairs(&mut server, "yeniden-onaylandi.kcad");
    let ids: Vec<&str> = pairs.as_array().unwrap().iter().filter_map(|p| p[0].as_str()).collect();
    assert_eq!(ids, both, "sorted by id");
    assert_ne!(pairs[0][1], pairs[1][1]);
}

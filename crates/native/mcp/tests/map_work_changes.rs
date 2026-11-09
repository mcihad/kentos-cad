//! Harita işi's objects changed since an approval through a real AI host (plan/map-cad.md MAP-01, docs/adr/2880): the
//! `.kcad` schema 210 sample (fixtures/kcad/v2/map-work-approved-objects.kcad: the approved design keeps its one line's
//! digest) opened over MCP reads the design up to date with nothing changed; its line given a height and a line drawn
//! on its layer, `map.work.query` counts one changed and one added and lists both ids. Saved, the file is schema 210 and
//! keeps the approval's one pair; approved again, the design is up to date and the saved file (schema 213: with the
//! parts, docs/adr/3280, and the values, docs/adr/3680) keeps both lines' pairs. docs/adr/3280: the schema 211 sample
//! says the changed line's part. docs/adr/3680: the schema 213 sample names the changed attribute with its values and
//! says the deleted line as it was approved; the values saved are the texts the digests are of.
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
    let saved_pairs = |server: &mut Server, name: &str, schema: u8| {
        let dir = std::env::temp_dir().join(format!("kentos-map-work-changes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let s = call(server, "drawing.save", json!({"drawing": drawing, "path": path}));
        assert_eq!(s["isError"], false, "{s}");
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.windows(10).any(|w| w == [b"\x67version\x18".as_slice(), &[schema]].concat()), "the file's schema {schema}");
        let back = serde_json::to_value(kentos_kcad::decode(&bytes).unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(dir);
        back["settings"]["mapWork"]["items"][2]["approvedObjects"].clone()
    };
    // Saved before a new approval: the approval's one pair, the line's digest then.
    let pairs = saved_pairs(&mut server, "degisti.kcad", 210);
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
    let pairs = saved_pairs(&mut server, "yeniden-onaylandi.kcad", 213);
    let ids: Vec<&str> = pairs.as_array().unwrap().iter().filter_map(|p| p[0].as_str()).collect();
    assert_eq!(ids, both, "sorted by id");
    assert_ne!(pairs[0][1], pairs[1][1]);
}

/// docs/adr/3280: the schema 211 sample (its approval keeps the line's parts) opened over MCP; the line's height
/// attribute set, `map.work.query` says its attributes part changed, and only it.
#[test]
fn the_changed_line_s_part_is_said_over_mcp() {
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/kcad/v2/map-work-approved-parts.kcad");
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let set = call(
        &mut server,
        "cad.entities.set",
        json!({"drawing": drawing, "uids": [LINE], "attrs": {"Kot": "1060"}, "operation": "attributes"}),
    );
    assert_eq!(set["isError"], false, "{set}");
    let r = call(&mut server, "map.work.query", json!({"drawing": drawing}));
    let c = r["structuredContent"]["output"]["items"][2]["changes"].clone();
    assert_eq!(c["parts"], json!({"geometry": 0, "attributes": 1, "layer": 0, "look": 0}), "{c}");
    assert_eq!(c["objects"], json!([{"uid": LINE, "parts": ["attributes"]}]), "{c}");
}

/// docs/adr/3680: the schema 213 sample (its approval keeps the line's value) opened over MCP; the line's height set,
/// `map.work.query` names the attribute with no value then and “1060” now; the line deleted, it says the line as it was
/// approved, which is the sample's kept text read back as JSON.
#[test]
fn the_changed_attribute_and_the_deleted_line_are_said_over_mcp() {
    let sample = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/kcad/v2/map-work-approved-values.kcad");
    let kept: Value = serde_json::from_str(include_str!("../../../../fixtures/kcad/v2/map-work-approved-values.json")).unwrap();
    let text = kept["settings"]["mapWork"]["items"][2]["approvedValues"][0].as_str().unwrap().to_owned();
    let mut server = Server::new();
    let opened = call(&mut server, "drawing.open", json!({"path": sample}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let set = call(
        &mut server,
        "cad.entities.set",
        json!({"drawing": drawing, "uids": [LINE], "attrs": {"Kot": "1060"}, "operation": "attributes"}),
    );
    assert_eq!(set["isError"], false, "{set}");
    let changes = |server: &mut Server| {
        let r = call(server, "map.work.query", json!({"drawing": drawing}));
        r["structuredContent"]["output"]["items"][2]["changes"].clone()
    };
    let c = changes(&mut server);
    assert_eq!(c["objects"], json!([{"uid": LINE, "parts": ["attributes"], "attributes": [{"name": "Kot", "now": "1060"}]}]), "{c}");
    let gone = call(&mut server, "cad.entities.delete", json!({"drawing": drawing, "uids": [LINE]}));
    assert_eq!(gone["isError"], false, "{gone}");
    let c = changes(&mut server);
    let approved: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(c["removedObjects"], json!([{"uid": LINE, "approved": approved}]), "{c}");
}

//! The building, parcel and owner example of docs/adr/0532 through the MCP server: three tables of a GeoPackage bound as
//! layers, the parcels' owners a many-to-many reference mapped into the file, a parcel and its owner written in one
//! transaction, the owners read back a page at a time. The map (the file's mapping rows), the table (the layers'
//! objects), the form (`commit`), Python (python/tests/test_source_relations.py) and this give the same answers.

use std::path::PathBuf;

use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/geopackage/v1")
}

fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
    let request = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": name, "arguments": arguments,
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": MODERN,
                "io.modelcontextprotocol/clientInfo": { "name": "iliski", "version": "1" },
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }
    });
    let answer = server.handle(&request).expect("an answer");
    let result = answer["result"].clone();
    assert_eq!(result["isError"], false, "{name}: {answer}");
    result["structuredContent"].clone()
}

fn done(server: &mut Server, drawing: &str, name: &str, mut arguments: Value) -> Value {
    arguments["drawing"] = json!(drawing);
    let answer = call(server, name, arguments);
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer["output"].clone()
}

const O1: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001";
const O2: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002";
const O3: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003";
const P1: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2001";
const P3: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2003";

#[test]
fn the_parcels_owners_are_mapped_written_together_and_paged_through_the_tools() {
    let file = std::env::temp_dir().join(format!("kentos-mcp-iliski-{}.gpkg", std::process::id()));
    std::fs::copy(fixtures().join("iliskili.gpkg"), &file).expect("a copy");
    let mut server = Server::new();
    let made = call(
        &mut server,
        "drawing.new",
        json!({ "name": "Kadastro", "srid": 5256 }),
    );
    let drawing = made["drawing"].as_str().expect("a handle").to_owned();
    for table in ["malik", "parsel", "bina"] {
        done(
            &mut server,
            &drawing,
            "cad.sources.edit",
            json!({ "edit": { "kind": "link", "path": file, "table": table } }),
        );
    }
    done(
        &mut server,
        &drawing,
        "cad.layers.edit",
        json!({ "operation": "fields", "layer": "import-parsel", "fields": [
            { "name": "ada", "kind": "text" }, { "name": "parsel_no", "kind": "text" },
            { "name": "malikler", "kind": "text", "multiple": true } ] }),
    );
    for relation in [
        json!({ "id": "parsel-malik", "name": "Parselin malikleri", "parent": "import-malik", "child": "import-parsel", "field": "malikler" }),
        json!({ "id": "bina-parsel", "name": "Parselin binaları", "parent": "import-parsel", "child": "import-bina", "field": "parsel" }),
    ] {
        done(
            &mut server,
            &drawing,
            "cad.relations.edit",
            json!({ "edit": { "kind": "create", "relation": relation } }),
        );
    }
    // How the reference is kept, then mapped: four mapping rows.
    let checked = done(
        &mut server,
        &drawing,
        "cad.sources.relate",
        json!({ "relation": "parsel-malik", "action": "check" }),
    );
    assert_eq!(checked["state"], "text", "{checked}");
    let mapped = done(
        &mut server,
        &drawing,
        "cad.sources.relate",
        json!({ "relation": "parsel-malik", "action": "map" }),
    );
    assert_eq!(
        (
            mapped["state"].clone(),
            mapped["rows"].clone(),
            mapped["mapping"].clone()
        ),
        (json!("mapped"), json!(4), json!("parsel_malik")),
        "{mapped}"
    );
    // The owners of the first parcel, in the reference's order, and the parcels of one owner.
    let owners = done(
        &mut server,
        &drawing,
        "cad.records.page",
        json!({ "relation": "parsel-malik", "of": P1, "side": "parents" }),
    );
    let uids: Vec<&str> = owners["records"]
        .as_array()
        .expect("records")
        .iter()
        .filter_map(|r| r["uid"].as_str())
        .collect();
    assert_eq!(
        (uids, owners["total"].clone()),
        (vec![O1, O2], json!(2)),
        "{owners}"
    );
    assert_eq!(owners["source"]["table"], "malik");
    assert_eq!(owners["source"]["edited"], false);
    let parcels = done(
        &mut server,
        &drawing,
        "cad.records.page",
        json!({ "relation": "parsel-malik", "of": O2 }),
    );
    assert_eq!(parcels["total"], 2, "{parcels}");
    // A third owner on the third parcel and the owner's own change, written in one transaction.
    done(
        &mut server,
        &drawing,
        "cad.entities.set",
        json!({ "uids": [P3], "operation": "attributes", "attrs": { "malikler": format!("{O3}; {O1}") } }),
    );
    done(
        &mut server,
        &drawing,
        "cad.entities.set",
        json!({ "uids": [O3], "operation": "attributes", "attrs": { "tur": "Kamu" } }),
    );
    let links = done(&mut server, &drawing, "cad.sources.describe", json!({}));
    eprintln!("LINKS {links}");
    let alone = call(
        &mut server,
        "cad.sources.sync",
        json!({ "drawing": drawing, "layer": "import-parsel", "direction": "write", "op": "plan" }),
    );
    assert_eq!(alone["status"], "completed", "{alone}");
    eprintln!("ALONE {alone}");
    let links = done(&mut server, &drawing, "cad.sources.describe", json!({}));
    eprintln!("LINKS2 {links}");
    let written = done(
        &mut server,
        &drawing,
        "cad.sources.sync",
        json!({ "layer": "import-parsel", "direction": "write", "with": ["import-malik"] }),
    );
    assert_eq!(
        (written["modified"].clone(), written["relations"].clone()),
        (
            json!(1),
            json!([{ "relation": "parsel-malik", "added": 2, "removed": 0 }])
        ),
        "{written}"
    );
    // The file's mapping holds the new rows, as the layers and the pages say.
    let after = done(
        &mut server,
        &drawing,
        "cad.sources.relate",
        json!({ "relation": "parsel-malik", "action": "check" }),
    );
    assert_eq!(
        (after["state"].clone(), after["rows"].clone()),
        (json!("mapped"), json!(6)),
        "{after}"
    );
    let third = done(
        &mut server,
        &drawing,
        "cad.records.page",
        json!({ "relation": "parsel-malik", "of": P3, "side": "parents" }),
    );
    let uids: Vec<&str> = third["records"]
        .as_array()
        .expect("records")
        .iter()
        .filter_map(|r| r["uid"].as_str())
        .collect();
    assert_eq!(uids, vec![O3, O1], "{third}");
    assert_eq!(
        third["source"]["edited"], false,
        "written: the layer matches its file again"
    );
    std::fs::remove_file(&file).ok();
}

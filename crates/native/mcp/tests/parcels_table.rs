//! Parsel alan çizelgesi through a real AI host (docs/adr/9306): the shared cases' drawing opened as a .kcad file over
//! MCP. `cad.parcels.report` gives the case's table words and counts and writes nothing; `cad.parcels.table` is
//! planned, then written as planned in one undo step, and undone; a line given as a parcel is refused with the case's
//! words.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn call(s: &mut Server, name: &str, args: Value) -> Value {
    s.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args,"_meta":{"io.modelcontextprotocol/protocolVersion":MODERN,"io.modelcontextprotocol/clientInfo":{"name":"parcels-table-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone()
}

fn objects(s: &mut Server, drawing: &Value) -> usize {
    call(s, "drawing.entities", json!({ "drawing": drawing }))["structuredContent"]["items"]
        .as_array()
        .map_or(0, Vec::len)
}

#[test]
fn the_parcel_table_over_mcp_is_the_case_s() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.parcels.report.json"
    ))
    .unwrap();
    let case = |prefix: &str| {
        cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"].as_str().unwrap().starts_with(prefix))
            .unwrap()
            .clone()
    };
    let mut server = Server::new();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).unwrap();
    let path =
        std::env::temp_dir().join(format!("kentos-parcels-table-{}.kcad", std::process::id()));
    std::fs::write(&path, kentos_kcad::encode(&snapshot).unwrap()).unwrap();
    let opened = call(&mut server, "drawing.open", json!({ "path": path }));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();
    let before = objects(&mut server, &drawing);

    // The report: the case's words and counts, nothing written.
    let step = case("plan: parsel katmanının")["steps"][0].clone();
    let mut args = step["input"].clone();
    args["drawing"] = drawing.clone();
    let r = call(&mut server, "cad.parcels.report", args);
    assert_eq!(r["isError"], false, "{r}");
    let out = &r["structuredContent"]["output"];
    let want = &step["result"]["output"];
    for key in [
        "cells", "places", "named", "numbered", "srid", "rule", "field",
    ] {
        assert_eq!(out[key], want[key], "{key}");
    }
    let names = |v: &Value| {
        v["parcels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p["heading"].clone(), p["status"].clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(names(out), names(want));
    assert_eq!(
        objects(&mut server, &drawing),
        before,
        "the report writes nothing"
    );

    // The table: planned first, then written as planned, one undo step.
    let mut args =
        json!({ "at": { "x": 487080.0, "y": 4420060.0 }, "drawing": drawing, "op": "plan" });
    let plan = call(&mut server, "cad.parcels.table", args.clone())["structuredContent"].clone();
    assert_eq!(plan["status"], "completed", "{plan}");
    assert_eq!(
        plan["output"]["rows"].as_u64().unwrap() as usize,
        want["cells"].as_array().unwrap().len()
    );
    assert_eq!(
        objects(&mut server, &drawing),
        before,
        "a plan writes nothing"
    );
    args["op"] = json!("execute");
    args["expected"] = json!({ "plan": plan["planId"] });
    let written = call(&mut server, "cad.parcels.table", args)["structuredContent"].clone();
    assert_eq!(written["status"], "completed", "{written}");
    assert_eq!(written["output"]["parcels"], 4);
    assert_eq!(objects(&mut server, &drawing), before + 1);
    call(&mut server, "drawing.undo", json!({ "drawing": drawing }));
    assert_eq!(objects(&mut server, &drawing), before, "undone in one step");

    // A line is no parcel: the case's words.
    let step = case("çizgi bir parsel değil")["steps"][0].clone();
    let mut args = step["input"].clone();
    args["drawing"] = drawing;
    let said = call(&mut server, "cad.parcels.report", args).to_string();
    assert!(
        said.contains(step["result"]["error"]["message"].as_str().unwrap()),
        "{said}"
    );
    let _ = std::fs::remove_file(path);
}

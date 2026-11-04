//! Public MCP discovery and query must match the desktop's read-only diagnostic contract.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn tool(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server.handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":name,"arguments":args,"_meta":{
            "io.modelcontextprotocol/protocolVersion":MODERN,
            "io.modelcontextprotocol/clientInfo":{"name":"inspection-test","version":"1"},
            "io.modelcontextprotocol/clientCapabilities":{}}}})).unwrap()["result"].clone();
    assert_eq!(result["isError"], false, "{result}");
    result["structuredContent"].clone()
}

#[test]
fn inspection_through_mcp_keeps_uuid_geometry_and_reports_physical_gap() {
    let mut server = Server::new();
    let new = tool(&mut server,"drawing.new",json!({"workspace":"cad"}));
    let drawing = &new["drawing"];
    let layer = &new["summary"]["activeLayer"];
    for (a,b) in [(500000.,500001.),(500001.004,500002.)] {
        let made = tool(&mut server,"cad.line.create",json!({"drawing":drawing,"layerId":layer,
            "a":{"x":a,"y":4500000.},"b":{"x":b,"y":4500000.}}));
        assert_eq!(made["status"],"completed","{made}");
    }
    let before = tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    let found = tool(&mut server,"cad.drawing.inspect",json!({"drawing":drawing,"maxGap":0.005}));
    assert_eq!(found["status"],"completed","{found}");
    let findings = found["output"]["findings"].as_array().unwrap();
    let gap = findings.iter().find(|f|f["code"]=="boundary_gap").expect("4 mm gap");
    assert!((gap["distance"].as_f64().unwrap()-0.004).abs()<1e-8);
    assert_eq!(gap["uids"].as_array().unwrap().len(),2);
    assert_eq!(tool(&mut server,"drawing.entities",json!({"drawing":drawing})),before);
    let empty = tool(&mut server,"cad.drawing.inspect",json!({"drawing":drawing,"uids":[]}));
    assert_eq!(empty["output"]["examined"],0);
}

#[test]
fn hatch_origin_edit_through_mcp_keeps_all_other_hatch_data_and_undoes_once() {
    let mut server = Server::new();
    let new = tool(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = &new["drawing"];
    let layer = &new["summary"]["activeLayer"];
    let circle = tool(&mut server, "cad.circle.create", json!({"drawing":drawing,"layerId":layer,"c":{"x":0.,"y":0.},"r":5.}));
    assert_eq!(circle["status"], "completed", "{circle}");
    let made = tool(&mut server,"cad.entities.hatch",json!({"drawing":drawing,"layerId":layer,
        "region":{"kind":"point","at":{"x":0.,"y":0.},"associative":true},
        "pattern":{"name":"ANSI31","scale":2.,"angle":31.}}));
    assert_eq!(made["status"], "completed", "{made}");
    let before = tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    let hatch = before["items"].as_array().unwrap().iter().find(|r|r["entity"]["kind"]=="hatch").unwrap();
    let done = tool(&mut server,"cad.hatches.edit",json!({"drawing":drawing,"edit":{"kind":"origin","uids":[hatch["uid"]],"origin":{"kind":"point","at":{"x":0.123,"y":0.456}}}}));
    assert_eq!(done["status"], "completed", "{done}");
    let after = tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    let mut expected = hatch["entity"].clone();
    expected["pattern"]["origin"] = json!({"x":0.123,"y":0.456});
    assert_eq!(after["items"].as_array().unwrap().iter().find(|r|r["uid"]==hatch["uid"]).unwrap()["entity"],expected);
    tool(&mut server,"drawing.undo",json!({"drawing":drawing}));
    let undone = tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    assert_eq!(undone["items"].as_array().unwrap().iter().map(|r|&r["entity"]).collect::<Vec<_>>(), before["items"].as_array().unwrap().iter().map(|r|&r["entity"]).collect::<Vec<_>>());
}

#[test]
fn annotation_inspection_through_mcp_reports_true_measure_separately_from_written_text() {
    let mut server = Server::new();
    let new = tool(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = &new["drawing"];
    let layer = &new["summary"]["activeLayer"];
    let made = tool(&mut server, "cad.entities.create", json!({"drawing":drawing,"layerId":layer,
        "objects":[{"geometry":{"kind":"dimension","a":{"x":0.,"y":0.},"b":{"x":2.5,"y":0.},"offset":0.3,"height":0.125,"text":"2400"}}]}));
    assert_eq!(made["status"], "completed", "{made}");
    let before = tool(&mut server, "drawing.entities", json!({"drawing":drawing}));
    let found = tool(&mut server, "cad.drawing.inspect", json!({"drawing":drawing,"checks":["annotations"]}));
    assert_eq!(found["status"], "completed", "{found}");
    let values = tool(&mut server, "cad.dimensions.query", json!({"drawing":drawing}));
    assert_eq!(found["output"]["dimensions"], values["output"]["dimensions"]);
    assert_eq!(found["output"]["dimensions"][0]["measured"], 2.5);
    assert_eq!(found["output"]["dimensions"][0]["shown"], "2400");
    assert!(found["output"]["findings"].as_array().unwrap().iter().any(|f| f["code"] == "dimension_override"));
    assert_eq!(tool(&mut server, "drawing.entities", json!({"drawing":drawing})), before);
}

#[test]
fn source_snap_query_preserves_coincident_owner_uuids_without_changing_default_cycle() {
    let mut server = Server::new();
    let new = tool(&mut server,"drawing.new",json!({"workspace":"cad"}));
    let drawing = &new["drawing"];
    let layer = &new["summary"]["activeLayer"];
    for (a,b) in [([0.,0.],[2.,0.]),([2.,0.],[2.,2.])] {
        let made = tool(&mut server,"cad.line.create",json!({"drawing":drawing,"layerId":layer,
            "a":{"x":a[0],"y":a[1]},"b":{"x":b[0],"y":b[1]}}));
        assert_eq!(made["status"],"completed");
    }
    let request = json!({"drawing":drawing,"at":{"x":2.,"y":0.},"aperture":0.01,"kinds":["endpoint"],"all":true});
    let ordinary = tool(&mut server,"cad.snap.find",request.clone());
    let mut request = request; request["sources"] = json!(true);
    let sources = tool(&mut server,"cad.snap.find",request);
    assert_eq!(ordinary["output"]["snaps"].as_array().unwrap().len(),1);
    let snaps = sources["output"]["snaps"].as_array().unwrap();
    assert_eq!(snaps.len(),2);
    assert_ne!(snaps[0]["uid"],snaps[1]["uid"]);
    assert_eq!(snaps[0]["point"],snaps[1]["point"]);
}

#[test]
fn unicode_profile_creation_and_diagnostics_use_the_same_public_mcp_contract() {
    let mut server=Server::new();
    let new=tool(&mut server,"drawing.new",json!({"workspace":"cad"}));
    let drawing=&new["drawing"];let layer=&new["summary"]["activeLayer"];
    let words="Ög\u{306}ç ⌀32 — العربية עברית";
    let made=tool(&mut server,"cad.entities.create",json!({"drawing":drawing,"layerId":layer,
        "objects":[{"geometry":{"kind":"text","p":{"x":5000000.,"y":4400000.},"height":0.18,"rotation":37.,
            "text":words,"font":"arimo","layout":{"engine":"unicode-v1"}}}]}));
    assert_eq!(made["status"],"completed","{made}");
    let before=tool(&mut server,"drawing.entities",json!({"drawing":drawing}));
    assert_eq!(before["items"][0]["entity"]["layout"]["engine"],"unicode-v1");
    assert_eq!(before["items"][0]["entity"]["text"],words);
    let found=tool(&mut server,"cad.drawing.inspect",json!({"drawing":drawing,"checks":["annotations"]}));
    let fallback=found["output"]["findings"].as_array().unwrap().iter().find(|f|f["code"]=="text_font_fallback").unwrap();
    assert_eq!(fallback["severity"],"info");
    assert!(fallback["message"].as_str().unwrap().contains("noto-sans-arabic"));
    assert_eq!(tool(&mut server,"drawing.entities",json!({"drawing":drawing})),before);
}
#[test]
fn standalone_table_field_diagnosis_and_refresh_match_through_mcp_and_undo() {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut setup = raw["setup"].take();
    setup["entities"] = json!([{"kind":"table","id":1,"layerId":"a","attrs":{},
        "p":{"x":0.,"y":0.},"rotation":0.,"height":0.125,"rows":[0.5],"columns":[3.],
        "cells":[["Eski"]],"fields":[{"row":0,"col":0,"source":{"kind":"project","value":"name"}}]}]);
    let path = std::env::temp_dir().join(format!(
        "kentos-table-field-mcp-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, serde_json::to_vec(&setup).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = tool(&mut server, "drawing.open", json!({"path":path}));
    let drawing = &opened["drawing"];
    let before = tool(&mut server, "drawing.entities", json!({"drawing":drawing}));
    let uid = before["items"][0]["uid"].clone();
    let found = tool(
        &mut server,
        "cad.drawing.inspect",
        json!({"drawing":drawing,"checks":["annotations"]}),
    );
    let stale = found["output"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == "table_field_stale")
        .unwrap();
    assert_eq!(stale["uids"], json!([uid]));
    assert_eq!(
        tool(&mut server, "drawing.entities", json!({"drawing":drawing})),
        before
    );
    let refreshed = tool(
        &mut server,
        "cad.tables.edit",
        json!({"drawing":drawing,"operation":{"kind":"refresh","uids":[uid]}}),
    );
    assert_eq!(refreshed["status"], "completed", "{refreshed}");
    let found = tool(
        &mut server,
        "cad.drawing.inspect",
        json!({"drawing":drawing,"checks":["annotations"]}),
    );
    assert!(
        !found["output"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"].as_str().unwrap().starts_with("table_field_"))
    );
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    let undone = tool(&mut server, "drawing.entities", json!({"drawing":drawing}));
    assert_eq!(undone["items"], before["items"]);
    std::fs::remove_file(path).unwrap();
}

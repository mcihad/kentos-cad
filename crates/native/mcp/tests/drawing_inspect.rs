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

#[test]
fn nested_block_fields_through_mcp_keep_the_real_owner_and_placed_location() {
    let mut setup: Value = serde_json::from_str::<Value>(include_str!(
        "../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap()["setup"]
        .clone();
    let outer = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4051";
    let inner = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4052";
    setup["blocks"] = json!([
        {"id":outer,"name":"Dış blok","base":{"x":0.,"y":0.},"units":"m","entities":[{"kind":"insert","id":1,"layerId":"a","attrs":{},"block":inner,"p":{"x":2.,"y":2.},"scale":2.,"rotation":std::f64::consts::FRAC_PI_2}]},
        {"id":inner,"name":"Alanlı iç","base":{"x":0.,"y":0.},"units":"m","entities":[{"kind":"text","id":1,"layerId":"a","attrs":{},"p":{"x":1.,"y":0.},"height":0.125,"rotation":0.,"text":"Eski","fields":[{"start":0,"end":4,"source":{"kind":"project","value":"name"}}]}]}
    ]);
    setup["entities"] = json!([{"kind":"insert","id":1,"layerId":"a","attrs":{},"block":outer,"p":{"x":10.,"y":20.},"scale":1.,"rotation":0.}]);
    let path = std::env::temp_dir().join(format!(
        "kentos-block-fields-mcp-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, serde_json::to_vec(&setup).unwrap()).unwrap();
    let mut server = Server::new();
    let opened = tool(&mut server, "drawing.open", json!({"path":path}));
    let drawing = &opened["drawing"];
    let before = tool(&mut server, "drawing.entities", json!({"drawing":drawing}));
    let owner = &before["items"][0]["uid"];
    let found = tool(
        &mut server,
        "cad.drawing.inspect",
        json!({"drawing":drawing,"checks":["annotations"],"uids":[owner]}),
    );
    assert_eq!(found["status"], "completed", "{found}");
    let stale = found["output"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == "block_text_field_stale")
        .unwrap();
    assert_eq!(stale["uids"], json!([owner]));
    assert_eq!(stale["points"], json!([{"x":12.,"y":24.}]));
    assert!(stale["message"].as_str().unwrap().contains("Alanlı iç"));
    assert_eq!(
        tool(&mut server, "drawing.entities", json!({"drawing":drawing})),
        before
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn explicit_text_profile_edit_through_mcp_keeps_curve_and_undoes_once() {
    let mut server = Server::new();
    let new = tool(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = &new["drawing"];
    let layer = &new["summary"]["activeLayer"];
    let mut geometry = json!({"kind":"text","p":{"x":5000000.,"y":4400000.},"height":0.18,"rotation":17.,
        "text":"  Ög\u{306}ç  ","font":"arimo","widthFactor":0.8,"mask":true,
        "path":{"pts":[{"x":2.,"y":1.}],"bulges":[0.4]},"layout":{"leftIndent":0.4,"tabs":[4.,8.]}});
    let made = tool(
        &mut server,
        "cad.entities.create",
        json!({"drawing":drawing,"layerId":layer,"objects":[{"geometry":geometry}]}),
    );
    assert_eq!(made["status"], "completed", "{made}");
    let before = tool(&mut server, "drawing.entities", json!({"drawing":drawing}));
    geometry["layout"]["engine"] = json!("unicode-v1");
    let edited = tool(
        &mut server,
        "cad.entities.edit",
        json!({"drawing":drawing,"operation":"properties",
        "changes":[{"kind":"update","uid":before["items"][0]["uid"],"geometry":geometry}]}),
    );
    assert_eq!(edited["status"], "completed", "{edited}");
    let after = tool(&mut server, "drawing.entities", json!({"drawing":drawing}));
    let mut expected = before["items"].clone();
    expected[0]["entity"]["layout"]["engine"] = json!("unicode-v1");
    assert_eq!(after["items"][0]["entity"], expected[0]["entity"]);
    assert_eq!(after["items"][0]["uid"], before["items"][0]["uid"]);
    assert_ne!(after["items"][0]["stamp"], before["items"][0]["stamp"]);
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        tool(&mut server, "drawing.entities", json!({"drawing":drawing}))["items"],
        before["items"]
    );
}

#[test]
fn unicode_readable_through_mcp_preserves_metadata_and_undoes_once() {
    let mut server = Server::new();
    let new = tool(&mut server, "drawing.new", json!({"workspace":"cad"}));
    let drawing = &new["drawing"];
    let made = tool(
        &mut server,
        "cad.entities.create",
        json!({"drawing":drawing,"layerId":new["summary"]["activeLayer"],
        "objects":[{"geometry":{"kind":"text","p":{"x":5000000.,"y":4400000.},"text":"Ög\u{301}\u{306}\u{323}ç — AV",
            "height":0.3,"rotation":137.,"font":"arimo","oblique":24.,"mask":true,"layout":{"engine":"unicode-v1"}}}]}),
    );
    assert_eq!(made["status"], "completed", "{made}");
    let before = tool(&mut server, "drawing.entities", json!({"drawing":drawing}));
    let edited = tool(
        &mut server,
        "cad.shapes.edit",
        json!({"drawing":drawing,"work":{"kind":"readable","ids":[before["items"][0]["uid"]]}}),
    );
    assert_eq!(edited["status"], "completed", "{edited}");
    assert_eq!(edited["output"]["count"], 1);
    let mut after = tool(&mut server, "drawing.entities", json!({"drawing":drawing}));
    assert_eq!(after["items"][0]["entity"]["rotation"], 317.);
    assert_ne!(
        after["items"][0]["entity"]["p"],
        before["items"][0]["entity"]["p"]
    );
    for key in ["p", "rotation"] {
        after["items"][0]["entity"][key] = before["items"][0]["entity"][key].clone();
    }
    assert_eq!(after["items"][0]["entity"], before["items"][0]["entity"]);
    assert_eq!(after["items"][0]["uid"], before["items"][0]["uid"]);
    tool(&mut server, "drawing.undo", json!({"drawing":drawing}));
    assert_eq!(
        tool(&mut server, "drawing.entities", json!({"drawing":drawing}))["items"],
        before["items"]
    );
}

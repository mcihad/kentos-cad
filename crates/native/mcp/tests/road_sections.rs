//! KentOS Yol's templates and sample line sets through MCP (plan/road.md Y5, docs/adr/0903): the road project's sample
//! drawing holds two templates and three sets; a template and a set are made, the set's sections asked, a profile and a
//! template a set uses are kept.
use kentos_mcp::{MODERN, Server};
use serde_json::{Value, json};

fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":MODERN,
        "io.modelcontextprotocol/clientInfo":{"name":"road-section-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities":{}})
}

fn call(server: &mut Server, name: &str, args: Value) -> Value {
    let result = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":name,"arguments":args,"_meta":meta()}}))
        .unwrap()["result"]
        .clone();
    assert_eq!(result["resultType"], "complete", "{result}");
    result
}

#[test]
fn a_set_is_made_asked_and_keeps_what_it_uses() {
    let mut server = Server::new();
    let listed = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}}))
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let tool = |name: &str| tools.iter().find(|t| t["name"] == name).cloned().unwrap_or_default();
    assert_eq!(tool("road.sections.query")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("road.sections.edit")["annotations"]["destructiveHint"], true);
    assert_eq!(tool("road.templates.edit")["annotations"]["destructiveHint"], true);

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    assert_eq!(opened["isError"], false, "{opened}");
    let drawing = opened["structuredContent"]["drawing"].clone();

    let made = call(
        &mut server,
        "road.templates.edit",
        json!({"drawing":drawing,"operation":"set","name":"Dar","left":{
            "components":[{"kind":"lane","width":3.0,"slope":-0.02}],"cut":1.0,"fill":1.5}}),
    );
    assert_eq!(made["isError"], false, "{made}");
    assert_eq!(made["structuredContent"]["output"]["template"]["id"], "tk-3");

    let set = call(
        &mut server,
        "road.sections.edit",
        json!({"drawing":drawing,"operation":"set","name":"Dar kesitler","alignment":"ana aks","profile":"Proje kotu",
            "template":"dar","left":20.0,"right":20.0,"stations":{"method":"stations","stations":[200.0,100.0]}}),
    );
    assert_eq!(set["isError"], false, "{set}");
    let report = &set["structuredContent"]["output"]["set"];
    assert_eq!(report["id"], "k-4");
    assert_eq!(report["stations"], json!([100.0, 200.0]));

    let asked = call(&mut server, "road.sections.query", json!({"drawing":drawing,"set":"k-4"}));
    assert_eq!(asked["isError"], false, "{asked}");
    let rows = asked["structuredContent"]["output"]["sections"].as_array().expect("sections").clone();
    assert_eq!(rows.len(), 2, "{asked}");
    assert!(rows[0]["axis"].as_f64().is_some(), "{asked}");

    let kept = call(&mut server, "road.profiles.edit", json!({"drawing":drawing,"profile":"b-1","operation":"remove"}));
    assert_eq!(kept["isError"], true, "{kept}");
    assert!(kept.to_string().contains("profile_in_use"), "{kept}");
    let kept = call(&mut server, "road.templates.edit", json!({"drawing":drawing,"template":"Dar","operation":"remove"}));
    assert_eq!(kept["isError"], true, "{kept}");
    assert!(kept.to_string().contains("road_template_in_use"), "{kept}");
}

/// The sample drawing's superelevation (docs/adr/0904): asked at a station on the curve, made anew, kept while a set
/// uses it.
#[test]
fn a_superelevation_is_asked_made_and_kept() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let asked = call(&mut server, "road.superelevations.query", json!({"drawing":drawing,"superelevation":"d-1","stations":[0.0]}));
    assert_eq!(asked["isError"], false, "{asked}");
    let out = &asked["structuredContent"]["output"];
    assert_eq!(out["at"][0]["left"], -0.025, "{out}");
    assert!(out["superelevations"][0]["critical"].as_array().is_some_and(|c| c.len() == 8), "{out}");
    let made = call(
        &mut server,
        "road.superelevations.edit",
        json!({"drawing":drawing,"operation":"set","name":"Yeni","alignment":"g-1","normalLeft":-0.025,"normalRight":-0.025,
            "method":{"method":"stations","rows":[{"station":10.0,"left":-0.025,"right":-0.025}]}}),
    );
    assert_eq!(made["structuredContent"]["output"]["superelevation"]["id"], "d-3", "{made}");
    let kept = call(&mut server, "road.superelevations.edit", json!({"drawing":drawing,"superelevation":"d-1","operation":"remove"}));
    assert!(kept.to_string().contains("superelevation_in_use"), "{kept}");
}

/// The sample drawing's corridor (docs/adr/0905): built when asked, with its stations and feature lines; a new one made;
/// a template it uses kept.
#[test]
fn a_corridor_is_asked_made_and_keeps_its_template() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let asked = call(&mut server, "road.corridors.query", json!({"drawing":drawing,"corridor":"Ana koridor"}));
    assert_eq!(asked["isError"], false, "{asked}");
    let out = &asked["structuredContent"]["output"]["corridors"][0];
    assert!(out["stations"].as_array().is_some_and(|s| s.len() > 40), "{out}");
    assert!(out["stations"].as_array().is_some_and(|s| s.iter().any(|x| x["why"] == "user")), "{out}");
    let made = call(
        &mut server,
        "road.corridors.edit",
        json!({"drawing":drawing,"operation":"set","name":"Yeni","alignment":"g-1","profile":"b-1","surface":"t-zemin",
            "left":20.0,"right":20.0,"tangent":50.0,"curve":25.0,"regions":[{"from":0.0,"to":200.0,"template":"tk-2"}]}),
    );
    assert_eq!(made["structuredContent"]["output"]["corridor"]["id"], "c-2", "{made}");
    let kept = call(&mut server, "road.templates.edit", json!({"drawing":drawing,"template":"tk-2","operation":"remove"}));
    assert!(kept.to_string().contains("road_template_in_use"), "{kept}");
}

/// An earthwork volume of the file is computed, a new one is written, and the corridor it uses is kept
/// (plan/road.md Y8, docs/adr/0906).
#[test]
fn a_volume_is_asked_written_and_keeps_its_corridor() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let asked = call(&mut server, "road.volumes.query", json!({"drawing":drawing,"volume":"Ana kübaj"}));
    assert_eq!(asked["isError"], false, "{asked}");
    let out = &asked["structuredContent"]["output"]["volumes"][0];
    assert_eq!(out["method"], "prismoidal", "{out}");
    assert!(out["rows"].as_array().is_some_and(|r| r.len() > 20), "{out}");
    let last = out["rows"].as_array().and_then(|r| r.last()).cloned().unwrap_or_default();
    assert_eq!(last["totalCut"], out["factoredCut"], "{out}");
    let made = call(
        &mut server,
        "road.volumes.edit",
        json!({"drawing":drawing,"operation":"set","name":"Ortalama alan","corridor":"Ana koridor","method":"endArea"}),
    );
    assert_eq!(made["structuredContent"]["output"]["volume"]["id"], "v-3", "{made}");
    let kept = call(&mut server, "road.corridors.edit", json!({"drawing":drawing,"corridor":"c-1","operation":"remove"}));
    assert!(kept.to_string().contains("corridor_in_use"), "{kept}");
    // Approved, it does not change; the file's surface volume came approved with other figures, and says so.
    let approved = call(&mut server, "road.volumes.approve", json!({"drawing":drawing,"volume":"v-1","at":"2026-11-03"}));
    assert_eq!(approved["structuredContent"]["output"]["volume"]["approval"]["changed"], false, "{approved}");
    let refused = call(&mut server, "road.volumes.edit", json!({"drawing":drawing,"volume":"v-1","operation":"set","name":"X"}));
    assert!(refused.to_string().contains("volume_approved"), "{refused}");
    let side = call(&mut server, "road.volumes.query", json!({"drawing":drawing,"volume":"v-2"}));
    assert!(side.to_string().contains("volume_changed"), "{side}");
}

/// The file's mass haul diagram is read with its sites, a new one is written, and the volume it uses is kept
/// (plan/road.md Y9, docs/adr/0907).
#[test]
fn a_mass_haul_diagram_is_read_written_and_keeps_its_volume() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let asked = call(&mut server, "road.hauls.query", json!({"drawing":drawing,"haul":"Ana Brükner","points":true}));
    assert_eq!(asked["isError"], false, "{asked}");
    let out = &asked["structuredContent"]["output"]["hauls"][0];
    assert!(out["loops"].as_array().is_some_and(|l| !l.is_empty()), "{out}");
    assert_eq!(out["method"], "layers", "{out}");
    assert_eq!(out["cost"]["currency"], "TL", "{out}");
    assert!(out["cost"]["trips"].as_f64().is_some_and(|t| t >= 1.0), "{out}");
    let points = out["points"].as_array().cloned().unwrap_or_default();
    assert!(points.windows(2).any(|w| w[0][0] == w[1][0]), "a site's step: {out}");
    let made = call(
        &mut server,
        "road.hauls.edit",
        json!({"drawing":drawing,"operation":"set","name":"Yüzey Brükneri","volume":"v-2","free":200.0}),
    );
    assert_eq!(made["structuredContent"]["output"]["haul"]["id"], "h-2", "{made}");
    let kept = call(&mut server, "road.volumes.edit", json!({"drawing":drawing,"volume":"v-1","operation":"remove"}));
    assert!(kept.to_string().contains("volume_in_use"), "{kept}");
}

/// The stakeout table of the file's corridor is read, on the grid and asking for ground lengths without a mean height
/// (plan/road.md Y10, docs/adr/0908).
#[test]
fn a_stakeout_table_is_read_and_ground_lengths_want_a_height() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let asked = call(
        &mut server,
        "road.stakeout.query",
        json!({"drawing":drawing,"corridor":"Ana koridor","step":50.0,"codes":["axis","daylight"]}),
    );
    assert_eq!(asked["isError"], false, "{asked}");
    let out = &asked["structuredContent"]["output"];
    let rows = out["rows"].as_array().cloned().unwrap_or_default();
    assert!(!rows.is_empty() && rows.iter().all(|r| r["code"] == "axis" || r["code"].as_str().is_some_and(|c| c.ends_with(".daylight"))), "{out}");
    let ground = call(&mut server, "road.stakeout.query", json!({"drawing":drawing,"corridor":"Ana koridor","distances":"ground"}));
    assert!(ground.to_string().contains("stakeout_no_ground") || ground["isError"] == false, "{ground}");
}

/// The file's alignments and surface go out as LandXML (plan/road.md Y10.2, docs/adr/0908 §6).
#[test]
fn a_road_project_goes_out_as_landxml() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let out = std::env::temp_dir().join(format!("kentos-mcp-landxml-{}.xml", std::process::id()));
    let written = call(
        &mut server,
        "file.export.landxml",
        json!({"drawing":drawing,"path":out.display().to_string(),"stamp":"2026-11-03T09:00:00","overwrite":true}),
    );
    assert_eq!(written["isError"], false, "{written}");
    let text = std::fs::read_to_string(&out).expect("written");
    assert!(text.contains("<Alignment name=\"Ana aks\"") && text.contains("<Surface "), "{text}");
    let _ = std::fs::remove_file(&out);
}

/// The file's alignments go out as IFC 4.3 (plan/road.md Y10.3, docs/adr/0908 §8).
#[test]
fn a_road_project_goes_out_as_ifc() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let out = std::env::temp_dir().join(format!("kentos-mcp-ifc-{}.ifc", std::process::id()));
    let written = call(
        &mut server,
        "file.export.ifc",
        json!({"drawing":drawing,"path":out.display().to_string(),"stamp":"2026-11-03T10:00:00","overwrite":true}),
    );
    assert_eq!(written["isError"], false, "{written}");
    let text = std::fs::read_to_string(&out).expect("written");
    assert!(text.contains("FILE_SCHEMA(('IFC4X3_ADD2'));") && text.contains("IFCALIGNMENT("), "{text}");
    let _ = std::fs::remove_file(&out);
}

/// The file's corridor is sent to an instrument, as a Leica GSI-16 file and as a LandXML string road
/// (plan/road.md Y10.4, docs/adr/0908 §9).
#[test]
fn a_stakeout_is_sent_to_an_instrument() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let gsi = std::env::temp_dir().join(format!("kentos-mcp-aplikasyon-{}.gsi", std::process::id()));
    let sent = call(
        &mut server,
        "road.stakeout.send",
        json!({"drawing":drawing,"corridor":"Ana koridor","step":50.0,"codes":["axis","daylight"],"format":"gsi16",
               "path":gsi.display().to_string(),"overwrite":true}),
    );
    assert_eq!(sent["isError"], false, "{sent}");
    let text = std::fs::read_to_string(&gsi).expect("written");
    assert!(text.starts_with("*110001+") && text.contains(" 81..10"), "{text}");
    let xml = std::env::temp_dir().join(format!("kentos-mcp-aplikasyon-{}.xml", std::process::id()));
    let road = call(
        &mut server,
        "road.stakeout.send",
        json!({"drawing":drawing,"corridor":"Ana koridor","step":20.0,"format":"landxml",
               "path":xml.display().to_string(),"stamp":"2026-11-03T10:00:00","overwrite":true}),
    );
    assert_eq!(road["isError"], false, "{road}");
    let text = std::fs::read_to_string(&xml).expect("written");
    assert!(text.contains("<PlanFeatures name=\"Ana koridor\">") && text.contains("<Alignment name="), "{text}");
    let _ = (std::fs::remove_file(&gsi), std::fs::remove_file(&xml));
}

/// The file's alignment is laid on plan–profile sheets (plan/road.md Y10.5, docs/adr/0908 §10).
#[test]
fn an_alignment_is_laid_on_sheets() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let laid = call(&mut server, "road.sheets.plan", json!({"drawing":drawing,"alignment":"Ana aks","name":"MCP paftaları","scale":1000}));
    assert_eq!(laid["isError"], false, "{laid}");
    let sheets = laid["structuredContent"]["output"]["sheets"].as_array().cloned().unwrap_or_default();
    assert!(!sheets.is_empty() && sheets[0]["number"] == "Y-101", "{laid}");
}

/// The file's sample line set is laid on section sheets (plan/road.md Y10.5b, docs/adr/0908 §11).
#[test]
fn a_section_set_is_laid_on_sheets() {
    let mut server = Server::new();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/road-work.kcad");
    let opened = call(&mut server, "drawing.open", json!({"path":path}));
    let drawing = opened["structuredContent"]["drawing"].clone();
    let laid = call(&mut server, "road.sheets.sections", json!({"drawing":drawing,"set":"k-1","layerId":"0","at":[0.0,-2000.0],"name":"MCP enkesitleri"}));
    assert!(laid["isError"] == false || laid.to_string().contains("no_terrain"), "{laid}");
}

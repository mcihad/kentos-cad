//! CAD-ADV-14's acceptance (docs/adr/1060; plan/cad-advanced.md): a drawing with a reference network three deep, two
//! different drawings of one name and a PDF underlay. The source moved and renamed is found by its content, never
//! by its name; changed, its reload is previewed (what changes, which ties follow, which break) and one undo brings
//! the model back; a PDF page whose stroke went keeps the other strokes' ties by content, not by their places. The
//! package opens in another folder with nothing else of the first one: every reference loaded, the clip and the
//! layer overrides kept, the nested sources found inside the package. No command writes a source file.
//!
//! Every expected value is written here from the geometry the test makes (lines and PDF strokes at known points),
//! not read from KentOS.

use super::*;
use std::path::Path;

fn cad_drawing() -> Session {
    Session::new(&NewDrawing {
        name: "Kaynak".into(),
        srid: 0,
        plot_scale: 50.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
        annotation: None,
        cad_work: None,
        heights: None,
    })
    .expect("a new drawing")
}

fn call(s: &mut Session, command: &str, op: &str, input: Value) -> Value {
    s.call(json!({ "command": command, "op": op, "input": input }))
        .expect("answers")
}

fn ok(answer: &Value) -> &Value {
    assert_eq!(answer["status"], "completed", "{answer}");
    &answer["output"]
}

fn active(s: &Session) -> String {
    s.document().layers().active().to_owned()
}

/// A drawing of these lines (x1, y1, x2, y2), saved at `path`; references attached first when given.
fn lines_at(path: &Path, lines: &[[f64; 4]], refs: &[(&Path, &str)]) {
    let mut s = cad_drawing();
    let layer = active(&s);
    let objects: Vec<Value> = lines
        .iter()
        .map(|l| json!({ "geometry": { "kind": "line", "a": { "x": l[0], "y": l[1] }, "b": { "x": l[2], "y": l[3] } } }))
        .collect();
    ok(&call(
        &mut s,
        "cad.entities.create",
        "execute",
        json!({ "layerId": layer, "objects": objects }),
    ));
    for (file, name) in refs {
        ok(&call(
            &mut s,
            "cad.xrefs.edit",
            "execute",
            json!({
            "operation": { "kind": "attach", "path": file, "name": name, "layerId": layer },
            "drawing": path }),
        ));
    }
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("folder");
    s.save(Some(path)).expect("saved");
}

/// A one-page PDF (200 × 200 pt) stroking these lines, its cross-reference table written with the real offsets.
fn pdf(strokes: &[[f64; 4]]) -> Vec<u8> {
    let content: String = strokes
        .iter()
        .map(|s| format!("{} {} m {} {} l S\n", s[0], s[1], s[2], s[3]))
        .collect();
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>".to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (k, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", k + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn sha(path: &Path) -> String {
    kentos_commands::encoding::sha256_hex(&std::fs::read(path).expect("reads"))
}

/// The references as a review lists them, by name.
fn review(s: &mut Session, drawing: &Path) -> Vec<Value> {
    let answer = call(
        s,
        "cad.xrefs.list",
        "execute",
        json!({ "drawing": drawing, "review": true }),
    );
    ok(&answer)["xrefs"].as_array().cloned().unwrap_or_default()
}

fn by_name<'a>(listed: &'a [Value], name: &str) -> &'a Value {
    listed
        .iter()
        .find(|x| x["name"] == name)
        .unwrap_or_else(|| panic!("{name} not listed: {listed:?}"))
}

/// An aligned dimension's input between two places on pieces of the insert `insert` (block paths `a` and `b`).
fn tied(insert: &str, a: (u32, &str), b: (u32, &str), layer: &str) -> Value {
    json!({ "measure": { "kind": "aligned",
                "a": { "kind": "place", "uid": insert, "place": { "kind": a.1 }, "path": [a.0] },
                "b": { "kind": "place", "uid": insert, "place": { "kind": b.1 }, "path": [b.0] } },
            "at": { "x": 0.0, "y": -5.0 }, "layerId": layer })
}

/// Whether a point is (x, y) to a millionth of a millimetre: the paper's metres are a product of factors (pt × 0,0254/72).
fn near(p: &Value, x: f64, y: f64) -> bool {
    p["x"].as_f64().is_some_and(|v| (v - x).abs() < 1e-9)
        && p["y"].as_f64().is_some_and(|v| (v - y).abs() < 1e-9)
}

fn dimension(s: &Session, uid: &str) -> Value {
    s.entity(uid).expect("the dimension")["entity"].clone()
}

/// What the model is: the definitions and the drawing's objects (no revision, no history).
/// A sheet with one fixed map frame named Harita whose own layer overrides are `overrides` (docs/adr/0314 §1).
fn vp_sheet(id: &str, name: &str, item: &str, overrides: Value) -> Value {
    json!({ "export": { "dpi": 300, "fileName": "[% @pafta_adi %]", "format": "png" }, "guides": [], "id": id,
        "items": [{ "bindings": [], "constraints": { "h": "left", "relativeTo": "margins", "v": "top" },
            "frame": { "height": 120000, "left": 20000, "top": 10000, "width": 180000 }, "hidden": false, "id": item,
            "kind": { "clipToAtlasFeature": false, "grids": [], "labelBand": 0, "layers": { "type": "all" },
                "layerOverrides": overrides,
                "overviewFrame": { "cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500 },
                "type": "map", "view": { "center": { "x": 5.0, "y": 4.0 }, "rotation": 0, "scale": 100, "type": "fixed" } },
            "locked": false, "name": "Harita", "opacity": 100, "padding": 0, "printable": true, "rotation": 0 }],
        "name": name,
        "page": { "margins": { "bottom": 10000, "left": 20000, "right": 10000, "top": 10000 }, "orientation": "landscape",
            "paper": "a3", "size": { "height": 297000, "width": 420000 } },
        "snapGrid": { "enabled": false, "spacing": 5000, "visible": false }, "variables": [], "variants": [] })
}

fn model(s: &Session) -> Value {
    let snapshot = serde_json::to_value(s.document().to_snapshot_v2()).expect("json");
    json!({ "blocks": snapshot["blocks"], "entities": snapshot["entities"], "uids": snapshot["uids"] })
}

#[test]
fn a_reference_network_moves_changes_and_packs_without_losing_its_ties() {
    let root = scratch("xref-network");
    let pt = 0.0254 / 72.0;
    // The network: A attaches B, B attaches C (C in a folder of its own); and two different drawings named plan.kcad.
    let c = root.join("kaynak/detay/c.kcad");
    let b = root.join("kaynak/b.kcad");
    let a = root.join("kaynak/a.kcad");
    lines_at(&c, &[[0.0, 0.0, 1.0, 0.0]], &[]);
    lines_at(&b, &[[0.0, 0.0, 2.0, 0.0]], &[(&c, "C")]);
    lines_at(&a, &[[0.0, 0.0, 3.0, 0.0]], &[(&b, "B")]);
    let plan = root.join("x/plan.kcad");
    lines_at(
        &plan,
        &[
            [0.0, 0.0, 10.0, 0.0],
            [10.0, 0.0, 10.0, 8.0],
            [0.0, 8.0, 10.0, 8.0],
        ],
        &[],
    );
    let other = root.join("y/plan.kcad");
    lines_at(&other, &[[0.0, 0.0, 3.0, 3.0]], &[]);
    let kat = root.join("altlik/kat.pdf");
    std::fs::create_dir_all(kat.parent().expect("folder")).expect("folder");
    let s1 = [10.0, 10.0, 110.0, 10.0];
    let s2 = [10.0, 20.0, 110.0, 20.0];
    let s3 = [10.0, 30.0, 110.0, 30.0];
    std::fs::write(&kat, pdf(&[s1, s2, s3])).expect("written");
    // Source files a reference only reads: read-only on the disk too.
    for f in [&a, &b, &c] {
        let mut perm = std::fs::metadata(f).expect("meta").permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(f, perm).expect("read-only");
    }
    let sources_before: Vec<String> = [&a, &b, &c, &other].iter().map(|f| sha(f)).collect();

    let project = root.join("proje/ana.kcad");
    std::fs::create_dir_all(project.parent().expect("folder")).expect("folder");
    let mut s = cad_drawing();
    let layer = active(&s);
    for (file, name) in [(&a, "A"), (&plan, "Plan")] {
        ok(&call(
            &mut s,
            "cad.xrefs.edit",
            "execute",
            json!({
            "operation": { "kind": "attach", "path": file, "name": name, "layerId": layer }, "drawing": project }),
        ));
    }
    let attached = call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({
        "operation": { "kind": "attach", "path": kat, "name": "Kat", "page": 1, "layerId": layer }, "drawing": project }),
    );
    let kat_insert = ok(&attached)["insert"].as_str().expect("insert").to_owned();
    s.save(Some(&project)).expect("saved");

    // One view of the network: depth, the revision used, what each brought.
    let listed = review(&mut s, &project);
    let depth = |n: &str| by_name(&listed, n)["depth"].as_u64();
    assert_eq!(
        (depth("A"), depth("A|B"), depth("A|B|C")),
        (Some(0), Some(1), Some(2)),
        "{listed:?}"
    );
    assert_eq!(by_name(&listed, "A")["brings"], json!(["A|B"]));
    assert_eq!(by_name(&listed, "A|B")["brings"], json!(["A|B|C"]));
    assert_eq!(by_name(&listed, "A|B|C")["parent"], "A|B");
    assert_eq!(by_name(&listed, "Plan")["digest"], sha(&plan));
    assert!(
        listed
            .iter()
            .all(|x| x["state"] == "loaded" && x["content"] == "source"),
        "{listed:?}"
    );

    // Ties into Plan's pieces: P to the second wall's end and the third's start, Q to the third's end only.
    let plan_insert = s
        .document()
        .entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Insert(i) => Some((i, e.base().id)),
            _ => None,
        })
        .find(|(i, _)| {
            s.document()
                .block(i.block)
                .is_some_and(|b| b.name == "Plan")
        })
        .and_then(|(_, id)| s.document().uid(kentos_domain::Slot(id)))
        .map(|u| u.to_string())
        .expect("Plan's insert");
    let p = ok(&call(
        &mut s,
        "cad.dimensions.create",
        "execute",
        tied(&plan_insert, (1, "end"), (2, "start"), &layer),
    ))["created"][0]
        .as_str()
        .expect("uid")
        .to_owned();
    let q = ok(&call(
        &mut s,
        "cad.dimensions.create",
        "execute",
        tied(&plan_insert, (0, "start"), (2, "end"), &layer),
    ))["created"][0]
        .as_str()
        .expect("uid")
        .to_owned();
    // Ties into the PDF's strokes: K to the second and third, K1 to the first.
    let k = ok(&call(
        &mut s,
        "cad.dimensions.create",
        "execute",
        tied(&kat_insert, (1, "end"), (2, "end"), &layer),
    ))["created"][0]
        .as_str()
        .expect("uid")
        .to_owned();
    let k1 = ok(&call(
        &mut s,
        "cad.dimensions.create",
        "execute",
        tied(&kat_insert, (0, "start"), (1, "start"), &layer),
    ))["created"][0]
        .as_str()
        .expect("uid")
        .to_owned();
    let k_before = dimension(&s, &k);
    assert!(
        near(&k_before["b"], 110.0 * pt, 30.0 * pt),
        "the third stroke's end: {k_before}"
    );
    // The underlay clipped and faded; Plan's layer hidden and not printed here, and two layer states of the sheets.
    ok(&call(
        &mut s,
        "cad.inserts.clip",
        "execute",
        json!({ "uids": [kat_insert], "operation": { "kind": "set",
        "boundary": { "kind": "rectangle", "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 120.0 * pt, "y": 25.0 * pt } } } }),
    ));
    ok(&call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({ "operation": { "kind": "underlay", "name": "Kat", "fade": 40 } }),
    ));
    let plan_layer = s
        .document()
        .block(
            s.document()
                .blocks()
                .iter()
                .find(|b| b.name == "Plan")
                .expect("Plan")
                .id,
        )
        .and_then(|b| b.entities.first().map(|e| e.base().layer_id.clone()))
        .expect("Plan's layer");
    ok(&call(
        &mut s,
        "cad.layers.states",
        "execute",
        json!({ "operation": "save", "name": "Pafta A" }),
    ));
    ok(&call(
        &mut s,
        "cad.layers.set",
        "execute",
        json!({ "operation": "visible", "layer": plan_layer, "value": false }),
    ));
    ok(&call(
        &mut s,
        "cad.layers.set",
        "execute",
        json!({ "operation": "plot", "layer": plan_layer, "value": false }),
    ));
    ok(&call(
        &mut s,
        "cad.layers.states",
        "execute",
        json!({ "operation": "save", "name": "Pafta B" }),
    ));
    let listed = review(&mut s, &project);
    assert_eq!(
        by_name(&listed, "Plan")["layerStates"],
        json!(["Pafta A", "Pafta B"]),
        "{listed:?}"
    );
    // Two sheets' own viewport overrides of Plan's layer (docs/adr/1110): one frozen, one in another colour and weight.
    ok(&call(
        &mut s,
        "cad.sheets.edit",
        "execute",
        json!({ "ops": [
            { "op": "addSheet", "sheet": vp_sheet("a-101", "A-101 Plan", "plan-a", json!([{ "layer": plan_layer, "frozen": true }])) },
            { "op": "addSheet", "sheet": vp_sheet("a-102", "A-102 Renkli", "plan-b",
                json!([{ "layer": plan_layer, "color": "#FF0000", "lineWeight": 700 }])) }
        ] }),
    ));
    let viewports = |listed: &[Value]| by_name(listed, "Plan")["viewportOverrides"].clone();
    let plan_path = s
        .document()
        .layers()
        .get(&plan_layer)
        .map(|n| n.name.clone())
        .expect("named");
    let want_viewports = json!([
        { "sheet": "A-101 Plan", "item": "plan-a", "viewport": "Harita", "layer": format!("Dış başvuru: Plan / {plan_path}"), "frozen": true },
        { "sheet": "A-102 Renkli", "item": "plan-b", "viewport": "Harita", "layer": format!("Dış başvuru: Plan / {plan_path}"),
          "color": "#FF0000", "lineWeight": 700 }
    ]);
    assert_eq!(viewports(&review(&mut s, &project)), want_viewports);
    // “Bu değer nereden geliyor?” of Plan's insert: the reference's layers and the two viewports' own overrides.
    let plan_slot = s
        .document()
        .entities()
        .find(|e| matches!(e, kentos_commands::Entity::Insert(i)
            if s.document().block(i.block).is_some_and(|b| b.name == "Plan")))
        .map(|e| kentos_domain::Slot(e.base().id))
        .expect("Plan's insert");
    let explained = kentos_native_application::appearance_origin::explain(s.document(), plan_slot);
    let part = explained.iter().find(|p| p.part == "xrefLayers").expect("the reference's layers explained");
    assert_eq!(part.viewports.len(), 2, "{part:?}");
    assert!(part.viewports[0].ends_with("— dondurulmuş"), "{part:?}");
    assert!(part.viewports[1].contains("renk #FF0000, kalınlık 0,7 mm"), "{part:?}");

    // Plan's file moved and renamed beside a different drawing of its old name: not found by the name, found by its
    // content as a candidate, bound only when asked.
    let moved = root.join("z/plan-tasindi.kcad");
    std::fs::create_dir_all(moved.parent().expect("folder")).expect("folder");
    std::fs::rename(&plan, &moved).expect("moved");
    let search = [
        other.parent().expect("y").display().to_string(),
        moved.parent().expect("z").display().to_string(),
    ];
    ok(&call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({ "operation": { "kind": "paths", "paths": search } }),
    ));
    let listed = review(&mut s, &project);
    let plan_row = by_name(&listed, "Plan");
    assert_eq!(plan_row["state"], "missing", "{plan_row}");
    assert_eq!(plan_row["content"], "cached");
    assert!(
        plan_row["tried"]
            .as_array()
            .expect("tried")
            .iter()
            .any(|t| t == &json!(format!("{} (içerik özeti farklı)", other.display()))),
        "{plan_row}"
    );
    assert_eq!(
        plan_row["candidates"],
        json!([
            { "path": moved.display().to_string(), "match": "content", "digest": sha(&moved) },
            { "path": other.display().to_string(), "match": "name", "digest": sha(&other) }
        ]),
        "{plan_row}"
    );
    let reload_missing = call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({ "operation": { "kind": "reload", "name": "Plan" }, "drawing": project }),
    );
    assert_eq!(
        reload_missing["error"]["code"], "file_not_found",
        "{reload_missing}"
    );
    let bound = call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({
        "operation": { "kind": "repath", "name": "Plan", "path": moved }, "drawing": project, "diff": true }),
    );
    assert_eq!(
        ok(&bound)["diff"],
        json!({ "same": 3, "changed": 0, "added": 0, "removed": 0 }),
        "{bound}"
    );

    // The architect's next revision: the second wall gone, a fourth line added. Previewed, then written; one undo.
    std::fs::remove_file(&moved).expect("replaced");
    lines_at(
        &moved,
        &[
            [0.0, 0.0, 10.0, 0.0],
            [0.0, 8.0, 10.0, 8.0],
            [0.0, 0.0, 0.0, 8.0],
        ],
        &[],
    );
    let listed = review(&mut s, &project);
    assert_eq!(by_name(&listed, "Plan")["state"], "changed", "{listed:?}");
    assert_eq!(
        by_name(&listed, "Plan")["current"],
        sha(&moved),
        "{listed:?}"
    );
    let before_reload = model(&s);
    let preview = call(
        &mut s,
        "cad.xrefs.edit",
        "plan",
        json!({
        "operation": { "kind": "reload", "name": "Plan" }, "drawing": project, "diff": true }),
    );
    let diff = &ok(&preview)["diff"];
    // The lines are new objects of a new drawing: none is known by its persistent id; the first and third walls are
    // the same content and keep their pieces, the second is gone, the new one is new.
    assert_eq!(
        (
            diff["same"].as_u64(),
            diff["changed"].as_u64(),
            diff["added"].as_u64(),
            diff["removed"].as_u64()
        ),
        (Some(2), Some(0), Some(1), Some(1)),
        "{preview}"
    );
    assert_eq!(
        diff["followers"],
        json!([{ "uid": p, "kind": "dimension", "effect": "breaks" }, { "uid": q, "kind": "dimension", "effect": "follows" }]),
        "{preview}"
    );
    assert!(
        preview["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|w| w["code"] == "ties_break"),
        "{preview}"
    );
    assert_eq!(model(&s), before_reload, "a preview writes nothing");
    let written = call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({
        "operation": { "kind": "reload", "name": "Plan" }, "drawing": project, "diff": true }),
    );
    assert_eq!(ok(&written)["diff"], *diff, "{written}");
    let p_now = dimension(&s, &p);
    assert_eq!(
        p_now["refs"]["a"]["broken"], "piece_gone",
        "the wall that went: {p_now}"
    );
    assert_eq!(
        p_now["refs"]["b"]["path"],
        json!([1]),
        "the third wall's start, now the second piece: {p_now}"
    );
    assert!(p_now["refs"]["b"]["broken"].is_null(), "{p_now}");
    // Its point a stays where the wall's end was: (10, 8), never the new piece's.
    assert_eq!(
        (p_now["a"]["x"].as_f64(), p_now["a"]["y"].as_f64()),
        (Some(10.0), Some(8.0)),
        "{p_now}"
    );
    let q_now = dimension(&s, &q);
    assert_eq!(
        (
            q_now["refs"]["a"]["path"].clone(),
            q_now["refs"]["b"]["path"].clone()
        ),
        (json!([0]), json!([1])),
        "{q_now}"
    );
    assert!(s.document().can_undo());
    assert_eq!(
        s.undo().as_deref(),
        Some("Dış başvuruyu yeniden yükle: Plan")
    );
    assert_eq!(
        model(&s),
        before_reload,
        "one undo brings back the model before the reload"
    );
    // Taken again, as the user would after the preview.
    ok(&call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({
        "operation": { "kind": "reload", "name": "Plan" }, "drawing": project }),
    ));

    // The PDF page without its first stroke: the other strokes keep their ties by their content.
    std::fs::write(&kat, pdf(&[s2, s3])).expect("written");
    let reloaded = call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({
        "operation": { "kind": "reload", "name": "Kat" }, "drawing": project, "diff": true }),
    );
    let kd = &ok(&reloaded)["diff"];
    assert_eq!(
        (
            kd["same"].as_u64(),
            kd["added"].as_u64(),
            kd["removed"].as_u64()
        ),
        (Some(2), Some(0), Some(1)),
        "{reloaded}"
    );
    let k_now = dimension(&s, &k);
    assert!(
        k_now["refs"]["a"]["broken"].is_null() && k_now["refs"]["b"]["broken"].is_null(),
        "{k_now}"
    );
    assert_eq!(
        (
            k_now["refs"]["a"]["path"].clone(),
            k_now["refs"]["b"]["path"].clone()
        ),
        (json!([0]), json!([1])),
        "{k_now}"
    );
    assert!(
        near(&k_now["b"], 110.0 * pt, 30.0 * pt),
        "still the third stroke's end: {k_now}"
    );
    let k1_now = dimension(&s, &k1);
    assert_eq!(
        k1_now["refs"]["a"]["broken"], "piece_gone",
        "the stroke that went: {k1_now}"
    );
    // The underlay's clip and look are the insert's and the reference's: a reload keeps them.
    let insert = s.entity(&kat_insert).expect("insert");
    assert!(insert["entity"]["clipping"].is_object(), "{insert}");

    // Layers back to the source's: the drawing's own override goes, the layer states (the sheets' own) stay.
    let reset = call(
        &mut s,
        "cad.xrefs.edit",
        "execute",
        json!({ "operation": { "kind": "resetLayers", "name": "Plan" }, "drawing": project }),
    );
    assert!(
        !ok(&reset)["reset"].as_array().expect("reset").is_empty(),
        "{reset}"
    );
    let node = s
        .document()
        .layers()
        .get(&plan_layer)
        .expect("the layer")
        .clone();
    assert!(node.visible && node.plot.unwrap_or(true), "{node:?}");
    let states = &s.document().settings().layer_states;
    let in_b = states
        .iter()
        .find(|st| st.name == "Pafta B")
        .and_then(|st| st.nodes.iter().find(|n| n.node == plan_layer));
    assert_eq!(
        in_b.map(|n| (n.visible, n.plot)),
        Some((false, Some(false))),
        "{states:?}"
    );
    // The viewports' own overrides went through the repath and the reset (docs/adr/1110 §1).
    assert_eq!(viewports(&review(&mut s, &project)), want_viewports);
    // The overrides of the sheets back on the drawing for the package.
    ok(&call(
        &mut s,
        "cad.layers.set",
        "execute",
        json!({ "operation": "visible", "layer": plan_layer, "value": false }),
    ));
    s.save(Some(&project)).expect("saved");

    // The package, opened elsewhere with nothing of the first folders.
    let out = root.join("teslim");
    let packed = call(
        &mut s,
        "file.package",
        "execute",
        json!({
        "path": out, "name": "ana", "from": project.parent().expect("folder") }),
    );
    let files: Vec<String> = ok(&packed)["files"]
        .as_array()
        .expect("files")
        .iter()
        .filter(|f| f["kind"] == "xref")
        .map(|f| f["path"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(files.len(), 5, "A, B, C, the plan and the PDF: {packed}");
    assert!(ok(&packed)["left"].is_null(), "nothing left out: {packed}");
    let elsewhere = root.join("baska/teslim");
    std::fs::create_dir_all(elsewhere.parent().expect("folder")).expect("folder");
    std::fs::rename(&out, &elsewhere).expect("moved");
    assert_eq!(
        [&a, &b, &c, &other]
            .iter()
            .map(|f| sha(f))
            .collect::<Vec<_>>(),
        sources_before,
        "no command wrote a source file"
    );
    for gone in ["kaynak", "x", "y", "z", "altlik", "proje"] {
        std::fs::remove_dir_all(root.join(gone)).expect("removed");
    }
    let opened_at = elsewhere.join("ana.kcad");
    let mut there = Session::open(&opened_at).expect("the package opens");
    let listed = review(&mut there, &opened_at);
    assert!(
        listed.iter().all(|x| x["state"] == "loaded"),
        "every reference found in the package: {listed:?}"
    );
    let insert = there.entity(&kat_insert).expect("insert");
    assert!(
        insert["entity"]["clipping"].is_object(),
        "the clip came along: {insert}"
    );
    assert_eq!(by_name(&listed, "Kat")["underlay"], json!({ "fade": 40 }));
    let node = there
        .document()
        .layers()
        .get(&plan_layer)
        .expect("the layer")
        .clone();
    assert!(!node.visible, "the drawing's override came along");
    assert_eq!(
        by_name(&listed, "Plan")["layerStates"],
        json!(["Pafta A", "Pafta B"])
    );
    assert_eq!(viewports(&listed), want_viewports, "the two sheets' overrides came along");
    // The package says what it carries of the drawing's resources (docs/adr/1111): the built-in line type at least.
    assert!(
        ok(&packed)["resources"]
            .as_array()
            .is_some_and(|r| r.iter().any(|x| x["kind"] == "lineType" && x["how"] == "builtIn")),
        "{packed}"
    );
    // A reload inside the package finds the nested sources there: B beside A, C by its name beside them.
    ok(&call(
        &mut there,
        "cad.xrefs.edit",
        "execute",
        json!({ "operation": { "kind": "reload", "name": "A" }, "drawing": opened_at }),
    ));
    let listed = review(&mut there, &opened_at);
    for n in ["A", "A|B", "A|B|C"] {
        let row = by_name(&listed, n);
        assert_eq!(row["state"], "loaded", "{row}");
        assert!(
            row["path"]
                .as_str()
                .unwrap_or_default()
                .starts_with(&elsewhere.display().to_string()),
            "{row}"
        );
    }
    // C was in a folder of its own beside A's: its relative path names nothing in the package, its name beside A does,
    // with the same bytes; it is bound there now.
    assert_eq!(by_name(&listed, "A|B|C")["via"], "path", "{listed:?}");
    let _ = std::fs::remove_dir_all(&root);
}

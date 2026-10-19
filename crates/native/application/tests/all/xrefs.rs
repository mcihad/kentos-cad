//! External references beyond the shared cases (docs/adr/0348): a reload moves a tie into the reference's pieces to
//! the same source object's new place and breaks one whose object the source no longer has; a source whose own
//! attached reference points back at it, or at the open drawing, is a cycle; a save writes the relative path.

use std::path::{Path, PathBuf};

use kentos_contracts::{CallOp, Entity};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/xref/v1")
}

/// A folder of this test's own under the system's temporary one.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-xrefs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

fn run(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), command, op, input).expect("runs")
}

fn ok(answer: &Value) -> &Value {
    assert_eq!(answer["status"], "completed", "{answer}");
    &answer["output"]
}

fn doc() -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Pafta",
        "settings": { "srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 50, "workspace": "cad" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

fn attach(doc: &mut Document, file: &Path, drawing: Option<&Path>) -> Value {
    let mut input = json!({ "operation": { "kind": "attach", "path": file, "name": "Plan", "layerId": "cizim" } });
    if let Some(d) = drawing {
        input["drawing"] = json!(d);
    }
    run(doc, "cad.xrefs.edit", CallOp::Execute, input)
}

fn state(doc: &mut Document, drawing: &Path) -> String {
    let answer = run(doc, "cad.xrefs.list", CallOp::Execute, json!({ "drawing": drawing }));
    let listed = ok(&answer)["xrefs"].as_array().cloned().unwrap_or_default();
    let plan = listed.iter().find(|x| x["name"] == "Plan").expect("Plan is listed");
    plan["state"].as_str().unwrap_or("").to_owned()
}

#[test]
fn a_reload_follows_the_ties_into_the_pieces() {
    let dir = scratch("reload");
    let file = dir.join("plan.kcad");
    std::fs::copy(fixtures().join("plan.kcad"), &file).expect("copy");
    let drawing = dir.join("ana.kcad");
    let mut d = doc();
    let answer = attach(&mut d, &file, Some(&drawing));
    let insert = ok(&answer)["insert"].as_str().expect("insert").to_owned();
    // A dimension tied to the door's circle (the definition's third piece, at 2) and to the second wall's end (at 1).
    let dimension: Entity = serde_json::from_value(json!({
        "kind": "dimension", "id": 0, "layerId": "cizim", "attrs": {}, "a": { "x": 2.0, "y": 1.0 },
        "b": { "x": 10.0, "y": 4.0 }, "offset": 1.0, "height": 0.25,
        "refs": {
            "a": { "uid": insert, "place": { "kind": "center" }, "path": [2] },
            "b": { "uid": insert, "place": { "kind": "end" }, "path": [1] }
        }
    }))
    .expect("dimension");
    let slot: Slot = d.add(dimension).expect("added");
    assert_eq!(state(&mut d, &drawing), "loaded");
    // The architect's next version: the second wall gone, so the circle is the second piece now, its radius 1.
    std::fs::copy(fixtures().join("plan2.kcad"), &file).expect("copy");
    assert_eq!(state(&mut d, &drawing), "changed");
    let answer = run(&mut d, "cad.xrefs.edit", CallOp::Execute,
                     json!({ "operation": { "kind": "reload", "name": "Plan" }, "drawing": drawing }));
    assert_eq!(ok(&answer)["layers"], json!(["Merdiven"]));
    assert_eq!(state(&mut d, &drawing), "loaded");
    let Some(Entity::Dimension(dim)) = d.get(slot) else { panic!("the dimension is gone") };
    let refs = dim.refs.as_ref().expect("its ties");
    let a = refs.a.as_ref().expect("a");
    let b = refs.b.as_ref().expect("b");
    assert_eq!((a.path.clone(), a.broken.as_deref()), (vec![1], None), "the circle's new place");
    // Its id is in the new definition no more: the place it had holds the circle, so the tie breaks (the core's
    // rule, docs/adr/0315 §1), never moved silently onto another object.
    assert!(b.broken.is_some(), "the wall went: {b:?}");
    assert_eq!((dim.a.x, dim.a.y), (2.0, 1.0), "the circle's centre, where the insert puts it");
    // Undo puts the old content and the ties back.
    assert!(d.undo().is_some());
    let Some(Entity::Dimension(dim)) = d.get(slot) else { panic!("the dimension is gone") };
    assert_eq!(dim.refs.as_ref().and_then(|r| r.a.as_ref()).map(|a| a.path.clone()), Some(vec![2]));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `plan.kcad` with its Detay reference's path set to `to`, written at `at`.
fn pointing(at: &Path, to: &Path) {
    let bytes = std::fs::read(fixtures().join("plan.kcad")).expect("read");
    let mut snapshot = kentos_kcad::decode(&bytes).expect("decode");
    for b in &mut snapshot.blocks {
        if let Some(x) = b.xref.as_mut()
            && b.name == "Detay"
        {
            x.path = to.to_string_lossy().into_owned();
        }
    }
    std::fs::write(at, kentos_kcad::encode_verified(&snapshot).expect("encode")).expect("write");
}

#[test]
fn a_source_pointing_back_is_a_cycle() {
    let dir = scratch("cycle");
    let file = dir.join("dongu.kcad");
    pointing(&file, &file);
    let mut d = doc();
    let answer = attach(&mut d, &file, None);
    assert_eq!(answer["error"]["code"], "xref_cycle", "{answer}");
    assert!(answer["error"]["message"].as_str().unwrap_or("").contains("“Detay”"), "{answer}");
    // Back at the open drawing.
    let drawing = dir.join("ana.kcad");
    let other = dir.join("plan.kcad");
    pointing(&other, &drawing);
    let answer = attach(&mut d, &other, Some(&drawing));
    assert_eq!(answer["error"]["code"], "xref_cycle", "{answer}");
    assert!(!d.can_undo());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_save_writes_the_relative_path() {
    let dir = scratch("save");
    std::fs::create_dir_all(dir.join("mimari")).expect("folder");
    let file = dir.join("mimari").join("plan.kcad");
    std::fs::copy(fixtures().join("plan.kcad"), &file).expect("copy");
    let mut d = doc();
    ok(&attach(&mut d, &file, None));
    let mut snapshot = d.to_snapshot_v2();
    kentos_native_application::xrefs::with_relative(&mut snapshot.blocks, &dir.join("ana.kcad"));
    let plan = snapshot.blocks.iter().find(|b| b.name == "Plan").and_then(|b| b.xref.clone()).expect("the reference");
    assert_eq!(plan.relative.as_deref(), Some("mimari/plan.kcad"));
    // The source moved with the drawing to another folder: found by its relative path.
    let moved = scratch("moved");
    std::fs::create_dir_all(moved.join("mimari")).expect("folder");
    std::fs::copy(&file, moved.join("mimari").join("plan.kcad")).expect("copy");
    let _ = std::fs::remove_dir_all(&dir);
    let bytes = kentos_kcad::encode_verified(&snapshot).expect("encode");
    let mut opened = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).expect("decode")).expect("open");
    assert_eq!(state(&mut opened, &moved.join("ana.kcad")), "loaded");
    let _ = std::fs::remove_dir_all(&moved);
}

/// A PDF underlay (docs/adr/0351 §2, §6): a dimension bound to its lines is said to be read off paper; a point of the
/// drawing's own is not.
#[test]
fn a_dimension_on_a_pdf_underlay_is_said_to_be_read_off_paper() {
    let mut doc = doc();
    let pdf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/pdf/v1/cizgiler.pdf");
    let attached = run(
        &mut doc,
        "cad.xrefs.edit",
        CallOp::Execute,
        json!({ "operation": { "kind": "attach", "path": pdf, "page": 1, "layerId": "cizim" } }),
    );
    let insert = ok(&attached)["insert"].as_str().expect("the insert").to_owned();
    // The PDF's first stroke, 10,10 → 110,10 pt: its end at 110 pt from the page's corner, in paper metres.
    let end = 110.0 * 0.0254 / 72.0;
    let low = 10.0 * 0.0254 / 72.0;
    let measure = |a: Value| {
        json!({ "measure": { "kind": "aligned", "a": a, "b": { "kind": "at", "at": { "x": 0.0, "y": 0.0 } } },
                "at": { "x": 0.0, "y": -0.01 }, "layerId": "cizim" })
    };
    let on_paper = run(
        &mut doc,
        "cad.dimensions.create",
        CallOp::Plan,
        measure(json!({ "kind": "place", "uid": insert, "place": { "kind": "end" }, "path": [0] })),
    );
    let codes: Vec<&str> = on_paper["warnings"].as_array().into_iter().flatten().filter_map(|w| w["code"].as_str()).collect();
    assert!(codes.contains(&"underlay_measure"), "{on_paper}");
    let own = run(&mut doc, "cad.dimensions.create", CallOp::Plan, measure(json!({ "kind": "at", "at": { "x": end, "y": low } })));
    assert_eq!(own["status"], "completed", "{own}");
    assert!(own["warnings"].as_array().is_none_or(|w| w.iter().all(|w| w["code"] != "underlay_measure")), "{own}");
}

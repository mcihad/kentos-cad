//! Planı tanı (plan/architecture-advanced.md M48, docs/adr/8048): an office's ground floor plan of the section building,
//! exploded and drawn in centimetres (scripts/fixtures/arch_recognize_plan.py: walls' outlines on DUVAR, columns on
//! KOLON, doors on KAPI, windows on PENCERE, and grids, stair treads, texts and a bed), taken in by DXF içe aktar and
//! read back as the definition's walls, doors and windows; its IFC model holds as many as the definition (Dijital Proje
//! Yön. Md. 9). The same plan with its doors, windows and columns as blocks (turned, mirrored, their pieces on layer 0)
//! reads alike, the blocks going with what they are.

use kentos_commands::{CallOp, DocumentSnapshotV2, Entity, OpeningForm, OpeningHand, OpeningSide};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn snapshot() -> DocumentSnapshotV2 {
    kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap()
}

/// The section building's storeys and layers with nothing drawn, the office's plan taken in: its objects' ids.
fn imported() -> (Document, Vec<String>) {
    imported_from("kesit-binasi-zemin.dxf")
}

/// [`imported`] of a plan of `fixtures/arch/v1/recognize`, its blocks kept as blocks.
fn imported_from(file: &str) -> (Document, Vec<String>) {
    let mut snap = snapshot();
    snap.entities.clear();
    snap.uids.clear();
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let path = format!(
        "{}/../../../fixtures/arch/v1/recognize/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    let answer = run(
        &mut doc,
        "file.import.dxf",
        CallOp::Execute,
        json!({ "path": path }),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
    let uids = doc
        .entities_with_uids()
        .map(|(u, _)| u.to_string())
        .collect();
    (doc, uids)
}

fn run(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), command, op, input).unwrap()
}

/// A wall's centre line as its two ends, lower first, to the micrometre, and its width.
type Line = ([i64; 2], [i64; 2], i64);

fn um(x: f64) -> i64 {
    (x * 1e6).round() as i64
}

fn line_of(a: kentos_commands::Vec2, b: kentos_commands::Vec2, width: f64) -> Line {
    let (p, q) = ([um(a.x), um(a.y)], [um(b.x), um(b.y)]);
    if p <= q {
        (p, q, um(width))
    } else {
        (q, p, um(width))
    }
}

/// An opening's middle in the plan, to the micrometre, its width, its form; a door's hinge and side as seen along its
/// wall's line from its lower end (a wall drawn the other way round has them the other way).
type Hole = (
    [i64; 2],
    i64,
    OpeningForm,
    Option<(OpeningHand, OpeningSide)>,
);

fn holes(doc: &Document) -> Vec<Hole> {
    let walls: std::collections::HashMap<String, kentos_commands::WallFields> = doc
        .entities_with_uids()
        .filter_map(|(u, e)| match e {
            Entity::Wall(w) => Some((u.to_string(), w.wall.clone())),
            _ => None,
        })
        .collect();
    let mut out: Vec<Hole> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Opening(o) => Some(&o.opening),
            _ => None,
        })
        .filter_map(|o| {
            let w = walls.get(&o.host)?;
            let (a, b) = (w.pts[0], w.pts[1]);
            let len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            let u = ((b.x - a.x) / len, (b.y - a.y) / len);
            let mid = match o.anchor {
                kentos_commands::OpeningAnchor::Start => o.at + o.width / 2.0,
                kentos_commands::OpeningAnchor::Center => o.at,
                kentos_commands::OpeningAnchor::End => o.at - o.width / 2.0,
            };
            let backwards = [um(a.x), um(a.y)] > [um(b.x), um(b.y)];
            let door = (o.form == OpeningForm::Door).then(|| {
                if backwards {
                    (o.hand.other(), o.side.other())
                } else {
                    (o.hand, o.side)
                }
            });
            Some((
                [um(a.x + u.0 * mid), um(a.y + u.1 * mid)],
                um(o.width),
                o.form,
                door,
            ))
        })
        .collect();
    out.sort_by_key(|h| h.0);
    out
}

/// The definition's ground floor: its walls and its openings.
fn definition() -> (Vec<Line>, Vec<Hole>) {
    let snap = snapshot();
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let ground = |e: &Entity| match e {
        Entity::Wall(w) => w.wall.arch.level.as_deref() == Some("zemin"),
        _ => false,
    };
    let mut lines: Vec<Line> = doc
        .entities()
        .filter(|e| ground(e))
        .map(|e| match e {
            Entity::Wall(w) => line_of(w.wall.pts[0], w.wall.pts[1], w.wall.width),
            _ => unreachable!(),
        })
        .collect();
    lines.sort();
    // The 1. kat's walls go: their openings with them.
    let upper: Vec<kentos_domain::Slot> = doc
        .entities()
        .filter(|e| matches!(e, Entity::Wall(_)) && !ground(e))
        .map(|e| kentos_domain::Slot(e.base().id))
        .collect();
    doc.remove(&upper);
    let holes = holes(&doc)
        .into_iter()
        .filter(|h| {
            h.0[1] == um(0.0) || h.0[1] == um(9.0) || h.0[0] == um(4.5) || h.0[0] == um(7.5)
        })
        .collect();
    (lines, holes)
}

/// The ground floor's columns: their middle to the micrometre, width, depth and fill.
type Post = ([i64; 2], i64, Option<i64>, bool);

fn columns_of(doc: &Document) -> Vec<Post> {
    let mut out: Vec<Post> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Column(c) if c.column.arch.level.as_deref().is_none_or(|l| l == "zemin") => {
                Some((
                    [um(c.column.at.x), um(c.column.at.y)],
                    um(c.column.width),
                    c.column.depth.map(um),
                    c.column.fill,
                ))
            }
            _ => None,
        })
        .collect();
    out.sort();
    out
}

fn walls_of(doc: &Document) -> Vec<Line> {
    let mut lines: Vec<Line> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Wall(w) => Some(line_of(w.wall.pts[0], w.wall.pts[1], w.wall.width)),
            _ => None,
        })
        .collect();
    lines.sort();
    lines
}

#[test]
fn the_office_s_ground_floor_reads_back_as_the_definition() {
    let (mut doc, uids) = imported();
    let answer = run(
        &mut doc,
        "cad.arch.recognize",
        CallOp::Execute,
        json!({ "uids": uids, "arch": { "level": "zemin" } }),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
    let out = &answer["output"];
    assert_eq!(
        (
            &out["walls"].as_array().unwrap().len(),
            &out["doors"],
            &out["windows"],
            &out["bare"]
        ),
        (&8, &json!(3), &json!(4), &json!(0)),
        "{out}"
    );
    let (walls, openings) = definition();
    assert_eq!(walls_of(&doc), walls);
    assert_eq!(holes(&doc), openings);
    // The 8 columns where the grids meet the outer walls, 30/30, filled.
    assert_eq!(
        columns_of(&doc),
        columns_of(&Document::from_snapshot_v2(snapshot()).unwrap())
    );
    // 56 wall lines, 3 doors' leaf, swing and two frames, 4 windows' five lines, 8 columns' outline and hatch go; the
    // grids, treads, names and the bed stay and are said.
    assert_eq!(
        out["removed"].as_array().unwrap().len(),
        56 + 3 * 4 + 4 * 5 + 8 * 2,
        "{out}"
    );
    let warning = answer["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "plan_unrecognized")
        .expect("what stays is said");
    // DXF içe aktar takes AKS and MERDIVEN into the drawing's Aks and Merdiven.
    assert_eq!(
        warning["message"],
        "23 nesne tanınmadı ve çizimde kaldı: 23 nesne yok sayılan katmanlarda (Aks, Merdiven, MOBILYA, YAZI)."
    );
    // Every wall on the storey given.
    assert!(doc.entities().all(|e| match e {
        Entity::Wall(w) => w.wall.arch.level.as_deref() == Some("zemin"),
        _ => true,
    }));
    // One undo step.
    assert_eq!(doc.undo().as_deref(), Some("Planı tanı"));
    assert!(walls_of(&doc).is_empty());
    assert_eq!(doc.entities().count(), 23 + 104);
}

#[test]
fn the_plan_kept_with_its_blocks_reads_alike_and_the_blocks_go() {
    let (mut doc, uids) = imported_from("kesit-binasi-zemin-bloklu.dxf");
    let inserts = doc
        .entities()
        .filter(|e| matches!(e, Entity::Insert(_)))
        .count();
    // 3 doors, 4 windows, 8 columns.
    assert_eq!(inserts, 15);
    let answer = run(
        &mut doc,
        "cad.arch.recognize",
        CallOp::Execute,
        json!({ "uids": uids, "arch": { "level": "zemin" } }),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
    let (walls, openings) = definition();
    assert_eq!(walls_of(&doc), walls);
    assert_eq!(holes(&doc), openings);
    assert_eq!(
        columns_of(&doc),
        columns_of(&Document::from_snapshot_v2(snapshot()).unwrap())
    );
    // The 56 wall lines and the 15 blocks go; the grids, treads, names and the bed stay.
    let out = &answer["output"];
    assert_eq!(out["removed"].as_array().unwrap().len(), 56 + 15, "{out}");
    assert!(!doc.entities().any(|e| matches!(e, Entity::Insert(_))));
    assert_eq!(out["left"].as_array().unwrap().len(), 23, "{out}");
}

#[test]
fn its_ifc_model_holds_as_many_walls_doors_and_windows_as_the_definition() {
    let (mut doc, uids) = imported();
    let answer = run(
        &mut doc,
        "cad.arch.recognize",
        CallOp::Execute,
        json!({ "uids": uids, "arch": { "level": "zemin" } }),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
    let path = std::env::temp_dir().join("kentos-plan-recognition.ifc");
    let model = run(
        &mut doc,
        "cad.arch.ifc",
        CallOp::Plan,
        json!({ "path": path.display().to_string() }),
    );
    assert_eq!(model["status"], "completed", "{model}");
    let out = &model["output"];
    assert_eq!(
        (
            &out["walls"],
            &out["doors"],
            &out["windows"],
            &out["columns"]
        ),
        (&json!(8), &json!(3), &json!(4), &json!(8)),
        "{out}"
    );
}

#[test]
fn sources_kept_stay_and_a_plan_without_its_walls_layer_is_refused() {
    let (mut doc, uids) = imported();
    let before = doc.entities().count();
    let answer = run(
        &mut doc,
        "cad.arch.recognize",
        CallOp::Execute,
        json!({ "uids": uids, "keepSources": true }),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
    assert!(
        answer["output"]["removed"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert_eq!(doc.entities().count(), before + 8 + 7 + 8);

    let (mut doc, uids) = imported();
    let duvar = doc
        .entities()
        .map(|e| e.base().layer_id.clone())
        .find(|id| doc.layers().get(id).is_some_and(|n| n.name == "DUVAR"))
        .unwrap();
    let answer = run(
        &mut doc,
        "cad.arch.recognize",
        CallOp::Execute,
        json!({ "uids": uids, "roles": [{ "layerId": duvar, "role": "ignore" }] }),
    );
    assert_eq!(answer["status"], "failed", "{answer}");
    assert_eq!(answer["error"]["code"], "plan_unrecognized");
}

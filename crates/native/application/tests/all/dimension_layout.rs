//! A dimension recipe's sheet scales, its values placed where they do not fit, its rows' rounding, and its copies and
//! moves with its objects (docs/adr/0730): the house of `dimension_spans` drawn on a sheet at twice the drawing's scale,
//! its walls row's thin values moved aside, a value moved by hand kept and given back, a wall whose parts round to more
//! than its overall, and the house copied, arrayed, pasted, moved and mirrored with its recipe. The shared command cases
//! hold the numbers against an independent reference (fixtures/commands/v1/cad.dimensionsets.*.layout.json).

use super::wall_contours::drawing;
use kentos_commands::{CallOp, DimensionSetId, DimensionSetRow, DimensionSetTight, Entity, EntityId};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

const CREATE: &str = "cad.dimensionsets.create";
const EDIT: &str = "cad.dimensionsets.edit";

fn run(doc: &mut Document, name: &str, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), name, op, input).unwrap()
}

fn done(doc: &mut Document, name: &str, input: Value) -> Value {
    let result = run(doc, name, CallOp::Execute, input);
    assert_eq!(result["status"], "completed", "{name}: {result}");
    result["output"].clone()
}

fn p(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

/// A 10 × 8 house on its walls' axes (25 cm), a 10 cm partition at x 4, a window in the south wall from 6 to 7.2, grids
/// at x 0, 4 and 10: the walls' uids (south, east, north, west, partition), the window's and the grids'.
fn house() -> (Document, Vec<String>, String, Vec<String>) {
    let mut doc = drawing(json!([]), "m");
    let mut walls = Vec::new();
    for (a, b, width) in [
        ((0., 0.), (10., 0.), 0.25),
        ((10., 0.), (10., 8.), 0.25),
        ((10., 8.), (0., 8.), 0.25),
        ((0., 8.), (0., 0.), 0.25),
        ((4., 0.), (4., 8.), 0.1),
    ] {
        let wall = json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": width, "justify": "center"});
        let out = done(&mut doc, "cad.walls.create", json!({"layerId": "cizim", "walls": [wall]}));
        walls.push(out["uids"][0].as_str().unwrap().to_owned());
    }
    let out = done(
        &mut doc,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [{"host": walls[0], "at": 6.6, "width": 1.2}]}),
    );
    let window = out["uids"][0].as_str().unwrap().to_owned();
    let mut grids = Vec::new();
    for (x, name) in [(0., "A"), (4., "B"), (10., "C")] {
        let out = done(
            &mut doc,
            "cad.grids.create",
            json!({"layerId": "cizim", "grids": [{"pts": [p(x, -2.), p(x, 10.)], "label": name}]}),
        );
        grids.push(out["uids"][0].as_str().unwrap().to_owned());
    }
    (doc, walls, window, grids)
}

fn facade(doc: &mut Document, walls: &[&String], extra: Value) -> DimensionSetId {
    let mut input = json!({"layerId": "cizim", "walls": walls, "direction": p(1., 0.),
        "rows": ["openings", "walls", "grids", "overall"], "at": p(5., -1.5)});
    for (k, v) in extra.as_object().unwrap() {
        input[k] = v.clone();
    }
    let out = done(doc, CREATE, input);
    DimensionSetId::parse(out["set"].as_str().unwrap()).unwrap()
}

fn dimension(doc: &Document, uid: EntityId) -> kentos_commands::DimensionEntity {
    match doc.slot_of(Uuid::from_bytes(uid.0)).and_then(|s| doc.get(s)) {
        Some(Entity::Dimension(d)) => d.clone(),
        other => panic!("a member dimension: {other:?}"),
    }
}

fn plot_scale(doc: &Document) -> f64 {
    doc.settings().plot_scale
}

#[test]
fn a_sheet_scale_draws_each_row_as_far_out_on_paper() {
    let (mut doc, w, ..) = house();
    let own = plot_scale(&doc);
    let id = facade(&mut doc, &[&w[0]], json!({"scales": [own * 2.0]}));
    let set = doc.dimension_set(id).unwrap().clone();
    assert_eq!(set.scales, [own * 2.0]);
    for m in &set.members {
        let d = dimension(&doc, m.uid);
        let a = d.annotative.as_deref().expect("annotative by the recipe");
        assert_eq!(a.at, own);
        assert_eq!(a.scales.len(), 1);
        // The south outer face is y −0.125: the row's line twice as far below it on the other sheet.
        let line = d.a.y + d.offset;
        let delta = line - -0.125;
        let there = a.scales[0].offset.unwrap();
        assert!((there - (d.offset + delta)).abs() < 1e-9, "{there} vs {}", d.offset + delta);
    }
    // The sheet's map draws them twice as large and as far.
    let sheet = doc.at_scale(own * 2.0).expect("another representation");
    let m = &set.members[0];
    let mine = dimension(&doc, m.uid);
    let theirs = dimension(&sheet, m.uid);
    assert!((theirs.height - mine.height * 2.0).abs() < 1e-12);
    assert!((theirs.offset - mine.annotative.as_ref().unwrap().scales[0].offset.unwrap()).abs() < 1e-12);
    // A sheet at a scale the recipe does not take leaves them out (docs/adr/0323).
    let other = doc.at_scale(own * 4.0).expect("left out");
    assert!(other.slot_of(Uuid::from_bytes(m.uid.0)).is_none());
    // The representations are the recipe's: another edit of them is refused.
    let refused = run(
        &mut doc,
        "cad.annotations.scales",
        CallOp::Execute,
        json!({"uids": [Uuid::from_bytes(m.uid.0).to_string()], "operation": "remove", "scales": [own * 2.0]}),
    );
    assert_eq!(refused["status"], "failed", "{refused}");
    // Their scales replaced: every member follows; none, the recipe leaves them as they are.
    done(&mut doc, EDIT, json!({"id": id, "operation": {"kind": "set", "scales": [own * 2.0, own * 4.0]}}));
    let set = doc.dimension_set(id).unwrap().clone();
    assert!(set.members.iter().all(|m| dimension(&doc, m.uid).annotative.unwrap().scales.len() == 2));
}

#[test]
fn values_that_do_not_fit_go_aside_and_a_value_moved_by_hand_stays() {
    let (mut doc, w, ..) = house();
    let id = facade(&mut doc, &[&w[0]], json!({"tight": "shift"}));
    let plan = run(
        &mut doc,
        EDIT,
        CallOp::Plan,
        json!({"id": id, "operation": {"kind": "set", "gapMm": 7.5}}),
    );
    assert_eq!(plan["status"], "completed", "{plan}");
    let set = doc.dimension_set(id).unwrap().clone();
    let moved: Vec<_> = set
        .members
        .iter()
        .filter(|m| dimension(&doc, m.uid).text_shift.is_some())
        .collect();
    assert!(!moved.is_empty(), "a thin wall's value goes aside");
    for m in &moved {
        assert_eq!(m.shift, dimension(&doc, m.uid).text_shift, "the recipe's place is recorded");
    }
    // A value moved by hand keeps its place through a source edit, and goes back with Yazıları tarife döndür.
    let first = moved[0].uid;
    done(
        &mut doc,
        "cad.dimensions.edit",
        json!({"edit": {"kind": "moveText", "uid": Uuid::from_bytes(first.0).to_string(), "along": 0.0, "across": 0.5}}),
    );
    let mine = dimension(&doc, first).text_shift.unwrap();
    assert_eq!((mine.along, mine.across), (0.0, 0.5));
    done(
        &mut doc,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [{"host": w[0], "at": 2.0, "width": 0.9}]}),
    );
    let mine = dimension(&doc, first).text_shift.unwrap();
    assert_eq!((mine.along, mine.across), (0.0, 0.5), "the user's place kept");
    done(&mut doc, EDIT, json!({"id": id, "operation": {"kind": "resetText"}}));
    let back = dimension(&doc, first).text_shift;
    let recorded = doc
        .dimension_set(id)
        .unwrap()
        .members
        .iter()
        .find(|m| m.uid == first)
        .unwrap()
        .shift;
    assert_eq!(back, recorded, "the recipe's place again");
    assert_ne!(back.map(|s| (s.along, s.across)), Some((0.0, 0.5)));
    // Keeping again: the recipe's places go, the values stand in their own.
    done(&mut doc, EDIT, json!({"id": id, "operation": {"kind": "set", "tight": "keep"}}));
    let set = doc.dimension_set(id).unwrap().clone();
    assert!(set.members.iter().all(|m| m.shift.is_none() && dimension(&doc, m.uid).text_shift.is_none()));
}

#[test]
fn parts_rounding_to_other_than_the_overall_are_said() {
    let mut doc = drawing(json!([]), "m");
    let wall = done(
        &mut doc,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{"pts": [p(0., 0.), p(3.0012, 0.)], "width": 0.2, "justify": "center"}]}),
    )["uids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    done(
        &mut doc,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [{"host": wall, "at": 1.5006, "width": 1.0004}]}),
    );
    let plan = run(
        &mut doc,
        CREATE,
        CallOp::Plan,
        json!({"layerId": "cizim", "wall": wall, "at": p(1.5, 1.5)}),
    );
    assert_eq!(plan["status"], "completed", "{plan}");
    let rounding = &plan["output"]["rounding"];
    assert_eq!(rounding[0]["row"], "openings", "{plan}");
    assert_eq!(rounding[0]["parts"], "3.000");
    assert_eq!(rounding[0]["overall"], "3.001");
    assert!(
        plan["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "dimension_set_rounding"),
        "{plan}"
    );
}

fn everything(w: &[String], window: &str, grids: &[String], doc: &Document, id: DimensionSetId) -> Vec<String> {
    let mut uids: Vec<String> = w.to_vec();
    uids.push(window.to_owned());
    uids.extend(grids.iter().cloned());
    uids.extend(
        doc.dimension_set(id)
            .unwrap()
            .members
            .iter()
            .map(|m| Uuid::from_bytes(m.uid.0).to_string()),
    );
    uids
}

/// Each row's lengths, in order along x, to a micrometre.
fn rows(doc: &Document, id: DimensionSetId) -> Vec<(DimensionSetRow, Vec<f64>)> {
    let set = doc.dimension_set(id).unwrap();
    set.rows
        .iter()
        .map(|r| {
            let mut out: Vec<(f64, f64)> = set
                .members
                .iter()
                .filter(|m| m.row == *r)
                .map(|m| {
                    let d = dimension(doc, m.uid);
                    (d.a.x.min(d.b.x), ((d.b.x - d.a.x).abs() * 1e6).round() / 1e6)
                })
                .collect();
            out.sort_by(|a, b| a.0.total_cmp(&b.0));
            (*r, out.into_iter().map(|(_, l)| l).collect())
        })
        .collect()
}

#[test]
fn a_copy_takes_the_whole_recipe_onto_the_copies() {
    let (mut doc, w, window, grids) = house();
    let id = facade(&mut doc, &[&w[0]], json!({}));
    let before = rows(&doc, id);
    let uids = everything(&w, &window, &grids, &doc, id);
    let out = done(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": uids, "copy": true, "transform": {"kind": "move", "dx": 20.0, "dy": 0.0}}),
    );
    let created: Vec<String> = serde_json::from_value(out["created"].clone()).unwrap();
    assert_eq!(doc.dimension_sets().len(), 2);
    let copy = doc.dimension_sets()[1].clone();
    assert_ne!(copy.id, id);
    assert!(created.contains(&Uuid::from_bytes(copy.wall.0).to_string()), "the copy's source is the wall's copy");
    assert!(copy.members.iter().all(|m| created.contains(&Uuid::from_bytes(m.uid.0).to_string())));
    assert_eq!(rows(&doc, copy.id), before, "the same rows on the copies");
    assert!((copy.at.x - 25.0).abs() < 1e-9 && (copy.at.y - -1.5).abs() < 1e-9);
    // The original stays as it was; one undo takes the copy and its recipe back.
    assert_eq!(rows(&doc, id), before);
    doc.undo();
    assert_eq!(doc.dimension_sets().len(), 1);
}

#[test]
fn an_array_takes_the_recipe_onto_each_item() {
    let (mut doc, w, window, grids) = house();
    let id = facade(&mut doc, &[&w[0]], json!({}));
    let before = rows(&doc, id);
    let uids = everything(&w, &window, &grids, &doc, id);
    done(
        &mut doc,
        "cad.entities.array",
        json!({"uids": uids, "layout": {"kind": "grid", "rows": 1, "cols": 3, "dx": 15.0, "dy": 0.0}}),
    );
    assert_eq!(doc.dimension_sets().len(), 3);
    for set in doc.dimension_sets().to_vec() {
        assert_eq!(rows(&doc, set.id), before);
    }
}

#[test]
fn a_paste_takes_the_recipe_over_the_pasted_objects() {
    let (mut doc, w, window, grids) = house();
    let id = facade(&mut doc, &[&w[0]], json!({}));
    let before = rows(&doc, id);
    let uids = everything(&w, &window, &grids, &doc, id);
    let clip = done(&mut doc, "cad.entities.copy", json!({"uids": uids, "base": p(0., 0.)}));
    assert_eq!(clip["dimensionSets"].as_array().unwrap().len(), 1);
    done(&mut doc, "cad.entities.paste", json!({"clip": clip, "at": p(0., 30.)}));
    assert_eq!(doc.dimension_sets().len(), 2);
    let pasted = doc.dimension_sets()[1].clone();
    assert_eq!(rows(&doc, pasted.id), before);
    assert!((pasted.at.y - 28.5).abs() < 1e-9);
}

#[test]
fn a_move_of_every_source_moves_the_rows_and_a_mirror_keeps_a_walls_face() {
    let (mut doc, w, window, grids) = house();
    let id = facade(&mut doc, &[&w[0]], json!({}));
    let before = rows(&doc, id);
    let uids = everything(&w, &window, &grids, &doc, id);
    done(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": uids, "transform": {"kind": "move", "dx": 0.0, "dy": 50.0}}),
    );
    let set = doc.dimension_set(id).unwrap().clone();
    assert!((set.at.y - 48.5).abs() < 1e-9, "the rows went with the house: {:?}", set.at);
    assert_eq!(rows(&doc, id), before);
    for m in &set.members {
        let d = dimension(&doc, m.uid);
        assert!(d.a.y > 47.0, "{d:?}");
    }
    // Turned half a turn about the house's middle: the same members measure the same intervals.
    let before_ids: Vec<EntityId> = set.members.iter().map(|m| m.uid).collect();
    let uids = everything(&w, &window, &grids, &doc, id);
    done(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": uids, "transform": {"kind": "rotate", "center": p(5., 54.), "angle": std::f64::consts::PI}}),
    );
    let set = doc.dimension_set(id).unwrap().clone();
    let mut after_ids: Vec<EntityId> = set.members.iter().map(|m| m.uid).collect();
    let mut sorted = before_ids.clone();
    sorted.sort();
    after_ids.sort();
    assert_eq!(after_ids, sorted, "the same members");
    assert!((set.at.y - 59.5).abs() < 1e-9, "{:?}", set.at);
    // A wall's own recipe, mirrored as a copy across x = 20: on the copy, its rows on the same side as the original's.
    let mut doc = drawing(json!([]), "m");
    let wall = done(
        &mut doc,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{"pts": [p(0., 0.), p(5., 0.)], "width": 0.2, "justify": "center"}]}),
    )["uids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let out = done(&mut doc, CREATE, json!({"layerId": "cizim", "wall": wall, "at": p(2.5, 1.0)}));
    let id = DimensionSetId::parse(out["set"].as_str().unwrap()).unwrap();
    let set = doc.dimension_set(id).unwrap().clone();
    let mut uids = vec![wall.clone()];
    uids.extend(set.members.iter().map(|m| Uuid::from_bytes(m.uid.0).to_string()));
    done(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": uids, "copy": true, "transform": {"kind": "mirror", "a": p(20., 0.), "b": p(20., 1.)}}),
    );
    let copy = doc.dimension_sets()[1].clone();
    // The mirrored wall's path runs back (docs/adr/0339 §1): its body on the same side of its path, the same face.
    assert_eq!(copy.face, set.face, "the mirrored wall's own face");
    assert!(copy.at.y > 0.5, "its rows above it as the original's: {:?}", copy.at);
    for m in &copy.members {
        let d = dimension(&doc, m.uid);
        assert!((d.a.y - 0.1).abs() < 1e-9 && (d.b.y - 0.1).abs() < 1e-9, "the upper face measured: {d:?}");
        assert!(d.a.y + d.offset > 0.5, "{d:?}");
    }
    // A recipe on an L wall's second edge, mirrored in place: the path runs back, so it is the first edge now.
    let mut doc = drawing(json!([]), "m");
    let wall = done(
        &mut doc,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{"pts": [p(0., 6.), p(0., 0.), p(8., 0.)], "width": 0.2,
                                              "justify": "center"}]}),
    )["uids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let out = done(&mut doc, CREATE, json!({"layerId": "cizim", "wall": wall, "edge": 1, "at": p(4.0, -1.0)}));
    let id = DimensionSetId::parse(out["set"].as_str().unwrap()).unwrap();
    let set = doc.dimension_set(id).unwrap().clone();
    let measured = |doc: &Document, id: DimensionSetId| -> Vec<(f64, f64)> {
        let mut out: Vec<(f64, f64)> = doc
            .dimension_set(id)
            .unwrap()
            .members
            .iter()
            .map(|m| {
                let d = dimension(doc, m.uid);
                ((d.a.x - d.b.x).abs(), d.a.y.max(d.b.y))
            })
            .collect();
        out.sort_by(|a, b| a.partial_cmp(b).unwrap());
        out
    };
    let before = measured(&doc, id);
    let mut uids = vec![wall.clone()];
    uids.extend(set.members.iter().map(|m| Uuid::from_bytes(m.uid.0).to_string()));
    done(
        &mut doc,
        "cad.entities.transform",
        json!({"uids": uids, "transform": {"kind": "mirror", "a": p(0., 0.), "b": p(1., 0.)}}),
    );
    let after = doc.dimension_set(id).unwrap().clone();
    assert_eq!(after.edge, Some(0), "the mirrored path's first edge is the same wall run");
    assert_eq!(after.face, set.face);
    assert!(after.broken.is_none(), "{:?}", after.broken);
    // The same intervals, on the measured face's mirror image (y → −y).
    let now = measured(&doc, id);
    assert_eq!(now.len(), before.len());
    for ((l, y), (l0, y0)) in now.iter().zip(&before) {
        assert!((l - l0).abs() < 1e-9 && (y + y0).abs() < 1e-9, "{now:?} {before:?}");
    }
    assert!((after.at.y - 1.0).abs() < 1e-9, "the rows' place mirrored: {:?}", after.at);
}

/// The KCAD example of schema 129 (fixtures/kcad/v2/dimension-set-scales.json) is the commands' drawing: the house's
/// south facade at the drawing's 1:50, drawn on 1:100 and 1:200 sheets too, its thin values moved aside and one of them
/// moved by hand; the north wall's own recipe on its outer face, drawn at 1:100 too. `KENTOS_WRITE_SCALES_MODEL=1`
/// writes it again.
#[test]
fn the_kcad_example_is_the_commands_drawing() {
    let (mut doc, w, ..) = house();
    let id = facade(&mut doc, &[&w[0]], json!({"scales": [100.0, 200.0], "tight": "shift"}));
    // The north wall runs west: its right face is the outer one.
    done(
        &mut doc,
        CREATE,
        json!({"layerId": "cizim", "wall": w[2], "face": "right", "at": p(5., 9.5), "scales": [100.0]}),
    );
    assert_eq!(doc.dimension_sets().len(), 2);
    let set = doc.dimension_set(id).unwrap().clone();
    let first = set.members.iter().find(|m| m.shift.is_some()).expect("a value moved aside").uid;
    done(
        &mut doc,
        "cad.dimensions.edit",
        json!({"edit": {"kind": "moveText", "uid": Uuid::from_bytes(first.0).to_string(), "along": 0.0, "across": 0.6}}),
    );
    let snapshot = doc.to_snapshot_v2();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/dimension-set-scales.json");
    if std::env::var_os("KENTOS_WRITE_SCALES_MODEL").is_some() {
        std::fs::write(path, serde_json::to_string_pretty(&snapshot).unwrap() + "\n").unwrap();
    }
    let stored: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let reopened = Document::from_snapshot_v2(stored.clone()).unwrap();
    assert_eq!(reopened.to_snapshot_v2().dimension_sets, stored.dimension_sets, "opened unchanged");
    assert_eq!(stored.dimension_sets[1].scales, [100.0]);
    assert!(stored.dimension_sets[1].span.is_none(), "a wall's own recipe");
    let set = &stored.dimension_sets[0];
    assert_eq!(set.scales, [100.0, 200.0]);
    assert_eq!(set.tight, DimensionSetTight::Shift);
    assert!(set.members.iter().any(|m| m.shift.is_some()), "the recipe's places recorded");
    // The value moved by hand stands where the user put it; the recipe's place for it is kept as it was.
    let moved = set.members.iter().find(|m| {
        stored.entities.iter().zip(&stored.uids).any(|(e, uid)| match e {
            Entity::Dimension(d) => {
                *uid == m.uid && d.text_shift.is_some_and(|s| (s.along, s.across) == (0.0, 0.6))
            }
            _ => false,
        })
    });
    assert!(moved.is_some_and(|m| m.shift.is_some_and(|s| (s.along, s.across) != (0.0, 0.6))));
}

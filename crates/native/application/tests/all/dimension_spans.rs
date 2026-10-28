//! Façade and space dimension recipes (docs/adr/0729): a house's south façade reaching its corner walls' outer faces,
//! following a door and a partition added later with its unchanged dimensions kept, a split wall's piece joining and
//! a join giving it back, one wall's recipe split into a façade, a turned wall keeping its face and identities, a
//! deleted source leaving and the last one breaking it, walls on two levels refused and broken, four sides in one
//! step, and a space's outline with its door. The shared command cases hold the rest against an independent
//! reference (fixtures/commands/v1/cad.dimensionsets.create.span.json).

use super::wall_contours::drawing;
use kentos_commands::{
    CallOp, DimensionSetBreak, DimensionSetId, DimensionSetRow, Entity, EntityId,
};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

const CREATE: &str = "cad.dimensionsets.create";

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
/// at x 0, 4 and 10: the walls' uids (south, east, north, west, partition).
fn house(level: Option<&str>) -> (Document, Vec<String>) {
    let mut doc = drawing(json!([]), "m");
    let arch = level.map_or(json!({}), |l| json!({"level": l}));
    let mut uids = Vec::new();
    for (a, b, width) in [
        ((0., 0.), (10., 0.), 0.25),
        ((10., 0.), (10., 8.), 0.25),
        ((10., 8.), (0., 8.), 0.25),
        ((0., 8.), (0., 0.), 0.25),
        ((4., 0.), (4., 8.), 0.1),
    ] {
        let mut wall = json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": width, "justify": "center"});
        if level.is_some() {
            wall["arch"] = arch.clone();
        }
        let out = done(&mut doc, "cad.walls.create", json!({"layerId": "cizim", "walls": [wall]}));
        uids.push(out["uids"][0].as_str().unwrap().to_owned());
    }
    done(
        &mut doc,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [{"host": uids[0], "at": 6.6, "width": 1.2}]}),
    );
    for (x, name) in [(0., "A"), (4., "B"), (10., "C")] {
        done(
            &mut doc,
            "cad.grids.create",
            json!({"layerId": "cizim", "grids": [{"pts": [p(x, -2.), p(x, 10.)], "label": name}]}),
        );
    }
    (doc, uids)
}

fn facade(doc: &mut Document, walls: &[&String], rows: &[&str]) -> DimensionSetId {
    let out = done(
        doc,
        CREATE,
        json!({"layerId": "cizim", "walls": walls, "direction": p(1., 0.), "rows": rows, "at": p(5., -1.5)}),
    );
    DimensionSetId::parse(out["set"].as_str().unwrap()).unwrap()
}

/// Each row's lengths along x, in order, to a micrometre.
fn row(doc: &Document, id: DimensionSetId, which: DimensionSetRow) -> Vec<f64> {
    let set = doc.dimension_set(id).unwrap();
    let mut out: Vec<(f64, f64)> = set
        .members
        .iter()
        .filter(|m| m.row == which)
        .map(|m| {
            let Some(Entity::Dimension(d)) = doc.slot_of(Uuid::from_bytes(m.uid.0)).and_then(|s| doc.get(s))
            else {
                panic!("a member dimension")
            };
            (d.a.x.min(d.b.x), ((d.b.x - d.a.x).abs() * 1e6).round() / 1e6)
        })
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.into_iter().map(|(_, l)| l).collect()
}

fn members(doc: &Document, id: DimensionSetId) -> Vec<EntityId> {
    doc.dimension_set(id).unwrap().members.iter().map(|m| m.uid).collect()
}

#[test]
fn the_south_facade_reaches_the_corners_and_follows_a_door_and_a_partition() {
    let (mut doc, w) = house(None);
    let id = facade(&mut doc, &[&w[0]], &["openings", "walls", "grids", "overall"]);
    assert_eq!(row(&doc, id, DimensionSetRow::Openings), [6.125, 1.2, 2.925]);
    assert_eq!(row(&doc, id, DimensionSetRow::Walls), [0.25, 3.825, 0.1, 5.825, 0.25]);
    assert_eq!(row(&doc, id, DimensionSetRow::Grids), [0.125, 4.0, 6.0, 0.125]);
    assert_eq!(row(&doc, id, DimensionSetRow::Overall), [10.25]);
    let before = members(&doc, id);
    // A door later: the openings row gains it; the overall and the walls keep their dimensions.
    done(
        &mut doc,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [{"host": w[0], "at": 2.0, "width": 0.9}]}),
    );
    assert_eq!(row(&doc, id, DimensionSetRow::Openings), [1.675, 0.9, 3.55, 1.2, 2.925]);
    let after = members(&doc, id);
    let kept = before.iter().filter(|u| after.contains(u)).count();
    assert!(kept >= 9, "the unchanged pairs keep their dimensions: {kept}");
    // A partition meeting the south wall later: the walls row gains its faces.
    done(
        &mut doc,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{"pts": [p(8., 0.), p(8., 8.)], "width": 0.1, "justify": "center"}]}),
    );
    assert_eq!(row(&doc, id, DimensionSetRow::Walls), [0.25, 3.825, 0.1, 3.9, 0.1, 1.825, 0.25]);
    // One undo takes the partition and its dimensions back.
    assert!(doc.undo().is_some());
    assert_eq!(row(&doc, id, DimensionSetRow::Walls), [0.25, 3.825, 0.1, 5.825, 0.25]);
}

#[test]
fn a_split_source_takes_its_piece_and_a_join_gives_it_back() {
    let (mut doc, w) = house(None);
    let id = facade(&mut doc, &[&w[0]], &["openings", "overall"]);
    done(
        &mut doc,
        "cad.walls.edit",
        json!({"uids": [w[0]], "operation": {"kind": "split", "at": p(5., 0.)}}),
    );
    let set = doc.dimension_set(id).unwrap();
    let span = set.span.as_ref().unwrap();
    assert_eq!(span.walls.len(), 1, "the piece joins the façade");
    assert_eq!(row(&doc, id, DimensionSetRow::Overall), [10.25]);
    assert_eq!(row(&doc, id, DimensionSetRow::Openings), [6.125, 1.2, 2.925]);
    let piece = span.walls[0].to_string();
    done(
        &mut doc,
        "cad.walls.edit",
        json!({"uids": [w[0], piece], "operation": {"kind": "join"}}),
    );
    let set = doc.dimension_set(id).unwrap();
    assert!(set.span.as_ref().unwrap().walls.is_empty(), "joined back into one wall");
    assert_eq!(row(&doc, id, DimensionSetRow::Overall), [10.25]);
    assert!(set.broken.is_none());
}

#[test]
fn one_walls_recipe_split_goes_on_as_a_facade() {
    let (mut doc, w) = house(None);
    let out = done(
        &mut doc,
        CREATE,
        json!({"layerId": "cizim", "wall": w[0], "face": "right", "rows": ["overall"], "at": p(5., -1.5)}),
    );
    let id = DimensionSetId::parse(out["set"].as_str().unwrap()).unwrap();
    // One wall's edge: its own face, axis to axis.
    assert_eq!(row(&doc, id, DimensionSetRow::Overall), [10.0]);
    done(
        &mut doc,
        "cad.walls.edit",
        json!({"uids": [w[0]], "operation": {"kind": "split", "at": p(5., 0.)}}),
    );
    let set = doc.dimension_set(id).unwrap();
    let span = set.span.as_ref().expect("a façade now");
    assert_eq!((span.direction.x, span.direction.y), (1.0, 0.0));
    assert_eq!(span.walls.len(), 1);
    assert!(set.edge.is_none());
    // As a façade its ends reach the corner walls' outer faces.
    assert_eq!(row(&doc, id, DimensionSetRow::Overall), [10.25]);
}

#[test]
fn a_turned_wall_keeps_its_face_and_its_dimensions() {
    let (mut doc, w) = house(None);
    let out = done(
        &mut doc,
        CREATE,
        json!({"layerId": "cizim", "wall": w[0], "face": "right", "at": p(5., -1.5)}),
    );
    let id = DimensionSetId::parse(out["set"].as_str().unwrap()).unwrap();
    let before = members(&doc, id);
    let lengths = row(&doc, id, DimensionSetRow::Openings);
    done(&mut doc, "cad.walls.edit", json!({"uids": [w[0]], "operation": {"kind": "flip"}}));
    let set = doc.dimension_set(id).unwrap();
    assert_eq!(set.face, kentos_commands::DimensionSetFace::Left, "the same physical face");
    assert_eq!(row(&doc, id, DimensionSetRow::Openings), lengths);
    assert_eq!(members(&doc, id).len(), before.len());
    assert!(members(&doc, id).iter().all(|u| before.contains(u)), "the same dimensions");
}

#[test]
fn a_deleted_source_leaves_and_the_last_one_breaks_the_set() {
    let (mut doc, w) = house(None);
    let id = facade(&mut doc, &[&w[0], &w[2]], &["overall"]);
    done(&mut doc, "cad.entities.delete", json!({"uids": [w[2]]}));
    let set = doc.dimension_set(id).unwrap();
    assert_eq!(set.sources().len(), 1);
    assert!(set.broken.is_none());
    done(&mut doc, "cad.entities.delete", json!({"uids": [w[0]]}));
    let set = doc.dimension_set(id).unwrap();
    assert_eq!(set.broken, Some(DimensionSetBreak::HostGone));
    assert!(!set.members.is_empty(), "the last dimensions stay, said");
}

#[test]
fn walls_on_two_levels_are_refused_and_a_moved_one_breaks_the_set() {
    let (mut doc, w) = house(Some("zemin"));
    done(
        &mut doc,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{"pts": [p(20., 0.), p(30., 0.)], "width": 0.25,
            "justify": "center", "arch": {"level": "kat-1"}}]}),
    );
    let upper = doc
        .entities_with_uids()
        .find_map(|(uid, e)| match e {
            Entity::Wall(x) if x.wall.arch.level.as_deref() == Some("kat-1") => Some(uid.to_string()),
            _ => None,
        })
        .unwrap();
    let refused = run(
        &mut doc,
        CREATE,
        CallOp::Plan,
        json!({"layerId": "cizim", "walls": [w[0], upper], "direction": p(1., 0.), "at": p(5., -1.5)}),
    );
    assert_eq!(refused["status"], "failed");
    assert_eq!(refused["error"]["code"], "invalid_dimension_set");
    let id = facade(&mut doc, &[&w[0], &w[2]], &["overall"]);
    done(&mut doc, "cad.levels.assign", json!({"uids": [w[2]], "level": "kat-1"}));
    assert_eq!(doc.dimension_set(id).unwrap().broken, Some(DimensionSetBreak::MixedSources));
}

#[test]
fn four_sides_in_one_step() {
    let (mut doc, w) = house(None);
    let out = done(
        &mut doc,
        CREATE,
        json!({"layerId": "cizim", "walls": [w[0], w[1], w[2], w[3]], "direction": p(1., 0.),
            "rows": ["openings", "overall"], "around": true, "at": p(5., -1.5)}),
    );
    assert_eq!(out["sets"].as_array().unwrap().len(), 4);
    assert_eq!(doc.dimension_sets().len(), 4);
    let undone = doc.undo();
    assert!(undone.is_some());
    assert!(doc.dimension_sets().is_empty(), "one undo step");
}

#[test]
fn a_space_measures_its_outline_and_its_door() {
    let (mut doc, w) = house(None);
    done(
        &mut doc,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [{"host": w[0], "at": 1.5, "width": 1.0}]}),
    );
    let made = done(
        &mut doc,
        "cad.spaces.create",
        json!({"layerId": "cizim", "spaces": [{"seed": p(2., 4.), "number": "01", "name": "Oda"}]}),
    );
    let space = made["uids"][0].as_str().unwrap().to_owned();
    let out = done(
        &mut doc,
        CREATE,
        json!({"layerId": "cizim", "space": space, "direction": p(1., 0.), "rows": ["openings", "overall"],
            "at": p(2., 0.8)}),
    );
    let id = DimensionSetId::parse(out["set"].as_str().unwrap()).unwrap();
    // The room from the west wall's inner face (0.125) to the partition's (3.95): the door from 1 to 2.
    assert_eq!(row(&doc, id, DimensionSetRow::Openings), [0.875, 1.0, 1.95]);
    assert_eq!(row(&doc, id, DimensionSetRow::Overall), [3.825]);
}

/// The KCAD example of schema 128 (fixtures/kcad/v2/dimension-set-spans.json): a south façade with its window, its
/// corner walls and a grid, as the command writes it. `KENTOS_WRITE_SPAN_MODEL=1` writes it again.
#[test]
fn the_kcad_example_is_the_commands_drawing() {
    let (mut doc, w) = house(None);
    facade(&mut doc, &[&w[0]], &["openings", "grids", "overall"]);
    let snapshot = doc.to_snapshot_v2();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/dimension-set-spans.json");
    if std::env::var_os("KENTOS_WRITE_SPAN_MODEL").is_some() {
        std::fs::write(path, serde_json::to_string_pretty(&snapshot).unwrap() + "\n").unwrap();
    }
    let stored: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let reopened = Document::from_snapshot_v2(stored.clone()).unwrap();
    assert_eq!(reopened.to_snapshot_v2().dimension_sets, stored.dimension_sets, "opened unchanged");
    assert!(stored.dimension_sets[0].span.is_some());
}

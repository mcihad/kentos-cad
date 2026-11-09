//! Yüzey yapıştır (docs/adr/3920): a pad surface pasted into the road project's sample ground
//! (`fixtures/kcad/v2/road-work.json`): its triangles come in as they are, the ground's records under it go, the rest is
//! triangulated round it; a paste that comes back to the surface, names no surface or one that cannot be made is said.

use kentos_contracts::{DocumentSnapshotV2, TerrainSurfaceDef};
use kentos_road::sources::{Gathered, Left, SourceObject, SurfaceState, gather_among};
use kentos_road::surface::{Surface, build};
use serde_json::{Value, json};

const SAMPLE: &str = include_str!("../../../../../fixtures/kcad/v2/road-work.json");
const BREAKLINE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e16";

/// The pad: its toe and its top as closed breaklines on `dolgu`.
const TOE: [[f64; 3]; 4] = [[10.0, 6.0, 101.0], [30.0, 7.0, 101.5], [29.0, 24.0, 102.5], [11.0, 23.0, 102.0]];
const TOP: [[f64; 3]; 4] = [[15.0, 11.0, 103.0], [25.0, 11.5, 103.0], [24.0, 19.0, 103.0], [16.0, 18.5, 103.0]];

fn ring(id: u32, uid_tail: &str, corners: &[[f64; 3]; 4]) -> (Value, String) {
    let pts: Vec<Value> = corners.iter().map(|c| json!({"x": c[0], "y": c[1]})).collect();
    let zs: Vec<Value> = corners.iter().map(|c| json!(c[2])).collect();
    (
        json!({"kind": "polygon", "id": id, "layerId": "dolgu", "attrs": {}, "pts": pts, "zs": zs}),
        format!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e{uid_tail}"),
    )
}

/// The sample with the pad drawn and three surfaces: the ground, the pad, and the ground with the pad pasted.
fn drawing(son_edits: Value, pad_edits: Value) -> Value {
    let mut doc: Value = serde_json::from_str(SAMPLE).expect("JSON");
    let mut layer = doc["layers"][1].clone();
    layer["id"] = json!("dolgu");
    layer["name"] = json!("Dolgu");
    doc["layers"].as_array_mut().expect("layers").push(layer);
    for (entity, uid) in [ring(8, "21", &TOE), ring(9, "22", &TOP)] {
        doc["entities"].as_array_mut().expect("entities").push(entity);
        doc["uids"].as_array_mut().expect("uids").push(json!(uid));
    }
    let surfaces = &mut doc["settings"]["road"]["surfaces"];
    let pad = json!({"id": "t-dolgu", "name": "Dolgu", "breaklines": {"layers": ["dolgu"]}, "edits": pad_edits});
    let son = json!({"id": "t-son", "name": "Son zemin", "points": {"layers": ["zemin"]},
                     "breaklines": {"uids": [BREAKLINE]}, "edits": son_edits});
    surfaces.as_array_mut().expect("surfaces").extend([pad, son]);
    doc
}

/// The surface `id` gathered among the drawing's surfaces.
fn gathered(doc: &Value, id: &str) -> (TerrainSurfaceDef, Gathered) {
    let snap: DocumentSnapshotV2 = serde_json::from_value(doc.clone()).expect("snapshot");
    let all = snap.settings.road.as_ref().expect("road").surfaces.clone();
    let def = all.iter().find(|s| s.id == id).expect("surface").clone();
    let uids: Vec<String> = snap.uids.iter().map(ToString::to_string).collect();
    let got = gather_among(
        &def,
        &all,
        &snap.layers,
        snap.entities.iter().zip(&uids).map(|(entity, uid)| SourceObject { uid, entity }),
    );
    (def, got)
}

/// A surface's kept triangles as their corners' places, each sorted, the list sorted.
fn triangles(s: &Surface) -> Vec<[[u64; 2]; 3]> {
    let place = |i: u32| {
        let v = s.vertices[i as usize];
        [v[0].to_bits(), v[1].to_bits()]
    };
    let mut out: Vec<[[u64; 2]; 3]> = s
        .triangles
        .iter()
        .map(|t| {
            let mut c = t.map(place);
            c.sort_unstable();
            c
        })
        .collect();
    out.sort_unstable();
    out
}

fn left(id: &str, why: &'static str) -> Left {
    Left { id: id.into(), role: "edits", why }
}

#[test]
fn the_pasted_pads_triangles_come_in_as_they_are_and_the_ground_under_it_goes() {
    let doc = drawing(json!([{"op": "paste", "terrain": "t-dolgu"}]), json!([]));
    let (_, pad) = gathered(&doc, "t-dolgu");
    let pad = build(&pad.input).expect("the pad is made");
    // Eight corners, the toe its hull: 2·8 − 4 − 2 triangles, 3·8 − 4 − 3 edges.
    assert_eq!((pad.vertices.len(), pad.triangles.len()), (8, 10));

    let (_, got) = gathered(&doc, "t-son");
    assert_eq!(got.left, vec![]);
    // The ground's four corners stay (outside the toe); the pad's eight corners come in, by place.
    assert_eq!(got.input.points.len(), 12);
    let pasted: Vec<[f64; 3]> =
        got.input.points.iter().filter(|p| p.source.as_deref() == Some("edits[0]")).map(|p| [p.x, p.y, p.z]).collect();
    let mut corners: Vec<[f64; 3]> = TOE.iter().chain(TOP.iter()).copied().collect();
    corners.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    assert_eq!(pasted, corners);
    // The ground's breakline crosses the pad: it goes whole (one piece). The pad's 17 edges come in, one breakline each.
    assert!(got.input.breaklines.iter().all(|b| b.source.as_deref() == Some("edits[0]") && b.points.len() == 2));
    assert_eq!(got.input.breaklines.len(), 17);

    let son = build(&got.input).expect("the pasted ground is made");
    let all = triangles(&son);
    for t in triangles(&pad) {
        assert!(all.binary_search(&t).is_ok(), "the pad's triangle {t:?} is the ground's");
    }
    // On the pad's top its height (read off a triangle: to the last bit or so); at the ground's corner its own.
    let top = son.height(20.0, 15.0).z.expect("on the pad");
    assert!((top - 103.0).abs() < 1e-9, "{top}");
    assert_eq!(son.height(0.0, 0.0).z, Some(100.0));
    // The sample's boundary is the rectangle: the whole of it is kept.
    assert_eq!(son.figures().area, 1200.0);
}

#[test]
fn a_breakline_beside_the_pad_stays_and_one_touching_it_goes() {
    // The ground's breakline moved below the toe (y = 3): neither end covered, it meets no toe edge.
    let mut doc = drawing(json!([{"op": "paste", "terrain": "t-dolgu"}]), json!([]));
    let bl = doc["entities"].as_array_mut().expect("entities").iter_mut().find(|e| e["id"] == 6).expect("breakline");
    bl["pts"] = json!([{"x": 5.0, "y": 3.0}, {"x": 35.0, "y": 3.0}]);
    let (_, got) = gathered(&doc, "t-son");
    assert_eq!(got.input.breaklines.iter().filter(|b| b.source.as_deref() == Some(BREAKLINE)).count(), 1);
    // Through the toe's corner (10, 6): touching counts, it goes.
    let bl = doc["entities"].as_array_mut().expect("entities").iter_mut().find(|e| e["id"] == 6).expect("breakline");
    bl["pts"] = json!([{"x": 5.0, "y": 3.0}, {"x": 15.0, "y": 9.0}]);
    let (_, got) = gathered(&doc, "t-son");
    assert_eq!(got.input.breaklines.iter().filter(|b| b.source.as_deref() == Some(BREAKLINE)).count(), 0);
}

#[test]
fn a_paste_that_comes_back_names_no_surface_or_one_not_made_is_said_and_pastes_nothing() {
    // The pad pastes the pasted ground back: each says the cycle and keeps its own sources.
    let doc = drawing(json!([{"op": "paste", "terrain": "t-dolgu"}]), json!([{"op": "paste", "terrain": "t-son"}]));
    let (_, son) = gathered(&doc, "t-son");
    assert_eq!(son.left, vec![left("edits[0]", "cycle")]);
    assert_eq!(son.input.points.len(), 4);
    assert_eq!(son.input.breaklines.len(), 1);
    let (_, pad) = gathered(&doc, "t-dolgu");
    assert_eq!(pad.left, vec![left("edits[0]", "cycle")]);
    // No such surface.
    let doc = drawing(json!([{"op": "paste", "terrain": "t-yok"}]), json!([]));
    assert_eq!(gathered(&doc, "t-son").1.left, vec![left("edits[0]", "no_surface")]);
    // A pad of no heights is not made.
    let mut doc = drawing(json!([{"op": "paste", "terrain": "t-dolgu"}]), json!([]));
    doc["settings"]["road"]["surfaces"][1]["breaklines"] = json!({"layers": ["0"]});
    assert_eq!(gathered(&doc, "t-son").1.left, vec![left("edits[0]", "unbuildable")]);
}

#[test]
fn the_pasted_ground_goes_out_of_date_when_the_pad_changes() {
    let doc = drawing(json!([{"op": "paste", "terrain": "t-dolgu"}]), json!([]));
    let (mut def, first) = gathered(&doc, "t-son");
    def.built = Some(first.digest.clone());
    let mut moved = doc.clone();
    moved["entities"][8]["zs"][0] = json!(103.5);
    let (_, then) = gathered(&moved, "t-son");
    assert_eq!(first.state(&def), SurfaceState::Current);
    assert_eq!(then.state(&def), SurfaceState::Stale);
    // Edits after the paste act on the pad's corners: its top corner's height set.
    let doc = drawing(
        json!([{"op": "paste", "terrain": "t-dolgu"}, {"op": "setHeight", "x": 15.0, "y": 11.0, "z": 103.25}]),
        json!([]),
    );
    let (_, got) = gathered(&doc, "t-son");
    assert_eq!(got.left, vec![]);
    assert!(got.input.points.iter().any(|p| (p.x, p.y, p.z) == (15.0, 11.0, 103.25)));
    assert!(got.input.breaklines.iter().flat_map(|b| &b.points).filter(|p| (p[0], p[1]) == (15.0, 11.0)).all(|p| p[2] == 103.25));
}

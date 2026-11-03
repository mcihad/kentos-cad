//! A terrain surface's input gathered from a drawing (plan/road.md Y2.2): the road project's sample
//! (`fixtures/kcad/v2/road-work.json`), and the same drawing changed as its user would change it.

use kentos_contracts::{DocumentSnapshotV2, TerrainSurfaceDef};
use kentos_road::sources::{gather, Gathered, Left, SourceObject, SurfaceState};
use kentos_road::surface::build;
use serde_json::{json, Value};

const SAMPLE: &str = include_str!("../../../../../fixtures/kcad/v2/road-work.json");

fn sample() -> Value {
    serde_json::from_str(SAMPLE).expect("JSON")
}

fn gathered(doc: &Value) -> (TerrainSurfaceDef, Gathered) {
    let snap: DocumentSnapshotV2 = serde_json::from_value(doc.clone()).expect("snapshot");
    let def = snap.settings.road.as_ref().expect("road").surfaces[0].clone();
    let uids: Vec<String> = snap.uids.iter().map(ToString::to_string).collect();
    let got = gather(
        &def,
        &snap.layers,
        snap.entities.iter().zip(&uids).map(|(entity, uid)| SourceObject { uid, entity }),
    );
    (def, got)
}

fn left(id: &str, role: &'static str, why: &'static str) -> Left {
    Left {
        id: id.into(),
        role,
        why,
    }
}

const U: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e1";

#[test]
fn the_samples_surface_is_its_points_breakline_and_boundary() {
    let (def, got) = gathered(&sample());
    assert_eq!(got.left, vec![]);
    assert_eq!(got.input.points.len(), 4);
    assert_eq!(got.input.points[2].z, 103.25);
    assert_eq!(got.input.points[2].source.as_deref(), Some(&*format!("{U}4")));
    assert_eq!(got.input.breaklines.len(), 1);
    assert_eq!(got.input.breaklines[0].points, vec![[5.0, 15.0, 101.0], [35.0, 15.0, 102.5]]);
    assert_eq!(got.input.boundary.as_ref().map(|b| b.points.len()), Some(4));
    assert_eq!(got.input.tolerance, Some(0.005));
    let s = build(&got.input).expect("surface");
    // Four corners and the breakline's two ends: the breakline splits the rectangle into six triangles.
    assert_eq!(s.vertices.len(), 6);
    assert_eq!(s.triangles.len(), 6);
    assert_eq!(s.height(20.0, 15.0).z, Some(101.75));
    assert_eq!(got.state(&def), SurfaceState::New);
    assert_eq!(got.digest.len(), 64);
}

#[test]
fn a_surface_goes_out_of_date_when_its_sources_change() {
    let mut doc = sample();
    let (_, first) = gathered(&doc);
    doc["settings"]["road"]["surfaces"][0]["built"] = json!(first.digest);
    let (def, again) = gathered(&doc);
    assert_eq!(again.digest, first.digest, "the same drawing, the same digest");
    assert_eq!(again.state(&def), SurfaceState::Current);
    // Another object of another layer changes nothing.
    doc["entities"][0]["b"]["x"] = json!(20.0);
    let (def, other) = gathered(&doc);
    assert_eq!(other.state(&def), SurfaceState::Current);
    // A point's elevation does.
    doc["entities"][3]["z"] = json!(103.5);
    let (def, moved) = gathered(&doc);
    assert_eq!(moved.state(&def), SurfaceState::Stale);
    // So does the definition's method.
    let mut doc = sample();
    doc["settings"]["road"]["surfaces"][0]["built"] = json!(first.digest);
    doc["settings"]["road"]["surfaces"][0]["maxEdge"] = json!(45.0);
    let (def, method) = gathered(&doc);
    assert_eq!(method.state(&def), SurfaceState::Stale);
}

#[test]
fn what_a_surface_cannot_use_is_said_by_its_id_and_why() {
    let mut doc = sample();
    // A point without an elevation, the breakline's second vertex without one, the boundary gone, a void that is an
    // open polyline, a named object of no kind a list takes, a layer that is not there.
    doc["entities"][1].as_object_mut().unwrap().remove("z");
    doc["entities"][5]["zs"] = json!([101.0, null]);
    let entities = doc["entities"].as_array_mut().unwrap();
    entities.pop();
    entities.push(json!({"kind": "polyline", "id": 7, "layerId": "0", "attrs": {},
        "pts": [{"x": 10.0, "y": 5.0}, {"x": 12.0, "y": 5.0}, {"x": 12.0, "y": 7.0}]}));
    let s = &mut doc["settings"]["road"]["surfaces"][0];
    s["voids"] = json!([format!("{U}7")]);
    s["boundary"] = json!(format!("{U}9"));
    s["points"]["uids"] = json!([format!("{U}1")]);
    s["points"]["layers"] = json!(["zemin", "yok"]);
    let (_, got) = gathered(&doc);
    assert_eq!(
        got.left,
        vec![
            left("yok", "points", "missing"),
            left(&format!("{U}1"), "points", "wrong_kind"),
            left(&format!("{U}2"), "points", "no_height"),
            left(&format!("{U}6"), "breaklines", "no_height"),
            left(&format!("{U}9"), "boundary", "missing"),
            left(&format!("{U}7"), "voids", "not_closed"),
        ]
    );
    assert_eq!(got.input.points.len(), 3);
    assert!(got.input.breaklines.is_empty());
    assert!(got.input.boundary.is_none());
}

#[test]
fn a_groups_layers_are_its_sources_and_arcs_are_left() {
    let mut doc = sample();
    let zemin = doc["layers"].as_array_mut().unwrap().pop().unwrap();
    doc["layers"].as_array_mut().unwrap().push(json!({"id": "g", "name": "Arazi", "type": "group", "visible": true,
        "locked": false, "expanded": true, "style": zemin["style"].clone(), "children": [zemin]}));
    doc["settings"]["road"]["surfaces"][0]["points"]["layers"] = json!(["g"]);
    doc["entities"][5]["bulges"] = json!([0.25, 0.0]);
    let (_, got) = gathered(&doc);
    assert_eq!(got.input.points.len(), 4);
    assert_eq!(got.left, vec![left(&format!("{U}6"), "breaklines", "arc_edge")]);
}

/// The sample's surface in three states as JSON for its picture (`KENTOS_ROAD_OUT=dir`), by hand.
#[test]
#[ignore = "pictures, run by hand"]
fn pictures() {
    let dir = std::env::var("KENTOS_ROAD_OUT").expect("KENTOS_ROAD_OUT");
    let mut doc = sample();
    let (_, first) = gathered(&doc);
    doc["settings"]["road"]["surfaces"][0]["built"] = json!(first.digest);
    let mut moved = doc.clone();
    moved["entities"][3]["z"] = json!(104.75);
    let mut broken = doc.clone();
    broken["entities"][1].as_object_mut().unwrap().remove("z");
    broken["entities"].as_array_mut().unwrap().pop();
    let mut out = Vec::new();
    for (title, d) in [("Kaydedilen yüzey", &doc), ("Z3'ün kotu değişti", &moved), ("Kaynak eksik", &broken)] {
        let (def, got) = gathered(d);
        let s = build(&got.input).expect("surface");
        out.push(json!({
            "title": title, "state": got.state(&def), "digest": got.digest, "built": def.built,
            "left": got.left, "vertices": s.vertices, "triangles": s.triangles, "constrained": s.constrained,
            "points": got.input.points, "definition": def,
        }));
    }
    std::fs::write(format!("{dir}/sources-out.json"), serde_json::to_string(&out).expect("json")).expect("written");
}

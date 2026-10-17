//! The constraints' solves as jobs (docs/adr/0337 §4): a stopped solve writes nothing, a job's staged solve is written
//! only while what it moved is still as it saw it.

use kentos_contracts::{CallEnvelope, CallOp, CommandResult, JobPhase};
use kentos_domain::Document;
use serde_json::{Value, json};

use crate::call::{call, call_in_job};
use crate::jobs::{Job, Progress};

fn line(id: u32, a: (f64, f64), b: (f64, f64)) -> Value {
    json!({"kind": "line", "id": id, "layerId": "cizim", "attrs": {}, "a": {"x": a.0, "y": a.1}, "b": {"x": b.0, "y": b.1}})
}

fn drawing() -> Document {
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Kısıtlar",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 100, "workspace": "cad"},
        "origin": {"x": 0, "y": 0},
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                    "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "cizim",
        "entities": [line(1, (0.0, 0.0), (8.0, 0.0)), line(2, (8.0, 0.0), (8.0, 4.0)),
                     line(3, (8.0, 4.0), (0.0, 4.0)), line(4, (0.0, 4.0), (0.0, 0.0))],
        "styles": {"items": [], "categories": []},
    });
    Document::from_snapshot(
        kentos_contracts::DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"),
    )
    .expect("opens")
}

fn envelope(command: &str, input: Value) -> CallEnvelope {
    CallEnvelope {
        command: command.into(),
        version: None,
        op: CallOp::Execute,
        input,
        request_id: None,
        expected: None,
        relax_constraints: false,
    }
}

fn uid(doc: &Document, slot: u32) -> String {
    doc.uid(kentos_domain::Slot(slot))
        .expect("an object")
        .to_string()
}

/// The rectangle's corners joined, bottom horizontal, opposite edges parallel, a square corner, width d1 and
/// height d2, no fix.
fn constrained() -> Document {
    let mut doc = drawing();
    let u: Vec<String> = (1..=4).map(|s| uid(&doc, s)).collect();
    let point = |k: usize, place: &str| json!({"uid": u[k], "element": {"kind": "point", "place": {"kind": place}}});
    let seg = |k: usize| json!({"uid": u[k], "element": {"kind": "segment", "part": 0, "ring": 0, "index": 0}});
    let mut rules: Vec<Value> = (0..4)
        .map(|k| json!({"rule": {"kind": "coincident", "a": point(k, "end"), "b": point((k + 1) % 4, "start")}}))
        .collect();
    rules.push(json!({"rule": {"kind": "horizontal", "a": seg(0)}}));
    rules.push(json!({"rule": {"kind": "parallel", "a": seg(0), "b": seg(2)}}));
    rules.push(json!({"rule": {"kind": "parallel", "a": seg(1), "b": seg(3)}}));
    rules.push(json!({"rule": {"kind": "perpendicular", "a": seg(0), "b": seg(1)}}));
    rules.push(json!({"rule": {"kind": "distance", "a": point(0, "start"), "b": point(0, "end"), "parameter": ""}, "value": "8"}));
    rules.push(json!({"rule": {"kind": "distance", "a": point(1, "start"), "b": point(1, "end"), "parameter": ""}, "value": "4"}));
    let answer = call(
        &mut doc,
        None,
        envelope(
            kentos_contracts::CAD_CONSTRAINTS_EDIT,
            json!({"edit": {"kind": "add", "constraints": rules}}),
        ),
    );
    assert!(
        matches!(answer.result, CommandResult::Completed { .. }),
        "{:?}",
        answer.result
    );
    doc
}

fn width_to(value: &str) -> CallEnvelope {
    envelope(
        kentos_contracts::CAD_PARAMETERS_EDIT,
        json!({"edits": [{"kind": "set", "name": "d1", "expression": value}]}),
    )
}

#[test]
fn a_stopped_solve_writes_nothing() {
    let mut doc = constrained();
    let at = doc.revision();
    let stopped = Progress::default();
    stopped.stop();
    // The parameters' solve.
    let answer = call_in_job(&mut doc, width_to("10"), &stopped);
    assert!(
        matches!(answer.result, CommandResult::Cancelled),
        "{:?}",
        answer.result
    );
    assert_eq!(doc.revision(), at, "nothing written");
    // An edit's follow: the top edge moved, the rest would follow.
    let top = uid(&doc, 3);
    let moved = envelope(
        kentos_contracts::CAD_ENTITIES_TRANSFORM,
        json!({"uids": [top], "transform": {"kind": "move", "dx": 0, "dy": 1}}),
    );
    let answer = call_in_job(&mut doc, moved.clone(), &stopped);
    assert!(
        matches!(answer.result, CommandResult::Cancelled),
        "{:?}",
        answer.result
    );
    assert_eq!(doc.revision(), at, "nothing written");
    // Not stopped, both write.
    let answer = call_in_job(&mut doc, moved, &Progress::default());
    assert!(
        matches!(answer.result, CommandResult::Completed { .. }),
        "{:?}",
        answer.result
    );
    let answer = call_in_job(&mut doc, width_to("10"), &Progress::default());
    assert!(
        matches!(answer.result, CommandResult::Completed { .. }),
        "{:?}",
        answer.result
    );
}

#[test]
fn a_staged_solve_waits_for_the_drawing_it_saw() {
    // Written as it is: nothing else changed.
    let mut doc = constrained();
    let mut job = Job::start("p1".into(), &doc, width_to("10"));
    let status = job.wait(&mut doc);
    assert_eq!(status.phase, JobPhase::Done, "{:?}", status.answer);
    let bottom = doc
        .get(kentos_domain::Slot(1))
        .expect("the bottom edge")
        .clone();
    let kentos_contracts::Entity::Line(l) = bottom else {
        panic!("a line")
    };
    assert!(((l.b.x - l.a.x).hypot(l.b.y - l.a.y) - 10.0).abs() < 1e-9);
    // An edge the solve moves changed meanwhile: nothing written.
    let mut job = Job::start("p2".into(), &doc, width_to("12"));
    let right = uid(&doc, 2);
    let answer = call(
        &mut doc,
        None,
        envelope(
            kentos_contracts::CAD_ENTITIES_TRANSFORM,
            json!({"uids": [right], "transform": {"kind": "move", "dx": 1, "dy": 0}}),
        ),
    );
    assert!(
        matches!(answer.result, CommandResult::Completed { .. }),
        "{:?}",
        answer.result
    );
    let at = doc.revision();
    let status = job.wait(&mut doc);
    assert_eq!(status.phase, JobPhase::Conflict, "{:?}", status.answer);
    assert_eq!(doc.revision(), at, "nothing written");
    // Cancelled while staged: dropped.
    let mut job = Job::start("p3".into(), &doc, width_to("12"));
    job.cancel();
    assert_eq!(job.wait(&mut doc).phase, JobPhase::Cancelled);
    assert_eq!(doc.revision(), at);
}

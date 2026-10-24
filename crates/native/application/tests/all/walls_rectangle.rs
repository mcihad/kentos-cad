//! ARCH-DRAW-01 beyond the shared cases: physical inner faces, protocol history, atomic refusal
//! and the original wall's interoperability. The expected dimensions are elementary rectangle
//! arithmetic; they are not regenerated from the command's output.

use kentos_commands::{
    CallEnvelope, CallOp, CommandResult, DocumentSnapshotV1, DrawingUnit, Entity, WallEntity,
    WallsRectangle,
};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, call::call, walls_rectangle};
use serde_json::{Value, json};

fn drawing(unit: &str) -> Document {
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.walls.rectangle.json"
    ))
    .unwrap();
    fixture["setup"]["settings"]["drawingUnit"] = json!(unit);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap())
        .unwrap()
}

fn input(x: f64, y: f64, dx: f64, dy: f64, reference: &str) -> WallsRectangle {
    serde_json::from_value(json!({
        "layerId":"duvar", "frame":{"kind":"corners", "a":{"x":x,"y":y},
            "b":{"x":x+dx,"y":y+dy}}, "reference":reference, "width":0.2
    }))
    .unwrap()
}

fn planned(doc: &Document, input: &WallsRectangle) -> WallEntity {
    match walls_rectangle::plan_in(doc, input) {
        CommandResult::Completed { output, .. } => match output.walls.into_iter().next().unwrap() {
            Entity::Wall(w) => w,
            other => panic!("not a wall: {other:?}"),
        },
        other => panic!("not planned: {other:?}"),
    }
}

fn area(pts: &[kentos_geometry_core::Vec2]) -> f64 {
    let first = pts[0];
    let mut sum = 0.0;
    for (a, b) in pts.iter().zip(pts.iter().cycle().skip(1)).take(pts.len()) {
        sum += (a.x - first.x) * (b.y - first.y) - (a.y - first.y) * (b.x - first.x);
    }
    sum.abs() / 2.0
}

fn inner_area(w: &WallEntity) -> f64 {
    let points = w
        .wall
        .pts
        .iter()
        .map(|p| kentos_geometry_core::Vec2::new(p.x, p.y))
        .collect::<Vec<_>>();
    let (_, holes) = kentos_geometry_core::geom::wall::body(
        &points,
        None,
        &kentos_formats::walls::spec(&w.wall),
    )
    .unwrap();
    area(&holes.unwrap()[0])
}

#[test]
fn clear_dimensions_are_physical_in_all_quadrants_units_and_large_coordinates() {
    for unit in ["mm", "cm", "m", "in", "ft"] {
        let doc = drawing(unit);
        for (x, y) in [(0.0, 0.0), (500_000.0, 4_400_000.0)] {
            for (dx, dy) in [(4.0, 3.0), (-4.0, 3.0), (-4.0, -3.0), (4.0, -3.0)] {
                for (reference, expected) in
                    [("inner", 12.0), ("outer", 3.6 * 2.6), ("center", 3.8 * 2.8)]
                {
                    let before = doc.to_snapshot_v2();
                    let w = planned(&doc, &input(x, y, dx, dy, reference));
                    assert!(w.wall.closed);
                    assert_eq!(w.wall.pts.len(), 4);
                    assert_eq!((w.wall.pts[0].x, w.wall.pts[0].y), (x, y));
                    assert!(
                        (inner_area(&w) - expected).abs() < 1e-8,
                        "{unit}, {x},{y}, {dx},{dy}, {reference}: {}",
                        inner_area(&w)
                    );
                    assert_eq!(doc.to_snapshot_v2(), before, "preview mutated document");
                }
            }
        }
    }
}

#[test]
fn the_new_command_records_one_undo_step_without_a_nested_wall_command() {
    let mut doc = drawing("m");
    let before = doc.to_snapshot_v2();
    let raw = serde_json::to_value(input(0.0, 0.0, 4.0, 3.0, "inner")).unwrap();
    let answer = call(
        &mut doc,
        None,
        CallEnvelope {
            command: "cad.walls.rectangle".into(),
            version: Some(1),
            op: CallOp::Execute,
            input: raw.clone(),
            request_id: Some("rectangular-wall-once".into()),
            expected: None,
            relax_constraints: false,
        },
    );
    assert!(
        matches!(answer.result, CommandResult::Completed { .. }),
        "{answer:?}"
    );
    let after = doc.to_snapshot_v2();
    let rows: Vec<_> = doc.journal().collect();
    assert_eq!(
        rows.len(),
        1,
        "nested walls.create must not be a second journal call"
    );
    assert_eq!(rows[0].command.as_deref(), Some("cad.walls.rectangle"));
    assert_eq!(rows[0].input.as_ref(), Some(&raw));
    assert_eq!(doc.undo_labels(), ["Duvar"]);
    let duplicate = call(
        &mut doc,
        None,
        CallEnvelope {
            command: "cad.walls.rectangle".into(),
            version: Some(1),
            op: CallOp::Execute,
            input: raw,
            request_id: Some("rectangular-wall-once".into()),
            expected: None,
            relax_constraints: false,
        },
    );
    assert!(matches!(duplicate.result, CommandResult::Completed { .. }));
    assert_eq!(
        doc.to_snapshot_v2(),
        after,
        "idempotent request wrote a second wall"
    );
    assert_eq!(doc.journal().count(), 1);
    assert_eq!(doc.undo().as_deref(), Some("Duvar"));
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    assert_eq!(doc.redo().as_deref(), Some("Duvar"));
    assert_eq!(doc.to_snapshot_v2().entities, after.entities);
}

#[test]
fn a_collapsed_inner_rectangle_is_refused_without_mutating_the_original_line() {
    for reference in ["outer", "center"] {
        let mut doc = drawing("m");
        let before = doc.to_snapshot_v2();
        let mut input = input(0.0, 0.0, 4.0, 3.0, reference);
        input.width = Some(4.0);
        match walls_rectangle::validate(&ExecutionContext::new(&mut doc), &input) {
            CommandResult::Failed { error } => {
                assert_eq!(error.code, "invalid_wall");
                assert_eq!(error.path.as_deref(), Some("frame"));
            }
            other => panic!("accepted no interior: {other:?}"),
        }
        let result = walls_rectangle::execute(&mut ExecutionContext::new(&mut doc), input);
        assert!(matches!(result, CommandResult::Failed { .. }));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn exact_inset_boundary_at_a_rotated_large_origin_reports_no_interior() {
    let mut doc = drawing("m");
    let before = doc.to_snapshot_v2();
    let input: WallsRectangle = serde_json::from_value(json!({"layerId":"duvar",
        "frame":{"kind":"size","corner":{"x":500000.0,"y":4400000.0},
            "length":4.0,"width":3.0,"toward":{"x":500010.0,"y":4400010.0}},
        "rotation":std::f64::consts::FRAC_PI_4,"reference":"outer","width":1.5}))
    .unwrap();
    match walls_rectangle::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Failed { error } => {
            assert_eq!(error.code, "invalid_wall");
            assert_eq!(error.path.as_deref(), Some("frame"));
        }
        other => panic!("the exact short side closes: {other:?}"),
    }
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn a_source_wall_still_accepts_hosted_openings_and_survives_snapshot_roundtrip() {
    let mut doc = drawing("m");
    let made = walls_rectangle::execute(
        &mut ExecutionContext::new(&mut doc),
        input(0.0, 0.0, 4.0, 3.0, "inner"),
    );
    let uid = match made {
        CommandResult::Completed { output, .. } => output.uids[0].clone(),
        other => panic!("not written: {other:?}"),
    };
    let opening = serde_json::from_value(json!({"layerId":"duvar","openings":[{
        "host":uid,"at":2.0,"width":0.9,"form":"door"
    }]}))
    .unwrap();
    let result = kentos_native_application::openings_create::execute(
        &mut ExecutionContext::new(&mut doc),
        opening,
    );
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    let back = Document::from_snapshot_v2(doc.to_snapshot_v2()).unwrap();
    assert_eq!(back.to_snapshot_v2(), doc.to_snapshot_v2());
    assert_eq!(back.settings().drawing_unit, Some(DrawingUnit::M));
    let wall = back
        .entities()
        .find_map(|e| match e {
            Entity::Wall(w) => Some(w),
            _ => None,
        })
        .unwrap();
    assert!((inner_area(wall) - 12.0).abs() < 1e-9);
}

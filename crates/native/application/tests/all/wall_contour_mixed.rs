//! Mixed, measured independent sources; no native noding in the reference.
use super::wall_contours::{completed, drawing};
use kentos_commands::*;
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, walls_from_lines};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn transformed(mut e: Value, x: f64, y: f64, reverse: bool) -> Value {
    if let Some(c) = e.get_mut("c") {
        c["x"] = json!(c["x"].as_f64().unwrap() + x);
        c["y"] = json!(c["y"].as_f64().unwrap() + y);
    }
    if let Some(pts) = e.get_mut("pts") {
        let pts = pts.as_array_mut().unwrap();
        for p in pts.iter_mut() {
            p["x"] = json!(p["x"].as_f64().unwrap() + x);
            p["y"] = json!(p["y"].as_f64().unwrap() + y);
        }
        if reverse {
            pts[1..].reverse();
        }
    }
    if reverse && let Some(bs) = e.get_mut("bulges") {
        let before = bs.as_array().unwrap().clone();
        let n = before.len();
        *bs = json!(
            (0..n)
                .map(|i| -before[(n - 1 - i) % n].as_f64().unwrap())
                .collect::<Vec<_>>()
        );
    }
    for key in ["holes", "parts"] {
        if let Some(items) = e.get_mut(key) {
            for item in items.as_array_mut().unwrap() {
                *item = transformed(item.take(), x, y, reverse);
            }
        }
    }
    e
}
fn length(w: &WallFields) -> f64 {
    let p = w
        .pts
        .iter()
        .map(|p| kentos_geometry_core::Vec2::new(p.x, p.y))
        .collect::<Vec<_>>();
    kentos_geometry_core::geom::bulge::bulge_path_edges(&p, w.bulges.as_deref(), w.closed)
        .iter()
        .map(kentos_geometry_core::geom::arrangement::edge_len)
        .sum()
}
#[test]
fn courtyard_infill_multipart_l_room_partial_edge_circle_and_arc_preserve_measured_network_and_sources()
 {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/walls/v1/mixed-contours.json"
    ))
    .unwrap();
    for unit in ["m", "mm", "ft"] {
        for (x, y) in [(0., 0.), (500000., 4400000.)] {
            for reverse in [false, true] {
                for keep in [false, true] {
                    for reverse_selection in [false, true] {
                        let sources = data["sources"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .cloned()
                            .map(|e| transformed(e, x, y, reverse))
                            .collect::<Vec<_>>();
                        let mut doc = drawing(json!(sources), unit);
                        let before = doc.to_snapshot_v2();
                        let mut uids = (1..=7)
                            .map(|i| doc.uid(Slot(i)).unwrap().to_string())
                            .collect::<Vec<_>>();
                        if reverse_selection {
                            uids.reverse();
                        }
                        let request:WallsFromLines=serde_json::from_value(json!({"uids":uids,"mode":"contours","width":0.2,"justify":"center","keepSources":keep,"arch":{"level":"zemin"}})).unwrap();
                        let result = walls_from_lines::plan_in(&doc, &request);
                        let CommandResult::Completed {
                            output: plan,
                            warnings,
                            ..
                        } = result
                        else {
                            panic!(
                                "{unit} {x} reverse={reverse} selection={reverse_selection}: {result:?}"
                            )
                        };
                        assert_eq!(doc.to_snapshot_v2(), before, "preview is read-only");
                        assert_eq!(plan.walls.len(), 9);
                        assert_eq!(warnings.len(), 1);
                        assert_eq!(warnings[0].code, "wall_shared_boundary");
                        assert!(
                            warnings[0]
                                .message
                                .starts_with("5 ortak kenar parçası (9.000000 m)")
                        );
                        let mut owners = BTreeMap::<String, usize>::new();
                        let mut closed = 0;
                        let mut total = 0.;
                        let mut analytic = 0;
                        for e in &plan.walls {
                            let Entity::Wall(w) = e else { panic!() };
                            total += length(&w.wall);
                            closed += usize::from(w.wall.closed);
                            analytic += usize::from(w.wall.bulges.is_some());
                            assert_eq!(w.wall.width, 0.2);
                            assert_eq!(w.wall.arch.level.as_deref(), Some("zemin"));
                            assert_eq!(w.base.color.as_deref(), Some("#3B94D9"));
                            *owners.entry(w.base.attrs["oda"].clone()).or_default() += 1;
                        }
                        assert_eq!(closed, 6);
                        assert_eq!(analytic, 2);
                        assert!(
                            (total - (115. + 8. * std::f64::consts::PI)).abs() < 1e-7,
                            "{total}"
                        );
                        let expected: BTreeMap<String, usize> = serde_json::from_value(
                            data["expected"][if reverse_selection {
                                "ownersReverse"
                            } else {
                                "ownersForward"
                            }]
                            .clone(),
                        )
                        .unwrap();
                        assert_eq!(owners, expected);
                        completed(walls_from_lines::execute(
                            &mut ExecutionContext::new(&mut doc),
                            request,
                        ));
                        assert_eq!(doc.len(), if keep { 16 } else { 9 });
                        let after = doc.to_snapshot_v2();
                        doc.undo().unwrap();
                        assert_eq!(doc.to_snapshot_v2().entities, before.entities);
                        doc.redo().unwrap();
                        assert_eq!(doc.to_snapshot_v2().entities, after.entities);
                    }
                }
            }
        }
    }
}
#[test]
fn forty_by_forty_room_grid_plans_past_the_old_dense_budget_and_commits_as_one_step() {
    let mut sources = Vec::new();
    for row in 0..40 {
        for col in 0..40 {
            let x = 500000. + f64::from(col) * 4.;
            let y = 4400000. + f64::from(row) * 3.;
            sources.push(json!({"id":sources.len()+1,"kind":"polygon","layerId":"cizim","attrs":{"row":row.to_string(),"col":col.to_string()},"pts":[{"x":x,"y":y},{"x":x+4.,"y":y},{"x":x+4.,"y":y+3.},{"x":x,"y":y+3.}]}));
        }
    }
    let mut doc = drawing(json!(sources), "m");
    let before = doc.to_snapshot_v2();
    let uids = (1..=1600)
        .map(|i| doc.uid(Slot(i)).unwrap().to_string())
        .collect::<Vec<_>>();
    let request: WallsFromLines = serde_json::from_value(
        json!({"uids":uids,"mode":"contours","width":0.2,"justify":"center","keepSources":false}),
    )
    .unwrap();
    let plan = completed(walls_from_lines::plan_in(&doc, &request));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(plan.walls.len(), 3276);
    let total = plan
        .walls
        .iter()
        .map(|e| {
            let Entity::Wall(w) = e else { panic!() };
            length(&w.wall)
        })
        .sum::<f64>();
    assert_eq!(total, 11480.);
    completed(walls_from_lines::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    assert_eq!(doc.len(), 3276);
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    doc.redo().unwrap();
    assert_eq!(doc.len(), 3276);
}

#[test]
fn one_invalid_member_or_reference_refuses_the_whole_mixed_selection_before_any_source_is_deleted()
{
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/walls/v1/mixed-contours.json"
    ))
    .unwrap();
    for case in [
        "open",
        "elevated",
        "locked",
        "reference",
        "unknown_level",
        "duplicate",
    ] {
        let mut sources = data["sources"].as_array().unwrap().clone();
        match case {
            "open" => sources[3]["closed"] = json!(false),
            "elevated" => sources[5]["zs"] = json!([3, 3, 3, 3]),
            "locked" => sources[6]["layerId"] = json!("kilitli"),
            "duplicate" => {
                let mut e = sources[6].clone();
                e["id"] = json!(8);
                sources.push(e);
            }
            _ => (),
        }
        let count = sources.len();
        let mut doc = drawing(json!(sources), "m");
        let uids = (1..=count)
            .map(|i| doc.uid(Slot(i as u32)).unwrap().to_string())
            .collect::<Vec<_>>();
        let mut request:WallsFromLines=serde_json::from_value(json!({"uids":uids,"mode":"contours","width":0.2,"justify":"center","keepSources":false})).unwrap();
        if case == "reference" {
            request.justify = Some(WallJustify::Left);
        }
        if case == "unknown_level" {
            request.arch = Some(ArchInfo {
                level: Some("missing".into()),
                ..Default::default()
            });
        }
        let before = doc.to_snapshot_v2();
        let plan = walls_from_lines::plan_in(&doc, &request);
        assert!(
            matches!(plan, CommandResult::Failed { .. }),
            "{case}: {plan:?}"
        );
        assert_eq!(
            doc.to_snapshot_v2(),
            before,
            "{case}: failed preview writes nothing"
        );
        let result = walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request);
        let CommandResult::Failed { error } = result else {
            panic!("{case}: {result:?}")
        };
        assert_eq!(
            error.code,
            match case {
                "locked" => "layer_locked",
                "unknown_level" => "level_not_found",
                _ => "invalid_wall",
            },
            "{case}"
        );
        assert_eq!(
            doc.to_snapshot_v2(),
            before,
            "{case}: failed commit deletes nothing"
        );
    }
}

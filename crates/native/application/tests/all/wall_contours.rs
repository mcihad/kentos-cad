//! Independent physical expectations; same ordinary closed walls and one command history.
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::{bulge::bulge_ring_area, wall};
use kentos_native_application::{ExecutionContext, walls_from_lines};
use serde_json::{Value, json};
fn drawing(entities: Value, unit: &str) -> Document {
    let mut setup: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.walls.convert.json"
    ))
    .unwrap();
    setup["setup"]["entities"] = entities;
    setup["setup"]["settings"]["drawingUnit"] = json!(unit);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&setup["setup"].to_string()).unwrap())
        .unwrap()
}
fn source(pts: Vec<[f64; 2]>, holes: Value) -> Value {
    let mut e = json!({"kind":"polygon","id":1,"layerId":"cizim","pts":pts.into_iter().map(|p|json!({"x":p[0],"y":p[1]})).collect::<Vec<_>>(),"attrs":{"oda":"A"},"color":"#123456"});
    if !holes.is_null() {
        e["holes"] = holes;
    }
    e
}
fn input(doc: &Document, justify: &str, keep: bool) -> WallsFromLines {
    serde_json::from_value(json!({"uids":[doc.uid(Slot(1)).unwrap().to_string()],"mode":"contours","justify":justify,"width":0.2,"keepSources":keep})).unwrap()
}
fn completed<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}
fn room_area(w: &WallEntity) -> f64 {
    let pts = w
        .wall
        .pts
        .iter()
        .map(|p| kentos_geometry_core::Vec2::new(p.x, p.y))
        .collect::<Vec<_>>();
    let spec = kentos_formats::walls::spec(&w.wall);
    let line = wall::line_at(&pts, w.wall.bulges.as_deref(), true, wall::left_face(&spec)).unwrap();
    bulge_ring_area(&line.pts, Some(&line.bulges))
}
#[test]
fn l_room_dimensions_ignore_winding_units_and_distant_origin_without_writing_preview() {
    let l = [[0., 0.], [6., 0.], [6., 2.], [2., 2.], [2., 5.], [0., 5.]];
    for unit in ["m", "mm", "cm", "in", "ft"] {
        for (x, y) in [(0., 0.), (500000., 4400000.)] {
            for reverse in [false, true] {
                let mut pts = l
                    .into_iter()
                    .map(|p| [p[0] + x, p[1] + y])
                    .collect::<Vec<_>>();
                if reverse {
                    pts[1..].reverse();
                }
                let doc = drawing(json!([source(pts, Value::Null)]), unit);
                let before = doc.to_snapshot_v2();
                for (reference, area) in [("left", 18.), ("right", 13.76), ("center", 15.84)] {
                    let plan = completed(walls_from_lines::plan_in(
                        &doc,
                        &input(&doc, reference, true),
                    ));
                    let Entity::Wall(w) = &plan.walls[0] else {
                        panic!()
                    };
                    assert!(
                        (room_area(w) - area).abs() < 1e-8,
                        "{unit} {x} {reverse} {reference}: {}",
                        room_area(w)
                    );
                    assert_eq!(w.wall.pts[0], Vec2 { x, y });
                    assert_eq!(w.base.attrs.get("oda").map(String::as_str), Some("A"));
                    assert_eq!(w.base.color.as_deref(), Some("#123456"));
                    assert_eq!(doc.to_snapshot_v2(), before);
                }
            }
        }
    }
}
#[test]
fn courtyard_holes_are_separate_hostable_walls_and_one_atomic_undo() {
    let outer = vec![[0., 0.], [10., 0.], [10., 8.], [0., 8.]];
    let holes = json!([{"pts":[{"x":4,"y":3},{"x":6,"y":3},{"x":6,"y":5},{"x":4,"y":5}]}]);
    for (reference, area) in [("left", 76.), ("right", 67.2), ("center", 71.6)] {
        let mut doc = drawing(json!([source(outer.clone(), holes.clone())]), "m");
        let before = doc.to_snapshot_v2();
        let request = input(&doc, reference, false);
        let plan = completed(walls_from_lines::plan_in(&doc, &request));
        assert_eq!(plan.walls.len(), 2);
        let area_found = plan
            .walls
            .iter()
            .map(|e| {
                let Entity::Wall(w) = e else { panic!() };
                room_area(w)
            })
            .sum::<f64>();
        assert!(
            (area_found - area).abs() < 1e-8,
            "{reference}: {area_found}"
        );
        let done = completed(walls_from_lines::execute(
            &mut ExecutionContext::new(&mut doc),
            request,
        ));
        assert_eq!(done.uids.len(), 2);
        assert_eq!(done.removed.len(), 1);
        assert_eq!(doc.len(), 2);
        assert_eq!(
            doc.journal().last().unwrap().command.as_deref(),
            Some("cad.walls.convert")
        );
        let after = doc.to_snapshot_v2();
        assert!(doc.undo().is_some());
        assert_eq!(doc.to_snapshot_v2().entities, before.entities);
        assert!(doc.redo().is_some());
        assert_eq!(doc.to_snapshot_v2().entities, after.entities);
    }
}
#[test]
fn circles_keep_exact_arcs_and_normalize_reversed_bulges() {
    for reverse in [false, true] {
        let entity = if reverse {
            json!({"kind":"polygon","id":1,"layerId":"cizim","pts":[{"x":3,"y":0},{"x":-3,"y":0}],"bulges":[-1,-1],"attrs":{}})
        } else {
            json!({"kind":"circle","id":1,"layerId":"cizim","c":{"x":0,"y":0},"r":3,"attrs":{}})
        };
        let doc = drawing(json!([entity]), "m");
        for (reference, radius) in [("left", 3_f64), ("right", 2.8), ("center", 2.9)] {
            let plan = completed(walls_from_lines::plan_in(
                &doc,
                &input(&doc, reference, true),
            ));
            let Entity::Wall(w) = &plan.walls[0] else {
                panic!()
            };
            assert_eq!(w.wall.pts.len(), 2);
            assert_eq!(w.wall.bulges, Some(vec![1., 1.]));
            assert!((room_area(w) - std::f64::consts::PI * radius * radius).abs() < 1e-10);
        }
    }
}
#[test]
fn repeated_conversion_mixed_open_source_shared_edge_and_collapsing_room_refuse_without_partial_writes()
 {
    let a = source(vec![[0., 0.], [4., 0.], [4., 3.], [0., 3.]], Value::Null);
    let mut doc = drawing(json!([a.clone()]), "m");
    let request = input(&doc, "left", true);
    completed(walls_from_lines::execute(
        &mut ExecutionContext::new(&mut doc),
        request.clone(),
    ));
    let before = doc.to_snapshot_v2();
    assert!(matches!(
        walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
        CommandResult::Failed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    for second in [
        json!({"kind":"line","id":2,"layerId":"cizim","a":{"x":8,"y":0},"b":{"x":10,"y":0},"attrs":{}}),
        json!({"kind":"polygon","id":2,"layerId":"cizim","pts":[{"x":4,"y":0},{"x":8,"y":0},{"x":8,"y":3},{"x":4,"y":3}],"attrs":{}}),
    ] {
        let mut doc = drawing(json!([a.clone(), second]), "m");
        let mut request = input(&doc, "left", false);
        request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
        let before = doc.to_snapshot_v2();
        assert!(matches!(
            walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
            CommandResult::Failed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    for width in [1.5, 2., 3., 8.] {
        let mut doc = drawing(json!([a.clone()]), "m");
        let mut request = input(&doc, "right", false);
        request.width = Some(width);
        let before = doc.to_snapshot_v2();
        assert!(
            matches!(
                walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
                CommandResult::Failed { .. }
            ),
            "width {width}"
        );
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn layer_lock_hidden_type_and_readonly_policy_keep_atomicity_and_elevated_source_is_not_flattened()
{
    let a = source(vec![[0., 0.], [4., 0.], [4., 3.], [0., 3.]], Value::Null);
    let mut second = source(
        vec![[10., 0.], [14., 0.], [14., 3.], [10., 3.]],
        Value::Null,
    );
    second["id"] = json!(2);
    second["layerId"] = json!("kilitli");
    let mut doc = drawing(json!([a.clone(), second]), "m");
    let mut request = input(&doc, "left", false);
    request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
    let before = doc.to_snapshot_v2();
    let CommandResult::Failed { error } =
        walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request)
    else {
        panic!()
    };
    assert_eq!(error.code, "layer_locked");
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut hidden = a.clone();
    hidden["layerId"] = json!("gizli");
    let mut doc = drawing(json!([hidden]), "m");
    let request = input(&doc, "left", true);
    let made = completed(walls_from_lines::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    assert_eq!(
        doc.get(doc.slot_of(made.uids[0].parse().unwrap()).unwrap())
            .unwrap()
            .base()
            .layer_id,
        "gizli"
    );
    let mut doc = drawing(json!([a.clone()]), "m");
    doc.set_object_policy(kentos_domain::ObjectPolicy::AttributesOnly);
    let request = input(&doc, "left", false);
    let before = doc.to_snapshot_v2();
    assert!(!matches!(
        walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
        CommandResult::Completed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut doc = drawing(json!([a.clone()]), "m");
    doc.set_layer_geometry("duvar", Some(LayerGeometry::Point), "Test target type")
        .unwrap(); // explicit incompatible target must fail in plan too
    let mut request = input(&doc, "left", false);
    request.layer_id = Some("duvar".into());
    let before = doc.to_snapshot_v2();
    assert!(matches!(
        walls_from_lines::plan_in(&doc, &request),
        CommandResult::Failed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut elevated = a;
    elevated["zs"] = json!([3, 3, 3, 3]);
    let mut doc = drawing(json!([elevated]), "m");
    let request = input(&doc, "left", false);
    let before = doc.to_snapshot_v2();
    assert!(matches!(
        walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
        CommandResult::Failed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn adjacent_room_axes_share_one_wall_and_preserve_sources_history_units_and_joints() {
    use kentos_geometry_core::geom::{
        arrangement::edge_len,
        bulge::bulge_path_edges,
        wall_join::{WallIn, solve},
    };
    for unit in ["m", "mm", "cm", "in", "ft"] {
        for (x, y) in [(0., 0.), (500000., 4400000.)] {
            for reverse in [false, true] {
                for keep in [false, true] {
                    let mut entities = Vec::new();
                    for (i, left) in [x, x + 4.].into_iter().enumerate() {
                        let mut pts = vec![
                            [left, y],
                            [left + 4., y],
                            [left + 4., y + 3.],
                            [left, y + 3.],
                        ];
                        if reverse {
                            pts[1..].reverse();
                        }
                        let mut e = source(pts, Value::Null);
                        e["id"] = json!(i + 1);
                        e["attrs"] = json!({"oda":if i==0{"A"}else{"B"}});
                        entities.push(e);
                    }
                    let mut doc = drawing(json!(entities), unit);
                    let before = doc.to_snapshot_v2();
                    let mut request = input(&doc, "center", keep);
                    request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
                    let reply = walls_from_lines::plan_in(&doc, &request);
                    let CommandResult::Completed {
                        output: plan,
                        warnings,
                    } = reply
                    else {
                        panic!("{reply:?}")
                    };
                    assert_eq!(plan.walls.len(), 3);
                    assert_eq!(doc.to_snapshot_v2(), before);
                    assert!(
                        warnings.iter().any(|w| w.code == "wall_shared_boundary"
                            && w.message.contains("3.000000 m"))
                    );
                    let fields: Vec<_> = plan
                        .walls
                        .iter()
                        .map(|e| {
                            let Entity::Wall(w) = e else { panic!() };
                            w.wall.clone()
                        })
                        .collect();
                    let paths: Vec<_> = fields
                        .iter()
                        .map(|w| {
                            w.pts
                                .iter()
                                .map(|p| kentos_geometry_core::Vec2::new(p.x, p.y))
                                .collect::<Vec<_>>()
                        })
                        .collect();
                    let length = fields
                        .iter()
                        .zip(&paths)
                        .map(|(w, p)| {
                            bulge_path_edges(p, w.bulges.as_deref(), w.closed)
                                .iter()
                                .map(edge_len)
                                .sum::<f64>()
                        })
                        .sum::<f64>();
                    assert!((length - 25.).abs() < 1e-8, "{length}");
                    let specs: Vec<_> = fields.iter().map(kentos_formats::walls::spec).collect();
                    let inputs: Vec<_> = paths
                        .iter()
                        .zip(&specs)
                        .map(|(pts, spec)| WallIn {
                            pts,
                            bulges: None,
                            spec,
                        })
                        .collect();
                    let joined = solve(&inputs, None);
                    assert!(joined.issues.is_empty(), "{:?}", joined.issues);
                    let mut nodes = joined
                        .joints
                        .iter()
                        .map(|j| (j.at.x.to_bits(), j.at.y.to_bits()))
                        .collect::<Vec<_>>();
                    nodes.sort_unstable();
                    nodes.dedup();
                    assert_eq!(nodes.len(), 2);
                    let body_area = joined
                        .drawn
                        .iter()
                        .flatten()
                        .flat_map(|d| &d.fill)
                        .map(|(outer, holes)| {
                            bulge_ring_area(outer, None).abs()
                                - holes
                                    .iter()
                                    .flatten()
                                    .map(|h| bulge_ring_area(h, None).abs())
                                    .sum::<f64>()
                        })
                        .sum::<f64>();
                    let magnitude = x.abs().max(y.abs());
                    let coordinate_ulp = (magnitude.next_up() - magnitude).max(f64::EPSILON);
                    // Perimeter < 64 m, hence absolute FP64 corner storage
                    // contributes no more than 64 m * coordinate ULP to area.
                    let area_bound = 64.0 * coordinate_ulp + 1e-12;
                    assert!(
                        (body_area - 4.96).abs() <= area_bound,
                        "joined body {body_area}, bound {area_bound}"
                    );
                    completed(walls_from_lines::execute(
                        &mut ExecutionContext::new(&mut doc),
                        request.clone(),
                    ));
                    assert_eq!(doc.len(), if keep { 5 } else { 3 });
                    let after = doc.to_snapshot_v2();
                    doc.undo().unwrap();
                    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
                    doc.redo().unwrap();
                    assert_eq!(doc.to_snapshot_v2().entities, after.entities);
                    if keep {
                        let last = doc.to_snapshot_v2();
                        assert!(matches!(
                            walls_from_lines::execute(
                                &mut ExecutionContext::new(&mut doc),
                                request
                            ),
                            CommandResult::Failed { .. }
                        ));
                        assert_eq!(doc.to_snapshot_v2(), last);
                    }
                }
            }
        }
    }
}
#[test]
fn shared_references_conflict_but_distinct_target_layers_keep_independent_walls() {
    let a = source(vec![[0., 0.], [4., 0.], [4., 3.], [0., 3.]], Value::Null);
    let mut b = source(vec![[4., 0.], [8., 0.], [8., 3.], [4., 3.]], Value::Null);
    b["id"] = json!(2);
    for reference in ["left", "right"] {
        let mut doc = drawing(json!([a.clone(), b.clone()]), "m");
        let mut request = input(&doc, reference, false);
        request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
        let before = doc.to_snapshot_v2();
        assert!(matches!(
            walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request),
            CommandResult::Failed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    b["layerId"] = json!("duvar");
    let doc = drawing(json!([a, b]), "m");
    let mut request = input(&doc, "center", true);
    request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
    let plan = completed(walls_from_lines::plan_in(&doc, &request));
    assert_eq!(plan.walls.len(), 2);
    assert!(
        plan.walls
            .iter()
            .all(|e| matches!(e,Entity::Wall(w) if w.wall.closed))
    );
}

#[test]
fn shared_quarter_arc_remains_analytic_and_open_run_bulges_are_canonical() {
    let b = 2_f64.sqrt() - 1.;
    let a = json!({"kind":"polygon","id":1,"layerId":"cizim","pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":0,"y":4}],"bulges":[0,b,0],"attrs":{"oda":"İç"}});
    let outer = json!({"kind":"polygon","id":2,"layerId":"cizim","pts":[{"x":0,"y":4},{"x":4,"y":0},{"x":6,"y":0},{"x":0,"y":6}],"bulges":[-b,0,b,0],"attrs":{"oda":"Dış"}});
    let mut doc = drawing(json!([a, outer]), "m");
    let mut request = input(&doc, "center", true);
    request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
    let plan = completed(walls_from_lines::plan_in(&doc, &request));
    assert_eq!(plan.walls.len(), 3);
    let mut length = 0.;
    let mut curves = 0;
    for e in &plan.walls {
        let Entity::Wall(w) = e else { panic!() };
        assert!(!w.wall.closed);
        assert!(w.wall.problem().is_none());
        if let Some(b) = &w.wall.bulges {
            assert_eq!(b.len(), w.wall.pts.len());
            assert_eq!(*b.last().unwrap(), 0.);
            curves += 1;
        }
        let pts = w
            .wall
            .pts
            .iter()
            .map(|p| kentos_geometry_core::Vec2::new(p.x, p.y))
            .collect::<Vec<_>>();
        length += kentos_geometry_core::geom::bulge::bulge_path_edges(
            &pts,
            w.wall.bulges.as_deref(),
            false,
        )
        .iter()
        .map(kentos_geometry_core::geom::arrangement::edge_len)
        .sum::<f64>();
    }
    assert_eq!(curves, 2);
    assert!((length - (12. + 5. * std::f64::consts::PI)).abs() < 1e-8);
    completed(walls_from_lines::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    assert_eq!(doc.len(), 5);
}

#[test]
fn coincident_room_contours_convert_once_per_explicit_level_with_atomic_history() {
    for unit in ["m", "mm", "cm", "in", "ft"] {
        for (x, y) in [(0., 0.), (500000., 4400000.)] {
            let mut a = source(
                vec![[x, y], [x + 4., y], [x + 4., y + 3.], [x, y + 3.]],
                Value::Null,
            );
            // Ordinary source attributes have no architectural meaning.
            a["attrs"]["level"] = json!("kat-1");
            let mut b = a.clone();
            b["id"] = json!(2);
            for p in b["pts"].as_array_mut().unwrap() {
                p["x"] = json!(p["x"].as_f64().unwrap() + 4.);
            }
            let mut doc = drawing(json!([a, b]), unit);
            let sources = doc.to_snapshot_v2();
            let mut request = input(&doc, "center", true);
            request.uids.push(doc.uid(Slot(2)).unwrap().to_string());
            for level in [None, Some("zemin"), Some("kat-1")] {
                request.arch = level.map(|id| ArchInfo {
                    level: Some(id.into()),
                    ..Default::default()
                });
                let before = doc.to_snapshot_v2();
                let plan = completed(walls_from_lines::plan_in(&doc, &request));
                assert_eq!(plan.walls.len(), 3);
                for e in &plan.walls {
                    let Entity::Wall(w) = e else { panic!() };
                    assert_eq!(w.wall.arch.level.as_deref(), level);
                    assert!(!w.wall.closed);
                }
                assert_eq!(doc.to_snapshot_v2(), before, "read-only plan");
                let done = completed(walls_from_lines::execute(
                    &mut ExecutionContext::new(&mut doc),
                    request.clone(),
                ));
                assert_eq!(done.uids.len(), 3);
                let after = doc.to_snapshot_v2();
                assert!(doc.undo().is_some());
                assert_eq!(doc.to_snapshot_v2().entities, before.entities, "one undo");
                assert!(doc.redo().is_some());
                assert_eq!(doc.to_snapshot_v2().entities, after.entities);
                assert!(
                    matches!(
                        walls_from_lines::execute(
                            &mut ExecutionContext::new(&mut doc),
                            request.clone()
                        ),
                        CommandResult::Failed { .. }
                    ),
                    "same-level repeat refused"
                );
                assert_eq!(doc.to_snapshot_v2(), after, "repeat is atomic");
            }
            assert_eq!(doc.len(), sources.entities.len() + 9);
        }
    }
}

#[test]
fn unknown_conversion_level_refuses_before_writing_or_removing_sources() {
    let mut doc = drawing(
        json!([source(
            vec![[0., 0.], [4., 0.], [4., 3.], [0., 3.]],
            Value::Null
        )]),
        "m",
    );
    let mut request = input(&doc, "center", false);
    request.arch = Some(ArchInfo {
        level: Some("missing-floor".into()),
        ..Default::default()
    });
    let before = doc.to_snapshot_v2();
    let result = walls_from_lines::execute(&mut ExecutionContext::new(&mut doc), request);
    let value = serde_json::to_value(result).unwrap();
    assert_eq!(value["status"], "failed");
    assert_eq!(value["error"]["code"], "level_not_found");
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn explicit_architecture_is_applied_in_legacy_each_and_pairs_modes_too() {
    for mode in ["each", "pairs"] {
        let doc = drawing(
            json!([
                {"kind":"line","id":1,"layerId":"cizim","a":{"x":0,"y":0},"b":{"x":4,"y":0},"attrs":{}},
                {"kind":"line","id":2,"layerId":"cizim","a":{"x":0,"y":0.2},"b":{"x":4,"y":0.2},"attrs":{}}
            ]),
            "m",
        );
        let req: WallsFromLines = serde_json::from_value(json!({"uids":[doc.uid(Slot(1)).unwrap().to_string(), doc.uid(Slot(2)).unwrap().to_string()],"mode":mode,"arch":{"level":"zemin","phase":"new","status":"proposed","discipline":"architecture"}})).unwrap();
        let plan = completed(walls_from_lines::plan_in(&doc, &req));
        assert_eq!(plan.walls.len(), if mode == "each" { 2 } else { 1 });
        for e in plan.walls {
            let Entity::Wall(w) = e else { panic!() };
            assert_eq!(w.wall.arch, req.arch.clone().unwrap());
        }
    }
}

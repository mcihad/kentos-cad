//! Independent physical sizes through the real plan/write/follower/file paths.
use super::wall_contours::{completed, drawing};
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_native_application::{
    ExecutionContext, schedules_create, spaces, spaces_create, spaces_discover, spaces_edit,
    spaces_quantities, tags_create,
};
use serde_json::json;

fn setup(unit: &str, far: bool) -> Document {
    let (x, y) = if far { (500000., 4400000.) } else { (0., 0.) };
    let p = |a: f64, b: f64| json!({"x":x+a,"y":y+b});
    drawing(
        json!([
            {"kind":"wall","id":1,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"arch":{"level":"zemin"},"pts":[p(0.,0.),p(10.,0.),p(10.,8.),p(0.,8.)]},
            {"kind":"column","id":2,"layerId":"cizim","attrs":{},"at":p(2.,2.),"width":0.6,"depth":0.8,"rotation":0.2,"arch":{"level":"zemin"}},
            {"kind":"column","id":3,"layerId":"cizim","attrs":{},"at":p(7.,4.),"shape":"round","width":1.0,"arch":{"level":"zemin"}},
            {"kind":"column","id":4,"layerId":"cizim","attrs":{},"at":p(5.,4.),"width":2.0,"depth":2.0,"arch":{"level":"kat-1"}},
            {"kind":"grid","id":5,"layerId":"cizim","attrs":{},"name":"A","pts":[p(2.,-2.),p(2.,10.)]},
            {"kind":"grid","id":6,"layerId":"cizim","attrs":{},"name":"1","pts":[p(-2.,2.),p(12.,2.)]}
        ]),
        unit,
    )
}
fn request(
    doc: &Document,
    method: &str,
    policy: Option<&str>,
    level: Option<&str>,
) -> SpacesCreate {
    let far = doc.get(Slot(1)).unwrap().base().id == 1
        && match doc.get(Slot(1)).unwrap() {
            Entity::Wall(w) => w.wall.pts[0].x > 1000.,
            _ => false,
        };
    let mut room = json!({"seed":{"x":if far {500005.}else{5.},"y":if far {4400004.}else{4.}},"method":method});
    if let Some(p) = policy {
        room["columns"] = json!(p);
    }
    if let Some(level) = level {
        room["arch"] = json!({"level":level});
    }
    serde_json::from_value(json!({"layerId":"cizim","spaces":[room]})).unwrap()
}
fn fields(e: &Entity) -> &SpaceFields {
    let Entity::Space(s) = e else { panic!() };
    &s.space
}
fn size(e: &Entity) -> f64 {
    spaces::area_of(&fields(e).parts)
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}
fn create(doc: &mut Document, input: SpacesCreate) -> Slot {
    let done = completed(spaces_create::execute(
        &mut ExecutionContext::new(doc),
        input,
    ));
    Slot(done.ids[0])
}
fn delta() -> f64 {
    0.6 * 0.8 + std::f64::consts::PI / 4.
}
fn tag_text(doc: &Document, slot: Slot) -> &str {
    let Entity::Tag(t) = doc.get(slot).unwrap() else {
        panic!()
    };
    &t.tag.text
}
fn table_values(doc: &Document, slot: Slot) -> String {
    let Entity::Table(t) = doc.get(slot).unwrap() else {
        panic!()
    };
    serde_json::to_string(&t.cells).unwrap()
}

#[test]
fn three_policies_and_methods_in_five_units_preserve_legacy_and_persist_exactly() {
    for unit in ["mm", "cm", "m", "in", "ft"] {
        for far in [false, true] {
            for (method, base) in [
                ("net", 9.8 * 7.8),
                ("centerline", 80.),
                ("gross", 10.2 * 8.2),
            ] {
                for policy in [None, Some("ignore"), Some("net"), Some("all")] {
                    let mut doc = setup(unit, far);
                    let before = doc.to_snapshot_v2();
                    let req = request(&doc, method, policy, Some("zemin"));
                    let expected = base
                        - if policy == Some("all") || (policy == Some("net") && method == "net") {
                            delta()
                        } else {
                            0.
                        };
                    let plan = completed(spaces_create::plan_in(&doc, &req));
                    close(size(&plan.spaces[0]), expected);
                    assert_eq!(doc.to_snapshot_v2(), before);
                    let slot = create(&mut doc, req);
                    let saved = doc.to_snapshot_v2();
                    close(size(doc.get(slot).unwrap()), expected);
                    let bytes = kentos_kcad::encode(&saved).unwrap();
                    let read = kentos_kcad::decode(&bytes).unwrap();
                    assert_eq!(read, saved);
                    assert_eq!(kentos_kcad::encode(&read).unwrap(), bytes);
                    doc.undo().unwrap();
                    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
                    doc.redo().unwrap();
                    assert_eq!(doc.to_snapshot_v2().entities, saved.entities);
                    assert_eq!(doc.to_snapshot_v2().uids, saved.uids);
                }
            }
            let mut doc = setup(unit, far);
            let req = request(&doc, "net", Some("net"), None);
            let slot = create(&mut doc, req);
            close(size(doc.get(slot).unwrap()), 9.8 * 7.8 - delta() - 4.);
        }
    }
}

#[test]
fn grid_settlement_precedes_room_label_and_schedule_in_one_undo_and_group() {
    let mut doc = setup("m", false);
    let mut col = doc.get(Slot(2)).unwrap().clone();
    let Entity::Column(c) = &mut col else {
        panic!()
    };
    c.column.rotation = 0.;
    c.column.grids = Some(vec![
        doc.uid(Slot(5)).unwrap().to_string(),
        doc.uid(Slot(6)).unwrap().to_string(),
    ]);
    assert!(doc.update(Slot(2), col));
    let req = request(&doc, "net", Some("net"), Some("zemin"));
    let room = create(&mut doc, req);
    let uid = doc.uid(room).unwrap().to_string();
    let tags: TagsCreate =
        serde_json::from_value(json!({"layerId":"cizim","tags":[{"of":uid,"content":"{area}"}]}))
            .unwrap();
    let tag = Slot(
        completed(tags_create::execute(
            &mut ExecutionContext::new(&mut doc),
            tags,
        ))
        .ids[0],
    );
    let table: SchedulesCreate = serde_json::from_value(
        json!({"layerId":"cizim","at":{"x":12.,"y":0.},"of":"spaces","columns":[{"field":"area"}]}),
    )
    .unwrap();
    let table = Slot(
        completed(schedules_create::execute(
            &mut ExecutionContext::new(&mut doc),
            table,
        ))
        .id,
    );
    assert!(
        tag_text(&doc, tag).contains("75.17"),
        "{}",
        tag_text(&doc, tag)
    );
    assert!(table_values(&doc, table).contains("75.17"));
    let before = doc.to_snapshot_v2();
    let mut grid = doc.get(Slot(5)).unwrap().clone();
    let Entity::Grid(g) = &mut grid else { panic!() };
    for p in &mut g.grid.pts {
        p.x = 10.;
    }
    assert!(doc.update(Slot(5), grid));
    let Entity::Column(c) = doc.get(Slot(2)).unwrap() else {
        panic!()
    };
    assert_eq!(c.column.at.x, 10.);
    close(
        size(doc.get(room).unwrap()),
        9.8 * 7.8 - std::f64::consts::PI / 4. - 0.2 * 0.8,
    );
    assert!(tag_text(&doc, tag).contains("75.49"));
    assert!(table_values(&doc, table).contains("75.49"));
    let after = doc.to_snapshot_v2();
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    doc.redo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, after.entities);
    doc.undo().unwrap();
    let group = doc.begin_group("Kolon ve aks");
    let mut grid = doc.get(Slot(5)).unwrap().clone();
    let Entity::Grid(g) = &mut grid else { panic!() };
    for p in &mut g.grid.pts {
        p.x = 10.;
    }
    assert!(doc.update(Slot(5), grid));
    let mut col = doc.get(Slot(2)).unwrap().clone();
    let Entity::Column(c) = &mut col else {
        panic!()
    };
    c.column.depth = Some(1.2);
    assert!(doc.update(Slot(2), col));
    doc.end_group(group);
    close(
        size(doc.get(room).unwrap()),
        9.8 * 7.8 - std::f64::consts::PI / 4. - 0.2 * 1.2,
    );
    assert!(tag_text(&doc, tag).contains("75.41"));
    assert!(table_values(&doc, table).contains("75.41"));
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn column_edits_additions_deletions_and_levels_refresh_only_opting_in_rooms() {
    let mut doc = setup("m", false);
    let old = request(&doc, "net", None, Some("zemin"));
    let old = create(&mut doc, old);
    let old_entity = doc.get(old).unwrap().clone();
    let req = request(&doc, "net", Some("net"), Some("zemin"));
    let room = create(&mut doc, req);
    let initial = doc.to_snapshot_v2();
    let mut col = doc.get(Slot(2)).unwrap().clone();
    let Entity::Column(c) = &mut col else {
        panic!()
    };
    c.column.width = 1.2;
    assert!(doc.update(Slot(2), col));
    close(
        size(doc.get(room).unwrap()),
        9.8 * 7.8 - 0.96 - std::f64::consts::PI / 4.,
    );
    assert_eq!(doc.get(old), Some(&old_entity));
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, initial.entities);
    doc.remove(&[Slot(2)]);
    close(
        size(doc.get(room).unwrap()),
        9.8 * 7.8 - std::f64::consts::PI / 4.,
    );
    doc.undo().unwrap();
    let mut col = doc.get(Slot(2)).unwrap().clone();
    let Entity::Column(c) = &mut col else {
        panic!()
    };
    c.column.arch.level = Some("kat-1".into());
    assert!(doc.update(Slot(2), col));
    close(
        size(doc.get(room).unwrap()),
        9.8 * 7.8 - std::f64::consts::PI / 4.,
    );
    doc.undo().unwrap();
    let mut copy = doc.get(Slot(2)).unwrap().clone();
    copy.base_mut().id = 0;
    let added = doc.add(copy).unwrap();
    close(size(doc.get(room).unwrap()), 9.8 * 7.8 - delta());
    assert!(doc.get(added).is_some());
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, initial.entities);
    let q = completed(spaces_quantities::plan_in(&doc, &Default::default()));
    close(
        q.spaces
            .iter()
            .find(|s| s.uid == doc.uid(room).unwrap().to_string())
            .unwrap()
            .area,
        9.8 * 7.8 - delta(),
    );
}

#[test]
fn full_coverage_and_bad_sources_retain_last_geometry_then_recover_the_same_room() {
    let mut doc = setup("m", false);
    let req = request(&doc, "net", Some("net"), Some("zemin"));
    let room = create(&mut doc, req);
    let initial = doc.to_snapshot_v2();
    let first = doc.get(room).unwrap().clone();
    let col = doc.get(Slot(2)).unwrap().clone();
    let mut huge = col.clone();
    let Entity::Column(c) = &mut huge else {
        panic!()
    };
    c.column.at = Vec2 { x: 5., y: 4. };
    c.column.width = 100.;
    c.column.depth = Some(100.);
    assert!(doc.update(Slot(2), huge));
    let f = fields(doc.get(room).unwrap());
    assert_eq!(f.broken, Some(SpaceBreak::NoFreeArea));
    assert_eq!(f.parts, fields(&first).parts);
    let q = completed(spaces_quantities::plan_in(&doc, &Default::default()));
    let row = q
        .spaces
        .iter()
        .find(|s| s.uid == doc.uid(room).unwrap().to_string())
        .unwrap();
    assert_eq!(row.columns, SpaceColumns::Net);
    assert_eq!(row.boundary_break, Some(SpaceBreak::NoFreeArea));
    assert!(row.rule.contains("son geçerli sınır"));
    close(row.area, size(&first));
    let wire = serde_json::to_value(row).unwrap();
    assert_eq!(wire["columns"], "net");
    assert_eq!(wire["boundaryBreak"], "noFreeArea");
    let before = doc.to_snapshot_v2();
    let req = request(&doc, "net", Some("net"), Some("zemin"));
    assert!(
        matches!(spaces_create::execute(&mut ExecutionContext::new(&mut doc),req),CommandResult::Failed{error} if error.code=="invalid_space")
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    let req: SpacesDiscover =
        serde_json::from_value(json!({"layerId":"cizim","level":"zemin","columns":"net"})).unwrap();
    assert!(
        completed(spaces_discover::plan_in(&doc, &req))
            .proposals
            .is_empty()
    );
    assert!(doc.update(Slot(2), col.clone()));
    assert_eq!(doc.get(room), Some(&first));
    let mut bad = col.clone();
    let Entity::Column(c) = &mut bad else {
        panic!()
    };
    c.column.at = Vec2 { x: 1e20, y: 1e20 };
    assert!(doc.update(Slot(2), bad));
    assert_eq!(
        fields(doc.get(room).unwrap()).broken,
        Some(SpaceBreak::InvalidBoundarySource)
    );
    assert_eq!(fields(doc.get(room).unwrap()).parts, fields(&first).parts);
    assert!(
        matches!(spaces_discover::plan_in(&doc,&req),CommandResult::Failed{error} if error.code=="invalid_space")
    );
    assert!(doc.update(Slot(2), col));
    assert_eq!(doc.get(room), Some(&first));
    assert_eq!(doc.to_snapshot_v2().uids, initial.uids);
}

#[test]
fn drawn_boundaries_require_explicit_rebinding_and_policy_edit_preview_is_read_only() {
    let mut doc = setup("m", false);
    let before = doc.to_snapshot_v2();
    let bad:SpacesCreate=serde_json::from_value(json!({"layerId":"cizim","spaces":[{"parts":[{"outer":{"pts":[{"x":1.,"y":1.},{"x":3.,"y":1.},{"x":3.,"y":3.},{"x":1.,"y":3.}]}}],"columns":"net"}]})).unwrap();
    assert!(
        matches!(spaces_create::plan_in(&doc,&bad),CommandResult::Failed{error} if error.code=="invalid_space"&&error.path.as_deref()==Some("spaces[0].columns"))
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    let req = request(&doc, "net", None, Some("zemin"));
    let room = create(&mut doc, req);
    let before = doc.to_snapshot_v2();
    let req:SpacesEdit=serde_json::from_value(json!({"uids":[doc.uid(room).unwrap().to_string()],"operation":{"kind":"set","columns":"net"}})).unwrap();
    let plan = completed(spaces_edit::plan_in(&doc, &req));
    close(size(&plan.spaces[0]), 9.8 * 7.8 - delta());
    assert_eq!(doc.to_snapshot_v2(), before);
    completed(spaces_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        req,
    ));
    assert_eq!(fields(doc.get(room).unwrap()).columns, SpaceColumns::Net);
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn column_pruning_and_invalidation_use_curved_room_extrema_not_just_two_endpoints() {
    let mut doc = drawing(
        json!([
            {"kind":"wall","id":1,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"bulges":[1.,1.],"arch":{"level":"zemin"},"pts":[{"x":499995.,"y":4400000.},{"x":500005.,"y":4400000.}]},
            {"kind":"column","id":2,"layerId":"cizim","attrs":{},"width":0.6,"depth":0.8,"at":{"x":500000.,"y":4400003.},"arch":{"level":"zemin"}}
        ]),
        "mm",
    );
    let mut req = request(&doc, "net", Some("net"), Some("zemin"));
    req.spaces[0].seed = Some(Vec2 {
        x: 500000.,
        y: 4400000.,
    });
    let room = create(&mut doc, req);
    close(
        size(doc.get(room).unwrap()),
        std::f64::consts::PI * 4.9 * 4.9 - 0.48,
    );
    let before = doc.to_snapshot_v2();
    let mut col = doc.get(Slot(2)).unwrap().clone();
    let Entity::Column(c) = &mut col else {
        panic!()
    };
    c.column.width = 1.;
    assert!(doc.update(Slot(2), col));
    close(
        size(doc.get(room).unwrap()),
        std::f64::consts::PI * 4.9 * 4.9 - 0.8,
    );
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn splitting_and_merging_detach_column_following_without_restoring_deducted_area() {
    let mut doc = setup("m", false);
    let req = request(&doc, "net", Some("net"), Some("zemin"));
    let room = create(&mut doc, req);
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(room).unwrap().to_string();
    let split: SpacesEdit = serde_json::from_value(json!({"uids":[uid],"operation":{"kind":"split","line":[{"x":4.,"y":-1.},{"x":4.,"y":9.}]}})).unwrap();
    let plan = completed(spaces_edit::plan_in(&doc, &split));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut sum = 0.;
    for e in plan.spaces.iter().chain(&plan.added) {
        let f = fields(e);
        assert!(f.seed.is_none());
        assert_eq!(f.columns, SpaceColumns::Ignore);
        sum += size(e);
    }
    close(sum, 9.8 * 7.8 - delta());
    completed(spaces_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        split,
    ));
    let uids: Vec<_> = doc
        .entities()
        .filter(|e| matches!(e, Entity::Space(_)))
        .map(|e| doc.uid(Slot(e.base().id)).unwrap().to_string())
        .collect();
    assert_eq!(uids.len(), 2);
    let merge: SpacesEdit =
        serde_json::from_value(json!({"uids":uids,"operation":{"kind":"merge"}})).unwrap();
    completed(spaces_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        merge,
    ));
    let f = fields(doc.get(room).unwrap());
    assert!(f.seed.is_none());
    assert_eq!(f.columns, SpaceColumns::Ignore);
    close(size(doc.get(room).unwrap()), 9.8 * 7.8 - delta());
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn bulk_column_deduction_recognises_an_equivalent_drawn_room_and_refuses_partial_overlap() {
    for policy in ["net", "all"] {
        let mut doc = setup("m", false);
        let mut col = doc.get(Slot(2)).unwrap().clone();
        let Entity::Column(c) = &mut col else {
            panic!()
        };
        c.column.rotation = 0.;
        assert!(doc.update(Slot(2), col));
        // Independently drawn Net footprint: analytic circular hole and rectangular hole.
        let drawn: SpacesCreate = serde_json::from_value(json!({"layerId":"cizim","spaces":[{
            "arch":{"level":"zemin"},"parts":[{
                "outer":{"pts":[{"x":0.1,"y":0.1},{"x":9.9,"y":0.1},{"x":9.9,"y":7.9},{"x":0.1,"y":7.9}]},
                "holes":[
                    {"pts":[{"x":1.7,"y":1.6},{"x":1.7,"y":2.4},{"x":2.3,"y":2.4},{"x":2.3,"y":1.6}]},
                    {"pts":[{"x":6.5,"y":4.},{"x":7.5,"y":4.}],"bulges":[-1.,-1.]}
                ]
            }]}]})).unwrap();
        let manual = create(&mut doc, drawn);
        let req: SpacesDiscover =
            serde_json::from_value(json!({"layerId":"cizim","level":"zemin","columns":policy}))
                .unwrap();
        let before = doc.to_snapshot_v2();
        let planned = completed(spaces_discover::plan_in(&doc, &req));
        assert_eq!(planned.proposals.len(), 1);
        assert_eq!(
            planned.proposals[0].existing,
            vec![doc.uid(manual).unwrap().to_string()]
        );
        assert!(planned.proposals[0].conflicts.is_empty());
        let done = completed(spaces_discover::execute(
            &mut ExecutionContext::new(&mut doc),
            req.clone(),
        ));
        assert!(done.ids.is_empty());
        assert_eq!(doc.to_snapshot_v2(), before);
        let partial: SpacesCreate = serde_json::from_value(json!({"layerId":"cizim","spaces":[{
            "arch":{"level":"zemin"},"parts":[{"outer":{"pts":[{"x":1.,"y":1.},{"x":4.,"y":1.},{"x":4.,"y":3.},{"x":1.,"y":3.}]}}]}]})).unwrap();
        let partial = create(&mut doc, partial);
        let before = doc.to_snapshot_v2();
        let planned = completed(spaces_discover::plan_in(&doc, &req));
        assert_eq!(
            planned.proposals[0].conflicts,
            vec![doc.uid(partial).unwrap().to_string()]
        );
        assert!(matches!(
            spaces_discover::execute(&mut ExecutionContext::new(&mut doc), req),
            CommandResult::Failed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn analytic_column_hole_remains_measurable_through_the_public_dimension_command() {
    use kentos_geometry_core::Vec2 as CorePoint;
    use kentos_geometry_core::geom::intersect::Edge;
    use kentos_native_application::{dimension_create, dimensions_create};
    for far in [false, true] {
        let mut doc = setup("mm", far);
        let req = request(&doc, "net", Some("net"), Some("zemin"));
        let room = create(&mut doc, req);
        let uid = doc.uid(room).unwrap().to_string();
        let (x, y) = if far { (500007., 4400004.) } else { (7., 4.) };
        let near = CorePoint::new(x + 0.5, y);
        let e = doc.get(room).unwrap();
        let (center, radius) = dimension_create::circle_at(e, near).unwrap();
        close(center.x, x);
        close(center.y, y);
        close(radius, 0.5);
        let Edge::Arc { r, sweep, .. } = dimension_create::arc_edge_at(e, near).unwrap() else {
            panic!()
        };
        close(r, 0.5);
        close(sweep.abs(), std::f64::consts::PI);
        for kind in ["radius", "arcLength"] {
            let target = kentos_commands::capabilities::capabilities_of("cad.dimensions.create")
                .unwrap()
                .targets
                .into_iter()
                .find(|t| t.operation.as_deref() == Some(kind))
                .unwrap();
            assert!(target.kinds.iter().any(|k| k == "space"));
            let input: DimensionsCreate = serde_json::from_value(json!({
                "measure":{"kind":kind,"of":{"uid":uid,"near":{"x":near.x,"y":near.y}}},
                "at":{"x":x+2.,"y":y+2.},"layerId":"cizim"}))
            .unwrap();
            let before = doc.to_snapshot_v2();
            let plan = completed(dimensions_create::plan_in(&doc, &input));
            assert_eq!(plan.entities.len(), 1);
            assert_eq!(doc.to_snapshot_v2(), before);
            let made = completed(dimensions_create::execute(
                &mut ExecutionContext::new(&mut doc),
                input,
            ));
            assert_eq!(made.ids.len(), 1);
            assert_eq!(doc.get(room), Some(e_from(&before, room)));
            doc.undo().unwrap();
            assert_eq!(doc.to_snapshot_v2().entities, before.entities);
        }
    }
}

fn e_from(snapshot: &DocumentSnapshotV2, slot: Slot) -> &Entity {
    snapshot
        .entities
        .iter()
        .find(|e| e.base().id == slot.0)
        .unwrap()
}

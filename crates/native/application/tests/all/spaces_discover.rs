use super::wall_contours::{completed, drawing};
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, spaces, spaces_create, spaces_discover};
use serde_json::json;

fn setup(unit: &str) -> Document {
    let walls = json!([
        {"kind":"wall","id":1,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"arch":{"level":"zemin"},
        "pts":[{"x":500000.,"y":4400000.},{"x":500010.,"y":4400000.},{"x":500010.,"y":4400008.},{"x":500000.,"y":4400008.}]},
        {"kind":"wall","id":2,"layerId":"cizim","attrs":{},"width":0.2,"arch":{"level":"zemin"},"pts":[{"x":500005.,"y":4400000.},{"x":500005.,"y":4400008.}]},
        {"kind":"wall","id":3,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,"arch":{"level":"kat-1"},
        "pts":[{"x":500000.,"y":4400000.},{"x":500012.,"y":4400000.},{"x":500012.,"y":4400010.},{"x":500000.,"y":4400010.}]}]);
    drawing(walls, unit)
}
fn by_uid<'a>(doc: &'a Document, uid: &str) -> &'a Entity {
    doc.get(doc.slot_of(uid.parse().unwrap()).unwrap()).unwrap()
}
fn request() -> SpacesDiscover {
    serde_json::from_value(json!({"layerId":"cizim","level":"zemin"})).unwrap()
}
fn fields(e: &Entity) -> &SpaceFields {
    let Entity::Space(s) = e else { panic!() };
    &s.space
}
fn execute(doc: &mut Document, input: SpacesDiscover) -> SpacesCreated {
    completed(spaces_discover::execute(
        &mut ExecutionContext::new(doc),
        input,
    ))
}

#[test]
fn measured_level_network_is_read_only_then_atomic_idempotent_and_persistent() {
    for unit in ["mm", "cm", "m", "in", "ft"] {
        let mut doc = setup(unit);
        let before = doc.to_snapshot_v2();
        let input = request();
        let plan = completed(spaces_discover::plan_in(&doc, &input));
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(plan.proposals.len(), 2);
        for p in &plan.proposals {
            assert!(p.existing.is_empty() && p.conflicts.is_empty());
            assert!((spaces::area_of(&p.space.parts) - 4.8 * 7.8).abs() < 1e-7);
        }
        let mut input = input;
        input.expected_revision = Some(plan.revision);
        let done = execute(&mut doc, input);
        assert_eq!(done.uids.len(), 2);
        let made = doc.to_snapshot_v2();
        for (old, new) in before.entities.iter().zip(&made.entities) {
            assert_eq!(old, new);
        }
        let entities: Vec<_> = done.uids.iter().map(|u| by_uid(&doc, u).clone()).collect();
        let repeat = completed(spaces_discover::plan_in(&doc, &request()));
        assert!(
            repeat
                .proposals
                .iter()
                .all(|p| p.existing.len() == 1 && p.conflicts.is_empty())
        );
        let repeated = execute(&mut doc, request());
        assert!(repeated.uids.is_empty());
        assert_eq!(doc.to_snapshot_v2().entities, made.entities);
        doc.undo().unwrap();
        assert_eq!(doc.to_snapshot_v2().entities, before.entities);
        doc.redo().unwrap();
        assert_eq!(doc.to_snapshot_v2().entities, made.entities);
        let opened = Document::from_snapshot_v2(doc.to_snapshot_v2()).unwrap();
        for (uid, e) in done.uids.iter().zip(entities) {
            assert_eq!(Some(by_uid(&opened, uid)), Some(&e));
        }
        let mut upper = request();
        upper.level = Some("kat-1".into());
        let plan = completed(spaces_discover::plan_in(&doc, &upper));
        assert_eq!(plan.proposals.len(), 1);
        assert!((spaces::area_of(&plan.proposals[0].space.parts) - 11.8 * 9.8).abs() < 1e-7);
        assert!(plan.proposals[0].existing.is_empty());
    }
}

#[test]
fn region_filters_representatives_without_cutting_sources_and_stale_or_locked_is_atomic() {
    let mut doc = setup("m");
    let mut input = request();
    input.bounds = Some([
        Vec2 {
            x: 500006.,
            y: 4400000.,
        },
        Vec2 {
            x: 500011.,
            y: 4400008.,
        },
    ]);
    let plan = completed(spaces_discover::plan_in(&doc, &input));
    assert_eq!(plan.proposals.len(), 1);
    assert!((spaces::area_of(&plan.proposals[0].space.parts) - 4.8 * 7.8).abs() < 1e-7);
    let mut wall = doc.get(Slot(2)).unwrap().clone();
    if let Entity::Wall(w) = &mut wall {
        w.wall.width = 0.4;
    }
    doc.update(Slot(2), wall);
    input.expected_revision = Some(plan.revision);
    let before = doc.to_snapshot_v2();
    assert!(matches!(
        spaces_discover::execute(&mut ExecutionContext::new(&mut doc), input),
        CommandResult::Conflict { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut bad = request();
    bad.level = Some("missing".into());
    assert!(
        matches!(spaces_discover::plan_in(&doc,&bad),CommandResult::Failed{error} if error.code=="level_not_found")
    );
    let mut locked = request();
    locked.layer_id = "kilitli".into();
    let before = doc.to_snapshot_v2();
    assert!(
        matches!(spaces_discover::execute(&mut ExecutionContext::new(&mut doc),locked),CommandResult::Failed{error} if error.code=="layer_locked")
    );
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn a_simple_plan_without_level_definitions_follows_moving_partition_in_one_undo() {
    let mut doc = setup("mm");
    doc.remove(&[Slot(3)]);
    for slot in [Slot(1), Slot(2)] {
        let mut e = doc.get(slot).unwrap().clone();
        if let Entity::Wall(w) = &mut e {
            w.wall.arch = Default::default();
        }
        doc.update(slot, e);
    }
    let mut settings = doc.settings().clone();
    settings.arch = None;
    doc.set_settings(settings);
    let mut input = request();
    input.level = None;
    let done = execute(&mut doc, input);
    assert_eq!(done.uids.len(), 2);
    let before = doc.to_snapshot_v2();
    let mut e = doc.get(Slot(2)).unwrap().clone();
    if let Entity::Wall(w) = &mut e {
        for p in &mut w.wall.pts {
            p.x += 1.;
        }
    }
    doc.update(Slot(2), e);
    let mut areas: Vec<_> = done
        .uids
        .iter()
        .map(|u| spaces::area_of(&fields(by_uid(&doc, u)).parts))
        .collect();
    areas.sort_by(f64::total_cmp);
    assert!((areas[0] - 3.8 * 7.8).abs() < 1e-7);
    assert!((areas[1] - 5.8 * 7.8).abs() < 1e-7);
    let moved = doc.to_snapshot_v2();
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    doc.redo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, moved.entities);
}

#[test]
fn exact_drawn_room_is_preserved_and_partial_overlap_blocks_every_new_room() {
    let mut doc = setup("m");
    let input:SpacesCreate=serde_json::from_value(json!({"layerId":"cizim","spaces":[{"arch":{"level":"zemin"},"name":"Elle çizilmiş","parts":[{"outer":{"pts":[{"x":500000.1,"y":4400000.1},{"x":500004.9,"y":4400000.1},{"x":500004.9,"y":4400007.9},{"x":500000.1,"y":4400007.9}]}}]}]})).unwrap();
    let old = completed(spaces_create::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ));
    let plan = completed(spaces_discover::plan_in(&doc, &request()));
    assert_eq!(
        plan.proposals
            .iter()
            .filter(|p| !p.existing.is_empty())
            .count(),
        1
    );
    let before = doc.to_snapshot_v2();
    let new = execute(&mut doc, request());
    assert_eq!(new.uids.len(), 1);
    assert_eq!(
        by_uid(&doc, &old.uids[0]),
        &before.entities.last().unwrap().clone()
    );
    doc.undo().unwrap();
    let slot = by_uid(&doc, &old.uids[0]).base().id;
    let mut e = doc.get(Slot(slot)).unwrap().clone();
    if let Entity::Space(s) = &mut e {
        s.space.parts[0].outer.pts[1].x -= 1.;
        s.space.parts[0].outer.pts[2].x -= 1.;
    }
    doc.update(Slot(slot), e);
    let plan = completed(spaces_discover::plan_in(&doc, &request()));
    assert!(plan.proposals.iter().any(|p| !p.conflicts.is_empty()));
    let before = doc.to_snapshot_v2();
    assert!(
        matches!(spaces_discover::execute(&mut ExecutionContext::new(&mut doc),request()),CommandResult::Failed{error} if error.code=="invalid_space")
    );
    assert_eq!(doc.to_snapshot_v2(), before);
}

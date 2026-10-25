//! Physical room sizes at overlapping semantic levels, independent of the solver.
use super::wall_contours::{completed, drawing};
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, spaces, spaces_create, spaces_edit};
use serde_json::{Value, json};

fn wall(id: u32, level: Option<&str>, width: f64, height: f64) -> Value {
    let mut e = json!({"kind":"wall","id":id,"layerId":"cizim","attrs":{},"width":0.2,"closed":true,
        "pts":[{"x":500000.,"y":4400000.},{"x":500000.+width,"y":4400000.},
        {"x":500000.+width,"y":4400000.+height},{"x":500000.,"y":4400000.+height}]});
    if let Some(level) = level {
        e["arch"] = json!({"level":level});
    }
    e
}
fn setup(unit: &str) -> Document {
    let mut doc = drawing(
        json!([
            wall(1, Some("zemin"), 6., 4.),
            wall(2, Some("kat-1"), 10., 8.),
            wall(3, None, 12., 10.)
        ]),
        unit,
    );
    let mut settings = doc.settings().clone();
    // Semantic identities stay distinct even at the same elevation.
    for level in &mut settings.arch.as_mut().unwrap().levels {
        level.elevation = 0.;
    }
    doc.set_settings(settings);
    doc
}
fn request(level: Option<&str>) -> SpacesCreate {
    let mut n = json!({"seed":{"x":500002.,"y":4400002.},"number":"Z-01","name":"Salon"});
    if let Some(level) = level {
        n["arch"] = json!({"level":level});
    }
    serde_json::from_value(json!({"layerId":"cizim","spaces":[n]})).unwrap()
}
fn size(e: &Entity) -> f64 {
    let Entity::Space(s) = e else {
        panic!("not a space")
    };
    spaces::area_of(&s.space.parts)
}
#[test]
fn explicit_room_levels_isolate_overlapping_walls_and_preserve_legacy_unassigned_lookup() {
    for unit in ["m", "mm", "ft"] {
        let mut doc = setup(unit);
        let before = doc.to_snapshot_v2();
        for (level, expected) in [
            (Some("zemin"), 5.8 * 3.8),
            (Some("kat-1"), 9.8 * 7.8),
            (None, 5.8 * 3.8),
        ] {
            let input = request(level);
            let plan = completed(spaces_create::plan_in(&doc, &input));
            assert!(
                (size(&plan.spaces[0]) - expected).abs() < 1e-7,
                "{unit} {level:?}"
            );
            assert_eq!(doc.to_snapshot_v2(), before);
            let out = completed(spaces_create::execute(
                &mut ExecutionContext::new(&mut doc),
                input,
            ));
            assert_eq!(out.ids.len(), 1);
            let space = doc.get(Slot(out.ids[0])).unwrap().clone();
            assert_eq!(space, {
                let mut e = plan.spaces[0].clone();
                e.base_mut().id = out.ids[0];
                e
            });
            let Entity::Space(s) = &space else { panic!() };
            assert_eq!(s.space.arch.level.as_deref(), level);
            assert!(s.space.broken.is_none());
            let reopened = Document::from_snapshot_v2(doc.to_snapshot_v2()).unwrap();
            assert_eq!(reopened.get(Slot(out.ids[0])), Some(&space));
            doc.undo().unwrap();
            assert_eq!(doc.to_snapshot_v2().entities, before.entities);
            doc.redo().unwrap();
            assert_eq!(doc.get(Slot(out.ids[0])), Some(&space));
            doc.undo().unwrap();
        }
    }
}
#[test]
fn another_level_cannot_close_a_seed_and_unknown_level_is_an_atomic_refusal() {
    let mut doc = setup("m");
    for (level, code) in [
        ("zemin", "space_not_enclosed"),
        ("missing", "level_not_found"),
    ] {
        let mut input = request(Some(level));
        input.spaces[0].seed = Some(Vec2 {
            x: 500008.,
            y: 4400006.,
        });
        let before = doc.to_snapshot_v2();
        assert!(
            matches!(spaces_create::plan_in(&doc,&input),CommandResult::Failed{error} if error.code==code)
        );
        assert!(
            matches!(spaces_create::execute(&mut ExecutionContext::new(&mut doc),input),CommandResult::Failed{error} if error.code==code)
        );
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}
#[test]
fn explicit_level_edit_preview_follow_history_quantities_and_lost_enclosure_stay_together() {
    let mut doc = setup("m");
    let ground = completed(spaces_create::execute(
        &mut ExecutionContext::new(&mut doc),
        request(Some("zemin")),
    ));
    let upper = completed(spaces_create::execute(
        &mut ExecutionContext::new(&mut doc),
        request(Some("kat-1")),
    ));
    let slot = Slot(ground.ids[0]);
    let upper_slot = Slot(upper.ids[0]);
    let upper_before = doc.get(upper_slot).unwrap().clone();
    let before = doc.to_snapshot_v2();
    let mut changed = doc.get(Slot(1)).unwrap().clone();
    let Entity::Wall(w) = &mut changed else {
        panic!()
    };
    w.wall.width = 0.4;
    assert!(doc.update(Slot(1), changed));
    assert!((size(doc.get(slot).unwrap()) - 5.6 * 3.6).abs() < 1e-7);
    assert_eq!(doc.get(upper_slot), Some(&upper_before));
    let quantities = completed(kentos_native_application::spaces_quantities::plan_in(
        &doc,
        &Default::default(),
    ));
    assert!(
        (quantities
            .spaces
            .iter()
            .find(|s| s.uid == ground.uids[0])
            .unwrap()
            .area
            - 5.6 * 3.6)
            .abs()
            < 1e-7
    );
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    doc.redo().unwrap();
    assert!((size(doc.get(slot).unwrap()) - 5.6 * 3.6).abs() < 1e-7);
    doc.undo().unwrap();
    let edit: SpacesEdit = serde_json::from_value(
        json!({"uids":ground.uids,"operation":{"kind":"set","level":"kat-1"}}),
    )
    .unwrap();
    let plan = completed(spaces_edit::plan_in(&doc, &edit));
    assert!((size(&plan.spaces[0]) - 9.8 * 7.8).abs() < 1e-7);
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    completed(spaces_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        edit,
    ));
    assert!((size(doc.get(slot).unwrap()) - 9.8 * 7.8).abs() < 1e-7);
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    doc.remove(&[Slot(1)]);
    let Entity::Space(s) = doc.get(slot).unwrap() else {
        panic!()
    };
    assert_eq!(s.space.broken, Some(SpaceBreak::LostEnclosure));
    assert!((size(doc.get(slot).unwrap()) - 5.8 * 3.8).abs() < 1e-7);
    assert_eq!(doc.get(upper_slot), Some(&upper_before));
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
}

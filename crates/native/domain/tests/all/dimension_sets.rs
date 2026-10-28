//! Independent expected source geometry, identity and undo checks for automatic wall dimensions.

use kentos_domain::contracts::{
    DimensionSet, DimensionSetBreak, DimensionSetFace, DimensionSetId, DimensionSetOpenings,
    DimensionSetRow, DocumentSnapshotV1, Entity, EntityId, TextShift, Vec2,
};
use kentos_domain::{Document, ObjectPolicy, Slot, Uuid};
use serde_json::json;

const SET: DimensionSetId = DimensionSetId([81; 16]);

fn entity(value: serde_json::Value) -> Entity {
    serde_json::from_value(value).unwrap()
}
fn fixture(origin: Vec2) -> (Document, Slot, Slot) {
    let value = json!({"format":"kentos.document","version":1,"name":"Ölçü sıraları",
        "settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50},
        "origin":origin,"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],"activeLayer":"a",
        "entities":[],"styles":{"items":[],"categories":[]}});
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&value.to_string()).unwrap())
            .unwrap();
    let wall = doc
        .add(entity(
            json!({"kind":"wall","id":0,"layerId":"a","attrs":{},"width":0.2,"justify":"center",
        "pts":[origin,{"x":origin.x+10.0,"y":origin.y}]}),
        ))
        .unwrap();
    let opening = doc.add(opening(&doc, wall, 4.0, 1.0)).unwrap();
    (doc, wall, opening)
}
fn opening(doc: &Document, wall: Slot, at: f64, width: f64) -> Entity {
    entity(
        json!({"kind":"opening","id":0,"layerId":"a","attrs":{},"host":doc.uid(wall).unwrap().to_string(),
        "at":at,"width":width,"place":[]}),
    )
}
fn recipe(doc: &Document, wall: Slot, origin: Vec2) -> DimensionSet {
    DimensionSet {
        id: SET,
        wall: EntityId(doc.uid(wall).unwrap().into_bytes()),
        edge: None,
        face: DimensionSetFace::Left,
        rows: vec![DimensionSetRow::Openings, DimensionSetRow::Overall],
        openings: DimensionSetOpenings::Ends,
        layer_id: "a".into(),
        at: Vec2 {
            x: origin.x,
            y: origin.y + 1.1,
        },
        gap_mm: 6.0,
        height: 0.125,
        look: Default::default(),
        row_places: Vec::new(),
        members: Vec::new(),
        suppressed: Vec::new(),
        broken: None,
        span: None,
    }
}
fn dim(doc: &Document, uid: EntityId) -> &kentos_domain::contracts::DimensionEntity {
    match doc
        .get(doc.slot_of(Uuid::from_bytes(uid.0)).unwrap())
        .unwrap()
    {
        Entity::Dimension(d) => d,
        _ => panic!("dimension"),
    }
}
fn lengths(doc: &Document) -> Vec<f64> {
    doc.dimension_set(SET)
        .unwrap()
        .members
        .iter()
        .map(|m| {
            let d = dim(doc, m.uid);
            (d.b.x - d.a.x).hypot(d.b.y - d.a.y)
        })
        .collect()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn checked_update(doc: &mut Document, slot: Slot, after: Entity) -> Result<(), String> {
    doc.run_step(
        |d| {
            d.update(slot, after);
            Ok(())
        },
        |d, r, _| {
            if let Some(f) = d.faults(r).first() {
                return Err(f.message.clone());
            }
            Ok(None)
        },
    )
    .map(|_| ())
}

#[test]
fn dynamic_opening_members_keep_source_pair_ids_and_one_step_undo() {
    for origin in [
        Vec2 { x: 0., y: 0. },
        Vec2 {
            x: 500000.,
            y: 4400000.,
        },
    ] {
        let (mut doc, wall, window) = fixture(origin);
        doc.add_dimension_set(recipe(&doc, wall, origin)).unwrap();
        assert_eq!(lengths(&doc), vec![3.5, 1., 5.5, 10.]);
        let initial = doc.dimension_set(SET).unwrap().clone();
        for m in &initial.members {
            close(dim(&doc, m.uid).a.y, origin.y + 0.1);
        }
        close(
            doc.dimension_set_row_offset(&initial, DimensionSetRow::Overall)
                .unwrap(),
            1.3,
        );
        let mut edited = doc.get(window).unwrap().clone();
        if let Entity::Opening(o) = &mut edited {
            o.opening.width = 2.;
            o.opening.at = 6.;
        }
        checked_update(&mut doc, window, edited).unwrap();
        assert_eq!(lengths(&doc), vec![5., 2., 3., 10.]);
        assert_eq!(doc.dimension_set(SET).unwrap().members, initial.members);
        let before_new = doc.to_snapshot_v2();
        let new_window = doc.add(opening(&doc, wall, 8.5, 1.)).unwrap();
        assert_eq!(lengths(&doc), vec![5., 2., 1., 1., 1., 10.]);
        let with_new = doc.dimension_set(SET).unwrap().clone();
        assert_eq!(with_new.members[0].uid, initial.members[0].uid);
        assert_eq!(with_new.members[1].uid, initial.members[1].uid);
        assert_eq!(
            with_new.members.last().unwrap().uid,
            initial.members.last().unwrap().uid
        );
        assert!(
            doc.slot_of(Uuid::from_bytes(initial.members[2].uid.0))
                .is_none()
        );
        doc.undo();
        assert_eq!(
            doc.to_snapshot_v2(),
            before_new,
            "one undo removes source and all derived membership changes"
        );
        doc.redo();
        assert_eq!(doc.dimension_set(SET).unwrap(), &with_new);
        doc.remove(&[new_window]);
        assert_eq!(lengths(&doc), vec![5., 2., 3., 10.]);
        assert_ne!(
            doc.dimension_set(SET).unwrap().members[2].uid,
            initial.members[2].uid,
            "a removed pair is newly allocated when it returns"
        );
    }
}

#[test]
fn manual_row_place_and_member_appearance_survive_source_edits() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, window) = fixture(origin);
    doc.add_dimension_set(recipe(&doc, wall, origin)).unwrap();
    let mut set = doc.dimension_set(SET).unwrap().clone();
    doc.set_dimension_set_row_offset(&mut set, DimensionSetRow::Overall, 2.)
        .unwrap();
    doc.update_dimension_set(set).unwrap();
    let member = doc.dimension_set(SET).unwrap().members[1].uid;
    let slot = doc.slot_of(Uuid::from_bytes(member.0)).unwrap();
    let mut appearance = doc.get(slot).unwrap().clone();
    if let Entity::Dimension(d) = &mut appearance {
        d.text = Some("Kapı <>".into());
        d.height = 0.22;
        d.mask = true;
        d.text_shift = Some(TextShift {
            along: 0.25,
            across: 0.15,
        });
    }
    checked_update(&mut doc, slot, appearance.clone()).unwrap();
    let mut source = doc.get(window).unwrap().clone();
    if let Entity::Opening(o) = &mut source {
        o.opening.width = 2.;
    }
    checked_update(&mut doc, window, source).unwrap();
    let d = dim(&doc, member);
    assert_eq!(d.text.as_deref(), Some("Kapı <>"));
    close(d.height, 0.22);
    assert_eq!(
        d.text_shift,
        Some(TextShift {
            along: 0.25,
            across: 0.15
        })
    );
    assert!(d.mask);
    let set = doc.dimension_set(SET).unwrap();
    close(
        doc.dimension_set_row_offset(set, DimensionSetRow::Overall)
            .unwrap(),
        2.,
    );
    close(
        dim(&doc, set.members.last().unwrap().uid).a.y
            + dim(&doc, set.members.last().unwrap().uid).offset,
        2.1,
    );
    // Explicit group style edits intentionally replace only appearance, preserving annotation placement.
    let mut set = set.clone();
    set.height = 0.3;
    doc.update_dimension_set(set).unwrap();
    close(dim(&doc, member).height, 0.3);
    assert_eq!(
        dim(&doc, member).text_shift,
        Some(TextShift {
            along: 0.25,
            across: 0.15
        })
    );
}

#[test]
fn delete_suppression_restore_all_delete_and_detach_are_undoable() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, window) = fixture(origin);
    doc.add_dimension_set(recipe(&doc, wall, origin)).unwrap();
    let old = doc.dimension_set(SET).unwrap().clone();
    let victim = old.members[1].uid;
    doc.remove(&[doc.slot_of(Uuid::from_bytes(victim.0)).unwrap()]);
    assert_eq!(
        doc.dimension_set(SET).unwrap().suppressed,
        vec![old.members[1].segment()]
    );
    let mut source = doc.get(window).unwrap().clone();
    if let Entity::Opening(o) = &mut source {
        o.opening.width = 2.;
    }
    doc.update(window, source);
    assert_eq!(doc.dimension_set(SET).unwrap().members.len(), 3);
    let mut restore = doc.dimension_set(SET).unwrap().clone();
    restore.suppressed.clear();
    doc.update_dimension_set(restore).unwrap();
    assert_eq!(doc.dimension_set(SET).unwrap().members.len(), 4);
    assert_ne!(doc.dimension_set(SET).unwrap().members[1].uid, victim);
    doc.undo();
    assert_eq!(doc.dimension_set(SET).unwrap().members.len(), 3);
    let before_all = doc.to_snapshot_v2();
    let slots: Vec<_> = doc
        .dimension_set(SET)
        .unwrap()
        .members
        .iter()
        .map(|m| doc.slot_of(Uuid::from_bytes(m.uid.0)).unwrap())
        .collect();
    doc.remove(&slots);
    assert!(
        doc.dimension_set(SET).is_none(),
        "all visible members deleted removes the recipe"
    );
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before_all);
    let len = doc.len();
    doc.remove_dimension_set(SET).unwrap();
    assert_eq!(doc.len(), len);
    assert!(
        doc.dimension_set_of_item(Uuid::from_bytes(old.members[0].uid.0))
            .is_none()
    );
    doc.undo();
    assert!(doc.dimension_set(SET).is_some());
}

#[test]
fn owned_geometry_is_refused_atomically_but_broken_hosts_keep_last_geometry() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, _) = fixture(origin);
    doc.add_dimension_set(recipe(&doc, wall, origin)).unwrap();
    let member = doc.dimension_set(SET).unwrap().members[0].uid;
    let slot = doc.slot_of(Uuid::from_bytes(member.0)).unwrap();
    let snapshot = doc.to_snapshot_v2();
    for change in 0..3 {
        let mut bad = doc.get(slot).unwrap().clone();
        if let Entity::Dimension(d) = &mut bad {
            match change {
                0 => d.a.x += 1.,
                1 => d.offset += 1.,
                _ => d.refs = None,
            }
        }
        assert!(
            checked_update(&mut doc, slot, bad)
                .unwrap_err()
                .contains("otomatik gruba")
        );
        assert_eq!(doc.to_snapshot_v2(), snapshot);
    }
    let cached = dim(&doc, member).clone();
    doc.remove(&[wall]);
    assert_eq!(
        doc.dimension_set(SET).unwrap().broken,
        Some(DimensionSetBreak::HostGone)
    );
    let d = dim(&doc, member);
    assert_eq!(d.a, cached.a);
    assert_eq!(d.b, cached.b);
    assert_eq!(d.offset, cached.offset);
    assert!(
        d.refs
            .as_ref()
            .unwrap()
            .a
            .as_ref()
            .unwrap()
            .broken
            .is_some()
    );
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), snapshot);
    doc.set_object_policy(ObjectPolicy::AttributesOnly);
    let set = doc.dimension_set(SET).unwrap().clone();
    assert!(doc.update_dimension_set(set).is_err());
    assert!(doc.remove_dimension_set(SET).is_err());
}

#[test]
fn preview_add_remove_and_source_motion_leave_scratch_and_allocation_unchanged() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, window) = fixture(origin);
    doc.add_dimension_set(recipe(&doc, wall, origin)).unwrap();
    let snapshot = doc.to_snapshot_v2();
    let next = doc.next_slot();
    let revision = doc.revision();
    let mut scratch = None;
    let delta = doc.preview_followers_with(&mut scratch, &[], &[opening(&doc, wall, 8.5, 1.)], &[]);
    assert_eq!(delta.added.len(), 3);
    assert_eq!(delta.removed.len(), 1);
    assert_eq!(doc.to_snapshot_v2(), snapshot);
    assert_eq!(doc.next_slot(), next);
    assert_eq!(doc.revision(), revision);
    let scratch = scratch.as_ref().unwrap();
    assert_eq!(scratch.to_snapshot_v2(), snapshot);
    assert_eq!(scratch.next_slot(), next);
    assert!(!scratch.can_undo());
    // Without the window the openings row is the whole face: drawn once, where the first row is; the overall row
    // would repeat it (docs/adr/0695's 27 October addendum).
    let delta = doc.preview_followers(&[], &[], &[window]);
    assert_eq!(delta.added.len(), 1);
    assert_eq!(delta.removed.len(), 4);
    let mut moved = doc.get(window).unwrap().clone();
    if let Entity::Opening(o) = &mut moved {
        o.opening.at = 5.;
    }
    let delta = doc.followers_delta(&[(window, moved)]);
    assert_eq!(delta.added.len(), 0);
    assert_eq!(delta.removed.len(), 0);
    assert_eq!(
        delta
            .updated
            .iter()
            .filter(|(_, e)| matches!(e, Entity::Dimension(_)))
            .count(),
        3
    );
    assert_eq!(doc.to_snapshot_v2(), snapshot);
}

#[test]
fn right_face_centres_and_removed_rows_use_physical_sources() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, _) = fixture(origin);
    let mut set = recipe(&doc, wall, origin);
    set.face = DimensionSetFace::Right;
    set.openings = DimensionSetOpenings::Centers;
    doc.add_dimension_set(set).unwrap();
    let set = doc.dimension_set(SET).unwrap();
    close(dim(&doc, set.members[0].uid).a.y, -0.1);
    close(dim(&doc, set.members[0].uid).b.y, 0.);
    close(dim(&doc, set.members[1].uid).a.y, 0.);
    close(dim(&doc, set.members[1].uid).b.y, -0.1);
    close(
        dim(&doc, set.members[0].uid).a.y + dim(&doc, set.members[0].uid).offset,
        1.1,
    );
    close(
        dim(&doc, set.members[1].uid).a.y + dim(&doc, set.members[1].uid).offset,
        1.1,
    );
    let removed = set.members.last().unwrap().uid;
    let mut one = set.clone();
    one.rows = vec![DimensionSetRow::Openings];
    doc.update_dimension_set(one).unwrap();
    assert_eq!(doc.dimension_set(SET).unwrap().members.len(), 2);
    assert!(doc.slot_of(Uuid::from_bytes(removed.0)).is_none());
}

#[test]
fn new_members_resolve_current_style_id_but_existing_manual_appearance_stays() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, _) = fixture(origin);
    let style: kentos_domain::contracts::DimensionStyleDef = serde_json::from_value(
        json!({"id":"style-a","name":"Mimari","height":2.5,"dimLineColor":"#ff0000"}),
    )
    .unwrap();
    let mut settings = doc.settings().clone();
    settings.dimension_styles.push(style.clone());
    doc.set_settings(settings);
    let mut set = recipe(&doc, wall, origin);
    set.look = style.look();
    set.height = style.height_at(50.);
    doc.add_dimension_set(set).unwrap();
    let old = doc.dimension_set(SET).unwrap().members[0].uid;
    let slot = doc.slot_of(Uuid::from_bytes(old.0)).unwrap();
    let mut manual = doc.get(slot).unwrap().clone();
    if let Entity::Dimension(d) = &mut manual {
        d.height = 0.22;
        d.look.dim_line_color = Some("#00ff00".into());
    }
    checked_update(&mut doc, slot, manual).unwrap();
    let mut settings = doc.settings().clone();
    settings.dimension_styles[0].height = 5.;
    settings.dimension_styles[0].dim_line_color = Some("#0000ff".into());
    doc.set_settings(settings);
    let old_members: std::collections::HashSet<_> = doc
        .dimension_set(SET)
        .unwrap()
        .members
        .iter()
        .map(|m| m.uid)
        .collect();
    doc.add(opening(&doc, wall, 8.5, 1.)).unwrap();
    close(dim(&doc, old).height, 0.22);
    assert_eq!(
        dim(&doc, old).look.dim_line_color.as_deref(),
        Some("#00ff00")
    );
    let new_members: Vec<_> = doc
        .dimension_set(SET)
        .unwrap()
        .members
        .iter()
        .filter(|m| !old_members.contains(&m.uid))
        .collect();
    assert_eq!(new_members.len(), 3);
    for member in new_members {
        let d = dim(&doc, member.uid);
        close(d.height, 0.25);
        assert_eq!(d.look.dim_style.as_deref(), Some("style-a"));
        assert_eq!(d.look.dim_line_color.as_deref(), Some("#0000ff"));
    }
}

#[test]
fn a_group_edit_cannot_authorize_or_hide_a_generic_member_edit_in_the_same_transaction() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, _) = fixture(origin);
    doc.add_dimension_set(recipe(&doc, wall, origin)).unwrap();
    let uid = doc.dimension_set(SET).unwrap().members[0].uid;
    let slot = doc.slot_of(Uuid::from_bytes(uid.0)).unwrap();
    let before = doc.to_snapshot_v2();
    let next_slot = doc.next_slot();
    for order in [false, true] {
        let result = doc.run_step(
            |doc| -> Result<(), String> {
                let group_edit = |doc: &mut Document| -> Result<(), String> {
                    let mut set = doc.dimension_set(SET).unwrap().clone();
                    set.gap_mm = 8.;
                    doc.update_dimension_set(set).map_err(|e| e.to_string())?;
                    Ok(())
                };
                if order {
                    group_edit(doc)?;
                }
                let mut bad = doc.get(slot).unwrap().clone();
                if let Entity::Dimension(d) = &mut bad {
                    d.offset += 9.;
                }
                doc.update(slot, bad);
                if !order {
                    group_edit(doc)?;
                }
                Ok(())
            },
            |doc, report, _| {
                let faults = doc.faults(report);
                if let Some(f) = faults
                    .iter()
                    .find(|f| f.code == Some("dimension_set_owned"))
                {
                    Err(f.message.clone())
                } else {
                    Ok(None)
                }
            },
        );
        assert!(
            result.is_err(),
            "metadata elsewhere in the transaction must not authorize a generic write"
        );
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.next_slot(), next_slot);
    }
    // Explicit detach is the supported choice and makes the same individual edit legal in one step.
    doc.run_step(
        |doc| -> Result<(), String> {
            doc.remove_dimension_set(SET).map_err(|e| e.to_string())?;
            let mut independent = doc.get(slot).unwrap().clone();
            if let Entity::Dimension(d) = &mut independent {
                d.offset += 9.;
            }
            doc.update(slot, independent);
            Ok(())
        },
        |doc, report, _| {
            if let Some(f) = doc.faults(report).first() {
                Err(f.message.clone())
            } else {
                Ok(None)
            }
        },
    )
    .unwrap();
    assert!(doc.dimension_set(SET).is_none());
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn scale_gap_refresh_is_a_derived_cache_and_history_uses_the_current_scale() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, window) = fixture(origin);
    doc.add_dimension_set(recipe(&doc, wall, origin)).unwrap();
    let mut manual = recipe(&doc, wall, origin);
    manual.id = DimensionSetId([82; 16]);
    doc.set_dimension_set_row_offset(&mut manual, DimensionSetRow::Overall, 2.)
        .unwrap();
    let manual_id = manual.id;
    doc.add_dimension_set(manual).unwrap();
    let members: Vec<_> = doc
        .dimension_sets()
        .iter()
        .flat_map(|s| s.members.iter().map(|m| m.uid))
        .collect();
    for uid in &members {
        let slot = doc.slot_of(Uuid::from_bytes(uid.0)).unwrap();
        let mut e = doc.get(slot).unwrap().clone();
        if let Entity::Dimension(d) = &mut e {
            d.height = 0.222;
            d.look.ext_offset = Some(1.75);
            d.text_shift = Some(TextShift {
                along: 0.2,
                across: 0.3,
            });
        }
        checked_update(&mut doc, slot, e).unwrap();
    }
    let automatic = doc.dimension_set(SET).unwrap().members.last().unwrap().uid;
    let fixed = doc
        .dimension_set(manual_id)
        .unwrap()
        .members
        .last()
        .unwrap()
        .uid;
    let mut thick = doc.get(wall).unwrap().clone();
    if let Entity::Wall(w) = &mut thick {
        w.wall.width = 0.4;
    }
    checked_update(&mut doc, wall, thick).unwrap();
    close(dim(&doc, automatic).offset, 1.2);
    close(dim(&doc, fixed).offset, 1.9);
    let next_slot = doc.next_slot();
    let scale_mark = doc.change_mark();
    let mut settings = doc.settings().clone();
    settings.plot_scale = 100.;
    doc.set_settings(settings);
    close(dim(&doc, automatic).offset, 1.5);
    close(dim(&doc, fixed).offset, 1.9);
    assert!(
        matches!(doc.changes_since(scale_mark),kentos_domain::Changes::Slots(slots)
        if slots.contains(&doc.slot_of(Uuid::from_bytes(automatic.0)).unwrap())),
        "cloud observes the cache update through the normal object journal"
    );
    assert_eq!(doc.next_slot(), next_slot);
    assert_eq!(
        doc.dimension_sets()
            .iter()
            .flat_map(|s| s.members.iter().map(|m| m.uid))
            .collect::<Vec<_>>(),
        members
    );
    for uid in &members {
        let d = dim(&doc, *uid);
        close(d.height, 0.222);
        assert_eq!(d.look.ext_offset, Some(1.75));
        assert_eq!(
            d.text_shift,
            Some(TextShift {
                along: 0.2,
                across: 0.3
            })
        );
    }
    doc.undo(); // settings did not add an object step: undo the earlier wall-thickness edit.
    let Entity::Wall(w) = doc.get(wall).unwrap() else {
        panic!("wall")
    };
    close(w.wall.width, 0.2);
    close(doc.settings().plot_scale, 100.);
    close(dim(&doc, automatic).offset, 1.6);
    close(dim(&doc, fixed).offset, 2.);
    doc.redo();
    let Entity::Wall(w) = doc.get(wall).unwrap() else {
        panic!("wall")
    };
    close(w.wall.width, 0.4);
    close(dim(&doc, automatic).offset, 1.5);
    close(dim(&doc, fixed).offset, 1.9);
    let before = doc.to_snapshot_v2();
    let mut moved = doc.get(window).unwrap().clone();
    if let Entity::Opening(o) = &mut moved {
        o.opening.at = 5.;
    }
    let mut scratch = None;
    let delta = doc.followers_delta_with(&mut scratch, &[(window, moved)]);
    assert!(!delta.updated.is_empty());
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(scratch.unwrap().to_snapshot_v2(), before);
    assert_eq!(doc.next_slot(), next_slot);
    // Failed settings changes restore the derived rows with the old scale, too.
    let refused = doc.run_step(
        |doc| -> Result<(), String> {
            let mut s = doc.settings().clone();
            s.plot_scale = 200.;
            doc.set_settings(s);
            Err("refuse".into())
        },
        |_, _, _| Ok(None),
    );
    assert!(refused.is_err());
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn member_delete_and_group_edit_in_one_transaction_refuse_in_both_orders() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, _) = fixture(origin);
    doc.add_dimension_set(recipe(&doc, wall, origin)).unwrap();
    let victim = doc.dimension_set(SET).unwrap().members[1].uid;
    let slot = doc.slot_of(Uuid::from_bytes(victim.0)).unwrap();
    let before = doc.to_snapshot_v2();
    let next = doc.next_slot();
    for set_first in [false, true] {
        let result = doc.run_step(
            |doc| -> Result<(), String> {
                let set = |doc: &mut Document| -> Result<(), String> {
                    let mut set = doc.dimension_set(SET).unwrap().clone();
                    set.gap_mm = 8.;
                    doc.update_dimension_set(set).map_err(|e| e.to_string())?;
                    Ok(())
                };
                if set_first {
                    set(doc)?;
                }
                doc.remove(&[slot]);
                if !set_first {
                    set(doc)?;
                }
                Ok(())
            },
            |doc, report, _| {
                if let Some(f) = doc.faults(report).first() {
                    Err(f.message.clone())
                } else {
                    Ok(None)
                }
            },
        );
        assert!(result.unwrap_err().contains("ayrı adımlarda"));
        assert_eq!(
            doc.to_snapshot_v2(),
            before,
            "refuse without silently recreating the deleted pair"
        );
        assert_eq!(
            doc.next_slot(),
            next,
            "refusal happens before allocating a replacement identity"
        );
    }
    // The normal individual actions retain their existing suppression and explicit restore behavior.
    doc.remove(&[slot]);
    assert!(doc.slot_of(Uuid::from_bytes(victim.0)).is_none());
    let mut set = doc.dimension_set(SET).unwrap().clone();
    set.gap_mm = 8.;
    doc.update_dimension_set(set).unwrap();
    assert_eq!(doc.dimension_set(SET).unwrap().members.len(), 3);
    assert_eq!(doc.dimension_set(SET).unwrap().suppressed.len(), 1);
    let mut set = doc.dimension_set(SET).unwrap().clone();
    set.suppressed.clear();
    doc.update_dimension_set(set).unwrap();
    assert_eq!(doc.dimension_set(SET).unwrap().members.len(), 4);
    assert_ne!(doc.dimension_set(SET).unwrap().members[1].uid, victim);
}

/// docs/adr/0700: a closed room's measured edge goes with its two ends: a vertex inserted before it renumbers it,
/// one of its ends removed breaks the set (its last geometry kept) instead of measuring another edge.
#[test]
fn an_edge_follows_its_two_ends_and_breaks_when_one_goes() {
    let origin = Vec2 { x: 0., y: 0. };
    let (mut doc, wall, _) = fixture(origin);
    let corners = |pts: &[(f64, f64)]| pts.iter().map(|&(x, y)| Vec2 { x, y }).collect::<Vec<_>>();
    let mut room = doc.get(wall).unwrap().clone();
    if let Entity::Wall(w) = &mut room {
        w.wall.pts = corners(&[(0., 0.), (10., 0.), (10., 6.), (0., 6.)]);
        w.wall.closed = true;
    }
    checked_update(&mut doc, wall, room).unwrap();
    let mut set = recipe(&doc, wall, origin);
    set.edge = Some(2);
    set.face = DimensionSetFace::Right;
    set.at = Vec2 { x: 5., y: 7. };
    doc.add_dimension_set(set).unwrap();
    // The top edge's outer face, mitred at both corners; the bottom edge's window is not on it: one dimension, the
    // overall row not repeating the openings row's whole length.
    let measured = lengths(&doc);
    assert_eq!(measured.len(), 1);
    for length in measured {
        close(length, 10.2);
    }
    let mut more = doc.get(wall).unwrap().clone();
    if let Entity::Wall(w) = &mut more {
        w.wall.pts.insert(1, Vec2 { x: 5., y: 0. });
    }
    checked_update(&mut doc, wall, more).unwrap();
    let followed = doc.dimension_set(SET).unwrap();
    assert_eq!(followed.edge, Some(3));
    assert_eq!(followed.broken, None);
    for length in lengths(&doc) {
        close(length, 10.2);
    }
    let mut fewer = doc.get(wall).unwrap().clone();
    if let Entity::Wall(w) = &mut fewer {
        w.wall.pts.remove(3); // (10, 6), the top edge's start
    }
    checked_update(&mut doc, wall, fewer).unwrap();
    let broken = doc.dimension_set(SET).unwrap();
    assert_eq!(broken.broken, Some(DimensionSetBreak::UnsupportedWall));
    assert_eq!(broken.members.len(), 1, "the last geometry stays for repair");
    doc.undo().unwrap();
    let back = doc.dimension_set(SET).unwrap();
    assert_eq!((back.edge, back.broken), (Some(3), None));
}

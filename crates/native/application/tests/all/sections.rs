//! Kesit çıkar (docs/adr/8042) on the section building (fixtures/arch/v1/section-building.kcad, written by
//! scripts/fixtures/section_building.py without KentOS code): B-B through the stair drawn into its block with the
//! layers EK-4 names, its words (title, levels, rooms) and its stair cut along its run; stale once the entrance door
//! changes, current again after Yeniden çiz; refusals.

use kentos_commands::{
    BuildingSectionState, BuildingSectionsEdit, BuildingSectionsOperation, BuildingSectionsQuery,
    CommandResult, DocumentSnapshotV2, Entity,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{ExecutionContext, sections, sections_edit, sections_query};

fn building() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

fn mark(doc: &Document, letter: &str) -> Uuid {
    doc.entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.mark == letter => Some(u),
            _ => None,
        })
        .unwrap()
}

fn completed<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn failed<T: std::fmt::Debug>(r: CommandResult<T>) -> String {
    match r {
        CommandResult::Failed { error } => error.code,
        other => panic!("{other:?}"),
    }
}

fn derive(doc: &mut Document, m: Uuid) -> kentos_commands::BuildingSectionsEdited {
    completed(sections_edit::execute(
        &mut ExecutionContext::new(doc),
        BuildingSectionsEdit {
            operation: BuildingSectionsOperation::Derive {
                mark: m.to_string(),
                at: Some(kentos_commands::Vec2 { x: 20.0, y: 0.0 }),
            },
            expected_revision: None,
        },
    ))
}

fn texts(doc: &Document, m: Uuid) -> Vec<String> {
    doc.block(sections::block_id(m))
        .unwrap()
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

fn layers(doc: &Document, m: Uuid) -> std::collections::BTreeSet<String> {
    doc.block(sections::block_id(m))
        .unwrap()
        .entities
        .iter()
        .map(|e| e.base().layer_id.clone())
        .collect()
}

#[test]
fn b_b_through_the_stair_is_drawn_into_its_block() {
    let mut doc = building();
    let b = mark(&doc, "B");
    let out = derive(&mut doc, b);
    assert_eq!(out.name, "B-B kesiti");
    assert_eq!(out.inserts.len(), 1);
    let block = doc.block(sections::block_id(b)).unwrap();
    assert_eq!(block.name, "MM-XXXX-KESITB");
    // Its words: the title and scale, every level, the stair hall on both storeys.
    let t = texts(&doc, b);
    for w in [
        "B-B KESİTİ",
        "Ölçek 1/50",
        "±0.00",
        "+3.00",
        "+6.00",
        "003 Merdiven holü",
        "103 Merdiven holü",
    ] {
        assert!(t.iter().any(|x| x == w), "{w} in {t:?}");
    }
    // Its layers by EK-4: concrete cut and hatched, brick cut and hatched, what is seen by its depth.
    let l = layers(&doc, b);
    for want in [
        "mm-tasi-betonarme",
        "mm-tasi-betonarme-t",
        "mm-duva-tugla",
        "mm-duva-tugla-t",
        "mm-xxxx-olcu",
    ] {
        assert!(l.contains(want), "{want} in {l:?}");
    }
    assert!(l.iter().any(|x| x.ends_with("-1")), "a seen layer in {l:?}");
    // The layers are the drawing's (opened in the step when it had none).
    for id in &l {
        assert!(doc.layers().get(id).is_some(), "{id} is a layer");
    }
    // Its placement on the marks' layer.
    let insert = doc
        .entities()
        .find(|e| matches!(e, Entity::Insert(i) if i.block == sections::block_id(b)))
        .unwrap();
    assert_eq!(insert.base().layer_id, "mm-xxxx-sembol");
}

#[test]
fn the_stair_cut_along_its_run_steps_nine_times_up_to_the_landing() {
    let doc = building();
    let b = mark(&doc, "B");
    let Some(Entity::ViewMark(v)) = doc.slot_of(b).and_then(|s| doc.get(s)) else {
        panic!()
    };
    let frame = sections::frame_of(&v.mark).unwrap();
    let m = sections::model(&doc);
    let s = kentos_geometry_core::geom::section::section(&frame, &m.solids);
    // The first flight's cut: a run of treads rising by 3.00/18 each.
    let flights: Vec<&kentos_geometry_core::geom::section::Cut> = s
        .cut
        .iter()
        .filter(|c| {
            matches!(
                m.solids[c.solid].height,
                kentos_geometry_core::geom::section::Height::Flight(_)
            )
        })
        .collect();
    assert_eq!(flights.len(), 1, "{flights:?}");
    let ring = &flights[0].ring;
    let rise = 3.0 / 18.0;
    for k in 1..=9 {
        let z = rise * k as f64;
        assert!(
            ring.iter().any(|q| (q.y - z).abs() < 1e-9),
            "a tread at {z} in {ring:?}"
        );
    }
}

#[test]
fn the_grids_bubbles_stand_in_one_row_and_no_line_runs_through_the_rooms_words() {
    let mut doc = building();
    let b = mark(&doc, "B");
    derive(&mut doc, b);
    let block = doc.block(sections::block_id(b)).unwrap();
    // B-B crosses grids 1, 2 and 3: their bubbles at one height (each grid once lowered the next one's).
    let ys: Vec<f64> = block
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Circle(c) if c.base.layer_id == "mm-akss-genel" => Some(c.c.y),
            _ => None,
        })
        .collect();
    assert_eq!(ys.len(), 3, "{ys:?}");
    assert!(ys.iter().all(|y| (y - ys[0]).abs() < 1e-9), "{ys:?}");
    // Grid 2 runs through the stair hall's middle: the hall's words beside it, no line through them.
    let grids: Vec<f64> = block
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) if l.base.layer_id == "mm-akss-genel" => Some(l.a.x),
            _ => None,
        })
        .collect();
    assert_eq!(grids.len(), 3, "{grids:?}");
    for words in ["003 Merdiven holü", "103 Merdiven holü"] {
        let t = block
            .entities
            .iter()
            .find_map(|e| match e {
                Entity::Text(t) if t.text == words => Some(t),
                _ => None,
            })
            .unwrap();
        let half = words.chars().count() as f64 * t.height * 0.6 / 2.0;
        assert!(
            grids.iter().all(|x| (x - t.p.x).abs() > half),
            "{words} at {:?}, grids {grids:?}",
            t.p
        );
        assert!(!t.mask, "{words} found a free place");
    }
}

#[test]
fn a_door_changed_makes_the_section_stale_and_yeniden_ciz_brings_it_back() {
    let mut doc = building();
    let b = mark(&doc, "B");
    derive(&mut doc, b);
    let state = |doc: &Document| {
        let rows = sections_query::rows(doc);
        rows.into_iter()
            .find(|r| r.mark.as_deref() == Some(b.to_string().as_str()))
            .unwrap()
            .state
    };
    assert_eq!(state(&doc), BuildingSectionState::Current);
    // The entrance door 10 cm higher.
    let (slot, mut door) = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Opening(o) if o.opening.number.as_deref() == Some("GK01") => {
                Some((doc.slot_of(u).unwrap(), o.clone()))
            }
            _ => None,
        })
        .unwrap();
    door.opening.head = Some(2.2);
    doc.update_many(vec![(Slot(slot.0), Entity::Opening(door))], "Kapı");
    assert_eq!(state(&doc), BuildingSectionState::Stale);
    let input = BuildingSectionsEdit {
        operation: BuildingSectionsOperation::Refresh {
            mark: b.to_string(),
        },
        expected_revision: None,
    };
    let out = completed(sections_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        input.clone(),
    ));
    assert!(out.changed);
    assert_eq!(state(&doc), BuildingSectionState::Current);
    // Drawn again with nothing changed: said, not written.
    assert_eq!(
        failed(sections_edit::execute(
            &mut ExecutionContext::new(&mut doc),
            input
        )),
        "nothing_to_set"
    );
    // The query reads both marks: B drawn, A not.
    let all = completed(sections_query::execute(
        &mut ExecutionContext::new(&mut doc),
        BuildingSectionsQuery::default(),
    ));
    assert_eq!(all.sections.len(), 2);
    assert!(
        all.sections
            .iter()
            .any(|r| r.name == "A-A kesiti" && r.state == BuildingSectionState::NotDrawn)
    );
}

#[test]
fn refusals_name_what_is_wrong() {
    let mut doc = building();
    let b = mark(&doc, "B");
    derive(&mut doc, b);
    let again = |doc: &mut Document, op: BuildingSectionsOperation| {
        failed(sections_edit::execute(
            &mut ExecutionContext::new(doc),
            BuildingSectionsEdit {
                operation: op,
                expected_revision: None,
            },
        ))
    };
    assert_eq!(
        again(
            &mut doc,
            BuildingSectionsOperation::Derive {
                mark: b.to_string(),
                at: None
            }
        ),
        "section_exists"
    );
    let a = mark(&doc, "A");
    assert_eq!(
        again(
            &mut doc,
            BuildingSectionsOperation::Refresh {
                mark: a.to_string()
            }
        ),
        "section_not_found"
    );
    let wall = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Wall(_)).then_some(u))
        .unwrap();
    assert_eq!(
        again(
            &mut doc,
            BuildingSectionsOperation::Derive {
                mark: wall.to_string(),
                at: None
            }
        ),
        "not_a_section_mark"
    );
    assert_eq!(
        again(
            &mut doc,
            BuildingSectionsOperation::Derive {
                mark: "x".into(),
                at: None
            }
        ),
        "invalid_uid"
    );
}

/// With `KENTOS_SECTION_OUT` set, the building with A-A and B-B drawn beside it is written there: the real window's
/// drawing (docs/adr/8042, Doğrulama).
#[test]
fn the_building_with_its_sections_is_written_when_asked() {
    let Ok(out) = std::env::var("KENTOS_SECTION_OUT") else {
        return;
    };
    let mut doc = building();
    for (letter, y) in [("A", 14.0), ("B", 0.0)] {
        let m = mark(&doc, letter);
        completed(sections_edit::execute(
            &mut ExecutionContext::new(&mut doc),
            BuildingSectionsEdit {
                operation: BuildingSectionsOperation::Derive {
                    mark: m.to_string(),
                    at: Some(kentos_commands::Vec2 { x: 19.0, y }),
                },
                expected_revision: None,
            },
        ));
    }
    std::fs::write(out, kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap()).unwrap();
}

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
    // Its placement on a layer of its own, which a set's plan and site maps leave out.
    let insert = doc
        .entities()
        .find(|e| matches!(e, Entity::Insert(i) if i.block == sections::block_id(b)))
        .unwrap();
    assert_eq!(insert.base().layer_id, "mm-xxxx-kesit");
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

/// Ruhsat takımı (docs/adr/8042 §6): B-B drawn and placed gets a sheet of its own after the plans, EK-2's KS and MMC
/// with its place "B-B", at the scale it is drawn for; its mark then points at the sheet's map and writes the sheet's
/// number below its letter. A section stale when the set is made, and drawn for 1/50 in a 1/100 set, is said.
#[test]
fn the_permit_set_gives_the_drawn_section_a_sheet_and_its_mark_the_sheet_s_number() {
    use kentos_native_application::permit_set::{PermitSet, execute};
    let mut doc = building();
    let b = mark(&doc, "B");
    derive(&mut doc, b);
    // The entrance door 10 cm higher: B-B stale.
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
    let (_, made, notes) =
        execute(&mut ExecutionContext::new(&mut doc), &PermitSet::default()).unwrap();
    let titles: Vec<&str> = made.iter().map(|m| m.title.as_str()).collect();
    let at = titles
        .iter()
        .position(|t| *t == "B-B kesiti")
        .unwrap_or_else(|| panic!("{titles:?}"));
    assert!(
        titles[..at].iter().any(|t| t.ends_with("planı")),
        "after the plans: {titles:?}"
    );
    assert!(
        !titles.contains(&"A-A kesiti"),
        "A-A is not drawn: {titles:?}"
    );
    let m = &made[at];
    assert_eq!(m.frame, Some(format!("{}-kesit-1", m.sheet)));
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    let sheet = book.sheets.iter().find(|s| s.id == m.sheet).unwrap();
    let var = |n: &str| {
        sheet
            .variables
            .iter()
            .find(|v| v.name == n)
            .map(|v| v.value.clone())
    };
    let text = |t: &str| Some(kentos_sheet::model::VarValue::Text(t.into()));
    assert_eq!(var("dosya_turu"), text("KS"));
    assert_eq!(var("dosya_tipi"), text("MMC"));
    assert_eq!(var("kat_konum"), text("B-B"));
    // The site plan's and the plans' maps leave the sections placed beside the building out (they printed on the site
    // plan at 1/500).
    let plans: Vec<&kentos_sheet::Sheet> = book
        .sheets
        .iter()
        .filter(|s| {
            s.drawing_info
                .as_ref()
                .is_some_and(|i| i.title.ends_with("planı"))
        })
        .collect();
    assert_eq!(plans.len(), 3, "site plan and two storeys");
    for s in plans {
        let frozen = s.items.iter().any(|i| {
            matches!(&i.kind, kentos_sheet::ItemKind::Map(m)
                if m.layer_overrides.iter().any(|o| o.layer == "mm-xxxx-kesit" && o.frozen))
        });
        assert!(frozen, "{}", s.name);
    }
    // The mark points at the sheet's map and writes the sheet's number below its letter.
    let Some(Entity::ViewMark(v)) = doc.slot_of(b).and_then(|s| doc.get(s)) else {
        panic!()
    };
    let target = v.mark.target.clone().unwrap();
    assert_eq!(target.sheet.as_deref(), Some(m.sheet.as_str()));
    assert_eq!(target.item, m.frame);
    assert_eq!(v.mark.words.bottom, m.number);
    assert_eq!(v.mark.words.top, "B");
    // Said: B-B stale, and drawn for 1/50 in the Ruhsat set's 1/100.
    assert!(
        notes.iter().any(|n| n.starts_with("B-B kesiti eski")),
        "{notes:?}"
    );
    assert!(
        notes
            .iter()
            .any(|n| n.contains("1/50") && n.contains("1/100")),
        "{notes:?}"
    );
}

/// Publication says a stale section a sheet prints (docs/adr/8042 §6): the set's B-B sheet once the door changes, not
/// after Yeniden çiz; a sheet that does not show it says nothing.
#[test]
fn a_sheet_that_prints_a_stale_section_is_said() {
    use kentos_native_application::permit_set::{PermitSet, execute};
    let mut doc = building();
    let b = mark(&doc, "B");
    derive(&mut doc, b);
    let (_, made, _) =
        execute(&mut ExecutionContext::new(&mut doc), &PermitSet::default()).unwrap();
    let section = made
        .iter()
        .find(|m| m.title == "B-B kesiti")
        .unwrap()
        .sheet
        .clone();
    let book = |doc: &Document| {
        kentos_sheet::project::read_project_sheets(doc.sheets().unwrap())
            .unwrap()
            .0
    };
    // Every sheet of the set: the site plan's 1/500 map holds B-B's placement but does not draw its layer.
    let chosen: Vec<String> = made.iter().map(|m| m.sheet.clone()).collect();
    assert!(sections_query::stale_on_sheets(&doc, &book(&doc), &chosen).is_empty());
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
    assert_eq!(
        sections_query::stale_on_sheets(&doc, &book(&doc), &chosen),
        vec![(section, "B-B kesiti".to_owned())]
    );
    completed(sections_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        BuildingSectionsEdit {
            operation: BuildingSectionsOperation::Refresh {
                mark: b.to_string(),
            },
            expected_revision: None,
        },
    ));
    assert!(sections_query::stale_on_sheets(&doc, &book(&doc), &chosen).is_empty());
}

/// Denetim › Mevzuat (PAİY 57/2-a-6, docs/adr/8042 §7): "bir tanesi ortak merdivenden geçmek üzere en az iki adet
/// kesit". None drawn: two asked. B-B moved off the stair (x 2, the west rooms) and drawn: one more asked and none
/// through the common stair. A-A (across the flights) drawn too: nothing said.
#[test]
fn the_regulation_asks_two_sections_one_through_the_common_stair() {
    let regulation = |doc: &mut Document| -> Vec<(String, String, String)> {
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(doc),
            "cad.arch.audit",
            kentos_commands::CallOp::Execute,
            serde_json::json!({"checks": ["regulation"]}),
        )
        .unwrap();
        answer["output"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| {
                f["code"]
                    .as_str()
                    .is_some_and(|c| c.starts_with("paiy_section"))
            })
            .map(|f| {
                let t = |k: &str| f[k].as_str().unwrap_or_default().to_owned();
                (t("code"), t("message"), t("rule"))
            })
            .collect()
    };
    let mut doc = building();
    let found = regulation(&mut doc);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "paiy_sections_missing");
    assert_eq!(
        found[0].1,
        "Mimari projede en az iki kesit gerekir, çizimde 0 kesit çıkarılmış — PAİY 57/2-a-6"
    );
    assert!(found[0].2.starts_with("PAİY 57/2-a-6 · RG "), "{found:?}");
    // B-B along x 2: the bedroom and the living room, not the stair hall.
    let b = mark(&doc, "B");
    let slot = doc.slot_of(b).unwrap();
    let Some(Entity::ViewMark(mut v)) = doc.get(slot).cloned() else {
        panic!()
    };
    for p in &mut v.mark.points {
        p.x = 2.0;
    }
    doc.update_many(vec![(slot, Entity::ViewMark(v))], "İşaret");
    derive(&mut doc, b);
    let codes: Vec<String> = regulation(&mut doc).into_iter().map(|f| f.0).collect();
    assert_eq!(
        codes,
        ["paiy_sections_missing", "paiy_section_stair_missing"]
    );
    let a = mark(&doc, "A");
    completed(sections_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        BuildingSectionsEdit {
            operation: BuildingSectionsOperation::Derive {
                mark: a.to_string(),
                at: Some(kentos_commands::Vec2 { x: 20.0, y: 15.0 }),
            },
            expected_revision: None,
        },
    ));
    assert!(regulation(&mut doc).is_empty());
}

/// Balansmanlı merdiven kesitte (docs/adr/8045 §2): its flights stop at its turn's zone as at a landing and each winder
/// stands between its two risers at its own height. A 1.00 m L of 5 winders either side (2.80 + 2.24 m, 18 risers of
/// 0.17 m) beside the building: two flights and ten winders, the treads after risers 6 to 15; the corner's two hold the
/// inner corner and reach the outer one; no landing.
#[test]
fn a_balanced_stair_s_winders_stand_one_by_one() {
    use kentos_geometry_core::geom::section::Height;
    let mut doc = building();
    let p = |x: f64, y: f64| serde_json::json!({"x": 30.0 + x, "y": y});
    let answer = kentos_native_application::registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.stairs.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "stairs": [{
            "pts": [p(0.0, 0.0), p(2.8, 0.0), p(2.8, 2.24)], "width": 1.0, "risers": 18, "riserHeight": 0.17,
            "tread": 0.28, "winders": 5, "arch": {"level": "zemin"}
        }]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let uid = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Stair(s) if s.stair.winders.is_some()).then_some(u))
        .expect("the balanced stair")
        .to_string();
    let m = sections::model(&doc);
    let mine: Vec<&kentos_geometry_core::geom::section::Solid> = m
        .solids
        .iter()
        .zip(&m.owners)
        .filter(|(_, o)| o.as_deref() == Some(uid.as_str()))
        .map(|(s, _)| s)
        .collect();
    let flights = mine
        .iter()
        .filter(|s| matches!(s.height, Height::Flight(_)))
        .count();
    let winders: Vec<(f64, f64, &Vec<kentos_geometry_core::Vec2>)> = mine
        .iter()
        .filter_map(|s| match s.height {
            Height::Prism { bottom, top } => Some((bottom, top, &s.rings[0])),
            _ => None,
        })
        .collect();
    assert_eq!((flights, winders.len()), (2, 10), "{mine:?}");
    // No slab holds either end: no waist, every winder stands on its floor.
    for (k, (bottom, top, _)) in winders.iter().enumerate() {
        assert!(
            (top - 0.17 * (6 + k) as f64).abs() < 1e-9,
            "winder {k}: {top}"
        );
        assert_eq!(*bottom, 0.0, "winder {k}");
    }
    let has = |ring: &Vec<kentos_geometry_core::Vec2>, x: f64, y: f64| {
        ring.iter()
            .any(|q| (q.x - x).abs() < 1e-9 && (q.y - y).abs() < 1e-9)
    };
    // The two at the corner (after risers 10 and 11): the inner corner and the outer one.
    for (_, top, ring) in &winders {
        if (top - 1.70).abs() < 1e-9 || (top - 1.87).abs() < 1e-9 {
            assert!(
                has(ring, 32.3, 0.5) && has(ring, 33.3, -0.5),
                "{top}: {ring:?}"
            );
        }
    }
}

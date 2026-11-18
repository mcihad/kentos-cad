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
/// with its place "BBX", at the scale it is drawn for; its mark then points at the sheet's map and writes the sheet's
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
    // Three letters or digits (EK-2 Tablo 2.1; docs/adr/8066): its letter twice, filled with X.
    assert_eq!(var("kat_konum"), text("BBX"));
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

/// Korkuluk kesitte ve görünüşte (docs/adr/8045 §3): a balustered railing on 1. kat is its top rail, a solid 5 cm deep
/// from +4.05 to +4.10, and its balusters, lines of a solid without a footprint from its floor (+3.00) up to its top rail
/// every gap and 2 cm (0.12 m): 2.40 m along, 20; they hide nothing behind them. A glass one: its top rail and its glass's
/// foot and ends. A solid panel stands whole. Crossing A-A they are cut there, on MM-KORK-CUBUK and -CAM, light.
#[test]
fn a_railing_stands_on_its_floor_its_balusters_seen_and_its_cut_light() {
    use kentos_geometry_core::geom::section::{Height, Solid};
    let mut doc = building();
    let answer = kentos_native_application::registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.railings.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "railings": [
            {"pts": [{"x": 3.0, "y": 1.2}, {"x": 3.0, "y": 3.6}], "arch": {"level": "k1"}},
            {"pts": [{"x": 8.0, "y": 1.2}, {"x": 8.0, "y": 3.6}], "infill": "glass", "arch": {"level": "k1"}},
            {"pts": [{"x": 9.0, "y": 1.2}, {"x": 9.0, "y": 3.6}], "infill": "solid", "arch": {"level": "k1"}}
        ]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let uids: Vec<String> = answer["output"]["uids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u.as_str().unwrap().to_owned())
        .collect();
    let m = sections::model(&doc);
    let of = |uid: &str| -> Vec<(Solid, String, bool)> {
        m.solids
            .iter()
            .zip(&m.owners)
            .zip(&m.looks)
            .filter(|((_, o), _)| o.as_deref() == Some(uid))
            .map(|((s, _), l)| (s.clone(), l.layer.clone(), l.heavy || l.fill.is_some()))
            .collect()
    };
    let prism = |s: &Solid, b: f64, t: f64| matches!(s.height, Height::Prism { bottom, top } if (bottom - b).abs() < 1e-9 && (top - t).abs() < 1e-9);
    let bars = of(&uids[0]);
    assert_eq!(bars.len(), 2, "its top rail and its balusters");
    assert!(
        prism(&bars[0].0, 4.05, 4.1) && bars[0].0.rings.len() == 1 && bars[0].0.marks.is_empty()
    );
    assert!(bars[1].0.rings.is_empty(), "the balusters hide nothing");
    assert_eq!(bars[1].0.marks.len(), 20);
    assert!(
        bars[1]
            .0
            .marks
            .iter()
            .all(|[a, b]| (a.1 - 3.0).abs() < 1e-9 && (b.1 - 4.05).abs() < 1e-9)
    );
    assert!(
        bars.iter()
            .all(|(_, layer, heavy)| layer == "MM-KORK-CUBUK" && !heavy)
    );
    let glass = of(&uids[1]);
    assert_eq!(
        (glass.len(), glass[1].0.marks.len()),
        (2, 3),
        "its foot and its two ends"
    );
    assert_eq!(glass[0].1, "MM-KORK-CAM");
    let panel = of(&uids[2]);
    assert_eq!(panel.len(), 1);
    assert!(prism(&panel[0].0, 3.0, 4.1) && panel[0].0.marks.len() == 2);
    assert_eq!(panel[0].1, "MM-KORK-DOLU");
    // In the IFC model (docs/adr/8128) the balustered one's body is its top rail and its 20 bars, 2 cm square from its
    // floor to the top rail's underside; the glass one's its top rail alone (its glass is not a body yet), the solid
    // one's its panel.
    let (b, _, _) = kentos_native_application::arch_ifc::model(&doc);
    let body = |uid: &str| {
        let id = Uuid::parse_str(uid).unwrap().as_u128();
        b.elements.iter().find(|e| e.id == Some(id)).map(|e| e.shapes.clone()).expect("written")
    };
    let barred = body(&uids[0]);
    assert_eq!(barred.len(), 21);
    assert!(barred[1..].iter().all(|sh| matches!(sh, kentos_ifc::building::Shape::Prism { rings, bottom, top, .. }
        if (rings[0][1][0] - rings[0][0][0] - 0.02).abs() < 1e-9 && bottom.abs() < 1e-9 && (top - 1.05).abs() < 1e-9)));
    assert_eq!((body(&uids[1]).len(), body(&uids[2]).len()), (1, 1));
    // A-A (y 2.40) cuts them: its block draws them on their layers.
    let a = mark(&doc, "A");
    derive(&mut doc, a);
    let block = doc.block(sections::block_id(a)).expect("A-A's block");
    let on = |layer: &str| {
        block
            .entities
            .iter()
            .filter(|e| e.base().layer_id.eq_ignore_ascii_case(layer))
            .count()
    };
    assert!(
        on("MM-KORK-CUBUK") > 0 && on("MM-KORK-CAM") > 0 && on("MM-KORK-DOLU") > 0,
        "{:?}",
        block
            .entities
            .iter()
            .map(|e| e.base().layer_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
    );
}

/// Rampa kesitte (docs/adr/8070 §1): an L ramp east of the building, 6 m east then 4 m north, 1.50 m wide, rising
/// 0.75 m, with the 1.50 m landing round its corner (from 5.25 to 6.75 along its line). Its sloped length is 10 − 1.5 =
/// 8.5 m, so its grade is 0.75 / 8.5 = 3/34 and the landing stands at 0.75 × 5.25 / 8.5 = 63/136 m: worked out here in
/// fractions, not by KentOS. No slab holds its ends: its runs are 0.15 m slabs (a slab's thickness), upright
/// 0.15·√(1 + (3/34)²). A section along y = 2 looking north cuts its second run across: a band from x = 35.25 to 36.75
/// whose top is 63/136 + 1.25 × 3/34 = 39/68. In the IFC model it is one IfcRamp, a quarter turn ramp of concrete.
#[test]
fn a_ramp_s_runs_slope_its_landing_stands_flat_and_the_model_writes_it() {
    use kentos_geometry_core::geom::section::{self as sec, Frame, Height};
    use kentos_geometry_core::Vec2 as P;
    let mut doc = building();
    let p = |x: f64, y: f64| serde_json::json!({"x": 30.0 + x, "y": y});
    let answer = kentos_native_application::registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.ramps.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "ramps": [{
            "pts": [p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0)], "width": 1.5, "rise": 0.75, "arch": {"level": "zemin"}
        }]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let uid = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Ramp(_)).then_some(u))
        .expect("the ramp")
        .to_string();
    let m = sections::model(&doc);
    assert!(
        !m.left.iter().any(|(_, o)| o.as_deref() == Some(uid.as_str())),
        "the ramp is drawn: {:?}",
        m.left
    );
    let mine: Vec<usize> = (0..m.solids.len())
        .filter(|&i| m.owners[i].as_deref() == Some(uid.as_str()))
        .collect();
    let landing = 63.0 / 136.0;
    let grade: f64 = 3.0 / 34.0;
    let upright = 0.15 * (1.0 + grade * grade).sqrt();
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let mut runs = Vec::new();
    let mut flats = Vec::new();
    for &i in &mine {
        match &m.solids[i].height {
            Height::Sloped(s) => runs.push((i, s.clone())),
            Height::Prism { bottom, top } => flats.push((i, *bottom, *top)),
            other => panic!("{other:?}"),
        }
    }
    assert_eq!((runs.len(), flats.len()), (2, 1), "{mine:?}");
    // The first run: up from 0 at its start to the landing's height where the landing begins.
    let (_, first) = &runs[0];
    assert!(near(first.z(P::new(30.0, 0.0)), 0.0), "{first:?}");
    assert!(near(first.z(P::new(35.25, 0.5)), landing), "{first:?}");
    assert!(near(first.thickness, upright), "{first:?}");
    // The second: from the landing's height up to 0.75 at its end.
    let (_, second) = &runs[1];
    assert!(near(second.z(P::new(36.0, 0.75)), landing), "{second:?}");
    assert!(near(second.z(P::new(36.0, 4.0)), 0.75), "{second:?}");
    // The landing: the band's corner square, flat at its height, a slab thick.
    let (k, bottom, top) = flats[0];
    assert!(near(top, landing) && near(bottom, landing - 0.15), "{bottom} {top}");
    let ring = &m.solids[k].rings[0];
    for (x, y) in [(35.25, -0.75), (36.75, -0.75), (36.75, 0.75), (35.25, 0.75)] {
        assert!(
            ring.iter().any(|q| near(q.x, x) && near(q.y, y)),
            "({x}, {y}) in {ring:?}"
        );
    }
    // Cut across its second run.
    let frame = Frame::new(P::new(34.0, 2.0), P::new(38.0, 2.0), false, None).unwrap();
    let cut = sec::section(&frame, &m.solids);
    let pieces: Vec<&sec::Cut> = cut.cut.iter().filter(|c| mine.contains(&c.solid)).collect();
    assert_eq!(pieces.len(), 1, "{pieces:?}");
    let xs: Vec<f64> = pieces[0].ring.iter().map(|q| q.x).collect();
    let zs: Vec<f64> = pieces[0].ring.iter().map(|q| q.y).collect();
    let span = |v: &[f64]| {
        (
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let ((u0, u1), (z0, z1)) = (span(&xs), span(&zs));
    assert!(near(u0, 1.25) && near(u1, 2.75), "{xs:?}");
    assert!(near(z1, 39.0 / 68.0) && near(z0, 39.0 / 68.0 - upright), "{zs:?}");
    // The IFC model: one IfcRamp of its three bodies, a quarter turn ramp of concrete, not left out.
    let (b, written, _) = kentos_native_application::arch_ifc::model(&doc);
    assert!(!written.left.iter().any(|l| l.id == uid), "{:?}", written.left);
    let ramps: Vec<&kentos_ifc::building::Element> = b
        .elements
        .iter()
        .filter(|e| e.class == kentos_ifc::building::Class::Ramp)
        .collect();
    assert_eq!(ramps.len(), 1);
    assert_eq!(ramps[0].predefined, "QUARTER_TURN_RAMP");
    assert_eq!(ramps[0].object_type, None);
    assert_eq!(ramps[0].material.as_deref(), Some("Betonarme"));
    assert_eq!(ramps[0].shapes.len(), 3);
    assert_eq!(ramps[0].tag.as_deref(), Some(uid.as_str()));
    assert_eq!(written.ramps, 1);
}

/// The ramp, the stair and the IFC element a test builds with railings both sides: the document, the object's id.
fn with_railings(command: &str, list: &str, fields: serde_json::Value) -> (Document, String) {
    let mut doc = building();
    let answer = kentos_native_application::registry::run(
        &mut ExecutionContext::new(&mut doc),
        command,
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", list: [fields]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let uid = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Ramp(r) if r.ramp.railings.is_some() => Some(u),
            Entity::Stair(s) if s.stair.railings.is_some() => Some(u),
            _ => None,
        })
        .expect("the object with railings")
        .to_string();
    (doc, uid)
}

/// Rampanın korkuluğu kesitte ve modelde (docs/adr/8128): the L ramp above (6 m east, 4 m north, 1.50 m wide, rising
/// 0.75 m, its landing from 5.25 to 6.75 along its line at 63/136 m, its grade 3/34) with railings both sides. Each
/// side's line runs 5 cm inside its band, 0.70 m off the centre line, as the plan draws it; its top rail is 5 cm across
/// and down, its top 1.10 m over the ramp's surface across from it (PAİY 38/1; a ramp's guard is measured from its
/// surface): over the runs sloped with them, over the landing flat at 63/136 + 1.10. The left side, inside the turn,
/// crosses the landing's corner in two 5 cm pieces; the right one runs round it: 8 top rails, 4 of them flat. Its
/// balusters every 0.12 m from 6 cm in stand on the surface: the left line 5.30 + 3.30 m long (72), the right 6.70 +
/// 4.70 m (95). A section along y = 2 looking north cuts the second run's two rails, 5 cm squares round x = 35.30 and
/// 36.70, their tops at 39/68 + 1.10. Worked out here in fractions. In the IFC model the ramp keeps its three bodies;
/// its two railings are IfcRailing BALUSTRADE, its parts, each its four pieces and its balusters (2 cm bars).
#[test]
fn a_ramp_s_railings_stand_over_its_surface_and_are_its_parts() {
    use kentos_geometry_core::Vec2 as P;
    use kentos_geometry_core::geom::section::{self as sec, Frame, Height};
    use kentos_native_application::sections::Part;
    let p = |x: f64, y: f64| serde_json::json!({"x": 30.0 + x, "y": y});
    let (doc, uid) = with_railings(
        "cad.ramps.create",
        "ramps",
        serde_json::json!({
            "pts": [p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0)], "width": 1.5, "rise": 0.75, "railings": "both",
            "arch": {"level": "zemin"}
        }),
    );
    let m = sections::model(&doc);
    let mine = |i: usize| m.owners[i].as_deref() == Some(uid.as_str());
    let body = (0..m.solids.len()).filter(|&i| mine(i) && m.parts[i].is_none()).count();
    assert_eq!(body, 3, "its runs and its landing");
    let rails: Vec<usize> = (0..m.solids.len())
        .filter(|&i| mine(i) && matches!(m.parts[i], Some(Part::Rail(_))))
        .collect();
    let (tops, lines): (Vec<usize>, Vec<usize>) = rails.iter().partition(|&&i| !m.solids[i].rings.is_empty());
    let (grade, landing) = (3.0 / 34.0, 63.0 / 136.0);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    // The ramp's surface across from a plan point: its second run north of the landing, its first west of it.
    let surface = |q: P| {
        if q.y > 0.75 + 1e-9 {
            landing + (q.y - 0.75) * grade
        } else if q.x < 35.25 - 1e-9 {
            (q.x - 30.0) * grade
        } else {
            landing
        }
    };
    assert_eq!(tops.len(), 8, "{tops:?}");
    let mut flats = 0;
    for &i in &tops {
        let ring = &m.solids[i].rings[0];
        let c = P::new(ring.iter().map(|q| q.x).sum::<f64>() / 4.0, ring.iter().map(|q| q.y).sum::<f64>() / 4.0);
        // Its line 0.70 m off the centre line: y = ±0.70 along the first leg, x = 36 ± 0.70 along the second.
        assert!(near(c.y.abs(), 0.70) || near((c.x - 36.0).abs(), 0.70), "{c:?}");
        match &m.solids[i].height {
            Height::Sloped(s) => {
                assert!(near(s.z(c), surface(c) + 1.10) && near(s.thickness, 0.05), "{c:?} {s:?}");
            }
            Height::Prism { bottom, top } => {
                flats += 1;
                assert!(near(*top, landing + 1.10) && near(*bottom, landing + 1.05), "{c:?} {top}");
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(flats, 4);
    let left = rails.iter().filter(|&&i| m.parts[i] == Some(Part::Rail(0)) && !m.solids[i].rings.is_empty()).count();
    assert_eq!(left, 4);
    // Its balusters: each from the surface under it up to the top rail's underside.
    let mut count = 0;
    for &i in &lines {
        for [(q, foot), (q2, head)] in &m.solids[i].marks {
            assert!(q == q2 && near(*foot, surface(*q)) && near(*head, surface(*q) + 1.05), "{q:?} {foot} {head}");
            count += 1;
        }
    }
    assert_eq!(count, 72 + 95);
    // Cut across its second run: its two top rails.
    let frame = Frame::new(P::new(34.0, 2.0), P::new(38.0, 2.0), false, None).unwrap();
    let cut = sec::section(&frame, &m.solids);
    let mut pieces: Vec<(f64, f64, f64, f64)> = cut
        .cut
        .iter()
        .filter(|c| tops.contains(&c.solid))
        .map(|c| {
            let us = c.ring.iter().map(|q| q.x);
            let zs = c.ring.iter().map(|q| q.y);
            (
                us.clone().fold(f64::INFINITY, f64::min),
                us.fold(f64::NEG_INFINITY, f64::max),
                zs.clone().fold(f64::INFINITY, f64::min),
                zs.fold(f64::NEG_INFINITY, f64::max),
            )
        })
        .collect();
    pieces.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    let top = 39.0 / 68.0 + 1.10;
    for ((u0, u1, z0, z1), at) in pieces.iter().zip([1.30, 2.70]) {
        assert!(near(*u0, at - 0.025) && near(*u1, at + 0.025), "{pieces:?}");
        assert!(near(*z1, top) && near(*z0, top - 0.05), "{pieces:?}");
    }
    // The IFC model: the ramp's body as it was, its railings its parts.
    let (b, _, _) = kentos_native_application::arch_ifc::model(&doc);
    let ramp = b
        .elements
        .iter()
        .position(|e| e.class == kentos_ifc::building::Class::Ramp)
        .expect("the ramp");
    assert_eq!(b.elements[ramp].shapes.len(), 3);
    let railings: Vec<&kentos_ifc::building::Element> =
        b.elements.iter().filter(|e| e.part_of == Some(ramp)).collect();
    assert_eq!(railings.len(), 2, "{railings:?}");
    for (r, bars) in railings.iter().zip([72, 95]) {
        assert_eq!(r.class, kentos_ifc::building::Class::Railing);
        assert_eq!(r.predefined, "BALUSTRADE");
        assert_eq!(r.shapes.len(), 4 + bars);
        // Each bar 2 cm square, from the surface to the top rail's underside (the ramp's storey's floor is ±0.00).
        let square = |sh: &&kentos_ifc::building::Shape| match sh {
            kentos_ifc::building::Shape::Prism { rings, bottom, top, .. } => {
                let xs: Vec<f64> = rings[0].iter().map(|q| q[0]).collect();
                let w = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) - xs.iter().copied().fold(f64::INFINITY, f64::min);
                near(w, 0.02) && near(top - bottom, 1.05)
            }
            _ => false,
        };
        assert_eq!(r.shapes.iter().filter(square).count(), bars);
        assert_eq!(r.object, b.elements[ramp].id);
        assert_eq!(r.storey, b.elements[ramp].storey);
    }
    assert_ne!(railings[0].id, railings[1].id);
}

/// Merdivenin korkuluğu kesitte ve modelde (docs/adr/8128): a straight stair east of the building, 18 risers of 0.17 m
/// and treads of 0.28 m from ±0.00 along y = 6 (its line 4.76 m), 1.20 m wide, railings both sides. Each side's top rail
/// runs along y = 6 ± 0.55, sloped with the line through its nosings: its top 0.17·(i + 1) + 1.10 over riser i at
/// x = 30 + 0.28·i, from 1.27 at its foot to 4.16 at its head (a stair's guard is measured from its nosings' line,
/// PAİY 38/1's 1.10 m). Its 40 balusters a side stand on its treads: the first at x = 30.06 on the first (0.17), the
/// last at 34.74 on the 17th (2.89), each up to the top rail's underside. In the IFC model the stair has its two
/// railings as its parts, each its top rail and its 40 bars.
#[test]
fn a_stair_s_railings_follow_its_nosings_and_are_its_parts() {
    use kentos_geometry_core::Vec2 as P;
    use kentos_geometry_core::geom::section::Height;
    use kentos_native_application::sections::Part;
    let p = |x: f64| serde_json::json!({"x": x, "y": 6.0});
    let (doc, uid) = with_railings(
        "cad.stairs.create",
        "stairs",
        serde_json::json!({
            "pts": [p(30.0), p(34.76)], "width": 1.2, "risers": 18, "riserHeight": 0.17, "tread": 0.28,
            "railings": "both", "arch": {"level": "zemin"}
        }),
    );
    let m = sections::model(&doc);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let rails: Vec<usize> = (0..m.solids.len())
        .filter(|&i| m.owners[i].as_deref() == Some(uid.as_str()) && matches!(m.parts[i], Some(Part::Rail(_))))
        .collect();
    let (tops, lines): (Vec<usize>, Vec<usize>) = rails.iter().partition(|&&i| !m.solids[i].rings.is_empty());
    assert_eq!((tops.len(), lines.len()), (2, 2), "{rails:?}");
    for (&i, y) in tops.iter().zip([6.55, 5.45]) {
        let Height::Sloped(s) = &m.solids[i].height else {
            panic!("{:?}", m.solids[i].height)
        };
        assert!(m.solids[i].rings[0].iter().all(|q| (q.y - y).abs() <= 0.025 + 1e-9), "{:?}", m.solids[i].rings);
        for k in 0..18 {
            let q = P::new(30.0 + 0.28 * k as f64, y);
            assert!(near(s.z(q), 0.17 * (k + 1) as f64 + 1.10), "riser {k}: {s:?}");
        }
        assert!(near(s.z(P::new(30.0, y)), 1.27) && near(s.z(P::new(34.76, y)), 4.16));
    }
    for &i in &lines {
        let marks = &m.solids[i].marks;
        assert_eq!(marks.len(), 40);
        let ([(a, foot), (_, head)], [(z, last), _]) = (marks[0], marks[39]);
        assert!(near(a.x, 30.06) && near(foot, 0.17) && near(head, 0.17 + 0.06 * 0.17 / 0.28 + 1.05), "{:?}", marks[0]);
        assert!(near(z.x, 34.74) && near(last, 2.89), "{:?}", marks[39]);
    }
    let (b, _, _) = kentos_native_application::arch_ifc::model(&doc);
    let stair = b
        .elements
        .iter()
        .position(|e| e.class == kentos_ifc::building::Class::Stair && e.tag.as_deref() == Some(uid.as_str()))
        .expect("the stair");
    let railings: Vec<&kentos_ifc::building::Element> =
        b.elements.iter().filter(|e| e.part_of == Some(stair)).collect();
    assert_eq!(railings.len(), 2, "{railings:?}");
    assert!(railings.iter().all(|r| r.class == kentos_ifc::building::Class::Railing && r.shapes.len() == 1 + 40));
}

/// Giydirme cephe (docs/adr/8070 §7): a 6 m curtain wall east of the building, 0.15 m deep, 3 m high, at most 1.50 m
/// between mullions and 1.20 m between transoms: 6 / 1.5 = 4 equal bays (5 mullions), 3 / 1.2 = 2.5 → 3 equal rows of
/// 1.00 m (4 transom lines); worked out here, not by KentOS. Mullions 5 cm wide, inside the ends: x 20–20.05,
/// 21.475–21.525 … 25.95–26; a bay's 4 transoms and 3 panels between the mullions' faces, the panels 24 mm thick down
/// the middle. A cut across its first bay meets its 4 transoms and 3 panels. In the IFC model: one IfcCurtainWall, its
/// 12 panels its own body, and 21 IfcMember MULLION (5 + 4 × 4), its parts; no door goes into it.
#[test]
fn a_curtain_wall_is_its_mullions_transoms_and_panels() {
    use kentos_geometry_core::Vec2 as P;
    use kentos_geometry_core::geom::section::{self as sec, Frame, Height};
    let mut doc = building();
    let answer = kentos_native_application::registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.walls.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "walls": [{
            "pts": [{"x": 20.0, "y": 0.0}, {"x": 26.0, "y": 0.0}], "width": 0.15, "height": 3.0,
            "arch": {"level": "zemin"},
            "curtain": {"spacing": 1.5, "rows": 1.2, "panel": "Cam", "frames": "Alüminyum"}
        }]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let uid = answer["output"]["uids"][0].as_str().unwrap().to_owned();
    let m = sections::model(&doc);
    let mine: Vec<usize> = (0..m.solids.len()).filter(|&i| m.owners[i].as_deref() == Some(uid.as_str())).collect();
    use kentos_native_application::sections::Part;
    let count = |f: &dyn Fn(Part) -> bool| mine.iter().filter(|&&i| m.parts[i].is_some_and(f)).count();
    assert_eq!(count(&|p| matches!(p, Part::Mullion(_))), 5);
    assert_eq!(count(&|p| matches!(p, Part::Transom(_))), 16);
    assert_eq!(count(&|p| p == Part::Panel), 12);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let span = |i: usize| {
        let r = &m.solids[i].rings[0];
        let xs: Vec<f64> = r.iter().map(|q| q.x).collect();
        let ys: Vec<f64> = r.iter().map(|q| q.y).collect();
        let lo = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let (b, t) = match m.solids[i].height {
            Height::Prism { bottom, top } => (bottom, top),
            _ => panic!("a prism"),
        };
        ((lo(&xs), hi(&xs)), (lo(&ys), hi(&ys)), (b, t))
    };
    let mullions: Vec<(f64, f64)> = mine
        .iter()
        .filter(|&&i| matches!(m.parts[i], Some(Part::Mullion(_))))
        .map(|&i| span(i).0)
        .collect();
    let want = [(20.0, 20.05), (21.475, 21.525), (22.975, 23.025), (24.475, 24.525), (25.95, 26.0)];
    for (got, want) in mullions.iter().zip(want) {
        assert!(near(got.0, want.0) && near(got.1, want.1), "{got:?} {want:?}");
    }
    let first_panel = mine.iter().find(|&&i| m.parts[i] == Some(Part::Panel)).copied().unwrap();
    let ((x0, x1), (y0, y1), (z0, z1)) = span(first_panel);
    assert!(near(x0, 20.05) && near(x1, 21.475), "{x0} {x1}");
    assert!(near(y0, -0.012) && near(y1, 0.012), "{y0} {y1}");
    assert!(near(z0, 0.05) && near(z1, 0.975), "{z0} {z1}");
    // A cut across its first bay, looking east.
    let frame = Frame::new(P::new(20.75, -1.0), P::new(20.75, 1.0), true, None).unwrap();
    let cut = sec::section(&frame, &m.solids);
    assert_eq!(cut.cut.iter().filter(|c| mine.contains(&c.solid)).count(), 7);
    // The IFC model.
    let (b, written, _) = kentos_native_application::arch_ifc::model(&doc);
    assert!(!written.left.iter().any(|l| l.id == uid), "{:?}", written.left);
    let id = kentos_domain::Uuid::parse_str(&uid).unwrap().as_u128();
    let (k, wall) = b
        .elements
        .iter()
        .enumerate()
        .find(|(_, e)| e.id == Some(id))
        .expect("the curtain wall");
    assert_eq!(wall.class, kentos_ifc::building::Class::CurtainWall);
    assert_eq!((wall.predefined, wall.object_type.as_deref()), ("USERDEFINED", Some("Giydirme cephe")));
    assert_eq!(wall.shapes.len(), 12);
    assert_eq!(wall.material.as_deref(), Some("Cam"));
    let parts: Vec<&kentos_ifc::building::Element> = b.elements.iter().filter(|e| e.part_of == Some(k)).collect();
    assert_eq!(parts.len(), 21);
    assert!(parts.iter().all(|e| e.class == kentos_ifc::building::Class::Member
        && e.predefined == "MULLION"
        && e.material.as_deref() == Some("Alüminyum")
        && e.object == Some(id)));
    let external = |e: &kentos_ifc::building::Element, set: &str| {
        e.psets.iter().find(|p| p.name == set).and_then(|p| p.props.iter().find(|(k, _)| k == "IsExternal")).map(|(_, v)| v.clone())
    };
    assert_eq!(external(wall, "Pset_CurtainWallCommon"), Some(kentos_ifc::building::Value::Bool(true)));
    assert_eq!(external(parts[0], "Pset_MemberCommon"), Some(kentos_ifc::building::Value::Bool(true)));
    assert_eq!((written.curtain_walls, written.members), (1, 21));
    let text = kentos_ifc::building::write(&b);
    assert_eq!(text.matches("IFCCURTAINWALL(").count(), 1);
    assert_eq!(text.matches("IFCMEMBER(").count(), 21);
    // `KENTOS_WRITE_IFC_CURTAIN=<path>` keeps the file for an outside reader (IfcOpenShell's validator).
    if let Some(path) = std::env::var_os("KENTOS_WRITE_IFC_CURTAIN") {
        std::fs::write(path, &text).expect("the file kept");
    }
}

/// Giydirme cephede kapı ve pencere (docs/adr/8129): the curtain wall above (6 m from x 20, 1.50 / 1.20 m, 3 m high:
/// its bays' cells 0.05–1.475, 1.525–2.975, 3.025–4.475, 4.525–5.95 along it, its rows' 0.05–0.975, 1.025–1.975,
/// 2.025–2.95 up) with a 0.90 m door round 2.25 along it (its type's head 2.10) and a 2 m window round 4.50 from 0.90 to
/// 2.10; worked by hand. The door fills the second bay from the floor up to the row its head is in (2.00–3.00):
/// 1.525–2.975 (at 2.25, 1.45 wide), its head 2.95, never lower than asked. The window the cells whose middles it
/// covers, the third and fourth bays' middle row: 3.025–5.95 (at 4.4875, 2.925 wide), 1.025–1.975. In place of their
/// 3 + 2 panels 7 remain; the door's three transoms (its foot one and the two across it) go, none of the window's (its
/// sill and head are its cells' frames): 13; the mullion at 4.50 inside the window stands under and over it: 6 pieces.
/// A niche goes into none. In the IFC model the door and the window are the curtain wall's parts (IfcDoor, IfcWindow)
/// beside its 19 members, no opening through it.
#[test]
fn a_curtain_wall_s_door_and_window_fill_its_cells() {
    use kentos_native_application::sections::Part;
    let mut doc = building();
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| {
        kentos_native_application::registry::run(&mut ExecutionContext::new(doc), name, kentos_commands::CallOp::Execute, input)
            .unwrap()
    };
    let answer = run(
        &mut doc,
        "cad.walls.create",
        serde_json::json!({"layerId": "0", "walls": [{
            "pts": [{"x": 20.0, "y": 0.0}, {"x": 26.0, "y": 0.0}], "width": 0.15, "height": 3.0,
            "arch": {"level": "zemin"},
            "curtain": {"spacing": 1.5, "rows": 1.2, "panel": "Cam", "frames": "Alüminyum"}
        }]}),
    );
    let uid = answer["output"]["uids"][0].as_str().unwrap().to_owned();
    let answer = run(
        &mut doc,
        "cad.openings.create",
        serde_json::json!({"layerId": "0", "openings": [
            {"host": uid, "at": 2.25, "width": 0.9},
            {"host": uid, "form": "window", "at": 4.5, "width": 2.0, "sill": 0.9, "head": 2.1}
        ]}),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let fields = |doc: &Document, k: usize| {
        let u = answer["output"]["uids"][k].as_str().unwrap();
        let slot = doc.slot_of(Uuid::parse_str(u).unwrap()).unwrap();
        let Some(Entity::Opening(o)) = doc.get(slot) else { panic!("an opening") };
        (u.to_owned(), o.opening.clone())
    };
    let (door_uid, door) = fields(&doc, 0);
    let (window_uid, window) = fields(&doc, 1);
    assert!(near(door.at, 2.25) && near(door.width, 1.45), "{door:?}");
    assert!(door.sill.unwrap_or(0.0) == 0.0 && near(door.head.unwrap(), 2.95), "{door:?}");
    assert!(near(window.at, 4.4875) && near(window.width, 2.925), "{window:?}");
    assert!(near(window.sill.unwrap(), 1.025) && near(window.head.unwrap(), 1.975), "{window:?}");
    assert!(door.broken.is_none() && window.broken.is_none());
    // Its jambs at the mullions' faces.
    assert!(near(door.place[0].x, 21.525) && near(door.place[2].x, 22.975), "{:?}", door.place);
    let m = sections::model(&doc);
    let mine: Vec<usize> = (0..m.solids.len()).filter(|&i| m.owners[i].as_deref() == Some(uid.as_str())).collect();
    let count = |f: &dyn Fn(Part) -> bool| mine.iter().filter(|&&i| m.parts[i].is_some_and(f)).count();
    assert_eq!(count(&|p| p == Part::Panel), 7);
    assert_eq!(count(&|p| matches!(p, Part::Transom(_))), 13);
    assert_eq!(count(&|p| matches!(p, Part::Mullion(_))), 6);
    // The mullion at 24.50 under and over the window.
    let at_450: Vec<(f64, f64)> = mine
        .iter()
        .filter(|&&i| matches!(m.parts[i], Some(Part::Mullion(_))))
        .filter(|&&i| m.solids[i].rings[0].iter().any(|q| near(q.x, 24.475)))
        .map(|&i| match m.solids[i].height {
            kentos_geometry_core::geom::section::Height::Prism { bottom, top } => (bottom, top),
            _ => panic!("a prism"),
        })
        .collect();
    assert_eq!(at_450, vec![(0.0, 1.025), (1.975, 3.0)]);
    // The door's lines on the faces, on its layer; its gap and the window's.
    assert!((0..m.solids.len()).any(|i| m.owners[i].as_deref() == Some(door_uid.as_str()) && m.looks[i].layer == "MM-KAPI-GENEL"));
    let gaps: Vec<&sections::Gap> = m.gaps.iter().filter(|g| [&door_uid, &window_uid].contains(&g.owner.as_ref().unwrap())).collect();
    assert_eq!(gaps.len(), 2);
    // A niche goes into none.
    let refused = run(
        &mut doc,
        "cad.openings.create",
        serde_json::json!({"layerId": "0", "openings": [{"host": uid, "form": "niche", "at": 0.75, "width": 0.6}]}),
    );
    assert_eq!(refused["error"]["code"], "not_a_wall", "{refused}");
    // The IFC model: its door and window are its parts, no opening goes through it.
    let (b, written, _) = kentos_native_application::arch_ifc::model(&doc);
    let id = Uuid::parse_str(&uid).unwrap().as_u128();
    let k = b.elements.iter().position(|e| e.id == Some(id)).expect("the curtain wall");
    let parts: Vec<&kentos_ifc::building::Element> = b.elements.iter().filter(|e| e.part_of == Some(k)).collect();
    use kentos_ifc::building::Class;
    assert_eq!(parts.iter().filter(|e| e.class == Class::Member).count(), 19);
    let door_el = parts.iter().find(|e| e.class == Class::Door).expect("its door");
    let window_el = parts.iter().find(|e| e.class == Class::Window).expect("its window");
    assert_eq!(door_el.overall.map(|(h, w)| (near(h, 2.95), near(w, 1.45))), Some((true, true)));
    assert!(window_el.overall.is_some_and(|(h, w)| near(h, 0.95) && near(w, 2.925)));
    assert!(!b.openings.iter().any(|o| o.host == k), "no opening through a curtain wall");
    assert_eq!(b.elements[k].shapes.len(), 7);
    assert!(written.doors >= 1 && written.windows >= 1);
    // Its grid at 1.00 m (six bays: cells 0.05–0.975, 1.025–1.975, 2.025–2.975 …): the door's cells again, those whose
    // middles its 1.525–2.975 covers, the third bay alone (2.025–2.975); the window's 3.025–5.95 covers three.
    let edited = run(
        &mut doc,
        "cad.walls.edit",
        serde_json::json!({"uids": [uid], "operation": {"kind": "set", "curtain": {"spacing": 1.0, "rows": 1.2, "panel": "Cam", "frames": "Alüminyum"}}}),
    );
    assert_eq!(edited["status"], "completed", "{edited}");
    let (_, door) = fields(&doc, 0);
    let (_, window) = fields(&doc, 1);
    assert!(near(door.at, 2.5) && near(door.width, 0.95) && near(door.head.unwrap(), 2.95), "{door:?}");
    assert!(near(window.at, 4.4875) && near(window.width, 2.925), "{window:?}");
}

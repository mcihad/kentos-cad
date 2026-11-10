//! Görünüş çıkar (docs/adr/8043) on the facade building (fixtures/arch/v1/facade-building.kcad, written by
//! scripts/fixtures/facade_building.py without KentOS code): the section building with four elevation marks 3.5 m
//! outside its walls. The north facade drawn from its mark, its doors and windows on their own layers with Türkiye's
//! opening lines, what is under the graded ground dashed; the four named by where they face, and by the north arrow;
//! stale once a window moves; its sheet in the Ruhsat takımı after the sections.

use kentos_commands::{
    BuildingSectionState, BuildingSectionsEdit, BuildingSectionsEdited, BuildingSectionsOperation,
    CallOp, CommandResult, DocumentSnapshotV2, Entity, ViewMarkForm,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::{
    ExecutionContext, registry, sections, sections_edit, sections_query,
};

fn facades() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/facade-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

fn mark(doc: &Document, form: ViewMarkForm, words: &str) -> Uuid {
    doc.entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.form == form && v.mark.mark == words => Some(u),
            _ => None,
        })
        .unwrap()
}

fn derive(doc: &mut Document, m: Uuid, at: (f64, f64)) -> BuildingSectionsEdited {
    match sections_edit::execute(
        &mut ExecutionContext::new(doc),
        BuildingSectionsEdit {
            operation: BuildingSectionsOperation::Derive {
                mark: m.to_string(),
                at: Some(kentos_commands::Vec2 { x: at.0, y: at.1 }),
            },
            expected_revision: None,
        },
    ) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn fields(doc: &Document, m: Uuid) -> kentos_commands::ViewMarkFields {
    match doc.slot_of(m).and_then(|s| doc.get(s)) {
        Some(Entity::ViewMark(v)) => v.mark.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_north_facade_is_drawn_from_its_mark_with_its_windows_opening_lines() {
    let mut doc = facades();
    let north = mark(&doc, ViewMarkForm::Elevation, "1");
    let out = derive(&mut doc, north, (20.0, 20.0));
    assert_eq!(out.name, "Kuzey görünüşü");
    let block = doc.block(sections::block_id(north)).unwrap();
    assert_eq!(block.name, "MM-XXXX-GORUNUSKUZEY");
    // One undo step of its own name; its placement on the elevations' layer (EK-4's general element), not the sections'.
    assert_eq!(
        doc.undo_labels().first().map(String::as_str),
        Some("Görünüş çıkar")
    );
    let placed = out
        .inserts
        .first()
        .and_then(|u| Uuid::parse_str(u).ok())
        .unwrap();
    let layer = doc
        .slot_of(placed)
        .and_then(|s| doc.get(s))
        .map(|e| e.base().layer_id.clone());
    assert_eq!(layer.as_deref(), Some("mm-xxxx-gorunus"));
    assert_eq!(
        doc.layers().get("mm-xxxx-gorunus").map(|l| l.name.as_str()),
        Some("MM-XXXX-GORUNUS")
    );
    let texts: Vec<&str> = block
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    for w in ["KUZEY GÖRÜNÜŞÜ", "Ölçek 1/50", "±0.00", "+3.00", "+6.00"] {
        assert!(texts.contains(&w), "{w} in {texts:?}");
    }
    // An elevation names no room and cuts nothing: no hatch.
    assert!(
        !texts.iter().any(|t| t.contains("Merdiven holü")),
        "{texts:?}"
    );
    assert!(!block.entities.iter().any(|e| matches!(e, Entity::Hatch(_))));
    // The north facade's windows and its entrance door on their own layers, nearest first (EK-4 4.14.3).
    let on = |prefix: &str| -> Vec<&kentos_commands::LineEntity> {
        block
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Line(l) if l.base.layer_id.starts_with(prefix) => Some(l),
                _ => None,
            })
            .collect()
    };
    // The facade is the nearest thing seen: its windows and door in the first depth slice (EK-4 4.14.3), not the
    // fourth; nothing of it in the others (it hides the rest of the building).
    let windows = on("mm-penc-genel-1");
    let doors = on("mm-kapi-genel-1");
    assert!(
        !windows.is_empty() && !doors.is_empty(),
        "windows {} doors {}",
        windows.len(),
        doors.len()
    );
    // Türkiye's opening lines: from the hinge side's corners to the middle of the other jamb, slanting.
    let slanted = |ls: &[&kentos_commands::LineEntity]| {
        ls.iter()
            .filter(|l| (l.a.x - l.b.x).abs() > 1e-6 && (l.a.y - l.b.y).abs() > 1e-6)
            .count()
    };
    assert!(slanted(&windows) >= 2, "{}", slanted(&windows));
    assert!(slanted(&doors) >= 2, "{}", slanted(&doors));
    // The beams (25 cm, flush with the 25 cm wall's outer plaster) and the slabs meet the plastered face without a
    // line (docs/adr/8043 §7): nothing runs along the facade at the beams' undersides (+2.50, +5.50) or the slab's
    // underside (+2.85); the 30/30 columns, 2.5 cm proud of it, are drawn.
    let level_lines = |z: f64| {
        block
            .entities
            .iter()
            .filter(|e| matches!(e, Entity::Line(l)
                if (l.a.y - z).abs() < 1e-6 && (l.b.y - z).abs() < 1e-6 && (l.a.x - l.b.x).abs() > 1.0))
            .count()
    };
    for z in [2.5, 2.85, 5.5] {
        assert_eq!(level_lines(z), 0, "a joint drawn at {z}");
    }
    let uprights = block
        .entities
        .iter()
        .filter(|e| {
            matches!(e, Entity::Line(l)
            if (l.a.x - l.b.x).abs() < 1e-6 && (l.a.y - l.b.y).abs() > 2.0
                && l.base.layer_id.starts_with("mm-tasi-betonarme"))
        })
        .count();
    assert!(uprights >= 8, "the four columns' edges: {uprights}");
    let deeper: Vec<&str> = block
        .entities
        .iter()
        .map(|e| e.base().layer_id.as_str())
        .filter(|l| l.ends_with("-2") || l.ends_with("-3") || l.ends_with("-4"))
        .collect();
    assert!(deeper.is_empty(), "{deeper:?}");
    // What is seen under the graded ground (the ground slab's edge) is dashed.
    assert!(block.entities.iter().any(|e| matches!(e, Entity::Line(l)
        if serde_json::to_value(l.base.look.line_type).unwrap() == serde_json::json!("dashed") && l.a.y < 0.0)));
}

#[test]
fn the_four_facades_are_named_by_where_they_face_and_by_the_north_arrow() {
    let mut doc = facades();
    let names = |doc: &Document| -> Vec<String> {
        ["1", "2", "3", "4"]
            .iter()
            .map(|n| sections::view_name(doc, &fields(doc, mark(doc, ViewMarkForm::Elevation, n))))
            .collect()
    };
    assert_eq!(
        names(&doc),
        [
            "Kuzey görünüşü",
            "Güney görünüşü",
            "Doğu görünüşü",
            "Batı görünüşü"
        ]
    );
    let one = mark(&doc, ViewMarkForm::Elevation, "1");
    derive(&mut doc, one, (20.0, 20.0));
    // A north arrow turned a right angle counter-clockwise: north is the drawing's left (−x).
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.north.create",
        CallOp::Execute,
        serde_json::json!({"at": {"x": 20.0, "y": 20.0}, "layerId": "isaret",
                           "north": {"kind": "angle", "angle": std::f64::consts::FRAC_PI_2}}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(
        names(&doc),
        [
            "Doğu görünüşü",
            "Batı görünüşü",
            "Güney görünüşü",
            "Kuzey görünüşü"
        ]
    );
    // The drawn one's title no longer says where it faces: stale; Yeniden çiz renames its block and its title.
    let row = sections_query::rows(&doc)
        .into_iter()
        .find(|r| r.mark.as_deref() == Some(one.to_string().as_str()))
        .unwrap();
    assert_eq!(
        (row.name.as_str(), row.state),
        ("Doğu görünüşü", BuildingSectionState::Stale)
    );
    let out = match sections_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        BuildingSectionsEdit {
            operation: BuildingSectionsOperation::Refresh {
                mark: one.to_string(),
            },
            expected_revision: None,
        },
    ) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    };
    assert_eq!(out.name, "Doğu görünüşü");
    assert_eq!(
        doc.undo_labels().first().map(String::as_str),
        Some("Görünüşü yeniden çiz")
    );
    let block = doc.block(sections::block_id(one)).unwrap();
    assert_eq!(block.name, "MM-XXXX-GORUNUSDOGU");
    assert!(
        block
            .entities
            .iter()
            .any(|e| matches!(e, Entity::Text(t) if t.text == "DOĞU GÖRÜNÜŞÜ"))
    );
}

#[test]
fn a_window_moved_makes_the_facade_stale() {
    let mut doc = facades();
    let north = mark(&doc, ViewMarkForm::Elevation, "1");
    derive(&mut doc, north, (20.0, 20.0));
    let state = |doc: &Document| {
        sections_query::rows(doc)
            .into_iter()
            .find(|r| r.mark.as_deref() == Some(north.to_string().as_str()))
            .map(|r| (r.name, r.state))
            .unwrap()
    };
    assert_eq!(
        state(&doc),
        ("Kuzey görünüşü".into(), BuildingSectionState::Current)
    );
    // The 1. kat's middle window on the north wall (P15) 30 cm higher.
    let (slot, mut window) = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Opening(o) if o.opening.number.as_deref() == Some("P15") => {
                Some((doc.slot_of(u).unwrap(), o.clone()))
            }
            _ => None,
        })
        .unwrap();
    window.opening.sill = Some(1.2);
    doc.update_many(vec![(Slot(slot.0), Entity::Opening(window))], "Pencere");
    assert_eq!(
        state(&doc),
        ("Kuzey görünüşü".into(), BuildingSectionState::Stale)
    );
}

#[test]
fn the_permit_set_gives_the_facade_its_gr_sheet_after_the_section() {
    use kentos_native_application::permit_set::{PermitSet, execute};
    let mut doc = facades();
    let (b, north) = (
        mark(&doc, ViewMarkForm::Section, "B"),
        mark(&doc, ViewMarkForm::Elevation, "1"),
    );
    derive(&mut doc, b, (20.0, 0.0));
    derive(&mut doc, north, (20.0, 20.0));
    // The north facade's middle window (P15) 30 cm higher: the set says the facade it prints is stale, as a facade.
    let (slot, mut window) = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Opening(o) if o.opening.number.as_deref() == Some("P15") => {
                Some((doc.slot_of(u).unwrap(), o.clone()))
            }
            _ => None,
        })
        .unwrap();
    window.opening.sill = Some(1.2);
    doc.update_many(vec![(Slot(slot.0), Entity::Opening(window))], "Pencere");
    let (_, made, notes) =
        execute(&mut ExecutionContext::new(&mut doc), &PermitSet::default()).unwrap();
    assert!(
        notes.iter().any(|n| n.starts_with("Kuzey görünüşü eski:")
            && n.contains("paftası eski görünüşü gösterir. Görünüşü seçip")),
        "{notes:?}"
    );
    let titles: Vec<&str> = made.iter().map(|m| m.title.as_str()).collect();
    let section = titles.iter().position(|t| *t == "B-B kesiti").unwrap();
    let facade = titles.iter().position(|t| *t == "Kuzey görünüşü").unwrap();
    assert_eq!(facade, section + 1, "{titles:?}");
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    let sheet = book
        .sheets
        .iter()
        .find(|s| s.id == made[facade].sheet)
        .unwrap();
    let var = |n: &str| {
        sheet
            .variables
            .iter()
            .find(|v| v.name == n)
            .map(|v| v.value.clone())
    };
    let text = |t: &str| Some(kentos_sheet::model::VarValue::Text(t.into()));
    assert_eq!(var("dosya_turu"), text("GR"));
    assert_eq!(var("kat_konum"), text("KUZ"));
    // The site plan's and the plans' maps leave the elevation placed beside the building out, as the sections.
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
                if m.layer_overrides.iter().any(|o| o.layer == "mm-xxxx-gorunus" && o.frozen))
        });
        assert!(frozen, "{}", s.name);
    }
    // The elevation mark writes its sheet's number below its number.
    let f = fields(&doc, north);
    assert_eq!(f.words.bottom, made[facade].number);
}

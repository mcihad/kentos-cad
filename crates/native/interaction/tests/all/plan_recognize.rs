//! Planı tanı (docs/adr/8048 §4): the office's ground floor plan of the section building
//! (scripts/fixtures/arch_recognize_plan.py) taken in and selected whole; the tool says what it reads, shows each
//! layer's role in its settings, changes one there, and writes the walls and openings in one step.

use crate::common::Bench;
use kentos_commands::{CallOp, Entity};
use kentos_native_application::{ExecutionContext, registry};

/// The plan taken in, every object of it selected, Planı tanı started.
fn office() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/arch/v1/recognize/kesit-binasi-zemin.dxf"
    );
    let answer = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        "file.import.dxf",
        CallOp::Execute,
        serde_json::json!({ "path": path }),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let slots: Vec<kentos_domain::Slot> = b
        .doc
        .entities()
        .map(|e| kentos_domain::Slot(e.base().id))
        .collect();
    b.selection.set(slots);
    b.start("planRecognize");
    b
}

fn role_of<'a>(rows: &'a [kentos_interaction::tool_settings::Setting], layer: &str) -> &'a str {
    rows.iter()
        .find(|r| r.label == layer)
        .map_or("", |r| r.value.as_str())
}

#[test]
fn the_plan_s_layers_say_their_roles_and_enter_writes_what_the_preview_said() {
    let mut b = office();
    let rows = b.session.settings(&Default::default());
    for (layer, role) in [
        ("DUVAR", "Duvar"),
        ("KAPI", "Kapı"),
        ("PENCERE", "Pencere"),
        ("KOLON", "Kolon"),
        ("AKS", "Yok say"),
        ("MERDIVEN", "Yok say"),
        ("YAZI", "Yok say"),
        ("MOBILYA", "Yok say"),
    ] {
        assert_eq!(role_of(&rows, layer), role, "{layer}: {rows:#?}");
    }
    let said = b.session.prompt().text();
    assert!(
        said.contains("8 duvar, 3 kapı, 4 pencere, 0 boşluk; 88 nesne silinir; 39 nesne kalır"),
        "{said}"
    );
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(!preview.areas.is_empty() && preview.strokes.len() > 7);
    let before = b.doc.len();
    b.confirm();
    assert_eq!(b.doc.len(), before - 88 + 8 + 7);
    let walls = b
        .doc
        .entities()
        .filter(|e| matches!(e, Entity::Wall(_)))
        .count();
    let openings = b
        .doc
        .entities()
        .filter(|e| matches!(e, Entity::Opening(_)))
        .count();
    assert_eq!((walls, openings), (8, 7));
    assert_eq!(
        b.last_text(),
        Some(
            "Planı tanı: 8 duvar, 3 kapı, 4 pencere, 0 boşluk eklendi, 88 nesne silindi, 39 nesne çizimde kaldı."
        )
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Planı tanı"));
    assert_eq!(b.doc.len(), before);
}

#[test]
fn a_role_changed_in_the_settings_is_read_and_without_walls_nothing_is_written() {
    let mut b = office();
    let rows = b.session.settings(&Default::default());
    let key = rows.iter().find(|r| r.label == "DUVAR").unwrap().key;
    assert!(b.run(|s, cx| s.configure(key, "ignore", cx)));
    let rows = b.session.settings(&Default::default());
    assert_eq!(role_of(&rows, "DUVAR"), "Yok say");
    let said = b.session.prompt().text();
    assert!(said.contains("Seçimde duvar tanınmadı"), "{said}");
    let before = b.doc.len();
    b.confirm();
    assert_eq!(b.doc.len(), before);
    // Back to a wall's: the plan reads again.
    assert!(b.run(|s, cx| s.configure(key, "Duvar", cx)));
    assert!(b.session.prompt().text().contains("8 duvar"));
}

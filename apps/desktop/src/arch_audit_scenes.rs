//! The pictures of docs/adr/0703: the Denetim tab over the shared cases' flat (`fixtures/commands/v1/cad.arch.audit.json`
//! `setup`): every finding; the end gap's row pressed (its walls selected, the view on them, Düzelt saying its fix);
//! Düzelt written and checked again; Geri al; one group; a clean drawing:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=denetim,denetim-bosluk,denetim-duzeltildi,denetim-geri-alindi,denetim-mahaller,denetim-temiz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! docs/adr/0717: the Katlar group (each finding's Düzelt gives Etkin kat), Kata ver ▾ open, Seç ve göster, every
//! object without a level given Zemin kat in one step:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=denetim-katlar,denetim-kata-ver,denetim-katsizlar,denetim-kat-verildi cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::Value;

use crate::app::{App, Message};
use crate::arch_audit::Event;
use crate::files_testing::make_cad;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.arch.audit.json");

/// The shared cases' drawing, its levels and every object as the cases have them.
fn setup() -> Value {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    file["setup"].clone()
}

pub(crate) fn open_flat(app: &mut App, edit: impl FnOnce(&mut Value)) {
    let mut d = setup();
    edit(&mut d);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(d).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -1.5,
            min_y: -4.0,
            max_x: 26.0,
            max_y: 14.0,
        },
        24.0,
    );
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::ArchAudit(e));
}

/// The row of the first finding of `code`, by its place in the order shown.
pub(crate) fn row_of(app: &App, code: &str) -> usize {
    let shown = app.arch_audit_shown();
    let findings = &app.arch_audit.audited.as_ref().expect("audited").findings;
    shown
        .iter()
        .position(|&i| findings[i].code == code)
        .expect("a finding of that code")
}

/// Mimari › Proje › Mimari denetim: the tab, every group's findings.
fn audited(app: &mut App) {
    open_flat(app, |_| {});
    run(app, "arch.audit");
}

/// The end gap's row pressed: the partition and the wall it stops short of selected, the view on them.
fn gap(app: &mut App) {
    audited(app);
    let row = row_of(app, "arch_wall_end_gap");
    event(app, Event::Press(row));
}

/// Düzelt: the end run on to the other wall in one undo step; the tab checked again.
fn fixed(app: &mut App) {
    gap(app);
    event(app, Event::Fix);
}

/// One Geri al: the gap back, the tab says the drawing changed since its check.
fn undone(app: &mut App) {
    fixed(app);
    run(app, "edit.undo");
}

/// One group: the spaces' findings alone.
fn spaces(app: &mut App) {
    audited(app);
    event(app, Event::Group(Some(kentos_commands::ArchAuditCheck::Spaces)));
    let row = row_of(app, "arch_space_narrow");
    event(app, Event::Press(row));
}

/// A drawing with nothing to say: the room and its partitions alone.
fn clean(app: &mut App) {
    open_flat(app, |d| {
        let keep = |e: &Value| e["kind"] == "wall" && [1, 2, 4].contains(&e["id"].as_u64().unwrap_or(0));
        let ents: Vec<Value> = d["entities"].as_array().expect("entities").clone();
        let uids: Vec<Value> = d["uids"].as_array().expect("uids").clone();
        let (ents, uids): (Vec<Value>, Vec<Value>) =
            ents.into_iter().zip(uids).filter(|(e, _)| keep(e)).unzip();
        d["entities"] = Value::Array(ents);
        d["uids"] = Value::Array(uids);
        d["dimensionSets"] = Value::Array(Vec::new());
    });
    run(app, "arch.audit");
}

/// The Katlar group (docs/adr/0717 §3): the objects without a level, the first one pressed, Düzelt giving Etkin kat.
fn levels(app: &mut App) {
    audited(app);
    event(app, Event::Group(Some(kentos_commands::ArchAuditCheck::Levels)));
    let row = row_of(app, "arch_level_missing");
    event(app, Event::Press(row));
}

/// Kata ver ▾'s Seç ve göster: every object without a level selected, the view on them.
fn levelless(app: &mut App) {
    levels(app);
    event(app, Event::ShowLevelless);
}

/// Kata ver ▾'s Zemin kat: every object without a level on Zemin kat in one step; the tab checked again.
fn given(app: &mut App) {
    levelless(app);
    event(app, Event::AssignLevel("zemin".into()));
}

/// Kata ver ▾ open over the Katlar group.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![("denetim-kata-ver", levels, |s, app| {
        crate::tools_screens::press_caption(s, app, "Kata ver (5)", false, false)
    })]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("denetim", audited),
        ("denetim-bosluk", gap),
        ("denetim-duzeltildi", fixed),
        ("denetim-geri-alindi", undone),
        ("denetim-mahaller", spaces),
        ("denetim-temiz", clean),
        ("denetim-katlar", levels),
        ("denetim-katsizlar", levelless),
        ("denetim-kat-verildi", given),
    ]
}

/// docs/adr/0717 §2: Kata ver ▾ selects what lacks a level, then gives every one the level chosen in one step that
/// Geri al takes back; the Katlar group is empty after it, each finding's Düzelt gave Etkin kat.
#[test]
fn kata_ver_gives_every_object_without_a_level_one() {
    let (mut app, _) = App::boot(None);
    audited(&mut app);
    let lacking = |app: &App| {
        app.arch_audit
            .audited
            .as_ref()
            .expect("audited")
            .findings
            .iter()
            .filter(|f| f.check == kentos_commands::ArchAuditCheck::Levels)
            .map(|f| (f.uids[0].clone(), f.fix.as_ref().map(|x| x.label.clone())))
            .collect::<Vec<_>>()
    };
    let before = lacking(&app);
    assert_eq!(before.len(), 5, "{before:?}");
    // Etkin kat is Zemin kat (the level at 0): each Düzelt gives it.
    assert!(before.iter().all(|(_, fix)| fix.as_deref() == Some("Katını Zemin kat yap")), "{before:?}");
    event(&mut app, Event::ShowLevelless);
    assert_eq!(app.selection.ids().len(), 5, "Seç ve göster selects them all");
    let revision = app.document.as_ref().expect("a drawing").model.revision();
    event(&mut app, Event::AssignLevel("kat-1".into()));
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(doc.revision() > revision);
    for (uid, _) in &before {
        let slot = doc.slot_of(kentos_domain::Uuid::parse_str(uid).expect("a uid")).expect("there");
        assert_eq!(doc.get(slot).and_then(|e| e.own_level()), Some("kat-1"), "{uid}");
    }
    assert!(lacking(&app).is_empty(), "checked again");
    assert_eq!(app.selection.ids().len(), 5, "the objects stay selected");
    run(&mut app, "edit.undo");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let back: Vec<Option<String>> = before
        .iter()
        .map(|(uid, _)| {
            let slot = doc.slot_of(kentos_domain::Uuid::parse_str(uid).expect("a uid")).expect("there");
            doc.get(slot).and_then(|e| e.own_level()).map(str::to_owned)
        })
        .collect();
    assert!(back.iter().all(|l| l.as_deref() != Some("kat-1")), "one step: {back:?}");
}

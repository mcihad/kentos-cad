//! The pictures and the tests of docs/adr/0732: the new checks over the shared case's drawing
//! (`fixtures/commands/v1/cad.arch.audit.json`, “Yeni denetimler …”), rows chosen with Ctrl/⌘ and their fixes previewed
//! on the drawing and written in one step; İstisna yap's note, the exceptions hidden, shown, lapsed; Kapsam; a row's menu:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=denetim-ekler,denetim-toplu-onizleme,denetim-toplu-duzeltildi,denetim-istisna-sor,denetim-istisna-gizli,denetim-istisna-goster,denetim-istisna-dustu,denetim-kapsam,denetim-satir-menusu cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::keyboard::Modifiers;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::arch_audit::Event;
use crate::arch_audit_scenes::{open_flat, row_of};
use crate::files_testing::make_cad;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.arch.audit.json");
/// The note the scenes give their exceptions.
const NOTE: &str = "Cephe derzi; bilerek bırakıldı.";

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::ArchAudit(e));
}

/// A row pressed with `modifiers` held (Ctrl/⌘ adds, Shift takes the range).
fn press(app: &mut App, row: usize, modifiers: Modifiers) {
    app.modifiers = modifiers;
    event(app, Event::Press(row));
    app.modifiers = Modifiers::default();
}

/// The row of the first finding of `code` that has a fix, by its place in the order shown.
fn fixable_row_of(app: &App, code: &str) -> usize {
    let shown = app.arch_audit_shown();
    let findings = &app.arch_audit.audited.as_ref().expect("audited").findings;
    shown
        .iter()
        .position(|&i| findings[i].code == code && findings[i].fix.is_some())
        .expect("a fixable finding of that code")
}

/// The row of the finding of `code` at `at`, by its place in the order shown.
fn row_at(app: &App, code: &str, at: [f64; 2]) -> usize {
    let shown = app.arch_audit_shown();
    let findings = &app.arch_audit.audited.as_ref().expect("audited").findings;
    shown
        .iter()
        .position(|&i| findings[i].code == code && (findings[i].at.x - at[0]).abs() < 1e-9 && (findings[i].at.y - at[1]).abs() < 1e-9)
        .expect("a finding of that code there")
}

/// The new checks' drawing of the shared cases: duplicated and short walls, overlapping separators, an empty tag,
/// stairs and ramps with their own warnings.
fn open_extras(app: &mut App) {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let case = file["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Yeni denetimler")))
        .expect("the new checks' case");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -1.0,
            min_y: -2.0,
            max_x: 16.0,
            max_y: 15.0,
        },
        24.0,
    );
}

fn command(app: &mut App, name: &str, input: Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
}

/// Mimari denetim over the new checks' drawing: every finding.
fn extras(app: &mut App) {
    open_extras(app);
    run(app, "arch.audit");
}

/// Three fixes chosen: the duplicated wall's and separator's deletes, the offset walls' end join (Ctrl/⌘); the drawing
/// shows them, the danger colour on what goes; Düzelt (3).
fn batch(app: &mut App) {
    extras(app);
    let first = fixable_row_of(app, "arch_wall_overlap");
    press(app, first, Modifiers::empty());
    let second = fixable_row_of(app, "arch_separator_overlap");
    press(app, second, Modifiers::COMMAND);
    let third = row_at(app, "wall_join_overlap", [6.0, 3.0]);
    press(app, third, Modifiers::COMMAND);
}

/// Düzelt (3): one step, “Denetim düzeltmeleri”; checked again.
fn batch_fixed(app: &mut App) {
    batch(app);
    event(app, Event::Fix);
}

/// Two findings chosen in the flat, İstisna yap… asked, a note written.
fn asked(app: &mut App) {
    open_flat(app, |_| {});
    run(app, "arch.audit");
    let gap = row_of(app, "arch_wall_ends_apart");
    press(app, gap, Modifiers::empty());
    let small = row_of(app, "arch_space_small");
    press(app, small, Modifiers::COMMAND);
    event(app, Event::Except(None));
    event(app, Event::Note(NOTE.to_owned()));
}

/// The note given: the two hidden, İstisnalar (2).
fn hidden(app: &mut App) {
    asked(app);
    event(app, Event::ExceptConfirm);
}

/// İstisnalar: Göster: the two listed quiet, their note after them.
fn shown(app: &mut App) {
    hidden(app);
    event(app, Event::View(1));
}

/// One excepted wall moved: its finding changed, so its exception lapsed and the finding is open again, in the warning
/// colour.
fn lapsed(app: &mut App) {
    hidden(app);
    let wall = "0192f5a1-7030-7000-8000-000000000005";
    command(
        app,
        "cad.entities.transform",
        json!({ "uids": [wall], "transform": { "kind": "move", "dx": 0.0, "dy": 0.02 } }),
    );
    event(app, Event::Check);
}

/// Kapsam: Seçili nesneler: the walls of the partition and the room's chosen, the rest counted outside.
fn scoped(app: &mut App) {
    open_flat(app, |_| {});
    run(app, "arch.audit");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let walls: Vec<kentos_domain::Slot> = ["0192f5a1-7030-7000-8000-000000000002", "0192f5a1-7030-7000-8000-000000000003"]
        .iter()
        .filter_map(|u| doc.slot_of(kentos_domain::Uuid::parse_str(u).ok()?))
        .collect();
    app.selection.set(walls);
    event(app, Event::Scope(2));
}

/// A row's menu: Seç ve göster, its Düzelt, İstisna yap….
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![("denetim-satir-menusu", |app| {
        open_flat(app, |_| {});
        run(app, "arch.audit");
    }, |s, app| {
        let row = row_of(app, "arch_wall_end_gap");
        let i = app.arch_audit_shown()[row];
        let message = app.arch_audit.audited.as_ref().expect("audited").findings[i].message.clone();
        crate::tools_screens::press_caption(s, app, &message, true, false)
    })]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("denetim-ekler", extras),
        ("denetim-toplu-onizleme", batch),
        ("denetim-toplu-duzeltildi", batch_fixed),
        ("denetim-istisna-sor", asked),
        ("denetim-istisna-gizli", hidden),
        ("denetim-istisna-goster", shown),
        ("denetim-istisna-dustu", lapsed),
        ("denetim-kapsam", scoped),
    ]
}

fn findings(app: &App) -> Vec<kentos_commands::ArchAuditFinding> {
    app.arch_audit.audited.as_ref().expect("audited").findings.clone()
}

fn exceptions(app: &App) -> Vec<kentos_commands::ArchAuditException> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.settings().arch.as_ref().map_or_else(Vec::new, |a| a.audit_exceptions.clone())
}

/// docs/adr/0732 §5: Ctrl/⌘ adds rows and takes one away, Shift takes a range; the chosen fixes are previewed — what
/// goes in the danger colour — and written as one undo step that one Geri al takes back.
#[test]
fn rows_chosen_together_are_previewed_and_fixed_in_one_step() {
    let (mut app, _) = App::boot(None);
    batch(&mut app);
    assert_eq!(app.arch_audit.chosen.len(), 3, "{:?}", app.arch_audit.chosen);
    // Ctrl/⌘ on a chosen row takes it away, again puts it back.
    let third = row_at(&app, "wall_join_overlap", [6.0, 3.0]);
    press(&mut app, third, Modifiers::COMMAND);
    assert_eq!(app.arch_audit.chosen.len(), 2);
    press(&mut app, third, Modifiers::COMMAND);
    assert_eq!(app.arch_audit.chosen.len(), 3);
    let preview = app.arch_audit_preview().expect("a preview while the tab shows");
    let danger = preview.strokes.iter().filter(|s| s.tone == kentos_interaction::Tone::Danger).count();
    let accent = preview.strokes.iter().filter(|s| s.tone == kentos_interaction::Tone::Accent).count();
    assert!(danger > 0, "what goes is in the danger colour");
    assert!(accent > 0, "what changes is outlined");
    let tag = preview.tag.expect("the preview's tag");
    assert!(tag.lines.iter().any(|l| l == "2 nesne silinir"), "{:?}", tag.lines);
    // The drawing is as it was: the preview wrote nothing.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let (revision, objects) = (doc.revision(), doc.entities().count());
    assert!(!app.arch_audit_stale());
    let before = findings(&app).len();
    event(&mut app, Event::Fix);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(doc.revision() > revision);
    assert_eq!(doc.entities().count(), objects - 2, "the duplicated wall and separator are gone");
    assert_eq!(doc.undo_labels().last().map(String::as_str), Some("Denetim düzeltmeleri"));
    assert!(findings(&app).len() < before, "checked again");
    assert!(app.arch_audit.chosen.is_empty());
    run(&mut app, "edit.undo");
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.entities().count(), objects, "one Geri al takes the three back");
    // Shift: the range from the row pressed last.
    event(&mut app, Event::Check);
    press(&mut app, 0, Modifiers::empty());
    press(&mut app, 3, Modifiers::SHIFT);
    let shown = app.arch_audit_shown();
    assert_eq!(app.arch_audit.chosen, shown[0..=3].iter().copied().collect());
}

/// docs/adr/0732 §3: İstisna yap writes the exceptions with their note in one undo step and hides them; Göster lists
/// them; an object changed lapses its exception, the finding open again; İstisnayı kaldır takes it away; Eski
/// istisnaları temizle drops one whose finding is gone.
#[test]
fn an_exception_hides_its_finding_until_the_finding_changes() {
    let (mut app, _) = App::boot(None);
    asked(&mut app);
    assert_eq!(app.arch_audit.note.as_deref(), Some(NOTE));
    let all = findings(&app).len();
    event(&mut app, Event::ExceptConfirm);
    assert!(app.arch_audit.note.is_none());
    let marked = exceptions(&app);
    assert_eq!(marked.len(), 2, "{marked:?}");
    assert!(marked.iter().all(|x| x.note.as_deref() == Some(NOTE)));
    assert_eq!(findings(&app).len(), all - 2, "hidden");
    assert_eq!(app.arch_audit.audited.as_ref().expect("audited").excepted, 2);
    // Göster: listed, marked.
    event(&mut app, Event::View(1));
    assert_eq!(findings(&app).iter().filter(|f| f.exception).count(), 2);
    event(&mut app, Event::View(0));
    // One Geri al takes the exceptions back.
    run(&mut app, "edit.undo");
    assert!(exceptions(&app).is_empty());
    run(&mut app, "edit.redo");
    assert_eq!(exceptions(&app).len(), 2);
    // The excepted wall moved: its exception lapses, the finding is open again.
    let wall = "0192f5a1-7030-7000-8000-000000000005";
    command(
        &mut app,
        "cad.entities.transform",
        json!({ "uids": [wall], "transform": { "kind": "move", "dx": 0.0, "dy": 0.02 } }),
    );
    event(&mut app, Event::Check);
    let lapsed: Vec<usize> = findings(&app)
        .iter()
        .enumerate()
        .filter(|(_, f)| f.lapsed)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(lapsed.len(), 1, "{:?}", findings(&app));
    // İstisnayı kaldır on its row.
    let row = app.arch_audit_shown().iter().position(|i| *i == lapsed[0]).expect("shown");
    event(&mut app, Event::Unexcept(Some(row)));
    assert_eq!(exceptions(&app).len(), 1);
    // The other's finding gone (the space taken away): Eski istisnaları temizle drops its exception.
    let space = exceptions(&app)[0].uids[0].clone();
    command(&mut app, "cad.entities.delete", json!({ "uids": [space] }));
    event(&mut app, Event::Check);
    assert_eq!(app.arch_audit.audited.as_ref().expect("audited").stale.len(), 1);
    event(&mut app, Event::Prune);
    assert!(exceptions(&app).is_empty());
}

/// docs/adr/0732 §2: Kapsam keeps to the active level or to the objects selected when it was chosen; the rest is
/// counted outside. Seçili nesneler without a selection says so and keeps the scope.
#[test]
fn kapsam_keeps_to_the_level_and_the_selection() {
    let (mut app, _) = App::boot(None);
    open_flat(&mut app, |_| {});
    run(&mut app, "arch.audit");
    let all = findings(&app).len();
    app.selection.set(Vec::new());
    event(&mut app, Event::Scope(2));
    assert_eq!(app.arch_audit.scope, crate::arch_audit::AuditScope::All, "nothing selected");
    event(&mut app, Event::Scope(1));
    let level = findings(&app).len();
    let outside = app.arch_audit.audited.as_ref().expect("audited").outside as usize;
    assert!(level < all && level + outside == all, "{level} + {outside} = {all}");
    scoped(&mut app);
    let ids = ["0192f5a1-7030-7000-8000-000000000002", "0192f5a1-7030-7000-8000-000000000003"];
    assert!(findings(&app).iter().all(|f| f.uids.iter().any(|u| ids.contains(&u.as_str()))));
    assert!(!findings(&app).is_empty());
    event(&mut app, Event::Scope(0));
    assert_eq!(findings(&app).len(), all);
}

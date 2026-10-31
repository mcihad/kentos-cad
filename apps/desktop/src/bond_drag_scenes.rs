//! The pictures of docs/adr/0790: the room of docs/adr/0780's (its inner wall bonded to the fixed outer east wall,
//! d1); Duvarı kaydır with the inner wall's edge following the pointer and the bonds marked as it moves; a grip of the
//! inner wall dragged (the wall turns: d1 cannot stay parallel); the inner wall fixed too, so no bond alone lets the
//! shift through and the card names the two to take away together; the shift refused and its card over the drawing
//! with “Bu iki bağı kaldır”; the two taken away and the shift going through. `tools_screens` takes them in both themes
//! at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bag-kaydir-onizleme,bag-tutamac-surukle,bag-birlikte,bag-birlikte-kart,bag-birlikte-kaldirildi cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use crate::app::{App, Message};
use crate::tools_scenes::{at, click, hover, run};
use crate::tools_screens::Scene;
use crate::viewport::Event;
use crate::wall_scenes::bonded_and_fixed;

/// Duvarı kaydır on the inner wall, its edge picked and the pointer 40 cm east: the bond would drag the fixed wall.
fn shift_preview(app: &mut App) {
    bonded_and_fixed(app);
    app.selection.clear();
    run(app, "tool.wallShift");
    click(app, [4.2, 2.5]);
    hover(app, [4.6, 2.5]);
}

/// The inner wall's top grip dragged 40 cm east, the button still down: the wall turns, d1 cannot stay parallel.
fn grip_drag(app: &mut App) {
    bonded_and_fixed(app);
    app.selection.set([Slot(2)]);
    let from = at(app, [4.2, 5.0]);
    let to = at(app, [4.6, 4.8]);
    let _ = app.update(Message::Viewport(Event::Moved(from)));
    let _ = app.update(Message::Viewport(Event::Pressed(from)));
    let _ = app.update(Message::Viewport(Event::Moved(to)));
}

/// The inner wall fixed too: no bond alone frees the shift; the card names the two together.
fn together(app: &mut App) {
    bonded_and_fixed(app);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let uid = doc.uid(Slot(2)).expect("an id").to_string();
    let fix: kentos_commands::ConstraintsEdit = serde_json::from_value(json!({"edit": {"kind": "add", "constraints": [
        {"rule": {"kind": "fix", "a": {"uid": uid, "element": {"kind": "wall", "edge": 0, "line": "axis"}}}}]}}))
    .expect("an input");
    let done = kentos_native_application::constraints_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        fix,
    );
    assert!(
        matches!(done, kentos_commands::CommandResult::Completed { .. }),
        "{done:?}"
    );
    app.selection.clear();
    run(app, "tool.wallShift");
    click(app, [4.2, 2.5]);
    hover(app, [4.6, 2.5]);
}

/// The shift clicked: refused, its card over the drawing with Bu iki bağı kaldır, the bonds marked.
fn together_card(app: &mut App) {
    together(app);
    click(app, [4.6, 2.5]);
}

/// Bu iki bağı kaldır pressed, then the same shift: it goes through, the card says what it does.
fn together_removed(app: &mut App) {
    together(app);
    click(app, [4.6, 2.5]);
    let _ = app.update(Message::BondNotice(crate::bond_notice::Event::Remove));
    hover(app, [4.61, 2.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("bag-kaydir-onizleme", shift_preview),
        ("bag-tutamac-surukle", grip_drag),
        ("bag-birlikte", together),
        ("bag-birlikte-kart", together_card),
        ("bag-birlikte-kaldirildi", together_removed),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(app: &App) -> Vec<String> {
        app.drawing_preview(&app.format())
            .map(|p| {
                p.labels
                    .into_iter()
                    .filter(|l| l.text.starts_with('×'))
                    .map(|l| l.text)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The sample drawing with its drawing area 1100 × 600 (as the screens' window lays it out).
    fn app() -> App {
        let mut app = crate::files_testing::app_with_drawing();
        app.viewport.camera.set_size(1100.0, 600.0);
        app
    }

    #[test]
    fn the_shift_marks_the_bonds_as_the_pointer_moves() {
        let mut app = app();
        shift_preview(&mut app);
        let marks = labels(&app);
        assert_eq!(marks.len(), 2, "{marks:?}");
        assert!(marks.iter().all(|m| m.ends_with("kaldırılınca geçer")), "{marks:?}");
        // Nothing written, no card yet.
        assert!(app.bonds_refused.is_none());
    }

    #[test]
    fn a_grip_dragged_marks_the_bond_it_turns() {
        let mut app = app();
        grip_drag(&mut app);
        let marks = labels(&app);
        assert_eq!(marks, ["× d1 — kaldırılınca geçer"], "{marks:?}");
    }

    #[test]
    fn no_bond_alone_frees_it_two_together_do_and_the_card_takes_them_away() {
        let mut app = app();
        together(&mut app);
        let marks = labels(&app);
        assert!(marks.iter().any(|m| m.contains("ile birlikte kaldırılınca geçer")), "{marks:?}");
        click(&mut app, [4.6, 2.5]);
        let error = app.bonds_refused.clone().expect("the refusal's card");
        let (ids, said) = kentos_interaction::bond_refusal::way_through(&error.bonds).expect("a way");
        assert_eq!(said, "Bu iki bağı kaldır");
        assert_eq!(ids.len(), 2);
        let before = app.document.as_ref().expect("a drawing").model.constraints().constraints.len();
        let _ = app.update(Message::BondNotice(crate::bond_notice::Event::Remove));
        let after = app.document.as_ref().expect("a drawing").model.constraints().constraints.len();
        assert_eq!(before - after, 2);
        assert!(app.bonds_refused.is_none());
        // Geri al lets Duvarı kaydır's edge go first; the next one undo step brings both bonds back.
        let _ = app.update(Message::Run("edit.undo"));
        let _ = app.update(Message::Run("edit.undo"));
        let back = app.document.as_ref().expect("a drawing").model.constraints().constraints.len();
        assert_eq!(back, before);
    }

    /// docs/adr/0797: a value written in Öznitelikler that the bonds refuse (the inner wall, fixed too, made thicker:
    /// its face moves, d1 cannot follow) leaves the same card with the way through; the next write that goes through
    /// takes it away.
    #[test]
    fn a_value_written_in_the_properties_leaves_the_card() {
        let mut app = app();
        together(&mut app);
        let _ = app.update(Message::Run("tool.select"));
        app.selection.set([Slot(2)]);
        let _ = app.update(Message::Properties(crate::properties::Event::Commit(
            crate::properties::Field::WallWidth(Slot(2)),
            "0.40".into(),
        )));
        let error = app.bonds_refused.clone().expect("the refusal's card");
        assert_eq!(error.code, "constraint_conflict");
        assert!(kentos_interaction::bond_refusal::way_through(&error.bonds).is_some());
        let _ = app.update(Message::BondNotice(crate::bond_notice::Event::Remove));
        assert!(app.bonds_refused.is_none());
        let _ = app.update(Message::Properties(crate::properties::Event::Commit(
            crate::properties::Field::WallWidth(Slot(2)),
            "0.40".into(),
        )));
        assert!(app.bonds_refused.is_none(), "it goes through now");
    }
}

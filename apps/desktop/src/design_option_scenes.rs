//! The pictures and the window's acceptance of docs/adr/0788 (Tasarım seçenekleri): the shared cases' flat
//! (`fixtures/commands/v1/cad.options.edit.json` `setup`); its kitchen selected in Tasarım seçenekleri with the new
//! set's name typed; the set with A and a copy B whose partition and counter moved, B chosen and compared with A; the
//! plan drawing A while A is primary and B once B is; a named view whose recipe shows B in Görünümler. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=secenek-tanimla,secenek-pencere,secenek-plan-a,secenek-plan-b,secenek-tarif cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::design_options_window::Event;
use crate::files_testing::make_cad;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.options.edit.json");

fn open(app: &mut App) {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.draft.snap = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -0.8,
            min_y: -1.6,
            max_x: 6.8,
            max_y: 4.8,
        },
        24.0,
    );
}

fn command(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::DesignOptions(e));
}

/// The kitchen's objects selected, Tasarım seçenekleri open with the name typed: Seçimden tanımla is the next press.
fn define(app: &mut App) {
    open(app);
    app.selection.set((5..=9).map(Slot));
    run(app, "arch.options");
    event(app, Event::Name("Mutfak".into()));
}

/// The set with A and B: B's partition 40 cm right (its door with it), its counter 30 cm right.
fn two(app: &mut App) -> Vec<String> {
    define(app);
    event(app, Event::Define);
    event(app, Event::Copy);
    event(app, Event::Close);
    app.selection.set(Vec::<Slot>::new());
    let set = app.document.as_ref().expect("a drawing").model.arch_options()[0].clone();
    let b = set.options[1].members.clone();
    for (uid, dx) in [(&b[0], 0.4), (&b[4], 0.3)] {
        command(
            app,
            "cad.entities.transform",
            json!({"uids": [uid], "transform": {"kind": "move", "dx": dx, "dy": 0.0}}),
        );
    }
    b
}

/// Tasarım seçenekleri on the set, B chosen: its facts, the comparison with A, the plan of Kabul et.
fn window(app: &mut App) {
    two(app);
    run(app, "arch.options");
    event(app, Event::Option(1));
}

/// The plan while A is primary: B is not drawn.
fn plan_a(app: &mut App) {
    two(app);
}

/// B made primary: the plan draws B's partition and counter, A's are not drawn.
fn plan_b(app: &mut App) {
    two(app);
    run(app, "arch.options");
    event(app, Event::Option(1));
    event(app, Event::Primary);
    event(app, Event::Close);
}

/// Görünümler on a view whose recipe shows B while A stays primary.
fn recipe(app: &mut App) {
    two(app);
    command(
        app,
        "cad.views.edit",
        json!({"edit": {"kind": "save", "name": "Mutfak B", "center": {"x": 3.0, "y": 2.0}, "width": 8.0, "height": 6.0}}),
    );
    command(
        app,
        "cad.views.edit",
        json!({"edit": {"kind": "recipe", "name": "Mutfak B", "recipe": {"level": {"id": "zemin"}, "options": [{"set": "mutfak", "option": "b"}]}}}),
    );
    let _ = app.update(Message::Run("view.namedViews"));
    let _ = app.update(Message::Views(crate::views::Event::Choose("Mutfak B".into())));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("secenek-tanimla", define),
        ("secenek-pencere", window),
        ("secenek-plan-a", plan_a),
        ("secenek-plan-b", plan_b),
        ("secenek-tarif", recipe),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Dialog;
    use crate::files_testing::last_said;

    fn shown(app: &App, slot: u32) -> bool {
        app.spatial.store().view_shown(f64::from(slot))
    }

    /// docs/adr/0788 §1, §2: Seçimden tanımla makes the selection A; Seçeneği kopyala adds B; Birincil yap swaps what
    /// the plan draws; Kabul et keeps B and deletes A in one undo step; Kapat writes nothing.
    #[test]
    fn the_window_defines_copies_switches_and_accepts() {
        let mut app = crate::files_testing::app_with_drawing();
        define(&mut app);
        assert_eq!(app.dialog, Some(Dialog::DesignOptions));
        event(&mut app, Event::Define);
        let doc = &app.document.as_ref().expect("a drawing").model;
        assert_eq!(doc.arch_options().len(), 1);
        assert_eq!(doc.arch_options()[0].options[0].members.len(), 5);
        event(&mut app, Event::Copy);
        assert!(last_said(&app).contains("B seçeneği eklendi"), "{}", last_said(&app));
        let b_wall = app.document.as_ref().expect("a drawing").model.entities().count() as u32;
        app.spatial.sync(&app.document.as_ref().expect("a drawing").model);
        assert!(shown(&app, 5), "A is drawn");
        assert!(!shown(&app, b_wall - 4), "B is not");
        event(&mut app, Event::Option(1));
        event(&mut app, Event::Primary);
        app.spatial.sync(&app.document.as_ref().expect("a drawing").model);
        assert!(!shown(&app, 5), "A is not drawn once B is primary");
        assert!(shown(&app, b_wall - 4), "B is");
        let before = app.document.as_ref().expect("a drawing").model.entities().count();
        event(&mut app, Event::Accept);
        let doc = &app.document.as_ref().expect("a drawing").model;
        assert!(doc.arch_options().is_empty());
        assert_eq!(doc.entities().count(), before - 5);
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        doc.undo();
        assert_eq!(doc.arch_options().len(), 1);
        assert_eq!(doc.entities().count(), before);
        event(&mut app, Event::Close);
        assert_eq!(app.dialog, None);
    }

    /// docs/adr/0788 §3: the recipe's Tasarım seçeneği chooses B for the set and Tarifi uygula writes it.
    #[test]
    fn the_recipe_chooses_an_option() {
        let mut app = crate::files_testing::app_with_drawing();
        two(&mut app);
        command(
            &mut app,
            "cad.views.edit",
            json!({"edit": {"kind": "save", "name": "Mutfak B", "center": {"x": 3.0, "y": 2.0}, "width": 8.0, "height": 6.0}}),
        );
        let _ = app.update(Message::Run("view.namedViews"));
        let _ = app.update(Message::Views(crate::views::Event::Choose("Mutfak B".into())));
        let _ = app.update(Message::Views(crate::views::Event::RecipeOption(0, Some(1))));
        let _ = app.update(Message::Views(crate::views::Event::RecipeApply));
        let r = app.recipe_draft("Mutfak B");
        assert_eq!(
            r.options,
            vec![kentos_commands::OptionChoice {
                set: "mutfak".into(),
                option: "b".into()
            }]
        );
        assert!(last_said(&app).contains("Mutfak B"), "{}", last_said(&app));
    }
}

//! The pictures of docs/adr/0723 (Rampa: ara sahanlık, kata bağlı başlangıç, metraj, erişilebilirlik): a project of
//! Zemin kat and 1. kat (3.06 m), Etkin kat 1. kat. Rampa under the cursor, 1.20 m up at %5 with En uzun kol 9 m and its
//! start 15 cm below the level, both railings: its two landings, its slope and its levels; the ramp written and
//! selected (Öznitelikler's Başlangıç kotu and Kata göre); 1. kat raised to 3.40 m (its levels follow); Metraj's
//! Rampalar; Mimari denetim's Erişilebilirlik with a narrow L and a steep ramp on Zemin kat, the landing's row chosen,
//! then fixed. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=rampa-sahanlik-arac,rampa-sahanlik-yazildi,rampa-sahanlik-kat,rampa-sahanlik-metraj,rampa-sahanlik-denetim,rampa-sahanlik-duzeltildi,rampa-sahanlik-oznitelik cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::{fit, opened};

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

fn levels(upper: f64) -> Value {
    json!([
        {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.06},
        {"id": "kat-1", "name": "1. kat", "elevation": upper, "height": 3.06}
    ])
}

/// The project with its two levels, Etkin kat 1. kat, the view round the ramp's place and the plan below it.
fn levelled(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    command(app, "cad.settings.set", json!({"arch": {"levels": levels(3.06)}}));
    app.choose_level(Some("kat-1".into()));
    fit(app, 30.0, 12.0);
    app.tab = "arch";
    app.command_expanded = true;
}

/// Rampa: 1.20 m up at %5, landings 1.50 m, En uzun kol 9 m, its start Kat −0.15, both railings; one click, the cursor
/// far along: the line ends where its slope says, a landing wherever a run would pass 9 m.
fn tool(app: &mut App) {
    levelled(app);
    run(app, "tool.ramp");
    for (key, value) in [("Y", "1.2"), ("E", "5"), ("H", "1.5"), ("L", "9"), ("Z", "kat-0.15")] {
        typed(app, key);
        typed(app, value);
    }
    for _ in 0..3 {
        typed(app, "O");
    }
    click(app, [1.0, 9.0]);
    hover(app, [31.0, 9.0]);
}

/// The ramp, the newest.
fn ramp_slot(app: &App) -> Option<Slot> {
    app.document.as_ref().and_then(|d| {
        d.model
            .entities()
            .filter(|e| matches!(e, Entity::Ramp(_)))
            .last()
            .map(|e| Slot(e.base().id))
    })
}

/// The ramp written (its direction clicked, Enter), the tool closed and the ramp selected.
fn written(app: &mut App) {
    tool(app);
    click(app, [31.0, 9.0]);
    typed(app, "");
    app.cancel();
    app.cancel();
    if let Some(slot) = ramp_slot(app) {
        app.selection.set([slot]);
    }
    app.ribbon_context = true;
    hover(app, [24.0, 12.5]);
}

/// The view on `[x0, y0]`–`[x1, y1]` (from the drawing's origin).
fn view_on(app: &mut App, a: [f64; 2], b: [f64; 2]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + a[0], min_y: N + a[1], max_x: E + b[0], max_y: N + b[1] },
        24.0,
    );
}

/// The upper level (1. kat) raised to 3.40 m in the project: the ramp's levels follow, its slope stays; the view on
/// its start, where its slope and its first level are written.
fn raised(app: &mut App) {
    written(app);
    command(app, "cad.settings.set", json!({"arch": {"levels": levels(3.4)}}));
    view_on(app, [-1.5, 7.0], [11.5, 11.0]);
    hover(app, [10.5, 10.6]);
}

/// Two more ramps on Zemin kat: an L 1.20 m wide whose landing is its width, and a short steep one.
fn more(app: &mut App) {
    command(app, "cad.ramps.create", json!({"layerId": "cizim", "ramps": [
        {"pts": [{"x": E + 1.0, "y": N + 1.0}, {"x": E + 7.0, "y": N + 1.0}, {"x": E + 7.0, "y": N + 5.0}],
         "width": 1.2, "rise": 0.5, "arch": {"level": "zemin"}},
        {"pts": [{"x": E + 12.0, "y": N + 2.0}, {"x": E + 15.0, "y": N + 2.0}], "width": 1.5, "rise": 0.3,
         "arch": {"level": "zemin"}}
    ]}));
}

fn quantities(app: &mut App) {
    raised(app);
    more(app);
    app.selection.clear();
    run(app, "arch.quantities");
    let _ = app.update(Message::Quantities(crate::quantities::Event::Tab(crate::quantities::Tab::Ramps)));
}

/// Mimari denetim with the profile: the ramps' findings; the narrow L's landing chosen (its Düzelt says its fix).
fn audit(app: &mut App) {
    raised(app);
    more(app);
    app.selection.clear();
    run(app, "arch.audit");
    let row = app
        .arch_audit_shown()
        .iter()
        .position(|&i| {
            app.arch_audit.audited.as_ref().expect("audited").findings[i].code == "ramp_landing_short"
        })
        .expect("the landing's finding");
    let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Press(row)));
}

/// Düzelt: the landing made 1.50 m in one step; checked again, the narrow turn and the steep ramp are left.
fn fixed(app: &mut App) {
    audit(app);
    let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Fix));
}

/// The ramp selected, Öznitelikler scrolled to its rows: Başlangıç kotu, Kata göre, Sahanlık, Korkuluk, Kat.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![("rampa-sahanlik-oznitelik", raised, |s, app| {
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Genel", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -5.0));
    })]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("rampa-sahanlik-arac", tool),
        ("rampa-sahanlik-yazildi", written),
        ("rampa-sahanlik-kat", raised),
        ("rampa-sahanlik-metraj", quantities),
        ("rampa-sahanlik-denetim", audit),
        ("rampa-sahanlik-duzeltildi", fixed),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    fn ramps(app: &App) -> Vec<kentos_commands::RampFields> {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter_map(|e| match e {
                Entity::Ramp(r) => Some(r.ramp.clone()),
                _ => None,
            })
            .collect()
    }

    /// The walkthrough as the pictures take it (docs/adr/0723): the tool writes the ramp bound to 1. kat with its two
    /// landings; the level raised, its start follows; Metraj counts it; Denetim's profile finds the narrow L's short
    /// landing, and its Düzelt mends it in one step.
    #[test]
    fn the_ramp_tool_binds_lands_and_is_measured_and_audited() {
        let mut app = app_with_drawing();
        written(&mut app);
        let r = &ramps(&app)[0];
        assert_eq!(r.base_offset, Some(-0.15));
        assert!((r.base.unwrap() - 2.91).abs() < 1e-9, "{:?}", r.base);
        assert_eq!(r.arch.level.as_deref(), Some("kat-1"));
        assert_eq!(r.pts.len(), 4, "two landings on its way: {:?}", r.pts);
        assert_eq!(r.railings, Some(kentos_commands::Railings::Both));
        command(&mut app, "cad.settings.set", json!({"arch": {"levels": levels(3.4)}}));
        let r = &ramps(&app)[0];
        assert!((r.base.unwrap() - 3.25).abs() < 1e-9, "{:?}", r.base);
        more(&mut app);
        let q = kentos_native_application::spaces_quantities::plan_in(
            &app.document.as_ref().unwrap().model,
            &kentos_commands::SpacesQuantities::default(),
        );
        let kentos_commands::CommandResult::Completed { output, .. } = q else { panic!("{q:?}") };
        assert_eq!(output.ramps.len(), 3);
        assert_eq!(output.ramps[0].landings, 2);
        assert!((output.ramps[0].run - 24.0).abs() < 1e-9, "{}", output.ramps[0].run);
        audit(&mut app);
        let codes: Vec<String> =
            app.arch_audit.audited.as_ref().unwrap().findings.iter().map(|f| f.code.clone()).collect();
        assert!(codes.contains(&"ramp_landing_short".to_owned()), "{codes:?}");
        assert!(codes.contains(&"ramp_slope_over_profile".to_owned()), "{codes:?}");
        let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Fix));
        let codes: Vec<String> =
            app.arch_audit.audited.as_ref().unwrap().findings.iter().map(|f| f.code.clone()).collect();
        assert!(!codes.contains(&"ramp_landing_short".to_owned()), "{codes:?}");
        assert!(codes.contains(&"ramp_turn_narrow".to_owned()), "{codes:?}");
    }
}

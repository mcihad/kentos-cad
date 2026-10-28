//! The pictures of docs/adr/0724 (Merdiven: kata bağlı yükseklik, üst katta gösterim, merkez direği, metraj, yönetmelik
//! profili): a project of Zemin kat and 1. kat (3.06 m), a stair well's walls on both. Merdiven under the cursor on
//! Zemin kat, a U whose height is the level's, both railings; the stair written and selected (Öznitelikler's Yükseklik:
//! Kat yüksekliği); 1. kat alone, the stair seen from above (its part past its cut); Zemin kat raised to 3.40 m (its
//! risers follow); Döner merdiven with Merkez direği; Metraj's Merdivenler; Mimari denetim's Merdiven profile (Konut,
//! asansörlü) with a narrow curved stair, then a railing fixed. `tools_screens` takes them in both themes at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=merdiven-kat-arac,merdiven-kat-yazildi,merdiven-ust-kat,merdiven-ust-kat-alt,merdiven-kat-yukseldi,merdiven-kat-oznitelik,merdiven-direk,merdiven-metraj,merdiven-profil-denetim,merdiven-profil-duzeltildi cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::Entity;
use kentos_domain::Slot;
use kentos_interaction::level_view::LevelView;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;

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

fn levels(ground: f64) -> Value {
    json!([
        {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": ground},
        {"id": "kat-1", "name": "1. kat", "elevation": ground, "height": 3.06}
    ])
}

fn p(x: f64, y: f64) -> Value {
    json!({"x": E + x, "y": N + y})
}

/// The view on `[x0, y0]`–`[x1, y1]` (from the drawing's origin).
fn view_on(app: &mut App, a: [f64; 2], b: [f64; 2]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + a[0], min_y: N + a[1], max_x: E + b[0], max_y: N + b[1] },
        24.0,
    );
}

/// The project with its two levels, a 7 × 5.4 m stair hall's walls on each, Etkin kat Zemin kat, the view round it.
fn levelled(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    command(app, "cad.settings.set", json!({"arch": {"levels": levels(3.06)}}));
    for level in ["zemin", "kat-1"] {
        command(app, "cad.walls.create", json!({"layerId": "cizim", "walls": [{
            "pts": [p(0.0, 0.0), p(7.0, 0.0), p(7.0, 5.4), p(0.0, 5.4)], "closed": true, "width": 0.25,
            "justify": "right", "arch": {"level": level}
        }]}));
    }
    app.choose_level(Some("zemin".into()));
    view_on(app, [0.2, 0.2], [6.0, 3.6]);
    app.tab = "arch";
    app.command_expanded = true;
    // The clicks where they are, not on the walls' corners.
    app.draft.snap = false;
}

/// Merdiven on Zemin kat: its height the level's (3.06 m · katın), both railings; a U clicked round the hall, the
/// cursor on its last flight.
fn tool(app: &mut App) {
    levelled(app);
    run(app, "tool.stair");
    for _ in 0..3 {
        typed(app, "O");
    }
    click(app, [1.5, 1.2]);
    click(app, [4.5, 1.2]);
    click(app, [4.5, 2.5]);
    hover(app, [1.0, 2.5]);
}

/// The stair, the newest.
fn stair_slot(app: &App) -> Option<Slot> {
    app.document.as_ref().and_then(|d| {
        d.model
            .entities()
            .filter(|e| matches!(e, Entity::Stair(_)))
            .last()
            .map(|e| Slot(e.base().id))
    })
}

/// The stair written (its last flight clicked, Enter), the tool closed and the stair selected.
fn written(app: &mut App) {
    tool(app);
    click(app, [1.0, 2.5]);
    typed(app, "");
    app.cancel();
    app.cancel();
    if let Some(slot) = stair_slot(app) {
        app.selection.set([slot]);
    }
    app.ribbon_context = true;
    hover(app, [6.5, 0.5]);
}

/// 1. kat alone (Yalnız etkin kat): Zemin kat's stair seen from above, its part past its cut.
fn above(app: &mut App) {
    written(app);
    app.selection.clear();
    app.choose_level(Some("kat-1".into()));
    let _ = app.update(Message::LevelView(LevelView::Active));
    hover(app, [6.5, 0.5]);
}

/// 1. kat with Zemin kat faint under it (Etkin kat ve alt kat): the stair from above at full weight, the rest faint.
fn above_below(app: &mut App) {
    above(app);
    let _ = app.update(Message::LevelView(LevelView::Below));
}

/// Zemin kat raised to 3.40 m: the bound stair's risers follow; selected, Öznitelikler says so.
fn raised(app: &mut App) {
    written(app);
    command(app, "cad.settings.set", json!({"arch": {"levels": levels(3.4)}}));
    if let Some(slot) = stair_slot(app) {
        app.selection.set([slot]);
    }
}

/// Döner merdiven with Merkez direği 0.20 m: its centre clicked, its start, the cursor round it.
fn post(app: &mut App) {
    levelled(app);
    view_on(app, [0.0, -0.4], [6.4, 4.6]);
    run(app, "tool.stairCurved");
    typed(app, "M");
    typed(app, "0.2");
    typed(app, "G");
    typed(app, "1");
    click(app, [3.1, 2.0]);
    click(app, [4.6, 2.0]);
    hover(app, [3.1, 3.5]);
    hover(app, [1.6, 2.0]);
}

/// Two more stairs on Zemin kat beside the hall: a curved one too narrow at its inner side and a straight one with a
/// left railing only.
fn more(app: &mut App) {
    command(app, "cad.stairs.create", json!({"layerId": "cizim", "stairs": [
        {"pts": [p(11.0, 2.7), p(9.6, 2.7)], "bulges": [1.0, 0.0], "width": 1.0, "risers": 10, "riserHeight": 0.17,
         "tread": 0.27, "post": 0.2, "railings": "both", "arch": {"level": "zemin"}},
        {"pts": [p(13.0, 0.5), p(13.0, 5.0)], "width": 1.1, "risers": 16, "riserHeight": 0.17, "tread": 0.28,
         "railings": "left", "arch": {"level": "zemin"}}
    ]}));
}

fn quantities(app: &mut App) {
    written(app);
    more(app);
    app.selection.clear();
    run(app, "arch.quantities");
    let _ = app.update(Message::Quantities(crate::quantities::Event::Tab(crate::quantities::Tab::Stairs)));
}

/// Mimari denetim, Merdiven Konut (asansörlü): the stairs' findings; the straight one's missing railing chosen.
fn audit(app: &mut App) {
    written(app);
    more(app);
    app.selection.clear();
    run(app, "arch.audit");
    view_on(app, [-0.2, -0.2], [14.2, 5.6]);
    let row = app
        .arch_audit_shown()
        .iter()
        .position(|&i| {
            let f = &app.arch_audit.audited.as_ref().expect("audited").findings[i];
            f.code == "stair_railing_missing"
        })
        .expect("the railing's finding");
    let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Press(row)));
}

/// Düzelt: both railings in one step; checked again, the narrow stairs are left.
fn fixed(app: &mut App) {
    audit(app);
    let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Fix));
}

/// The stair selected, Öznitelikler scrolled to its rows: Rıht, Toplam yükseklik, Yükseklik (Kat yüksekliği).
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![("merdiven-kat-oznitelik", raised, |s, app| {
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Genel", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -8.0));
    })]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("merdiven-kat-arac", tool),
        ("merdiven-kat-yazildi", written),
        ("merdiven-ust-kat", above),
        ("merdiven-ust-kat-alt", above_below),
        ("merdiven-kat-yukseldi", raised),
        ("merdiven-direk", post),
        ("merdiven-metraj", quantities),
        ("merdiven-profil-denetim", audit),
        ("merdiven-profil-duzeltildi", fixed),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    fn stairs(app: &App) -> Vec<(Slot, kentos_commands::StairFields)> {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter_map(|e| match e {
                Entity::Stair(s) => Some((Slot(e.base().id), s.stair.clone())),
                _ => None,
            })
            .collect()
    }

    /// The walkthrough as the pictures take it (docs/adr/0724): the tool writes a U bound to Zemin kat's height; 1. kat
    /// alone shows it from above; the level raised, its risers follow; Döner merdiven writes its centre post; Metraj
    /// counts the stairs; the Merdiven profile finds the missing railing and the narrow inner tread, and Düzelt mends the
    /// railing in one step.
    #[test]
    fn the_stair_tool_binds_shows_above_is_measured_and_audited() {
        let mut app = app_with_drawing();
        written(&mut app);
        let (slot, s) = stairs(&app).remove(0);
        assert!(s.height_from_level, "bound to Zemin kat's height");
        assert_eq!(s.arch.level.as_deref(), Some("zemin"));
        assert_eq!(s.risers, 18);
        assert!((f64::from(s.risers) * s.riser_height - 3.06).abs() < 1e-9);
        assert_eq!(s.railings, Some(kentos_commands::Railings::Both));
        assert_eq!(s.pts.len(), 4, "a U: {:?}", s.pts);
        assert!(s.cut.is_some(), "its cut at 1.10 m");
        // 1. kat alone: from above, not picked.
        app.selection.clear();
        app.choose_level(Some("kat-1".into()));
        let _ = app.update(Message::LevelView(LevelView::Active));
        let (_, looks) = app.spatial.level_looks();
        assert_eq!(looks.upper, vec![slot.0]);
        assert!(app.spatial.draws_otherwise());
        // Zemin kat raised: the risers follow.
        command(&mut app, "cad.settings.set", json!({"arch": {"levels": levels(3.4)}}));
        let (_, s) = stairs(&app).remove(0);
        assert!((f64::from(s.risers) * s.riser_height - 3.4).abs() < 1e-9, "{s:?}");
        // Döner merdiven with its post.
        let mut app = app_with_drawing();
        post(&mut app);
        click(&mut app, [1.6, 2.0]);
        typed(&mut app, "");
        let (_, s) = stairs(&app).pop().expect("the curved stair");
        assert_eq!(s.post, Some(0.2));
        assert!(s.bulges.is_some());
        // Metraj and Denetim.
        let mut app = app_with_drawing();
        written(&mut app);
        more(&mut app);
        let q = kentos_native_application::spaces_quantities::plan_in(
            &app.document.as_ref().unwrap().model,
            &kentos_commands::SpacesQuantities::default(),
        );
        let kentos_commands::CommandResult::Completed { output, .. } = q else { panic!("{q:?}") };
        assert_eq!(output.stairs.len(), 3);
        assert!(output.stairs[0].height_from_level);
        assert!(output.stairs[1].inner_tread.is_some() && output.stairs[1].radius.is_some());
        assert_eq!(output.totals.stair_risers, 18 + 10 + 16);
        audit(&mut app);
        let codes: Vec<String> =
            app.arch_audit.audited.as_ref().unwrap().findings.iter().map(|f| f.code.clone()).collect();
        assert!(codes.contains(&"stair_railing_missing".to_owned()), "{codes:?}");
        assert!(codes.contains(&"stair_winder_narrow".to_owned()), "{codes:?}");
        assert!(codes.contains(&"stair_width_narrow".to_owned()), "{codes:?}");
        let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Fix));
        let codes: Vec<String> =
            app.arch_audit.audited.as_ref().unwrap().findings.iter().map(|f| f.code.clone()).collect();
        assert!(!codes.contains(&"stair_railing_missing".to_owned()), "{codes:?}");
        assert!(codes.contains(&"stair_winder_narrow".to_owned()), "{codes:?}");
    }
}

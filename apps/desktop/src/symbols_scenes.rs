//! The pictures of docs/adr/0721 (Mimari işaretler): the walkthrough's apartment with KentOS's wall styles (Dış duvar
//! 25 round it, İç duvar 15 between the rooms) and a 1. kat 3.06 m up; Kot bound to 1. kat (the tool's Kat and Fark,
//! then the mark selected with Öznitelikler's Kot, Kat and Fark rows); Kuzey oku on the true north, its ghost and then
//! placed; Revizyon işaretle with its number, date and description round the kitchen, and Revizyon listesi placed
//! beside the plan; Malzeme lejandı of the walls' materials; the Mimari tab's Belgele menus (İşaret ▾, Revizyon ▾,
//! Çizelge ▾); Denetim after the walls took their styles (the interior chain still bound, docs/adr/0721 §6).
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=isaret-stil-denetim,isaret-kot-arac,isaret-kot-oznitelik,isaret-kuzey-arac,isaret-kuzey,isaret-revizyon-onizleme,isaret-revizyon-listesi,isaret-lejant,isaret-belgele-isaret,isaret-belgele-revizyon,isaret-belgele-cizelge cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::App;
use crate::architecture_settings::Commit;
use crate::tools_scenes::{click, hover, run};
use crate::tools_screens::{Pointed, Scene, open_split};
use crate::walkthrough_scenes::{interior, view};

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

fn setting(app: &mut App, tool: &'static str, key: &'static str, value: &str) {
    let _ = app.tool_setting(Commit {
        tool,
        key,
        value: value.into(),
    });
}

/// The finished ground floor with its walls styled (Dış duvar 25 round it, İç duvar 15 between the rooms) and a 1. kat
/// 3.06 m up, nothing drawn on it yet; Mimari in front.
fn styled(app: &mut App) {
    interior(app);
    let walls: Vec<(String, f64)> = {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities()
            .filter_map(|e| match e {
                Entity::Wall(w) => doc.uid(Slot(e.base().id)).map(|u| (u.to_string(), w.wall.width)),
                _ => None,
            })
            .collect()
    };
    for (width, style) in [(0.25, "Dış duvar 25"), (0.15, "İç duvar 15")] {
        let uids: Vec<&String> = walls.iter().filter(|(_, w)| (w - width).abs() < 1e-9).map(|(u, _)| u).collect();
        if !uids.is_empty() {
            command(app, "cad.walls.edit", json!({ "uids": uids, "operation": { "kind": "set", "style": style } }));
        }
    }
    if let Some(doc) = app.document.as_mut() {
        let mut settings = doc.model.settings().clone();
        let arch = settings.arch.get_or_insert_with(Default::default);
        arch.levels.push(kentos_commands::ArchLevel {
            id: "kat-1".into(),
            name: "1. kat".into(),
            elevation: 3.06,
            height: Some(3.06),
        });
        doc.model.set_settings(settings);
    }
    view(app);
    app.tab = "arch";
    app.command_expanded = true;
    app.selection.clear();
    hover(app, [14.0, 11.0]);
}

/// Kot started, Kat 1. kat and Fark −2 cm chosen in its settings; the west room's point given, the mark hanging from
/// the pointer: +3.04.
fn kot_tool(app: &mut App) {
    styled(app);
    run(app, "tool.dimElevation");
    setting(app, "dimElevation", "K", "kat-1");
    setting(app, "dimElevation", "F", "-0.02");
    click(app, [2.5, 6.5]);
    hover(app, [3.4, 7.3]);
}

/// The Kot written and selected: Öznitelikler's Kot (read from the level), Kat and Fark.
fn kot_selected(app: &mut App) {
    kot_tool(app);
    click(app, [3.4, 7.3]);
    app.cancel();
    let newest = app
        .document
        .as_ref()
        .and_then(|d| d.model.entities().filter(|e| matches!(e, Entity::Dimension(_))).last().map(|e| e.base().id));
    if let Some(id) = newest {
        app.selection.set([Slot(id)]);
    }
    hover(app, [14.0, 11.0]);
}

/// Kuzey oku with Yön Coğrafi kuzey: its ghost at the pointer off the plan's north-east corner.
fn north_tool(app: &mut App) {
    styled(app);
    run(app, "tool.northArrow");
    setting(app, "northArrow", "Y", "coğrafi");
    hover(app, [14.0, 10.0]);
}

/// The arrow placed: the meridian's convergence said.
fn north_placed(app: &mut App) {
    north_tool(app);
    click(app, [14.0, 10.0]);
    hover(app, [14.0, 4.0]);
}

/// Revizyon işaretle: revision 1 dated and described in its settings, the first corner given south-west of the
/// kitchen, the cloud and its delta following the pointer.
fn revision_preview(app: &mut App) {
    styled(app);
    run(app, "tool.revisionMark");
    setting(app, "revisionMark", "R", "1");
    setting(app, "revisionMark", "T", "2026-10-20");
    setting(app, "revisionMark", "A", "Mutfak duvarı taşındı");
    click(app, [4.2, 5.4]);
    hover(app, [8.6, 9.6]);
}

/// Two revisions marked (the kitchen's, then 2 round the east window), Revizyon listesi placed east of the plan.
fn revision_list(app: &mut App) {
    revision_preview(app);
    click(app, [8.6, 9.6]);
    setting(app, "revisionMark", "R", "2");
    setting(app, "revisionMark", "T", "2026-10-27");
    setting(app, "revisionMark", "A", "Doğu penceresi genişledi");
    click(app, [11.4, 1.0]);
    click(app, [12.9, 3.6]);
    app.cancel();
    run(app, "arch.revisionList");
    hover(app, [14.6, 9.4]);
    click(app, [14.6, 9.4]);
    app.cancel();
    app.selection.clear();
    hover(app, [14.0, -2.0]);
}

/// Malzeme lejandı of the styled walls placed east of the plan: each material's hatch and name.
fn legend(app: &mut App) {
    styled(app);
    run(app, "tool.materialLegend");
    hover(app, [14.6, 9.2]);
    click(app, [14.6, 9.2]);
    app.cancel();
    app.selection.clear();
    hover(app, [14.0, -2.0]);
}

/// Denetim over the styled walls: the interior chain still bound to their faces (docs/adr/0721 §6), nothing found.
fn styled_audit(app: &mut App) {
    styled(app);
    run(app, "arch.audit");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("isaret-stil-denetim", styled_audit),
        ("isaret-kot-arac", kot_tool),
        ("isaret-kuzey-arac", north_tool),
        ("isaret-kuzey", north_placed),
        ("isaret-revizyon-onizleme", revision_preview),
        ("isaret-revizyon-listesi", revision_list),
        ("isaret-lejant", legend),
    ]
}

/// The Mimari tab's Belgele menus: İşaret ▾ (Kesit, Görünüş, Detay, Kuzey oku), Revizyon ▾ (Revizyon işaretle,
/// Revizyon listesi), Çizelge ▾ (with Malzeme lejandı); the Kot's Öznitelikler scrolled to its Kot, Kat and Fark.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("isaret-kot-oznitelik", kot_selected, |s, app| {
            let mut update = |app: &mut App, message: crate::app::Message| {
                let _ = app.update(message);
            };
            let at = crate::tools_screens::caption_at(s, app, "Geometri", false);
            s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -2.0));
        }),
        ("isaret-belgele-isaret", styled, |s, app| open_split(s, app, "archMarks")),
        ("isaret-belgele-revizyon", styled, |s, app| open_split(s, app, "archRevisions")),
        ("isaret-belgele-cizelge", styled, |s, app| open_split(s, app, "archSchedules")),
    ]
}

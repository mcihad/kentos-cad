//! The pictures of docs/adr/0343 (grids, columns, stairs and ramps): a structural grid drawn with the Aks tool (A–D 6, 6
//! and 4.5 m apart, 1–3 5 m apart, each a series), columns at every crossing with Kolon's Aks kesişimleri (S1 …), a U
//! stair of 18 risers that fits its walking line and the level's 3.06 m, and a 5 % ramp; CAD's Mimari tab with Taşıyıcı
//! and Merdiven; Aks's series under the cursor; Kolon at a crossing (B-2); a column's Öznitelikler; Merdiven's L under
//! the cursor with its checks; a stair that does not fit, selected (its Denetim rows); Rampa under the cursor; Aksları
//! numarala with two grids picked; grid B moved, its columns following. `tools_screens` takes them in both themes at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=aks-serit,aks-sistem,aks-arac,kolon-arac,kolon-oznitelikler,merdiven-arac,merdiven-denetim,rampa,aks-numarala,aks-izleme cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::{fit, opened};

/// The level the stair climbs: Zemin kat of 3.06 m.
fn on_level(app: &mut App) {
    let Some(doc) = app.document.as_mut() else {
        return;
    };
    let mut settings = doc.model.settings().clone();
    settings.arch.get_or_insert_with(Default::default).levels = vec![kentos_commands::ArchLevel {
        id: "zemin".into(),
        name: "Zemin kat".into(),
        elevation: 0.0,
        height: Some(3.06),
    }];
    doc.model.set_settings(settings);
}

/// The grid alone: slots 1–4 the lettered grids A–D, 5–7 the numbered 1–3.
fn grids(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 100.0);
    on_level(app);
    fit(app, 18.0, 15.0);
    app.command_expanded = true;
    run(app, "tool.grid");
    typed(app, "S");
    typed(app, "6 6 4.5");
    typed(app, "E");
    typed(app, "A");
    click(app, [0.0, -1.5]);
    click(app, [0.0, 11.5]);
    click(app, [5.0, 5.0]);
    typed(app, "S");
    typed(app, "5 5");
    typed(app, "E");
    typed(app, "1");
    click(app, [-1.5, 0.0]);
    click(app, [18.0, 0.0]);
    click(app, [5.0, 5.0]);
    typed(app, "");
}

/// The grid, its twelve columns (slots 8–19, S1 …), the U stair (20) and the ramp (21).
fn system(app: &mut App) {
    grids(app);
    run(app, "tool.column");
    typed(app, "K");
    typed(app, "S1");
    typed(app, "A");
    typed(app, "");
    run(app, "tool.stair");
    typed(app, "G");
    typed(app, "1");
    for p in [[7.5, 1.2], [9.68, 1.2], [9.68, 3.2], [7.5, 3.2]] {
        click(app, p);
    }
    typed(app, "");
    run(app, "tool.ramp");
    typed(app, "E");
    typed(app, "5");
    click(app, [2.0, 13.5]);
    click(app, [12.0, 13.5]);
    typed(app, "");
    typed(app, "");
    hover(app, [19.0, -1.0]);
}

fn ribbon(app: &mut App) {
    system(app);
    app.tab = "arch";
}

fn written(app: &mut App) {
    system(app);
}

/// Aks with Seri (5 5): the first grid's two points given, the cursor on the side the others go.
fn grid_tool(app: &mut App) {
    grids(app);
    run(app, "tool.grid");
    typed(app, "S");
    typed(app, "3 3");
    typed(app, "E");
    typed(app, "E");
    click(app, [20.5, -1.5]);
    click(app, [20.5, 11.5]);
    hover(app, [23.0, 6.0]);
}

/// Kolon near B-2: the column the command would put at the crossing, tied to its grids.
fn column_tool(app: &mut App) {
    grids(app);
    run(app, "tool.column");
    typed(app, "B");
    typed(app, "E");
    typed(app, "0.45");
    hover(app, [6.15, 5.1]);
}

/// Column S5 selected: Öznitelikler's column rows, its place B-1.
fn column_props(app: &mut App) {
    system(app);
    app.selection.set([Slot(12)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Merdiven's L: two points given, the cursor at the third; the checks in its tag.
fn stair_tool(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    click(app, [13.0, 1.0]);
    click(app, [15.5, 1.0]);
    hover(app, [15.5, 4.0]);
}

/// A straight stair too short for its risers, selected: its Denetim rows.
fn stair_checks(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    click(app, [13.0, 1.0]);
    click(app, [13.0, 4.0]);
    typed(app, "");
    typed(app, "");
    app.selection.set([Slot(8)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Rampa: 0.6 m up an L, 8 % given; the cursor at its end.
fn ramp_tool(app: &mut App) {
    grids(app);
    run(app, "tool.ramp");
    typed(app, "Y");
    typed(app, "0.6");
    typed(app, "E");
    typed(app, "8");
    click(app, [2.0, 13.5]);
    click(app, [9.0, 13.5]);
    hover(app, [9.0, 16.0]);
}

/// Aksları numarala from E: D and C picked, the plan's labels drawn.
fn numbering(app: &mut App) {
    system(app);
    run(app, "tool.gridNumber");
    typed(app, "B");
    typed(app, "E");
    click(app, [16.5, 8.0]);
    click(app, [12.0, 8.0]);
    hover(app, [6.0, 8.0]);
}

/// Grid B moved 1 m east: its four columns stand at its new crossings, their places kept.
fn following(app: &mut App) {
    system(app);
    if let Some(doc) = app.document.as_mut() {
        let uids = kentos_interaction::properties::uids_of(&doc.model, &[Slot(2)]);
        let _ = kentos_native_application::transform::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::EntitiesTransform {
                uids,
                transform: kentos_commands::Transform::Move { dx: 1.0, dy: 0.0 },
                copy: None,
                expected_revision: None,
            },
        );
    }
    app.selection.set([Slot(2)]);
    hover(app, [19.0, -1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("aks-serit", ribbon),
        ("aks-sistem", written),
        ("aks-arac", grid_tool),
        ("kolon-arac", column_tool),
        ("kolon-oznitelikler", column_props),
        ("merdiven-arac", stair_tool),
        ("merdiven-denetim", stair_checks),
        ("rampa", ramp_tool),
        ("aks-numarala", numbering),
        ("aks-izleme", following),
    ]
}

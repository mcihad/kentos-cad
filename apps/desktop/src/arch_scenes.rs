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
//! KENTOS_SHOTS_ONLY=merdiven-yukseklik,merdiven-l,merdiven-u,merdiven-u-yazildi,merdiven-sigmaz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # docs/adr/0704
//! KENTOS_SHOTS_ONLY=rampa-egim,rampa-l-korkuluk,merdiven-korkuluk,rampa-sigmaz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # docs/adr/0705
//! KENTOS_SHOTS_ONLY=aks-agi,aks-agi-dondur,aks-agi-yazildi cargo test -p kentos-desktop tools_screens::screens -- --ignored --nocapture   # docs/adr/0707
//! KENTOS_SHOTS_ONLY=doner-merdiven,doner-merdiven-yazildi,doner-merdiven-dar,doner-merdiven-serit cargo test -p kentos-desktop tools_screens::screens -- --ignored --nocapture   # docs/adr/0709
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, click, hover, run, typed};
use crate::tools_screens::{Pointed, Scene, open_split};
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

/// A straight stair of 15 risers given for 3 m (0.20 m each: 2R+G 0.68 m), selected: its Denetim rows.
fn stair_checks(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    typed(app, "S");
    typed(app, "15");
    click(app, [13.0, 1.0]);
    click(app, [13.0, 4.0]);
    typed(app, "");
    typed(app, "");
    app.selection.set([Slot(8)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// The view on `min`–`max` (metres from the grid's origin), the layers folded so that the tool's settings stand whole.
fn close_on(app: &mut App, min: [f64; 2], max: [f64; 2]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + min[0],
            min_y: N + min[1],
            max_x: E + max[0],
            max_y: N + max[1],
        },
        24.0,
    );
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// docs/adr/0704: Merdiven from its height: one click, the cursor past where the stair ends. 18 risers of 0.167 m for
/// 3.00 m, the flight ending at its last riser however far the cursor is, the risers past the 1.10 m cut dashed; the
/// tool's settings beside.
fn stair_height(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    click(app, [2.0, 0.6]);
    close_on(app, [-1.0, -0.6], [7.0, 7.4]);
    hover(app, [2.0, 6.6]);
}

/// An L with its numbers: the turn clicked, the second flight ending at the last riser.
fn stair_l(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    typed(app, "N");
    click(app, [6.8, 1.0]);
    click(app, [9.4, 1.0]);
    close_on(app, [5.6, -0.6], [12.6, 6.4]);
    hover(app, [9.4, 5.4]);
}

/// A U whose flights are a width apart (1.20 m wide, 1.30 m between their lines): one landing across both, no riser
/// between the turns; the last flight ending at the last riser.
fn stair_u(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    click(app, [13.0, 0.6]);
    click(app, [13.0, 3.6]);
    click(app, [14.3, 3.6]);
    close_on(app, [11.0, -0.8], [17.0, 5.2]);
    hover(app, [14.3, 0.1]);
}

/// The U written by its fourth click, numbered, selected.
fn stair_u_written(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    typed(app, "N");
    for p in [[13.0, 0.6], [13.0, 3.6], [14.3, 3.6], [14.3, 0.2]] {
        click(app, p);
    }
    typed(app, "");
    app.selection.set([Slot(8)]);
    close_on(app, [11.4, -0.6], [15.9, 4.8]);
    hover(app, [16.6, -0.4]);
}

/// An L whose first flight holds every riser: the tag says why in red and nothing is written.
fn stair_no_fit(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    click(app, [0.6, 1.0]);
    click(app, [8.6, 1.0]);
    close_on(app, [-0.6, -1.0], [10.4, 6.0]);
    hover(app, [8.6, 3.8]);
}

/// docs/adr/0705: Rampa from its slope: one click, the cursor past where it ends. 0.50 m at 6 % ends 8.33 m on, its
/// levels at its ends; both railings; the tool's settings beside.
fn ramp_slope(app: &mut App) {
    grids(app);
    run(app, "tool.ramp");
    typed(app, "O");
    typed(app, "O");
    typed(app, "O");
    click(app, [1.0, 2.5]);
    close_on(app, [-0.5, -0.6], [11.5, 6.4]);
    hover(app, [10.8, 2.5]);
}

/// An L ramp written with its left railing, selected: its Korkuluk row.
fn ramp_l_railing(app: &mut App) {
    grids(app);
    run(app, "tool.ramp");
    typed(app, "O");
    click(app, [1.0, 1.0]);
    click(app, [6.0, 1.0]);
    click(app, [6.0, 3.0]);
    typed(app, "");
    typed(app, "");
    app.selection.set([Slot(8)]);
    close_on(app, [-0.5, -0.8], [9.5, 6.8]);
    hover(app, [9.0, 6.5]);
}

/// The U stair written with railings on both sides: the railings past its cut dashed with its risers.
fn stair_railings(app: &mut App) {
    grids(app);
    run(app, "tool.stair");
    typed(app, "O");
    typed(app, "O");
    typed(app, "O");
    for p in [[13.0, 0.6], [13.0, 3.6], [14.3, 3.6], [14.3, 0.2]] {
        click(app, p);
    }
    typed(app, "");
    app.selection.set([Slot(8)]);
    close_on(app, [11.4, -0.6], [15.9, 4.8]);
    hover(app, [16.6, -0.4]);
}

/// An L ramp whose first flight takes its whole sloped length: the tag says why in red; nothing is written.
fn ramp_no_fit(app: &mut App) {
    grids(app);
    run(app, "tool.ramp");
    click(app, [0.6, 1.0]);
    click(app, [11.0, 1.0]);
    close_on(app, [-0.6, -1.0], [14.4, 6.0]);
    hover(app, [11.0, 3.6]);
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

/// docs/adr/0707: Aks ağı under the cursor on an empty level: 6 6 4.5 by 5 5 from 1 and A, a 0.30 × 0.50 column at
/// every crossing; the tool's settings beside.
fn grid_system(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 100.0);
    on_level(app);
    fit(app, 18.0, 13.0);
    app.command_expanded = true;
    run(app, "tool.gridSystem");
    typed(app, "X");
    typed(app, "6 6 4.5");
    typed(app, "Y");
    typed(app, "2*5");
    typed(app, "K");
    hover(app, [0.5, 1.0]);
}

/// İki nokta: the origin clicked, the system turned 30° toward the cursor.
fn grid_system_turned(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 100.0);
    on_level(app);
    fit(app, 20.0, 18.0);
    app.command_expanded = true;
    run(app, "tool.gridSystem");
    typed(app, "P");
    click(app, [3.0, 1.0]);
    let turn = 30f64.to_radians();
    hover(app, [3.0 + 12.0 * turn.cos(), 1.0 + 12.0 * turn.sin()]);
}

/// The system written with its columns tagged from S1, column B-3 selected: its place and grids in Öznitelikler.
fn grid_system_written(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 100.0);
    on_level(app);
    fit(app, 18.0, 13.0);
    app.command_expanded = true;
    run(app, "tool.gridSystem");
    typed(app, "X");
    typed(app, "6 6 4.5");
    typed(app, "Y");
    typed(app, "2*5");
    typed(app, "K");
    typed(app, "S");
    typed(app, "S1");
    click(app, [0.5, 1.0]);
    typed(app, "");
    app.selection.set([Slot(14)]);
    hover(app, [19.5, 13.5]);
}

/// The pointer round `centre` at `r` from 0° to `to` degrees, 15° a step: Döner merdiven's way (docs/adr/0709).
fn round(app: &mut App, centre: [f64; 2], r: f64, to: f64) {
    let steps = (to.abs() / 15.0).ceil() as i32;
    for k in 1..=steps {
        let a = (to * f64::from(k) / f64::from(steps)).to_radians();
        hover(app, [centre[0] + r * a.cos(), centre[1] + r * a.sin()]);
    }
}

/// docs/adr/0709: Döner merdiven in bay A–B / 1–2: its centre and its walking line's start 1 m off clicked, the pointer
/// gone round three quarters; 3.06 m in 18 risers along its arc, radial, railings on both sides, the cut 1.10 m up.
fn spiral_tool(app: &mut App) {
    grids(app);
    run(app, "tool.stairCurved");
    typed(app, "Y");
    typed(app, "3.06");
    for _ in 0..3 {
        typed(app, "O");
    }
    click(app, [3.0, 2.5]);
    click(app, [4.0, 2.5]);
    close_on(app, [-0.5, -0.2], [7.0, 5.2]);
    round(app, [3.0, 2.5], 1.0, 250.0);
}

/// Two written: counter-clockwise with railings and numbers in bay A–B / 1–2, clockwise 1.40 m out in bay C–D / 1–2;
/// the first selected: Biçim, Yarıçap and İç kenarda basamak in Öznitelikler.
fn spiral_written(app: &mut App) {
    spiral_tool(app);
    click(app, [3.0, 1.5]);
    typed(app, "N");
    typed(app, "G");
    typed(app, "1.1");
    click(app, [14.25, 2.5]);
    click(app, [15.65, 2.5]);
    round(app, [14.25, 2.5], 1.4, -200.0);
    click(app, [13.0376, 3.2]);
    typed(app, "");
    app.selection.set([Slot(8)]);
    close_on(app, [-0.5, -0.5], [17.5, 5.5]);
    hover(app, [17.2, -0.2]);
}

/// Its start 0.40 m from its centre for a 1.00 m wide stair: the tag says why in red, nothing is written.
fn spiral_tight(app: &mut App) {
    grids(app);
    run(app, "tool.stairCurved");
    click(app, [3.0, 2.5]);
    click(app, [3.4, 2.5]);
    close_on(app, [-0.5, -0.2], [7.0, 5.2]);
    round(app, [3.0, 2.5], 0.4, 120.0);
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
                layer_id: None,
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
        ("merdiven-yukseklik", stair_height),
        ("merdiven-l", stair_l),
        ("merdiven-u", stair_u),
        ("merdiven-u-yazildi", stair_u_written),
        ("merdiven-sigmaz", stair_no_fit),
        ("rampa-egim", ramp_slope),
        ("rampa-l-korkuluk", ramp_l_railing),
        ("merdiven-korkuluk", stair_railings),
        ("rampa-sigmaz", ramp_no_fit),
        ("rampa", ramp_tool),
        ("aks-numarala", numbering),
        ("aks-agi", grid_system),
        ("aks-agi-dondur", grid_system_turned),
        ("aks-agi-yazildi", grid_system_written),
        ("aks-izleme", following),
        ("doner-merdiven", spiral_tool),
        ("doner-merdiven-yazildi", spiral_written),
        ("doner-merdiven-dar", spiral_tight),
    ]
}

/// Mimari's Merdiven ▾ open: Merdiven and Döner merdiven (docs/adr/0709).
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![(
        "doner-merdiven-serit",
        |app| {
            spiral_written(app);
            // No selection: Mimari stays in front instead of Mimari nesne.
            app.selection.set([]);
            app.tab = "arch";
        },
        |s, app| open_split(s, app, "archStairs"),
    )]
}

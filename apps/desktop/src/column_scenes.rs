//! The pictures of docs/adr/0712 (Kolonun geçici ölçüleri): a 7 × 5 m room of Dış duvar 25 with two columns; the west
//! column selected, its four clear distances drawn across the gaps (to the walls' faces, to the other column's side);
//! a click on the west number, its text field; 1.20 typed, the column moved, its distances anew; the same near enough
//! to read; a column of an Aks ağı tied to its grids, a click on its number saying why it does not move.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kolon-mesafe,kolon-mesafe-yazi-kutusu,kolon-mesafe-kaydirildi,kolon-mesafe-yakin,kolon-mesafe-aksa-bagli cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::{fit, opened, wall_tool};

/// A 7 × 5 m room of Dış duvar 25 (slot 1), Kolon's columns at (2, 1.6) and (5.2, 1.6) (slots 2 and 3), typed so no
/// snap moves them; the west one selected, the pointer in the room's north.
fn room(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    fit(app, 7.0, 5.0);
    app.tab = "arch";
    wall_tool(app, "Dış duvar 25");
    for p in [[0.0, 0.0], [7.0, 0.0], [7.0, 5.0], [0.0, 5.0]] {
        click(app, p);
    }
    typed(app, "K");
    run(app, "tool.column");
    for (x, y) in [(2.0, 1.6), (5.2, 1.6)] {
        typed(app, &format!("{},{}", E + x, N + y));
    }
    typed(app, "");
    app.selection.set([Slot(2)]);
    hover(app, [3.6, 3.9]);
}

/// The west number: the leftmost.
fn west(app: &App) -> kentos_interaction::Vec2 {
    app.session
        .temporary_numbers()
        .into_iter()
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .expect("a number")
}

/// A click on it: its text field, the value ready.
fn field(app: &mut App) {
    room(app);
    let n = west(app);
    click(app, [n.x - E, n.y - N]);
}

/// 1.20 typed, Enter: the column 1.20 from the west wall's face, its four distances anew.
fn written(app: &mut App) {
    field(app);
    let _ = app.update(Message::TextField(crate::text_field::Event::Input(
        "1.20".into(),
    )));
    let _ = app.update(Message::TextField(crate::text_field::Event::Keep));
    hover(app, [3.6, 3.9]);
}

/// The west half of the room near enough to read the values (1:50, their real 2.5 mm height).
fn near(app: &mut App) {
    room(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.6,
            min_y: N - 0.6,
            max_x: E + 6.0,
            max_y: N + 3.4,
        },
        24.0,
    );
    hover(app, [3.6, 2.9]);
}

/// docs/adr/0707's Aks ağı written with its columns; column B-2 selected, its distances to the columns round it; a
/// click on a number says it stands on its grids.
fn tied(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    crate::arch_scenes::on_level(app);
    fit(app, 18.0, 13.0);
    app.command_expanded = true;
    run(app, "tool.gridSystem");
    typed(app, "X");
    typed(app, "6 6 4.5");
    typed(app, "Y");
    typed(app, "2*5");
    typed(app, "K");
    click(app, [0.5, 1.0]);
    typed(app, "");
    // B-2: the second column of the second row.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let b2 = doc
        .entities_with_uids()
        .find_map(|(uid, e)| match e {
            kentos_commands::Entity::Column(c) if c.column.place.as_deref() == Some("B-2") => {
                doc.slot_of(uid)
            }
            _ => None,
        })
        .expect("column B-2");
    app.selection.set([b2]);
    hover(app, [9.0, 8.0]);
    let n = west(app);
    click(app, [n.x - E, n.y - N]);
    hover(app, [9.0, 8.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kolon-mesafe", room),
        ("kolon-mesafe-yazi-kutusu", field),
        ("kolon-mesafe-kaydirildi", written),
        ("kolon-mesafe-yakin", near),
        ("kolon-mesafe-aksa-bagli", tied),
    ]
}

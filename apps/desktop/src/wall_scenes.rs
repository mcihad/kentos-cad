//! The pictures of docs/adr/0339 (walls): CAD's Mimari tab; Duvar drawing a wall of KentOS's Dış duvar 25 (its
//! options in the command line, the faces and components from the command's plan under the cursor); a room drawn with
//! it, closed (K), and an inner wall, at 1:50 (standard: the components' hatches) and at 1:20 (detailed: the plaster
//! too); a wall selected with Öznitelikler's wall rows; Duvar stilleri with the section; Çizgiden duvar's Çiftler over
//! two pairs of lines; Proje ayarları › Mimari with two levels. `tools_screens` takes them in both themes at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=duvar-serit,duvar-ciz,duvar-yazildi,duvar-ayrintili,duvar-sematik,duvar-oznitelikler,duvar-stilleri,duvar-cizgiden,duvar-katlar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0340's: a room of four separate walls at mitred corners with an inner wall teeing in, at 1:50 and its
//! tee at 1:20; two walls crossing and a corner too sharp, its mark; the Mimari tab's Duvar düzenle; Duvar birleşimi
//! with an end picked; Birleşimleri denetle; Öznitelikler's join rows; Duvarı böl:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=duvar-birlesim,duvar-birlesim-ayrintili,duvar-birlesim-x,duvar-duzenle-serit,duvar-birlesim-araci,duvar-birlesim-denetle,duvar-birlesim-oznitelikler,duvar-bol cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0700's and 0699's: Duvar ölçüleri on one closed wall, the edge under the pointer, every edge at once,
//! written and taken back with one Geri al; a wall style renamed and its name given back by Geri al:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=olcu-kenar,olcu-butun-kenarlar,olcu-butun-kenarlar-yazildi,olcu-butun-kenarlar-geri-alindi,duvar-stili-adi-kaydedildi,duvar-stili-geri-alindi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0701's: the room's door selected, its temporary distances on the inner and the outer face, the text
//! field over a number, the door moved by it and taken back:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=gecici-olcu,gecici-olcu-dis,gecici-olcu-yazi-kutusu,gecici-olcu-tasindi,gecici-olcu-geri-alindi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! KENTOS_SHOTS_ONLY=duvar-mesafe,duvar-mesafe-yazi-kutusu,duvar-mesafe-kaydirildi,duvar-mesafe-oda,duvar-mesafe-oda-genisledi,duvar-mesafe-etki,duvar-mesafe-bag,duvar-mesafe-bag-etki,duvar-mesafe-bag-izledi cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture   # docs/adr/0710
//! KENTOS_SHOTS_ONLY=mimari-bolme-yay,mimari-bolme-yay-arasi cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture   # docs/adr/0734
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open_edited, run, typed};
use crate::tools_screens::Scene;

pub(crate) fn fit(app: &mut App, w: f64, h: f64) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.5,
            min_y: N - 1.5,
            max_x: E + w + 1.5,
            max_y: N + h + 1.5,
        },
        24.0,
    );
}

/// A CAD drawing at 1:`scale`, of `objects`.
pub(crate) fn opened(app: &mut App, objects: Objects, scale: f64) {
    open_edited(app, objects, |d| {
        d["settings"]["plotScale"] = json!(scale);
    });
    make_cad(app);
}

/// Duvar with KentOS's Dış duvar 25 chosen by name.
pub(crate) fn wall_tool(app: &mut App, style: &str) {
    app.command_expanded = true;
    run(app, "tool.wall");
    typed(app, "S");
    typed(app, style);
}

/// A 7 × 5 m room of Dış duvar 25, closed with K, and an inner wall of İç duvar 15 across it.
fn room(app: &mut App, scale: f64) {
    opened(app, Objects::new(), scale);
    fit(app, 7.0, 5.0);
    wall_tool(app, "Dış duvar 25");
    for p in [[0.0, 0.0], [7.0, 0.0], [7.0, 5.0], [0.0, 5.0]] {
        click(app, p);
    }
    typed(app, "K");
    wall_tool(app, "İç duvar 15");
    click(app, [4.2, 0.0]);
    click(app, [4.2, 5.0]);
    // Enter writes the wall, the next leaves the tool.
    typed(app, "");
    typed(app, "");
    hover(app, [9.0, -1.0]);
}

/// CAD's Mimari tab over the room.
fn ribbon(app: &mut App) {
    room(app, 50.0);
    app.tab = "arch";
}

/// Duvar drawing: three corners clicked, the cursor at the fourth, the wall's chips in the command line.
fn drawing(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    fit(app, 7.0, 5.0);
    wall_tool(app, "Dış duvar 25");
    click(app, [0.0, 0.0]);
    click(app, [7.0, 0.0]);
    click(app, [7.0, 5.0]);
    hover(app, [1.5, 5.0]);
}

fn written(app: &mut App) {
    room(app, 50.0);
}

/// At 1:20 the plaster shows too: the room's corner where the inner wall meets the outer one, near.
fn detailed(app: &mut App) {
    room(app, 20.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + 3.0,
            min_y: N - 0.6,
            max_x: E + 7.6,
            max_y: N + 1.6,
        },
        24.0,
    );
    hover(app, [7.4, -0.4]);
}

/// At 1:200 a wall is its fill alone.
fn schematic(app: &mut App) {
    room(app, 200.0);
}

/// The room's outer wall selected: Öznitelikler's wall rows.
fn props(app: &mut App) {
    room(app, 50.0);
    app.selection.set([Slot(1)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Duvar stilleri over the room, the project's Dış duvar 25 chosen.
fn styles(app: &mut App) {
    room(app, 50.0);
    run(app, "arch.wallStyles");
}

/// Çizgiden duvar's Çiftler over two pairs of parallel lines, the walls to come from the command's plan.
fn from_lines(app: &mut App) {
    let mut o = Objects::new();
    o.line("cizim", [0.0, 0.0], [7.0, 0.0]);
    o.line("cizim", [0.0, 0.2], [7.0, 0.2]);
    o.line("cizim", [0.0, 3.0], [5.0, 3.0]);
    o.line("cizim", [0.0, 3.25], [5.0, 3.25]);
    o.line("cizim", [6.0, 1.0], [6.0, 4.0]);
    opened(app, o, 50.0);
    fit(app, 7.0, 4.5);
    app.command_expanded = true;
    app.selection
        .set([Slot(1), Slot(2), Slot(3), Slot(4), Slot(5)]);
    run(app, "tool.wallFromLines");
    typed(app, "Ç");
    hover(app, [6.6, -0.8]);
}

/// Proje ayarları › Mimari with two levels and the detail by the scale.
fn levels(app: &mut App) {
    room(app, 50.0);
    run(app, "arch.settings");
    let add = || crate::project::settings_message(crate::project::SettingsEvent::AddLevel);
    let _ = app.update(add());
    let _ = app.update(add());
}

/// One open wall of `style` from `a` to `b`, drawn with Duvar and written with Enter.
pub(crate) fn wall_between(app: &mut App, style: &str, a: [f64; 2], b: [f64; 2]) {
    wall_tool(app, style);
    click(app, a);
    click(app, b);
    typed(app, "");
    typed(app, "");
}

/// A 7 × 5 m room of four separate walls of Dış duvar 25 meeting at mitred corners (docs/adr/0340 §2), and an inner
/// wall of İç duvar 15 teeing into both long walls; the walls' cores pass the plaster.
pub(crate) fn joined_room(app: &mut App, scale: f64) {
    opened(app, Objects::new(), scale);
    fit(app, 7.0, 5.0);
    let c = [[0.0, 0.0], [7.0, 0.0], [7.0, 5.0], [0.0, 5.0]];
    for k in 0..4 {
        wall_between(app, "Dış duvar 25", c[k], c[(k + 1) % 4]);
    }
    wall_between(app, "İç duvar 15", [4.2, 0.0], [4.2, 5.0]);
    hover(app, [9.0, -1.0]);
}

fn joins_written(app: &mut App) {
    joined_room(app, 50.0);
}

/// At 1:20, the inner wall's tee into the outer wall near: its brick passes the plaster, the plaster lines broken.
fn joins_detailed(app: &mut App) {
    joined_room(app, 20.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + 3.4,
            min_y: N - 0.6,
            max_x: E + 5.0,
            max_y: N + 0.9,
        },
        24.0,
    );
    hover(app, [4.9, -0.5]);
}

/// Two walls crossing and a corner too sharp to mitre: the cross cleans up, the corner is said with its mark.
fn joins_cross(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    fit(app, 8.0, 5.0);
    wall_between(app, "İç duvar 15", [0.0, 2.0], [6.0, 2.0]);
    wall_between(app, "Dış duvar 25", [3.0, -0.5], [3.0, 4.5]);
    wall_between(app, "İç duvar 15", [5.0, 4.5], [8.0, 4.5]);
    wall_between(app, "İç duvar 15", [8.0, 4.5], [5.0625, 5.25]);
    hover(app, [9.0, -1.0]);
}

/// CAD's Mimari tab with Duvar düzenle over the joined room.
fn edit_ribbon(app: &mut App) {
    joined_room(app, 50.0);
    app.tab = "arch";
}

/// Duvar birleşimi with the room's top right corner picked: the joins in the command line.
fn joins_tool(app: &mut App) {
    joined_room(app, 50.0);
    app.command_expanded = true;
    run(app, "tool.wallJoins");
    click(app, [6.9, 5.0]);
    hover(app, [6.2, 4.2]);
}

/// The sharp corner's issue in Birleşimleri denetle.
fn joins_check(app: &mut App) {
    joins_cross(app);
    run(app, "arch.wallJoinCheck");
}

/// The room's bottom wall selected: Öznitelikler's joins rows.
fn joins_props(app: &mut App) {
    joined_room(app, 50.0);
    app.selection.set([Slot(1)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Duvarı böl: the room's bottom wall split at the door's place, the two parts joined inline.
fn split(app: &mut App) {
    joined_room(app, 50.0);
    app.command_expanded = true;
    run(app, "tool.wallSplit");
    click(app, [2.0, 0.0]);
    hover(app, [2.6, -0.4]);
}

/// The new rectangle's inner clear dimensions, its actual wall body and reference ring.
fn rectangle_drawing(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    fit(app, 4.0, 3.0);
    app.tab = "arch";
    app.command_expanded = true;
    run(app, "tool.wallRectangle");
    typed(app, "E");
    typed(app, "0.2m");
    click(app, [0.0, 0.0]);
    hover(app, [4.0, 3.0]);
}

fn rectangle_written(app: &mut App) {
    rectangle_drawing(app);
    click(app, [4.0, 3.0]);
    typed(app, "");
    hover(app, [5.0, 3.5]);
}

fn rectangle_dimensions(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    fit(app, 4.0, 3.0);
    app.tab = "arch";
    app.command_expanded = true;
    run(app, "tool.wallRectangle");
    typed(app, "R");
    typed(app, "Dış ölçü");
    click(app, [0.0, 0.0]);
    typed(app, "B");
    typed(app, "4m,3m");
    hover(app, [4.0, 3.0]);
}

/// The same active wall preview and options at the user's 18 px interface text size.
fn rectangle_large_text(app: &mut App) {
    let _ = app.settings.choose(&[("appearance.textSize", json!(18))]);
    app.apply_settings();
    rectangle_dimensions(app);
}

/// docs/adr/0700: one closed wall drawn with Dikdörtgen duvar, 8 × 6 m inside, 25 cm, a door on its bottom edge and a
/// window on its right, at 1:100; the camera far enough out for the dimension rows outside it.
fn closed_room(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 3.5,
            min_y: N - 3.5,
            max_x: E + 11.5,
            max_y: N + 9.5,
        },
        24.0,
    );
    app.tab = "arch";
    app.command_expanded = true;
    run(app, "tool.wallRectangle");
    typed(app, "E");
    typed(app, "0.25m");
    click(app, [0.0, 0.0]);
    click(app, [8.0, 6.0]);
    typed(app, "");
    run(app, "tool.door");
    click(app, [2.5, -0.1]);
    typed(app, "");
    run(app, "tool.window");
    click(app, [8.1, 3.5]);
    typed(app, "");
    app.selection.clear();
}

/// docs/adr/0701: the closed room with a 15 cm wall teeing into its bottom edge at 4 m.
fn room_with_partition(app: &mut App) {
    closed_room(app);
    wall_tool(app, "İç duvar 15");
    click(app, [4.0, 0.0]);
    click(app, [4.0, 3.0]);
    typed(app, "");
    typed(app, "");
    app.selection.clear();
}

/// The room's door: its slot.
fn the_door(app: &App) -> Slot {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities_with_uids()
        .find_map(|(uid, e)| match e {
            kentos_commands::Entity::Opening(o)
                if o.opening.form == kentos_commands::OpeningForm::Door =>
            {
                doc.slot_of(uid)
            }
            _ => None,
        })
        .expect("a door")
}

/// The door selected, the pointer in the room: its temporary distances on the inner face, to the corner and to the
/// tee's face.
fn opening_distances(app: &mut App) {
    room_with_partition(app);
    let door = the_door(app);
    app.selection.set([door]);
    hover(app, [2.0, 1.2]);
}

/// The pointer outside: the outer face's distances, to the corner and to the window.
fn opening_distances_outer(app: &mut App) {
    room_with_partition(app);
    let door = the_door(app);
    app.selection.set([door]);
    hover(app, [2.0, -1.4]);
}

/// A click on the corner's number: its text field, the value ready.
fn opening_distance_field(app: &mut App) {
    opening_distances(app);
    let n = app.session.temporary_numbers();
    click(app, [n[0].x - E, n[0].y - N]);
}

/// 1 m typed, Enter: the door moved, the corner fixed; its new distances.
fn opening_distance_written(app: &mut App) {
    opening_distance_field(app);
    let _ = app.update(Message::TextField(crate::text_field::Event::Input(
        "1 m".into(),
    )));
    let _ = app.update(Message::TextField(crate::text_field::Event::Keep));
    hover(app, [2.0, 1.2]);
}

/// One Geri al: the door back where it was.
fn opening_distance_undone(app: &mut App) {
    opening_distance_written(app);
    run(app, "edit.undo");
    hover(app, [2.0, 1.25]);
}

/// docs/adr/0710: the room's inner wall selected, the pointer beside it: its clear distances to the outer wall's inner
/// faces on both sides.
fn wall_distances(app: &mut App) {
    room(app, 50.0);
    app.selection.set([Slot(2)]);
    hover(app, [3.6, 2.5]);
}

/// A click on the west number: its text field, the value ready.
fn wall_distance_field(app: &mut App) {
    wall_distances(app);
    let n = app.session.temporary_numbers();
    let west = n
        .iter()
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .copied()
        .expect("a number");
    click(app, [west.x - E, west.y - N]);
}

/// 3.60 typed, Enter: the inner wall 40 cm west, its ends sliding along the outer wall; its new distances.
fn wall_distance_written(app: &mut App) {
    wall_distance_field(app);
    let _ = app.update(Message::TextField(crate::text_field::Event::Input(
        "3.60".into(),
    )));
    let _ = app.update(Message::TextField(crate::text_field::Event::Keep));
    hover(app, [3.2, 2.5]);
}

/// The outer wall selected, the pointer by its east edge inside: that edge's distance to the inner wall.
fn wall_distance_room(app: &mut App) {
    room(app, 50.0);
    app.selection.set([Slot(1)]);
    hover(app, [6.6, 2.5]);
}

/// 3.00 typed for the east edge: the room 40 cm wider, its corners sliding along its edges, the inner wall in place.
fn wall_distance_room_wider(app: &mut App) {
    wall_distance_room(app);
    let n = app.session.temporary_numbers();
    click(app, [n[0].x - E, n[0].y - N]);
    let _ = app.update(Message::TextField(crate::text_field::Event::Input(
        "3.00".into(),
    )));
    let _ = app.update(Message::TextField(crate::text_field::Event::Keep));
    hover(app, [7.0, 2.5]);
}

/// docs/adr/0770 §3: 3.60 typed in the west number's field, not yet kept: the card of what it would do.
fn wall_distance_effects(app: &mut App) {
    wall_distance_field(app);
    let _ = app.update(Message::TextField(crate::text_field::Event::Input(
        "3.60".into(),
    )));
}

/// The east padlock clicked: the inner wall's east distance kept as a bond, its padlock filled and named.
fn wall_distance_bonded(app: &mut App) {
    wall_distances(app);
    let numbers = app.session.temporary_numbers();
    let east = numbers
        .iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .copied()
        .expect("a number");
    let lock = app
        .session
        .temporary_squares()
        .into_iter()
        .min_by(|a, b| {
            (a.x - east.x)
                .hypot(a.y - east.y)
                .total_cmp(&(b.x - east.x).hypot(b.y - east.y))
        })
        .expect("a padlock");
    click(app, [lock.x - E, lock.y - N]);
    hover(app, [3.6, 2.5]);
}

/// The outer wall's east edge given 3.00 while the inner wall is bonded to it: the card says it follows.
fn wall_distance_bonded_effects(app: &mut App) {
    wall_distance_bonded(app);
    app.selection.set([Slot(1)]);
    hover(app, [6.6, 2.5]);
    let n = app.session.temporary_numbers();
    click(app, [n[0].x - E, n[0].y - N]);
    let _ = app.update(Message::TextField(crate::text_field::Event::Input(
        "3.00".into(),
    )));
}

/// The outer wall's east edge shifted 40 cm east (Kaydır, as a grip writes it): the bonded inner wall follows, its east
/// distance as it was.
fn wall_distance_bonded_followed(app: &mut App) {
    wall_distance_bonded(app);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let Some(kentos_commands::Entity::Wall(w)) = doc.get(Slot(1)) else {
        panic!("the outer wall")
    };
    let pts = &w.wall.pts;
    let n = pts.len();
    let east = (0..n)
        .max_by(|&a, &b| {
            (pts[a].x + pts[(a + 1) % n].x).total_cmp(&(pts[b].x + pts[(b + 1) % n].x))
        })
        .expect("an edge");
    // The edge's left is (-dy, dx): east is its left when dy < 0.
    let by = if pts[(east + 1) % n].y - pts[east].y < 0.0 {
        0.4
    } else {
        -0.4
    };
    let uid = doc.uid(Slot(1)).expect("an id").to_string();
    let input = kentos_commands::WallsEdit {
        uids: vec![uid],
        expected_revision: None,
        operation: kentos_commands::WallsOperation::Shift {
            edge: Some(east as u32),
            by,
        },
    };
    let done = kentos_native_application::walls_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        input,
    );
    assert!(
        matches!(done, kentos_commands::CommandResult::Completed { .. }),
        "{done:?}"
    );
    app.selection.set([Slot(2)]);
    hover(app, [4.4, 2.5]);
}

/// Duvar ölçüleri on the closed room: the right edge under the pointer, its outer face, the window's jambs.
fn dimensions_edge(app: &mut App) {
    closed_room(app);
    run(app, "tool.wallDimensions");
    click(app, [8.1, 1.0]);
    hover(app, [10.0, 2.0]);
}

/// docs/adr/0695's 27 October addendum: a wall without openings, Duvar ölçüleri from the ribbon, placed: one
/// dimension, not its length twice (the openings row and the overall row).
fn dimensions_plain_wall(app: &mut App) {
    opened(app, Objects::new(), 50.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.0,
            min_y: N - 3.0,
            max_x: E + 9.0,
            max_y: N + 3.0,
        },
        24.0,
    );
    app.tab = "arch";
    app.command_expanded = true;
    run(app, "tool.wall");
    click(app, [0.0, 0.0]);
    click(app, [8.0, 0.0]);
    typed(app, "");
    typed(app, "");
    app.selection.clear();
    run(app, "tool.wallDimensions");
    click(app, [4.0, 0.0]);
    click(app, [4.0, 1.2]);
    hover(app, [6.0, 2.2]);
}

/// Kenarlar → Bütün kenarlar: a chain on each of the four edges, on the outer faces, still a preview.
fn dimensions_around(app: &mut App) {
    closed_room(app);
    run(app, "tool.wallDimensions");
    click(app, [8.1, 1.0]);
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallDimensions",
        key: "N",
        value: "all".into(),
    });
    hover(app, [4.0, -1.4]);
}

/// The four chains written with one Enter, selected.
fn dimensions_around_written(app: &mut App) {
    dimensions_around(app);
    typed(app, "");
    hover(app, [11.0, 8.5]);
}

/// One Geri al takes all four chains back.
fn dimensions_around_undone(app: &mut App) {
    dimensions_around_written(app);
    run(app, "edit.undo");
}

/// docs/adr/0699: the room's style renamed in Duvar stilleri and saved; the wall selected shows the new name.
fn style_renamed(app: &mut App) {
    room(app, 50.0);
    run(app, "arch.wallStyles");
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Choose(
        Some("kentos-dis-duvar-25".into()),
    )));
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Name(
        "Dış duvar 25 QA".into(),
    )));
    let _ = app.update(Message::WallStyles(crate::wall_styles::Event::Save));
    app.selection.set([Slot(1)]);
}

/// One Geri al gives the old name back: the style table is in the step.
fn style_rename_undone(app: &mut App) {
    style_renamed(app);
    run(app, "edit.undo");
    app.selection.set([Slot(1)]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("gecici-olcu", opening_distances),
        ("gecici-olcu-dis", opening_distances_outer),
        ("gecici-olcu-yazi-kutusu", opening_distance_field),
        ("gecici-olcu-tasindi", opening_distance_written),
        ("gecici-olcu-geri-alindi", opening_distance_undone),
        ("duvar-mesafe", wall_distances),
        ("duvar-mesafe-yazi-kutusu", wall_distance_field),
        ("duvar-mesafe-kaydirildi", wall_distance_written),
        ("duvar-mesafe-oda", wall_distance_room),
        ("duvar-mesafe-oda-genisledi", wall_distance_room_wider),
        ("duvar-mesafe-etki", wall_distance_effects),
        ("duvar-mesafe-bag", wall_distance_bonded),
        ("duvar-mesafe-bag-etki", wall_distance_bonded_effects),
        ("duvar-mesafe-bag-izledi", wall_distance_bonded_followed),
        ("olcu-kenar", dimensions_edge),
        ("olcu-aciksiz-duvar", dimensions_plain_wall),
        ("olcu-butun-kenarlar", dimensions_around),
        ("olcu-butun-kenarlar-yazildi", dimensions_around_written),
        ("olcu-butun-kenarlar-geri-alindi", dimensions_around_undone),
        ("duvar-stili-adi-kaydedildi", style_renamed),
        ("duvar-stili-geri-alindi", style_rename_undone),
        ("mimari-serit-13", architecture_ribbon),
        ("mimari-serit-18", architecture_ribbon_large),
        ("mimari-duvar-baglam-18", architecture_wall_context),
        ("mimari-aciklik-baglam-18", architecture_opening_context),
        ("mimari-mahal-baglam-18", architecture_space_context),
        ("mimari-arac-ayarlari", architecture_options),
        ("mimari-bolme-onizleme", partition_preview),
        ("mimari-bolme-buyuk-yazi", partition_large_text),
        ("mimari-bolme-yay", partition_arc),
        ("mimari-bolme-yay-arasi", partition_arc_between),
        ("duvar-dikdortgen-onizleme", rectangle_drawing),
        ("duvar-dikdortgen-yazildi", rectangle_written),
        ("duvar-dikdortgen-boyutlar", rectangle_dimensions),
        ("duvar-dikdortgen-buyuk-yazi", rectangle_large_text),
        ("duvar-birlesim", joins_written),
        ("duvar-birlesim-ayrintili", joins_detailed),
        ("duvar-birlesim-x", joins_cross),
        ("duvar-duzenle-serit", edit_ribbon),
        ("duvar-birlesim-araci", joins_tool),
        ("duvar-birlesim-denetle", joins_check),
        ("duvar-birlesim-oznitelikler", joins_props),
        ("duvar-bol", split),
        ("duvar-serit", ribbon),
        ("duvar-ciz", drawing),
        ("duvar-yazildi", written),
        ("duvar-ayrintili", detailed),
        ("duvar-sematik", schematic),
        ("duvar-oznitelikler", props),
        ("duvar-stilleri", styles),
        ("duvar-cizgiden", from_lines),
        ("duvar-katlar", levels),
    ]
}

fn architecture_options(app: &mut App) {
    rectangle_drawing(app);
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallRectangle",
        key: "E",
        value: "250mm".into(),
    });
}
fn partition_preview(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    fit(app, 8.0, 6.0);
    app.tab = "arch";
    run(app, "tool.wallRectangle");
    typed(app, "B");
    typed(app, "8m,6m");
    click(app, [0.0, 0.0]);
    click(app, [8.0, 6.0]);
    typed(app, "");
    run(app, "tool.wallPartition");
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallPartition",
        key: "E",
        value: "150mm".into(),
    });
    click(app, [4.0, 0.0]);
    hover(app, [4.0, 3.0]);
}
/// A quarter ring of 25 cm wall of radius 8 m about the plan's corner and two radial walls at 15° and 75°, with
/// Ölçülü bölme started at 150 mm (docs/adr/0734).
fn curved(app: &mut App) {
    let mut o = Objects::new();
    let quarter = (std::f64::consts::PI / 8.0).tan();
    let at = |r: f64, deg: f64| {
        crate::furniture_scenes::xy(r * deg.to_radians().cos(), r * deg.to_radians().sin())
    };
    o.push(
        "duvar",
        json!({"kind": "wall", "pts": [at(8.0, 0.0), at(8.0, 90.0)], "bulges": [quarter, 0.0], "width": 0.25,
               "justify": "center"}),
    );
    for deg in [15.0, 75.0] {
        o.push(
            "duvar",
            json!({"kind": "wall", "pts": [at(2.0, deg), at(10.0, deg)], "width": 0.15, "justify": "center"}),
        );
    }
    open_edited(app, o, |d| {
        d["layers"] = json!([crate::tools_scenes::layer("duvar", "Duvar", "fg", 0.5)]);
        d["activeLayer"] = json!("duvar");
        d["settings"]["plotScale"] = json!(100.0);
    });
    make_cad(app);
    fit(app, 10.0, 10.0);
    app.tab = "arch";
    app.command_expanded = true;
    run(app, "tool.wallPartition");
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallPartition",
        key: "E",
        value: "150mm".into(),
    });
}
/// The arc picked as the source, the pointer inside: the concentric partition and its clear distance.
fn partition_arc(app: &mut App) {
    curved(app);
    let m = 8.0 * std::f64::consts::FRAC_1_SQRT_2;
    click(app, [m, m]);
    hover(app, [5.5, 0.6]);
}
/// İki duvar arasında: the arc, then the two radial walls; the partition written between them.
fn partition_arc_between(app: &mut App) {
    curved(app);
    let _ = app.tool_setting(crate::architecture_settings::Commit {
        tool: "wallPartition",
        key: "B",
        value: "between".into(),
    });
    let m = 8.0 * std::f64::consts::FRAC_1_SQRT_2;
    click(app, [m, m]);
    for deg in [15.0f64, 75.0] {
        click(
            app,
            [6.0 * deg.to_radians().cos(), 6.0 * deg.to_radians().sin()],
        );
    }
    // Placed on the inside: the written wall between the two radial walls.
    click(app, [5.5, 0.6]);
    hover(app, [11.0, 10.0]);
}
fn partition_large_text(app: &mut App) {
    let _ = app.settings.choose(&[("appearance.textSize", json!(18))]);
    app.apply_settings();
    partition_preview(app);
}

fn architecture_ribbon(app: &mut App) {
    crate::opening_scenes::room(app, 100.0);
    app.tab = "arch";
}
fn architecture_ribbon_large(app: &mut App) {
    let _ = app.settings.choose(&[("appearance.textSize", json!(18))]);
    app.apply_settings();
    architecture_ribbon(app);
}
fn architecture_wall_context(app: &mut App) {
    architecture_ribbon_large(app);
    app.selection.set([Slot(1)]);
    app.ribbon_context = true;
}
fn architecture_opening_context(app: &mut App) {
    architecture_ribbon_large(app);
    app.selection.set([Slot(6)]);
    app.ribbon_context = true;
}
fn architecture_space_context(app: &mut App) {
    let _ = app.settings.choose(&[("appearance.textSize", json!(18))]);
    app.apply_settings();
    crate::space_scenes::spaces(app);
    app.selection.set([Slot(12)]);
    app.ribbon_context = true;
}

#[cfg(test)]
mod bond_tests {
    use super::*;

    /// The sample drawing with its drawing area 1100 × 600 (as the screens' window lays it out).
    fn sized() -> App {
        let mut app = crate::files_testing::app_with_drawing();
        app.viewport.camera.set_size(1100.0, 600.0);
        app
    }

    fn inner_x(app: &App) -> f64 {
        match app.document.as_ref().expect("a drawing").model.get(Slot(2)) {
            Some(kentos_commands::Entity::Wall(w)) => w.wall.pts[0].x,
            _ => panic!("the inner wall"),
        }
    }

    /// docs/adr/0770 §3: the padlock keeps the distance as a bond without moving anything; the outer wall shifted, the
    /// bonded inner wall follows it; one undo takes both back; a second click on the padlock lets the bond go.
    #[test]
    fn the_padlock_bonds_and_the_bonded_wall_follows() {
        let mut app = sized();
        wall_distance_bonded(&mut app);
        let set = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .constraints()
            .clone();
        assert_eq!(set.constraints.len(), 1, "one bond");
        assert_eq!(set.parameters[0].name, "d1");
        let before = inner_x(&app);
        let mut app = sized();
        wall_distance_bonded_followed(&mut app);
        assert!(
            (inner_x(&app) - before - 0.4).abs() < 1e-9,
            "{} {before}",
            inner_x(&app)
        );
        run(&mut app, "edit.undo");
        assert!(
            (inner_x(&app) - before).abs() < 1e-9,
            "one undo takes the follow back"
        );
        hover(&mut app, [3.6, 2.5]);
        // The padlock again: the bond goes, nothing moves.
        let numbers = app.session.temporary_numbers();
        let east = numbers
            .iter()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .copied()
            .expect("a number");
        let lock = app
            .session
            .temporary_squares()
            .into_iter()
            .min_by(|a, b| {
                (a.x - east.x)
                    .hypot(a.y - east.y)
                    .total_cmp(&(b.x - east.x).hypot(b.y - east.y))
            })
            .expect("a padlock");
        click(&mut app, [lock.x - E, lock.y - N]);
        let model = &app.document.as_ref().expect("a drawing").model;
        assert!(
            model.constraints().constraints.is_empty(),
            "the bond let go"
        );
        assert!((inner_x(&app) - before).abs() < 1e-9);
    }

    /// Typing in the field shows what the value would do, before Enter; nothing is written.
    #[test]
    fn typing_shows_the_effects_and_writes_nothing() {
        let mut app = sized();
        wall_distance_effects(&mut app);
        let before = inner_x(&app);
        let tag = app
            .session
            .preview(&app.format())
            .and_then(|p| p.tag)
            .expect("the card");
        assert!(
            tag.lines.iter().any(|l| l.starts_with("Bu yan mesafe:")),
            "{:?}",
            tag.lines
        );
        assert!(
            tag.lines.iter().any(|l| l.starts_with("Karşı yan mesafe:")),
            "{:?}",
            tag.lines
        );
        assert_eq!(inner_x(&app), before, "nothing written");
    }
}

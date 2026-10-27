//! The pictures of docs/adr/0722 (Döşeme): the walkthrough's apartment (12 × 9 m clear, 25 cm shell, two partitions,
//! three rooms and their doors). Döşeme's preview from the walls' outer faces with its settings; the floor written and
//! a balcony drawn north of the plan with Korkuluk (its free sides only); Delik cutting the north-east wet room out of
//! the floor, then its own slab 5 cm lower; the slabs written and the balcony selected (Öznitelikler and the Döşeme
//! tab); an edge offset that turns the outline over; Metraj's Döşemeler, Mahaller (Süpürgelik) and Geçişler tabs.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=doseme-arac,doseme-balkon-onizleme,doseme-delik,doseme-islak-hacim,doseme-yazildi,doseme-oteleme-hata,doseme-metraj,doseme-metraj-mahaller,doseme-metraj-gecisler cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::architecture_settings::Commit;
use crate::tools_scenes::{E, N, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::walkthrough_scenes::{rooms, view};

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

/// A corner typed on the command line (east, north): a click would snap to the walls' corners and the windows' jambs.
fn corner(app: &mut App, p: [f64; 2]) {
    typed(app, &format!("{},{}", E + p[0], N + p[1]));
}

fn setting(app: &mut App, key: &'static str, value: &str) {
    let _ = app.tool_setting(Commit { tool: "slab", key, value: value.into() });
}

/// The apartment's rooms found, closer than the walkthrough's view and with room north of it for the balcony; Mimari
/// in front.
fn apartment(app: &mut App) {
    rooms(app);
    view(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E - 1.5, min_y: N - 1.5, max_x: E + 13.5, max_y: N + 11.5 },
        24.0,
    );
    app.tab = "arch";
    app.command_expanded = true;
    app.selection.clear();
}

/// Döşeme started (Dış yüzler, 15 cm), the pointer in the west room: the outline of the walls' outer faces and its area.
fn tool(app: &mut App) {
    apartment(app);
    run(app, "tool.slab");
    hover(app, [2.0, 6.0]);
}

/// The floor written; Balkon with Korkuluk, its corners drawn north of the plan: the railing along its three free sides.
fn balcony_preview(app: &mut App) {
    tool(app);
    click(app, [2.0, 6.0]);
    setting(app, "T", "balcony");
    setting(app, "E", "railing");
    setting(app, "S", "draw");
    corner(app, [4.5, 9.25]);
    corner(app, [10.5, 9.25]);
    corner(app, [10.5, 10.75]);
    hover(app, [4.5, 10.75]);
}

/// The balcony written; Delik on, the pointer in the north-east room (the wet room): the hole the floor would lose.
fn hole(app: &mut App) {
    balcony_preview(app);
    corner(app, [4.5, 10.75]);
    typed(app, "");
    setting(app, "T", "floor");
    setting(app, "E", "none");
    setting(app, "S", "room");
    setting(app, "D", "yes");
    hover(app, [8.0, 7.0]);
}

/// The hole cut; Delik off, Oda, Kot farkı −5 cm and its material: the wet room's own lower slab.
fn wet_room(app: &mut App) {
    hole(app);
    click(app, [8.0, 7.0]);
    setting(app, "D", "no");
    setting(app, "F", "-5cm");
    setting(app, "M", "Su yalıtımlı şap");
    hover(app, [8.0, 7.2]);
}

/// The slab id of the role given, the newest.
fn slab_of(app: &App, role: kentos_commands::SlabRole) -> Option<u32> {
    app.document.as_ref().and_then(|d| {
        d.model
            .entities()
            .filter(|e| matches!(e, Entity::Slab(s) if s.slab.role == role))
            .last()
            .map(|e| e.base().id)
    })
}

/// Everything written, the tool closed and the balcony selected: Öznitelikler's slab rows and the Döşeme tab.
fn written(app: &mut App) {
    wet_room(app);
    click(app, [8.0, 7.2]);
    app.cancel();
    app.cancel();
    if let Some(id) = slab_of(app, kentos_commands::SlabRole::Balcony) {
        app.selection.set([Slot(id)]);
    }
    app.ribbon_context = true;
    hover(app, [14.0, 11.0]);
}

/// Kenar ötelemesi −5 m from the walls' outline: the outline turns over, the tag says so in red and nothing is written.
fn offset_failure(app: &mut App) {
    tool(app);
    setting(app, "Ö", "-5m");
    hover(app, [2.0, 6.0]);
}

/// The rooms' floors (laminate west and south-east, ceramic north-east: the wet room) and Metraj opened.
fn measured(app: &mut App, tab: crate::quantities::Tab) {
    wet_room(app);
    click(app, [8.0, 7.2]);
    app.cancel();
    app.cancel();
    let rooms: Vec<(String, f64, f64)> = {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities()
            .filter_map(|e| match e {
                Entity::Space(s) => {
                    let p = kentos_native_application::spaces::label_point(&s.space.parts)?;
                    doc.uid(Slot(e.base().id)).map(|u| (u.to_string(), p.x, p.y))
                }
                _ => None,
            })
            .collect()
    };
    for (uid, x, y) in rooms {
        let (x, y) = (x - E, y - N);
        let floor = if x > 4.2 && y > 5.2 { "Seramik" } else { "Laminat" };
        let name = if x < 4.0 { "Salon" } else if y > 5.2 { "Banyo" } else { "Yatak odası" };
        command(app, "cad.spaces.edit", json!({ "uids": [uid], "operation": { "kind": "set", "floor": floor, "name": name } }));
    }
    app.selection.clear();
    run(app, "arch.quantities");
    let _ = app.update(Message::Quantities(crate::quantities::Event::Tab(tab)));
}

fn quantities(app: &mut App) {
    measured(app, crate::quantities::Tab::Slabs);
}

fn quantities_rooms(app: &mut App) {
    measured(app, crate::quantities::Tab::Spaces);
}

fn quantities_transitions(app: &mut App) {
    measured(app, crate::quantities::Tab::Transitions);
}

/// The balcony selected, Öznitelikler scrolled to its slab rows: Tür, Kalınlık, Kot farkı, Üst kot, Kenar, Kenar eni,
/// Kenar uzunluğu, Malzeme, Kat, Evre, Metraj, Alan, Çevre, Hacim.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![("doseme-oznitelik", written, |s, app| {
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Genel", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -7.0));
    })]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("doseme-arac", tool),
        ("doseme-balkon-onizleme", balcony_preview),
        ("doseme-delik", hole),
        ("doseme-islak-hacim", wet_room),
        ("doseme-yazildi", written),
        ("doseme-oteleme-hata", offset_failure),
        ("doseme-metraj", quantities),
        ("doseme-metraj-mahaller", quantities_rooms),
        ("doseme-metraj-gecisler", quantities_transitions),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    /// The walkthrough through the tool, as the pictures take it: the floor, the balcony's railing on its free sides,
    /// the wet room cut out and its own lower slab; Metraj sums them and finds the one floor change (docs/adr/0722).
    #[test]
    fn the_slab_tool_writes_the_floor_the_balcony_and_the_wet_room() {
        let mut app = app_with_drawing();
        wet_room(&mut app);
        click(&mut app, [8.0, 7.2]);
        let doc = &app.document.as_ref().expect("a drawing").model;
        let slabs: Vec<&kentos_commands::SlabFields> = doc
            .entities()
            .filter_map(|e| match e {
                Entity::Slab(s) => Some(&s.slab),
                _ => None,
            })
            .collect();
        assert_eq!(slabs.len(), 3, "the floor, the balcony and the wet room");
        let area = |s: &kentos_commands::SlabFields| kentos_native_application::spaces::area_of(&s.parts);
        // The floor: the shell's outer faces, 12.5 × 9.5, less the wet room (7.85 × 3.85 clear).
        let wet = 7.85 * 3.85;
        // The walls were drawn with the pointer: their corners carry its rounding (a few micrometres).
        assert!((area(slabs[0]) - (12.5 * 9.5 - wet)).abs() < 1e-4, "floor {}", area(slabs[0]));
        assert_eq!(slabs[0].parts[0].holes.len(), 1);
        // The balcony: 6 × 1.5, its railing along its three free sides (the side on the wall's face is not free).
        assert!((area(slabs[1]) - 9.0).abs() < 1e-9, "{:?}", slabs[1].parts);
        assert!((kentos_interaction::slab_tools::edge_length(slabs[1]) - 9.0).abs() < 1e-9);
        assert_eq!(slabs[1].role, kentos_commands::SlabRole::Balcony);
        // The wet room's own slab, 5 cm lower, its material written.
        assert!((area(slabs[2]) - wet).abs() < 1e-4, "{} {:?}", area(slabs[2]), slabs[2]);
        assert!((slabs[2].offset + 0.05).abs() < 1e-12);
        assert_eq!(slabs[2].material.as_deref(), Some("Su yalıtımlı şap"));
    }
}

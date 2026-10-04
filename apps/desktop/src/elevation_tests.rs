//! Kot ver (docs/adr/0142) through the real app: the ribbon's commands and
//! the prompt's chips reach the tool, what it says lands in the message log,
//! and what it writes is one named undo step. The tool's own mechanics are
//! `crates/native/interaction/tests/all/elevating.rs`. Test code only.

use iced::{Point, Rectangle, Size};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::Level;

use crate::app::{App, Message};
use crate::catalog::{Standing, catalog};
use crate::elevation_scenes::{BARE_SPOT, CIRCLE, FOOTPATH, KERB, ROAD, SPOT, elevation_ground};
use crate::files_testing::{app_with_drawing as sample, last_said};
use crate::tools_scenes::{open, run, typed};
use crate::viewport::Event;

/// The sample opened on a drawing area of 1000 × 800 logical pixels.
fn app() -> App {
    let mut app = sample();
    let area = Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0));
    let _ = app.update(Message::Viewport(Event::Resized(area)));
    app
}

fn chip(app: &mut App, key: &'static str) {
    let _ = app.update(Message::PromptOption(key));
}

fn prompt(app: &App) -> String {
    app.session.prompt().text()
}

/// The lines said since `mark` (a log length).
fn said(app: &App, mark: usize) -> Vec<String> {
    app.log.lines().skip(mark).map(|l| l.text.clone()).collect()
}

fn entity(app: &App, slot: Slot) -> &Entity {
    app.document
        .as_ref()
        .expect("open")
        .model
        .get(slot)
        .expect("the object")
}

fn line_z(app: &App, slot: Slot) -> (Option<f64>, Option<f64>) {
    match entity(app, slot) {
        Entity::Line(l) => (l.za, l.zb),
        other => panic!("{other:?}"),
    }
}

fn path_z(app: &App, slot: Slot) -> Option<Vec<Option<f64>>> {
    match entity(app, slot) {
        Entity::Polyline(p) | Entity::Polygon(p) => p.zs.clone(),
        other => panic!("{other:?}"),
    }
}

fn point_z(app: &App, slot: Slot) -> Option<f64> {
    match entity(app, slot) {
        Entity::Point(p) => p.z,
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_ribbons_command_starts_kot_ver_and_the_log_names_it() {
    let mut app = app();
    assert_eq!(
        catalog().get("tool.setElevation").map(|c| c.standing),
        Some(Standing::Ported)
    );
    assert!(app.available("tool.setElevation"));
    run(&mut app, "tool.setElevation");
    assert_eq!(app.session.tool_id(), "setElevation");
    assert_eq!(app.log.last().map(|l| l.level), Some(Level::Command));
    assert_eq!(last_said(&app), "Kot ver");
    assert_eq!(
        prompt(&app),
        "Kot ver: kot verilecek nesneleri seçin [Artır (A) / Sıfırla (S)]"
    );
}

#[test]
fn its_names_in_the_command_line_start_it() {
    for name in ["KOTVER", "ktv", "SetElevation"] {
        let mut app = app();
        let _ = app.submit_line(name);
        assert_eq!(app.session.tool_id(), "setElevation", "{name}");
    }
}

#[test]
fn a_typed_number_gives_the_selection_its_elevation_in_one_named_undo_step() {
    let mut app = app();
    open(&mut app, elevation_ground());
    // The kerb, a survey point without an elevation, and the circle that takes none.
    app.selection.set([KERB, BARE_SPOT, CIRCLE]);
    run(&mut app, "tool.setElevation");
    assert_eq!(
        prompt(&app),
        "Kot ver: kotu yazın (m) [Artır (A) / Sıfırla (S)]"
    );
    typed(&mut app, "100.5");
    let n = app.log.len();
    assert_eq!(
        said(&app, n - 2),
        [
            "Kot verildi: 2 nesne.",
            "1 nesne kot almaz (yalnız çizgi, çoklu çizgi, alan ve nokta)."
        ]
    );
    assert_eq!(app.log.last().map(|l| l.level), Some(Level::Warn));
    assert_eq!(path_z(&app, KERB), Some(vec![Some(100.5); 4]));
    assert_eq!(point_z(&app, BARE_SPOT), Some(100.5));
    // Back at the first step with nothing selected.
    assert!(app.selection.is_empty());
    assert_eq!(
        prompt(&app),
        "Kot ver: kot verilecek nesneleri seçin [Artır (A) / Sıfırla (S)]"
    );
    // One undo step, by the tool's name.
    run(&mut app, "edit.undo");
    assert_eq!(last_said(&app), "Geri alındı: Kot ver");
    assert_eq!(
        path_z(&app, KERB),
        Some(vec![Some(98.5), Some(101.25), Some(104.0), Some(105.25)])
    );
    assert_eq!(point_z(&app, BARE_SPOT), None);
}

#[test]
fn the_chips_are_options_of_the_prompt_artir_a_toggle_and_sifirla_at_once() {
    let mut app = app();
    open(&mut app, elevation_ground());
    app.selection.set([ROAD, SPOT]);
    run(&mut app, "tool.setElevation");
    chip(&mut app, "A");
    assert_eq!(
        prompt(&app),
        "Kot ver: eklenecek farkı yazın (m) [Artır (A): açık / Sıfırla (S)]"
    );
    typed(&mut app, "-1.5");
    assert_eq!(last_said(&app), "Kotlar −1.500 m değişti: 2 nesne.");
    assert_eq!(line_z(&app, ROAD), (Some(98.5), Some(107.5)));
    assert_eq!(point_z(&app, SPOT), Some(99.8));
    // Back at picking with nothing selected; Sıfırla with objects picked writes at once.
    assert!(app.selection.is_empty());
    app.selection.set([ROAD, SPOT]);
    chip(&mut app, "S");
    assert_eq!(last_said(&app), "Kot silindi: 2 nesne.");
    assert_eq!(line_z(&app, ROAD), (None, None));
    assert_eq!(point_z(&app, SPOT), None);
}

/// The app with the elevation ground open.
fn app_after_picking() -> App {
    let mut app = app();
    open(&mut app, elevation_ground());
    app
}

/// The ribbon's methods each start the tool the way they say: Artır on, Sıfırla at once.
#[test]
fn the_ribbons_methods_start_it_the_way_they_say() {
    let mut app = app_after_picking();
    app.selection.set([FOOTPATH]);
    let _ = app.update(Message::RunMethod {
        id: "tool.setElevation",
        option: "A",
        label: "Artır",
    });
    assert_eq!(
        prompt(&app),
        "Kot ver: eklenecek farkı yazın (m) [Artır (A): açık / Sıfırla (S)]"
    );
    // Sabit is the button itself: a fresh run, Artır off.
    run(&mut app, "tool.cancel");
    run(&mut app, "tool.cancel");
    run(&mut app, "tool.setElevation");
    assert_eq!(
        prompt(&app),
        "Kot ver: kotu yazın (m) [Artır (A) / Sıfırla (S)]"
    );
    // Sıfırla with a selection resets it.
    run(&mut app, "tool.cancel");
    run(&mut app, "tool.cancel");
    app.selection.set([FOOTPATH, ROAD]);
    let _ = app.update(Message::RunMethod {
        id: "tool.setElevation",
        option: "S",
        label: "Sıfırla",
    });
    assert_eq!(last_said(&app), "Kot silindi: 2 nesne.");
    assert_eq!(path_z(&app, FOOTPATH), None);
    assert_eq!(line_z(&app, ROAD), (None, None));
}

#[test]
fn esc_steps_back_from_the_number_to_the_picking_and_then_leaves() {
    let mut app = app_after_picking();
    app.selection.set([ROAD]);
    run(&mut app, "tool.setElevation");
    run(&mut app, "tool.cancel");
    assert_eq!(app.session.tool_id(), "setElevation");
    assert!(prompt(&app).contains("kot verilecek nesneleri seçin"));
    assert_eq!(app.selection.len(), 1, "the selection stays");
    run(&mut app, "tool.cancel");
    assert_eq!(app.session.tool_id(), "select");
}

#[test]
fn a_locked_layer_refuses_in_the_commands_own_words_and_the_tool_stays_at_the_number() {
    let mut app = app_after_picking();
    let doc = app.document.as_mut().expect("open");
    doc.model.toggle_layer_locked("yol");
    app.selection.set([ROAD]);
    run(&mut app, "tool.setElevation");
    typed(&mut app, "5");
    assert_eq!(
        last_said(&app),
        "“Yol” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
    );
    assert_eq!(line_z(&app, ROAD), (Some(100.0), Some(109.0)));
    assert!(prompt(&app).starts_with("Kot ver: kotu yazın (m)"));
}

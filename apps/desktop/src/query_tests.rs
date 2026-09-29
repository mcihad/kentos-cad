//! docs/adr/0141's queries and selections through the real app: the ribbon's
//! commands and the prompt's chips reach the tools, what they say lands in
//! the message log, and what they write is one named undo step. The tools'
//! own mechanics are `crates/native/interaction/tests`. Test code only.

use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{Key, Modifiers};
use iced::{Point, Rectangle, Size};
use kentos_domain::Slot;
use kentos_interaction::Level;

use crate::app::{App, Message};
use crate::catalog::{Standing, catalog};
use crate::files_testing::{app_with_drawing as sample, last_said};
use crate::keys::KeyPress;
use crate::tools_scenes::{
    click, far_ground, islands_ground, open, run, station_ground, survey_ground, typed,
};
use crate::viewport::Event;

/// The sample opened on a drawing area of 1000 × 800 logical pixels: what the visible
/// line work is (İçine tıkla) and what a click picks depend on the view.
fn app_with_drawing() -> App {
    let mut app = sample();
    let area = Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0));
    let _ = app.update(Message::Viewport(Event::Resized(area)));
    app
}

fn chip(app: &mut App, key: &'static str) {
    let _ = app.update(Message::PromptOption(key));
}

/// The lines said since `mark` (a log length), without the echoes of taken points.
fn said(app: &App, mark: usize) -> Vec<String> {
    app.log
        .lines()
        .skip(mark)
        .map(|l| l.text.clone())
        .filter(|text| !text.trim_start().starts_with("Y "))
        .collect()
}

/// The prompt as the command line shows it.
fn prompt(app: &App) -> String {
    app.session.prompt().text()
}

#[test]
fn the_ribbons_command_starts_dik_ayak_olc_and_the_log_names_it() {
    let mut app = app_with_drawing();
    assert!(app.available("tool.stationOffset"));
    run(&mut app, "tool.stationOffset");
    assert_eq!(app.session.tool_id(), "stationOffset");
    assert_eq!(app.log.last().map(|l| l.level), Some(Level::Command));
    assert_eq!(last_said(&app), "Dik ayak ölç");
    assert_eq!(prompt(&app), "Dik ayak ölç: hattın başına tıklayın (A)");
}

/// The chips are options of the prompt: pressed, they go in as if typed.
#[test]
fn mesafe_olc_s_chip_makes_the_first_point_fixed_and_says_each_ray() {
    let mut app = app_with_drawing();
    open(&mut app, survey_ground());
    run(&mut app, "tool.measure");
    assert_eq!(
        prompt(&app),
        "Mesafe ölç: ilk noktayı belirtin [Sabit ilk nokta (S)]"
    );
    chip(&mut app, "S");
    assert!(app.memory.measure_fixed);
    click(&mut app, [6.0, 6.0]);
    let mark = app.log.len();
    click(&mut app, [46.0, 6.0]);
    click(&mut app, [38.0, 30.0]);
    // The corners snap; the rays are measured from the first corner, in order.
    assert_eq!(
        said(&app, mark),
        [
            "1: 40.000 m, semt 100.0000 g",
            "2: 40.000 m, semt 59.0334 g"
        ]
    );
    // The switch is the session's: another run starts with it on.
    run(&mut app, "tool.cancel");
    run(&mut app, "tool.measure");
    assert!(prompt(&app).ends_with("[Sabit ilk nokta (S): açık]"));
}

#[test]
fn alan_hesapla_measures_inside_a_region_and_alan_olarak_ciz_is_one_named_undo() {
    let mut app = app_with_drawing();
    open(&mut app, islands_ground());
    run(&mut app, "tool.area");
    chip(&mut app, "I");
    assert!(app.memory.area_inside);
    let mark = app.log.len();
    click(&mut app, [4.0, 16.0]);
    // 24 × 20 m with a 6 × 6 m building inside: 480 − 36 net; the perimeter is the outer ring's, 88.
    assert_eq!(said(&app, mark), ["Alan 444.00 m²   Çevre 88.000 m"]);
    assert!(prompt(&app).ends_with("[İçine tıkla (I): açık / Alan olarak çiz (A)]"));
    let before = app
        .document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .count();
    chip(&mut app, "A");
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.model.entities().count(), before + 1);
    assert_eq!(last_said(&app), "Alan olarak çizildi: 444.00 m².");
    // One undo takes it away, by the chip's name.
    run(&mut app, "edit.undo");
    assert_eq!(last_said(&app), "Geri alındı: Alan olarak çiz");
    assert_eq!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .entities()
            .count(),
        before
    );
}

#[test]
fn dik_ayak_olc_reads_a_point_against_the_line_into_the_log() {
    let mut app = app_with_drawing();
    open(&mut app, station_ground());
    run(&mut app, "tool.stationOffset");
    // The road's two ends; the parcel's corner, snapped, is 0.452 m right of it.
    click(&mut app, [4.0, 8.0]);
    click(&mut app, [54.0, 26.0]);
    let mark = app.log.len();
    click(&mut app, [22.0, 14.0]);
    click(&mut app, [22.0, 30.0]);
    assert_eq!(
        said(&app, mark),
        [
            "Dik ayak 18.968 m, dik boy 0.452 m (sağda)",
            "Dik ayak 24.388 m, dik boy 14.603 m (solda)"
        ]
    );
    // Nothing is written.
    assert!(!app.document.as_ref().expect("open").model.can_undo());
    // Başka hat: the chip asks for a new start.
    chip(&mut app, "H");
    assert_eq!(prompt(&app), "Dik ayak ölç: hattın başına tıklayın (A)");
}

#[test]
fn kapsam_denetimi_selects_what_lies_far_and_says_how_many() {
    let mut app = app_with_drawing();
    open(&mut app, far_ground());
    run(&mut app, "view.extentCheck");
    assert_eq!(
        last_said(&app),
        "2 nesne çizimin geri kalanından çok uzakta; seçildi."
    );
    assert_eq!(app.selection.len(), 2);
    assert_eq!(app.last_level, Some(Level::Warn));
}

/// Enter, with the modifiers held.
fn enter(app: &mut App, modifiers: Modifiers) {
    app.modifiers = modifiers;
    let _ = app.update(Message::Key(KeyPress {
        key: Key::Named(Named::Enter),
        physical: Physical::Code(Code::Enter),
        modifiers,
        text: None,
        repeat: false,
    }));
    app.modifiers = Modifiers::default();
}

#[test]
fn the_selecting_tools_of_the_sec_list_run_and_leave_by_escape() {
    let mut app = app_with_drawing();
    for (id, tool) in [
        ("tool.selectFence", "selectFence"),
        ("tool.selectCircle", "selectCircle"),
        ("tool.selectContaining", "selectContaining"),
    ] {
        assert_eq!(
            catalog().get(id).map(|c| c.standing),
            Some(Standing::Ported),
            "{id}"
        );
        run(&mut app, id);
        assert_eq!(app.session.tool_id(), tool);
        assert_eq!(app.log.last().map(|l| l.level), Some(Level::Command));
        run(&mut app, "tool.cancel");
        assert_eq!(app.session.tool_id(), "select", "{id} leaves by Esc");
    }
}

/// A fence along y = 2 crosses the two parcels and not the building inside the first.
#[test]
fn a_fence_selects_and_shift_with_enter_adds_to_the_selection() {
    let mut app = app_with_drawing();
    open(&mut app, islands_ground());
    let fence = |app: &mut App| {
        run(app, "tool.selectFence");
        click(app, [-4.0, 2.0]);
        click(app, [48.0, 2.0]);
    };
    // Enter replaces what was selected.
    app.selection.set([Slot(3)]);
    fence(&mut app);
    enter(&mut app, Modifiers::default());
    assert_eq!(last_said(&app), "Çit 2 nesneyi kesti; seçildi.");
    assert_eq!(app.selection.ids(), [Slot(1), Slot(2)]);
    assert_eq!(app.session.tool_id(), "select", "back to Seç");
    // Shift with Enter adds to it.
    app.selection.set([Slot(3)]);
    fence(&mut app);
    enter(&mut app, Modifiers::SHIFT);
    assert_eq!(app.selection.ids(), [Slot(3), Slot(1), Slot(2)]);
    assert_eq!(app.session.tool_id(), "select");
}

#[test]
fn daireyle_sec_kesisen_chip_is_pressed_and_kept() {
    let mut app = app_with_drawing();
    open(&mut app, islands_ground());
    run(&mut app, "tool.selectCircle");
    assert_eq!(
        prompt(&app),
        "Daireyle seç: dairenin merkezine tıklayın [Kesişen (K)]"
    );
    chip(&mut app, "K");
    assert!(app.memory.circle_crossing);
    // A circle round the building's middle: it holds the building; touching takes the parcels.
    click(&mut app, [11.0, 9.0]);
    typed(&mut app, "5");
    assert_eq!(last_said(&app), "Daireye dokunan 2 nesne; seçildi.");
    assert_eq!(app.selection.ids(), [Slot(1), Slot(3)]);
    assert_eq!(app.session.tool_id(), "select");
    run(&mut app, "tool.selectCircle");
    assert!(prompt(&app).ends_with("[Kesişen (K): açık]"));
}

#[test]
fn icerenalani_sec_takes_the_area_a_click_is_in_and_stays() {
    let mut app = app_with_drawing();
    open(&mut app, islands_ground());
    run(&mut app, "tool.selectContaining");
    click(&mut app, [4.0, 16.0]);
    // The parcel as an object: 24 × 20 m (its building is another object, not a hole).
    assert_eq!(last_said(&app), "Alan seçildi (1/1, 480.00 m²).");
    assert_eq!(app.selection.ids(), [Slot(1)]);
    assert_eq!(
        app.session.tool_id(),
        "selectContaining",
        "it stays for more clicks"
    );
    click(&mut app, [34.0, 10.0]);
    assert_eq!(app.selection.ids(), [Slot(2)]);
    // A right click is Enter: back to Seç, the selection kept.
    run(&mut app, "tool.confirm");
    assert_eq!(app.session.tool_id(), "select");
    assert_eq!(app.selection.ids(), [Slot(2)]);
}

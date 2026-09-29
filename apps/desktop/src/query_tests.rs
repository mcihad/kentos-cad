//! docs/adr/0141's queries and selections through the real app: the ribbon's
//! commands and the prompt's chips reach the tools, what they say lands in
//! the message log, and what they write is one named undo step. The tools'
//! own mechanics are `crates/native/interaction/tests`. Test code only.

use iced::{Point, Rectangle, Size};
use kentos_interaction::Level;

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing as sample, last_said};
use crate::tools_scenes::{
    click, far_ground, islands_ground, open, run, station_ground, survey_ground,
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
    // 24 × 20 m with a 6 × 6 m building inside: 480 − 36; 88 + 24 round.
    assert_eq!(said(&app, mark), ["Alan 444.00 m²   Çevre 112.000 m"]);
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

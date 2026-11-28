//! The interaction traces (`fixtures/interaction/v1`, docs/adr/0018) played
//! on the desktop: the files the web plays in a browser
//! (`apps/web/scripts/e2e/interaction.mjs`), unchanged, in the same three
//! variants (US keyboard, Turkish Q, a 2× screen).
//!
//! The player drives the real [`App`] with what its window would send, no
//! window needed:
//!
//! - keys are Iced key events a US or a Turkish Q keyboard produces (AltGr
//!   as Ctrl+Alt, as Windows reports it), through the app's own
//!   subscription (`keys::key_event`);
//! - the pointer is Iced mouse events through the drawing area's own
//!   gesture code (`viewport::gesture`), at the window pixel the world point
//!   falls on through the same camera, rounded to the screen's device pixels
//!   (a usage scenario's pictures take the point itself, as the web's player
//!   does: `Player::exact_pointer`);
//! - while the command line has the keyboard, keys go to a model of the
//!   KentOS UI command line as `view.rs` configures it, its suggestion list
//!   included; the widget operations of the app's own tasks (focusing the
//!   command line when a letter is typed on the drawing, letting it go when a
//!   tool starts from it) run on the model too. A test holds the model to the
//!   real widget;
//! - a `dialog` step uses the open window's controls by their words: the
//!   message each sends (`answers.rs`), as a click or typing would;
//! - `run` is the command's message, as a ribbon button sends it;
//!   `saveAndReopen` is Ctrl+S and Ctrl+O with the file picker answered by a
//!   temporary file, and the app's own tasks run to their end.
//!
//! After each step the expectations are compared by the web runner's rules:
//! clicked points within `clickTolerance`, typed edges exactly, the scale
//! within a relative 1e-9. `kentos-cad snapshot --iz` plays a trace into an
//! image.
//!
//! | File | What it holds |
//! |---|---|
//! | `format.rs` | the trace format, read strictly |
//! | `keyboard.rs` | the variants and the keyboards that type a trace |
//! | `command_line.rs` | the model of the command line while it has the keyboard |
//! | `player.rs` | the player: steps as window events, what a step can see |
//! | `answers.rs` | the controls of an open window a `dialog` step uses |
//! | `compare.rs` | expectations against what the app shows |

mod answers;
mod command_line;
mod compare;
mod format;
mod journal;
mod keyboard;
#[cfg(test)]
#[path = "layer_keys_tests.rs"]
mod layer_keys_tests;
pub mod player;

use std::path::{Path, PathBuf};

pub use answers::Control;
/// The traces not played yet and why (format.rs), for the programs' test that plays every trace.
#[cfg(any(test, feature = "testing"))]
pub use format::PENDING;
pub use format::Trace;
pub use keyboard::{VARIANTS, Variant};
pub use player::Player;

#[cfg(any(test, feature = "testing"))]
use crate::app::App;
#[cfg(any(test, feature = "testing"))]
use player::AREA;

/// The traces' folder, next to the sources.
pub fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1")
}

/// The 2+1 flat as KentOS Mimari draws it: the acceptance trace played with the layer key on and KentOS's exterior wall
/// 25, the style Mimari's tools start with (its package's arch_start.rs). The permit package's and the regulation
/// rules' tests read it (docs/adr/9199 §4.11).
#[cfg(any(test, feature = "testing"))]
pub fn flat() -> App {
    static REPLAY: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = REPLAY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let traces = Trace::all().expect("the traces read");
    let trace = traces
        .iter()
        .find(|t| t.id == "acceptance-flat")
        .expect("the 2+1 trace");
    let variant = VARIANTS[0];
    let (mut app, _) = App::boot(None);
    app.memory.layer_keys = true;
    let style = kentos_commands::builtin_wall_styles()
        .into_iter()
        .find(|s| s.id == "kentos-dis-duvar-25")
        .expect("KentOS's exterior wall");
    kentos_interaction::wall::take_style(&mut app.memory, &style);
    let problems = Player::new(&mut app, trace, variant, AREA, scratch_file(trace, variant))
        .expect("the trace starts")
        .play(None);
    assert!(problems.is_empty(), "{problems:#?}");
    app
}

/// A temporary file for a trace's saves.
pub fn scratch_file(trace: &Trace, variant: Variant) -> PathBuf {
    // A file of its own for each play: two tests playing the same trace in the same variant at once (every trace's
    // test and a trace's own) wrote and reopened each other's file.
    static PLAYS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let play = PLAYS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "kentos-iz-{}-{}-{}-{play}.kcad",
        std::process::id(),
        trace.id,
        variant.id
    ))
}

/// Plays a whole trace on a new app; its problems, empty when it passes.
#[cfg(any(test, feature = "testing"))]
pub fn play_trace(trace: &Trace, variant: Variant) -> Result<Vec<String>, String> {
    let (mut app, _) = App::boot(None);
    let mut player = Player::new(&mut app, trace, variant, AREA, scratch_file(trace, variant))?;
    Ok(player.play(None))
}

#[cfg(test)]
mod tests {
    use super::format::Step;
    use super::*;

    #[test]
    fn a_dialog_pick_changes_the_import_unit_and_not_the_blocks_real_size() {
        let trace = Trace::all()
            .expect("traces")
            .into_iter()
            .find(|t| t.id == "cad-sheet-block-units")
            .expect("unit trace");
        let (mut app, _) = App::boot(None);
        let mut player = Player::new(
            &mut app,
            &trace,
            VARIANTS[0],
            AREA,
            scratch_file(&trace, VARIANTS[0]),
        )
        .expect("player");
        assert!(
            player.play(Some(5)).is_empty(),
            "through the import, before undo"
        );
        let doc = &player.app.document.as_ref().expect("drawing").model;
        let child = doc
            .blocks()
            .iter()
            .find(|b| b.name == "CHILD")
            .expect("child block");
        let kentos_commands::Entity::Line(line) = &child.entities[0] else {
            panic!("line");
        };
        assert!((line.b.x - line.a.x - 2.0).abs() < 1e-9);
        let insert = doc
            .entities()
            .find_map(|e| match e {
                kentos_commands::Entity::Insert(i) if i.block == child.id => Some(i),
                _ => None,
            })
            .expect("child insert");
        assert_eq!(
            insert.scale, 10.0,
            "cm selection was applied; ignoring it in this GIS drawing gives 1000"
        );
    }

    /// A pending trace is still there and still cannot be read: once the
    /// desktop plays its feature, its entry must go and the trace be played.
    #[test]
    fn pending_traces_are_still_pending() {
        for (id, why) in format::PENDING {
            let path = folder().join(format!("{id}.json"));
            assert!(path.exists(), "{id}: izi yok, bekleyenlerden çıkarın");
            assert!(
                Trace::read(&path).is_err(),
                "{id} artık okunuyor; bekleyenlerden çıkarıp oynatın ({why})"
            );
        }
    }

    #[test]
    fn a_point_off_the_drawing_area_stops_the_trace() {
        let mut trace = Trace::by_id("polygon-accept").expect("reads");
        trace.steps.truncate(1);
        trace.steps.push(Step {
            run: None,
            template: None,
            apply_template: None,
            key: None,
            text: None,
            move_to: None,
            rest: None,
            click: Some([500.0, 0.0]),
            drag: None,
            mouse: None,
            double_click: None,
            click_number: None,
            click_lock: None,
            right_click: None,
            focus: None,
            save_and_reopen: None,
            shift: None,
            hold: None,
            shot: None,
            dialog: None,
            fill: None,
            check: None,
            press: None,
            panel: None,
            pick: None,
            sort: None,
            row: None,
            ctrl: None,
            edit: None,
            overview: None,
            magnifier: None,
            timebar: None,
            bond_notice: None,
            paragraph: None,
            chart_press: None,
            expect: None,
            note: None,
        });
        let problems = play_trace(&trace, VARIANTS[0]).expect("starts");
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].contains("çizim alanının dışında"),
            "{problems:?}"
        );
    }
}

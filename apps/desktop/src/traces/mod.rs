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
//!   falls on through the same camera, rounded to the screen's device pixels;
//! - while the command line has the keyboard, keys go to a model of the
//!   KentOS UI command line as `view.rs` configures it, its suggestion list
//!   included; the widget operations of the app's own tasks (focusing the
//!   command line when a letter is typed on the drawing, letting it go when a
//!   tool starts from it) run on the model too. A test holds the model to the
//!   real widget;
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
//! | `compare.rs` | expectations against what the app shows |

mod command_line;
mod compare;
mod format;
mod keyboard;
mod player;

use std::path::{Path, PathBuf};

pub use format::Trace;
pub use keyboard::{VARIANTS, Variant};
pub use player::Player;

#[cfg(test)]
use crate::app::App;
#[cfg(test)]
use player::AREA;

/// The traces' folder, next to the sources.
pub fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1")
}

/// A temporary file for a trace's saves.
pub fn scratch_file(trace: &Trace, variant: Variant) -> PathBuf {
    std::env::temp_dir().join(format!(
        "kentos-iz-{}-{}-{}.kcad",
        std::process::id(),
        trace.id,
        variant.id
    ))
}

/// Plays a whole trace on a new app; its problems, empty when it passes.
#[cfg(test)]
pub fn play_trace(trace: &Trace, variant: Variant) -> Result<Vec<String>, String> {
    let (mut app, _) = App::boot(None);
    let mut player = Player::new(&mut app, trace, variant, AREA, scratch_file(trace, variant))?;
    Ok(player.play(None))
}

#[cfg(test)]
mod tests {
    use super::format::Step;
    use super::*;

    /// Every trace × every variant, as `pnpm e2e:interaction` plays them on the web.
    #[test]
    fn every_trace_passes_in_every_variant() {
        let traces = Trace::all().expect("the traces read");
        assert!(traces.len() >= 4, "the four traces are there");
        let mut report = Vec::new();
        let mut failed = 0;
        for variant in VARIANTS {
            for trace in &traces {
                let problems = play_trace(trace, variant).unwrap_or_else(|e| vec![e]);
                let mark = if problems.is_empty() { "✓" } else { "✗" };
                report.push(format!(
                    "{mark} [{}] {}: {}",
                    variant.id, trace.id, trace.title
                ));
                for p in &problems {
                    report.push(format!("  {p}"));
                }
                if !problems.is_empty() {
                    failed += 1;
                }
            }
        }
        println!("{}", report.join("\n"));
        assert_eq!(failed, 0, "\n{}", report.join("\n"));
    }

    #[test]
    fn a_point_off_the_drawing_area_stops_the_trace() {
        let mut trace = Trace::by_id("polygon-accept").expect("reads");
        trace.steps.truncate(1);
        trace.steps.push(Step {
            run: None,
            key: None,
            text: None,
            move_to: None,
            click: Some([500.0, 0.0]),
            double_click: None,
            right_click: None,
            focus: None,
            save_and_reopen: None,
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

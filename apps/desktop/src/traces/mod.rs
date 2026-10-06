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
mod keyboard;
mod player;

use std::path::{Path, PathBuf};

pub use answers::Control;
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
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, PoisonError};

    /// A panic's message, as the payload carries it.
    fn said(payload: &(dyn std::any::Any + Send)) -> &str {
        payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("(iletisiz)")
    }

    /// Every trace × every variant, as `pnpm e2e:interaction` plays them on the web.
    /// Each play has an app of its own, so the plays share out among the
    /// machine's cores; the report keeps their order, and a play that panics
    /// is a problem of its own, not the end of the others.
    #[test]
    fn every_trace_passes_in_every_variant() {
        let traces = Trace::all().expect("the traces read");
        assert!(traces.len() >= 4, "the four traces are there");
        let plays: Vec<(Variant, &Trace)> = VARIANTS
            .iter()
            .flat_map(|&variant| traces.iter().map(move |trace| (variant, trace)))
            .collect();
        let next = AtomicUsize::new(0);
        let results = Mutex::new(vec![Vec::new(); plays.len()]);
        let threads = std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(plays.len());
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| {
                    // A long play does not hold up the short ones behind it: each thread takes the next.
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&(variant, trace)) = plays.get(i) else {
                            break;
                        };
                        let problems = panic::catch_unwind(AssertUnwindSafe(|| {
                            play_trace(trace, variant).unwrap_or_else(|e| vec![e])
                        }))
                        .unwrap_or_else(|payload| vec![format!("panik: {}", said(&*payload))]);
                        results.lock().unwrap_or_else(PoisonError::into_inner)[i] = problems;
                    }
                });
            }
        });
        let results = results.into_inner().unwrap_or_else(PoisonError::into_inner);
        let mut report = Vec::new();
        let mut failed = 0;
        for ((variant, trace), problems) in plays.iter().zip(&results) {
            let mark = if problems.is_empty() { "✓" } else { "✗" };
            report.push(format!(
                "{mark} [{}] {}: {}",
                variant.id, trace.id, trace.title
            ));
            for p in problems {
                report.push(format!("  {p}"));
            }
            if !problems.is_empty() {
                failed += 1;
            }
        }
        for (id, why) in format::PENDING {
            report.push(format!("– {id}: atlandı, geçmedi: {why}"));
        }
        println!("{}", report.join("\n"));
        assert_eq!(failed, 0, "\n{}", report.join("\n"));
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
            double_click: None,
            right_click: None,
            focus: None,
            save_and_reopen: None,
            shift: None,
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
            overview: None,
            magnifier: None,
            paragraph: None,
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

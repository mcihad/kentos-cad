//! The user coordinate system on the desktop (docs/adr/0261): Dünya and
//! Önceki through `cad.ucs.set`, and the session's history Önceki takes
//! from. Every change of the drawing's UCS, whoever made it (a tool, Proje
//! ayarları, Python, an agent), puts the one before on the history, at most
//! ten; Önceki gives the last back as an explicit frame. The history is the
//! session's, never the drawing's: another drawing starts it anew.

use kentos_contracts::{CommandResult, Ucs, UcsSet, UcsTo};
use kentos_native_application::{ExecutionContext, ucs_set};

use crate::app::App;

/// How many UCSs Önceki can go back.
const MOST_PREVIOUS: usize = 10;

/// The UCSs before the current one, newest last, and what was seen last.
#[derive(Debug, Default)]
pub(crate) struct UcsHistory {
    /// The drawing (its session) and its UCS when last seen; none: no drawing seen yet.
    seen: Option<(u64, Option<Ucs>)>,
    previous: Vec<Ucs>,
    /// Önceki is writing: what it leaves is not put back on the history.
    going_back: bool,
}

impl App {
    /// Notes a change of the drawing's UCS (after every message).
    pub(crate) fn follow_ucs(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let now = (doc.session, doc.model.settings().ucs);
        // Karelaj's nodes follow the axes (docs/adr/0261 §4).
        self.draft.ucs = now.1.map(|u| {
            (
                kentos_interaction::Vec2::new(u.origin.x, u.origin.y),
                u.angle,
            )
        });
        let h = &mut self.ucs_history;
        match h.seen {
            Some((session, before)) if session == now.0 => {
                if before != now.1 {
                    if !h.going_back {
                        h.previous.push(before.unwrap_or(Ucs::WORLD));
                        if h.previous.len() > MOST_PREVIOUS {
                            h.previous.remove(0);
                        }
                    }
                    h.seen = Some(now);
                }
            }
            // Another drawing: its history starts anew.
            _ => {
                h.previous.clear();
                h.seen = Some(now);
            }
        }
    }

    /// Dünya: the world's axes again.
    pub(crate) fn ucs_world(&mut self) {
        self.set_ucs(UcsTo::World);
    }

    /// Önceki: the UCS before this one, from the session's history.
    pub(crate) fn ucs_previous(&mut self) {
        let Some(before) = self.ucs_history.previous.pop() else {
            self.warn("Önceki UCS yok: bu oturumda UCS değişmedi.");
            return;
        };
        self.ucs_history.going_back = true;
        self.set_ucs(UcsTo::Frame {
            origin: before.origin,
            angle: before.angle,
        });
        self.follow_ucs();
        self.ucs_history.going_back = false;
    }

    /// Runs `cad.ucs.set` and says where the axes are now.
    fn set_ucs(&mut self, to: UcsTo) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = UcsSet {
            to,
            expected_revision: None,
        };
        match ucs_set::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let u = output.ucs;
                let said = match u.name {
                    Some(name) => format!("UCS: {name}"),
                    None if u.origin.x == 0.0 && u.origin.y == 0.0 && u.angle == 0.0 => {
                        "UCS: Dünya".to_owned()
                    }
                    None => {
                        let f = kentos_interaction::Format::of(doc.model.settings());
                        format!(
                            "UCS: başlangıç {}, açı {}",
                            f.point(kentos_interaction::Vec2::new(u.origin.x, u.origin.y)),
                            f.angle(u.angle)
                        )
                    }
                };
                self.output(said);
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }
}

//! Çizime yaz (docs/adr/0203 §8; the web's `writeNetwork`, `writeLevels`) through `cad.survey.adjust`
//! (docs/adr/0622 §9): the tables go to the command as they are typed (a known point by its name alone is the
//! drawing's), the command adjusts the network and writes the new points only, never the known ones. Yatay ağ moves
//! the drawing's points of the same name to their adjusted places with the vertices on them and adds the others on
//! the chosen layer; Kot ağı gives the drawing's points of the same name their adjusted heights, their `Z (m)` and the
//! vertices on them too, and says the points it has not. One undo step named after the window; a locked layer refuses
//! it all, in the command's words.

use kentos_commands::{
    AdjustDifference, AdjustKnownHeight, AdjustKnownPoint, AdjustObservation, AdjustWork,
    CommandResult, LevelMethod, SurveyAdjust, SurveyAdjusted, Vec2 as Wire,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, codes, survey_adjust};

use super::{LEVEL_TITLE, LevelForm, LevelKind, NETWORK_TITLE, NetworkForm, cell, number};
use crate::app::App;

/// Yatay ağ's tables as the command takes them.
fn network_work(form: &NetworkForm) -> AdjustWork {
    let known = form
        .known
        .filled()
        .filter(|(_, r)| !cell(r, 0).is_empty())
        .map(|(_, r)| AdjustKnownPoint {
            name: cell(r, 0).to_owned(),
            // The table's Y is east, X north.
            at: match (number(cell(r, 1)), number(cell(r, 2))) {
                (Some(y), Some(x)) => Some(Wire { x: y, y: x }),
                _ => None,
            },
            sigma: number(cell(r, 3)).map(|s| s / 1000.0),
        })
        .collect();
    let rows = form
        .rows
        .filled()
        .map(|(_, r)| AdjustObservation {
            station: cell(r, 0).to_owned(),
            target: cell(r, 1).to_owned(),
            direction: number(cell(r, 2)),
            distance: number(cell(r, 3)),
        })
        .collect();
    AdjustWork::Horizontal { known, rows }
}

/// Kot ağı's tables as the command takes them.
fn level_work(form: &LevelForm) -> AdjustWork {
    let known = form
        .known
        .filled()
        .filter(|(_, r)| !cell(r, 0).is_empty())
        .map(|(_, r)| AdjustKnownHeight {
            name: cell(r, 0).to_owned(),
            h: number(cell(r, 1)),
            sigma: number(cell(r, 2)).map(|s| s / 1000.0),
        })
        .collect();
    let rows = form
        .rows
        .filled()
        .map(|(_, r)| AdjustDifference {
            from: cell(r, 0).to_owned(),
            to: cell(r, 1).to_owned(),
            dh: number(cell(r, 2)).unwrap_or(f64::NAN),
            length: number(cell(r, 3)).unwrap_or(f64::NAN),
        })
        .collect();
    AdjustWork::Level {
        method: match form.kind {
            LevelKind::Geometric => LevelMethod::Geometric,
            LevelKind::Trigonometric => LevelMethod::Trigonometric,
        },
        known,
        rows,
    }
}

impl App {
    /// Runs the adjustment's command on the drawing: what it wrote, its warnings said but the points it has not
    /// (the window says them itself), or none with its refusal said.
    fn adjust(&mut self, input: SurveyAdjust) -> Option<SurveyAdjusted> {
        let doc = self.document.as_mut()?;
        let result = survey_adjust::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match result {
            CommandResult::Completed { output, warnings } => {
                for w in warnings
                    .into_iter()
                    .filter(|w| w.code != codes::POINTS_NOT_DRAWN)
                {
                    self.warn(w.message);
                }
                Some(output)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                None
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => None,
        }
    }

    /// The slots of the persistent ids written.
    fn slots_of(&self, uids: &[String]) -> Vec<Slot> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        uids.iter()
            .filter_map(|u| Uuid::parse_str(u).ok())
            .filter_map(|u| doc.model.slot_of(u))
            .collect()
    }

    /// Yatay ağ dengelemesi's Çizime yaz: the new points moved or added.
    pub(crate) fn network_write(&mut self) {
        let form = &self.calc.network;
        let Some(Ok(_)) = &form.solved.result else {
            return;
        };
        let input = SurveyAdjust {
            work: network_work(form),
            layer_id: form.layer.clone(),
            unit: None,
            expected_revision: None,
        };
        let Some(done) = self.adjust(input) else {
            return;
        };
        if done.moved.is_empty() && done.created.is_empty() {
            return;
        }
        let (moved, added) = (done.moved.len(), done.created.len());
        let mut written = self.slots_of(&done.moved);
        written.extend(done.ids.iter().map(|id| Slot(*id)));
        self.selection.set(written);
        self.say(
            Level::Success,
            format!(
                "{NETWORK_TITLE}: {moved} nokta dengelenmiş yerine taşındı, {added} nokta eklendi (Ctrl+Z geri alır)."
            ),
        );
        self.calc.open = None;
        self.dialog = None;
    }

    /// Kot ağı dengelemesi's Çizime yaz: the new points' heights.
    pub(crate) fn level_write(&mut self) {
        let form = &self.calc.level;
        let Some(Ok(_)) = &form.solved.result else {
            return;
        };
        let input = SurveyAdjust {
            work: level_work(form),
            layer_id: None,
            unit: None,
            expected_revision: None,
        };
        let Some(done) = self.adjust(input) else {
            return;
        };
        let absent = (!done.absent.is_empty()).then(|| {
            format!(
                "{LEVEL_TITLE}: {} çizimde yok; yazılmadı.",
                done.absent.join(", ")
            )
        });
        if done.moved.is_empty() {
            if let Some(absent) = absent {
                self.warn(absent);
            }
            return;
        }
        let n = done.moved.len();
        let written = self.slots_of(&done.moved);
        self.selection.set(written);
        self.say(
            Level::Success,
            format!("{LEVEL_TITLE}: {n} noktanın kotu yazıldı (Ctrl+Z geri alır)."),
        );
        if let Some(absent) = absent {
            self.warn(absent);
        }
        self.calc.open = None;
        self.dialog = None;
    }
}

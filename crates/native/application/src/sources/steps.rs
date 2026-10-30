//! A bind's steps, said to the job it runs as (docs/adr/0757 §1): how far it is and what it does, and whether it was
//! asked to stop. Run at once (a script's, a plan's) it says nothing and is never stopped.

use kentos_commands::{CallAnswer, CommandResult, SourcesEdit};
use kentos_domain::Document;
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::jobs::{Job, JobOutcome, Progress};

/// The bind's steps: the share of its job's bar each begins at, and what the panel says of it. The shares are a
/// database table's 50 000 rows of 28 columns (docs/adr/0757's measure, 1,41 s): the read 27 %, the rows made into
/// objects 19 %, the objects written 7 %, the layer's digest 16 %, the base 31 %.
pub const BIND_STEPS: [(f64, &str); 5] = [
    (0.0, "Tablo okunuyor"),
    (0.27, "Satırlar nesneye çevriliyor"),
    (0.46, "Nesneler katmana yazılıyor"),
    (0.53, "Katmanın özeti çıkarılıyor"),
    (0.69, "Kaynakla eşleşme kaydediliyor"),
];

/// The step `share` is in, and the share the next begins at (1 after the last).
pub fn bind_step(share: f64) -> (usize, f64) {
    let at = BIND_STEPS
        .iter()
        .rposition(|(from, _)| share >= *from)
        .unwrap_or(0);
    let next = BIND_STEPS.get(at + 1).map_or(1.0, |(from, _)| *from);
    (at, next)
}

/// A bind's job, when it runs as one.
#[derive(Clone, Copy, Default)]
pub(crate) struct Steps<'a>(pub Option<&'a Progress>);

impl Steps<'_> {
    /// At step `k`: said, and a stop asked for ends the bind here (nothing of it is written).
    pub fn at(self, k: usize) -> Result<(), Stop> {
        let Some(p) = self.0 else {
            return Ok(());
        };
        if p.stopped() {
            return Err(Stop::Cancelled);
        }
        let share = BIND_STEPS.get(k).map_or(1.0, |(from, _)| *from);
        p.set((share * 1000.0).round() as u64, 1000);
        Ok(())
    }
}

/// `cad.sources.edit` as job `id` on `doc`'s reading copy (docs/adr/0757 §1): the command itself, its steps said to the
/// job and a stop heard, its write a journal row and one undo step as at once. No protocol summary: the desktop's bind
/// does not ask for one, and it would cost a whole table's objects once more (each added object's stamp and value:
/// 1,6 s of 2,4 s for 50 000 parcels).
pub fn edit_job(id: String, doc: &Document, input: SourcesEdit) -> Job {
    Job::start_with(
        id,
        kentos_commands::CAD_SOURCES_EDIT.into(),
        None,
        doc,
        move |mut copy, progress| {
            let result = {
                let mut cx = ExecutionContext::new(&mut copy);
                cx.job = Some(progress);
                super::edit_execute(&mut cx, input)
            };
            let result = match result {
                CommandResult::Completed { output, warnings } => CommandResult::Completed {
                    output: serde_json::to_value(output).unwrap_or(Value::Null),
                    warnings,
                },
                CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
                CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
                CommandResult::Conflict { error } => CommandResult::Conflict { error },
                CommandResult::Cancelled => CommandResult::Cancelled,
                CommandResult::Failed { error } => CommandResult::Failed { error },
            };
            JobOutcome {
                answer: CallAnswer::of(result),
                copy: Some(copy),
                effect: None,
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_share_is_in_one_step() {
        assert_eq!(bind_step(0.0), (0, 0.27));
        assert_eq!(bind_step(0.26), (0, 0.27));
        assert_eq!(bind_step(0.27), (1, 0.46));
        assert_eq!(bind_step(0.95), (4, 1.0));
        assert_eq!(bind_step(1.0), (4, 1.0));
        let steps: Vec<f64> = BIND_STEPS.iter().map(|(s, _)| *s).collect();
        assert!(
            steps.windows(2).all(|w| w[0] < w[1]),
            "the steps go forward"
        );
    }

    #[test]
    fn a_stopped_job_ends_the_bind_and_a_running_one_hears_its_step() {
        let p = Progress::default();
        assert!(Steps(Some(&p)).at(2).is_ok());
        assert_eq!(p.fraction(), Some(0.46));
        p.stop();
        assert!(matches!(Steps(Some(&p)).at(3), Err(Stop::Cancelled)));
        assert!(Steps(None).at(3).is_ok(), "run at once it is never stopped");
    }
}

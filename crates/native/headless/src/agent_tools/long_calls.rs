//! Long commands as jobs (docs/adr/8057 §27): a command that runs an extension's Python tool computes the tool in its
//! plan and in its execution; called on the desktop's interface thread (Asistan, the agents' link) it held the window
//! while Python ran. The owner's guard sends such a command's plan and execution as jobs (`start`: computed on a
//! reading copy on another thread, written when asked, `job`); a host on another thread waits by asking
//! ([`Host::job_wait`](super::Host::job_wait)), the desktop's chat answers the call when the job ends ([`finish`]).

use serde_json::{Value, json};

use super::{Guard, Host};
use crate::HeadlessError;

/// A long command's job, for whoever answers it when it ends.
#[derive(Clone, Debug, PartialEq)]
pub struct LongCall {
    /// The job's id (`start`'s answer).
    pub job: String,
    pub command: String,
    pub input: Value,
    /// The drawing's revision the call was made on: a plan is remembered with it.
    pub revision: String,
    /// A plan's job (else an execution's).
    pub plan: bool,
}

impl LongCall {
    /// [`is_long`] for a test of another crate.
    #[doc(hidden)]
    pub fn is_long_for_test(command: &str, input: &Value) -> bool {
        is_long(command, input)
    }
}

/// Whether `command` with `input` runs an extension's Python tool (`ext.…`): its plan and execution go as jobs.
pub(super) fn is_long(command: &str, input: &Value) -> bool {
    matches!(command, "cad.processing.run" | "cad.processing.batch")
        && input
            .get("tool")
            .and_then(Value::as_str)
            .is_some_and(|tool| tool.starts_with("ext."))
}

/// A job's answer, as a call's, once the job ended (done, failed, conflict, cancelled); none while it goes.
pub fn job_answer(status: &Value) -> Option<Value> {
    match status.get("phase").and_then(Value::as_str)? {
        "done" | "failed" | "conflict" | "cancelled" => {
            Some(status.get("answer").cloned().unwrap_or(Value::Null))
        }
        _ => None,
    }
}

/// What a plan's answer says to write it: the same command, input and plan id.
pub(super) fn plan_next(plan: &str) -> String {
    format!(
        "Yazmak için aynı komut ve girdiyle mode: \"execute\", planId: \"{plan}\". Çizim bu arada değişirse plan \
         geçersizdir."
    )
}

/// `request` (a call's envelope) sent as a job: its answer as a call's through [`finish`], or, from a host that
/// answers the call when the job ends, that it runs.
pub(super) fn send(
    host: &mut dyn Host,
    guard: &mut Guard,
    request: &Value,
    command: &str,
    input: &Value,
    revision: &str,
    plan: bool,
) -> Result<Value, HeadlessError> {
    let started = host.request("start", request)?;
    let job = started
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| HeadlessError::new("job_failed", "İş başlatılamadı: kimliği gelmedi."))?
        .to_owned();
    let long = LongCall {
        job,
        command: command.to_owned(),
        input: input.clone(),
        revision: revision.to_owned(),
        plan,
    };
    if let Some(answer) = job_answer(&started) {
        return finish(guard, &long, answer);
    }
    match host.job_wait(&long)? {
        Some(answer) => finish(guard, &long, answer),
        None => Ok(json!({
            "status": "running",
            "job": long.job,
            "note": "Eklentinin Python aracı arka planda çalışıyor: bitince bu çağrı cevaplanır.",
        })),
    }
}

/// A long command's answer when its job ended: a plan's id remembered by the guard (with what to send to write it),
/// an execution's answer as it is.
pub fn finish(guard: &mut Guard, long: &LongCall, answer: Value) -> Result<Value, HeadlessError> {
    let mut answer = answer;
    if long.plan
        && let Some(plan) = answer
            .get("planId")
            .and_then(Value::as_str)
            .map(str::to_owned)
    {
        guard.remember(&plan, &long.command, &long.input, &long.revision);
        answer["next"] = json!(plan_next(&plan));
    }
    Ok(answer)
}

//! One run of an extension's İşlemler tool in its version's process (docs/adr/8057 §7): what the desktop's provider and
//! a headless host's share. The process is the pool's (kept between runs, closed when idle); the run gives it the
//! tool's inputs as data, passes on what it says as it goes, asks its code to stop when the user does, answers a
//! built-in tool its code asks for (docs/adr/8057 §47), and reads how it ended into İşlemler' answer and a note for the
//! host. The window, when there is one, waits for nothing: the run is on İşlemler' own thread.

use std::cell::RefCell;
use std::sync::Arc;
use std::time::{Duration, Instant};

use kentos_processing::Feedback;
use kentos_processing::external::{ExternalTool, Failure};
use kentos_python_env::process::{Ended, Heard, KILL_AFTER, Pool, Start};
use serde_json::{Value, json};

use crate::note::Note;
use crate::plan::Plan;

/// What a run prints that goes to İşlemler' log, lines at most: the rest is in the extension's own output.
const PRINTED_LINES: usize = 200;

/// Runs `tool` with `input` (`{ params, inputs, layers, srid }`, what `kentos_processing::external` gives) in the
/// process of `plan`'s version, which `start` starts when the pool has none: `feedback` hears its progress, log and
/// printed lines and is asked whether the user stopped it; what its code asks is answered by [`crate::calls`]; `note`
/// is told it started and how it ended. The code's answer, or why there is none.
pub fn run_tool(
    pool: &Arc<Pool>,
    plan: &Plan,
    start: &dyn Fn() -> Result<Start, String>,
    tool: &ExternalTool,
    input: Value,
    feedback: &mut dyn Feedback,
    note: &dyn Fn(Note),
) -> Result<Value, Failure> {
    let part = format!("processing:{}", plan.local);
    let key = plan.key();
    let invoke = json!({
        "kind": "processing",
        "local": plan.local,
        "handler": plan.handler,
        "params": input.get("params").cloned().unwrap_or_else(|| json!({})),
        "inputs": input.get("inputs").cloned().unwrap_or_else(|| json!({})),
        "layers": input.get("layers").cloned().unwrap_or_else(|| json!({})),
        "settings": {},
        "document": false,
    });
    note(Note::Started {
        ext: plan.ext.clone(),
        part: part.clone(),
    });
    let began = Instant::now();
    let feedback = RefCell::new(feedback);
    let mut printed = 0usize;
    let mut output = String::new();
    // A built-in tool its code asks for runs on its own objects, in the drawing's coordinate system (§47).
    let srid = input["srid"]
        .as_u64()
        .and_then(|s| u32::try_from(s).ok())
        .unwrap_or(0);
    let ended = pool.run_answering(
        &key,
        start,
        invoke,
        Duration::from_secs(tool.timeout),
        &|| feedback.borrow().canceled(),
        &mut |heard| {
            let mut f = feedback.borrow_mut();
            match heard {
                Heard::Progress { done, total, text } => {
                    let share = total
                        .filter(|t| *t > 0.0)
                        .map_or(0.0, |t| (done / t).clamp(0.0, 1.0));
                    f.progress(share, text.as_deref().unwrap_or_default());
                }
                Heard::Log { level, text } if level == "info" => f.info(text),
                Heard::Log { text, .. } => f.warn(text),
                Heard::Out { text, .. } => {
                    if output.len() < 64 * 1024 {
                        output.push_str(&text);
                    }
                    for line in text.lines() {
                        if printed < PRINTED_LINES {
                            f.info(line.to_owned());
                        }
                        printed += 1;
                    }
                }
            }
        },
        &mut |method, params| crate::calls::tool_call(method, params, srid),
    );
    let canceled = feedback.borrow().canceled();
    // What its process holds now (measured where the system says it): its log line says it.
    let memory = pool
        .running()
        .into_iter()
        .find(|r| r.key == key)
        .and_then(|r| r.memory);
    let run = Concluded {
        ext: &plan.ext,
        part: &part,
        label: &tool.tool.label,
        timeout: tool.timeout,
        canceled,
        took: began.elapsed().as_secs_f64(),
        output: &output,
        memory,
    };
    let (answer, said) = run.conclude(ended);
    if let Some(said) = said {
        note(said);
    }
    answer
}

/// How a run ended, read: what İşlemler gets and what the host is told.
pub(crate) struct Concluded<'a> {
    pub(crate) ext: &'a str,
    pub(crate) part: &'a str,
    pub(crate) label: &'a str,
    pub(crate) timeout: u64,
    /// The user stopped it (Durdur).
    pub(crate) canceled: bool,
    pub(crate) took: f64,
    /// What it printed.
    pub(crate) output: &'a str,
    /// Its process's resident memory after the run, bytes, where the system says it.
    pub(crate) memory: Option<u64>,
}

/// A number as Turkish writes it: a decimal comma.
fn turkish(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}").replace('.', ",")
}

impl Concluded<'_> {
    fn worked(&self, code: &'static str, message: String) -> Option<Note> {
        Some(Note::Worked {
            ext: self.ext.to_owned(),
            part: self.part.to_owned(),
            code,
            message,
        })
    }

    fn failed(
        &self,
        code: &'static str,
        message: String,
        detail: Option<String>,
        crash: bool,
    ) -> (Result<Value, Failure>, Option<Note>) {
        let note = Note::Failed {
            ext: self.ext.to_owned(),
            part: self.part.to_owned(),
            title: self.label.to_owned(),
            code,
            message: message.clone(),
            detail,
            crash,
        };
        (Err(Failure::Failed(message)), Some(note))
    }

    pub(crate) fn conclude(&self, ended: Ended) -> (Result<Value, Failure>, Option<Note>) {
        let label = self.label;
        match ended {
            Ended::Answered(done) if done["ok"] == true => {
                let memory = self
                    .memory
                    .map(|b| format!("; süreç belleği {} MB", turkish(b as f64 / 1_048_576.0, 0)))
                    .unwrap_or_default();
                (
                    Ok(done["result"].clone()),
                    self.worked(
                        "run_finished",
                        format!("Bitti ({} sn{memory}).", turkish(self.took, 1)),
                    ),
                )
            }
            Ended::Answered(done) if done["stopped"] == true => {
                let words = done["stoppedAfterMs"].as_u64().map_or_else(
                    || "Durduruldu; çizim değişmedi.".to_owned(),
                    |ms| format!("Durdur'dan {ms} ms sonra durdu; çizim değişmedi."),
                );
                let note = self.worked("run_stopped", words);
                if self.canceled {
                    (Err(Failure::Stopped), note)
                } else {
                    (
                        Err(Failure::Failed(format!(
                            "“{label}” kendiliğinden durdu; çizim değişmedi."
                        ))),
                        note,
                    )
                }
            }
            Ended::Answered(done) => {
                let message = done["message"].as_str().unwrap_or_default();
                if done["unavailable"] == true {
                    return self.failed(
                        "run_unavailable",
                        format!("“{label}” bu KentOS sürümünde çalışmaz: {message}"),
                        None,
                        false,
                    );
                }
                if done["refused"] == true {
                    return self.failed(
                        "handler_refused",
                        format!("“{label}” çalıştırılmadı: {message}"),
                        None,
                        false,
                    );
                }
                let exception = done["exception"].as_str().unwrap_or("Hata");
                let mut detail = done["error"].as_str().unwrap_or_default().to_owned();
                if !self.output.is_empty() {
                    detail = format!("{}\n{detail}", self.output);
                }
                self.failed(
                    "run_failed",
                    format!("“{label}” hata verdi: {exception}: {message}"),
                    (!detail.is_empty()).then_some(detail),
                    false,
                )
            }
            Ended::StoppedBefore => (Err(Failure::Stopped), None),
            Ended::Killed { timed_out: false } => (
                Err(Failure::Stopped),
                self.worked(
                    "run_killed",
                    format!(
                        "Durdur'a {} sn içinde yanıt vermedi; süreci sonlandırıldı, çizim değişmedi.",
                        KILL_AFTER.as_secs()
                    ),
                ),
            ),
            Ended::Killed { timed_out: true } => self.failed(
                "run_timed_out",
                format!(
                    "“{label}” {} sn içinde bitmedi; süreci sonlandırıldı, çizim değişmedi.",
                    self.timeout
                ),
                None,
                false,
            ),
            Ended::NotStarted(why) => self.failed(
                "runner_not_started",
                format!("“{label}” çalıştırılamadı: {why}"),
                None,
                false,
            ),
            Ended::Crashed(words) => self.failed(
                "runner_crashed",
                format!(
                    "“{label}” çalışırken eklentinin süreci kapandı; çizim değişmedi. {}",
                    words.lines().next().unwrap_or_default()
                ),
                Some(words),
                true,
            ),
        }
    }
}

#[cfg(test)]
mod tests;

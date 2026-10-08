//! Runs a model (the web's `processing/modelRunner.ts`): its steps in
//! dependency order, each a normal tool run whose values come from fixed
//! values, the model's inputs or earlier steps' outputs (a features output
//! reaches the next step as the ids of the objects it made or chose). The
//! whole model is one undo step; if a step fails or is stopped, what the
//! earlier steps did is taken back.
//!
//! The desktop can run a model apart from the drawing (docs/adr/0125): on
//! another thread, over the drawing's reading copy, each step prepared,
//! computed and applied there ([`record_model`]); then the recorded steps
//! are applied on the drawing as they were on the copy, in one undo step
//! ([`replay_model`]).

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::features::Host;
use crate::model::{Model, ModelStep, ValueSource, check_model, order_steps, step_name};
use crate::parameters::default_values;
use crate::runner::{Job, LogLine, Outcome, Prepared, RunRecord, Runner, Status};
use crate::types::{Defaults, Feedback, OutputKind, RunResult, Target, Tool, Values};

/// The prefix of model ids in history and commands ("model:builtin.parcelSheet").
pub const MODEL_PREFIX: &str = "model:";

/// A model as the dialog sees it: its inputs are the parameters.
pub fn model_as_tool(model: &Model, lookup: &dyn Fn(&str) -> Option<Tool>) -> Tool {
    let steps: Vec<&ModelStep> = match order_steps(model) {
        Ok(order) => order
            .iter()
            .filter_map(|id| model.steps.iter().find(|s| &s.id == id))
            .collect(),
        Err(_) => model.steps.iter().collect(),
    };
    let help = steps
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{}. {}", i + 1, step_name(s, lookup)))
        .collect::<Vec<_>>()
        .join("\n\n");
    Tool {
        id: format!("{MODEL_PREFIX}{}", model.id),
        label: model.label.clone(),
        category: model.category.clone(),
        description: if model.description.is_empty() {
            format!("{} adımlı model.", model.steps.len())
        } else {
            model.description.clone()
        },
        help: Some(help),
        keywords: Vec::new(),
        aliases: Vec::new(),
        icon: Some("processing".into()),
        parameters: model.inputs.iter().filter_map(|i| i.def()).collect(),
        outputs: Vec::new(),
        targets: vec![Target::Client],
        validate: None,
        preview: None,
        run: None,
    }
}

/// The value a step parameter gets from its source.
fn source_value(
    src: &ValueSource,
    inputs: &Values,
    outputs: &BTreeMap<String, Values>,
    model: &Model,
    lookup: &dyn Fn(&str) -> Option<Tool>,
) -> Value {
    match src {
        ValueSource::Value(v) => v.clone(),
        ValueSource::Input(name) => inputs.get(name).cloned().unwrap_or(Value::Null),
        ValueSource::Output { step, output } => {
            let out = outputs.get(step).and_then(|o| o.get(output)).cloned();
            let features = model
                .steps
                .iter()
                .find(|s| &s.id == step)
                .and_then(|s| lookup(&s.tool))
                .and_then(|t| t.outputs.into_iter().find(|o| &o.name == output))
                .is_some_and(|o| o.kind == OutputKind::Features);
            if features {
                let ids = out.filter(Value::is_array).unwrap_or_else(|| json!([]));
                json!({ "scope": "ids", "ids": ids })
            } else {
                out.unwrap_or(Value::Null)
            }
        }
    }
}

/// A step's outputs: features as the ids it returned (selected, changed) or else the ones it added.
fn step_outputs(tool: &Tool, result: &RunResult, added: &[Slot]) -> Values {
    let mut out = Values::new();
    for o in &tool.outputs {
        let v = result.outputs.get(&o.name);
        let value = match o.kind {
            OutputKind::Features => Some(match v {
                Some(list @ Value::Array(_)) => list.clone(),
                _ => json!(added.iter().map(|s| s.0).collect::<Vec<_>>()),
            }),
            _ => v.cloned(),
        };
        if let Some(value) = value {
            out.insert(o.name.clone(), value);
        }
    }
    out
}

/// How a model's step runs ([`run_model_with`]): given the runner, the
/// host, the step's tool and values, and where its messages go.
pub type StepRun<'a> =
    dyn FnMut(&mut Runner, &mut dyn Host, &Tool, &Values, &mut Vec<LogLine>) -> Outcome + 'a;

/// Runs `model` with its inputs on the host's drawing, as one undo step.
pub fn run_model(
    model: &Model,
    inputs: &Values,
    runner: &mut Runner,
    host: &mut dyn Host,
    lookup: &dyn Fn(&str) -> Option<Tool>,
    log: &mut Vec<LogLine>,
) -> Outcome {
    run_model_with(
        model,
        inputs,
        runner,
        host,
        lookup,
        log,
        &mut |runner, host, tool, values, log| runner.run(host, tool, values, true, log),
    )
}

/// A model's step as it went on the copy ([`record_model`]): the job and
/// what it computed, or how it ended before computing.
#[derive(Clone)]
pub enum RecordedStep {
    Computed {
        job: Box<Job>,
        result: Box<RunResult>,
        canceled: bool,
    },
    Ended(Box<Outcome>),
}

/// Runs `model` on `host` (the drawing's reading copy, on any thread) as
/// [`run_model`] does, each step prepared, computed and applied there, and
/// records the steps for [`replay_model`]. `feedback` hears the steps'
/// progress as the whole model's share, and stops them; the steps' messages
/// go to `log` in the order a run here gives them.
pub fn record_model(
    model: &Model,
    inputs: &Values,
    host: &mut dyn Host,
    lookup: &dyn Fn(&str) -> Option<Tool>,
    feedback: &mut dyn Feedback,
    log: &mut Vec<LogLine>,
) -> Vec<RecordedStep> {
    let count = order_steps(model).map_or(0, |o| o.len()).max(1);
    let mut steps = Vec::new();
    let mut runner = Runner::new();
    run_model_with(
        model,
        inputs,
        &mut runner,
        host,
        lookup,
        log,
        &mut |runner, host, tool, values, log| {
            let job = match runner.prepare(&*host, tool, values, true, log) {
                Prepared::Ready(job) => job,
                Prepared::Done(outcome) => {
                    steps.push(RecordedStep::Ended(Box::new(outcome.clone())));
                    return outcome;
                }
            };
            let done = steps.len();
            let mut share = Share {
                inner: &mut *feedback,
                log: &mut *log,
                done,
                count,
            };
            let result = if share.canceled() {
                RunResult::default()
            } else {
                Runner::compute(&job, host.doc(), &mut share)
            };
            let canceled = share.canceled();
            share.progress(1.0, "");
            steps.push(RecordedStep::Computed {
                job: Box::new(job.clone()),
                result: Box::new(result.clone()),
                canceled,
            });
            runner.finish(host, job, result, canceled, log)
        },
    );
    steps
}

/// Applies recorded steps on the host's drawing as they went on the copy,
/// in one undo step, and records the model in `runner`'s history: on the
/// drawing the copy was made of, unchanged since, it ends as the recorded
/// run did. Messages of the replay go to `log`.
pub fn replay_model(
    model: &Model,
    inputs: &Values,
    runner: &mut Runner,
    host: &mut dyn Host,
    lookup: &dyn Fn(&str) -> Option<Tool>,
    steps: Vec<RecordedStep>,
    log: &mut Vec<LogLine>,
) -> Outcome {
    let mut steps = steps.into_iter();
    run_model_with(
        model,
        inputs,
        runner,
        host,
        lookup,
        log,
        &mut |runner, host, _tool, _values, log| match steps.next() {
            Some(RecordedStep::Computed {
                job,
                result,
                canceled,
            }) => runner.finish(host, *job, *result, canceled, log),
            Some(RecordedStep::Ended(outcome)) => *outcome,
            // More steps than were recorded: the drawing is not the copy's.
            None => Outcome::Invalid { issues: Vec::new() },
        },
    )
}

/// Ends a model that ran apart without touching the drawing: stopped
/// (Durdur, its drawing gone) or broken off (`why`); recorded in
/// `runner`'s history in the model's words.
pub fn end_model(
    model: &Model,
    inputs: &Values,
    runner: &mut Runner,
    lookup: &dyn Fn(&str) -> Option<Tool>,
    why: Option<&str>,
) -> Outcome {
    let (status, message) = match why {
        None => (
            Status::Canceled,
            "Model durduruldu; çizim değişmedi.".to_owned(),
        ),
        Some(why) => (
            Status::Error,
            format!("“{}” çalışırken hata: {why}", model.label),
        ),
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let record = runner.add_record(RunRecord {
        seq: 0,
        tool_id: model_as_tool(model, lookup).id,
        label: model.label.clone(),
        values: inputs.clone(),
        started: now,
        ms: 0,
        status,
        summary: message.clone(),
        added: Vec::new(),
        touched: Vec::new(),
        target: None,
    });
    Outcome::Stopped {
        status,
        message,
        record,
    }
}

/// A step's feedback as a share of the whole model (step `done` of
/// `count`); its messages join the model's log.
struct Share<'a> {
    inner: &'a mut dyn Feedback,
    log: &'a mut Vec<LogLine>,
    done: usize,
    count: usize,
}

impl Feedback for Share<'_> {
    fn files(&self) -> Option<std::sync::Arc<dyn crate::files::Files>> {
        self.inner.files()
    }

    fn progress(&mut self, fraction: f64, label: &str) {
        let share = (self.done as f64 + fraction.clamp(0.0, 1.0)) / self.count as f64;
        self.inner.progress(share, label);
    }

    fn info(&mut self, message: String) {
        self.log.push(LogLine {
            level: crate::runner::Level::Info,
            text: message,
        });
    }

    fn warn(&mut self, message: String) {
        self.log.push(LogLine::warn(message));
    }

    fn canceled(&self) -> bool {
        self.inner.canceled()
    }
}

/// [`run_model`] with each step run by `run_step`.
pub fn run_model_with(
    model: &Model,
    inputs: &Values,
    runner: &mut Runner,
    host: &mut dyn Host,
    lookup: &dyn Fn(&str) -> Option<Tool>,
    log: &mut Vec<LogLine>,
    run_step: &mut StepRun,
) -> Outcome {
    let as_tool = model_as_tool(model, lookup);
    let input_issues = runner.validate(&as_tool, inputs, host.doc());
    if !input_issues.is_empty() {
        return Outcome::Invalid {
            issues: input_issues,
        };
    }
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let clock = Instant::now();
    let record = |runner: &mut Runner,
                  status: Status,
                  summary: String,
                  added: Vec<Slot>,
                  touched: Vec<Slot>| {
        runner.add_record(RunRecord {
            seq: 0,
            tool_id: as_tool.id.clone(),
            label: model.label.clone(),
            values: inputs.clone(),
            started,
            ms: clock.elapsed().as_millis() as u64,
            status,
            summary,
            added,
            touched,
            target: None,
        })
    };
    let problems = check_model(model, lookup);
    if let Some(p) = problems.first() {
        let place = p
            .step
            .as_ref()
            .and_then(|id| model.steps.iter().find(|s| &s.id == id))
            .map(|s| format!("“{}” adımı: ", step_name(s, lookup)))
            .unwrap_or_default();
        let more = if problems.len() > 1 {
            format!(
                " ({} sorun daha var; modeli düzenleyin)",
                problems.len() - 1
            )
        } else {
            String::new()
        };
        let message = format!("Model çalıştırılamaz. {place}{}{more}", p.message);
        let record = record(
            runner,
            Status::Error,
            message.clone(),
            Vec::new(),
            Vec::new(),
        );
        return Outcome::Stopped {
            status: Status::Error,
            message,
            record,
        };
    }
    let order = order_steps(model).unwrap_or_default();
    if order.is_empty() {
        let message = "Modelde adım yok. Model tasarımcısında bir araç ekleyin.".to_owned();
        let record = record(
            runner,
            Status::Error,
            message.clone(),
            Vec::new(),
            Vec::new(),
        );
        return Outcome::Stopped {
            status: Status::Error,
            message,
            record,
        };
    }
    let group = host.doc_mut().begin_group(&model.label);
    let mut outputs: BTreeMap<String, Values> = BTreeMap::new();
    let mut added: Vec<Slot> = Vec::new();
    let mut touched: Vec<Slot> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut summaries: Vec<String> = Vec::new();
    let mut edited = false;
    let mut select: Option<Vec<Slot>> = None;
    for (i, id) in order.iter().enumerate() {
        let Some(step) = model.steps.iter().find(|s| &s.id == id) else {
            continue;
        };
        let Some(tool) = lookup(&step.tool) else {
            continue;
        };
        let name = format!("{}. adım ({})", i + 1, step_name(step, lookup));
        let mut values = default_values(&tool, &Defaults::of(host.doc()));
        for (param, src) in &step.values {
            values.insert(
                param.clone(),
                source_value(src, inputs, &outputs, model, lookup),
            );
        }
        match run_step(runner, host, &tool, &values, log) {
            Outcome::Ok {
                result,
                added: step_added,
                touched: step_touched,
                edited: step_edited,
                record,
            } => {
                outputs.insert(step.id.clone(), step_outputs(&tool, &result, &step_added));
                added.extend(step_added);
                for id in step_touched {
                    if seen.insert(id) {
                        touched.push(id);
                    }
                }
                edited |= step_edited;
                if result.select.is_some() {
                    select = result.select;
                }
                summaries.push(record.summary);
            }
            failed => {
                host.doc_mut().cancel_group(group);
                let (status, why) = match failed {
                    Outcome::Invalid { issues } => (
                        Status::Error,
                        issues
                            .into_iter()
                            .next()
                            .map(|i| i.message)
                            .unwrap_or_default(),
                    ),
                    Outcome::Stopped {
                        status, message, ..
                    } => (status, message),
                    Outcome::Ok { .. } => (Status::Error, String::new()),
                };
                let message = if status == Status::Canceled {
                    "Model durduruldu; çizim değişmedi.".to_owned()
                } else {
                    format!("{name} çalışmadı: {why} Önceki adımların sonuçları geri alındı.")
                };
                let record = record(runner, status, message.clone(), Vec::new(), Vec::new());
                return Outcome::Stopped {
                    status,
                    message,
                    record,
                };
            }
        }
    }
    host.doc_mut().end_group(group);
    let mut model_outputs = Values::new();
    for o in &model.outputs {
        if let Some(v) = outputs.get(&o.step).and_then(|out| out.get(&o.output)) {
            model_outputs.insert(o.name.clone(), v.clone());
        }
    }
    let summary = format!("{} adım çalıştı. {}", order.len(), summaries.join(" "));
    let result = RunResult {
        changes: None,
        select,
        outputs: model_outputs,
        summary: Some(summary.clone()),
        refused: None,
    };
    let record = record(runner, Status::Ok, summary, added.clone(), touched.clone());
    Outcome::Ok {
        result,
        added,
        touched,
        edited,
        record,
    }
}

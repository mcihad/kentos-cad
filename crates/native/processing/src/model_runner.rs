//! Runs a model (the web's `processing/modelRunner.ts`): its steps in
//! dependency order, each a normal tool run whose values come from fixed
//! values, the model's inputs or earlier steps' outputs (a features output
//! reaches the next step as the ids of the objects it made or chose). The
//! whole model is one undo step; if a step fails or is stopped, what the
//! earlier steps did is taken back.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::features::Host;
use crate::model::{Model, ModelStep, ValueSource, check_model, order_steps, step_name};
use crate::parameters::default_values;
use crate::runner::{LogLine, Outcome, RunRecord, Runner, Status};
use crate::types::{Defaults, OutputKind, RunResult, Target, Tool, Values};

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
        parameters: model.inputs.clone(),
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

/// Runs `model` with its inputs on the host's drawing, as one undo step.
pub fn run_model(
    model: &Model,
    inputs: &Values,
    runner: &mut Runner,
    host: &mut dyn Host,
    lookup: &dyn Fn(&str) -> Option<Tool>,
    log: &mut Vec<LogLine>,
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
        match runner.run(host, &tool, &values, true, log) {
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

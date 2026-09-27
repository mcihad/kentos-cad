//! Models (the web's `processing/model.ts`, QGIS "Model Designer"): tools
//! wired into a flow diagram. A step runs one tool; each of its parameters
//! takes a fixed value, one of the model's own inputs, or an output of an
//! earlier step — a features output becomes a `{ scope: 'ids' }` value, so
//! steps chain through the objects they create, choose or change.

use std::collections::{BTreeMap, HashSet};

use serde_json::Value;

use crate::parameters::is_visible;
use crate::types::{DefaultValue, ParamDef, ParamKind, Tool, Values};

/// Where a step parameter's value comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum ValueSource {
    Value(Value),
    Input(String),
    Output { step: String, output: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelStep {
    /// Unique within the model.
    pub id: String,
    /// The processing tool's id.
    pub tool: String,
    /// By parameter, in the order they were set.
    pub values: Vec<(String, ValueSource)>,
    /// The box's place in the diagram (px).
    pub position: Option<(f64, f64)>,
    /// A short caption in the diagram; the tool's name when none.
    pub caption: Option<String>,
}

impl ModelStep {
    pub fn source(&self, param: &str) -> Option<&ValueSource> {
        self.values
            .iter()
            .find(|(name, _)| name == param)
            .map(|(_, s)| s)
    }
}

/// A model output, taken from a step output.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelOutput {
    pub name: String,
    pub label: String,
    pub step: String,
    pub output: String,
}

#[derive(Clone)]
pub struct Model {
    pub id: String,
    pub label: String,
    pub category: String,
    pub description: String,
    /// What the user fills in when running it (the same definitions as tool parameters).
    pub inputs: Vec<ParamDef>,
    pub steps: Vec<ModelStep>,
    pub outputs: Vec<ModelOutput>,
    /// Where the input boxes are in the diagram (px), by input name.
    pub input_positions: BTreeMap<String, (f64, f64)>,
}

/// A problem that stops a model from running, with the step it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelIssue {
    pub step: Option<String>,
    pub message: String,
}

/// Whether a value of type `from` (a model input or a step output) can feed
/// a parameter of type `to`: the same type, a number or a choice where text
/// is written, and text into an expression or a field name.
pub fn can_feed(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    match to {
        "string" => from == "number" || from == "enum",
        "expression" | "field" => from == "string",
        _ => false,
    }
}

/// Required and with nothing to fall back on: a point, an expression, a
/// field or a text that may not be empty.
fn needs_source(p: &ParamDef) -> bool {
    if p.optional || p.default.is_some() {
        return false;
    }
    match p.kind {
        ParamKind::Point | ParamKind::Expression { .. } | ParamKind::Field { .. } => true,
        ParamKind::Text { allow_empty, .. } => !allow_empty,
        _ => false,
    }
}

/// How a step is named in messages and in the diagram.
pub fn step_name(step: &ModelStep, lookup: &dyn Fn(&str) -> Option<Tool>) -> String {
    step.caption
        .clone()
        .filter(|c| !c.is_empty())
        .or_else(|| lookup(&step.tool).map(|t| t.label))
        .unwrap_or_else(|| step.tool.clone())
}

/// Problems that stop a model from running: unknown tools, parameters,
/// inputs, outputs, cycles.
pub fn check_model(model: &Model, lookup: &dyn Fn(&str) -> Option<Tool>) -> Vec<ModelIssue> {
    let mut issues = Vec::new();
    let ids: HashSet<&str> = model.steps.iter().map(|s| s.id.as_str()).collect();
    if ids.len() != model.steps.len() {
        issues.push(ModelIssue {
            step: None,
            message: "Adım kimlikleri benzersiz olmalı.".into(),
        });
    }
    for s in &model.steps {
        let Some(tool) = lookup(&s.tool) else {
            issues.push(ModelIssue {
                step: Some(s.id.clone()),
                message: format!("Bilinmeyen işlem aracı: {}", s.tool),
            });
            continue;
        };
        // What is known before running: fixed values and fixed defaults (for visibility).
        let known: Values = tool
            .parameters
            .iter()
            .filter_map(|p| {
                let value = match (s.source(&p.name), &p.default) {
                    (Some(ValueSource::Value(v)), _) | (_, Some(DefaultValue::Value(v))) => {
                        Some(v.clone())
                    }
                    _ => None,
                };
                value.map(|v| (p.name.clone(), v))
            })
            .collect();
        let issue = |message: String| ModelIssue {
            step: Some(s.id.clone()),
            message,
        };
        for p in &tool.parameters {
            match s.source(&p.name) {
                None => {
                    if needs_source(p) && is_visible(p, &known) {
                        issues.push(issue(format!(
                            "“{}” için bir değer ya da bağlantı gerekiyor.",
                            p.label
                        )));
                    }
                }
                Some(ValueSource::Input(name)) => {
                    match model.inputs.iter().find(|i| &i.name == name) {
                        None => issues.push(issue(format!(
                            "“{}” olmayan bir model girdisine bağlı: {name}",
                            p.label
                        ))),
                        Some(input) if !can_feed(input.type_name(), p.type_name()) => {
                            issues.push(issue(format!(
                                "“{}” “{}” girdisinden beslenemez: türler uymuyor.",
                                p.label, input.label
                            )));
                        }
                        Some(_) => {}
                    }
                }
                Some(ValueSource::Output { step, output }) => {
                    let out = model
                        .steps
                        .iter()
                        .find(|x| &x.id == step)
                        .and_then(|from| lookup(&from.tool))
                        .and_then(|t| t.outputs.into_iter().find(|o| &o.name == output));
                    match out {
                        None => issues.push(issue(format!(
                            "“{}” olmayan bir çıktıya bağlı: {step}.{output}",
                            p.label
                        ))),
                        Some(out) if !can_feed(out.kind.type_name(), p.type_name()) => {
                            issues.push(issue(format!(
                                "“{}” “{}” çıktısından beslenemez: türler uymuyor.",
                                p.label, out.label
                            )));
                        }
                        Some(_) => {}
                    }
                }
                Some(ValueSource::Value(_)) => {}
            }
        }
    }
    if let Err(message) = order_steps(model) {
        issues.push(ModelIssue {
            step: None,
            message,
        });
    }
    issues
}

/// Steps in an order where every step comes after the steps it reads from
/// (Kahn, as the web walks it), or the cycle.
pub fn order_steps(model: &Model) -> Result<Vec<String>, String> {
    let mut deps: Vec<(String, Vec<String>)> = model
        .steps
        .iter()
        .map(|s| {
            let mut from: Vec<String> = Vec::new();
            for (_, v) in &s.values {
                if let ValueSource::Output { step, .. } = v
                    && !from.contains(step)
                {
                    from.push(step.clone());
                }
            }
            (s.id.clone(), from)
        })
        .collect();
    let mut order: Vec<String> = Vec::new();
    let mut ready: std::collections::VecDeque<String> = deps
        .iter()
        .filter(|(_, d)| d.is_empty())
        .map(|(id, _)| id.clone())
        .collect();
    while let Some(id) = ready.pop_front() {
        order.push(id.clone());
        for (other, d) in &mut deps {
            let before = d.len();
            d.retain(|x| x != &id);
            if d.len() != before && d.is_empty() {
                ready.push_back(other.clone());
            }
        }
    }
    if order.len() == model.steps.len() {
        return Ok(order);
    }
    let stuck: Vec<&str> = model
        .steps
        .iter()
        .filter(|s| !order.contains(&s.id))
        .map(|s| s.id.as_str())
        .collect();
    Err(format!(
        "Modelde döngü var: {} birbirini bekliyor.",
        stuck.join(" → ")
    ))
}

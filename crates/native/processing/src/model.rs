//! Models (the web's `processing/model.ts`, QGIS "Model Designer"): tools
//! wired into a flow diagram. A step runs one tool; each of its parameters
//! takes a fixed value, one of the model's own inputs, or an output of an
//! earlier step — a features output becomes a `{ scope: 'ids' }` value, so
//! steps chain through the objects they create, choose or change.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use serde::Deserialize;
use serde::de::{MapAccess, Visitor};
use serde_json::{Map, Value, json};

use crate::parameters::is_visible;
use crate::types::{DefaultValue, ParamDef, ParamKind, Tool, Values};
use crate::web_param::param_from_json;

/// Where a step parameter's value comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum ValueSource {
    Value(Value),
    Input(String),
    Output { step: String, output: String },
}

impl ValueSource {
    /// As the web writes it: `{ kind: 'value', value }`, `{ kind: 'input', name }`,
    /// `{ kind: 'output', step, output }`.
    pub fn to_json(&self) -> Value {
        match self {
            ValueSource::Value(v) => json!({ "kind": "value", "value": js_numbers(v) }),
            ValueSource::Input(name) => json!({ "kind": "input", "name": name }),
            ValueSource::Output { step, output } => {
                json!({ "kind": "output", "step": step, "output": output })
            }
        }
    }

    pub fn from_json(v: &Value) -> Option<Self> {
        let text = |key: &str| v.get(key).and_then(Value::as_str).map(str::to_owned);
        match v.get("kind")?.as_str()? {
            "value" => Some(ValueSource::Value(
                v.get("value").cloned().unwrap_or(Value::Null),
            )),
            "input" => Some(ValueSource::Input(text("name")?)),
            "output" => Some(ValueSource::Output {
                step: text("step")?,
                output: text("output")?,
            }),
            _ => None,
        }
    }
}

/// A value with its whole numbers written as JavaScript writes them (`1`,
/// not `1.0`): JSON has one kind of number, serde_json two.
pub fn js_numbers(v: &Value) -> Value {
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 9.007_199_254_740_992e15 => {
                json!(f as i64)
            }
            _ => v.clone(),
        },
        Value::Array(list) => Value::Array(list.iter().map(js_numbers).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), js_numbers(v)))
                .collect(),
        ),
        _ => v.clone(),
    }
}

fn point_json((x, y): (f64, f64)) -> Value {
    js_numbers(&json!({ "x": x, "y": y }))
}

fn point_of(v: &Value) -> Option<(f64, f64)> {
    Some((v.get("x")?.as_f64()?, v.get("y")?.as_f64()?))
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

    /// As the web writes it; `values` in the order they were set.
    pub fn to_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("id".into(), json!(self.id));
        o.insert("tool".into(), json!(self.tool));
        o.insert(
            "values".into(),
            Value::Object(
                self.values
                    .iter()
                    .map(|(name, src)| (name.clone(), src.to_json()))
                    .collect(),
            ),
        );
        if let Some(at) = self.position {
            o.insert("position".into(), point_json(at));
        }
        if let Some(caption) = &self.caption {
            o.insert("caption".into(), json!(caption));
        }
        Value::Object(o)
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

/// A model input as the web keeps it: a parameter definition in the web's
/// JSON shape (web_param.rs), its fields as written, so that a model saved
/// and read again is the same model.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelInput(pub Map<String, Value>);

impl ModelInput {
    fn text(&self, key: &str) -> &str {
        self.0.get(key).and_then(Value::as_str).unwrap_or("")
    }

    pub fn name(&self) -> &str {
        self.text("name")
    }

    pub fn label(&self) -> &str {
        self.text("label")
    }

    /// The web's `type`: what sources are matched by.
    pub fn type_name(&self) -> &str {
        self.text("type")
    }

    /// The parameter it is, as the model's dialog asks it.
    pub fn def(&self) -> Option<ParamDef> {
        param_from_json(&Value::Object(self.0.clone()))
    }
}

#[derive(Clone)]
pub struct Model {
    pub id: String,
    pub label: String,
    pub category: String,
    pub description: String,
    /// What the user fills in when running it (the same definitions as tool parameters).
    pub inputs: Vec<ModelInput>,
    pub steps: Vec<ModelStep>,
    pub outputs: Vec<ModelOutput>,
    /// Where the input boxes are in the diagram (px), by input name.
    pub input_positions: BTreeMap<String, (f64, f64)>,
}

impl Model {
    /// As the web writes it (its `ProcessingModel`).
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "label": self.label,
            "category": self.category,
            "description": self.description,
            "inputs": self.inputs.iter().map(|i| js_numbers(&Value::Object(i.0.clone()))).collect::<Vec<_>>(),
            "steps": self.steps.iter().map(ModelStep::to_json).collect::<Vec<_>>(),
            "outputs": self.outputs.iter().map(|o| json!({
                "name": o.name,
                "label": o.label,
                "from": { "step": o.step, "output": o.output },
            })).collect::<Vec<_>>(),
            "inputPositions": self.input_positions.iter()
                .map(|(name, at)| (name.clone(), point_json(*at)))
                .collect::<Map<String, Value>>(),
        })
    }

    /// A model from the web's JSON text, its steps' values in the order
    /// they are written (as JavaScript reads them); none when it is not one.
    pub fn from_text(text: &str) -> Option<Model> {
        serde_json::from_str(text).ok()
    }
}

/// An object's entries in the order they are written: serde_json's own map
/// sorts them, JavaScript's keeps them.
#[derive(Default)]
struct Ordered(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for Ordered {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Ordered;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Ordered, A::Error> {
                let mut out = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, Value>()? {
                    out.push((k, v));
                }
                Ok(Ordered(out))
            }
        }
        d.deserialize_map(Entries)
    }
}

#[derive(Deserialize)]
struct WireStep {
    #[serde(default)]
    id: String,
    #[serde(default)]
    tool: String,
    #[serde(default)]
    values: Ordered,
    #[serde(default)]
    position: Option<Value>,
    #[serde(default)]
    caption: Option<String>,
}

#[derive(Deserialize)]
struct WireModel {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    inputs: Vec<Value>,
    #[serde(default)]
    steps: Vec<WireStep>,
    #[serde(default)]
    outputs: Vec<Value>,
    #[serde(rename = "inputPositions", default)]
    input_positions: Option<Map<String, Value>>,
}

impl<'de> Deserialize<'de> for Model {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let w = WireModel::deserialize(d)?;
        Ok(Model {
            id: w.id,
            label: w.label,
            category: w.category,
            description: w.description,
            inputs: w
                .inputs
                .into_iter()
                .filter_map(|v| match v {
                    Value::Object(map) => Some(ModelInput(map)),
                    _ => None,
                })
                .collect(),
            steps: w
                .steps
                .into_iter()
                .map(|s| ModelStep {
                    id: s.id,
                    tool: s.tool,
                    values: s
                        .values
                        .0
                        .iter()
                        .filter_map(|(k, v)| Some((k.clone(), ValueSource::from_json(v)?)))
                        .collect(),
                    position: s.position.as_ref().and_then(point_of),
                    caption: s.caption,
                })
                .collect(),
            outputs: w
                .outputs
                .iter()
                .filter_map(|o| {
                    let text = |v: Option<&Value>| v.and_then(Value::as_str).map(str::to_owned);
                    Some(ModelOutput {
                        name: text(o.get("name"))?,
                        label: text(o.get("label")).unwrap_or_default(),
                        step: text(o.get("from").and_then(|f| f.get("step")))?,
                        output: text(o.get("from").and_then(|f| f.get("output")))?,
                    })
                })
                .collect(),
            input_positions: w
                .input_positions
                .unwrap_or_default()
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), point_of(v)?)))
                .collect(),
        })
    }
}

/// A problem that stops a model from running, with the step it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelIssue {
    pub step: Option<String>,
    pub message: String,
}

impl ModelIssue {
    /// As the web writes it: `{ step?, message }`.
    pub fn to_json(&self) -> Value {
        match &self.step {
            Some(step) => json!({ "step": step, "message": self.message }),
            None => json!({ "message": self.message }),
        }
    }
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
                    match model.inputs.iter().find(|i| i.name() == name) {
                        None => issues.push(issue(format!(
                            "“{}” olmayan bir model girdisine bağlı: {name}",
                            p.label
                        ))),
                        Some(input) if !can_feed(input.type_name(), p.type_name()) => {
                            issues.push(issue(format!(
                                "“{}” “{}” girdisinden beslenemez: türler uymuyor.",
                                p.label,
                                input.label()
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

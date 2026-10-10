//! Editing a model draft: the model designer's operations (the web's
//! processing/modelEdit.ts), pure so they are tested without the UI. Every
//! function changes the draft in place; the designer keeps a copy before
//! each change for its own undo. fixtures/processing/v1/designer.json
//! replays them step by step on both platforms.

use std::collections::HashSet;

use serde_json::{Map, json};

use crate::model::{Model, ModelInput, ModelOutput, ModelStep, ValueSource, step_name};
use crate::parameters::default_value;
use crate::text::fold_turkish;
use crate::types::{Defaults, ParamDef, ParamKind, Tool};

/// A box of the diagram: a model input or a step.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeRef {
    Input(String),
    Step(String),
}

/// A model input the designer offers: its type, the button's word, its
/// icon (the web's name) and its tip.
pub struct InputType {
    pub type_name: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
}

/// The kinds of model input the designer offers, in the web's order.
pub const INPUT_TYPES: [InputType; 6] = [
    InputType {
        type_name: "features",
        label: "Nesneler",
        icon: "select",
        description: "Seçili, görünen, bütün nesneler ya da bir katman",
    },
    InputType {
        type_name: "number",
        label: "Sayı",
        icon: "units",
        description: "Mesafe, adet, ondalık basamak …",
    },
    InputType {
        type_name: "string",
        label: "Metin",
        icon: "text",
        description: "Önek, alan adı, ifade …",
    },
    InputType {
        type_name: "boolean",
        label: "Evet / hayır",
        icon: "check",
        description: "Açık ya da kapalı bir seçenek",
    },
    InputType {
        type_name: "layer",
        label: "Katman",
        icon: "layers",
        description: "Sonuçların yazılacağı katman",
    },
    InputType {
        type_name: "point",
        label: "Nokta",
        icon: "point",
        description: "Haritada gösterilen bir nokta",
    },
];

/// An identifier from a Turkish label: “Nokta öneki” → `noktaOneki`; one
/// that does not start with a letter gets `g` before it.
pub fn slug(label: &str) -> String {
    let folded = fold_turkish(label).to_lowercase();
    let words = folded
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|w| !w.is_empty());
    let mut s = String::new();
    for (i, w) in words.enumerate() {
        if i == 0 {
            s.push_str(w);
        } else {
            let mut chars = w.chars();
            if let Some(first) = chars.next() {
                s.extend(first.to_uppercase());
                s.push_str(chars.as_str());
            }
        }
    }
    if s.starts_with(|c: char| c.is_ascii_lowercase()) {
        s
    } else {
        format!("g{s}")
    }
}

/// `base`, or `base2`, `base3` … the first one not taken.
fn unique(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut name = base.to_owned();
    let mut k = 2;
    while taken(&name) {
        name = format!("{base}{k}");
        k += 1;
    }
    name
}

/// A new, empty model (the web's `newModel`) under `id`.
pub fn new_model(id: String) -> Model {
    Model {
        id,
        label: "Yeni model".into(),
        category: "points".into(),
        description: String::new(),
        inputs: Vec::new(),
        steps: Vec::new(),
        outputs: Vec::new(),
        input_positions: Default::default(),
    }
}

/// A copy of a model under a new id (to change a built-in one).
pub fn copy_model(model: &Model, id: String) -> Model {
    Model {
        id,
        label: copy_label(&model.label),
        ..model.clone()
    }
}

/// A copy's name: “<ad> (kopya)”.
pub fn copy_label(label: &str) -> String {
    format!("{label} (kopya)")
}

/// Adds an input of a type; its name. `at`: where its box goes (else in
/// the first column, under the others).
pub fn add_input(
    model: &mut Model,
    type_name: &str,
    label: &str,
    at: Option<(f64, f64)>,
) -> String {
    let name = unique(&slug(label), |n| model.inputs.iter().any(|i| i.name() == n));
    let mut def = Map::new();
    def.insert("type".into(), json!(type_name));
    def.insert("name".into(), json!(name));
    def.insert("label".into(), json!(label));
    match type_name {
        "features" => {
            def.insert("default".into(), json!({ "scope": "selection" }));
        }
        "number" => {
            def.insert("default".into(), json!(0));
        }
        "string" => {
            def.insert("default".into(), json!(""));
            def.insert("allowEmpty".into(), json!(true));
        }
        "boolean" => {
            def.insert("default".into(), json!(false));
        }
        "layer" => {
            def.insert("default".into(), json!({ "newName": label }));
        }
        _ => {}
    }
    model.inputs.push(ModelInput(def));
    let place = at.unwrap_or((40.0, 40.0 + (model.inputs.len() - 1) as f64 * 90.0));
    model.input_positions.insert(name.clone(), place);
    name
}

/// Removes an input; the steps that read it fall back to the tool's default.
pub fn remove_input(model: &mut Model, name: &str) {
    model.inputs.retain(|i| i.name() != name);
    model.input_positions.remove(name);
    for s in &mut model.steps {
        s.values
            .retain(|(_, src)| !matches!(src, ValueSource::Input(n) if n == name));
    }
}

/// Adds a step running a tool; its id. `from` (the box the user had
/// selected): the new step's first parameter it can feed is connected to it,
/// so adding tools one after another builds a chain.
pub fn add_step(
    model: &mut Model,
    tool_id: &str,
    lookup: &dyn Fn(&str) -> Option<Tool>,
    at: Option<(f64, f64)>,
    from: Option<&NodeRef>,
) -> String {
    let tool = lookup(tool_id);
    let base = slug(tool.as_ref().map_or(tool_id, |t| t.label.as_str()));
    let id = unique(&base, |n| model.steps.iter().any(|s| s.id == n));
    let position = Some(at.unwrap_or_else(|| next_free_spot(model)));
    model.steps.push(ModelStep {
        id: id.clone(),
        tool: tool_id.to_owned(),
        values: Vec::new(),
        position,
        caption: None,
    });
    if let (Some(tool), Some(from)) = (&tool, from) {
        for p in &tool.parameters {
            let found =
                sources_for(model, &id, p, lookup)
                    .into_iter()
                    .find(|o| match (from, &o.src) {
                        (NodeRef::Input(name), ValueSource::Input(n)) => n == name,
                        (NodeRef::Step(step), ValueSource::Output { step: s, .. }) => s == step,
                        _ => false,
                    });
            if let Some(found) = found {
                set_source(model, &id, &p.name, Some(found.src));
                break;
            }
        }
    }
    id
}

/// Where a step goes when none is given: right of the rightmost, else (300, 60).
fn next_free_spot(model: &Model) -> (f64, f64) {
    let x = model
        .steps
        .iter()
        .map(|s| s.position.map_or(0.0, |p| p.0))
        .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x))));
    (x.map_or(300.0, |x| x + 280.0), 60.0)
}

/// Removes a step, the connections that read its outputs and the model
/// outputs taken from it.
pub fn remove_step(model: &mut Model, id: &str) {
    model.steps.retain(|s| s.id != id);
    for s in &mut model.steps {
        s.values
            .retain(|(_, src)| !matches!(src, ValueSource::Output { step, .. } if step == id));
    }
    model.outputs.retain(|o| o.step != id);
}

/// The model input type a parameter can become (“Yeni model girdisi yap”);
/// none for a choice from a fixed list.
pub fn input_type_for(p: &ParamDef) -> Option<&'static str> {
    match p.kind {
        ParamKind::Features { .. } => Some("features"),
        ParamKind::Number { .. } => Some("number"),
        ParamKind::Text { .. } | ParamKind::Expression { .. } | ParamKind::Field { .. } => {
            Some("string")
        }
        ParamKind::Boolean => Some("boolean"),
        ParamKind::Layer { .. } => Some("layer"),
        ParamKind::Point => Some("point"),
        // A file's rows are not kept in a model (docs/adr/0200 §7).
        ParamKind::Choice { .. }
        | ParamKind::File { .. }
        | ParamKind::SaveFile { .. }
        | ParamKind::Network { .. }
        | ParamKind::RasterValues { .. }
        | ParamKind::RasterPairs { .. } => None,
    }
}

/// “Yeni model girdisi yap”: a model input made from a step's parameter, a
/// column left of the step, and the parameter fed by it. The input takes
/// the parameter's label and default (the step's fixed value is not carried
/// over); a features input its kinds, a number its limits, whole-number rule
/// and unit, a text whether it may be empty. Its name, or none when the
/// parameter cannot become one (a choice from a list).
pub fn input_from_param(
    model: &mut Model,
    step_id: &str,
    p: &ParamDef,
    defaults: &Defaults,
) -> Option<String> {
    let type_name = input_type_for(p)?;
    let position = model.steps.iter().find(|s| s.id == step_id)?.position;
    let at = (
        position.map_or(300.0, |p| p.0) - 290.0,
        position.map_or(40.0, |p| p.1),
    );
    let name = add_input(model, type_name, &p.label, Some(at));
    let d = default_value(p, defaults);
    let created = &mut model.inputs.last_mut()?.0;
    match (&p.kind, type_name) {
        (ParamKind::Features { kinds, .. }, _) => {
            match kinds {
                Some(kinds) => created.insert("kinds".into(), json!(kinds)),
                None => created.remove("kinds"),
            };
            created.insert("default".into(), d);
        }
        (
            ParamKind::Number {
                min,
                max,
                integer,
                unit,
                ..
            },
            _,
        ) => {
            created.insert("default".into(), d);
            for (key, v) in [("min", *min), ("max", *max)] {
                match v {
                    Some(v) => created.insert(key.into(), json!(v)),
                    None => created.remove(key),
                };
            }
            if *integer {
                created.insert("integer".into(), json!(true));
            }
            if !unit.is_empty() {
                created.insert("unit".into(), json!(unit));
            }
        }
        (kind, "string") => {
            created.insert("default".into(), if d.is_string() { d } else { json!("") });
            match kind {
                ParamKind::Text { allow_empty, .. } if *allow_empty => {
                    created.insert("allowEmpty".into(), json!(true));
                }
                _ => {
                    created.remove("allowEmpty");
                }
            }
        }
        _ => {
            if !d.is_null() {
                created.insert("default".into(), d);
            }
        }
    }
    set_source(
        model,
        step_id,
        &p.name,
        Some(ValueSource::Input(name.clone())),
    );
    Some(name)
}

/// A step's caption as typed: trimmed; a blank one goes back to the tool's name.
pub fn set_caption(model: &mut Model, step_id: &str, text: &str) {
    if let Some(step) = model.steps.iter_mut().find(|s| s.id == step_id) {
        let text = crate::text::js_trim(text);
        step.caption = (!text.is_empty()).then(|| text.to_owned());
    }
}

/// Where a step parameter gets its value; `None` goes back to the tool's
/// default. A parameter set again keeps its place among the others (as a
/// JavaScript object keeps a key's).
pub fn set_source(model: &mut Model, step_id: &str, param: &str, src: Option<ValueSource>) {
    let Some(step) = model.steps.iter_mut().find(|s| s.id == step_id) else {
        return;
    };
    match src {
        Some(src) => match step.values.iter_mut().find(|(name, _)| name == param) {
            Some(slot) => slot.1 = src,
            None => step.values.push((param.to_owned(), src)),
        },
        None => step.values.retain(|(name, _)| name != param),
    }
}

/// Makes a step's output a model output: under the output's own name, or,
/// when a model output has that name already, the name with the count of
/// model outputs plus one after it. The name, or none when the step or its
/// output is not there.
pub fn add_output(
    model: &mut Model,
    step_id: &str,
    output: &str,
    lookup: &dyn Fn(&str) -> Option<Tool>,
) -> Option<String> {
    let step = model.steps.iter().find(|s| s.id == step_id)?;
    let out = lookup(&step.tool)?
        .outputs
        .into_iter()
        .find(|o| o.name == output)?;
    let name = if model.outputs.iter().any(|o| o.name == output) {
        format!("{output}{}", model.outputs.len() + 1)
    } else {
        output.to_owned()
    };
    model.outputs.push(ModelOutput {
        name: name.clone(),
        label: out.label,
        step: step_id.to_owned(),
        output: output.to_owned(),
    });
    Some(name)
}

/// A source a parameter can take, for menus: where it comes from (“Girdi”
/// or the step's name).
#[derive(Clone, Debug, PartialEq)]
pub struct SourceOption {
    pub src: ValueSource,
    pub label: String,
    pub group: String,
}

/// The inputs and earlier outputs that can feed a parameter: never the step
/// itself, never a step that reads from it (no cycle).
pub fn sources_for(
    model: &Model,
    step_id: &str,
    param: &ParamDef,
    lookup: &dyn Fn(&str) -> Option<Tool>,
) -> Vec<SourceOption> {
    let mut out = Vec::new();
    for i in &model.inputs {
        if crate::model::can_feed(i.type_name(), param.type_name()) {
            out.push(SourceOption {
                src: ValueSource::Input(i.name().to_owned()),
                label: i.label().to_owned(),
                group: "Girdi".into(),
            });
        }
    }
    let downstream = dependents(model, step_id);
    for s in &model.steps {
        if s.id == step_id || downstream.contains(&s.id) {
            continue;
        }
        for o in lookup(&s.tool).map(|t| t.outputs).unwrap_or_default() {
            if crate::model::can_feed(o.kind.type_name(), param.type_name()) {
                out.push(SourceOption {
                    src: ValueSource::Output {
                        step: s.id.clone(),
                        output: o.name.clone(),
                    },
                    label: o.label.clone(),
                    group: step_name(s, lookup),
                });
            }
        }
    }
    out
}

/// The steps that read (directly or not) from a step: they cannot feed it.
fn dependents(model: &Model, step_id: &str) -> HashSet<String> {
    let mut found = HashSet::new();
    let mut todo = vec![step_id.to_owned()];
    while let Some(id) = todo.pop() {
        for s in &model.steps {
            if !found.contains(&s.id)
                && s.values
                    .iter()
                    .any(|(_, v)| matches!(v, ValueSource::Output { step, .. } if *step == id))
            {
                found.insert(s.id.clone());
                todo.push(s.id.clone());
            }
        }
    }
    found
}

/// One edge of the diagram: a source and the step it feeds, with the
/// parameters it feeds there.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelEdge {
    pub from: NodeRef,
    pub to: String,
    pub params: Vec<String>,
}

/// One edge per (source, target step) pair, in the order the steps and
/// their values come.
pub fn edges_of(model: &Model) -> Vec<ModelEdge> {
    let mut edges: Vec<ModelEdge> = Vec::new();
    for s in &model.steps {
        for (param, src) in &s.values {
            let from = match src {
                ValueSource::Value(_) => continue,
                ValueSource::Input(name) => NodeRef::Input(name.clone()),
                ValueSource::Output { step, .. } => NodeRef::Step(step.clone()),
            };
            match edges.iter_mut().find(|e| e.from == from && e.to == s.id) {
                Some(e) => e.params.push(param.clone()),
                None => edges.push(ModelEdge {
                    from,
                    to: s.id.clone(),
                    params: vec![param.clone()],
                }),
            }
        }
    }
    edges
}

/// Lays the diagram out in columns: the inputs, then each step one column
/// after what it reads.
pub fn auto_layout(model: &mut Model) {
    let order = crate::model::order_steps(model)
        .unwrap_or_else(|_| model.steps.iter().map(|s| s.id.clone()).collect());
    let mut depth: Vec<(String, u32)> = Vec::new();
    let depth_of = |depth: &[(String, u32)], id: &str| {
        depth.iter().find(|(d, _)| d == id).map_or(0, |(_, n)| *n)
    };
    for id in &order {
        let Some(s) = model.steps.iter().find(|x| &x.id == id) else {
            continue;
        };
        let deps: Vec<u32> = s
            .values
            .iter()
            .filter_map(|(_, v)| match v {
                ValueSource::Output { step, .. } => Some(depth_of(&depth, step)),
                _ => None,
            })
            .collect();
        let d = deps.iter().max().map_or(1, |m| m + 1);
        depth.push((id.clone(), d));
    }
    let mut rows: Vec<(u32, u32)> = Vec::new();
    let mut place = |col: u32| {
        let row = match rows.iter_mut().find(|(c, _)| *c == col) {
            Some((_, r)) => {
                *r += 1;
                *r - 1
            }
            None => {
                rows.push((col, 1));
                0
            }
        };
        (40.0 + f64::from(col) * 290.0, 40.0 + f64::from(row) * 100.0)
    };
    model.input_positions = model
        .inputs
        .iter()
        .map(|i| (i.name().to_owned(), place(0)))
        .collect();
    for id in &order {
        let col = depth.iter().find(|(d, _)| d == id).map_or(1, |(_, n)| *n);
        let at = place(col);
        if let Some(s) = model.steps.iter_mut().find(|x| &x.id == id) {
            s.position = Some(at);
        }
    }
}

//! The forms a layer enforces, kept by the document (docs/adr/0402 §4): after a step, the objects it
//! added to or changed on such a layer are completed before the step is recorded, in the same step
//! (as linked texts and associative dimensions follow): a new object takes the form's defaults, every
//! one its calculations. Every way into the drawing (a tool, Python, MCP, an import) gets them alike.
//! A new object's missing required values are not refused here: `cad.forms.validate` says them and the
//! desktop opens the object's form. An expression reading a value only a host gives (the moment, the
//! user, the coordinate system's name) is left out ([`kentos_forms::HOST_ONLY`]).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{FormBody, FormItem, ProjectForm, ProjectSettings, VariableValue};
use kentos_expression::{Unit, Value, Variable};
use kentos_forms::{OtherLayer, Prepared, World};
use uuid::Uuid;

use crate::Document;
use crate::history::Op;
use crate::store::Stored;

impl Document {
    /// The other layers `body` reads (docs/adr/0404): its fields' lists and its related items, each
    /// with its fields and its objects' persistent ids and attributes.
    pub fn form_world(&self, body: &FormBody) -> World {
        let mut world = World::new();
        for (_, item) in body.walk() {
            let layer = match item {
                FormItem::Field(f) => f.lookup.as_ref().map(|l| l.layer.as_str()),
                FormItem::Related { layer, .. } => Some(layer.as_str()),
                _ => None,
            };
            let Some(layer) = layer else { continue };
            if world.contains_key(layer) {
                continue;
            }
            let Some(node) = self.layers.get(layer) else {
                continue;
            };
            world.insert(
                layer.to_owned(),
                OtherLayer {
                    name: node.name.clone(),
                    fields: node.fields.clone(),
                    rows: self
                        .by_layer_with_uids(layer)
                        .map(|(uid, e)| (uid.to_string(), e.base().attrs.clone()))
                        .collect(),
                },
            );
        }
        world
    }

    /// The ops that complete the records `ops` (a step about to be recorded, applied) added or changed
    /// on a layer whose form enforces its rules, applied: the step takes them in.
    pub(crate) fn follow_forms(&mut self, ops: &[Op]) -> Vec<Op> {
        let enforcing: Vec<&ProjectForm> = self
            .settings
            .forms
            .iter()
            .filter(|f| f.enforce && f.release().is_some())
            .collect();
        if enforcing.is_empty() {
            return Vec::new();
        }
        let layers: HashSet<&str> = enforcing.iter().map(|f| f.layer.as_str()).collect();
        // The objects the step left on such a layer, in the order they came, and whether each is new.
        let mut order: Vec<Uuid> = Vec::new();
        let mut new: HashMap<Uuid, bool> = HashMap::new();
        for op in ops {
            match op {
                Op::Add(s) | Op::Update { after: s, .. }
                    if layers.contains(s.entity.base().layer_id.as_str()) =>
                {
                    let added = matches!(op, Op::Add(_));
                    match new.get_mut(&s.uid) {
                        Some(was) => *was |= added,
                        None => {
                            order.push(s.uid);
                            new.insert(s.uid, added);
                        }
                    }
                }
                Op::Remove(s) => {
                    new.remove(&s.uid);
                }
                _ => {}
            }
        }
        if new.is_empty() {
            return Vec::new();
        }
        let variables = document_variables(&self.name, &self.settings);
        let unit = Unit::per_metre(self.settings.unit().per_metre());
        let prepared: Vec<(String, Prepared)> = enforcing
            .iter()
            .filter_map(|f| {
                let body = &f.release()?.body;
                let fields = self
                    .layers
                    .get(&f.layer)
                    .map_or(&[][..], |n| n.fields.as_slice());
                let node = self.layers.get(&f.layer);
                let subtypes = node.and_then(|n| n.subtypes.as_ref());
                let groups = node.map_or(&[][..], |n| n.contingents.as_slice());
                Prepared::new_with(body, fields, &variables, unit, &self.form_world(body))
                    .ok()
                    .map(|p| p.with_subtypes(subtypes).with_contingents(groups))
                    .map(|p| (f.layer.clone(), p))
            })
            .collect();
        let mut out = Vec::new();
        for uid in order {
            let Some(&added) = new.get(&uid) else {
                continue;
            };
            let Some(current) = self
                .store
                .slot_of(uid)
                .and_then(|slot| self.store.get(slot))
                .cloned()
            else {
                continue;
            };
            let layer = current.entity.base().layer_id.clone();
            let name = self
                .layers
                .get(&layer)
                .map_or_else(String::new, |n| n.name.clone());
            let id = f64::from(current.slot().0);
            let mut entity = (*current.entity).clone();
            for (_, p) in prepared.iter().filter(|(l, _)| *l == layer) {
                let mut object = kentos_forms::object_of(&entity, &name, id);
                object.uid = Some(current.uid.to_string());
                let (attrs, _) = p.complete(&entity.base().attrs, &object, added);
                entity.base_mut().attrs = attrs;
            }
            if entity.base().attrs != current.entity.base().attrs {
                out.push(Op::Update {
                    after: Stored {
                        uid: current.uid,
                        entity: Arc::new(entity),
                    },
                    before: current,
                });
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}

/// The `@` values the document knows: the project's own and the built-in ones that need no register,
/// as `kentos_project::variables::expression_variables` gives them (the coordinate system's name, the
/// moment and the user are a host's).
fn document_variables(name: &str, settings: &ProjectSettings) -> Vec<Variable> {
    let mut out: Vec<Variable> = settings
        .variables
        .iter()
        .map(|v| Variable {
            name: v.name.clone(),
            value: match &v.value {
                VariableValue::Null => Value::Null,
                VariableValue::Bool(b) => Value::Bool(*b),
                VariableValue::Number(x) => Value::Num(*x),
                VariableValue::Text(t) => Value::text(t.clone()),
            },
            description: String::new(),
        })
        .collect();
    let epsg = if settings.srid != 0 && settings.custom_crs.is_none() {
        Value::Num(f64::from(settings.srid))
    } else {
        Value::Null
    };
    for (n, value) in [
        ("proje_adi", Value::text(name.to_owned())),
        ("epsg", epsg),
        ("olcek", Value::Num(settings.plot_scale)),
    ] {
        out.push(Variable {
            name: n.to_owned(),
            value,
            description: String::new(),
        });
    }
    out
}

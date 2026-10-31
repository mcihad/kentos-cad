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
        // The layers a spatial list reads: their objects' shapes too (docs/adr/0439 §5).
        let spatial: HashSet<&str> = body
            .walk()
            .into_iter()
            .filter_map(|(_, item)| match item {
                FormItem::Field(f) => f
                    .lookup
                    .as_ref()
                    .filter(|l| l.spatial.is_some())
                    .map(|l| l.layer.as_str()),
                _ => None,
            })
            .collect();
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
            if let Some(other) = self.other_layer(layer, spatial.contains(layer)) {
                world.insert(layer.to_owned(), other);
            }
        }
        // The relations the rules' `ilişkili_toplam` names (docs/adr/0761): their child layers, with their
        // objects' shapes when a call reads a related record's measures.
        let (named, with_shapes) = kentos_forms::related_needs(body);
        if named.is_empty() {
            return world;
        }
        let fold = |t: &str| kentos_expression::js::text::fold_turkish(t.trim());
        for r in &self.settings.relations {
            if !named
                .iter()
                .any(|n| fold(n) == fold(&r.id) || fold(n) == fold(&r.name))
            {
                continue;
            }
            let other = match world.get_mut(&r.child) {
                Some(o) => {
                    if with_shapes && o.shapes.is_empty() {
                        o.shapes = self
                            .other_layer(&r.child, true)
                            .map_or_else(Vec::new, |n| n.shapes);
                    }
                    o
                }
                None => {
                    let Some(other) = self.other_layer(&r.child, with_shapes) else {
                        continue;
                    };
                    world.entry(r.child.clone()).or_insert(other)
                }
            };
            other.relations.push(kentos_forms::WorldRelation {
                id: r.id.clone(),
                name: r.name.clone(),
                field: r.field.clone(),
            });
        }
        world
    }

    /// The layer `layer` as a form reads it: its name, fields and objects, with their shapes when asked.
    fn other_layer(&self, layer: &str, shapes: bool) -> Option<OtherLayer> {
        let node = self.layers.get(layer)?;
        let objects: Vec<_> = self.by_layer_with_uids(layer).collect();
        Some(OtherLayer {
            name: node.name.clone(),
            fields: node.fields.clone(),
            rows: objects
                .iter()
                .map(|(uid, e)| (uid.to_string(), e.base().attrs.clone()))
                .collect(),
            shapes: if shapes {
                objects
                    .iter()
                    .map(|(_, e)| kentos_forms::object_of(e, "", 0.0).shape)
                    .collect()
            } else {
                Vec::new()
            },
            relations: Vec::new(),
        })
    }

    /// The forms the drawing's writes are completed by (docs/adr/0402 §4): released, their rules enforced.
    pub fn enforcing_forms(&self) -> impl Iterator<Item = &ProjectForm> {
        self.settings
            .forms
            .iter()
            .filter(|f| f.enforce && f.release().is_some())
    }

    /// What the forms `layer` enforces give a new record before its subtype's and its fields' defaults
    /// (docs/adr/0752): each field's plain default by its key (the first form's), and the keys of the fields they
    /// default by an expression, which the step works out once the record is made.
    pub fn form_defaults(&self, layer: &str) -> (Vec<(String, String)>, HashSet<String>) {
        let mut plain: Vec<(String, String)> = Vec::new();
        let mut by_expression = HashSet::new();
        for f in self.enforcing_forms().filter(|f| f.layer == layer) {
            let Some(release) = f.release() else { continue };
            for (_, item) in release.body.walk() {
                let FormItem::Field(x) = item else { continue };
                if x.mirror {
                    continue;
                }
                if let Some(d) = x.default.as_deref().filter(|d| !d.is_empty()) {
                    if !plain.iter().any(|(k, _)| *k == x.field) {
                        plain.push((x.field.clone(), d.to_owned()));
                    }
                } else if x.default_expr.is_some() {
                    by_expression.insert(x.field.clone());
                }
            }
        }
        (plain, by_expression)
    }

    /// The ops that complete the records `ops` (a step about to be recorded, applied) added or changed
    /// on a layer whose form enforces its rules, applied: the step takes them in. A record whose form reads
    /// its related records (`ilişkili_toplam`, docs/adr/0761) is completed again when the step adds,
    /// changes, links, unlinks or removes one of them, round after round while the completions change a
    /// relation's records in turn.
    pub(crate) fn follow_forms(&mut self, ops: &[Op]) -> Vec<Op> {
        let enforcing: Vec<ProjectForm> = self.enforcing_forms().cloned().collect();
        if enforcing.is_empty() {
            return Vec::new();
        }
        let layers: HashSet<&str> = enforcing.iter().map(|f| f.layer.as_str()).collect();
        // The objects the step left on such a layer, in the order they came, and whether each is new.
        let mut order: Vec<Uuid> = Vec::new();
        let mut new: HashMap<Uuid, bool> = HashMap::new();
        // Each changed object's attributes before the step: what a list's copy compares with.
        let mut before: HashMap<Uuid, kentos_forms::Attrs> = HashMap::new();
        for op in ops {
            match op {
                Op::Add(s) | Op::Update { after: s, .. }
                    if layers.contains(s.entity.base().layer_id.as_str()) =>
                {
                    let added = matches!(op, Op::Add(_));
                    if let Op::Update { before: b, .. } = op {
                        before
                            .entry(s.uid)
                            .or_insert_with(|| b.entity.base().attrs.clone());
                    }
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
        // docs/adr/0761: what the forms' `ilişkili_toplam` reads; a step touching none of it is not theirs.
        let reads = self.related_reads(&enforcing);
        let touches = |s: &Stored| {
            reads
                .iter()
                .any(|(_, child, _)| s.entity.base().layer_id == *child)
        };
        let related = ops.iter().any(|op| match op {
            Op::Add(s) | Op::Remove(s) => touches(s),
            Op::Update { before, after } => touches(before) || touches(after),
            _ => false,
        });
        if new.is_empty() && !related {
            return Vec::new();
        }
        // Only the forms of the layers the step left records on are prepared.
        let of = |layers: &HashSet<String>| -> Vec<ProjectForm> {
            enforcing
                .iter()
                .filter(|f| layers.contains(&f.layer))
                .cloned()
                .collect()
        };
        let mut out = if new.is_empty() {
            Vec::new()
        } else {
            let left: HashSet<String> = order
                .iter()
                .filter(|u| new.contains_key(u))
                .filter_map(|u| self.store.get(self.store.slot_of(*u)?))
                .map(|s| s.entity.base().layer_id.clone())
                .collect();
            let prepared = self.prepare_forms(&of(&left));
            self.complete_records(&prepared, &order, &new, &before)
        };
        if reads.is_empty() {
            return out;
        }
        // The parents whose related records the step (then each round) touched, worked out over the
        // world as the last completions left it: a parent reads its records' own calculations.
        let mut last: Vec<Op> = Vec::new();
        for round in 0..MOST_RELATED_ROUNDS {
            let touched: Vec<&Op> = if round == 0 {
                ops.iter().chain(&out).collect()
            } else {
                last.iter().collect()
            };
            let parents = self.related_parents(&touched, &reads);
            if parents.is_empty() {
                break;
            }
            let layers: HashSet<String> = parents
                .iter()
                .filter_map(|u| self.store.get(self.store.slot_of(*u)?))
                .map(|s| s.entity.base().layer_id.clone())
                .collect();
            let prepared = self.prepare_forms(&of(&layers));
            let again: HashMap<Uuid, bool> = parents.iter().map(|u| (*u, false)).collect();
            let kept: HashMap<Uuid, kentos_forms::Attrs> = parents
                .iter()
                .filter_map(|u| {
                    let slot = self.store.slot_of(*u)?;
                    Some((*u, self.store.get(slot)?.entity.base().attrs.clone()))
                })
                .collect();
            last = self.complete_records(&prepared, &parents, &again, &kept);
            if last.is_empty() {
                break;
            }
            out.extend(last.iter().cloned());
        }
        out
    }

    /// What the enforcing forms' `ilişkili_toplam` reads (docs/adr/0761): (the form's layer, the relation's
    /// child layer, the child field's name), each once.
    fn related_reads(&self, enforcing: &[ProjectForm]) -> Vec<(String, String, String)> {
        let fold = |t: &str| kentos_expression::js::text::fold_turkish(t.trim());
        let mut out: Vec<(String, String, String)> = Vec::new();
        for f in enforcing {
            let Some(release) = f.release() else { continue };
            let named = kentos_forms::named_relations(&release.body);
            for r in &self.settings.relations {
                if r.parent != f.layer
                    || !named
                        .iter()
                        .any(|n| fold(n) == fold(&r.id) || fold(n) == fold(&r.name))
                {
                    continue;
                }
                let field = self
                    .layers
                    .get(&r.child)
                    .and_then(|n| n.fields.iter().find(|x| x.key() == r.field))
                    .map_or_else(|| r.field.clone(), |x| x.name.clone());
                let read = (f.layer.clone(), r.child.clone(), field);
                if !out.contains(&read) {
                    out.push(read);
                }
            }
        }
        out
    }

    /// The enforcing forms prepared over the document as it is now, by their layers.
    fn prepare_forms(&self, enforcing: &[ProjectForm]) -> Vec<(String, Prepared)> {
        let variables = document_variables(&self.name, &self.settings);
        let unit = Unit::per_metre(self.settings.unit().per_metre());
        enforcing
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
                    .map(|p| {
                        p.with_subtypes(subtypes)
                            .with_contingents(groups)
                            .with_rules(node.map_or(&[][..], |n| n.rules.as_slice()))
                            .with_indexes(node.map_or(&[][..], |n| n.indexes.as_slice()))
                    })
                    .map(|p| (f.layer.clone(), p))
            })
            .collect()
    }

    /// The records on a form's layer the related records `touched` name (before and after), as
    /// `reads` says: (the form's layer, the child layer, the child field's name). Each once, in order.
    fn related_parents(&self, touched: &[&Op], reads: &[(String, String, String)]) -> Vec<Uuid> {
        let mut out: Vec<Uuid> = Vec::new();
        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut take = |s: &Stored, out: &mut Vec<Uuid>| {
            let base = s.entity.base();
            for (parent_layer, child, field) in reads {
                if base.layer_id != *child {
                    continue;
                }
                let Some(held) = base.attrs.get(field) else {
                    continue;
                };
                // A many-to-many field holds several ids (docs/adr/0409 §1).
                for id in held
                    .split(';')
                    .filter_map(|h| Uuid::parse_str(h.trim()).ok())
                {
                    let on_layer = self
                        .store
                        .slot_of(id)
                        .and_then(|slot| self.store.get(slot))
                        .is_some_and(|p| p.entity.base().layer_id == *parent_layer);
                    if on_layer && seen.insert(id) {
                        out.push(id);
                    }
                }
            }
        };
        for op in touched {
            match op {
                Op::Add(s) | Op::Remove(s) => take(s, &mut out),
                Op::Update { before, after } => {
                    take(before, &mut out);
                    take(after, &mut out);
                }
                _ => {}
            }
        }
        out
    }

    /// The ops completing the records `order` names on the forms' layers, applied.
    fn complete_records(
        &mut self,
        prepared: &[(String, Prepared)],
        order: &[Uuid],
        new: &HashMap<Uuid, bool>,
        before: &HashMap<Uuid, kentos_forms::Attrs>,
    ) -> Vec<Op> {
        let mut out = Vec::new();
        for uid in order {
            let Some(&added) = new.get(uid) else {
                continue;
            };
            let Some(current) = self
                .store
                .slot_of(*uid)
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
            let was = (!added).then(|| before.get(uid)).flatten();
            for (_, p) in prepared.iter().filter(|(l, _)| *l == layer) {
                let mut object = kentos_forms::object_of(&entity, &name, id);
                object.uid = Some(current.uid.to_string());
                let copied = p.copied(was, &entity.base().attrs);
                let (attrs, _) = p.complete(&copied, &object, added);
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

/// How many rounds a step's related records carry up (docs/adr/0761): a relation's parent may be another
/// relation's child; a circle of relations stops here.
const MOST_RELATED_ROUNDS: usize = 8;

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

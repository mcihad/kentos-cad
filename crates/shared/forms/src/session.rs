//! A form ready for records (docs/adr/0401 §5, §6; docs/adr/0402 §3): its rules compiled once, then
//! asked per record.
//!
//! - An item is shown when its own rule and every enclosing group's rule hold (no rule: shown).
//! - A field is required when it, its layer field or its `requiredWhen` says so, and only while shown; it
//!   is read-only when marked so, calculated or its `readOnlyWhen` holds.
//! - A record is completed before it is judged ([`Prepared::complete`]): a new record's defaults, the
//!   calculations in their order, and with `clearHidden` the hidden fields emptied.
//! - A record is valid when every shown field keeps its field's rules and its choice filter, a shown
//!   required field is filled and every hard check of a shown, filled field holds; a soft check warns.
//! - A write through the form changes only the form's fields, none hidden or read-only.

use kentos_contracts::{
    FormBody, FormField, FormIssue, FormItem, FormItemState, FormSeverity, LayerField, check_value,
    js_trim,
};
use kentos_expression::js::text::fold_turkish;
use kentos_expression::template::{self, Template};
use kentos_expression::{Expr, Schema, Unit, Variable, compile_with};

use crate::lookup::{List, Related, World, list_of, related_of};
use crate::record::{Attrs, Object, field_text_of, holds, schema_in, text_of};
use crate::{codes, has_errors, issue};

/// The `@` values that change with the moment or the user: a command gives them (`moment`, `user`),
/// the document's following does not (docs/adr/0402 §5).
pub const MOMENT: [&str; 3] = ["tarih", "simdi", "kullanici"];

/// The `@` values only a host gives: the moment's, the user's and the coordinate system's name (the
/// document does not know the systems' register). A default or a calculation reading one the host
/// did not give is left out rather than worked out empty.
pub const HOST_ONLY: [&str; 4] = ["tarih", "simdi", "kullanici", "koordinat_sistemi"];

/// The `@` names a choice filter reads for the value it asks about.
pub const CHOICE_CODE: &str = "secenek";
pub const CHOICE_LABEL: &str = "secenek_etiketi";

/// How many times a record is completed before it settles: a form that empties hidden fields may hide
/// more once a calculation sees the emptied ones.
const ROUNDS: usize = 3;

/// A form compiled over its layer's fields.
#[derive(Clone, Debug)]
pub struct Prepared {
    fields: Vec<LayerField>,
    items: Vec<Node>,
    /// The calculated field items' places in `items`, each after the calculated fields it reads.
    calc_order: Vec<usize>,
    clear_hidden: bool,
}

/// An item, flattened in the form's order with its parent.
#[derive(Clone, Debug)]
struct Node {
    id: String,
    parent: Option<usize>,
    visible: Option<Expr>,
    field: Option<Bound>,
    note: Option<Note>,
    /// A related item's objects (docs/adr/0404 §4).
    related: Option<Related>,
}

/// A note whose text puts values in.
#[derive(Clone, Debug)]
struct Note {
    template: Template,
    exprs: Vec<Expr>,
}

/// A field item: its layer field and its rules.
#[derive(Clone, Debug)]
struct Bound {
    /// Its layer field's place in `fields`.
    index: usize,
    item: FormField,
    required_when: Option<Expr>,
    read_only_when: Option<Expr>,
    checks: Vec<Expr>,
    default_expr: Option<Expr>,
    calculate: Option<Expr>,
    /// Each listed value's code and the filter asked of it.
    choices: Option<Vec<(String, Expr)>>,
    /// The default's expression reads a host's value that was not given: it is left out.
    default_waits: bool,
    /// The calculation reads a host's value that was not given: it is left out.
    calc_waits: bool,
    /// The field's list from another layer (docs/adr/0404 §1).
    lookup: Option<List>,
}

impl Prepared {
    /// `body` over the layer's `fields`, its geometry values in metres; refused with the preflight's
    /// issues when one is an error.
    pub fn new(
        body: &FormBody,
        fields: &[LayerField],
        variables: &[Variable],
    ) -> Result<Prepared, Vec<FormIssue>> {
        Prepared::new_in(body, fields, variables, Unit::METRE)
    }

    /// [`Prepared::new`] whose geometry values speak `unit` (the project's drawing unit).
    pub fn new_in(
        body: &FormBody,
        fields: &[LayerField],
        variables: &[Variable],
        unit: Unit,
    ) -> Result<Prepared, Vec<FormIssue>> {
        Prepared::new_with(body, fields, variables, unit, &World::new())
    }

    /// [`Prepared::new_in`] reading the other layers `world` holds: the fields' lists from them and
    /// the related items (docs/adr/0404).
    pub fn new_with(
        body: &FormBody,
        fields: &[LayerField],
        variables: &[Variable],
        unit: Unit,
        world: &World,
    ) -> Result<Prepared, Vec<FormIssue>> {
        let issues = crate::preflight_in(body, fields, variables, world);
        if has_errors(&issues) {
            return Err(issues);
        }
        let schema = schema_in(fields, variables, unit);
        let mut items = Vec::new();
        let ctx = Ctx {
            fields,
            schema: &schema,
            world,
            variables,
            unit,
        };
        walk(&body.items, None, &mut items, &ctx);
        let calc_order = calc_order(&items, fields).unwrap_or_default();
        Ok(Prepared {
            fields: fields.to_vec(),
            items,
            calc_order,
            clear_hidden: body.clear_hidden,
        })
    }

    /// The layer fields' keys the form shows, in its order.
    pub fn keys(&self) -> Vec<&str> {
        self.bound()
            .map(|(_, b)| self.fields[b.index].key())
            .collect()
    }

    /// The keys of the fields whose values are unique in the layer.
    pub fn unique_keys(&self) -> Vec<&str> {
        self.bound()
            .filter(|(_, b)| b.item.unique)
            .map(|(_, b)| self.fields[b.index].key())
            .collect()
    }

    /// The keys of the fields a new record opens with the session's last value.
    pub fn reuse_keys(&self) -> Vec<&str> {
        self.bound()
            .filter(|(_, b)| b.item.reuse_last && b.calculate.is_none())
            .map(|(_, b)| self.fields[b.index].key())
            .collect()
    }

    /// Every item's state for the record, in the form's order.
    pub fn states(&self, attrs: &Attrs, object: &Object) -> Vec<FormItemState> {
        let mut shown: Vec<bool> = Vec::with_capacity(self.items.len());
        let mut out = Vec::with_capacity(self.items.len());
        for node in &self.items {
            let here = node.parent.is_none_or(|p| shown[p])
                && node
                    .visible
                    .as_ref()
                    .is_none_or(|e| holds(e, attrs, object));
            shown.push(here);
            let (required, read_only) = node.field.as_ref().map_or((false, false), |b| {
                // A field the layer requires is required on every form (the preflight shows it always).
                let required = here
                    && (b.item.required
                        || self.fields[b.index].required
                        || b.required_when
                            .as_ref()
                            .is_some_and(|e| holds(e, attrs, object)));
                let read_only = b.item.read_only
                    || b.calculate.is_some()
                    || b.read_only_when
                        .as_ref()
                        .is_some_and(|e| holds(e, attrs, object));
                (required, read_only)
            });
            out.push(FormItemState {
                id: node.id.clone(),
                shown: here,
                required,
                read_only,
                choices: None,
                text: None,
                options: None,
                related: None,
            });
        }
        out
    }

    /// [`Prepared::states`] with what a field's choice filter offers and a note's text for the record:
    /// what a form's screen shows.
    pub fn described(&self, attrs: &Attrs, object: &Object) -> Vec<FormItemState> {
        let notes = self.notes(attrs, object);
        let mut states = self.states(attrs, object);
        for (node, state) in self.items.iter().zip(&mut states) {
            state.choices = self.choices(&state.id, attrs, object);
            state.text = notes
                .iter()
                .find(|(id, _)| *id == state.id)
                .map(|(_, t)| t.clone());
            if let Some(list) = node.field.as_ref().and_then(|b| b.lookup.as_ref()) {
                state.options = Some(list.choices(attrs));
            }
            if let Some(related) = &node.related {
                state.related = Some(related.of(object));
            }
        }
        states
    }

    /// The listed values a field item offers for the record, in the list's order: none when the item
    /// has no filter (every listed value is offered).
    pub fn choices(&self, item: &str, attrs: &Attrs, object: &Object) -> Option<Vec<String>> {
        let b = self.items.iter().find(|n| n.id == item)?.field.as_ref()?;
        if let Some(list) = &b.lookup {
            return Some(
                list.offered(attrs)
                    .into_iter()
                    .map(|r| r.0.clone())
                    .collect(),
            );
        }
        let choices = b.choices.as_ref()?;
        Some(
            choices
                .iter()
                .filter(|(_, e)| holds(e, attrs, object))
                .map(|(code, _)| code.clone())
                .collect(),
        )
    }

    /// Each note's text for the record, by its item's id, in the form's order (a plain note as
    /// written).
    pub fn notes(&self, attrs: &Attrs, object: &Object) -> Vec<(String, String)> {
        self.items
            .iter()
            .filter_map(|n| {
                let note = n.note.as_ref()?;
                let values: Vec<Option<String>> = note
                    .exprs
                    .iter()
                    .map(|e| text_of(e, attrs, object))
                    .collect();
                Some((n.id.clone(), note.template.fill(&values, None)))
            })
            .collect()
    }

    /// The record completed (docs/adr/0402 §3): a new record's empty fields take their plain
    /// defaults, then their expressions' in the form's order; the calculations follow in their order;
    /// with `clearHidden` the fields the record hides are emptied and the calculations done again. An
    /// expression reading the moment or the user is left out when they were not given. Its issues:
    /// a calculation whose value its field refuses (an error: the record is not written), a default
    /// whose value its field refuses (a warning: it is not put in), a record that does not settle.
    pub fn complete(&self, attrs: &Attrs, object: &Object, new: bool) -> (Attrs, Vec<FormIssue>) {
        let mut out = attrs.clone();
        let mut issues = Vec::new();
        if new {
            self.defaults(&mut out, object, &mut issues);
        }
        for round in 0..ROUNDS {
            let start = out.clone();
            let mut round_issues = Vec::new();
            let shown = if self.clear_hidden {
                Some(self.states(&out, object))
            } else {
                None
            };
            for &at in &self.calc_order {
                let node = &self.items[at];
                let Some(b) = &node.field else { continue };
                if shown.as_ref().is_some_and(|s| !s[at].shown) {
                    continue;
                }
                self.calculate(b, &node.id, &mut out, object, &mut round_issues);
            }
            if self.clear_hidden {
                let states = self.states(&out, object);
                for (node, state) in self.items.iter().zip(&states) {
                    if let Some(b) = &node.field
                        && !state.shown
                    {
                        out.remove(&self.fields[b.index].name);
                    }
                }
            }
            let settled = out == start;
            if settled || !self.clear_hidden {
                issues.extend(round_issues);
                return (out, issues);
            }
            if round + 1 == ROUNDS {
                issues.extend(round_issues);
                issues.push(issue(
                    codes::UNSTABLE,
                    FormSeverity::Error,
                    None,
                    None,
                    None,
                    "Formun hesapları ve gizlenen alanları birbirini değiştirip duruyor; kurallarını düzeltin.".into(),
                ));
            }
        }
        (out, issues)
    }

    /// A new record's empty fields with their defaults.
    fn defaults(&self, out: &mut Attrs, object: &Object, issues: &mut Vec<FormIssue>) {
        for (_, b) in self.bound() {
            let field = &self.fields[b.index];
            if let Some(default) = &b.item.default
                && is_empty(out, &field.name)
                && let Ok(v) = check_value(&loose(field), default)
                && !v.is_empty()
            {
                out.insert(field.name.clone(), v);
            }
        }
        for (node, b) in self.bound() {
            let Some(e) = &b.default_expr else { continue };
            let field = &self.fields[b.index];
            if b.default_waits || !is_empty(out, &field.name) {
                continue;
            }
            let Some(text) = field_text_of(e, out, object) else {
                continue;
            };
            match check_value(&loose(field), &text) {
                Ok(v) if !v.is_empty() => {
                    out.insert(field.name.clone(), v);
                }
                Ok(_) => {}
                Err(r) => issues.push(issue(
                    codes::DEFAULT_INVALID,
                    FormSeverity::Warning,
                    Some(&node.id),
                    Some(field.key()),
                    Some("defaultExpr"),
                    format!(
                        "“{}” alanının varsayılanı ({text}) alana uymuyor: {} Varsayılan konmadı.",
                        field.label(),
                        r.message
                    ),
                )),
            }
        }
    }

    /// One calculated field worked out into `out`.
    fn calculate(
        &self,
        b: &Bound,
        id: &str,
        out: &mut Attrs,
        object: &Object,
        issues: &mut Vec<FormIssue>,
    ) {
        let Some(e) = &b.calculate else { return };
        if b.calc_waits {
            return;
        }
        let field = &self.fields[b.index];
        let Some(text) = field_text_of(e, out, object) else {
            out.remove(&field.name);
            return;
        };
        match check_value(&loose(field), &text) {
            Ok(v) if v.is_empty() => {
                out.remove(&field.name);
            }
            Ok(v) => {
                out.insert(field.name.clone(), v);
            }
            Err(r) => issues.push(issue(
                codes::CALC_INVALID,
                FormSeverity::Error,
                Some(id),
                Some(field.key()),
                Some("calculate"),
                format!(
                    "“{}” alanının hesabı ({text}) alana uymuyor: {}",
                    field.label(),
                    r.message
                ),
            )),
        }
    }

    /// What is wrong with the record, in the form's order: errors and the soft checks' warnings. The
    /// record is judged as it is; [`Prepared::complete`] it first.
    pub fn validate(&self, attrs: &Attrs, object: &Object) -> Vec<FormIssue> {
        let states = self.states(attrs, object);
        let mut issues = Vec::new();
        for (node, state) in self.items.iter().zip(&states) {
            let Some(b) = &node.field else { continue };
            if !state.shown {
                continue;
            }
            let field = &self.fields[b.index];
            let label = b.item.label.as_deref().unwrap_or(field.label());
            let at = |code: &str, severity: FormSeverity, rule: Option<String>, message: String| {
                issue(
                    code,
                    severity,
                    Some(&node.id),
                    Some(field.key()),
                    rule.as_deref(),
                    message,
                )
            };
            let value = attrs.get(&field.name).map_or("", |v| js_trim(v));
            if value.is_empty() {
                if state.required {
                    issues.push(at(
                        codes::REQUIRED,
                        FormSeverity::Error,
                        None,
                        format!("“{label}” zorunlu; doldurun."),
                    ));
                }
                continue;
            }
            let canonical = match check_value(&loose(field), value) {
                Ok(v) => v,
                Err(r) => {
                    issues.push(at(
                        codes::VALUE_INVALID,
                        FormSeverity::Error,
                        None,
                        r.message,
                    ));
                    continue;
                }
            };
            if let Some(list) = &b.lookup
                && !list.offered(attrs).iter().any(|r| r.0 == canonical)
            {
                issues.push(at(
                    codes::LOOKUP_MISSING,
                    FormSeverity::Error,
                    Some("lookup".into()),
                    format!("“{label}” için “{value}” listede yok; listeden seçin."),
                ));
                continue;
            }
            if let Some(choices) = &b.choices
                && let Some((_, e)) = choices.iter().find(|(code, _)| *code == canonical)
                && !holds(e, attrs, object)
            {
                issues.push(at(
                    codes::CHOICE_FILTERED,
                    FormSeverity::Error,
                    Some("choiceFilter".into()),
                    format!("“{label}” için “{value}” bu kayıtta seçilemez; listeden seçin."),
                ));
                continue;
            }
            for (i, (check, expr)) in b.item.checks.iter().zip(&b.checks).enumerate() {
                if !holds(expr, attrs, object) {
                    let (code, severity) = if check.warning {
                        (codes::CHECK_WARNING, FormSeverity::Warning)
                    } else {
                        (codes::CHECK_FAILED, FormSeverity::Error)
                    };
                    issues.push(at(
                        code,
                        severity,
                        Some(format!("checks/{i}")),
                        check.message.clone(),
                    ));
                }
            }
        }
        issues
    }

    /// The record with the form's plain and expression defaults in its empty fields and its
    /// calculations: a new record's values ([`Prepared::complete`] with `new`).
    pub fn with_defaults(&self, attrs: &Attrs, object: &Object) -> Attrs {
        self.complete(attrs, object, true).0
    }

    /// Whether the record's unique fields keep their uniqueness among `others` (the layer's other
    /// objects' attributes): a `form_unique` error for each that does not.
    pub fn unique_issues<'a>(
        &self,
        attrs: &Attrs,
        object: &Object,
        others: impl Iterator<Item = &'a Attrs>,
    ) -> Vec<FormIssue> {
        let states = self.states(attrs, object);
        let wanted: Vec<(usize, &Bound, String)> = self
            .items
            .iter()
            .zip(&states)
            .enumerate()
            .filter_map(|(at, (node, state))| {
                let b = node.field.as_ref()?;
                let v = attrs.get(&self.fields[b.index].name)?;
                (b.item.unique && state.shown && !js_trim(v).is_empty()).then(|| (at, b, v.clone()))
            })
            .collect();
        if wanted.is_empty() {
            return Vec::new();
        }
        let mut taken = vec![false; wanted.len()];
        for other in others {
            for (k, (_, b, v)) in wanted.iter().enumerate() {
                if other.get(&self.fields[b.index].name) == Some(v) {
                    taken[k] = true;
                }
            }
        }
        wanted
            .iter()
            .zip(taken)
            .filter(|(_, t)| *t)
            .map(|((at, b, v), _)| {
                let field = &self.fields[b.index];
                let label = b.item.label.as_deref().unwrap_or(field.label());
                issue(
                    codes::UNIQUE,
                    FormSeverity::Error,
                    Some(&self.items[*at].id),
                    Some(field.key()),
                    Some("unique"),
                    format!("“{label}” katmanda tekil olmalı; “{v}” başka bir nesnede var."),
                )
            })
            .collect()
    }

    /// A write through the form: `values` by the layer fields' keys (empty: no value). The record's
    /// attributes after it with its warnings, or every reason it does not go in ([`Prepared::assess`]).
    pub fn apply(
        &self,
        before: &Attrs,
        values: &[(String, String)],
        object: &Object,
        new: bool,
    ) -> Result<(Attrs, Vec<FormIssue>), Vec<FormIssue>> {
        let (after, issues) = self.assess(before, values, object, new);
        if has_errors(&issues) {
            Err(issues)
        } else {
            Ok((after, issues))
        }
    }

    /// What a write through the form would leave and what is wrong with it: a key the form does not
    /// show, a value its field refuses (left out of the record), a change to a hidden or read-only
    /// field (a calculated one's value given as its calculation's is no change), then the completed
    /// record's issues (a field already said is not said again). `new`: the record takes the form's
    /// defaults.
    pub fn assess(
        &self,
        before: &Attrs,
        values: &[(String, String)],
        object: &Object,
        new: bool,
    ) -> (Attrs, Vec<FormIssue>) {
        let mut written = before.clone();
        let mut issues = Vec::new();
        let mut changed: Vec<usize> = Vec::new();
        let mut asked: Vec<(usize, Option<String>)> = Vec::new();
        for (key, text) in values {
            let Some(index) = self.fields.iter().position(|f| f.key() == key) else {
                issues.push(refused(
                    codes::FIELD_UNKNOWN,
                    None,
                    key,
                    format!("Katmanda “{key}” alanı yok."),
                ));
                continue;
            };
            let field = &self.fields[index];
            if !self.bound().any(|(_, b)| b.index == index) {
                issues.push(refused(
                    codes::NOT_ON_FORM,
                    None,
                    key,
                    format!("Form “{}” alanını göstermiyor.", field.label()),
                ));
                continue;
            }
            // A list from another layer takes a value's label for its code (docs/adr/0404 §2).
            let text = self
                .bound()
                .find(|(_, b)| b.index == index)
                .and_then(|(_, b)| b.lookup.as_ref())
                .map_or(text.as_str(), |list| list.code_of(text));
            match check_value(&loose(field), text) {
                Ok(v) if v.is_empty() => {
                    written.remove(&field.name);
                }
                Ok(v) => {
                    written.insert(field.name.clone(), v);
                }
                Err(r) => {
                    let item = self.item_of(index);
                    issues.push(refused(codes::VALUE_INVALID, item, key, r.message));
                    continue;
                }
            }
            if self.calculated(index) {
                asked.push((index, written.get(&field.name).cloned()));
            } else if before.get(&field.name) != written.get(&field.name) {
                changed.push(index);
            }
        }
        let (after, completed) = self.complete(&written, object, new);
        issues.extend(self.locked(&changed, &after, object));
        // A calculated field given a value other than its calculation's.
        for (index, value) in asked {
            if after.get(&self.fields[index].name) != value.as_ref() {
                let item = self.item_of(index);
                let field = &self.fields[index];
                issues.push(refused(
                    codes::READ_ONLY,
                    item,
                    field.key(),
                    format!(
                        "“{}” hesaplanan bir alan; değeri formun hesabından gelir.",
                        field.label()
                    ),
                ));
            }
        }
        issues.extend(completed);
        let said: Vec<Option<String>> = issues.iter().map(|i| i.field.clone()).collect();
        issues.extend(
            self.validate(&after, object)
                .into_iter()
                .filter(|i| !said.contains(&i.field)),
        );
        (after, issues)
    }

    /// The check a write from elsewhere passes when the form enforces its rules (docs/adr/0401 §8,
    /// docs/adr/0402 §4): a changed layer field the form does not show, hides or locks is refused (a
    /// calculated one given its calculation's value is not refused), and the record after it,
    /// completed, must be valid. Attributes that are not the layer's fields are not the form's. Its
    /// issues hold the soft checks' warnings too.
    pub fn enforce(&self, before: &Attrs, after: &Attrs, object: &Object) -> Vec<FormIssue> {
        let mut issues = Vec::new();
        let mut changed = Vec::new();
        let (completed, worked) = self.complete(after, object, false);
        for (index, field) in self.fields.iter().enumerate() {
            if before.get(&field.name) == after.get(&field.name) {
                continue;
            }
            if self.calculated(index) {
                if after.get(&field.name) != completed.get(&field.name) {
                    issues.push(refused(
                        codes::READ_ONLY,
                        self.item_of(index),
                        field.key(),
                        format!(
                            "“{}” hesaplanan bir alan; değeri formun hesabından gelir.",
                            field.label()
                        ),
                    ));
                }
                continue;
            }
            if self.bound().any(|(_, b)| b.index == index) {
                changed.push(index);
            } else {
                issues.push(refused(
                    codes::NOT_ON_FORM,
                    None,
                    field.key(),
                    format!(
                        "Form “{}” alanını göstermiyor; bu alan yazılamaz.",
                        field.label()
                    ),
                ));
            }
        }
        issues.extend(self.locked(&changed, &completed, object));
        issues.extend(worked);
        issues.extend(self.validate(&completed, object));
        issues
    }

    /// The changed fields the form hides or locks in the record after the change.
    fn locked(&self, changed: &[usize], after: &Attrs, object: &Object) -> Vec<FormIssue> {
        let states = self.states(after, object);
        let mut issues = Vec::new();
        for (node, state) in self.items.iter().zip(&states) {
            let Some(b) = &node.field else { continue };
            if !changed.contains(&b.index) {
                continue;
            }
            let field = &self.fields[b.index];
            let (code, why) = if !state.shown {
                // A form that empties hidden fields empties this one: no value is kept from the write.
                if self.clear_hidden {
                    continue;
                }
                (codes::HIDDEN, "gizli")
            } else if state.read_only {
                (codes::READ_ONLY, "salt okunur")
            } else {
                continue;
            };
            issues.push(refused(
                code,
                Some(&node.id),
                field.key(),
                format!(
                    "“{}” bu kayıtta {why}; değeri değiştirilemez.",
                    field.label()
                ),
            ));
        }
        issues
    }

    fn bound(&self) -> impl Iterator<Item = (&Node, &Bound)> {
        self.items
            .iter()
            .filter_map(|n| n.field.as_ref().map(|b| (n, b)))
    }

    fn calculated(&self, index: usize) -> bool {
        self.bound()
            .any(|(_, b)| b.index == index && b.calculate.is_some())
    }

    fn item_of(&self, index: usize) -> Option<&str> {
        self.bound()
            .find(|(_, b)| b.index == index)
            .map(|(n, _)| n.id.as_str())
    }
}

/// What a form compiles against.
struct Ctx<'a> {
    fields: &'a [LayerField],
    schema: &'a Schema,
    world: &'a World,
    variables: &'a [Variable],
    unit: Unit,
}

fn walk(list: &[FormItem], parent: Option<usize>, out: &mut Vec<Node>, ctx: &Ctx) {
    // The preflight compiled every rule: a rule here compiles.
    let compile = |rule: &Option<String>| {
        rule.as_deref()
            .and_then(|r| compile_with(r, ctx.schema).ok())
    };
    for item in list {
        let at = out.len();
        let mut related = None;
        let (visible, field, note) = match item {
            FormItem::Group { visible_when, .. } => (compile(visible_when), None, None),
            FormItem::Related {
                layer,
                field,
                visible_when,
                ..
            } => {
                related = related_of(layer, field, ctx.world);
                (compile(visible_when), None, None)
            }
            FormItem::Field(f) => {
                let index = ctx
                    .fields
                    .iter()
                    .position(|l| l.key() == f.field)
                    .unwrap_or(0);
                let default_expr = compile(&f.default_expr);
                let calculate = compile(&f.calculate);
                let bound = Bound {
                    index,
                    item: f.clone(),
                    required_when: compile(&f.required_when),
                    read_only_when: compile(&f.read_only_when),
                    checks: f
                        .checks
                        .iter()
                        .filter_map(|c| compile(&Some(c.rule.clone())))
                        .collect(),
                    default_waits: default_expr.as_ref().is_some_and(waits),
                    calc_waits: calculate.as_ref().is_some_and(waits),
                    default_expr,
                    calculate,
                    choices: f
                        .choice_filter
                        .as_deref()
                        .map(|src| choice_filters(src, &ctx.fields[index], ctx.schema)),
                    lookup: f.lookup.as_ref().and_then(|l| {
                        let still: Vec<Variable> = ctx
                            .variables
                            .iter()
                            .filter(|v| {
                                !MOMENT.contains(&fold_turkish(&v.name).to_lowercase().as_str())
                            })
                            .cloned()
                            .collect();
                        list_of(l, ctx.fields, ctx.world, &still, ctx.unit)
                    }),
                };
                (compile(&f.visible_when), Some(bound), None)
            }
            FormItem::Note { text, .. } => (None, None, note_of(text, ctx.schema)),
            FormItem::Tab { .. } => (None, None, None),
        };
        out.push(Node {
            id: item.id().to_owned(),
            parent,
            visible,
            field,
            note,
            related,
        });
        walk(item.items(), Some(at), out, ctx);
    }
}

/// A note's template and its expressions, when it puts values in.
fn note_of(text: &str, schema: &Schema) -> Option<Note> {
    let template = template::parse(text).ok()?;
    if template.is_plain() {
        return None;
    }
    let exprs = template
        .expressions()
        .map(|(src, _)| compile_with(src, schema).ok())
        .collect::<Option<Vec<Expr>>>()?;
    Some(Note { template, exprs })
}

/// A choice filter compiled for each of the field's listed values (`@secenek`, `@secenek_etiketi`).
pub(crate) fn choice_filters(
    src: &str,
    field: &LayerField,
    schema: &Schema,
) -> Vec<(String, Expr)> {
    let listed = field.values.as_deref().unwrap_or_default();
    listed
        .iter()
        .filter_map(|v| {
            let mut s = schema.clone();
            s.variables.push(Variable {
                name: CHOICE_CODE.into(),
                value: kentos_expression::Value::text(v.code.clone()),
                description: String::new(),
            });
            s.variables.push(Variable {
                name: CHOICE_LABEL.into(),
                value: kentos_expression::Value::text(v.label.clone()),
                description: String::new(),
            });
            compile_with(src, &s).ok().map(|e| (v.code.clone(), e))
        })
        .collect()
}

/// The schema without the moment's and the user's values.
pub(crate) fn still_schema(schema: &Schema) -> Schema {
    let mut s = schema.clone();
    s.variables
        .retain(|v| !MOMENT.contains(&fold_turkish(&v.name).to_lowercase().as_str()));
    s
}

/// Whether `e` reads a host's value that was not given.
fn waits(e: &Expr) -> bool {
    e.unknown_variables
        .iter()
        .any(|n| HOST_ONLY.contains(&fold_turkish(n).to_lowercase().as_str()))
}

/// Whether `src` reads the moment or the user (compiled without them, it names them unknown).
pub(crate) fn reads_moment(src: &str, still: &Schema) -> bool {
    compile_with(src, still).is_ok_and(|e| {
        e.unknown_variables
            .iter()
            .any(|n| MOMENT.contains(&fold_turkish(n).to_lowercase().as_str()))
    })
}

/// The calculated field items' places, each after the calculated fields it reads (in the form's order
/// where nothing orders them); none when they read each other in a circle.
fn calc_order(items: &[Node], fields: &[LayerField]) -> Option<Vec<usize>> {
    let calcs: Vec<(usize, &Bound)> = items
        .iter()
        .enumerate()
        .filter_map(|(at, n)| {
            n.field
                .as_ref()
                .filter(|b| b.calculate.is_some())
                .map(|b| (at, b))
        })
        .collect();
    let reads = |b: &Bound| -> Vec<usize> {
        let e = b
            .calculate
            .as_ref()
            .map(|e| e.fields.as_slice())
            .unwrap_or_default();
        calcs
            .iter()
            .enumerate()
            .filter(|(_, (_, other))| e.contains(&fields[other.index].name))
            .map(|(k, _)| k)
            .collect()
    };
    let deps: Vec<Vec<usize>> = calcs.iter().map(|(_, b)| reads(b)).collect();
    let mut done = vec![false; calcs.len()];
    let mut order = Vec::with_capacity(calcs.len());
    while order.len() < calcs.len() {
        let next = (0..calcs.len()).find(|&k| !done[k] && deps[k].iter().all(|&d| done[d]))?;
        done[next] = true;
        order.push(calcs[next].0);
    }
    Some(order)
}

/// Whether the record has no value in `name`.
fn is_empty(attrs: &Attrs, name: &str) -> bool {
    attrs.get(name).is_none_or(|v| js_trim(v).is_empty())
}

/// The field without its own `required`: the form asks for it only while it is shown.
fn loose(field: &LayerField) -> LayerField {
    let mut f = field.clone();
    f.required = false;
    f
}

fn refused(code: &str, item: Option<&str>, key: &str, message: String) -> FormIssue {
    issue(code, FormSeverity::Error, item, Some(key), None, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_filter_reads_the_value_it_asks_about() {
        let mut f = LayerField::new("Tür", kentos_contracts::LayerFieldKind::Text);
        f.values = Some(vec![
            kentos_contracts::FieldChoice {
                code: "a".into(),
                label: "Ahşap".into(),
            },
            kentos_contracts::FieldChoice {
                code: "b".into(),
                label: "Betonarme".into(),
            },
        ]);
        let schema = crate::record::schema_of(std::slice::from_ref(&f), &[]);
        let filters = choice_filters("@secenek_etiketi gibi 'B%'", &f, &schema);
        let object = Object::default();
        let kept: Vec<&str> = filters
            .iter()
            .filter(|(_, e)| holds(e, &Attrs::new(), &object))
            .map(|(c, _)| c.as_str())
            .collect();
        assert_eq!(kept, ["b"]);
    }
}

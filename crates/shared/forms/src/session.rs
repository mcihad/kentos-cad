//! A form ready for records (docs/adr/0401 §5, §6): its rules compiled once, then asked per record.
//!
//! - An item is shown when its own rule and every enclosing group's rule hold (no rule: shown).
//! - A field is required when it, its layer field or its `requiredWhen` says so, and only while shown; it
//!   is read-only when marked so or its `readOnlyWhen` holds.
//! - A record is valid when every shown field keeps its field's rules, a shown required field is
//!   filled and every check of a shown, filled field holds. A hidden field is not asked and keeps
//!   its value.
//! - A write through the form changes only the form's fields, none hidden or read-only.

use kentos_contracts::{
    FormBody, FormField, FormIssue, FormItem, FormItemState, FormSeverity, LayerField, check_value,
    js_trim,
};
use kentos_expression::{Expr, Variable, compile_with};

use crate::record::{Attrs, Object, holds, schema_of};
use crate::{codes, has_errors, issue, preflight};

/// A form compiled over its layer's fields.
#[derive(Clone, Debug)]
pub struct Prepared {
    fields: Vec<LayerField>,
    items: Vec<Node>,
}

/// An item, flattened in the form's order with its parent.
#[derive(Clone, Debug)]
struct Node {
    id: String,
    parent: Option<usize>,
    visible: Option<Expr>,
    field: Option<Bound>,
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
}

impl Prepared {
    /// `body` over the layer's `fields`; refused with the preflight's issues when one is an error.
    pub fn new(
        body: &FormBody,
        fields: &[LayerField],
        variables: &[Variable],
    ) -> Result<Prepared, Vec<FormIssue>> {
        let issues = preflight(body, fields, variables);
        if has_errors(&issues) {
            return Err(issues);
        }
        let schema = schema_of(fields, variables);
        // The preflight compiled every rule: a rule here compiles.
        let compile =
            |rule: &Option<String>| rule.as_deref().and_then(|r| compile_with(r, &schema).ok());
        let mut items = Vec::new();
        fn walk(
            list: &[FormItem],
            parent: Option<usize>,
            out: &mut Vec<Node>,
            fields: &[LayerField],
            compile: &dyn Fn(&Option<String>) -> Option<Expr>,
        ) {
            for item in list {
                let at = out.len();
                let (visible, field) = match item {
                    FormItem::Group { visible_when, .. } => (compile(visible_when), None),
                    FormItem::Field(f) => {
                        let index = fields.iter().position(|l| l.key() == f.field).unwrap_or(0);
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
                        };
                        (compile(&f.visible_when), Some(bound))
                    }
                    FormItem::Tab { .. } | FormItem::Note { .. } => (None, None),
                };
                out.push(Node {
                    id: item.id().to_owned(),
                    parent,
                    visible,
                    field,
                });
                walk(item.items(), Some(at), out, fields, compile);
            }
        }
        walk(&body.items, None, &mut items, fields, &compile);
        Ok(Prepared {
            fields: fields.to_vec(),
            items,
        })
    }

    /// The layer fields' keys the form shows, in its order.
    pub fn keys(&self) -> Vec<&str> {
        self.bound()
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
            });
        }
        out
    }

    /// What is wrong with the record, in the form's order (every issue an error).
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
            let at = |code: &str, rule: Option<String>, message: String| {
                issue(
                    code,
                    FormSeverity::Error,
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
                        None,
                        format!("“{label}” zorunlu; doldurun."),
                    ));
                }
                continue;
            }
            if let Err(r) = check_value(&loose(field), value) {
                issues.push(at(codes::VALUE_INVALID, None, r.message));
                continue;
            }
            for (i, (check, expr)) in b.item.checks.iter().zip(&b.checks).enumerate() {
                if !holds(expr, attrs, object) {
                    issues.push(at(
                        codes::CHECK_FAILED,
                        Some(format!("checks/{i}")),
                        check.message.clone(),
                    ));
                }
            }
        }
        issues
    }

    /// The record with the form's defaults in its empty fields: a new record's values (a default
    /// applies only when asked, never to a field with a value).
    pub fn with_defaults(&self, attrs: &Attrs) -> Attrs {
        let mut out = attrs.clone();
        for (_, b) in self.bound() {
            let field = &self.fields[b.index];
            if let Some(default) = &b.item.default
                && out.get(&field.name).is_none_or(|v| js_trim(v).is_empty())
                && let Ok(v) = check_value(&loose(field), default)
                && !v.is_empty()
            {
                out.insert(field.name.clone(), v);
            }
        }
        out
    }

    /// A write through the form: `values` by the layer fields' keys (empty: no value). The record's
    /// attributes after it, or every reason it does not go in ([`Prepared::assess`]).
    pub fn apply(
        &self,
        before: &Attrs,
        values: &[(String, String)],
        object: &Object,
    ) -> Result<Attrs, Vec<FormIssue>> {
        let (after, issues) = self.assess(before, values, object);
        if issues.is_empty() {
            Ok(after)
        } else {
            Err(issues)
        }
    }

    /// What a write through the form would leave and what is wrong with it: a key the form does not
    /// show, a value its field refuses (left out of the record), a change to a hidden or read-only
    /// field, then the record's validation (a field already said is not said again).
    pub fn assess(
        &self,
        before: &Attrs,
        values: &[(String, String)],
        object: &Object,
    ) -> (Attrs, Vec<FormIssue>) {
        let mut after = before.clone();
        let mut issues = Vec::new();
        let mut changed: Vec<usize> = Vec::new();
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
            match check_value(&loose(field), text) {
                Ok(v) if v.is_empty() => {
                    after.remove(&field.name);
                }
                Ok(v) => {
                    after.insert(field.name.clone(), v);
                }
                Err(r) => {
                    let item = self.item_of(index);
                    issues.push(refused(codes::VALUE_INVALID, item, key, r.message));
                    continue;
                }
            }
            if before.get(&field.name) != after.get(&field.name) {
                changed.push(index);
            }
        }
        issues.extend(self.locked(&changed, &after, object));
        let said: Vec<Option<String>> = issues.iter().map(|i| i.field.clone()).collect();
        issues.extend(
            self.validate(&after, object)
                .into_iter()
                .filter(|i| !said.contains(&i.field)),
        );
        (after, issues)
    }

    /// The check a write from elsewhere passes when the form enforces its rules (docs/adr/0401 §8):
    /// a changed layer field the form does not show, hides or locks is refused, and the record after
    /// it must be valid. Attributes that are not the layer's fields are not the form's.
    pub fn enforce(&self, before: &Attrs, after: &Attrs, object: &Object) -> Vec<FormIssue> {
        let mut issues = Vec::new();
        let mut changed = Vec::new();
        for (index, field) in self.fields.iter().enumerate() {
            if before.get(&field.name) == after.get(&field.name) {
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
        issues.extend(self.locked(&changed, after, object));
        issues.extend(self.validate(after, object));
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

    fn item_of(&self, index: usize) -> Option<&str> {
        self.bound()
            .find(|(_, b)| b.index == index)
            .map(|(n, _)| n.id.as_str())
    }
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

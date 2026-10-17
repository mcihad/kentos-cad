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
    AttachmentAccept, FieldChoice, FormBody, FormField, FormIssue, FormItem, FormItemState,
    FormSeverity, FormWidget, LayerField, check_value, js_trim,
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
    /// The layer's subtypes (docs/adr/0422 §2): what a record's kind narrows.
    subtypes: Option<kentos_contracts::LayerSubtypes>,
    /// The layer's value groups (docs/adr/0423 §2).
    groups: Vec<kentos_contracts::FieldGroup>,
    /// The form's workflow (docs/adr/0428) and its state field's place in `fields`.
    workflow: Option<(kentos_contracts::FormWorkflow, usize)>,
    /// The layer's record rules (docs/adr/0438 §1), shown on their fields.
    rules: crate::LayerRules,
    /// The rules' schema: what the layer's rules compile against.
    schema: Schema,
    /// The day the rules read (`@tarih`): what a listed value's validity is judged on (docs/adr/0438 §5).
    today: Option<String>,
}

/// When a record's checks are asked (docs/adr/0438 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    /// As a value changes: the checks without a moment.
    Change,
    /// When the record is saved: those and the ones asked on saving (and on submitting, without a
    /// workflow).
    Save,
    /// When the record goes forward in the workflow: every check.
    Submit,
}

/// An item's state for the record, without its id.
#[derive(Clone, Copy, Debug, Default)]
struct Flags {
    shown: bool,
    required: bool,
    read_only: bool,
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
    /// An attachments item's bounds (docs/adr/0407 §1).
    attachments: Option<Bounds>,
}

/// An attachments item: what it counts, its bounds and its name.
#[derive(Clone, Debug)]
struct Bounds {
    accept: AttachmentAccept,
    min: Option<u32>,
    max: Option<u32>,
    label: String,
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
        let today = variables
            .iter()
            .find(|v| v.name == "tarih")
            .map(|v| kentos_expression::value::to_text(&v.value).into_owned())
            .filter(|t| !t.is_empty());
        Ok(Prepared {
            rules: crate::LayerRules::default(),
            today,
            fields: fields.to_vec(),
            items,
            calc_order,
            clear_hidden: body.clear_hidden,
            subtypes: None,
            groups: Vec::new(),
            workflow: body.workflow.clone().and_then(|w| {
                let at = fields.iter().position(|f| f.key() == w.field)?;
                Some((w, at))
            }),
            schema,
        })
    }

    /// The form over a layer with record rules (docs/adr/0438 §1): a record breaking one is told on
    /// the rule's field. Rules that do not compile are the command's to refuse; here they are left out.
    pub fn with_rules(mut self, rules: &[kentos_contracts::LayerRule]) -> Prepared {
        self.rules = crate::LayerRules::new(
            rules,
            &self.fields,
            &self.schema.variables,
            self.schema.unit,
        )
        .unwrap_or_default();
        self
    }

    /// Whether a check asked at `on` is asked at `moment`.
    fn asked(&self, on: Option<kentos_contracts::CheckMoment>, moment: Moment) -> bool {
        use kentos_contracts::CheckMoment;
        match (on, moment) {
            (None, _) | (Some(_), Moment::Submit) => true,
            (Some(_), Moment::Change) => false,
            (Some(CheckMoment::Save), Moment::Save) => true,
            (Some(CheckMoment::Submit), Moment::Save) => self.workflow.is_none(),
        }
    }

    /// The form's workflow (docs/adr/0428).
    pub fn workflow(&self) -> Option<&kentos_contracts::FormWorkflow> {
        self.workflow.as_ref().map(|(w, _)| w)
    }

    /// The record's state in the workflow: its state field's value, else the initial state.
    pub fn state_of(&self, attrs: &Attrs) -> Option<String> {
        let (w, at) = self.workflow.as_ref()?;
        Some(
            attrs
                .get(&self.fields[*at].name)
                .map(|v| js_trim(v).to_owned())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| w.initial.clone()),
        )
    }

    /// Whether the record is in a draft state (docs/adr/0432): the form's own requirements warn.
    pub fn in_draft(&self, attrs: &Attrs) -> bool {
        match (self.workflow(), self.state_of(attrs)) {
            (Some(w), Some(s)) => w.drafts_in(&s),
            _ => false,
        }
    }

    /// Whether the record is read-only by its state (docs/adr/0428 §2).
    pub fn locked_by_state(&self, attrs: &Attrs) -> bool {
        match (self.workflow(), self.state_of(attrs)) {
            (Some(w), Some(s)) => w.locks(&s),
            _ => false,
        }
    }

    /// The form over a layer with value groups (docs/adr/0423 §2): a restrictive group narrows its
    /// fields' choices to what its rows allow with the record's other values; a combination none
    /// allows is an issue.
    pub fn with_contingents(mut self, groups: &[kentos_contracts::FieldGroup]) -> Prepared {
        self.groups = groups.to_vec();
        self
    }

    /// What the restrictive groups let the field at `index` take on the record: none when they do
    /// not narrow it.
    fn grouped(&self, index: usize, attrs: &Attrs) -> Option<Vec<String>> {
        let key = self.fields[index].key();
        let subtype = self.subtype(attrs).map(|t| t.code.clone());
        let mut out: Option<Vec<String>> = None;
        for g in self.groups.iter().filter(|g| g.restrictive) {
            let Some(at) = g.fields.iter().position(|k| k == key) else {
                continue;
            };
            let values = g.values_of(&self.fields, &|n: &str| attrs.get(n).cloned());
            if let Some(codes) = g.offered(at, &values, subtype.as_deref()) {
                out = Some(match out {
                    None => codes,
                    Some(now) => now.into_iter().filter(|c| codes.contains(c)).collect(),
                });
            }
        }
        out
    }

    /// The form over a layer with `subtypes` (docs/adr/0422 §2): a record's subtype narrows the values
    /// its choices offer, gives a new record its defaults and makes fields required.
    pub fn with_subtypes(mut self, subtypes: Option<&kentos_contracts::LayerSubtypes>) -> Prepared {
        self.subtypes = subtypes.cloned();
        self
    }

    /// The subtype of the record with `attrs`.
    fn subtype(&self, attrs: &Attrs) -> Option<&kentos_contracts::Subtype> {
        self.subtypes
            .as_ref()?
            .of(&self.fields, |n| attrs.get(n).cloned())
    }

    /// The layer fields' keys the form shows, in its order.
    pub fn keys(&self) -> Vec<&str> {
        self.bound()
            .map(|(_, b)| self.fields[b.index].key())
            .collect()
    }

    /// The keys of the fields whose values are unique in the layer.
    pub fn unique_keys(&self) -> Vec<&str> {
        // A field the layer keeps unique is the form's too (docs/adr/0438 §3).
        self.bound()
            .filter(|(_, b)| b.item.unique || self.fields[b.index].unique)
            .map(|(_, b)| self.fields[b.index].key())
            .collect()
    }

    /// The suggesting field items (docs/adr/0407 §3): each item's id and its layer field's name.
    pub fn suggesting(&self) -> Vec<(&str, &str)> {
        self.bound()
            .filter(|(_, b)| b.item.suggest)
            .map(|(n, b)| (n.id.as_str(), self.fields[b.index].name.as_str()))
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
        self.items
            .iter()
            .zip(self.flags(attrs, object))
            .map(|(node, f)| FormItemState {
                id: node.id.clone(),
                shown: f.shown,
                required: f.required,
                read_only: f.read_only,
                choices: None,
                text: None,
                options: None,
                more: None,
                preview: None,
                related: None,
                suggestions: None,
                attachments: None,
            })
            .collect()
    }

    /// Each item's shown, required and read-only, in the form's order: what [`Prepared::states`]
    /// says, without the items' ids (docs/adr/0424: a layer's audit asks every record, and the ids'
    /// copies were most of its time).
    fn flags(&self, attrs: &Attrs, object: &Object) -> Vec<Flags> {
        let mut out: Vec<Flags> = Vec::with_capacity(self.items.len());
        // The record's subtype, once (docs/adr/0422 §2).
        let subtype = self.subtype(attrs);
        // A workflow's state field changes by its steps only; a locked state locks the whole record
        // (docs/adr/0428 §2).
        let state_field = self.workflow.as_ref().map(|(_, at)| *at);
        let locked = self.locked_by_state(attrs);
        // On a locked record the reasons of the steps out of its state stay open, to be written.
        let reasons: Vec<&str> = match (locked, self.workflow(), self.state_of(attrs)) {
            (true, Some(w), Some(state)) => w
                .steps_from(&state)
                .filter_map(|s| s.reason.as_deref())
                .collect(),
            _ => Vec::new(),
        };
        for node in &self.items {
            let here = node.parent.is_none_or(|p| out[p].shown)
                && node
                    .visible
                    .as_ref()
                    .is_none_or(|e| holds(e, attrs, object));
            let (required, read_only) = node.field.as_ref().map_or((false, false), |b| {
                // A field the layer requires is required on every form (the preflight shows it always).
                let required = here
                    && (b.item.required
                        || self.fields[b.index].required
                        || subtype.is_some_and(|t| {
                            let key = self.fields[b.index].key();
                            t.fields.iter().any(|r| r.required && r.field == key)
                        })
                        || b.required_when
                            .as_ref()
                            .is_some_and(|e| holds(e, attrs, object)));
                let read_only = b.item.read_only
                    || (locked && !reasons.contains(&self.fields[b.index].key()))
                    || state_field == Some(b.index)
                    || self
                        .workflow()
                        .and_then(|w| w.log.as_deref())
                        .is_some_and(|k| k == self.fields[b.index].key())
                    || (b.calculate.is_some() && !b.item.overridable)
                    || b.read_only_when
                        .as_ref()
                        .is_some_and(|e| holds(e, attrs, object));
                (required, read_only)
            });
            out.push(Flags {
                shown: here,
                required,
                read_only,
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
            if let Some(b) = node.field.as_ref()
                && let Some(list) = &b.lookup
            {
                // A page of the list and the count of the rest (docs/adr/0439 §1); the chosen
                // object's columns, live.
                let mut options = list.choices(attrs, object);
                if let Some(n) = list.limit
                    && options.len() > n
                {
                    state.more = u32::try_from(options.len() - n).ok();
                    options.truncate(n);
                }
                state.options = Some(options);
                let name = &self.fields[b.index].name;
                state.preview = attrs
                    .get(name)
                    .map(|v| js_trim(v))
                    .filter(|v| !v.is_empty())
                    .and_then(|v| list.preview(v));
            }
            // A hidden item's related objects are not looked for (docs/adr/0440 §6).
            if let Some(related) = node.related.as_ref().filter(|_| state.shown) {
                state.related = Some(related.of(object));
            }
            if let (Some(b), Some(held)) = (&node.attachments, &object.attachments) {
                state.attachments = Some(held.counted(b.accept));
            }
        }
        states
    }

    /// A field item's whole list from another layer for the record, past its page (docs/adr/0439 §1):
    /// what a searched list searches; none for an item without one.
    pub fn options(&self, item: &str, attrs: &Attrs, object: &Object) -> Option<Vec<FieldChoice>> {
        let b = self.items.iter().find(|n| n.id == item)?.field.as_ref()?;
        Some(b.lookup.as_ref()?.choices(attrs, object))
    }

    /// The listed values a field item offers for the record, in the list's order: none when the item
    /// has no filter (every listed value is offered).
    pub fn choices(&self, item: &str, attrs: &Attrs, object: &Object) -> Option<Vec<String>> {
        let b = self.items.iter().find(|n| n.id == item)?.field.as_ref()?;
        if let Some(list) = &b.lookup {
            return Some(
                list.offered(attrs, object)
                    .into_iter()
                    .map(|r| r.code.clone())
                    .collect(),
            );
        }
        let field = &self.fields[b.index];
        // A subtype that narrows the field offers its values only (docs/adr/0422 §2), and a value
        // the record holds.
        let subtype = self.subtype(attrs);
        let narrows = subtype.is_some_and(|t| {
            t.fields
                .iter()
                .any(|r| r.field == field.key() && r.values.is_some())
        });
        let grouped = self.grouped(b.index, attrs);
        if b.choices.is_none() && !narrows && grouped.is_none() {
            return None;
        }
        let held_codes: Vec<String> = attrs.get(&field.name).map_or_else(Vec::new, |v| {
            if field.holds_many() {
                field.codes_of(v).into_iter().map(str::to_owned).collect()
            } else {
                vec![js_trim(v).to_owned()]
            }
        });
        let listed: Vec<(String, Option<&Expr>)> = match &b.choices {
            Some(c) => c.iter().map(|(code, e)| (code.clone(), Some(e))).collect(),
            None => field
                .values
                .iter()
                .flatten()
                .map(|c| (c.code.clone(), None))
                .collect(),
        };
        // A retired value is offered only to the record that holds it (docs/adr/0418).
        Some(
            listed
                .into_iter()
                .filter(|(code, _)| {
                    !field.is_withdrawn_on(code, self.today.as_deref()) || held_codes.contains(code)
                })
                .filter(|(code, _)| {
                    kentos_contracts::subtype_offers(field, subtype, code)
                        || held_codes.contains(code)
                })
                .filter(|(code, _)| {
                    grouped.as_ref().is_none_or(|g| g.contains(code)) || held_codes.contains(code)
                })
                .filter(|(_, e)| e.is_none_or(|e| holds(e, attrs, object)))
                .map(|(code, _)| code)
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

    /// The record's fields' names the lists copy into or stamp.
    fn copy_targets(&self) -> std::collections::HashSet<&str> {
        self.items
            .iter()
            .filter_map(|n| n.field.as_ref()?.lookup.as_ref())
            .flat_map(|l| l.targets())
            .collect()
    }

    /// `attrs` with the copies of each list whose value was chosen anew (docs/adr/0439 §3): its value
    /// differs from `before`'s (none: a new record), or its stamp names another value. The chosen
    /// object's fields are copied over the record's, the stamp says where from; an emptied value
    /// leaves the copies as they are.
    pub fn copied(&self, before: Option<&Attrs>, attrs: &Attrs) -> Attrs {
        let mut out = attrs.clone();
        for b in self.items.iter().filter_map(|n| n.field.as_ref()) {
            let Some(list) = b.lookup.as_ref().filter(|l| l.copies()) else {
                continue;
            };
            let field = &self.fields[b.index];
            if field.multiple {
                continue;
            }
            let now = attrs.get(&field.name).map_or("", |v| js_trim(v));
            if now.is_empty() {
                continue;
            }
            let was = before
                .and_then(|a| a.get(&field.name))
                .map_or("", |v| js_trim(v));
            let mut copied = out.clone();
            list.copy_into(&mut copied, now, self.today.as_deref());
            // A value kept whose copies already stand is left alone: a copy is not a live view.
            if before.is_some() && was == now && !list.stale_stamp(&out, now) {
                continue;
            }
            out = copied;
        }
        out
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
            // A new record starts in the workflow's initial state (docs/adr/0428 §2).
            if let Some((w, at)) = &self.workflow
                && is_empty(&out, &self.fields[*at].name)
            {
                out.insert(self.fields[*at].name.clone(), w.initial.clone());
            }
            self.defaults(&mut out, object, &mut issues);
        }
        for round in 0..ROUNDS {
            let start = out.clone();
            let mut round_issues = Vec::new();
            let shown = if self.clear_hidden {
                Some(self.flags(&out, object))
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
                let states = self.flags(&out, object);
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
                && let Ok(v) = check_loose(field, default)
                && !v.is_empty()
            {
                out.insert(field.name.clone(), v);
            }
        }
        // The record's subtype's defaults, where the form gives none (docs/adr/0422 §2).
        if self.subtypes.is_some() {
            for (_, b) in self.bound() {
                let field = &self.fields[b.index];
                if b.item.default.is_some()
                    || b.default_expr.is_some()
                    || !is_empty(out, &field.name)
                {
                    continue;
                }
                let Some(t) = self.subtype(out) else { break };
                if let Some(d) = t
                    .fields
                    .iter()
                    .find(|r| r.field == field.key())
                    .and_then(|r| r.default.clone())
                {
                    out.insert(field.name.clone(), d);
                }
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
            match check_loose(field, &text) {
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
        // An overridable calculation fills the field only while it is empty (docs/adr/0417).
        if b.item.overridable && out.get(&field.name).is_some_and(|v| !v.is_empty()) {
            return;
        }
        let Some(text) = field_text_of(e, out, object) else {
            out.remove(&field.name);
            return;
        };
        match check_loose(field, &text) {
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
        self.validate_at(attrs, object, Moment::Save)
    }

    /// [`Prepared::validate`] asking the checks of `moment` (docs/adr/0438 §2): as the value changes
    /// the checks without a moment, on saving those asked on saving too, on a workflow step every one.
    /// The layer's record rules are asked whatever the moment.
    pub fn validate_at(&self, attrs: &Attrs, object: &Object, moment: Moment) -> Vec<FormIssue> {
        let states = self.flags(attrs, object);
        let draft = self.in_draft(attrs);
        let subtype = self.subtype(attrs);
        let mut issues = Vec::new();
        for (node, state) in self.items.iter().zip(&states) {
            if let (Some(bounds), Some(held), true) =
                (&node.attachments, &object.attachments, state.shown)
            {
                issues.extend(attachment_issues(
                    &node.id,
                    bounds,
                    held.counted(bounds.accept),
                ));
            }
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
                    // A draft keeps the form's own requirements as warnings; the layer's and the
                    // subtype's refuse all the same (docs/adr/0432).
                    let dataset = field.required
                        || subtype.is_some_and(|t| {
                            t.fields
                                .iter()
                                .any(|r| r.required && r.field == field.key())
                        });
                    if draft && !dataset {
                        issues.push(at(
                            codes::REQUIRED,
                            FormSeverity::Warning,
                            None,
                            format!("“{label}” zorunlu; taslakta boş kalabilir, ilerletmeden önce doldurun."),
                        ));
                    } else {
                        issues.push(at(
                            codes::REQUIRED,
                            FormSeverity::Error,
                            None,
                            format!("“{label}” zorunlu; doldurun."),
                        ));
                    }
                }
                continue;
            }
            let canonical = match check_loose(field, value) {
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
            if crate::widget_of(&b.item, field) == FormWidget::Color && !crate::is_colour(value) {
                issues.push(at(
                    codes::COLOR_INVALID,
                    FormSeverity::Error,
                    None,
                    crate::colour_message(label, value),
                ));
                continue;
            }
            let held: Vec<&str> = if field.multiple {
                field.codes_of(&canonical)
            } else {
                vec![canonical.as_str()]
            };
            if let Some(list) = &b.lookup {
                let offered = list.offered(attrs, object);
                if held
                    .iter()
                    .any(|code| !offered.iter().any(|r| r.code == *code))
                {
                    issues.push(at(
                        codes::LOOKUP_MISSING,
                        FormSeverity::Error,
                        Some("lookup".into()),
                        format!("“{label}” için “{value}” listede yok; listeden seçin."),
                    ));
                    continue;
                }
                // A code more than one object holds (docs/adr/0439 §4).
                if list.policy != kentos_contracts::LookupDuplicates::First
                    && let Some((code, n)) = held
                        .iter()
                        .find_map(|c| list.duplicated(c).map(|n| (*c, n)))
                {
                    let refused = list.policy == kentos_contracts::LookupDuplicates::Refuse;
                    issues.push(at(
                        codes::LOOKUP_DUPLICATE,
                        if refused {
                            FormSeverity::Error
                        } else {
                            FormSeverity::Warning
                        },
                        Some("lookup".into()),
                        if refused {
                            format!(
                                "“{label}” için “{code}” listenin katmanında {n} kayıtta var; anahtar tekil değil, başka bir değer seçin ya da kaynağı düzeltin."
                            )
                        } else {
                            format!(
                                "“{label}” için “{code}” listenin katmanında {n} kayıtta var; ilkininki alındı. Kaynağı düzeltin ya da yinelenen kuralını seçin."
                            )
                        },
                    ));
                    if refused {
                        continue;
                    }
                }
                // The copy's source changed after the copy (docs/adr/0439 §3).
                if !field.multiple && list.stale(attrs, &canonical) {
                    issues.push(at(
                        codes::LOOKUP_STALE,
                        FormSeverity::Warning,
                        Some("lookup.stamp".into()),
                        format!(
                            "“{label}” için kopyalanan değerler kaynakta değişti; değeri yeniden seçerek güncelleyin."
                        ),
                    ));
                }
            }
            // A multiple field's codes are asked one by one (docs/adr/0408 §2).
            let codes: Vec<&str> = if field.multiple {
                field.codes_of(&canonical)
            } else {
                vec![canonical.as_str()]
            };
            if let Some(choices) = &b.choices
                && codes.iter().any(|code| {
                    choices
                        .iter()
                        .find(|(c, _)| c == code)
                        .is_some_and(|(_, e)| !holds(e, attrs, object))
                })
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
                if self.asked(check.on, moment) && !holds(expr, attrs, object) {
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
        for broken in self.rules.broken(&self.fields, None, attrs, object) {
            let item = broken
                .field
                .and_then(|f| self.fields.iter().position(|x| x.key() == f.key()))
                .and_then(|i| self.item_of(i));
            issues.push(issue(
                codes::LAYER_RULE,
                if broken.rule.warning {
                    FormSeverity::Warning
                } else {
                    FormSeverity::Error
                },
                item,
                broken.field.map(|f| f.key()),
                Some(&format!("rules/{}", broken.rule.id)),
                broken.rule.message.clone(),
            ));
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
        let states = self.flags(attrs, object);
        let wanted: Vec<(usize, &Bound, String)> = self
            .items
            .iter()
            .zip(&states)
            .enumerate()
            .filter_map(|(at, (node, state))| {
                let b = node.field.as_ref()?;
                let v = attrs.get(&self.fields[b.index].name)?;
                let unique = b.item.unique || self.fields[b.index].unique;
                (unique && state.shown && !js_trim(v).is_empty()).then(|| (at, b, v.clone()))
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
        self.assess_at(before, values, object, new, Moment::Save)
    }

    /// [`Prepared::assess`] asking the checks of `moment` (docs/adr/0438 §2): a screen asks as the
    /// values change, and on saving.
    pub fn assess_at(
        &self,
        before: &Attrs,
        values: &[(String, String)],
        object: &Object,
        new: bool,
        moment: Moment,
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
            // A list from another layer takes a value's label for its code (docs/adr/0404 §2), each
            // of a multiple field's (docs/adr/0409 §1).
            let list = self
                .bound()
                .find(|(_, b)| b.index == index)
                .and_then(|(_, b)| b.lookup.as_ref());
            let coded: String = match list {
                Some(list) if field.multiple => text
                    .split(';')
                    .map(|p| list.code_of(p))
                    .collect::<Vec<_>>()
                    .join(kentos_contracts::MULTIPLE_JOIN),
                Some(list) => list.code_of(text).to_owned(),
                None => text.clone(),
            };
            match check_loose(field, &coded) {
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
        // A list's value chosen anew brings its copies (docs/adr/0439 §3).
        let written = self.copied((!new).then_some(before), &written);
        let (after, completed) = self.complete(&written, object, new);
        issues.extend(self.locked(&changed, &after, object));
        issues.extend(self.retired(&changed, before, &after));
        issues.extend(self.misfits(&changed, before, &after));
        issues.extend(self.combinations(&changed, &after, new));
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
            self.validate_at(&after, object, moment)
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
        // A list's copies written as the list copies them are the form's (docs/adr/0439 §3).
        let copies = self.copied(Some(before), after);
        let targets = self.copy_targets();
        for (index, field) in self.fields.iter().enumerate() {
            if before.get(&field.name) == after.get(&field.name) {
                continue;
            }
            if targets.contains(field.name.as_str())
                && copies.get(&field.name) == after.get(&field.name)
            {
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
        issues.extend(self.retired(&changed, before, &completed));
        issues.extend(self.misfits(&changed, before, &completed));
        issues.extend(self.combinations(&changed, &completed, false));
        issues.extend(worked);
        issues.extend(self.validate(&completed, object));
        issues
    }

    /// The changed fields the form hides or locks in the record after the change.
    /// A changed field given a retired value its record did not hold (docs/adr/0418): a value the
    /// record keeps stays, a new one is refused.
    fn retired(&self, changed: &[usize], before: &Attrs, after: &Attrs) -> Vec<FormIssue> {
        let mut issues = Vec::new();
        for &index in changed {
            let field = &self.fields[index];
            let today = self.today.as_deref();
            let held = before
                .get(&field.name)
                .map_or_else(Vec::new, |v| field.retired_in_on(v, today));
            let now = after
                .get(&field.name)
                .map_or_else(Vec::new, |v| field.retired_in_on(v, today));
            if let Some(r) = now.iter().find(|r| !held.contains(r)) {
                // Retired, or outside its validity on the day the rules read (docs/adr/0438 §5).
                let message = if field.is_retired(r) {
                    format!(
                        "“{}” alanının “{r}” değeri kullanımdan kalktı; başka bir değer seçin.",
                        field.label()
                    )
                } else {
                    format!(
                        "“{}” alanının “{r}” değeri bu tarihte geçerli değil; başka bir değer seçin.",
                        field.label()
                    )
                };
                issues.push(refused(
                    codes::VALUE_RETIRED,
                    self.item_of(index),
                    field.key(),
                    message,
                ));
            }
        }
        issues
    }

    /// The values the record's subtype does not offer (docs/adr/0422 §2): one written anew, or every
    /// one when the subtype itself changed; a value the record keeps under its kind stays.
    fn misfits(&self, changed: &[usize], before: &Attrs, after: &Attrs) -> Vec<FormIssue> {
        let Some(subtypes) = &self.subtypes else {
            return Vec::new();
        };
        let Some(t) = self.subtype(after) else {
            return Vec::new();
        };
        let kind_changed = subtypes
            .sorter(&self.fields)
            .is_some_and(|s| before.get(&s.name) != after.get(&s.name));
        let words = subtypes.label_of(&self.fields, t).to_owned();
        let mut issues = Vec::new();
        for (f, codes) in
            kentos_contracts::subtype_misfits(&self.fields, Some(t), |n| after.get(n).cloned())
        {
            let Some(index) = self.fields.iter().position(|x| x.key() == f.key()) else {
                continue;
            };
            if !kind_changed && !changed.contains(&index) {
                continue;
            }
            issues.push(refused(
                codes::VALUE_NOT_IN_SUBTYPE,
                self.item_of(index),
                f.key(),
                format!(
                    "“{}” alanının “{}” değeri “{words}” alt türünde yok; alt türün değerlerinden birini seçin ya da alanı boşaltın.",
                    f.label(),
                    codes.join(", ")
                ),
            ));
        }
        issues
    }

    /// The value groups whose fields changed (any, for a new record) and whose rows do not allow the
    /// record's values (docs/adr/0423 §2): an error for a restrictive group, a warning for an advisory
    /// one, at the first changed field of the group.
    fn combinations(&self, changed: &[usize], after: &Attrs, new: bool) -> Vec<FormIssue> {
        if self.groups.is_empty() {
            return Vec::new();
        }
        let subtype = self.subtype(after).map(|t| t.code.clone());
        let mut issues = Vec::new();
        for u in kentos_contracts::unmet(&self.groups, &self.fields, subtype.as_deref(), |n| {
            after.get(n).cloned()
        }) {
            let first = u.group.fields.iter().find_map(|k| {
                let index = self.fields.iter().position(|f| f.key() == k)?;
                (changed.contains(&index) || new).then_some(index)
            });
            let Some(index) = first else { continue };
            let words = kentos_contracts::combination_words(u.group, &self.fields, &u.values);
            let severity = if u.group.restrictive {
                FormSeverity::Error
            } else {
                FormSeverity::Warning
            };
            issues.push(issue(
                codes::COMBINATION,
                severity,
                self.item_of(index),
                Some(self.fields[index].key()),
                None,
                format!("“{}” değer grubunda bu bileşim yok: {words}.", u.group.name),
            ));
        }
        issues
    }

    fn locked(&self, changed: &[usize], after: &Attrs, object: &Object) -> Vec<FormIssue> {
        let states = self.flags(after, object);
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

    /// Whether the field is calculated and locked (an overridable calculation leaves it open).
    fn calculated(&self, index: usize) -> bool {
        self.bound()
            .any(|(_, b)| b.index == index && b.calculate.is_some() && !b.item.overridable)
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
        let mut attachments = None;
        let (visible, field, note) = match item {
            FormItem::Attachments {
                label,
                accept,
                min,
                max,
                visible_when,
                ..
            } => {
                attachments = Some(Bounds {
                    accept: *accept,
                    min: *min,
                    max: *max,
                    label: label.clone().unwrap_or_else(|| match accept {
                        AttachmentAccept::Any => "Ekler".into(),
                        AttachmentAccept::Photo => "Fotoğraflar".into(),
                        AttachmentAccept::Document => "Belgeler".into(),
                    }),
                });
                (compile(visible_when), None, None)
            }
            FormItem::Group { visible_when, .. } | FormItem::Location { visible_when, .. } => {
                (compile(visible_when), None, None)
            }
            FormItem::Related {
                layer,
                field,
                visible_when,
                ..
            } => {
                related = related_of(layer, field, ctx.world);
                (compile(visible_when), None, None)
            }
            // A mirror is shown, not bound: the field's own item writes it (docs/adr/0437).
            FormItem::Field(f) if f.mirror => (compile(&f.visible_when), None, None),
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
            FormItem::Tab { visible_when, .. } => (compile(visible_when), None, None),
        };
        out.push(Node {
            id: item.id().to_owned(),
            parent,
            visible,
            field,
            note,
            related,
            attachments,
        });
        walk(item.items(), Some(at), out, ctx);
    }
}

/// What an attachments item says of a record holding `count` of what it counts.
fn attachment_issues(id: &str, b: &Bounds, count: u32) -> Option<FormIssue> {
    let what = match b.accept {
        AttachmentAccept::Any => "ek",
        AttachmentAccept::Photo => "fotoğraf",
        AttachmentAccept::Document => "belge",
    };
    let at = |code: &str, rule: &str, message: String| {
        issue(
            code,
            FormSeverity::Error,
            Some(id),
            None,
            Some(rule),
            message,
        )
    };
    if let Some(min) = b.min.filter(|m| count < *m) {
        return Some(at(
            codes::ATTACHMENTS_FEW,
            "min",
            format!(
                "“{}” en az {min} {what} ister; kayıtta {count} var.",
                b.label
            ),
        ));
    }
    if let Some(max) = b.max.filter(|m| count > *m) {
        return Some(at(
            codes::ATTACHMENTS_MANY,
            "max",
            format!(
                "“{}” en çok {max} {what} alır; kayıtta {count} var.",
                b.label
            ),
        ));
    }
    None
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

/// The field's value checked without its own `required`: the form asks for it only while it is shown.
/// A field that is not required is checked as it is, without a copy (docs/adr/0424: a record's
/// check ran once a field, and a copy of a long value list each time was most of a layer's audit).
fn check_loose(
    field: &LayerField,
    text: &str,
) -> Result<String, kentos_contracts::fields::Refusal> {
    if !field.required {
        return check_value(field, text);
    }
    let mut f = field.clone();
    f.required = false;
    check_value(&f, text)
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
                retired: false,
                description: None,
                from: None,
                until: None,
            },
            kentos_contracts::FieldChoice {
                code: "b".into(),
                label: "Betonarme".into(),
                retired: false,
                description: None,
                from: None,
                until: None,
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

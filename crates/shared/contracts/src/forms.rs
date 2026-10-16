//! The project's forms (docs/adr/0401, `plan/form.md`'s `core/1` profile): a form is a layer's data
//! entry screen, kept with the project: tabs, groups and fields bound to the layer's fields by their
//! stable keys ([`LayerField::key`](crate::LayerField::key)), the rules that show, require or lock a
//! field, and the checks a record must pass. A form has a draft, edited by the designer, and
//! releases: frozen copies with their digest that data entry uses. A form that enforces its rules
//! makes its latest release the layer's write profile: an attribute write that breaks it is refused
//! wherever it comes from.
//!
//! These are the shapes and their structural rules; what the rules mean (fields that exist, kinds a
//! widget takes, expressions that compile, a required field that can be hidden) is the forms core's
//! (`kentos-forms`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The forms' format and profile, as a form's tools name them.
pub const FORMS_PROFILE: &str = "kentos.forms/1 core";

/// The most forms a project keeps.
pub const MOST_FORMS: usize = 100;
/// The most items (tabs, groups, fields, notes) a form's body has, all levels counted.
pub const MOST_FORM_ITEMS: usize = 500;
/// The most releases a form keeps; publishing past it drops the oldest.
pub const MOST_FORM_RELEASES: usize = 50;
/// The longest name, label or id, in characters.
pub const MOST_FORM_NAME: usize = 64;
/// The longest help, note, message or placeholder, in characters.
pub const MOST_FORM_TEXT: usize = 1000;
/// The longest rule expression, in characters.
pub const MOST_FORM_RULE: usize = 2000;
/// The most checks a field has.
pub const MOST_FORM_CHECKS: usize = 20;
/// How deep tabs and groups nest.
pub const MOST_FORM_DEPTH: usize = 4;

/// A form of the project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectForm {
    /// Stable, unique in the project: letters, digits, `-`, `_` and `.`; 1–64 characters.
    pub id: String,
    /// What the lists show; trimmed, 1–64 characters.
    pub name: String,
    /// The layer the form edits: a layer's id (not a group's).
    pub layer: String,
    /// The draft the designer edits.
    pub draft: FormBody,
    /// The frozen releases, oldest first; data entry uses the last.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<FormRelease>>", optional))]
    pub releases: Vec<FormRelease>,
    /// The latest release's rules hold for every attribute write on the layer (the write profile);
    /// written only as `true`, and only with a release.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub enforce: bool,
}

/// A form's content: its items and the draft's revision (each kept change adds one).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormBody {
    /// Raised by one with every saved change to the draft: an update names the revision it was made on.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    pub items: Vec<FormItem>,
    /// A write through the form empties the fields the record hides (docs/adr/0402 §3); written only
    /// as `true`. Without it a hidden field keeps its value.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub clear_hidden: bool,
}

/// A frozen copy of a form's body: what data entry and the write profile use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormRelease {
    /// 1, 2, …: one more than the release before.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
    /// The first 32 hex digits of the SHA-256 of the body's canonical JSON ([`body_digest`]).
    pub digest: String,
    pub body: FormBody,
    /// What changed, as the publisher wrote it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

/// An item of a form: a tab (only at the top), a group, a field or a note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
// A form's items are data read once per form, never a hot path: a field's settings stay inline.
#[allow(clippy::large_enum_variant)]
pub enum FormItem {
    /// A tab: its items show when it is chosen. Tabs stand at the top only, and then the top holds
    /// nothing but tabs.
    Tab {
        id: String,
        label: String,
        items: Vec<FormItem>,
    },
    /// A group: a titled box of items, in one or two columns, that a rule may hide.
    Group {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        label: Option<String>,
        /// 1 or 2; none: 1.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        columns: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        visible_when: Option<String>,
        /// It opens closed (its title opens it); written only as `true`.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        collapsed: bool,
        items: Vec<FormItem>,
    },
    /// A field of the layer.
    Field(FormField),
    /// A line of text: a hint for whoever fills the form. A `{…}` in it is an expression put in with
    /// the record's value (docs/adr/0225 §6's templates; docs/adr/0402 §1).
    Note { id: String, text: String },
    /// The objects of another layer whose field holds this record's persistent id (docs/adr/0404 §4):
    /// a list in the form, each opened by its own form.
    Related {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        label: Option<String>,
        /// The other layer's id.
        layer: String,
        /// Its field's key that holds this record's id.
        field: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        visible_when: Option<String>,
    },
}

/// A field's list of values from another layer (docs/adr/0404 §1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormLookup {
    /// The list's layer id.
    pub layer: String,
    /// Its field's key whose value is written; `$uid`: the object's persistent id (a reference).
    pub key: String,
    /// Its field's key shown; none: the key's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// The list's object is offered when its field equals the record's (a chain: il, ilçe, mahalle).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<LookupMatch>>", optional))]
    pub matches: Vec<LookupMatch>,
    /// Asked of each of the list's objects (its fields by name, the project's `@` values).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<String>,
}

/// A field of the list's layer that must equal a field of the record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LookupMatch {
    /// The list's field's key.
    pub field: String,
    /// The record's field's key.
    pub record: String,
}

/// The lookup key that writes the object's persistent id (docs/adr/0404 §3).
pub const LOOKUP_UID: &str = "$uid";

/// A field of a form: the layer's field it edits (by its key) and how.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormField {
    /// The item's id, unique in the form.
    pub id: String,
    /// The layer field's key ([`LayerField::key`](crate::LayerField::key)): its stable id or its name.
    pub field: String,
    /// The label; none: the field's alias or name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// A line under the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub help: Option<String>,
    /// The empty box's hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub placeholder: Option<String>,
    /// How it is edited; none: the field kind's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub widget: Option<FormWidget>,
    /// Shown, never edited; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub read_only: bool,
    /// May not be left empty while shown; written only as `true`. The layer field's own `required`
    /// holds as well.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub required: bool,
    /// Shown only when this expression is true (the record's fields by name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub visible_when: Option<String>,
    /// Required when this expression is true (and the field is shown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub required_when: Option<String>,
    /// Locked when this expression is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub read_only_when: Option<String>,
    /// The value an empty field shows when the form opens, in the field's canonical text; written
    /// only when the record is saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub default: Option<String>,
    /// The value of a new record's empty field: an expression over the record (its plain defaults
    /// in), evaluated in the form's order. Not with `default`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub default_expr: Option<String>,
    /// The field's value, worked out from the record at every write; the field is read-only. Not
    /// with `default`, `defaultExpr` or `readOnlyWhen`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub calculate: Option<String>,
    /// Asked of each of the field's listed values: `@secenek` is its code, `@secenek_etiketi` its
    /// label; a value it is not true for is not offered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub choice_filter: Option<String>,
    /// No two objects of the layer have the same value (empty ones aside); written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub unique: bool,
    /// A new record's field opens with the value last written through the form in the session (the
    /// form's interface keeps it); written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub reuse_last: bool,
    /// The field's values from another layer (docs/adr/0404 §1). Not with the field's own list or
    /// `choiceFilter`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lookup: Option<FormLookup>,
    /// What the record must satisfy, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<FormCheck>>", optional))]
    pub checks: Vec<FormCheck>,
}

impl FormField {
    /// A field item `id` bound to `field`, with no rule.
    pub fn new(id: impl Into<String>, field: impl Into<String>) -> FormField {
        FormField {
            id: id.into(),
            field: field.into(),
            label: None,
            help: None,
            placeholder: None,
            widget: None,
            read_only: false,
            required: false,
            visible_when: None,
            required_when: None,
            read_only_when: None,
            default: None,
            default_expr: None,
            calculate: None,
            choice_filter: None,
            unique: false,
            reuse_last: false,
            lookup: None,
            checks: Vec::new(),
        }
    }
}

/// A check of a field: an expression that must be true when the field is shown and filled, and what
/// the form says when it is not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormCheck {
    pub rule: String,
    pub message: String,
    /// A soft check: when not true it warns and the record is still written; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub warning: bool,
}

/// How a field is edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FormWidget {
    /// One line of text.
    Text,
    /// Several lines of text.
    TextArea,
    /// A number: integer or decimal fields.
    Number,
    /// A date (YYYY-AA-GG).
    Date,
    /// A check box: yes or no fields.
    Check,
    /// A switch: yes or no fields.
    Switch,
    /// A drop-down list: fields with a value list.
    List,
    /// Radio buttons: fields with a value list.
    Radio,
    /// A segmented choice: fields with a short value list.
    Segmented,
}

impl FormWidget {
    pub const ALL: [FormWidget; 9] = [
        FormWidget::Text,
        FormWidget::TextArea,
        FormWidget::Number,
        FormWidget::Date,
        FormWidget::Check,
        FormWidget::Switch,
        FormWidget::List,
        FormWidget::Radio,
        FormWidget::Segmented,
    ];

    /// Its name in the interface.
    pub fn label(self) -> &'static str {
        match self {
            FormWidget::Text => "Tek satır",
            FormWidget::TextArea => "Çok satır",
            FormWidget::Number => "Sayı",
            FormWidget::Date => "Tarih",
            FormWidget::Check => "Onay kutusu",
            FormWidget::Switch => "Anahtar",
            FormWidget::List => "Açılır liste",
            FormWidget::Radio => "Seçenek düğmeleri",
            FormWidget::Segmented => "Parçalı seçim",
        }
    }
}

/// How much an issue weighs: an error stops a save or a publish, a warning is said.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FormSeverity {
    Error,
    Warning,
}

/// What a form's preflight or a record's validation found (docs/adr/0401 §4, §6): a stable code,
/// where (the item, the field's key, the rule) and the sentence that says it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormIssue {
    /// `form_field_unknown`, `form_required`, … (the forms core's `codes`).
    pub code: String,
    pub severity: FormSeverity,
    /// The item it is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub item: Option<String>,
    /// The layer field's key it is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
    /// The rule it is about: `visibleWhen`, `requiredWhen`, `readOnlyWhen`, `default`, `checks/0` …
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rule: Option<String>,
    pub message: String,
}

/// An item's state for a record: shown (its own rule and its group's), required, read-only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FormItemState {
    pub id: String,
    pub shown: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub required: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub read_only: bool,
    /// The listed values a field with a choice filter offers for the record, in the list's order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub choices: Option<Vec<String>>,
    /// A note's text with the record's values put in (a note whose text has `{…}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    /// A field's list from another layer for the record: codes and labels (docs/adr/0404 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub options: Option<Vec<crate::FieldChoice>>,
    /// A related item's objects: their persistent ids (docs/adr/0404 §4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub related: Option<Vec<String>>,
}

impl FormItem {
    /// The item's id.
    pub fn id(&self) -> &str {
        match self {
            FormItem::Tab { id, .. }
            | FormItem::Group { id, .. }
            | FormItem::Note { id, .. }
            | FormItem::Related { id, .. } => id,
            FormItem::Field(f) => &f.id,
        }
    }

    /// The items it holds (a tab's or a group's).
    pub fn items(&self) -> &[FormItem] {
        match self {
            FormItem::Tab { items, .. } | FormItem::Group { items, .. } => items,
            _ => &[],
        }
    }

    /// The items it holds, to change.
    pub fn items_mut(&mut self) -> Option<&mut Vec<FormItem>> {
        match self {
            FormItem::Tab { items, .. } | FormItem::Group { items, .. } => Some(items),
            _ => None,
        }
    }
}

impl FormBody {
    /// Every item, depth first, with its depth (0 at the top).
    pub fn walk(&self) -> Vec<(usize, &FormItem)> {
        fn go<'a>(items: &'a [FormItem], depth: usize, out: &mut Vec<(usize, &'a FormItem)>) {
            for i in items {
                out.push((depth, i));
                go(i.items(), depth + 1, out);
            }
        }
        let mut out = Vec::new();
        go(&self.items, 0, &mut out);
        out
    }

    /// Every field item, depth first.
    pub fn fields(&self) -> Vec<&FormField> {
        self.walk()
            .into_iter()
            .filter_map(|(_, i)| match i {
                FormItem::Field(f) => Some(f),
                _ => None,
            })
            .collect()
    }
}

impl ProjectForm {
    /// The release data entry uses: the last.
    pub fn release(&self) -> Option<&FormRelease> {
        self.releases.last()
    }

    /// The body data entry uses: the latest release's, else the draft's (a form not yet
    /// published is tried out on its draft).
    pub fn live(&self) -> &FormBody {
        self.release().map_or(&self.draft, |r| &r.body)
    }

    /// The layer field keys the form binds, in its draft and its releases.
    pub fn bound_keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = Vec::new();
        let bodies = std::iter::once(&self.draft).chain(self.releases.iter().map(|r| &r.body));
        for body in bodies {
            for f in body.fields() {
                if !keys.contains(&f.field.as_str()) {
                    keys.push(&f.field);
                }
            }
        }
        keys
    }
}

/// The digest a release keeps of its body: the first 32 hex digits of the SHA-256 of its
/// canonical JSON (docs/adr/0253 §2's rule).
pub fn body_digest(body: &FormBody) -> String {
    let value = serde_json::to_value(body).unwrap_or(serde_json::Value::Null);
    crate::canonical::id_of(&value)
}

/// Whether `id` can name a form or an item: 1–64 of letters, digits, `-`, `_` and `.`.
pub fn form_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.chars().count() <= MOST_FORM_NAME
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Whether `name` names a form, a tab or a label: trimmed, 1–64 characters, no control characters.
pub fn form_name_ok(name: &str) -> bool {
    let t = name.trim();
    !t.is_empty()
        && t == name
        && name.chars().count() <= MOST_FORM_NAME
        && !name.chars().any(char::is_control)
}

fn text_ok(text: &str, most: usize) -> bool {
    text.chars().count() <= most && !text.chars().any(|c| c.is_control() && c != '\n')
}

/// What is wrong with a form's body by its shape, when anything is: ids that hold and are unique,
/// tabs only at the top and alone there, at most four levels and 500 items, texts and rules within
/// their lengths, one or two columns, a check's rule and message not empty.
pub fn body_problem(body: &FormBody) -> Option<String> {
    let all = body.walk();
    if all.len() > MOST_FORM_ITEMS {
        return Some(format!(
            "Bir formda en çok {MOST_FORM_ITEMS} öğe olur; {} var.",
            all.len()
        ));
    }
    let tabs_at_top = body
        .items
        .iter()
        .filter(|i| matches!(i, FormItem::Tab { .. }))
        .count();
    if tabs_at_top > 0 && tabs_at_top != body.items.len() {
        return Some(
            "Formun üstünde sekme varsa üstte yalnız sekmeler olur; öbür öğeler bir sekmenin içine girer.".into(),
        );
    }
    let mut seen = std::collections::HashSet::new();
    for (depth, item) in &all {
        let id = item.id();
        if !form_id_ok(id) {
            return Some(format!(
                "“{id}” bir öğe kimliği olamaz: 1–64 harf, rakam, “-”, “_” ya da “.” olur."
            ));
        }
        if !seen.insert(id) {
            return Some(format!("“{id}” kimliği formda iki kez kullanılmış."));
        }
        if *depth >= MOST_FORM_DEPTH {
            return Some(format!(
                "“{id}” çok derinde: sekme ve gruplar en çok {MOST_FORM_DEPTH} düzey iç içe olur."
            ));
        }
        match item {
            FormItem::Tab { label, .. } => {
                if *depth > 0 {
                    return Some(format!("“{label}” sekmesi yalnız formun üstünde olur."));
                }
                if !form_name_ok(label) {
                    return Some(format!(
                        "“{id}” sekmesinin adı boş olamaz ve en çok {MOST_FORM_NAME} karakterdir."
                    ));
                }
            }
            FormItem::Group {
                label,
                columns,
                visible_when,
                ..
            } => {
                if label.as_deref().is_some_and(|l| !form_name_ok(l)) {
                    return Some(format!(
                        "“{id}” grubunun adı boş olamaz ve en çok {MOST_FORM_NAME} karakterdir."
                    ));
                }
                if columns.is_some_and(|c| !(1..=2).contains(&c)) {
                    return Some(format!("“{id}” grubu bir ya da iki sütun olur."));
                }
                if visible_when.as_deref().is_some_and(|r| !rule_ok(r)) {
                    return Some(rule_problem(id));
                }
            }
            FormItem::Related {
                label,
                layer,
                field,
                visible_when,
                ..
            } => {
                if label.as_deref().is_some_and(|l| !form_name_ok(l)) {
                    return Some(format!(
                        "“{id}” öğesinin adı boş olamaz ve en çok {MOST_FORM_NAME} karakterdir."
                    ));
                }
                if [layer, field]
                    .iter()
                    .any(|t| t.trim().is_empty() || t.chars().count() > MOST_FORM_NAME)
                {
                    return Some(format!(
                        "“{id}” öğesinin katmanı ve alanı yazılmalı, en çok {MOST_FORM_NAME} karakterdir."
                    ));
                }
                if visible_when.as_deref().is_some_and(|r| !rule_ok(r)) {
                    return Some(rule_problem(id));
                }
            }
            FormItem::Note { text, .. } => {
                if text.trim().is_empty() || !text_ok(text, MOST_FORM_TEXT) {
                    return Some(format!(
                        "“{id}” notu boş olamaz ve en çok {MOST_FORM_TEXT} karakterdir."
                    ));
                }
            }
            FormItem::Field(f) => {
                if f.field.trim().is_empty() || f.field.chars().count() > MOST_FORM_NAME {
                    return Some(format!("“{id}” öğesinin bağlı olduğu alan yazılmamış."));
                }
                if f.label.as_deref().is_some_and(|l| !form_name_ok(l)) {
                    return Some(format!(
                        "“{id}” alanının etiketi boş olamaz ve en çok {MOST_FORM_NAME} karakterdir."
                    ));
                }
                for t in [&f.help, &f.placeholder].into_iter().flatten() {
                    if !text_ok(t, MOST_FORM_TEXT) {
                        return Some(format!(
                            "“{id}” alanının yardımı ya da ipucu en çok {MOST_FORM_TEXT} karakterdir."
                        ));
                    }
                }
                for r in [
                    &f.visible_when,
                    &f.required_when,
                    &f.read_only_when,
                    &f.default_expr,
                    &f.calculate,
                    &f.choice_filter,
                ]
                .into_iter()
                .flatten()
                {
                    if !rule_ok(r) {
                        return Some(rule_problem(id));
                    }
                }
                if f.checks.len() > MOST_FORM_CHECKS {
                    return Some(format!(
                        "“{id}” alanında en çok {MOST_FORM_CHECKS} denetim olur."
                    ));
                }
                for c in &f.checks {
                    if !rule_ok(&c.rule)
                        || c.message.trim().is_empty()
                        || !text_ok(&c.message, MOST_FORM_TEXT)
                    {
                        return Some(format!(
                            "“{id}” alanının denetiminin kuralı ve iletisi boş olamaz; kural en çok {MOST_FORM_RULE}, ileti en çok {MOST_FORM_TEXT} karakterdir."
                        ));
                    }
                }
                if let Some(l) = &f.lookup {
                    let names = [Some(&l.layer), Some(&l.key), l.label.as_ref()]
                        .into_iter()
                        .flatten()
                        .chain(l.matches.iter().flat_map(|m| [&m.field, &m.record]));
                    if names
                        .clone()
                        .any(|t| t.trim().is_empty() || t.chars().count() > MOST_FORM_NAME)
                        || l.matches.len() > MOST_FORM_CHECKS
                        || l.filter.as_deref().is_some_and(|r| !rule_ok(r))
                    {
                        return Some(format!(
                            "“{id}” alanının listesinin katmanı, alanları ve süzgeci yazılmalı; adlar en çok {MOST_FORM_NAME}, süzgeç {MOST_FORM_RULE} karakter, en çok {MOST_FORM_CHECKS} eşleşme olur."
                        ));
                    }
                }
                if f.default
                    .as_deref()
                    .is_some_and(|d| !text_ok(d, MOST_FORM_TEXT))
                {
                    return Some(format!(
                        "“{id}” alanının varsayılanı en çok {MOST_FORM_TEXT} karakterdir."
                    ));
                }
            }
        }
    }
    None
}

fn rule_ok(rule: &str) -> bool {
    !rule.trim().is_empty() && text_ok(rule, MOST_FORM_RULE)
}

fn rule_problem(id: &str) -> String {
    format!("“{id}” öğesinin kuralı boş olamaz ve en çok {MOST_FORM_RULE} karakterdir.")
}

/// What is wrong with a project's forms by their shape, for a reader (KCAD, the server); none when
/// they hold: at most 100, ids and names that hold, unique ids, bodies that hold, releases numbered
/// 1, 2, … with the digest of their body, at most 50, `enforce` only with a release.
pub fn forms_problem(forms: &[ProjectForm]) -> Option<String> {
    if forms.len() > MOST_FORMS {
        return Some(format!(
            "Projede en çok {MOST_FORMS} form olur; {} var.",
            forms.len()
        ));
    }
    for (i, f) in forms.iter().enumerate() {
        if !form_id_ok(&f.id) {
            return Some(format!(
                "“{}” bir form kimliği olamaz: 1–64 harf, rakam, “-”, “_” ya da “.” olur.",
                f.id
            ));
        }
        if forms[..i].iter().any(|g| g.id == f.id) {
            return Some(format!("“{}” kimlikli iki form var.", f.id));
        }
        if !form_name_ok(&f.name) {
            return Some(format!(
                "“{}” formunun adı boş olamaz, en çok {MOST_FORM_NAME} karakterdir ve boşlukla başlayıp bitmez.",
                f.id
            ));
        }
        if f.layer.trim().is_empty() {
            return Some(format!("“{}” formunun katmanı yazılmamış.", f.name));
        }
        if let Some(p) = body_problem(&f.draft) {
            return Some(format!("“{}” formunun taslağı: {p}", f.name));
        }
        if f.releases.len() > MOST_FORM_RELEASES {
            return Some(format!(
                "“{}” formunun en çok {MOST_FORM_RELEASES} yayını olur.",
                f.name
            ));
        }
        for (k, r) in f.releases.iter().enumerate() {
            let expected = f.releases.first().map_or(1, |first| first.version) + k as u32;
            if r.version == 0 || r.version != expected {
                return Some(format!(
                    "“{}” formunun yayınları birer artan sürümlerle sıralanır; {} beklenirken {} var.",
                    f.name, expected, r.version
                ));
            }
            if let Some(p) = body_problem(&r.body) {
                return Some(format!("“{}” formunun {}. yayını: {p}", f.name, r.version));
            }
            if r.digest != body_digest(&r.body) {
                return Some(format!(
                    "“{}” formunun {}. yayınının özeti içeriğine uymuyor; yayın değiştirilmiş.",
                    f.name, r.version
                ));
            }
            if r.note
                .as_deref()
                .is_some_and(|n| !text_ok(n, MOST_FORM_TEXT))
            {
                return Some(format!(
                    "“{}” formunun {}. yayınının notu en çok {MOST_FORM_TEXT} karakterdir.",
                    f.name, r.version
                ));
            }
        }
        if f.enforce && f.releases.is_empty() {
            return Some(format!(
                "“{}” formu yayınlanmadan kuralları her yazmada uygulanamaz; önce yayınlayın.",
                f.name
            ));
        }
    }
    None
}

/// The forms a project keeps: those that hold, the first of an id, at most 100.
pub fn sanitized_forms(forms: Vec<ProjectForm>) -> Vec<ProjectForm> {
    let mut kept: Vec<ProjectForm> = Vec::with_capacity(forms.len().min(MOST_FORMS));
    for f in forms {
        if kept.len() == MOST_FORMS {
            break;
        }
        if forms_problem(std::slice::from_ref(&f)).is_none() && !kept.iter().any(|k| k.id == f.id) {
            kept.push(f);
        }
    }
    kept
}

/// The ids of the forms that bind the fields of `layer`.
pub fn forms_of_layer<'a>(forms: &'a [ProjectForm], layer: &str) -> Vec<&'a ProjectForm> {
    forms.iter().filter(|f| f.layer == layer).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(id: &str, key: &str) -> FormItem {
        FormItem::Field(FormField::new(id, key))
    }

    fn form(items: Vec<FormItem>) -> ProjectForm {
        ProjectForm {
            id: "bina".into(),
            name: "Bina".into(),
            layer: "yapi".into(),
            draft: FormBody {
                revision: 1,
                items,
                ..Default::default()
            },
            releases: Vec::new(),
            enforce: false,
        }
    }

    #[test]
    fn a_body_s_shape_is_checked() {
        let ok = form(vec![FormItem::Tab {
            id: "genel".into(),
            label: "Genel".into(),
            items: vec![field("durum", "Durum"), field("kat", "Kat")],
        }]);
        assert_eq!(forms_problem(std::slice::from_ref(&ok)), None);
        // A tab beside a field at the top.
        let mixed = form(vec![
            field("durum", "Durum"),
            FormItem::Tab {
                id: "t".into(),
                label: "T".into(),
                items: vec![],
            },
        ]);
        assert!(
            body_problem(&mixed.draft)
                .unwrap()
                .contains("yalnız sekmeler")
        );
        // The same id twice.
        let twice = form(vec![field("a", "A"), field("a", "B")]);
        assert!(body_problem(&twice.draft).unwrap().contains("iki kez"));
        // A tab inside a group.
        let deep = form(vec![FormItem::Group {
            id: "g".into(),
            label: None,
            columns: None,
            visible_when: None,
            collapsed: false,
            items: vec![FormItem::Tab {
                id: "t".into(),
                label: "T".into(),
                items: vec![],
            }],
        }]);
        assert!(
            body_problem(&deep.draft)
                .unwrap()
                .contains("yalnız formun üstünde")
        );
    }

    #[test]
    fn a_release_keeps_its_digest_and_its_number() {
        let mut f = form(vec![field("durum", "Durum")]);
        f.releases.push(FormRelease {
            version: 1,
            digest: body_digest(&f.draft),
            body: f.draft.clone(),
            note: None,
        });
        f.enforce = true;
        assert_eq!(forms_problem(std::slice::from_ref(&f)), None);
        // A changed body breaks its digest.
        let mut tampered = f.clone();
        tampered.releases[0].body.revision = 2;
        assert!(forms_problem(&[tampered]).unwrap().contains("özeti"));
        // Enforcing needs a release.
        let mut early = f.clone();
        early.releases.clear();
        assert!(forms_problem(&[early]).unwrap().contains("yayınlanmadan"));
        assert_eq!(f.bound_keys(), ["Durum"]);
    }

    #[test]
    fn the_json_is_the_documented_one() {
        let f = form(vec![FormItem::Group {
            id: "g".into(),
            label: Some("Genel".into()),
            columns: Some(2),
            visible_when: None,
            collapsed: false,
            items: vec![FormItem::Field(FormField {
                required_when: Some("Durum = 'review'".into()),
                ..FormField::new("not", "Not")
            })],
        }]);
        let json = serde_json::to_value(&f).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "id": "bina", "name": "Bina", "layer": "yapi",
                "draft": {"revision": 1, "items": [{
                    "type": "group", "id": "g", "label": "Genel", "columns": 2,
                    "items": [{"type": "field", "id": "not", "field": "Not", "requiredWhen": "Durum = 'review'"}]
                }]}
            })
        );
        let back: ProjectForm = serde_json::from_value(json).unwrap();
        assert_eq!(back, f);
    }
}

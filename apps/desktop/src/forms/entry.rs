//! Formla düzenle (docs/adr/0401 §6, §9): one object's record through its layer's form.
//!
//! ```text
//! ┌ Formla düzenle · Bina denetimi ● ───────────────────────────┐
//! │ Kapalı alan · Bina                                           │
//! │ [Genel] [Ayrıntı ①]                                          │
//! │   Durum *      [Ek inceleme gerekli          ▾]              │
//! │   Kat sayısı   [3                             ]              │
//! │                Zemin kat dahil.                              │
//! │   Açıklama *   [                              ]              │
//! │                ⚠ “Açıklama” zorunlu; doldurun.               │
//! │ 1 sorun · İlk soruna git              [Vazgeç] [Kaydet]      │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! The form's latest release is used (its draft while it is not published). As the user types,
//! the forms core answers which items show, which are required and locked, and what is wrong; an
//! issue shows under its field once the field was touched or Kaydet was pressed. Kaydet writes
//! the values that changed through `cad.forms.submit` (one undo step) and closes the window; when
//! the record does not go in, the window stays on the first issue. A record without any of the
//! form's fields is a new one: the form's defaults fill it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use iced::widget::{Column, Row, container, row, scrollable, space, text_editor, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{
    FormBody, FormIssue, FormItem, FormItemState, FormSeverity, FormWidget, FormsSubmit,
    LayerField, LayerFieldKind,
};
use kentos_domain::Slot;
use kentos_forms::{Attrs, Object, Prepared};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, forms_submit};
use kentos_ui::attribute::Date;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::form::Form;
use kentos_ui::widget::radio::RadioGroup;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tabs::{Tab, Tabs};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{DatePicker, Dialog as Frame, Segmented, Switch, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Formla düzenle";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const FIRST_ISSUE: &str = "İlk soruna git";

/// The window.
#[derive(Debug)]
pub struct Entry {
    form: String,
    name: String,
    /// The form's layer.
    layer: String,
    draft: bool,
    uid: String,
    prepared: Prepared,
    body: FormBody,
    fields: Vec<LayerField>,
    object: Object,
    before: Attrs,
    /// A record without any of the form's fields: the defaults fill it.
    new_record: bool,
    /// The values typed, by the fields' keys.
    values: BTreeMap<String, String>,
    /// A long text's editor, by the field's key.
    notes: BTreeMap<String, text_editor::Content>,
    tab: usize,
    tried: bool,
    touched: BTreeSet<String>,
    after: Attrs,
    items: Vec<FormItemState>,
    issues: Vec<FormIssue>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Value(String, String),
    Note(String, text_editor::Action),
    Tab(usize),
    FirstIssue,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::FormEntry(event)
}

/// The id of a field's text input, for the keyboard to go to it.
fn input_id(key: &str) -> iced::widget::Id {
    iced::widget::Id::from(format!("form-entry-{key}"))
}

/// A choice of a segmented control: its place and words.
#[derive(Clone, Copy, PartialEq)]
struct Opt<'a>(usize, &'a str);

impl fmt::Display for Opt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.1)
    }
}

impl Entry {
    /// The values typed, as a write gives them.
    fn written(&self) -> Vec<(String, String)> {
        self.values
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// The record as the form sees it now.
    fn recompute(&mut self) {
        let start = if self.new_record {
            self.prepared.with_defaults(&self.before)
        } else {
            self.before.clone()
        };
        let (after, issues) = self.prepared.assess(&start, &self.written(), &self.object);
        self.items = self.prepared.states(&after, &self.object);
        self.after = after;
        self.issues = issues;
    }

    fn state(&self, id: &str) -> Option<&FormItemState> {
        self.items.iter().find(|s| s.id == id)
    }

    fn dirty(&self) -> bool {
        self.after != self.before
    }

    /// The issue shown under a field: once touched, or after Kaydet.
    fn issue_of(&self, key: &str) -> Option<&FormIssue> {
        (self.tried || self.touched.contains(key)).then_some(())?;
        self.issues.iter().find(|i| i.field.as_deref() == Some(key))
    }

    /// The tab holding the first issue's item.
    fn tab_of(&self, item: &str) -> Option<usize> {
        self.body
            .items
            .iter()
            .position(|t| matches!(t, FormItem::Tab { .. }) && kentos_forms_find(t.items(), item))
    }
}

fn kentos_forms_find(items: &[FormItem], id: &str) -> bool {
    crate::forms::designer::find(items, id).is_some()
}

impl App {
    /// Formla düzenle on `slot` through the form `form`.
    pub(crate) fn open_entry(&mut self, slot: Slot, form: &str) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(f) = self.project_forms().iter().find(|f| f.id == form).cloned() else {
            return Task::none();
        };
        let Some(e) = doc.model.get(slot).cloned() else {
            return Task::none();
        };
        let Some(uid) = doc.model.uid(slot).map(|u| u.to_string()) else {
            return Task::none();
        };
        let fields = self.layer_fields_of(&f.layer);
        let (body, draft) = match f.releases.last() {
            Some(r) => (r.body.clone(), false),
            None => (f.draft.clone(), true),
        };
        let variables = self.form_variables();
        let prepared = match Prepared::new(&body, &fields, &variables) {
            Ok(p) => p,
            Err(issues) => {
                let first = issues
                    .iter()
                    .find(|i| i.severity == FormSeverity::Error)
                    .map_or_else(String::new, |i| i.message.clone());
                self.say(
                    Level::Warn,
                    format!(
                        "“{}” formu kullanılamıyor: {first} Formu Formlar penceresinde düzeltin.",
                        f.name
                    ),
                );
                return Task::none();
            }
        };
        let object = Object {
            kind: kentos_native_application::layer_filter::kind_label(&e).to_owned(),
            layer: self.form_layer_name(&f.layer),
            id: f64::from(slot.0),
            label: e.base().label.clone(),
            measured: Default::default(),
        };
        let before = e.base().attrs.clone();
        let new_record = !prepared.keys().iter().any(|k| {
            fields
                .iter()
                .find(|f| f.key() == *k)
                .is_some_and(|f| before.contains_key(&f.name))
        });
        let mut notes = BTreeMap::new();
        for (key, name) in fields.iter().map(|f| (f.key().to_owned(), f.name.clone())) {
            notes.insert(
                key,
                text_editor::Content::with_text(before.get(&name).map_or("", String::as_str)),
            );
        }
        let mut entry = Entry {
            form: f.id.clone(),
            name: f.name.clone(),
            layer: f.layer.clone(),
            draft,
            uid,
            prepared,
            body,
            fields,
            object,
            before,
            new_record,
            values: BTreeMap::new(),
            notes,
            tab: 0,
            tried: false,
            touched: BTreeSet::new(),
            after: Attrs::new(),
            items: Vec::new(),
            issues: Vec::new(),
        };
        entry.recompute();
        // The editors of the long texts start with the values the record shows.
        for (key, content) in entry.notes.iter_mut() {
            if let Some(f) = entry.fields.iter().find(|f| f.key() == key) {
                *content = text_editor::Content::with_text(
                    entry.after.get(&f.name).map_or("", String::as_str),
                );
            }
        }
        self.form_entry = Some(entry);
        self.dialog = Some(Dialog::FormEntry);
        Task::none()
    }

    /// The `@` values a form's rules read in the open drawing (as the commands give them).
    fn form_variables(&self) -> Vec<kentos_expression::Variable> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut all = kentos_project::variables::expression_variables(
            doc.model.name(),
            doc.model.settings(),
            0.0,
            "",
        );
        all.retain(|v| !matches!(v.name.as_str(), "tarih" | "simdi" | "kullanici"));
        all
    }

    pub(crate) fn form_entry_event(&mut self, event: Event) -> Task<Message> {
        let Some(e) = self.form_entry.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Value(key, text) => {
                e.touched.insert(key.clone());
                e.values.insert(key, text);
                e.recompute();
            }
            Event::Note(key, action) => {
                let edit = action.is_edit();
                if let Some(content) = e.notes.get_mut(&key) {
                    content.perform(action);
                    if edit {
                        let text = content.text();
                        let text = text.strip_suffix('\n').unwrap_or(&text).to_owned();
                        e.touched.insert(key.clone());
                        e.values.insert(key, text);
                        e.recompute();
                    }
                }
            }
            Event::Tab(i) => e.tab = i,
            Event::FirstIssue => return self.go_to_first_issue(),
            Event::Cancel => {
                self.form_entry = None;
                self.dialog = None;
            }
            Event::Save => return self.save_entry(),
        }
        Task::none()
    }

    fn go_to_first_issue(&mut self) -> Task<Message> {
        let Some(e) = self.form_entry.as_mut() else {
            return Task::none();
        };
        e.tried = true;
        let Some(first) = e.issues.first().cloned() else {
            return Task::none();
        };
        if let Some(t) = first.item.as_deref().and_then(|i| e.tab_of(i)) {
            e.tab = t;
        }
        match first.field {
            Some(key) => iced::widget::operation::focus(input_id(&key)),
            None => Task::none(),
        }
    }

    /// Kaydet: the record through the form; the window closes when it went in.
    fn save_entry(&mut self) -> Task<Message> {
        let Some(e) = self.form_entry.as_ref() else {
            return Task::none();
        };
        if !e.issues.is_empty() {
            self.say(
                Level::Warn,
                format!("Kayıt formun kurallarına uymuyor: {}", e.issues[0].message),
            );
            return self.go_to_first_issue();
        }
        // Only what changed goes in; a new record's defaults with it.
        let values: BTreeMap<String, String> = e
            .values
            .iter()
            .filter(|(k, _)| {
                let name = e
                    .fields
                    .iter()
                    .find(|f| f.key() == k.as_str())
                    .map(|f| f.name.clone());
                name.is_some_and(|n| e.after.get(&n) != e.before.get(&n))
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let input = FormsSubmit {
            form: e.form.clone(),
            draft: e.draft,
            uid: e.uid.clone(),
            values,
            defaults: e.new_record,
            expected_revision: None,
        };
        let name = e.name.clone();
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = forms_submit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match self.answered(result) {
            Some(done) => {
                self.form_entry = None;
                self.dialog = None;
                let said = if done.changed {
                    format!("Kayıt “{name}” formuyla yazıldı.")
                } else {
                    "Kayıtta değişiklik yok.".to_owned()
                };
                self.say(Level::Success, said);
                Task::none()
            }
            None => self.go_to_first_issue(),
        }
    }

    pub(crate) fn form_entry_view(&self) -> Element<'_, Message> {
        let Some(e) = &self.form_entry else {
            return iced::widget::text("").into();
        };
        let locked = self.forms_locked().is_some()
            || self
                .document
                .as_ref()
                .is_some_and(|d| d.model.layers().is_locked(&e.object_layer_id()));
        let mut body = Column::new().spacing(12);
        let mut head = format!("{} · {}", e.object.kind, e.object.layer);
        if e.draft {
            head.push_str(" · form yayınlanmadı, taslağıyla");
        }
        body = body.push(label::caption(head).style(muted));
        if locked {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                "Katman kilitli ya da yetkiniz yok: kayıt yalnız görüntülenir.",
            )]));
        }
        let tabbed = matches!(e.body.items.first(), Some(FormItem::Tab { .. }));
        if tabbed {
            let tabs: Vec<Tab<'_>> = e
                .body
                .items
                .iter()
                .filter(|t| e.state(t.id()).is_some_and(|s| s.shown))
                .map(|t| {
                    let label = match t {
                        FormItem::Tab { label, .. } => label.clone(),
                        _ => String::new(),
                    };
                    let count = if e.tried {
                        e.issues
                            .iter()
                            .filter(|i| {
                                i.item
                                    .as_deref()
                                    .is_some_and(|id| kentos_forms_find(t.items(), id))
                            })
                            .count()
                    } else {
                        0
                    };
                    Tab::new(label).badge(count).closable(false)
                })
                .collect();
            body = body.push(Tabs::new(tabs, e.tab, |i| msg(Event::Tab(i))));
            let shown: Vec<&FormItem> = e
                .body
                .items
                .iter()
                .filter(|t| e.state(t.id()).is_some_and(|s| s.shown))
                .collect();
            if let Some(tab) = shown.get(e.tab.min(shown.len().saturating_sub(1))) {
                body = body.push(
                    scrollable(container(self.entry_block(e, tab.items(), locked)).padding([4, 8]))
                        .height(typography::scaled(380.0)),
                );
            }
        } else {
            body = body.push(
                scrollable(container(self.entry_block(e, &e.body.items, locked)).padding([4, 8]))
                    .height(typography::scaled(380.0)),
            );
        }
        let count = if e.tried { e.issues.len() } else { 0 };
        let mut footer = Row::new().spacing(8).align_y(Center);
        if count > 0 {
            footer = footer
                .push(words::text_line(Kind::Warn, format!("{count} sorun")))
                .push(words::ghost(FIRST_ISSUE, Some(msg(Event::FirstIssue))));
        }
        footer = footer
            .push(space::horizontal().width(Fill))
            .push(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .push(words::primary(SAVE, (!locked).then(|| msg(Event::Save))));
        body = body.push(footer);
        let title = if e.dirty() {
            format!("{TITLE} · {} ●", e.name)
        } else {
            format!("{TITLE} · {}", e.name)
        };
        overlay::modal(
            Frame::new(title).push(body).width(620.0),
            msg(Event::Cancel),
        )
    }

    /// The items of a tab or of the top: fields and notes in a form, a group under its title, two
    /// columns side by side.
    fn entry_block<'a>(
        &'a self,
        e: &'a Entry,
        items: &'a [FormItem],
        locked: bool,
    ) -> Element<'a, Message> {
        let mut out = Column::new().spacing(14);
        let mut run: Vec<&FormItem> = Vec::new();
        let flush = |run: &mut Vec<&'a FormItem>, out: Column<'a, Message>| {
            if run.is_empty() {
                return out;
            }
            let form = self.entry_form(e, run, locked);
            run.clear();
            out.push(form)
        };
        for item in items {
            if !e.state(item.id()).is_some_and(|s| s.shown) {
                continue;
            }
            match item {
                FormItem::Group {
                    label: title,
                    columns,
                    items: inner,
                    ..
                } => {
                    out = flush(&mut run, out);
                    let shown: Vec<&FormItem> = inner
                        .iter()
                        .filter(|i| e.state(i.id()).is_some_and(|s| s.shown))
                        .collect();
                    let content: Element<'a, Message> = if *columns == Some(2) && shown.len() > 1 {
                        let half = shown.len().div_ceil(2);
                        row![
                            container(self.entry_form_in(e, &shown[..half], locked, 80.0))
                                .width(Fill),
                            container(self.entry_form_in(e, &shown[half..], locked, 80.0))
                                .width(Fill),
                        ]
                        .spacing(16)
                        .into()
                    } else {
                        self.entry_form(e, &shown, locked)
                    };
                    let mut group = Column::new().spacing(8);
                    if let Some(t) = title {
                        group = group.push(label::strong(t.clone()));
                    }
                    out = out.push(
                        container(group.push(content))
                            .padding(10)
                            .style(style::container::field_box)
                            .width(Fill),
                    );
                }
                FormItem::Note { text, .. } => {
                    out = flush(&mut run, out);
                    out = out.push(
                        container(
                            iced::widget::text(text.clone())
                                .size(typography::caption())
                                .style(muted)
                                .width(Fill),
                        )
                        .width(Fill),
                    );
                }
                _ => run.push(item),
            }
        }
        flush(&mut run, out).into()
    }

    /// Fields and notes in one form; a nested group is shown by [`Self::entry_block`].
    fn entry_form<'a>(
        &'a self,
        e: &'a Entry,
        items: &[&'a FormItem],
        locked: bool,
    ) -> Element<'a, Message> {
        self.entry_form_in(e, items, locked, 130.0)
    }

    /// [`Self::entry_form`] with its label column `width` wide (narrower in two columns).
    fn entry_form_in<'a>(
        &'a self,
        e: &'a Entry,
        items: &[&'a FormItem],
        locked: bool,
        width: f32,
    ) -> Element<'a, Message> {
        let mut form = Form::new().label_width(width);
        for item in items {
            match item {
                FormItem::Field(f) => {
                    let Some(field) = e.fields.iter().find(|l| l.key() == f.field) else {
                        continue;
                    };
                    let state = e.state(&f.id);
                    let read_only = locked || state.is_some_and(|s| s.read_only);
                    let words = f.label.clone().unwrap_or_else(|| field.label().to_owned());
                    let control = entry_control(e, f, field, read_only);
                    form = form.field(words, control);
                    if state.is_some_and(|s| s.required) {
                        form = form.required();
                    }
                    match e.issue_of(field.key()) {
                        Some(issue) => form = form.error(Some(issue.message.clone())),
                        None => {
                            if let Some(help) = &f.help {
                                form = form.help(help.clone());
                            }
                        }
                    }
                }
                FormItem::Note { text, .. } => {
                    form =
                        form.row(container(label::caption(text.clone()).style(muted)).width(Fill));
                }
                FormItem::Group { .. } => {
                    form = form.row(self.entry_block(e, std::slice::from_ref(*item), locked));
                }
                FormItem::Tab { .. } => {}
            }
        }
        form.into()
    }

    /// The window's controls by their words (a trace's `dialog` step): a field by its label.
    pub(crate) fn form_entry_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(e) = &self.form_entry else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(FIRST_ISSUE) => Some(msg(Event::FirstIssue)),
            Control::Fill(words, text) => {
                let key = e
                    .fields
                    .iter()
                    .find(|f| f.label() == words || f.name == words)
                    .map(|f| f.key().to_owned())
                    .ok_or_else(|| format!("“{TITLE}” penceresinde “{words}” alanı yok"))?;
                Some(msg(Event::Value(key, text.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

impl Entry {
    fn object_layer_id(&self) -> String {
        self.layer.clone()
    }
}

/// A field's control by its widget; read-only shows the value without an edit.
fn entry_control<'a>(
    e: &'a Entry,
    item: &'a kentos_contracts::FormField,
    field: &'a LayerField,
    read_only: bool,
) -> Element<'a, Message> {
    let key = field.key().to_owned();
    let value: String = e
        .values
        .get(&key)
        .cloned()
        .or_else(|| e.after.get(&field.name).cloned())
        .unwrap_or_default();
    let set = {
        let key = key.clone();
        move |v: String| msg(Event::Value(key.clone(), v))
    };
    let placeholder = item.placeholder.clone().unwrap_or_default();
    let choices: Vec<(String, String)> = match &field.values {
        Some(list) if !list.is_empty() => list
            .iter()
            .map(|c| (c.code.clone(), c.label.clone()))
            .collect(),
        _ if field.kind == LayerFieldKind::Boolean => vec![
            ("true".into(), "Evet".into()),
            ("false".into(), "Hayır".into()),
        ],
        _ => Vec::new(),
    };
    // The value's code: as typed, a label's code, or the record's.
    let code = kentos_contracts::check_value(
        &LayerField {
            required: false,
            ..field.clone()
        },
        &value,
    )
    .unwrap_or_else(|_| value.clone());
    let at = choices.iter().position(|(c, _)| *c == code);
    match kentos_forms::widget_of(item, field) {
        FormWidget::Check | FormWidget::Switch if read_only => label::body(if code == "true" {
            "Evet"
        } else if code == "false" {
            "Hayır"
        } else {
            "—"
        })
        .into(),
        FormWidget::Check => {
            let on = code == "true";
            let set = set.clone();
            check_box(
                if on { Check::Checked } else { Check::Unchecked },
                Some(set(if on { "false" } else { "true" }.to_owned())),
            )
        }
        FormWidget::Switch => {
            let set = set.clone();
            Switch::new(code == "true", move |on| {
                set(if on { "true" } else { "false" }.to_owned())
            })
            .into()
        }
        FormWidget::List if !read_only => {
            let picks = choices.clone();
            let set_pick = set.clone();
            let mut select = Select::new(
                choices.iter().map(|(_, l)| Choice::new(l.clone())),
                at,
                move |i| set_pick(picks[i].0.clone()),
            );
            if !field.required && !item.required {
                select = select.clear(set(String::new()));
            }
            select
                .placeholder(if placeholder.is_empty() {
                    "Seçin".to_owned()
                } else {
                    placeholder
                })
                .into()
        }
        FormWidget::Radio if !read_only => {
            let picks = choices.clone();
            let mut group =
                RadioGroup::new(at, move |i: usize| set(picks[i].0.clone())).horizontal();
            for (i, (_, l)) in choices.iter().enumerate() {
                group = group.option(i, l.clone(), String::new());
            }
            group.into()
        }
        FormWidget::Segmented if !read_only => {
            let opts: Vec<Opt<'a>> = field
                .values
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| {
                    v.iter()
                        .enumerate()
                        .map(|(i, c)| Opt(i, c.label.as_str()))
                        .collect()
                })
                .unwrap_or_else(|| vec![Opt(0, "Evet"), Opt(1, "Hayır")]);
            let picks = choices.clone();
            Segmented::optional(opts.clone(), at.map(|i| opts[i]), move |o: Opt<'_>| {
                set(picks[o.0].0.clone())
            })
            .into()
        }
        FormWidget::Date if !read_only => {
            let date = parse_date(&code);
            let set_date = set.clone();
            DatePicker::date(date, move |d: Option<Date>| {
                set_date(d.map_or_else(String::new, |d| {
                    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
                }))
            })
            .anchor(focus_ring(
                text_input("YYYY-AA-GG", &value)
                    .size(typography::body())
                    .id(input_id(&key))
                    .on_input(set)
                    .padding([5, 8])
                    .style(style::field::input),
            ))
            .clearable(!field.required)
            .into()
        }
        FormWidget::TextArea if !read_only => {
            let k = key.clone();
            match e.notes.get(&key) {
                Some(content) => text_editor(content)
                    .id(input_id(&key))
                    .size(typography::body())
                    .on_action(move |a| msg(Event::Note(k.clone(), a)))
                    .placeholder(placeholder)
                    .height(typography::scaled(72.0))
                    .padding([5, 8])
                    .into(),
                None => space::horizontal().into(),
            }
        }
        _ => {
            // Shown as the field displays it when read-only; typed as is otherwise.
            let shown = if read_only {
                at.map_or_else(|| value.clone(), |i| choices[i].1.clone())
            } else {
                value.clone()
            };
            let input = text_input(&placeholder, &shown)
                .size(typography::body())
                .id(input_id(&key))
                .padding([5, 8])
                .style(style::field::input);
            if read_only {
                input.into()
            } else {
                focus_ring(input.on_input(set).on_submit(msg(Event::Save))).into()
            }
        }
    }
}

fn parse_date(text: &str) -> Option<Date> {
    let mut parts = text.split('-');
    let y = parts.next()?.parse().ok()?;
    let m = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    Date::new(y, m, d)
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}

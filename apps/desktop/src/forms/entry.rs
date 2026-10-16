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
//! the forms core answers which items show, which are required and locked, what a choice filter
//! offers, what a note says and what is wrong, as `cad.forms.validate` would (docs/adr/0402 §7):
//! the record is completed with the form's calculations, which show locked; a warning shows yellow
//! and does not stop Kaydet. An issue shows under its field once the field was touched or Kaydet
//! was pressed. Kaydet writes the values that changed through `cad.forms.submit` (one undo step)
//! with this device's moment and the signed-in person, and closes the window; when the record does
//! not go in, the window stays on the first error. A record without any of the form's fields is a
//! new one: the form's defaults fill it and its "reuse last" fields start with the session's last
//! value. A collapsed group opens and closes by its title.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use iced::widget::{Column, Row, container, row, scrollable, space, text_editor, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{
    FormBody, FormIssue, FormItem, FormItemState, FormSeverity, FormWidget, FormsSubmit,
    LayerField, LayerFieldKind,
};
use kentos_domain::{Document, Slot};
use kentos_forms::Attrs;
use kentos_interaction::Level;
use kentos_native_application::forms::{RecordScreen, record_screen, unique_on_layer};
use kentos_native_application::{ExecutionContext, forms_submit};
use kentos_ui::attribute::Date;
use kentos_ui::icon::{Icon, icon};
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
/// The scrolled items' room: clear of the scrollbar on the right.
const SCROLLED: iced::Padding = iced::Padding {
    top: 4.0,
    right: 16.0,
    bottom: 4.0,
    left: 8.0,
};

/// What the session remembers of the forms (docs/adr/0402 §7): the last values of the "reuse last"
/// fields by form, and the moment a test fixes.
#[derive(Debug, Default)]
pub struct Memory {
    /// A form's id to its fields' keys and their last saved values.
    last: BTreeMap<String, BTreeMap<String, String>>,
    /// The moment the rules read instead of the clock (a test's).
    pub(crate) clock: Option<String>,
    /// The person the rules read instead of the signed-in one (a test's).
    pub(crate) person: Option<String>,
}

/// The window.
pub struct Entry {
    form: String,
    name: String,
    draft: bool,
    uid: String,
    screen: RecordScreen,
    body: FormBody,
    fields: Vec<LayerField>,
    before: Attrs,
    /// The moment and the person the rules read (`@tarih`, `@simdi`, `@kullanici`).
    moment: Option<String>,
    user: Option<String>,
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
    /// The groups closed now (a collapsed group starts closed).
    closed: BTreeSet<String>,
    /// The unique fields' values last asked, and what the layer said.
    unique_asked: Vec<Option<String>>,
    unique: Vec<FormIssue>,
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("form", &self.form)
            .field("uid", &self.uid)
            .field("after", &self.after)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Value(String, String),
    Note(String, text_editor::Action),
    Tab(usize),
    /// A collapsed group's title: open or close it.
    Group(String),
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

    /// The record as the form sees it now: the values typed, a new record's defaults, the
    /// calculations; the unique fields asked of the layer only when their values changed.
    fn recompute(&mut self, doc: &Document) {
        let prepared = &self.screen.prepared;
        let (after, mut issues) = prepared.assess(
            &self.before,
            &self.written(),
            &self.screen.object,
            self.new_record,
        );
        self.items = prepared.described(&after, &self.screen.object);
        let asked: Vec<Option<String>> = prepared
            .unique_keys()
            .iter()
            .map(|k| self.name_of(k).and_then(|n| after.get(n).cloned()))
            .collect();
        if asked != self.unique_asked || self.unique_asked.is_empty() && !asked.is_empty() {
            self.unique = unique_on_layer(doc, &self.screen, &after);
            self.unique_asked = asked;
        }
        // A field already said is not said again.
        for u in &self.unique {
            if !issues.iter().any(|i| i.field == u.field) {
                issues.push(u.clone());
            }
        }
        self.after = after;
        self.issues = issues;
    }

    /// A field's name by its key.
    fn name_of(&self, key: &str) -> Option<&String> {
        self.fields.iter().find(|f| f.key() == key).map(|f| &f.name)
    }

    fn errors(&self) -> impl Iterator<Item = &FormIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == FormSeverity::Error)
    }

    /// A field's value in the record as the form completes it now (by the field's name).
    #[cfg(test)]
    pub(crate) fn value_of(&self, name: &str) -> Option<String> {
        self.after.get(name).cloned()
    }

    /// What an item's choice filter offers now.
    #[cfg(test)]
    pub(crate) fn offered(&self, item: &str) -> Option<Vec<String>> {
        self.state(item).and_then(|s| s.choices.clone())
    }

    /// Whether a group is closed now.
    #[cfg(test)]
    pub(crate) fn is_closed(&self, group: &str) -> bool {
        self.closed.contains(group)
    }

    fn state(&self, id: &str) -> Option<&FormItemState> {
        self.items.iter().find(|s| s.id == id)
    }

    fn dirty(&self) -> bool {
        self.after != self.before
    }

    /// The issue shown under a field: once touched, or after Kaydet; an error before a warning.
    fn issue_of(&self, key: &str) -> Option<&FormIssue> {
        (self.tried || self.touched.contains(key)).then_some(())?;
        let mine = |i: &&FormIssue| i.field.as_deref() == Some(key);
        self.errors()
            .find(mine)
            .or_else(|| self.issues.iter().find(mine))
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
        let Some(uid) = doc.model.uid(slot).map(|u| u.to_string()) else {
            return Task::none();
        };
        let fields = self.layer_fields_of(&f.layer);
        let (body, draft) = match f.releases.last() {
            Some(r) => (r.body.clone(), false),
            None => (f.draft.clone(), true),
        };
        let (moment, user) = self.form_moment();
        let screen = match record_screen(
            &doc.model,
            &f.id,
            &uid,
            draft,
            moment.as_deref(),
            user.as_deref(),
        ) {
            Ok(s) => s,
            Err(error) => {
                self.say(Level::Warn, error.message);
                return Task::none();
            }
        };
        let before = screen.attrs.clone();
        let new_record = !screen.prepared.keys().iter().any(|k| {
            fields
                .iter()
                .find(|f| f.key() == *k)
                .is_some_and(|f| before.contains_key(&f.name))
        });
        // A new record's "reuse last" fields start with the session's last values, as if typed.
        let mut values = BTreeMap::new();
        if new_record && let Some(last) = self.form_memory.last.get(&f.id) {
            for key in screen.prepared.reuse_keys() {
                if let Some(v) = last.get(key) {
                    values.insert(key.to_owned(), v.clone());
                }
            }
        }
        let closed = collapsed_groups(&body.items);
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
            draft,
            uid,
            screen,
            body,
            fields,
            before,
            moment,
            user,
            new_record,
            values,
            notes,
            tab: 0,
            tried: false,
            touched: BTreeSet::new(),
            after: Attrs::new(),
            items: Vec::new(),
            issues: Vec::new(),
            closed,
            unique_asked: Vec::new(),
            unique: Vec::new(),
        };
        entry.recompute(&doc.model);
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

    /// The moment and the person a form's rules read (docs/adr/0402 §5): this device's clock in its
    /// zone to the second (a test's fixed moment instead) and the signed-in person, if any.
    fn form_moment(&self) -> (Option<String>, Option<String>) {
        let moment = self.form_memory.clock.clone().unwrap_or_else(|| {
            let utc = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64);
            let offset = crate::cloud::local_time::Zone::system().offset_at(utc);
            kentos_geometry_core::time::write(((utc + i64::from(offset)) * 1000) as f64, false)
        });
        let user = self
            .form_memory
            .person
            .clone()
            .or_else(|| {
                self.cloud
                    .me
                    .as_ref()
                    .map(|m| m.user.display_name.trim().to_owned())
            })
            .filter(|n| !n.is_empty());
        (Some(moment), user)
    }

    pub(crate) fn form_entry_event(&mut self, event: Event) -> Task<Message> {
        let (Some(e), Some(doc)) = (self.form_entry.as_mut(), self.document.as_ref()) else {
            return Task::none();
        };
        match event {
            Event::Value(key, text) => {
                // A long text's editor shows a value given as a whole (a trace's, a pick's) too.
                if let Some(content) = e.notes.get_mut(&key)
                    && content.text().trim_end_matches('\n') != text
                {
                    *content = text_editor::Content::with_text(&text);
                }
                e.touched.insert(key.clone());
                e.values.insert(key, text);
                e.recompute(&doc.model);
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
                        e.recompute(&doc.model);
                    }
                }
            }
            Event::Tab(i) => e.tab = i,
            Event::Group(id) => {
                if !e.closed.remove(&id) {
                    e.closed.insert(id);
                }
            }
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
        let Some(first) = e.errors().next().or(e.issues.first()).cloned() else {
            return Task::none();
        };
        // A closed group holding it opens.
        if let Some(item) = first.item.as_deref() {
            let holding: Vec<String> = e
                .closed
                .iter()
                .filter(|g| {
                    crate::forms::designer::find(&e.body.items, g)
                        .is_some_and(|group| kentos_forms_find(group.items(), item))
                })
                .cloned()
                .collect();
            for g in holding {
                e.closed.remove(&g);
            }
        }
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
        let first = e.errors().next().map(|i| i.message.clone());
        if let Some(first) = first {
            self.say(
                Level::Warn,
                format!("Kayıt formun kurallarına uymuyor: {first}"),
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
            moment: e.moment.clone(),
            user: e.user.clone(),
            expected_revision: None,
        };
        let name = e.name.clone();
        let form = e.form.clone();
        // The "reuse last" fields' values, kept for the next new record when this one goes in.
        let reused: Vec<(String, String)> = e
            .screen
            .prepared
            .reuse_keys()
            .iter()
            .filter_map(|k| {
                let v = e.after.get(e.name_of(k)?)?;
                Some(((*k).to_owned(), v.clone()))
            })
            .collect();
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
                if !reused.is_empty() {
                    self.form_memory
                        .last
                        .entry(form)
                        .or_default()
                        .extend(reused);
                }
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
                .is_some_and(|d| d.model.layers().is_locked(&e.screen.layer));
        let mut body = Column::new().spacing(12);
        let mut head = format!("{} · {}", e.screen.object.kind, e.screen.object.layer);
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
                    scrollable(
                        container(self.entry_block(e, tab.items(), locked)).padding(SCROLLED),
                    )
                    .height(typography::scaled(380.0)),
                );
            }
        } else {
            body = body.push(
                scrollable(container(self.entry_block(e, &e.body.items, locked)).padding(SCROLLED))
                    .height(typography::scaled(380.0)),
            );
        }
        let (errors, warnings) = if e.tried {
            (e.errors().count(), e.issues.len() - e.errors().count())
        } else {
            (0, 0)
        };
        let mut footer = Row::new().spacing(8).align_y(Center);
        if errors > 0 {
            footer = footer
                .push(words::text_line(Kind::Error, format!("{errors} sorun")))
                .push(words::ghost(FIRST_ISSUE, Some(msg(Event::FirstIssue))));
        }
        if warnings > 0 {
            footer = footer.push(words::text_line(Kind::Warn, format!("{warnings} uyarı")));
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
                    id,
                    label: title,
                    columns,
                    items: inner,
                    collapsed,
                    ..
                } => {
                    out = flush(&mut run, out);
                    let closed = e.closed.contains(id);
                    if *collapsed {
                        // Its title opens and closes it.
                        let words = title.clone().unwrap_or_else(|| "Grup".to_owned());
                        let head = iced::widget::button(
                            row![
                                icon(if closed {
                                    Icon::ChevronRight
                                } else {
                                    Icon::ChevronDown
                                })
                                .size(12.0),
                                label::strong(words),
                            ]
                            .spacing(6)
                            .align_y(Center),
                        )
                        .padding([2, 4])
                        .style(style::button::ghost)
                        .on_press(msg(Event::Group(id.clone())));
                        if closed {
                            out = out.push(
                                container(head)
                                    .padding(6)
                                    .style(style::container::field_box)
                                    .width(Fill),
                            );
                            continue;
                        }
                    }
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
                    if *collapsed {
                        group = group.push(
                            iced::widget::button(
                                row![
                                    icon(Icon::ChevronDown).size(12.0),
                                    label::strong(
                                        title.clone().unwrap_or_else(|| "Grup".to_owned())
                                    ),
                                ]
                                .spacing(6)
                                .align_y(Center),
                            )
                            .padding([2, 4])
                            .style(style::button::ghost)
                            .on_press(msg(Event::Group(id.clone()))),
                        );
                    } else if let Some(t) = title {
                        group = group.push(label::strong(t.clone()));
                    }
                    out = out.push(
                        container(group.push(content))
                            .padding(10)
                            .style(style::container::field_box)
                            .width(Fill),
                    );
                }
                FormItem::Note { id, text } => {
                    out = flush(&mut run, out);
                    let text = e
                        .state(id)
                        .and_then(|s| s.text.clone())
                        .unwrap_or_else(|| text.clone());
                    out = out.push(
                        container(
                            iced::widget::text(text)
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
                        Some(issue) if issue.severity == FormSeverity::Error => {
                            form = form.error(Some(issue.message.clone()));
                        }
                        Some(issue) => form = form.warning(Some(issue.message.clone())),
                        None => {
                            if let Some(help) = &f.help {
                                form = form.help(help.clone());
                            } else if let Some(calc) = &f.calculate {
                                form = form.help(format!("Hesaplanır: {calc}"));
                            }
                        }
                    }
                }
                FormItem::Note { id, text } => {
                    let text = e
                        .state(id)
                        .and_then(|s| s.text.clone())
                        .unwrap_or_else(|| text.clone());
                    form = form.row(container(label::caption(text).style(muted)).width(Fill));
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

/// The ids of the groups that start closed.
fn collapsed_groups(items: &[FormItem]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for item in items {
        if let FormItem::Group {
            id,
            collapsed: true,
            ..
        } = item
        {
            out.insert(id.clone());
        }
        out.extend(collapsed_groups(item.items()));
    }
    out
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
    // What the item's choice filter offers for the record; every listed value without one.
    let offered = e.state(&item.id).and_then(|s| s.choices.as_ref());
    let choices: Vec<(String, String)> = match &field.values {
        Some(list) if !list.is_empty() => list
            .iter()
            .filter(|c| offered.is_none_or(|o| o.contains(&c.code)))
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
                        .filter(|c| offered.is_none_or(|o| o.contains(&c.code)))
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

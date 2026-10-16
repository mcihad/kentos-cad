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
use kentos_contracts::FormsFill;
use kentos_contracts::{
    FormBody, FormIssue, FormItem, FormItemState, FormSeverity, FormWidget, FormsSubmit,
    LayerField, LayerFieldKind,
};
use kentos_domain::{Document, Slot};
use kentos_forms::Attrs;
use kentos_interaction::Level;
use kentos_native_application::forms::{
    RecordScreen, preview_screen, record_screen, unique_on_layer,
};
use kentos_native_application::{ExecutionContext, forms_fill, forms_submit};
use kentos_ui::attribute::{Date, DateTime, Time};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::form::Form;
use kentos_ui::widget::radio::RadioGroup;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tabs::{Tab, Tabs};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{
    DatePicker, Dialog as Frame, Segmented, Switch, TimePicker, focus_ring, overlay,
};
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
    /// The drawing's revision when the window opened: Kaydet writes only onto it (a change made
    /// meanwhile is a conflict, the window and its values stay).
    revision: String,
    /// The designer's preview (docs/adr/0403 §3): nothing is written.
    pub(crate) preview: bool,
    /// Shown in the attribute table's Form view (docs/adr/0416), not as a window: Kaydet and Vazgeç
    /// keep it open on its record.
    pub(crate) docked: bool,
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
    pub(crate) issues: Vec<FormIssue>,
    /// The groups closed now (a collapsed group starts closed).
    closed: BTreeSet<String>,
    /// The unique fields' values last asked, and what the layer said.
    unique_asked: Vec<Option<String>>,
    unique: Vec<FormIssue>,
    /// Many records at once (docs/adr/0403 §6): each with its attributes and shape; empty for one.
    members: Vec<Member>,
    /// The fields whose values differ among the records (“Çeşitli değerler”).
    mixed: BTreeSet<String>,
    /// A suggesting field item's values in the layer, the most used first (docs/adr/0407 §3).
    suggestions: BTreeMap<String, Vec<String>>,
    /// The record's place among the layer's records and their count (docs/adr/0408 §3); none in
    /// the preview and with many records.
    place: Option<(usize, usize)>,
    /// The overridable calculations (docs/adr/0417) that follow their inputs in this window: empty
    /// or holding their calculation's value when it opened, and not typed into since.
    follows: BTreeSet<String>,
}

/// A record of the many-record mode.
#[derive(Debug, Clone)]
struct Member {
    uid: String,
    attrs: Attrs,
    object: kentos_forms::Object,
}

/// What every record has the same: a name with one value in all of them; the others named apart.
fn common(all: &[&Attrs]) -> (Attrs, BTreeSet<String>) {
    let mut names: BTreeSet<&String> = BTreeSet::new();
    for a in all {
        names.extend(a.keys());
    }
    let mut same = Attrs::new();
    let mut mixed = BTreeSet::new();
    for name in names {
        let first = all.first().and_then(|a| a.get(name));
        if all.iter().all(|a| a.get(name) == first) {
            if let Some(v) = first {
                same.insert(name.clone(), v.clone());
            }
        } else {
            mixed.insert(name.clone());
        }
    }
    (same, mixed)
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
    /// A related object's form opens (docs/adr/0404 §4).
    OpenRelated(String),
    /// A related object, or the one a reference names, is shown in the drawing.
    ShowObject(String),
    /// A related item's object linked to the record, or its link to it broken (docs/adr/0405 §4):
    /// the item and the object.
    Link(String, String),
    Unlink(String, String),
    /// An attachments item's Ekle (docs/adr/0407 §1): photos only when it counts photos.
    AttachAdd(bool),
    /// An attachment of the record opened or removed, by its place in the object's list.
    AttachOpen(usize),
    AttachRemove(usize),
    /// The layer's previous (-1) or next (+1) record, by the form (docs/adr/0408 §3).
    Step(i32),
    /// A related item's Yeni: a child at the record's place, linked to it, opened by its form
    /// (docs/adr/0409 §3).
    NewRelated(String),
    FirstIssue,
    /// Uymayanları boşalt (docs/adr/0422 §2): the values the record's subtype does not offer emptied.
    ClearMisfits,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::FormEntry(event)
}

impl Entry {
    /// The record's persistent id, as text (for the flow tests).
    #[cfg(test)]
    pub(crate) fn uid_text(&self) -> String {
        self.uid.clone()
    }

    /// An event of this window: the preview's go to the designer's preview.
    fn msg(&self, event: Event) -> Message {
        if self.preview {
            Message::FormPreview(event)
        } else {
            Message::FormEntry(event)
        }
    }
}

/// The id of a field's text input, for the keyboard to go to it.
pub(crate) fn input_id(key: &str) -> iced::widget::Id {
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
    /// The fields' keys whose values the record's subtype does not offer (docs/adr/0422 §2).
    pub(crate) fn misfits(&self) -> Vec<String> {
        self.issues
            .iter()
            .filter(|i| i.code == kentos_forms::codes::VALUE_NOT_IN_SUBTYPE)
            .filter_map(|i| i.field.clone())
            .collect()
    }

    /// A field's value as the record would hold it now.
    #[cfg(test)]
    pub(crate) fn after_value(&self, name: &str) -> Option<String> {
        self.after.get(name).cloned()
    }

    /// Whether the field's item is an overridable calculation (docs/adr/0417).
    fn overridable(&self, key: &str) -> bool {
        self.body
            .fields()
            .iter()
            .any(|f| f.field == key && f.overridable && f.calculate.is_some())
    }

    /// The values typed, as a write gives them; an overridable calculation that follows its inputs
    /// is given empty, so that it is worked out again.
    fn written(&self) -> Vec<(String, String)> {
        self.values
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .chain(
                self.follows
                    .iter()
                    .filter(|k| !self.values.contains_key(*k))
                    .map(|k| (k.clone(), String::new())),
            )
            .collect()
    }

    /// The record as the form sees it now: the values typed, a new record's defaults, the
    /// calculations; the unique fields asked of the layer only when their values changed.
    fn recompute(&mut self, doc: &Document) {
        if !self.members.is_empty() {
            self.recompute_many();
            return;
        }
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

    /// Many records (docs/adr/0403 §6): each takes the values typed and is completed by itself; an
    /// item shows, is required or locked when it is so for any record, a list offers what every
    /// record's filter offers; the issues are the first record's that has an error (its warnings
    /// else). The record shown is what all of them have the same.
    fn recompute_many(&mut self) {
        let prepared = &self.screen.prepared;
        let written = self.written();
        let mut afters = Vec::with_capacity(self.members.len());
        let mut merged: Option<Vec<FormItemState>> = None;
        let mut issues: Option<Vec<FormIssue>> = None;
        for m in &self.members {
            let (after, found) = prepared.assess(&m.attrs, &written, &m.object, false);
            let states = prepared.described(&after, &m.object);
            merged = Some(match merged {
                None => states,
                Some(mut all) => {
                    for (a, s) in all.iter_mut().zip(states) {
                        a.shown |= s.shown;
                        a.required |= s.required;
                        a.read_only |= s.read_only;
                        a.choices = match (a.choices.take(), s.choices) {
                            (Some(x), Some(y)) => {
                                Some(x.into_iter().filter(|c| y.contains(c)).collect())
                            }
                            (x, y) => x.or(y),
                        };
                    }
                    all
                }
            });
            let errored = kentos_forms::has_errors(&found);
            if issues.is_none()
                || errored && !issues.as_deref().is_some_and(kentos_forms::has_errors)
            {
                issues = Some(found);
            }
            afters.push(after);
        }
        let all: Vec<&Attrs> = afters.iter().collect();
        let (same, mixed) = common(&all);
        self.items = merged.unwrap_or_default();
        self.issues = issues.unwrap_or_default();
        self.after = same;
        self.mixed = mixed;
    }

    /// A field's label (its alias, else its name) by its key.
    fn label_of(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.key() == key)
            .map(|f| f.label())
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

    /// Whether a field's values differ among the records now (the many-record mode).
    #[cfg(test)]
    pub(crate) fn is_mixed(&self, name: &str) -> bool {
        self.mixed.contains(name)
    }

    /// A related item's objects now.
    #[cfg(test)]
    pub(crate) fn related_of(&self, item: &str) -> Vec<String> {
        self.state(item)
            .and_then(|s| s.related.clone())
            .unwrap_or_default()
    }

    /// Whether a group is closed now.
    #[cfg(test)]
    pub(crate) fn is_closed(&self, group: &str) -> bool {
        self.closed.contains(group)
    }

    pub(crate) fn state(&self, id: &str) -> Option<&FormItemState> {
        self.items.iter().find(|s| s.id == id)
    }

    /// The values typed, by the fields' keys, and whether the record is new: a scenario's input
    /// (docs/adr/0420).
    pub(crate) fn typed(&self) -> (BTreeMap<String, String>, bool) {
        (self.values.clone(), self.new_record)
    }

    /// The record's slot in the open drawing.
    pub(crate) fn uid_slot(&self, app: &App) -> Option<Slot> {
        app.document
            .as_ref()
            .and_then(|d| record_slot(&d.model, &self.uid))
    }

    /// Whether a value was typed that the record does not hold yet.
    pub(crate) fn dirty_now(&self) -> bool {
        self.dirty()
    }

    fn dirty(&self) -> bool {
        if !self.members.is_empty() {
            return !self.values.is_empty();
        }
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
        let entry = self.entry_of(
            &f,
            body,
            draft,
            uid,
            screen,
            moment,
            user,
            false,
            BTreeMap::new(),
        );
        self.form_entry = Some(entry);
        self.dialog = Some(Dialog::FormEntry);
        Task::none()
    }

    /// The record's form in the attribute table's Form view (docs/adr/0416): the same editor, no
    /// window.
    pub(crate) fn open_entry_docked(&mut self, slot: Slot, form: &str) -> Task<Message> {
        let task = self.open_entry(slot, form);
        if let Some(e) = self.form_entry.as_mut() {
            e.docked = true;
            if self.dialog == Some(Dialog::FormEntry) {
                self.dialog = None;
            }
        }
        task
    }

    /// The docked editor again on its record, as the drawing has it now.
    fn redock(&mut self, uid: &str, form: &str) -> Task<Message> {
        let slot = self
            .document
            .as_ref()
            .and_then(|d| record_slot(&d.model, uid));
        match slot {
            Some(slot) => self.open_entry_docked(slot, form),
            None => {
                self.form_entry = None;
                Task::none()
            }
        }
    }

    /// Formla düzenle on many records of one layer (docs/adr/0403 §6): what they have the same shows,
    /// a value typed goes to all of them through `cad.forms.fill`.
    pub(crate) fn open_entry_many(&mut self, slots: &[Slot], form: &str) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(f) = self.project_forms().iter().find(|f| f.id == form).cloned() else {
            return Task::none();
        };
        let uids: Vec<String> = slots
            .iter()
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .collect();
        let (body, draft) = match f.releases.last() {
            Some(r) => (r.body.clone(), false),
            None => (f.draft.clone(), true),
        };
        let (moment, user) = self.form_moment();
        let mut screens = Vec::new();
        for uid in &uids {
            match record_screen(
                &doc.model,
                &f.id,
                uid,
                draft,
                moment.as_deref(),
                user.as_deref(),
            ) {
                Ok(s) => screens.push((uid.clone(), s)),
                Err(error) => {
                    self.say(Level::Warn, error.message);
                    return Task::none();
                }
            }
        }
        let members: Vec<Member> = screens
            .iter()
            .map(|(uid, s)| Member {
                uid: uid.clone(),
                attrs: s.attrs.clone(),
                object: s.object.clone(),
            })
            .collect();
        let Some((uid, screen)) = screens.into_iter().next() else {
            return Task::none();
        };
        let mut entry = self.entry_of(
            &f,
            body,
            draft,
            uid,
            screen,
            moment,
            user,
            false,
            BTreeMap::new(),
        );
        let all: Vec<&Attrs> = members.iter().map(|m| &m.attrs).collect();
        let (same, _) = common(&all);
        entry.before = same;
        entry.new_record = false;
        entry.values.clear();
        entry.members = members;
        if let Some(doc) = &self.document {
            entry.recompute(&doc.model);
        }
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

    /// The window's state for a record through `body`: a new record's defaults and last values, its
    /// groups as the form starts them, the record as the form completes it (`typed`: values kept
    /// from a preview being built again).
    #[allow(clippy::too_many_arguments)]
    fn entry_of(
        &self,
        f: &kentos_contracts::ProjectForm,
        body: FormBody,
        draft: bool,
        uid: String,
        screen: RecordScreen,
        moment: Option<String>,
        user: Option<String>,
        preview: bool,
        typed: BTreeMap<String, String>,
    ) -> Entry {
        let fields = self.layer_fields_of(&f.layer);
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
        values.extend(typed);
        let closed = collapsed_groups(&body.items);
        let mut notes = BTreeMap::new();
        for key in fields.iter().map(|f| f.key().to_owned()) {
            notes.insert(key, text_editor::Content::new());
        }
        let revision = self
            .document
            .as_ref()
            .map_or_else(String::new, |d| d.model.revision().to_string());
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
            revision,
            preview,
            docked: false,
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
            members: Vec::new(),
            mixed: BTreeSet::new(),
            suggestions: BTreeMap::new(),
            place: None,
            follows: BTreeSet::new(),
        };
        if let (Some(doc), false) = (&self.document, preview) {
            let mut at = None;
            let mut n = 0;
            for (u, _) in doc.model.by_layer_with_uids(&f.layer) {
                if u.to_string() == entry.uid {
                    at = Some(n);
                }
                n += 1;
            }
            entry.place = at.map(|i| (i, n));
        }
        if let Some(doc) = &self.document {
            entry.suggestions = kentos_native_application::forms::suggestions(
                &doc.model,
                &f.layer,
                &entry.screen.prepared,
            )
            .into_iter()
            .collect();
        }
        // An overridable calculation follows while it is empty or holds what its calculation gives.
        let overridable: Vec<(String, String)> = entry
            .body
            .fields()
            .into_iter()
            .filter(|f| f.overridable && f.calculate.is_some())
            .filter_map(|f| {
                let name = entry
                    .fields
                    .iter()
                    .find(|x| x.key() == f.field)?
                    .name
                    .clone();
                Some((f.field.clone(), name))
            })
            .collect();
        for (key, name) in overridable {
            let mut emptied = entry.before.clone();
            let held = emptied.remove(&name);
            let (worked, _) =
                entry
                    .screen
                    .prepared
                    .complete(&emptied, &entry.screen.object, entry.new_record);
            if held.is_none() || held == worked.get(&name).cloned() {
                entry.follows.insert(key);
            }
        }
        if let Some(doc) = &self.document {
            entry.recompute(&doc.model);
        }
        // The editors of the long texts start with the values the record shows.
        for (key, content) in entry.notes.iter_mut() {
            if let Some(f) = entry.fields.iter().find(|f| f.key() == key) {
                *content = text_editor::Content::with_text(
                    entry.after.get(&f.name).map_or("", String::as_str),
                );
            }
        }
        entry
    }

    /// The designer's preview of its copy (docs/adr/0403 §3) on the chosen object of the form's
    /// layer, else on a new, empty record; the values typed into the last one kept. The reason it
    /// cannot be shown otherwise.
    pub(crate) fn rebuild_preview(&mut self, body: &FormBody, form: &str) -> Result<(), String> {
        let Some(doc) = &self.document else {
            return Err("Açık çizim yok.".into());
        };
        let Some(f) = self.project_forms().iter().find(|f| f.id == form).cloned() else {
            return Err("Form yok.".into());
        };
        let uid = self
            .selection
            .ids()
            .first()
            .filter(|_| self.selection.len() == 1)
            .and_then(|slot| {
                let e = doc.model.get(*slot)?;
                (e.base().layer_id == f.layer).then(|| doc.model.uid(*slot))?
            })
            .map(|u| u.to_string());
        let (moment, user) = self.form_moment();
        let screen = preview_screen(
            &doc.model,
            &f.id,
            body,
            uid.as_deref(),
            moment.as_deref(),
            user.as_deref(),
        )
        .map_err(|e| e.message)?;
        let kept = self.form_preview.as_ref().map(|e| {
            // The collapsed groups the designer opened.
            let opened: BTreeSet<String> = collapsed_groups(&e.body.items)
                .difference(&e.closed)
                .cloned()
                .collect();
            (e.values.clone(), e.tab, e.touched.clone(), opened)
        });
        let mut entry = self.entry_of(
            &f,
            body.clone(),
            true,
            uid.unwrap_or_default(),
            screen,
            moment,
            user,
            true,
            kept.as_ref().map(|k| k.0.clone()).unwrap_or_default(),
        );
        if let Some((_, tab, touched, opened)) = kept {
            entry.tab = tab;
            entry.touched = touched;
            // A group the designer opened stays open; a new collapsed one starts closed.
            entry.closed.retain(|g| !opened.contains(g));
        }
        // The designer sees every issue the rules give, touched or not.
        entry.tried = true;
        self.form_preview = Some(entry);
        Ok(())
    }

    /// The moment and the person a form's rules read (docs/adr/0402 §5): this device's clock in its
    /// zone to the second (a test's fixed moment instead) and the signed-in person, if any.
    pub(crate) fn form_moment(&self) -> (Option<String>, Option<String>) {
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
        self.entry_event(false, event)
    }

    /// An event of the designer's preview: typing, tabs and groups; nothing is written.
    pub(crate) fn form_preview_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Save | Event::Cancel | Event::FirstIssue => Task::none(),
            event => self.entry_event(true, event),
        }
    }

    /// A related item's object linked to the record (its field takes the record's id) or unlinked (its
    /// field emptied), through `cad.entities.set` (the object's own form's rules hold); the window's
    /// lists follow, its values stay.
    fn link_related(&mut self, item: &str, uid: &str, link: bool) -> Task<Message> {
        let Some(e) = self.form_entry.as_ref() else {
            return Task::none();
        };
        let Some(FormItem::Related { layer, field, .. }) =
            crate::forms::designer::find(&e.body.items, item).cloned()
        else {
            return Task::none();
        };
        let record = e.uid.clone();
        let Some((name, many)) = self
            .layer_fields_of(&layer)
            .iter()
            .find(|f| f.key() == field)
            .map(|f| (f.name.clone(), f.multiple))
        else {
            return Task::none();
        };
        // A many-to-many field keeps its other ids: the record's is added or taken off
        // (docs/adr/0409 §1).
        let held: Vec<String> = if many {
            self.document
                .as_ref()
                .and_then(|d| {
                    let slot = d.model.slot_of(kentos_domain::Uuid::parse_str(uid).ok()?)?;
                    d.model.get(slot)?.base().attrs.get(&name).cloned()
                })
                .map(|v| {
                    v.split(';')
                        .map(|h| h.trim().to_owned())
                        .filter(|h| !h.is_empty() && *h != record)
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let value = match (many, link) {
            (true, true) => Some(
                [held, vec![record]]
                    .concat()
                    .join(kentos_contracts::MULTIPLE_JOIN),
            ),
            (true, false) => (!held.is_empty()).then(|| held.join(kentos_contracts::MULTIPLE_JOIN)),
            (false, true) => Some(record),
            (false, false) => None,
        };
        let input = serde_json::json!({
            "uids": [uid],
            "attrs": { name: value },
            "operation": "attributes",
        });
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_contracts::CAD_ENTITIES_SET,
            kentos_contracts::CallOp::Execute,
            input,
        );
        let refused = match answer {
            Ok(v) if v["status"] == "completed" => None,
            Ok(v) => Some(v["error"]["message"].as_str().unwrap_or("").to_owned()),
            Err(e) => Some(format!("{e:?}")),
        };
        if let Some(why) = refused {
            self.say(Level::Warn, why);
            return Task::none();
        }
        self.say(
            Level::Success,
            if link {
                "Nesne kayda bağlandı."
            } else {
                "Nesnenin kayda bağı koparıldı."
            },
        );
        self.refresh_entry()
    }

    /// Yeni of a related item (docs/adr/0409 §3): a point of the other layer at the record's place,
    /// its field holding the record's id (`cad.point.create`, the child's own form's rules too), then
    /// opened by its layer's form.
    fn new_related(&mut self, item: &str) -> Task<Message> {
        let Some(e) = self.form_entry.as_ref() else {
            return Task::none();
        };
        let Some(FormItem::Related { layer, field, .. }) =
            crate::forms::designer::find(&e.body.items, item).cloned()
        else {
            return Task::none();
        };
        let anchor = e.screen.object.measured.anchor;
        let record = e.uid.clone();
        let Some(name) = self
            .layer_fields_of(&layer)
            .iter()
            .find(|f| f.key() == field)
            .map(|f| f.name.clone())
        else {
            return Task::none();
        };
        // A record without a geometry (docs/adr/0410 §4) when the child layer holds records, or the
        // record itself has no place; else a point at the record's place.
        let (east, north) = match anchor {
            Some(at) if !self.layer_holds_records(&layer) => at,
            _ => {
                let attrs = [(name, record)].into_iter().collect();
                let Some(slot) = self.add_record(&layer, attrs) else {
                    return Task::none();
                };
                self.say(Level::Success, "Alt kayıt oluşturuldu ve kayda bağlandı.");
                if let Some(form) = self.form_of_layer(&layer) {
                    self.selection.set([slot]);
                    return self.open_entry(slot, &form);
                }
                return self.refresh_entry();
            }
        };
        let input = serde_json::json!({
            "p": { "x": east, "y": north },
            "layerId": layer,
            "attrs": { name: record },
        });
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_contracts::CAD_POINT_CREATE,
            kentos_contracts::CallOp::Execute,
            input,
        );
        let made = match answer {
            Ok(v) if v["status"] == "completed" => v["output"]["uid"]
                .as_str()
                .and_then(|u| kentos_domain::Uuid::parse_str(u).ok())
                .and_then(|u| doc.model.slot_of(u)),
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("").to_owned();
                self.say(Level::Warn, format!("Alt kayıt oluşturulamadı: {why}"));
                return Task::none();
            }
            Err(e) => {
                self.say(Level::Warn, format!("Alt kayıt oluşturulamadı: {e:?}"));
                return Task::none();
            }
        };
        let Some(slot) = made else {
            return Task::none();
        };
        self.say(Level::Success, "Alt kayıt oluşturuldu ve kayda bağlandı.");
        // The child's own form, else the record shows its new child.
        if let Some(form) = self.form_of_layer(&layer) {
            self.selection.set([slot]);
            return self.open_entry(slot, &form);
        }
        self.refresh_entry()
    }

    /// The window's record prepared again (another object changed): the values typed stay, the
    /// drawing's revision is the new one.
    pub(crate) fn refresh_entry(&mut self) -> Task<Message> {
        let Some(e) = self.form_entry.as_ref() else {
            return Task::none();
        };
        let (form, uid, draft) = (e.form.clone(), e.uid.clone(), e.draft);
        let (moment, user) = (e.moment.clone(), e.user.clone());
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Ok(screen) = record_screen(
            &doc.model,
            &form,
            &uid,
            draft,
            moment.as_deref(),
            user.as_deref(),
        ) else {
            return Task::none();
        };
        let revision = doc.model.revision().to_string();
        if let (Some(e), Some(doc)) = (self.form_entry.as_mut(), self.document.as_ref()) {
            e.screen = screen;
            e.revision = revision;
            e.unique_asked.clear();
            e.recompute(&doc.model);
        }
        Task::none()
    }

    /// A related object's form, when the record shown has nothing unsaved.
    fn open_related(&mut self, uid: &str) -> Task<Message> {
        if self.form_entry.as_ref().is_some_and(Entry::dirty) {
            self.say(
                Level::Warn,
                "Kayıtta kaydedilmemiş değişiklik var: önce Kaydet’e ya da Vazgeç’e basın.",
            );
            return Task::none();
        }
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(slot) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.model.slot_of(u))
        else {
            return Task::none();
        };
        let Some(layer) = doc.model.get(slot).map(|e| e.base().layer_id.clone()) else {
            return Task::none();
        };
        match self.form_of_layer(&layer) {
            Some(form) => {
                self.form_entry = None;
                self.selection.set([slot]);
                self.open_entry(slot, &form)
            }
            None => {
                self.say(
                    Level::Warn,
                    "İlişkili nesnenin katmanının formu yok; Formlar penceresinden bir form oluşturun.",
                );
                Task::none()
            }
        }
    }

    fn entry_event(&mut self, preview: bool, event: Event) -> Task<Message> {
        let slot = if preview {
            &mut self.form_preview
        } else {
            &mut self.form_entry
        };
        let (Some(e), Some(doc)) = (slot.as_mut(), self.document.as_ref()) else {
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
                // Typing over an overridable calculation keeps the value; emptying it follows again.
                if e.overridable(&key) {
                    if text.is_empty() {
                        e.follows.insert(key.clone());
                    } else {
                        e.follows.remove(&key);
                    }
                }
                // Many records: emptying a field they had no one value in leaves it as it was.
                let untouched = !e.members.is_empty()
                    && text.is_empty()
                    && e.name_of(&key).is_some_and(|n| !e.before.contains_key(n));
                if untouched {
                    e.values.remove(&key);
                } else {
                    e.values.insert(key, text);
                }
                e.recompute(&doc.model);
            }
            Event::ClearMisfits => {
                for key in e.misfits() {
                    e.touched.insert(key.clone());
                    e.values.insert(key, String::new());
                }
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
            Event::OpenRelated(uid) => return self.open_related(&uid),
            Event::Link(item, uid) => return self.link_related(&item, &uid, true),
            Event::Unlink(item, uid) => return self.link_related(&item, &uid, false),
            Event::AttachAdd(_)
            | Event::AttachOpen(_)
            | Event::AttachRemove(_)
            | Event::Step(_)
            | Event::NewRelated(_)
                if preview => {}
            Event::NewRelated(item) => return self.new_related(&item),
            Event::Step(step) => {
                if e.dirty() {
                    self.say(
                        Level::Warn,
                        "Kaydedilmemiş değişiklik var: önce Kaydet’e basın ya da Vazgeç’le bırakın.",
                    );
                    return Task::none();
                }
                let Some((at, _)) = e.place else {
                    return Task::none();
                };
                let form = e.form.clone();
                let layer = e.screen.layer.clone();
                let Some(to) = at.checked_add_signed(step as isize) else {
                    return Task::none();
                };
                let slot = doc
                    .model
                    .by_layer_with_uids(&layer)
                    .nth(to)
                    .and_then(|(u, _)| doc.model.slot_of(u));
                if let Some(slot) = slot {
                    self.selection.set([slot]);
                    return if e.docked {
                        self.open_entry_docked(slot, &form)
                    } else {
                        self.open_entry(slot, &form)
                    };
                }
            }
            Event::AttachAdd(photos) => {
                if let Some(slot) = record_slot(&doc.model, &e.uid) {
                    let add = if photos {
                        crate::attachments::Event::AddPhotos(slot)
                    } else {
                        crate::attachments::Event::Add(slot)
                    };
                    return Task::done(Message::Attachment(add));
                }
            }
            Event::AttachOpen(index) => {
                if let Some(slot) = record_slot(&doc.model, &e.uid) {
                    return self.attachment_event(crate::attachments::Event::Open(slot, index));
                }
            }
            Event::AttachRemove(index) => {
                if let Some(slot) = record_slot(&doc.model, &e.uid) {
                    return self.attachment_event(crate::attachments::Event::Remove(slot, index));
                }
            }
            Event::ShowObject(uid) => {
                let slot = kentos_domain::Uuid::parse_str(&uid)
                    .ok()
                    .and_then(|u| doc.model.slot_of(u));
                if let Some(slot) = slot {
                    self.selection.set([slot]);
                    self.zoom_selection();
                }
            }
            Event::Group(id) => {
                if !e.closed.remove(&id) {
                    e.closed.insert(id);
                }
            }
            Event::FirstIssue => return self.go_to_first_issue(),
            Event::Cancel => {
                if e.docked {
                    let (uid, form) = (e.uid.clone(), e.form.clone());
                    return self.redock(&uid, &form);
                }
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
        if !e.members.is_empty() {
            return self.save_many();
        }
        // Only what changed goes in; a new record's defaults with it; a following overridable
        // calculation goes in empty, to be worked out again (docs/adr/0417).
        let values: BTreeMap<String, String> = e
            .written()
            .into_iter()
            .filter(|(k, _)| {
                let name = e
                    .fields
                    .iter()
                    .find(|f| f.key() == k.as_str())
                    .map(|f| f.name.clone());
                name.is_some_and(|n| e.after.get(&n) != e.before.get(&n))
            })
            .collect();
        let input = FormsSubmit {
            form: e.form.clone(),
            draft: e.draft,
            uid: e.uid.clone(),
            values,
            defaults: e.new_record,
            moment: e.moment.clone(),
            user: e.user.clone(),
            expected_revision: Some(e.revision.clone()),
        };
        let name = e.name.clone();
        let form = e.form.clone();
        let docked = e.docked.then(|| e.uid.clone());
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
                if let Some(uid) = &docked {
                    let _ = self.redock(uid, &form);
                }
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

    /// Kaydet on many records: the values typed go to every one through `cad.forms.fill`.
    fn save_many(&mut self) -> Task<Message> {
        let Some(e) = self.form_entry.as_ref() else {
            return Task::none();
        };
        let input = FormsFill {
            form: e.form.clone(),
            draft: e.draft,
            uids: e.members.iter().map(|m| m.uid.clone()).collect(),
            values: e.values.clone(),
            moment: e.moment.clone(),
            user: e.user.clone(),
            expected_revision: Some(e.revision.clone()),
        };
        let (name, count) = (e.name.clone(), e.members.len());
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = forms_fill::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match self.answered(result) {
            Some(done) => {
                self.form_entry = None;
                self.dialog = None;
                let said = if done.changed.is_empty() {
                    "Kayıtlarda değişiklik yok.".to_owned()
                } else {
                    format!(
                        "“{name}” formuyla {} / {count} kayıt yazıldı.",
                        done.changed.len()
                    )
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
        let body = self.entry_column(e, Some(380.0), true);
        let title = if e.dirty() {
            format!("{TITLE} · {} ●", e.name)
        } else {
            format!("{TITLE} · {}", e.name)
        };
        overlay::modal(
            Frame::new(title).push(body).width(620.0),
            e.msg(Event::Cancel),
        )
    }

    /// The docked editor (docs/adr/0416): the form's title, its pages and its footer, scrolled in the
    /// room the table gives it; none when no record's form is docked.
    pub(crate) fn docked_entry_view(&self) -> Option<Element<'_, Message>> {
        let e = self.form_entry.as_ref().filter(|e| e.docked)?;
        let title = if e.dirty() {
            format!("{} ●", e.name)
        } else {
            e.name.clone()
        };
        let body = self.entry_column(e, None, false);
        let pages = scrollable(
            container(
                Column::new()
                    .spacing(10)
                    .push(label::strong(title))
                    .push(body),
            )
            .padding([8, 12]),
        )
        .height(Fill);
        Some(
            Column::new()
                .push(pages)
                .push(kentos_ui::widget::horizontal_divider())
                .push(container(self.entry_footer(e, self.entry_locked(e))).padding([6, 12]))
                .height(Fill)
                .into(),
        )
    }

    /// The editor's column: what it shows, its pages scrolled `height` high (none: as tall as they
    /// are) and its footer.
    fn entry_column<'a>(
        &'a self,
        e: &'a Entry,
        height: Option<f32>,
        footer: bool,
    ) -> Column<'a, Message> {
        let locked = self.entry_locked(e);
        let mut body = Column::new().spacing(12);
        let mut head = if e.members.is_empty() {
            format!("{} · {}", e.screen.object.kind, e.screen.object.layer)
        } else {
            format!(
                "{} nesne · {} · değiştirdiğiniz alanlar hepsine yazılır",
                e.members.len(),
                e.screen.object.layer
            )
        };
        if e.draft {
            head.push_str(" · form yayınlanmadı, taslağıyla");
        }
        // ◀ 3 / 12 ▶: the layer's records one after another (docs/adr/0408 §3).
        let step = |glyph: Icon, to: i32, can: bool| {
            iced::widget::button(icon(glyph).size(12.0))
                .padding([3, 6])
                .style(style::button::tool(false))
                .on_press_maybe(can.then(|| e.msg(Event::Step(to))))
        };
        let mut top = Row::new()
            .spacing(6)
            .align_y(Center)
            .push(container(label::caption(head).style(muted)).width(Fill));
        if let Some((at, n)) = e.place.filter(|(_, n)| *n > 1) {
            top = top
                .push(step(Icon::ChevronLeft, -1, at > 0))
                .push(label::caption(format!("{} / {n}", at + 1)).style(muted))
                .push(step(Icon::ChevronRight, 1, at + 1 < n));
        }
        body = body.push(top);
        if locked {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                "Katman kilitli ya da yetkiniz yok: kayıt yalnız görüntülenir.",
            )]));
        }
        // The record's kind changed under values it does not offer (docs/adr/0422 §2): kept until the
        // user chooses, never emptied silently.
        let misfits = e.misfits();
        if !misfits.is_empty() && !locked {
            let names: Vec<&str> = misfits
                .iter()
                .map(|k| e.label_of(k).unwrap_or(k))
                .collect();
            body = body.push(
                row![
                    container(words::summary(vec![words::text_line(
                        Kind::Warn,
                        format!(
                            "Kaydın alt türü şu değerleri sunmuyor: {}. Alt türün değerlerinden birini seçin ya da boşaltın.",
                            names.join(", ")
                        ),
                    )]))
                    .width(Fill),
                    words::secondary("Uymayanları boşalt", Some(e.msg(Event::ClearMisfits))),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        body = self.entry_pages(e, locked, body, height);
        if footer {
            body = body.push(self.entry_footer(e, locked));
        }
        body
    }

    /// The editor's footer: its problems and warnings, Vazgeç and Kaydet.
    fn entry_footer<'a>(&'a self, e: &'a Entry, locked: bool) -> Row<'a, Message> {
        let (errors, warnings) = if e.tried {
            (e.errors().count(), e.issues.len() - e.errors().count())
        } else {
            (0, 0)
        };
        let mut footer = Row::new().spacing(8).align_y(Center);
        if errors > 0 {
            footer = footer
                .push(words::text_line(Kind::Error, format!("{errors} sorun")))
                .push(words::ghost(FIRST_ISSUE, Some(e.msg(Event::FirstIssue))));
        }
        if warnings > 0 {
            footer = footer.push(words::text_line(Kind::Warn, format!("{warnings} uyarı")));
        }
        footer = footer
            .push(space::horizontal().width(Fill))
            .push(words::secondary(CANCEL, Some(e.msg(Event::Cancel))))
            .push(words::primary(SAVE, (!locked).then(|| e.msg(Event::Save))));
        footer
    }

    fn entry_locked(&self, e: &Entry) -> bool {
        self.forms_locked().is_some()
            || self
                .document
                .as_ref()
                .is_some_and(|d| d.model.layers().is_locked(&e.screen.layer))
    }

    /// The form's pages under `body`: its tabs and the chosen one's items, or its items; scrolled
    /// `height` high, else as tall as they are (the designer's preview scrolls them itself).
    fn entry_pages<'a>(
        &'a self,
        e: &'a Entry,
        locked: bool,
        mut body: Column<'a, Message>,
        height: Option<f32>,
    ) -> Column<'a, Message> {
        let scrolled = |content: Element<'a, Message>| -> Element<'a, Message> {
            match height {
                Some(h) => scrollable(container(content).padding(SCROLLED))
                    .height(typography::scaled(h))
                    .into(),
                None => content,
            }
        };
        let tabbed = matches!(e.body.items.first(), Some(FormItem::Tab { .. }));
        if !tabbed {
            return body.push(scrolled(self.entry_block(e, &e.body.items, locked)));
        }
        let shown: Vec<&FormItem> = e
            .body
            .items
            .iter()
            .filter(|t| e.state(t.id()).is_some_and(|s| s.shown))
            .collect();
        let tabs: Vec<Tab<'_>> = shown
            .iter()
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
        body = body.push(Tabs::new(tabs, e.tab, |i| e.msg(Event::Tab(i))));
        if let Some(tab) = shown.get(e.tab.min(shown.len().saturating_sub(1))) {
            body = body.push(scrolled(self.entry_block(e, tab.items(), locked)));
        }
        body
    }

    /// The designer's preview of its copy (docs/adr/0403 §3): what the record is, the pages, and
    /// what is wrong with it; nothing here writes.
    pub(crate) fn entry_preview<'a>(&'a self, e: &'a Entry) -> Element<'a, Message> {
        let head = if e.uid.is_empty() {
            format!("Yeni kayıt · {}", e.screen.object.layer)
        } else {
            format!(
                "Seçili nesne · {} · {}",
                e.screen.object.kind, e.screen.object.layer
            )
        };
        let mut body = Column::new()
            .spacing(10)
            .push(label::caption(head).style(muted));
        body = self.entry_pages(e, false, body, None);
        let errors = e.errors().count();
        let warnings = e.issues.len() - errors;
        let mut summary = Vec::new();
        if errors > 0 {
            summary.push(words::text_line(Kind::Error, format!("{errors} sorun")));
        }
        if warnings > 0 {
            summary.push(words::text_line(Kind::Warn, format!("{warnings} uyarı")));
        }
        if summary.is_empty() {
            summary.push(words::text_line(
                Kind::Ok,
                "Kayıt formun kurallarına uyuyor.",
            ));
        }
        body.push(Row::with_children(summary).spacing(12)).into()
    }

    /// The items of a tab or of the top: fields and notes in a form, a group under its title, two
    /// columns side by side.
    pub(crate) fn entry_block<'a>(
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
                        .on_press(e.msg(Event::Group(id.clone())));
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
                    // The designer's narrow preview keeps one column.
                    let content: Element<'a, Message> =
                        if *columns == Some(2) && shown.len() > 1 && !e.preview {
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
                            .on_press(e.msg(Event::Group(id.clone()))),
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
                FormItem::Related {
                    id, label: title, ..
                } => {
                    out = flush(&mut run, out);
                    out = out.push(self.related_box(e, id, title.as_deref()));
                }
                FormItem::Attachments {
                    id,
                    label: title,
                    accept,
                    min,
                    max,
                    ..
                } => {
                    out = flush(&mut run, out);
                    out = out.push(self.attachments_box(
                        e,
                        id,
                        title.as_deref(),
                        *accept,
                        *min,
                        *max,
                    ));
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
                    // An overridable calculation typed over: ↺ gives it back to its calculation.
                    let control: Element<'a, Message> = if f.overridable
                        && f.calculate.is_some()
                        && !read_only
                        && !e.follows.contains(&f.field)
                    {
                        let back = kentos_ui::widget::tip(
                            iced::widget::button(icon(Icon::Undo).size(12.0))
                                .padding([4, 6])
                                .style(style::button::tool(false))
                                .on_press(e.msg(Event::Value(f.field.clone(), String::new()))),
                            kentos_ui::widget::Tip::new("Hesaba dön")
                                .body("Yazılan değer bırakılır, alan yeniden hesaplanır"),
                            iced::widget::tooltip::Position::Top,
                        );
                        Row::new()
                            .spacing(4)
                            .align_y(Center)
                            .push(container(control).width(Fill))
                            .push(back)
                            .into()
                    } else {
                        control
                    };
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
                                form = form.help(if f.overridable {
                                    format!("Boşken hesaplanır: {calc}; üzerine yazılabilir")
                                } else {
                                    format!("Hesaplanır: {calc}")
                                });
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
                FormItem::Group { .. }
                | FormItem::Related { .. }
                | FormItem::Attachments { .. } => {
                    form = form.row(self.entry_block(e, std::slice::from_ref(*item), locked));
                }
                FormItem::Tab { .. } => {}
            }
        }
        form.into()
    }

    /// An attachments item (docs/adr/0407 §1): the record's photos and documents, opened, removed and
    /// added in the form (written at once, as Öznitelikler's Ekler; not from the designer's preview).
    fn attachments_box<'a>(
        &'a self,
        e: &'a Entry,
        id: &str,
        title: Option<&str>,
        accept: kentos_contracts::AttachmentAccept,
        min: Option<u32>,
        max: Option<u32>,
    ) -> Element<'a, Message> {
        use kentos_contracts::{AttachmentAccept, AttachmentKind};
        let photos = accept == AttachmentAccept::Photo;
        let documents = accept == AttachmentAccept::Document;
        let views = self
            .document
            .as_ref()
            .and_then(|d| record_slot(&d.model, &e.uid))
            .filter(|_| !e.preview)
            .map(|slot| self.attachment_views(slot))
            .unwrap_or_default();
        let shown: Vec<_> = views
            .into_iter()
            .filter(|v| match accept {
                AttachmentAccept::Any => true,
                AttachmentAccept::Photo => v.kind == AttachmentKind::Photo,
                AttachmentAccept::Document => v.kind != AttachmentKind::Photo,
            })
            .collect();
        let words = title.map_or_else(
            || {
                match accept {
                    AttachmentAccept::Any => "Ekler",
                    AttachmentAccept::Photo => "Fotoğraflar",
                    AttachmentAccept::Document => "Belgeler",
                }
                .to_owned()
            },
            str::to_owned,
        );
        let mut bounds = Vec::new();
        if let Some(n) = min {
            bounds.push(format!("en az {n}"));
        }
        if let Some(n) = max {
            bounds.push(format!("en çok {n}"));
        }
        let count = if bounds.is_empty() {
            shown.len().to_string()
        } else {
            format!("{} · {}", shown.len(), bounds.join(", "))
        };
        let can = !e.preview && e.members.is_empty();
        let mut col = Column::new().spacing(6).push(
            row![
                icon(crate::icons::from_web(Some(if photos {
                    "photo"
                } else {
                    "document"
                })))
                .size(13.0),
                label::strong(words),
                label::caption(count).style(muted),
                space::horizontal().width(Fill),
                words::secondary(
                    if photos { "Fotoğraf ekle" } else { "Ekle" },
                    can.then(|| e.msg(Event::AttachAdd(photos)))
                ),
            ]
            .spacing(8)
            .align_y(Center),
        );
        if shown.is_empty() {
            col = col.push(
                label::caption(if e.preview {
                    "Önizlemede ekler gösterilmez."
                } else if photos {
                    "Fotoğraf yok."
                } else if documents {
                    "Belge yok."
                } else {
                    "Ek yok."
                })
                .style(muted),
            );
        }
        for v in shown {
            col = col.push(
                row![
                    icon(crate::icons::from_web(Some(crate::attachments::kind_icon(
                        v.kind
                    ))))
                    .size(14.0),
                    container(label::body(v.attachment.name.clone())).width(Fill),
                    label::caption(v.source.clone()).style(muted),
                    words::ghost("Aç", Some(e.msg(Event::AttachOpen(v.index)))),
                    words::ghost("Kaldır", can.then(|| e.msg(Event::AttachRemove(v.index)))),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        if let Some(issue) = e.issues.iter().find(|i| i.item.as_deref() == Some(id)) {
            col = col.push(words::text_line(Kind::Error, issue.message.clone()));
        }
        container(col)
            .padding(10)
            .style(style::container::field_box)
            .width(Fill)
            .into()
    }

    /// A related item (docs/adr/0404 §4): the objects referring to the record, each opened by its own
    /// form or shown in the drawing (not from the designer's preview).
    fn related_box<'a>(
        &'a self,
        e: &'a Entry,
        id: &str,
        title: Option<&str>,
    ) -> Element<'a, Message> {
        let uids = e
            .state(id)
            .and_then(|s| s.related.clone())
            .unwrap_or_default();
        let mut col = Column::new().spacing(6).push(
            row![
                icon(Icon::Link).size(13.0),
                label::strong(title.unwrap_or("İlişkili kayıtlar").to_owned()),
                label::caption(format!("{}", uids.len())).style(muted),
            ]
            .spacing(6)
            .align_y(Center),
        );
        if uids.is_empty() {
            col = col.push(label::caption("Bu kayda başvuran nesne yok.").style(muted));
        }
        let doc = self.document.as_ref();
        let live = !e.preview && !e.uid.is_empty();
        // The other layer's objects not yet linked to the record, to link one (docs/adr/0405 §4).
        let candidates: Vec<(String, String)> =
            match (doc, crate::forms::designer::find(&e.body.items, id)) {
                (Some(d), Some(FormItem::Related { layer, field, .. })) if live => {
                    let node = d.model.layers().get(layer);
                    let name = node
                        .and_then(|n| n.fields.iter().find(|f| f.key() == field))
                        .map(|f| f.name.clone())
                        .unwrap_or_default();
                    d.model
                        .by_layer_with_uids(layer)
                        .filter(|(u, x)| {
                            !uids.contains(&u.to_string())
                                && x.base().attrs.get(&name).is_none_or(|v| v.trim() != e.uid)
                        })
                        .take(200)
                        .map(|(u, x)| {
                            let first = node.and_then(|n| {
                                n.fields.iter().find_map(|f| {
                                    x.base().attrs.get(&f.name).filter(|v| !v.is_empty())
                                })
                            });
                            let words = x
                                .base()
                                .label
                                .clone()
                                .or_else(|| first.cloned())
                                .unwrap_or_else(|| u.to_string().chars().take(8).collect());
                            (u.to_string(), words)
                        })
                        .collect()
                }
                _ => Vec::new(),
            };
        // A table of the other layer's fields, its numbers totalled (docs/adr/0425).
        if let (Some(d), Some(FormItem::Related {
            layer,
            field,
            view: kentos_contracts::RelatedView::Table,
            columns,
            ..
        })) = (doc, crate::forms::designer::find(&e.body.items, id))
        {
            col = col.push(related_table(d, layer, field, columns, &uids, e, id, live));
            return self.related_footer(e, id, col, live, candidates);
        }
        for uid in uids.iter().take(50) {
            let words = doc
                .and_then(|d| {
                    let slot = d.model.slot_of(kentos_domain::Uuid::parse_str(uid).ok()?)?;
                    let e = d.model.get(slot)?;
                    // Its label, else its first field's value, else its kind and id.
                    let first = d
                        .model
                        .layers()
                        .get(&e.base().layer_id)
                        .and_then(|n| {
                            n.fields
                                .iter()
                                .find_map(|f| e.base().attrs.get(&f.name).filter(|v| !v.is_empty()))
                        })
                        .cloned();
                    Some(e.base().label.clone().or(first).unwrap_or_else(|| {
                        format!(
                            "{} {}",
                            kentos_forms::kind_label(e),
                            uid.chars().take(8).collect::<String>()
                        )
                    }))
                })
                .unwrap_or_else(|| uid.clone());
            col = col.push(
                row![
                    label::body(words).width(Fill),
                    words::ghost("Git", live.then(|| e.msg(Event::ShowObject(uid.clone())))),
                    words::ghost(
                        "Bağı kopar",
                        live.then(|| e.msg(Event::Unlink(id.to_owned(), uid.clone())))
                    ),
                    words::secondary("Aç", live.then(|| e.msg(Event::OpenRelated(uid.clone())))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        if uids.len() > 50 {
            col = col.push(label::caption(format!("{} nesne daha.", uids.len() - 50)).style(muted));
        }
        self.related_footer(e, id, col, live, candidates)
    }

    /// A related item's Yeni and Bağla, and its frame.
    fn related_footer<'a>(
        &'a self,
        e: &'a Entry,
        id: &str,
        mut col: Column<'a, Message>,
        live: bool,
        candidates: Vec<(String, String)>,
    ) -> Element<'a, Message> {
        let mut actions = Row::new().spacing(6).align_y(Center);
        if live {
            actions = actions.push(words::secondary(
                "Yeni",
                Some(e.msg(Event::NewRelated(id.to_owned()))),
            ));
        }
        if !candidates.is_empty() {
            let item = id.to_owned();
            let picks = candidates.clone();
            actions = actions.push(
                container(
                    Select::new(
                        candidates.iter().map(|(_, w)| Choice::new(w.clone())),
                        None,
                        move |i| Message::FormEntry(Event::Link(item.clone(), picks[i].0.clone())),
                    )
                    .placeholder("Bağla: bir nesne seçin"),
                )
                .width(Fill),
            );
        }
        col = col.push(actions);
        container(col)
            .padding(10)
            .style(style::container::field_box)
            .width(Fill)
            .into()
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
    // The record's date and time or time of day as people write it (16.10.2026 14:30, 08:00;
    // docs/adr/0412 §2); what is typed stays as typed.
    let value: String = e
        .values
        .get(&key)
        .cloned()
        .or_else(|| {
            e.after.get(&field.name).map(|v| match field.kind {
                LayerFieldKind::DateTime | LayerFieldKind::Time => {
                    kentos_contracts::display_value(field, v)
                }
                _ => v.clone(),
            })
        })
        .unwrap_or_default();
    let set = {
        let key = key.clone();
        move |v: String| e.msg(Event::Value(key.clone(), v))
    };
    let placeholder = if e.mixed.contains(&field.name) && !e.values.contains_key(&key) {
        "Çeşitli değerler".to_owned()
    } else {
        item.placeholder.clone().unwrap_or_default()
    };
    // What the item's choice filter offers for the record; every listed value without one.
    let offered = e.state(&item.id).and_then(|s| s.choices.as_ref());
    // A list from another layer offers its values for the record (docs/adr/0404 §2).
    let looked: Option<Vec<(String, String)>> = item.lookup.as_ref().map(|_| {
        e.state(&item.id)
            .and_then(|s| s.options.as_ref())
            .map(|o| {
                o.iter()
                    .map(|c| (c.code.clone(), c.label.clone()))
                    .collect()
            })
            .unwrap_or_default()
    });
    // A retired value only for the record holding it (docs/adr/0418).
    let listed = field.offered_values(e.after.get(&field.name).map_or("", String::as_str));
    let choices: Vec<(String, String)> = match &field.values {
        _ if looked.is_some() => looked.clone().unwrap_or_default(),
        Some(list) if !list.is_empty() => listed
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
    // A list from another layer: a label typed is its value's (as the core writes it).
    let at = choices.iter().position(|(c, _)| *c == code).or_else(|| {
        looked
            .as_ref()
            .and(choices.iter().position(|(_, l)| *l == value))
    });
    // A list from another layer is not known before the record: its segmented choice is a list.
    let widget = match kentos_forms::widget_of(item, field) {
        FormWidget::Segmented if looked.is_some() => FormWidget::List,
        w => w,
    };
    match widget {
        FormWidget::Check | FormWidget::Switch if read_only => label::body(if code == "true" {
            "Evet"
        } else if code == "false" {
            "Hayır"
        } else {
            "—"
        })
        .into(),
        // A check box for each listed value the record may hold (docs/adr/0408 §2).
        FormWidget::CheckList => {
            let held: Vec<String> = field
                .codes_of(&code)
                .into_iter()
                .map(str::to_owned)
                .collect();
            let mut col = Column::new().spacing(4);
            for (c, words) in &choices {
                let on = held.contains(c);
                let next: Vec<&str> = choices
                    .iter()
                    .map(|(x, _)| x.as_str())
                    .filter(|x| {
                        if x == c {
                            !on
                        } else {
                            held.iter().any(|h| h == x)
                        }
                    })
                    .collect();
                let toggle = set(next.join(kentos_contracts::MULTIPLE_JOIN));
                col = col.push(
                    row![
                        check_box(
                            if on { Check::Checked } else { Check::Unchecked },
                            (!read_only).then_some(toggle),
                        ),
                        label::body(words.clone()),
                    ]
                    .spacing(8)
                    .align_y(Center),
                );
            }
            // A held code no longer offered still shows, so it can be taken off.
            for h in held
                .iter()
                .filter(|h| !choices.iter().any(|(c, _)| c == *h))
            {
                let next: Vec<&str> = held
                    .iter()
                    .filter(|x| *x != h)
                    .map(String::as_str)
                    .collect();
                col = col.push(
                    row![
                        check_box(
                            Check::Checked,
                            (!read_only).then(|| set(next.join(kentos_contracts::MULTIPLE_JOIN))),
                        ),
                        label::body(kentos_contracts::display_value(field, h)).style(muted),
                    ]
                    .spacing(8)
                    .align_y(Center),
                );
            }
            col.into()
        }
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
            let held = field.retired_in(e.after.get(&field.name).map_or("", String::as_str));
            let opts: Vec<Opt<'a>> = field
                .values
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| {
                    v.iter()
                        .filter(|c| !c.retired || held.contains(&c.code))
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
        // A date and time, and a time of day, by their pickers (docs/adr/0412 §2).
        FormWidget::Date if !read_only && field.kind == LayerFieldKind::DateTime => {
            let at = code
                .split_once('T')
                .and_then(|(d, t)| Some(DateTime::new(parse_date(d)?, Time::parse(t)?)));
            let set_at = set.clone();
            DatePicker::date_time(at, move |d: Option<DateTime>| {
                set_at(d.map_or_else(String::new, |d| {
                    format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                        d.date.year(),
                        d.date.month(),
                        d.date.day(),
                        d.time.hour(),
                        d.time.minute(),
                        d.time.second()
                    )
                }))
            })
            .anchor(focus_ring(
                text_input("GG.AA.YYYY SS:DD", &value)
                    .size(typography::body())
                    .id(input_id(&key))
                    .on_input(set)
                    .padding([5, 8])
                    .style(style::field::input),
            ))
            .clearable(!field.required)
            .into()
        }
        FormWidget::Date if !read_only && field.kind == LayerFieldKind::Time => {
            let set_at = set.clone();
            TimePicker::new(Time::parse(&code), move |t: Option<Time>| {
                set_at(t.map_or_else(String::new, |t| {
                    format!("{:02}:{:02}:{:02}", t.hour(), t.minute(), t.second())
                }))
            })
            .anchor(focus_ring(
                text_input("SS:DD", &value)
                    .size(typography::body())
                    .id(input_id(&key))
                    .on_input(set)
                    .padding([5, 8])
                    .style(style::field::input),
            ))
            .clearable(!field.required)
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
        // A slider between the field's least and most (docs/adr/0406 §2); the value beside it.
        FormWidget::Slider if !read_only => {
            let (lo, hi) = (
                field
                    .min
                    .as_deref()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0),
                field
                    .max
                    .as_deref()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(100.0),
            );
            let places = if field.kind == LayerFieldKind::Integer {
                0
            } else {
                field.scale.unwrap_or(2).min(6) as usize
            };
            let step = 10f64.powi(-(places as i32));
            let now = value.trim().parse::<f64>().unwrap_or(lo).clamp(lo, hi);
            let set_value = set.clone();
            row![
                iced::widget::slider(lo..=hi.max(lo), now, move |v| {
                    set_value(format!("{v:.places$}"))
                })
                .step(step)
                .style(style::field::slider)
                .width(Fill),
                container(label::body(if value.trim().is_empty() {
                    "—".to_owned()
                } else {
                    value.clone()
                }))
                .width(typography::scaled(56.0))
                .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(10)
            .align_y(Center)
            .into()
        }
        FormWidget::Color if !read_only => colour_control(&value, placeholder, &key, set),
        FormWidget::TextArea if !read_only => {
            let k = key.clone();
            match e.notes.get(&key) {
                Some(content) => text_editor(content)
                    .id(input_id(&key))
                    .size(typography::body())
                    .on_action(move |a| e.msg(Event::Note(k.clone(), a)))
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
            // The layer's values offered while typing (docs/adr/0407 §3): those holding what is
            // typed, the value itself left out.
            if item.suggest && !read_only {
                let offered: Vec<&String> = e
                    .suggestions
                    .get(&item.id)
                    .map(|all| kentos_forms::suggestions_for(all, &value, 8))
                    .unwrap_or_default();
                let picked: Vec<String> = offered.iter().map(|s| (*s).clone()).collect();
                let on_pick = set.clone();
                return kentos_ui::widget::Suggest::new(value.as_str(), placeholder.as_str())
                    .id(input_id(&key))
                    .on_input(set)
                    .suggestions(
                        offered
                            .into_iter()
                            .map(|s| kentos_ui::widget::suggest::Suggestion::new(s.as_str())),
                    )
                    .on_pick(move |i| on_pick(picked.get(i).cloned().unwrap_or_default()))
                    .on_submit(e.msg(Event::Save))
                    .into();
            }
            let input = text_input(&placeholder, &shown)
                .size(typography::body())
                .id(input_id(&key))
                .padding([5, 8])
                .style(style::field::input);
            if read_only {
                input.into()
            } else {
                focus_ring(input.on_input(set).on_submit(e.msg(Event::Save))).into()
            }
        }
    }
}

/// The record's slot by its persistent id.
fn record_slot(doc: &Document, uid: &str) -> Option<Slot> {
    doc.slot_of(kentos_domain::Uuid::parse_str(uid).ok()?)
}

/// A colour field (docs/adr/0407 §2): the colour's swatch opens the picker, the text takes
/// `#RRGGBB` as typed.
fn colour_control<'a>(
    value: &str,
    placeholder: String,
    key: &str,
    set: impl Fn(String) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    use kentos_ui::widget::color::{ColorPicker, parse_hex, to_hex};
    let shown = parse_hex(value).filter(|_| kentos_forms::is_colour(value));
    let size = typography::scaled(24.0);
    let face: Element<'a, Message> = container(space())
        .width(size)
        .height(size)
        .style(move |t: &iced::Theme| container::Style {
            background: shown.map(iced::Background::Color),
            border: iced::Border {
                color: Tokens::of(t).muted,
                width: 1.0,
                radius: kentos_ui::theme::shape::radius(3.0).into(),
            },
            ..container::Style::default()
        })
        .into();
    let pick = set.clone();
    let picker = ColorPicker::new(shown.unwrap_or(iced::Color::BLACK), move |c| {
        pick(to_hex(c))
    })
    .alpha()
    .anchor(face);
    let input = text_input(
        if placeholder.is_empty() {
            "#RRGGBB"
        } else {
            &placeholder
        },
        value,
    )
    .size(typography::body())
    .id(input_id(key))
    .padding([5, 8])
    .style(style::field::input)
    .on_input(set);
    row![picker, focus_ring(input)]
        .spacing(8)
        .align_y(Center)
        .into()
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


/// The related objects as a table (docs/adr/0425): the item's columns (the other layer's first four
/// fields when it names none), each object a row with Aç, Git and Bağı kopar; the decimal columns'
/// totals under them.
#[allow(clippy::too_many_arguments)]
fn related_table<'a>(
    d: &crate::document::Document,
    layer: &str,
    link: &str,
    columns: &[String],
    uids: &[String],
    e: &'a Entry,
    id: &str,
    live: bool,
) -> Element<'a, Message> {
    let fields: Vec<kentos_contracts::LayerField> = d
        .model
        .layers()
        .get(layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let shown: Vec<kentos_contracts::LayerField> = if columns.is_empty() {
        fields.iter().filter(|f| f.key() != link).take(4).cloned().collect()
    } else {
        columns
            .iter()
            .filter_map(|c| fields.iter().find(|f| f.key() == c).cloned())
            .collect()
    };
    let numeric = |f: &kentos_contracts::LayerField| {
        matches!(f.kind, LayerFieldKind::Integer | LayerFieldKind::Decimal)
    };
    let cell = |text: String, f: Option<&kentos_contracts::LayerField>| -> Element<'a, Message> {
        let t = label::body(text);
        if f.is_some_and(numeric) {
            container(t).width(Fill).align_x(iced::alignment::Horizontal::Right).into()
        } else {
            container(t).width(Fill).into()
        }
    };
    let mut head = Row::new().spacing(8);
    for f in &shown {
        let words = label::caption(f.label().to_owned()).style(muted);
        head = head.push(if numeric(f) {
            container(words).width(Fill).align_x(iced::alignment::Horizontal::Right)
        } else {
            container(words).width(Fill)
        });
    }
    head = head.push(space::horizontal().width(84));
    let mut rows = Column::new().spacing(2).push(head);
    // A decimal is a measure and is totalled (an area, a share); an integer is often a number or a floor.
    let mut totals: Vec<Option<f64>> = shown
        .iter()
        .map(|f| (f.kind == LayerFieldKind::Decimal).then_some(0.0))
        .collect();
    for uid in uids.iter().take(200) {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        let mut line = Row::new().spacing(8).align_y(Center);
        for (i, f) in shown.iter().enumerate() {
            let raw = x.base().attrs.get(&f.name).cloned().unwrap_or_default();
            if let (Some(t), Ok(v)) = (totals[i].as_mut(), raw.trim().parse::<f64>()) {
                *t += v;
            }
            line = line.push(cell(kentos_contracts::display_value(f, &raw), Some(f)));
        }
        let tool = |glyph: Icon, tip: &'static str, on: Option<Message>| {
            kentos_ui::widget::tip(
                iced::widget::button(icon(glyph).size(13.0))
                    .padding([3, 5])
                    .style(style::button::tool(false))
                    .on_press_maybe(on),
                kentos_ui::widget::Tip::new(tip),
                iced::widget::tooltip::Position::Top,
            )
        };
        line = line.push(
            row![
                tool(Icon::Open, "Aç", live.then(|| e.msg(Event::OpenRelated(uid.clone())))),
                tool(Icon::Target, "Çizimde göster", live.then(|| e.msg(Event::ShowObject(uid.clone())))),
                tool(
                    Icon::Close,
                    "Bağı kopar",
                    live.then(|| e.msg(Event::Unlink(id.to_owned(), uid.clone())))
                ),
            ]
            .spacing(2)
            .width(84),
        );
        rows = rows.push(line);
    }
    if totals.iter().any(Option::is_some) && !uids.is_empty() {
        let mut foot = Row::new().spacing(8);
        for (i, f) in shown.iter().enumerate() {
            let text = match totals[i] {
                Some(t) => {
                    let scale = f.scale.unwrap_or(0) as usize;
                    format!("{t:.scale$}")
                }
                None if i == 0 => "Toplam".to_owned(),
                None => String::new(),
            };
            foot = foot.push(cell(text, Some(f).filter(|f| f.kind == LayerFieldKind::Decimal)));
        }
        foot = foot.push(space::horizontal().width(84));
        rows = rows.push(container(foot).padding(iced::Padding { top: 4.0, ..iced::Padding::ZERO }));
    }
    if uids.len() > 200 {
        rows = rows.push(label::caption(format!("{} nesne daha.", uids.len() - 200)).style(muted));
    }
    rows.into()
}

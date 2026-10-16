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
    /// The drawing's revision when the window opened: Kaydet writes only onto it (a change made
    /// meanwhile is a conflict, the window and its values stay).
    revision: String,
    /// The designer's preview (docs/adr/0403 §3): nothing is written.
    pub(crate) preview: bool,
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
    /// Many records at once (docs/adr/0403 §6): each with its attributes and shape; empty for one.
    members: Vec<Member>,
    /// The fields whose values differ among the records (“Çeşitli değerler”).
    mixed: BTreeSet<String>,
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
    FirstIssue,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::FormEntry(event)
}

impl Entry {
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

    fn state(&self, id: &str) -> Option<&FormItemState> {
        self.items.iter().find(|s| s.id == id)
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
        };
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
        let Some(name) = self
            .layer_fields_of(&layer)
            .iter()
            .find(|f| f.key() == field)
            .map(|f| f.name.clone())
        else {
            return Task::none();
        };
        let value = link.then_some(record);
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

    /// The window's record prepared again (another object changed): the values typed stay, the
    /// drawing's revision is the new one.
    fn refresh_entry(&mut self) -> Task<Message> {
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
            expected_revision: Some(e.revision.clone()),
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
        let locked = self.forms_locked().is_some()
            || self
                .document
                .as_ref()
                .is_some_and(|d| d.model.layers().is_locked(&e.screen.layer));
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
        body = body.push(label::caption(head).style(muted));
        if locked {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                "Katman kilitli ya da yetkiniz yok: kayıt yalnız görüntülenir.",
            )]));
        }
        body = self.entry_pages(e, locked, body, Some(380.0));
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
        body = body.push(footer);
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
                FormItem::Group { .. } | FormItem::Related { .. } => {
                    form = form.row(self.entry_block(e, std::slice::from_ref(*item), locked));
                }
                FormItem::Tab { .. } => {}
            }
        }
        form.into()
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
        if !candidates.is_empty() {
            let item = id.to_owned();
            let picks = candidates.clone();
            col = col.push(
                Select::new(
                    candidates.iter().map(|(_, w)| Choice::new(w.clone())),
                    None,
                    move |i| Message::FormEntry(Event::Link(item.clone(), picks[i].0.clone())),
                )
                .placeholder("Bağla: bağsız bir nesne seçin"),
            );
        }
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
    let value: String = e
        .values
        .get(&key)
        .cloned()
        .or_else(|| e.after.get(&field.name).cloned())
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
    let choices: Vec<(String, String)> = match &field.values {
        _ if looked.is_some() => looked.clone().unwrap_or_default(),
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

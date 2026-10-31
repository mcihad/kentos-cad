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

use iced::widget::{Column, Row, button, container, row, scrollable, space, text_editor};
use iced::{Center, Element, Fill, Task};
use kentos_commands::FormsFill;
use kentos_commands::{
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
use kentos_ui::widget::{DatePicker, Dialog as Frame, Segmented, Switch, TimePicker};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Formla düzenle";
const SAVE: &str = "Kaydet";
const RELOAD: &str = "Yeniden yükle";
const OVERWRITE: &str = "Üzerine yaz";
/// A new release's form opened with what was typed (docs/adr/0442 §2).
const NEW_RELEASE: &str = "Yeni sürümle aç";
/// Each field takes a note for the next workflow step (docs/adr/0442 §4).
const FIELD_NOTES: &str = "Alanlara not";
/// Kaydet carries the step to the layers' sources (docs/adr/0532 §1).
const COMMIT: &str = "Kaynağa da yaz";
const CANCEL: &str = "Vazgeç";
const FIRST_ISSUE: &str = "İlk soruna git";
const NEXT_ISSUE: &str = "Sonraki sorun";
const UNSAVED: &str = "Kaydedilmedi.";
const DISCARD: &str = "Kaydetmeden kapat";
const KEEP_EDITING: &str = "Düzenlemeye dön";
const SAVE_AND_CLOSE: &str = "Kaydet ve kapat";
const BACK: &str = "Geri";
const NEXT: &str = "İleri";
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
    pub(crate) name: String,
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
    /// The items the user left (docs/adr/0760 §2): a check kept for leaving its field is asked on them
    /// as the values change.
    left: kentos_forms::Left,
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
    /// What is typed into a searched list (docs/adr/0431), by the field's key, until a value is picked.
    queries: BTreeMap<String, String>,
    /// What a searched list searched for (docs/adr/0439 §6): the text typed once typing paused. A
    /// search waits for its turn (`pending`, by a growing number): an answer whose number is no
    /// longer the field's, or that came for another field, is dropped.
    searched: BTreeMap<String, String>,
    pending: BTreeMap<String, u64>,
    turn: u64,
    /// How many pages of suggestions a searched list shows (Daha fazla).
    pages: BTreeMap<String, usize>,
    /// The children linked or unlinked in this window, by the related item (docs/adr/0440 §2): written
    /// with the record by Kaydet, dropped by Vazgeç.
    links: Vec<(String, kentos_commands::FormChild)>,
    /// A related table's or cards' page shown, by the item (docs/adr/0440 §6).
    related_pages: BTreeMap<String, usize>,
    /// The record's snapshot as it stands in this window and the one its log's last step was taken on
    /// (docs/adr/0440 §4): what a reviewer compares.
    snapshot: Option<String>,
    /// A paged list's whole list for the record, by its item (docs/adr/0439 §1): what its search searches.
    whole: BTreeMap<String, Vec<kentos_commands::FieldChoice>>,
    /// The record's place among the layer's records and their count (docs/adr/0408 §3); none in
    /// the preview and with many records.
    place: Option<(usize, usize)>,
    /// The overridable calculations (docs/adr/0417) that follow their inputs in this window: empty
    /// or holding their calculation's value when it opened, and not typed into since.
    follows: BTreeSet<String>,
    /// Kaydet refused because the record or the form changed since the window opened (docs/adr/0438
    /// §6, §7): what changed; the bar offers Yeniden yükle and Üzerine yaz.
    conflict: Option<String>,
    /// The next Kaydet writes over the record as it is now (Üzerine yaz).
    overwrite: bool,
    /// The conflict is a new release of the form (docs/adr/0442 §2): Yeni sürümle aç instead of
    /// Üzerine yaz.
    release_changed: bool,
    /// Alanlara not (docs/adr/0442 §4): each field takes a note, sent with the next workflow step.
    reviewing: bool,
    /// Kaynağa da yaz: the record and its children go to their layers' bound file in the same step.
    commit: bool,
    field_notes: BTreeMap<String, String>,
    /// Bilgi: the record's attachments as `cad.attachments.inspect` read them (docs/adr/0442 §7).
    facts: Option<Vec<kentos_commands::AttachmentFacts>>,
    /// The language its words are shown in (docs/adr/0443 §5): a translation's tag; none: the form's own.
    pub(crate) language: Option<String>,
    /// How many times Sonraki sorun went on (docs/adr/0443 §6): the next press goes to the issue after.
    issue_at: usize,
    /// Vazgeç (or Esc) with what was typed unsaved asks first (docs/adr/0443 §6).
    asking_close: bool,
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
    /// The record and the form as they are now, what was typed kept (docs/adr/0438 §7).
    Reload,
    /// Kaydet over the record as it is now.
    Overwrite,
    Value(String, String),
    /// Typed into a searched list (docs/adr/0431): the field's key and the text; no value yet.
    Query(String, String),
    /// An action pressed, by its item (docs/adr/0441 §6, 0442 §5).
    RunAction(String),
    /// Alanlara not on or off, and a field's note typed (docs/adr/0442 §4).
    Reviewing(bool),
    /// Kaynağa da yaz on or off (docs/adr/0532 §1).
    Commit(bool),
    FieldNote(String, String),
    /// Bilgi on the attachments: what each is, read from its bytes (docs/adr/0442 §7); again off.
    InspectAttachments,
    /// A related item's page (docs/adr/0440 §6): its id and the page, from 0.
    RelatedPage(String, usize),
    /// A searched list's turn came (docs/adr/0439 §6): the field's key and the search's number.
    Search(String, u64),
    /// A searched list shows a page more.
    More(String),
    Note(String, text_editor::Action),
    Tab(usize),
    /// A collapsed group's title: open or close it.
    Group(String),
    /// A related object's form opens (docs/adr/0404 §4).
    OpenRelated(String),
    /// Haritadan seç (docs/adr/0759 §1): a list item's value picked on the map, by its item.
    PickOnMap(String),
    /// The focus entered (true) or left (false) a field's item, by its id (docs/adr/0760 §2).
    Focus(String, bool),
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
    /// The language its words are shown in (docs/adr/0443 §5); none: the form's own.
    Language(Option<String>),
    /// Asked when closing with what was typed unsaved (docs/adr/0443 §6): closed without it, or back
    /// to the form.
    Discard,
    KeepEditing,
    /// Uymayanları boşalt (docs/adr/0422 §2): the values the record's subtype does not offer emptied.
    ClearMisfits,
    /// A step of the form's workflow, by its label (docs/adr/0428): `cad.forms.transition`.
    Transition(String),
    Save,
    Cancel,
}

/// How long typing pauses before a searched list is searched (docs/adr/0439 §6).
const SEARCH_PAUSE: std::time::Duration = std::time::Duration::from_millis(150);
/// A related table's or cards' page (docs/adr/0440 §6).
const RELATED_PAGE: usize = 50;
/// The most forms open one over another (docs/adr/0440 §7).
pub(crate) const MOST_NESTED: usize = 4;
/// The suggestions of a searched list's page.
const SEARCH_PAGE: usize = 12;
/// A list longer than this is searched rather than laid out.
const LONG_LIST: usize = 200;
/// The least width (at 12 px text) the fields of a form take in two columns (docs/adr/0751): two of a label and a
/// field at their ease.
const TWO_COLUMNS: f32 = 880.0;

fn msg(event: Event) -> Message {
    Message::FormEntry(event)
}

impl Entry {
    /// The record's persistent id, as text (for the flow tests).
    #[cfg(test)]
    pub(crate) fn uid_text(&self) -> String {
        self.uid.clone()
    }

    /// Whether Vazgeç asked before dropping what was typed (for the tests).
    #[cfg(test)]
    pub(crate) fn asking_close(&self) -> bool {
        self.asking_close
    }

    /// The errors' fields in the form's order, and how far Sonraki sorun went (for the tests).
    #[cfg(test)]
    pub(crate) fn issue_fields(&self) -> Vec<String> {
        self.errors().filter_map(|i| i.field.clone()).collect()
    }

    /// The record as the window holds it and its items' states (for the conformance test).
    #[cfg(test)]
    pub(crate) fn after_attrs(&self) -> &Attrs {
        &self.after
    }

    #[cfg(test)]
    pub(crate) fn item_states(&self) -> &[FormItemState] {
        &self.items
    }

    /// A listed value's label in the chosen language (for the tests).
    #[cfg(test)]
    pub(crate) fn translated_choice(&self, id: &str, code: &str) -> Option<String> {
        self.said(id).and_then(|w| w.choices.get(code)).cloned()
    }

    #[cfg(test)]
    pub(crate) fn issue_at(&self) -> usize {
        self.issue_at
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

    /// What Haritadan seç needs of a list item (docs/adr/0759 §1): its list layer, its field's key and
    /// its label; none for an item without a list from another layer, a many-valued field, or where
    /// nothing is written (the preview, the many-record mode).
    pub(crate) fn map_pick_of(&self, item: &str) -> Option<(String, String, String)> {
        if self.preview || !self.members.is_empty() {
            return None;
        }
        let FormItem::Field(f) = crate::forms::designer::find(&self.body.items, item)? else {
            return None;
        };
        let lookup = f.lookup.as_ref()?;
        let field = self.fields.iter().find(|x| x.key() == f.field)?;
        if field.holds_many() {
            return None;
        }
        let label = self
            .words_of(&f.id, f.label.as_ref())
            .unwrap_or_else(|| field.label().to_owned());
        Some((lookup.layer.clone(), f.field.clone(), label))
    }

    /// The value the object `uid` (its attributes `values`) picked on the map gives the list item
    /// `item`, judged on the record as it is typed now (docs/adr/0759 §1).
    pub(crate) fn map_picked(
        &self,
        item: &str,
        uid: &str,
        values: &Attrs,
    ) -> Option<Result<(String, String), kentos_forms::PickMiss>> {
        self.screen
            .prepared
            .picked(item, &self.after, &self.screen.object, uid, values)
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
        // The links made or taken off in this window count with the record's related objects
        // (docs/adr/0444 §1), as the submission counts them.
        let mut added: BTreeMap<String, i32> = BTreeMap::new();
        for (item, c) in &self.links {
            if let Some(FormItem::Related { layer, field, .. }) =
                crate::forms::designer::find(&self.body.items, item)
            {
                let linked = self
                    .state(item)
                    .and_then(|s| s.related.as_ref())
                    .is_some_and(|r| r.contains(&c.uid));
                let delta = match (c.unlink, linked) {
                    (true, true) => -1,
                    (false, false) => 1,
                    _ => 0,
                };
                *added
                    .entry(kentos_forms::added_key(layer, field))
                    .or_default() += delta;
            }
        }
        self.screen.object.added = added;
        let prepared = &self.screen.prepared;
        // The checks kept for saving are asked once Kaydet was pressed (docs/adr/0438 §2).
        let moment = if self.tried {
            kentos_forms::Moment::Save
        } else {
            kentos_forms::Moment::Change
        };
        let (after, mut issues) = prepared.assess_left(
            &self.before,
            &self.written(),
            &self.screen.object,
            self.new_record,
            moment,
            &self.left,
        );
        self.items = prepared.described(&after, &self.screen.object);
        self.snapshot =
            kentos_native_application::forms::record_snapshot(doc, &self.form, &self.uid, &after);
        // A paged list's search reaches every value (docs/adr/0439 §1).
        self.whole = self
            .items
            .iter()
            .filter(|s| s.more.is_some())
            .filter_map(|s| {
                Some((
                    s.id.clone(),
                    prepared.options(&s.id, &after, &self.screen.object)?,
                ))
            })
            .collect();
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

    /// An item's words in the chosen language (docs/adr/0443 §5), else `own`.
    pub(crate) fn words_of(&self, id: &str, own: Option<&String>) -> Option<String> {
        self.said(id)
            .and_then(|w| w.label.clone())
            .or_else(|| own.cloned())
    }

    fn said(&self, id: &str) -> Option<&kentos_commands::ItemWords> {
        let tag = self.language.as_ref()?;
        self.body.translations.get(tag)?.items.get(id)
    }

    /// A field's help line in the chosen language.
    fn help_in(&self, id: &str) -> Option<String> {
        self.said(id).and_then(|w| w.help.clone())
    }

    /// A note's text in the chosen language, when it has no `{…}` of the record's values to fill.
    fn note_in(&self, id: &str) -> Option<String> {
        self.said(id)
            .and_then(|w| w.label.clone())
            .filter(|t| !t.contains('{'))
    }

    /// The form's name in the chosen language.
    pub(crate) fn name_in(&self) -> String {
        self.language
            .as_ref()
            .and_then(|tag| self.body.translations.get(tag))
            .and_then(|w| w.name.clone())
            .unwrap_or_else(|| self.name.clone())
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
        self.related_now(item)
    }

    /// A related item's objects as this window has them (docs/adr/0440 §2): the drawing's, the ones
    /// linked here after them, the ones unlinked here left out.
    pub(crate) fn related_now(&self, item: &str) -> Vec<String> {
        let mut out = self
            .state(item)
            .and_then(|s| s.related.clone())
            .unwrap_or_default();
        for (i, c) in &self.links {
            if i != item {
                continue;
            }
            if c.unlink {
                out.retain(|u| *u != c.uid);
            } else if !out.contains(&c.uid) {
                out.push(c.uid.clone());
            }
        }
        out
    }

    /// Whether the window holds links not written yet.
    #[cfg(test)]
    pub(crate) fn pending_for_tests(&self) -> usize {
        self.links.len()
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
        self.after != self.before || !self.links.is_empty()
    }

    /// The issue shown under a field: once touched, or after Kaydet; an error before a warning.
    fn issue_of(&self, key: &str) -> Option<&FormIssue> {
        (self.tried || self.touched.contains(key)).then_some(())?;
        let mine = |i: &&FormIssue| i.field.as_deref() == Some(key);
        self.errors()
            .find(mine)
            .or_else(|| self.issues.iter().find(mine))
    }

    #[cfg(test)]
    pub(crate) fn tab_for_tests(&self) -> usize {
        self.tab
    }

    #[cfg(test)]
    pub(crate) fn conflict_for_tests(&self) -> bool {
        self.conflict.is_some()
    }

    /// An item's state for the record.
    #[cfg(test)]
    pub(crate) fn state_for_tests(&self, id: &str) -> Option<&FormItemState> {
        self.state(id)
    }

    /// A searched list's search waiting for its turn (docs/adr/0439 §6), and what it searched.
    #[cfg(test)]
    pub(crate) fn search_for_tests(&self, key: &str) -> (Option<u64>, Option<String>) {
        (
            self.pending.get(key).copied(),
            self.searched.get(key).cloned(),
        )
    }

    /// The tabs shown, in order: a step form's steps.
    fn shown_tabs(&self) -> Vec<&FormItem> {
        self.body
            .items
            .iter()
            .filter(|t| {
                matches!(t, FormItem::Tab { .. }) && self.state(t.id()).is_some_and(|s| s.shown)
            })
            .collect()
    }

    /// Going to step `to` of a step form (docs/adr/0437): the first step before it with an error, which
    /// holds the record there; none when the way is open (or the form is not one of steps).
    fn blocking_step(&self, to: usize) -> Option<usize> {
        if !self.body.steps || to <= self.tab {
            return None;
        }
        let tabs = self.shown_tabs();
        (self.tab..to.min(tabs.len())).find(|&k| {
            self.errors().any(|i| {
                i.item
                    .as_deref()
                    .is_some_and(|id| kentos_forms_find(tabs[k].items(), id))
            })
        })
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
        f: &kentos_commands::ProjectForm,
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
            conflict: None,
            overwrite: false,
            release_changed: false,
            reviewing: false,
            commit: false,
            field_notes: BTreeMap::new(),
            facts: None,
            language: None,
            issue_at: 0,
            asking_close: false,
            touched: BTreeSet::new(),
            left: kentos_forms::Left::new(),
            after: Attrs::new(),
            items: Vec::new(),
            issues: Vec::new(),
            closed,
            unique_asked: Vec::new(),
            unique: Vec::new(),
            members: Vec::new(),
            mixed: BTreeSet::new(),
            suggestions: BTreeMap::new(),
            queries: BTreeMap::new(),
            searched: BTreeMap::new(),
            pending: BTreeMap::new(),
            turn: 0,
            pages: BTreeMap::new(),
            whole: BTreeMap::new(),
            links: Vec::new(),
            related_pages: BTreeMap::new(),
            snapshot: None,
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
            Event::Save | Event::Cancel | Event::FirstIssue | Event::Reload | Event::Overwrite => {
                Task::none()
            }
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
        // Through a project relation, the link waits for Kaydet with the record (docs/adr/0440 §2).
        let relation = self.document.as_ref().and_then(|d| {
            let form_layer = d
                .model
                .settings()
                .forms
                .iter()
                .find(|f| f.id == e.form)
                .map(|f| f.layer.clone())?;
            d.model
                .settings()
                .relations
                .iter()
                .find(|r| r.parent == form_layer && r.child == layer && r.field == field)
                .map(|r| r.id.clone())
        });
        if let Some(relation) = relation {
            if let Some(e) = self.form_entry.as_mut() {
                let undone = e
                    .links
                    .iter()
                    .position(|(i, c)| *i == item && c.uid == uid && c.unlink == link);
                match undone {
                    // Linking what was unlinked here (or the other way) takes the pending one back.
                    Some(at) => {
                        e.links.remove(at);
                    }
                    None => e.links.push((
                        item.to_owned(),
                        kentos_commands::FormChild {
                            relation,
                            uid: uid.to_owned(),
                            values: BTreeMap::new(),
                            unlink: !link,
                        },
                    )),
                }
            }
            // The bounds count the pending link now (docs/adr/0444 §1).
            if let (Some(e), Some(doc)) = (self.form_entry.as_mut(), self.document.as_ref()) {
                e.recompute(&doc.model);
            }
            self.say(
                Level::Info,
                if link {
                    "Nesne bağlanacak; Kaydet ile kayıtla birlikte yazılır."
                } else {
                    "Nesnenin bağı koparılacak; Kaydet ile kayıtla birlikte yazılır."
                },
            );
            return Task::none();
        }
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
                    .join(kentos_commands::MULTIPLE_JOIN),
            ),
            (true, false) => (!held.is_empty()).then(|| held.join(kentos_commands::MULTIPLE_JOIN)),
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
            kentos_commands::CAD_ENTITIES_SET,
            kentos_commands::CallOp::Execute,
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
            kentos_commands::CAD_POINT_CREATE,
            kentos_commands::CallOp::Execute,
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
            // The record as the drawing holds it now (a related record's change worked its calculations
            // out again, docs/adr/0761): what the typed values go onto.
            if !e.preview && !e.new_record && e.members.is_empty() {
                e.before = screen.attrs.clone();
            }
            e.screen = screen;
            e.revision = revision;
            e.unique_asked.clear();
            e.recompute(&doc.model);
        }
        Task::none()
    }

    /// The form closed: the record's form under it, if one waits, comes back with its draft and its
    /// related objects read anew (docs/adr/0440 §7); else the window closes.
    pub(crate) fn entry_closed(&mut self) -> Task<Message> {
        self.form_entry = self.form_stack.pop();
        if self.form_entry.is_none() {
            self.dialog = None;
            return Task::none();
        }
        let task = self.refresh_entry();
        if let (Some(e), Some(doc)) = (self.form_entry.as_mut(), self.document.as_ref()) {
            e.recompute(&doc.model);
        }
        task
    }

    /// A related object's form over the record's (docs/adr/0440 §7): the record's form waits with its
    /// draft and comes back when the child's closes; at most [`MOST_NESTED`] forms deep.
    fn open_related(&mut self, uid: &str) -> Task<Message> {
        if self.form_stack.len() + 1 >= MOST_NESTED {
            self.say(
                Level::Warn,
                format!(
                    "En çok {MOST_NESTED} form iç içe açılır; önce açık formlardan birini kapatın."
                ),
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
                if let Some(parent) = self.form_entry.take() {
                    self.form_stack.push(parent);
                }
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
            Event::Query(key, text) => {
                e.queries.insert(key.clone(), text);
                e.pages.remove(&key);
                // Typing pauses before the list is searched; a newer key press takes the turn.
                e.turn += 1;
                e.pending.insert(key.clone(), e.turn);
                let after = e.msg(Event::Search(key, e.turn));
                return crate::hover_card::after(SEARCH_PAUSE, after);
            }
            Event::Search(key, turn) => {
                if e.pending.get(&key) == Some(&turn) {
                    e.pending.remove(&key);
                    if let Some(text) = e.queries.get(&key) {
                        e.searched.insert(key, text.clone());
                    }
                }
                return Task::none();
            }
            Event::More(key) => {
                *e.pages.entry(key).or_insert(1) += 1;
                return Task::none();
            }
            Event::RelatedPage(item, page) => {
                e.related_pages.insert(item, page);
                return Task::none();
            }
            Event::Value(key, text) => {
                e.queries.remove(&key);
                e.searched.remove(&key);
                e.pending.remove(&key);
                e.pages.remove(&key);
                // A long text's editor shows a value given as a whole (a trace's, a pick's) too.
                if let Some(content) = e.notes.get_mut(&key)
                    && content.text().trim_end_matches('\n') != text
                {
                    *content = text_editor::Content::with_text(&text);
                }
                e.touched.insert(key.clone());
                if let Some(item) = chosen_item(&e.body.items, &e.fields, &key) {
                    e.left.insert(item);
                }
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
            Event::Tab(i) => {
                // Adım adım (docs/adr/0437): a step with an error is not left forward.
                match e.blocking_step(i) {
                    Some(stuck) => {
                        e.tried = true;
                        e.tab = stuck;
                        self.say(
                            Level::Warn,
                            "Bu adımda sorun var; düzeltmeden sonraki adıma geçilmez.",
                        );
                    }
                    None => e.tab = i,
                }
            }
            Event::OpenRelated(uid) => return self.open_related(&uid),
            Event::PickOnMap(item) => return self.pick_on_map(&item),
            Event::Focus(item, on) => {
                // Leaving a field asks its checks kept for leaving (docs/adr/0760 §2); entering one says
                // nothing. A field never typed in is left too: its checks see an empty value and pass.
                if !on && e.left.insert(item) {
                    e.recompute(&doc.model);
                }
            }
            Event::RunAction(item) => {
                use kentos_commands::ActionKind as K;
                let Some(FormItem::Action {
                    kind,
                    tool,
                    step,
                    runs,
                    ..
                }) = crate::forms::designer::find(&e.body.items, &item).cloned()
                else {
                    return Task::none();
                };
                let slot = record_slot(&doc.model, &e.uid);
                match kind {
                    // The record chosen, the tool's window opened with it (a registered tool only),
                    // where it runs as the action says (docs/adr/0442 §6).
                    K::Tool => {
                        let Some(tool) = tool.filter(|t| self.processing.registry.get(t).is_some())
                        else {
                            return Task::none();
                        };
                        if let Some(slot) = slot {
                            self.selection.set([slot]);
                        }
                        self.open_processing(&tool, None);
                        let place = match runs {
                            kentos_commands::ActionRuns::Auto => None,
                            kentos_commands::ActionRuns::Here => Some("client"),
                            kentos_commands::ActionRuns::Background => Some("worker"),
                        };
                        if let Some(place) = place {
                            let _ = self.update(Message::Processing(
                                crate::processing::Event::Target(place.into()),
                            ));
                        }
                    }
                    // The calculations again in this window; Kaydet writes them (docs/adr/0442 §5).
                    K::Recompute => {
                        if let (Some(e), Some(doc)) =
                            (self.form_entry.as_mut(), self.document.as_ref())
                        {
                            e.follows = e
                                .body
                                .walk()
                                .into_iter()
                                .filter_map(|(_, i)| match i {
                                    FormItem::Field(f) if f.calculate.is_some() => {
                                        Some(f.field.clone())
                                    }
                                    _ => None,
                                })
                                .collect();
                            for k in e.follows.clone() {
                                e.values.remove(&k);
                            }
                            e.recompute(&doc.model);
                        }
                        self.say(
                            Level::Info,
                            "Hesaplar kayıtta yeniden çalıştı; yazmak için Kaydet’e basın.",
                        );
                    }
                    K::Step => {
                        if let Some(step) = step {
                            return self.take_step(step);
                        }
                    }
                    K::Show => {
                        if let Some(slot) = slot {
                            self.selection.set([slot]);
                            return self.update(Message::Run("view.zoomSelection"));
                        }
                    }
                }
                return Task::none();
            }
            Event::InspectAttachments => {
                if e.facts.is_some() {
                    if let Some(e) = self.form_entry.as_mut() {
                        e.facts = None;
                    }
                    return Task::none();
                }
                let folder = doc
                    .path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(|p| p.display().to_string());
                let input = kentos_commands::AttachmentsInspect {
                    uids: vec![e.uid.clone()],
                    folder,
                };
                // Writes nothing: run on the drawing itself, no copy.
                let Some(d) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = kentos_native_application::attachments_inspect::execute(
                    &mut kentos_native_application::ExecutionContext::new(&mut d.model),
                    input,
                );
                if let kentos_commands::CommandResult::Completed { output, .. } = result
                    && let Some(e) = self.form_entry.as_mut()
                {
                    e.facts = output.objects.into_iter().next().map(|o| o.attachments);
                }
                return Task::none();
            }
            Event::Commit(on) => {
                if let Some(e) = self.form_entry.as_mut() {
                    e.commit = on;
                }
                return Task::none();
            }
            Event::Reviewing(on) => {
                if let Some(e) = self.form_entry.as_mut() {
                    e.reviewing = on;
                    if !on {
                        e.field_notes.clear();
                    }
                }
                return Task::none();
            }
            Event::FieldNote(key, text) => {
                if let Some(e) = self.form_entry.as_mut() {
                    if text.is_empty() {
                        e.field_notes.remove(&key);
                    } else {
                        e.field_notes.insert(key, text);
                    }
                }
                return Task::none();
            }
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
            Event::FirstIssue => return self.go_to_next_issue(),
            Event::Language(tag) => {
                e.language = tag.filter(|t| e.body.translations.contains_key(t));
            }
            Event::Cancel if e.dirty() && !e.preview && !e.asking_close => {
                e.asking_close = true;
            }
            // Esc or × on the question goes back to the form: it is the question's Vazgeç.
            Event::Cancel if e.asking_close => e.asking_close = false,
            Event::KeepEditing => e.asking_close = false,
            Event::Cancel | Event::Discard => {
                e.asking_close = false;
                if e.docked {
                    let (uid, form) = (e.uid.clone(), e.form.clone());
                    return self.redock(&uid, &form);
                }
                return self.entry_closed();
            }
            Event::Save => {
                // Kaydet ve kapat: a refused write goes back to the form, its issues shown.
                e.asking_close = false;
                return self.save_entry();
            }
            Event::Reload => {
                // A new release: the record's form is its body now; values of fields it no longer
                // has are dropped and said (docs/adr/0442 §2).
                let dropped = self.form_entry.as_mut().map_or_else(Vec::new, |e| {
                    if !e.release_changed {
                        return Vec::new();
                    }
                    e.release_changed = false;
                    let Some(body) = self
                        .document
                        .as_ref()
                        .and_then(|d| {
                            d.model
                                .settings()
                                .forms
                                .iter()
                                .find(|f| f.id == e.form)
                                .cloned()
                        })
                        .and_then(|f| {
                            if e.draft {
                                Some(f.draft)
                            } else {
                                f.releases.last().map(|r| r.body.clone())
                            }
                        })
                    else {
                        return Vec::new();
                    };
                    let bound: Vec<String> = body
                        .walk()
                        .into_iter()
                        .filter_map(|(_, i)| match i {
                            FormItem::Field(f) => Some(f.field.clone()),
                            _ => None,
                        })
                        .collect();
                    let gone: Vec<String> = e
                        .values
                        .keys()
                        .filter(|k| !bound.contains(k))
                        .cloned()
                        .collect();
                    for k in &gone {
                        e.values.remove(k);
                    }
                    e.body = body;
                    gone
                });
                if !dropped.is_empty() {
                    self.say(
                        Level::Warn,
                        format!(
                            "Yeni sürümde olmayan alanlara yazdıklarınız bırakıldı: {}.",
                            dropped.join(", ")
                        ),
                    );
                }
                let task = self.refresh_entry();
                if let Some(e) = self.form_entry.as_mut() {
                    e.before = e.screen.attrs.clone();
                    e.conflict = None;
                }
                if let (Some(e), Some(doc)) = (self.form_entry.as_mut(), self.document.as_ref()) {
                    e.recompute(&doc.model);
                }
                self.say(
                    Level::Info,
                    "Kayıt yeniden yüklendi; yazdıklarınız duruyor.",
                );
                return task;
            }
            Event::Overwrite => {
                if let Some(e) = self.form_entry.as_mut() {
                    e.overwrite = true;
                    e.conflict = None;
                }
                return self.save_entry();
            }
            Event::Transition(label) => return self.take_step(label),
        }
        Task::none()
    }

    /// A workflow step (docs/adr/0428): the record with the values typed, through `cad.forms.transition`;
    /// the window closes when it went in.
    fn take_step(&mut self, label: String) -> Task<Message> {
        let Some(e) = self.form_entry.as_ref() else {
            return Task::none();
        };
        if !e.members.is_empty() || e.preview || e.uid.is_empty() {
            return Task::none();
        }
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
        let input = kentos_commands::FormsTransition {
            form: e.form.clone(),
            draft: e.draft,
            uid: e.uid.clone(),
            step: label.clone(),
            values,
            notes: e
                .field_notes
                .iter()
                .filter(|(_, v)| !v.trim().is_empty())
                .map(|(k, v)| (k.clone(), v.trim().to_owned()))
                .collect(),
            moment: e.moment.clone(),
            user: e.user.clone(),
            expected_revision: Some(e.revision.clone()),
        };
        let form = e.form.clone();
        let docked = e.docked.then(|| e.uid.clone());
        let state_words = |code: &str| -> String {
            e.screen
                .prepared
                .workflow()
                .and_then(|w| e.fields.iter().find(|f| f.key() == w.field))
                .and_then(|f| f.values.as_ref())
                .and_then(|v| v.iter().find(|c| c.code == code))
                .map_or_else(|| code.to_owned(), |c| c.label.clone())
        };
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = kentos_native_application::forms_transition::execute(
            &mut ExecutionContext::new(&mut doc.model),
            input,
        );
        // The words of the new state, read before the entry is closed.
        let words_of = |done: &kentos_commands::FormsTransitioned| state_words(&done.to);
        match result {
            kentos_commands::CommandResult::Completed { output, .. } => {
                let to = words_of(&output);
                self.form_entry = None;
                self.dialog = None;
                if let Some(uid) = &docked {
                    let _ = self.redock(uid, &form);
                }
                self.say(
                    Level::Success,
                    format!("“{label}”: kayıt “{to}” durumunda."),
                );
                Task::none()
            }
            other => {
                let _ = self.answered(other);
                self.go_to_first_issue()
            }
        }
    }

    fn go_to_first_issue(&mut self) -> Task<Message> {
        if let Some(e) = self.form_entry.as_mut() {
            e.issue_at = 0;
        }
        self.go_to_next_issue()
    }

    /// İlk soruna git, then Sonraki sorun (docs/adr/0443 §6): each press the next error in the form's
    /// order, round again after the last; with no error the first issue.
    fn go_to_next_issue(&mut self) -> Task<Message> {
        let Some(e) = self.form_entry.as_mut() else {
            return Task::none();
        };
        e.tried = true;
        let errors: Vec<FormIssue> = e.errors().cloned().collect();
        let first = if errors.is_empty() {
            e.issues.first().cloned()
        } else {
            let at = e.issue_at % errors.len();
            e.issue_at = at + 1;
            Some(errors[at].clone())
        };
        let Some(first) = first else {
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
        // The checks kept for saving are asked now (docs/adr/0438 §2).
        if let (Some(e), Some(doc)) = (self.form_entry.as_mut(), self.document.as_ref())
            && !e.tried
            && e.members.is_empty()
        {
            e.tried = true;
            e.recompute(&doc.model);
        }
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
            // The record and the release the window opened with (docs/adr/0438 §6, §7): another write
            // to this record, or a new release, is said instead of written over; Üzerine yaz drops them.
            expected_revision: None,
            opened: (!e.overwrite).then(|| e.before.clone()),
            children: e.links.iter().map(|(_, c)| c.clone()).collect(),
            commit: e.commit,
            release: if e.draft || e.overwrite {
                None
            } else {
                e.screen.version
            },
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
        // Changed since the window opened: the bar says what, the values typed stay.
        if let kentos_commands::CommandResult::Conflict { error } = &result
            && matches!(
                error.code.as_str(),
                kentos_commands::codes::RECORD_CHANGED
                    | kentos_commands::codes::FORM_RELEASE_CHANGED
            )
        {
            let words = error.message.clone();
            let release = error.code == kentos_commands::codes::FORM_RELEASE_CHANGED;
            if let Some(e) = self.form_entry.as_mut() {
                e.conflict = Some(words.clone());
                e.overwrite = false;
                e.release_changed = release;
            }
            self.say(Level::Warn, words);
            return Task::none();
        }
        match self.answered(result) {
            Some(done) => {
                let back = self.entry_closed();
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
                let mut said = match (done.changes.len(), done.children.len()) {
                    (0, _) => "Kayıtta değişiklik yok.".to_owned(),
                    (_, 0) => format!("Kayıt “{name}” formuyla yazıldı."),
                    (_, n) => {
                        format!("Kayıt “{name}” formuyla, {n} alt kaydıyla birlikte yazıldı.")
                    }
                };
                // With Kaynağa da yaz, the file's side of the same step (docs/adr/0532 §1).
                if let Some(committed) = &done.committed {
                    let path = self
                        .document
                        .as_ref()
                        .and_then(|d| {
                            d.model
                                .settings()
                                .sources
                                .iter()
                                .find(|s| {
                                    committed.layers.first().is_some_and(|l| l.layer == s.layer)
                                })
                                .or_else(|| d.model.settings().sources.first())
                                .map(|s| s.path.clone())
                        })
                        .unwrap_or_default();
                    said.push(' ');
                    said.push_str(&self.together_words(committed, &path));
                }
                self.say(Level::Success, said);
                back
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

    /// How the record's form is opened, a line a fact, and whether its release's fields changed since it
    /// was published (docs/adr/0444 §2, §3).
    fn entry_how(&self, e: &Entry, locked: bool) -> (Vec<String>, bool) {
        let form = self
            .project_forms()
            .iter()
            .find(|f| f.id == e.form)
            .cloned();
        let mut lines = Vec::new();
        let release = form.as_ref().and_then(|f| f.releases.last().cloned());
        lines.push(match (&release, e.draft) {
            (_, true) | (None, _) => "Taslağıyla açıldı (yayın yok).".to_owned(),
            (Some(r), false) => format!("{}. yayınla açıldı.", r.version),
        });
        if form.as_ref().is_some_and(|f| f.enforce) {
            lines.push("Kuralları katmanın yazma kuralı: başka yoldan yazılan kayıt da bu formla denetlenir.".into());
        }
        let needs = kentos_forms::requirements(&e.body);
        lines.push(if needs.is_empty() {
            "Saf form: yalnız kaydın değerleriyle çalışır.".into()
        } else {
            format!(
                "Gerektirdikleri: {}; bu bilgisayar hepsini sağlar.",
                needs.join(", ")
            )
        });
        if locked {
            lines.push("Salt okunur: katman kilitli.".into());
        } else if e.screen.prepared.locked_by_state(&e.before) {
            lines.push("Salt okunur: kaydın iş akışı durumu kilitli.".into());
        } else {
            lines.push("Yazılabilir: yazma komutun kendi kurallarıyla.".into());
        }
        let stale = !e.draft
            && release.as_ref().is_some_and(|r| {
                r.fields
                    .as_ref()
                    .is_some_and(|d| *d != kentos_commands::bound_fields_digest(&r.body, &e.fields))
            });
        if stale {
            lines.push("Katmanın alanları bu yayından sonra değişti.".into());
        }
        (lines, stale)
    }

    pub(crate) fn form_entry_view(&self) -> Element<'_, Message> {
        let Some(e) = &self.form_entry else {
            return iced::widget::text("").into();
        };
        let name = e.name_in();
        // One size a form, whether what is typed is saved or not (docs/adr/0746).
        let key = format!("{TITLE} · {name}");
        let title = if e.dirty() {
            format!("{key} ●")
        } else {
            key.clone()
        };
        // The pages take what the window has when it was given a size of its own (docs/adr/0746), else they
        // are 460 high (at 12 px text), less in a low window: the footer is always in sight (docs/adr/0749).
        let sized = kentos_ui::widget::sizable::sizes::get(&key).is_some();
        let locked = self.entry_locked(e);
        let mut head = self.entry_head(e, locked);
        let (tabs, page) = self.entry_tabs_and_page(e, locked, Some(f32::INFINITY));
        if let Some(tabs) = tabs {
            head = head.push(tabs);
        }
        let page = container(page.unwrap_or_else(|| space::vertical().into()))
            .height(Fill)
            .max_height(if sized {
                f32::INFINITY
            } else {
                typography::scaled(460.0)
            });
        let frame = Frame::new(title)
            .push(head)
            .fill(page)
            .push(self.entry_footer(e, locked));
        frame.key(key).width(760.0).modal(e.msg(Event::Cancel))
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
        let body = self.entry_head(e, locked);
        let mut body = self.entry_pages(e, locked, body, height);
        if footer {
            body = body.push(self.entry_footer(e, locked));
        }
        body
    }

    /// What the editor shows above its pages: the record, its place among the layer's, how the form is
    /// opened, and what is wrong with its values.
    fn entry_head<'a>(&'a self, e: &'a Entry, locked: bool) -> Column<'a, Message> {
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
        // The forms waiting under this one (docs/adr/0440 §7), the nearest last.
        if let Some(under) = self.form_stack.last() {
            head.push_str(&format!(
                " · altta “{}” formu bekliyor{}",
                under.name,
                match self.form_stack.len() {
                    1 => String::new(),
                    n => format!(" ({n} form)"),
                }
            ));
        }
        // ◀ 3 / 12 ▶: the layer's records one after another (docs/adr/0408 §3).
        let step = |glyph: Icon, to: i32, can: bool| {
            iced::widget::button(icon(glyph).size(12.0))
                .padding(kentos_ui::theme::metrics::icon_small(12.0))
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
        // How the form is opened (docs/adr/0444 §3): its release, whose rules write, what it needs and
        // why it is read-only.
        let (lines, stale) = self.entry_how(e, locked);
        top = top.push(kentos_ui::widget::tip(
            icon(Icon::Info)
                .size(13.0)
                .tone(kentos_ui::icon::Tone::Muted),
            kentos_ui::widget::Tip::new("Bu form").body(lines.join("\n")),
            iced::widget::tooltip::Position::Bottom,
        ));
        body = body.push(top);
        if stale && !e.preview {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                "Katmanın alanları bu yayından sonra değişti; tasarımcı formu gözden geçirip yeniden yayınlamalı.",
            )]));
        }
        if locked {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                "Katman kilitli ya da yetkiniz yok: kayıt yalnız görüntülenir.",
            )]));
        }
        // The record's state locks it (docs/adr/0428 §2): a step takes it out.
        if !locked && e.screen.prepared.locked_by_state(&e.before) {
            let state = e.screen.prepared.state_of(&e.before).unwrap_or_default();
            let words_of = e
                .screen
                .prepared
                .workflow()
                .and_then(|w| e.fields.iter().find(|f| f.key() == w.field))
                .and_then(|f| f.values.as_ref())
                .and_then(|v| v.iter().find(|c| c.code == state))
                .map_or(state.clone(), |c| c.label.clone());
            body = body.push(words::summary(vec![words::text_line(
                Kind::Info,
                format!(
                    "Kayıt “{words_of}” durumunda; değiştirilemez. İş akışının bir adımı onu düzenlemeye açar."
                ),
            )]));
        }
        // The snapshot the last step was taken on, against the record now (docs/adr/0440 §4).
        let logged = e
            .screen
            .prepared
            .workflow()
            .and_then(|w| w.log.as_ref())
            .and_then(|k| e.fields.iter().find(|f| f.key() == k))
            .and_then(|f| e.before.get(&f.name))
            .and_then(|log| {
                kentos_native_application::forms::logged_snapshot(log).map(str::to_owned)
            });
        if let (Some(then), Some(now)) = (logged, e.snapshot.as_ref()) {
            body = body.push(words::summary(vec![if then == *now {
                words::text_line(
                    Kind::Ok,
                    format!("Son adım bu kayıt üzerinde atıldı (anlık görüntü #{then})."),
                )
            } else {
                words::text_line(
                    Kind::Warn,
                    format!(
                        "Kayıt, alt kayıtları ya da ekleri son adımdan sonra değişti: adım #{then} üzerinde atıldı, şimdi #{now}."
                    ),
                )
            }]));
        }
        // A draft (docs/adr/0432): kept incomplete; the form's requirements warn until a step that
        // asks for a valid record.
        if !locked && e.screen.prepared.in_draft(&e.after) {
            let missing = e
                .issues
                .iter()
                .filter(|i| {
                    i.code == kentos_forms::codes::REQUIRED && i.severity == FormSeverity::Warning
                })
                .count();
            if missing > 0 {
                body = body.push(words::summary(vec![words::text_line(
                    Kind::Info,
                    format!(
                        "Taslak: {missing} zorunlu alan boş; kayıt böyle kaydedilir, ilerletmeden önce doldurun."
                    ),
                )]));
            }
        }
        // The record's kind changed under values it does not offer (docs/adr/0422 §2): kept until the
        // user chooses, never emptied silently.
        let misfits = e.misfits();
        if !misfits.is_empty() && !locked {
            let names: Vec<&str> = misfits.iter().map(|k| e.label_of(k).unwrap_or(k)).collect();
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
        // Changed since the window opened (docs/adr/0438 §6, §7): reload keeping what was typed, or
        // write over it.
        // A new release (docs/adr/0442 §2): the record opened again by it, the values typed carried.
        if e.conflict.is_some() && e.release_changed {
            return footer
                .push(words::text_line(
                    Kind::Warn,
                    "Form siz düzenlerken yeniden yayınlandı; kayıt yeni sürümün kurallarıyla yazılır.",
                ))
                .push(space::horizontal().width(Fill))
                .push(words::secondary(CANCEL, Some(e.msg(Event::Cancel))))
                .push(words::primary(NEW_RELEASE, Some(e.msg(Event::Reload))));
        }
        if e.asking_close {
            return footer
                .push(words::text_line(Kind::Warn, UNSAVED))
                .push(space::horizontal().width(Fill))
                .push(words::secondary(DISCARD, Some(e.msg(Event::Discard))))
                .push(words::secondary(
                    KEEP_EDITING,
                    Some(e.msg(Event::KeepEditing)),
                ))
                .push(words::primary(
                    SAVE_AND_CLOSE,
                    (!locked).then(|| e.msg(Event::Save)),
                ));
        }
        if e.conflict.is_some() {
            return footer
                .push(words::text_line(
                    Kind::Warn,
                    "Kayıt ya da form siz düzenlerken değişti.",
                ))
                .push(space::horizontal().width(Fill))
                .push(words::secondary(CANCEL, Some(e.msg(Event::Cancel))))
                .push(words::secondary(RELOAD, Some(e.msg(Event::Reload))))
                .push(words::primary(
                    OVERWRITE,
                    (!locked).then(|| e.msg(Event::Overwrite)),
                ));
        }
        if errors > 0 {
            // Where the last press went, then the next (docs/adr/0443 §6).
            let (said, press) = if e.issue_at == 0 || errors == 1 {
                (format!("{errors} sorun"), FIRST_ISSUE)
            } else {
                (
                    format!("{}/{errors} sorun", (e.issue_at - 1) % errors + 1),
                    NEXT_ISSUE,
                )
            };
            footer = footer
                .push(words::text_line(Kind::Error, said))
                .push(words::ghost(press, Some(e.msg(Event::FirstIssue))));
        }
        if warnings > 0 {
            footer = footer.push(words::text_line(Kind::Warn, format!("{warnings} uyarı")));
        }
        footer = footer
            .push(space::horizontal().width(Fill))
            .push(words::secondary(CANCEL, Some(e.msg(Event::Cancel))));
        // Adım adım (docs/adr/0437): Geri and İleri; Kaydet and the workflow's steps on the last step.
        if e.body.steps {
            let last = e.shown_tabs().len().saturating_sub(1);
            if e.tab > 0 {
                footer = footer.push(words::secondary(BACK, Some(e.msg(Event::Tab(e.tab - 1)))));
            }
            if e.tab < last {
                return footer.push(words::primary(NEXT, Some(e.msg(Event::Tab(e.tab + 1)))));
            }
        }
        // The workflow's steps from the record's state (docs/adr/0428), before Kaydet.
        let live = !e.preview && !e.uid.is_empty() && e.members.is_empty();
        if live
            && !locked
            && e.screen
                .prepared
                .workflow()
                .is_some_and(|w| w.notes.is_some())
        {
            footer = footer.push(
                iced::widget::toggler(e.reviewing)
                    .label(FIELD_NOTES)
                    .size(16.0)
                    .text_size(typography::body())
                    .on_toggle(|on| e.msg(Event::Reviewing(on))),
            );
        }
        // Kaynağa da yaz (docs/adr/0532 §1): only where the record's layer is bound to a source.
        if live && !locked && self.bound_link(&e.screen.layer).is_some() {
            footer = footer.push(
                iced::widget::toggler(e.commit)
                    .label(COMMIT)
                    .size(16.0)
                    .text_size(typography::body())
                    .on_toggle(|on| e.msg(Event::Commit(on))),
            );
        }
        // The form's languages (docs/adr/0443 §5): only what is shown changes.
        if !e.body.translations.is_empty() {
            let tags: Vec<Option<String>> = std::iter::once(None)
                .chain(e.body.translations.keys().cloned().map(Some))
                .collect();
            let chosen = tags.iter().position(|t| *t == e.language);
            let picks = tags.clone();
            footer = footer.push(
                container(
                    Select::new(
                        tags.iter()
                            .map(|t| Choice::new(language_name(t.as_deref()))),
                        chosen,
                        move |i| Message::FormEntry(Event::Language(picks[i].clone())),
                    )
                    .placeholder("Dil"),
                )
                .width(140.0),
            );
        }
        let by_state = e.screen.prepared.locked_by_state(&e.after);
        if let (Some(w), Some(state), true) = (
            e.screen.prepared.workflow(),
            e.screen.prepared.state_of(&e.before),
            live,
        ) {
            for step in w.steps_from(&state) {
                footer = footer.push(words::secondary(
                    step.label.as_str(),
                    (!locked).then(|| e.msg(Event::Transition(step.label.clone()))),
                ));
            }
        }
        footer.push(words::primary(
            SAVE,
            (!locked && !by_state).then(|| e.msg(Event::Save)),
        ))
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
        let (tabs, page) = self.entry_tabs_and_page(e, locked, height);
        if let Some(tabs) = tabs {
            body = body.push(tabs);
        }
        if let Some(page) = page {
            body = body.push(page);
        }
        body
    }

    /// The form's tabs (none when it has none) and the chosen tab's items or its items, scrolled `height`
    /// high, else as tall as they are.
    fn entry_tabs_and_page<'a>(
        &'a self,
        e: &'a Entry,
        locked: bool,
        height: Option<f32>,
    ) -> (Option<Element<'a, Message>>, Option<Element<'a, Message>>) {
        let scrolled = |content: Element<'a, Message>| -> Element<'a, Message> {
            match height {
                // Infinite: as high as the window gives (a resized window, docs/adr/0746).
                Some(h) if h.is_infinite() => scrollable(container(content).padding(SCROLLED))
                    .height(Fill)
                    .into(),
                Some(h) => scrollable(container(content).padding(SCROLLED))
                    .height(typography::scaled(h))
                    .into(),
                None => content,
            }
        };
        let tabbed = matches!(e.body.items.first(), Some(FormItem::Tab { .. }));
        if !tabbed {
            return (
                None,
                Some(scrolled(self.entry_block(e, &e.body.items, locked))),
            );
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
                    FormItem::Tab { id, label, .. } => {
                        e.words_of(id, Some(label)).unwrap_or_default()
                    }
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
        // A step form's tabs are numbered (docs/adr/0437).
        let tabs: Vec<Tab<'_>> = if e.body.steps {
            tabs.into_iter()
                .zip(shown.iter())
                .enumerate()
                .map(|(k, (_, t))| {
                    let label = match t {
                        FormItem::Tab { id, label, .. } => {
                            e.words_of(id, Some(label)).unwrap_or_default()
                        }
                        _ => String::new(),
                    };
                    Tab::new(format!("{}. {label}", k + 1)).closable(false)
                })
                .collect()
        } else {
            tabs
        };
        let page = shown
            .get(e.tab.min(shown.len().saturating_sub(1)))
            .map(|tab| scrolled(self.entry_block(e, tab.items(), locked)));
        (
            Some(Tabs::new(tabs, e.tab, |i| e.msg(Event::Tab(i))).into()),
            page,
        )
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
            let items: Vec<&'a FormItem> = std::mem::take(run);
            // A wide window (a big form made larger, docs/adr/0746) takes the fields in two columns, down then across:
            // the form's order is Tab's; the designer's narrow preview and the docked editor keep one (docs/adr/0751).
            let form: Element<'a, Message> = if items.len() > 1 && !e.preview && !e.docked {
                let wide = typography::scaled(TWO_COLUMNS);
                kentos_ui::widget::Responsive::new(move |size| {
                    if size.width < wide {
                        return self.entry_form(e, &items, locked);
                    }
                    let half = items.len().div_ceil(2);
                    row![
                        container(self.entry_form(e, &items[..half], locked)).width(Fill),
                        container(self.entry_form(e, &items[half..], locked)).width(Fill),
                    ]
                    .spacing(24)
                    .into()
                })
                .height(iced::Length::Shrink)
                .into()
            } else {
                self.entry_form(e, &items, locked)
            };
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
                    across,
                    ..
                } => {
                    out = flush(&mut run, out);
                    let closed = e.closed.contains(id);
                    if *collapsed {
                        // Its title opens and closes it.
                        let words = e
                            .words_of(id, title.as_ref())
                            .unwrap_or_else(|| "Grup".to_owned());
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
                        if *columns == Some(2) && shown.len() > 1 && !e.preview && *across {
                            // A row at a time: Tab goes across, then down (docs/adr/0443 §6).
                            let mut rows = Column::new().spacing(8);
                            for pair in shown.chunks(2) {
                                let right: Element<'a, Message> = match pair.get(1) {
                                    Some(r) => {
                                        self.entry_form_in(e, std::slice::from_ref(r), locked, 80.0)
                                    }
                                    None => space::horizontal().width(Fill).into(),
                                };
                                rows = rows.push(
                                    row![
                                        container(self.entry_form_in(e, &pair[..1], locked, 80.0))
                                            .width(Fill),
                                        container(right).width(Fill),
                                    ]
                                    .spacing(16),
                                );
                            }
                            rows.into()
                        } else if *columns == Some(2) && shown.len() > 1 && !e.preview {
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
                                        e.words_of(id, title.as_ref())
                                            .unwrap_or_else(|| "Grup".to_owned())
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
                FormItem::Location { label: title, .. } => {
                    out = flush(&mut run, out);
                    let uid = if e.preview { "" } else { e.uid.as_str() };
                    out = out.push(self.location_box(uid, title.as_deref()));
                }
                // A divider (docs/adr/0441 §2): a line across, its title on it.
                FormItem::Divider { label: title, .. } => {
                    out = flush(&mut run, out);
                    let line = || {
                        container(iced::widget::rule::horizontal(1).style(style::field::hairline))
                            .width(Fill)
                    };
                    out = out.push(match title {
                        Some(t) => Element::from(
                            row![label::caption(t.clone()).style(muted), line()]
                                .spacing(8)
                                .align_y(Center),
                        ),
                        None => line().into(),
                    });
                }
                // An action (docs/adr/0441 §6, 0442 §5): its kind's work on the record; a read-only
                // record turns off the kinds that write (docs/adr/0442 §5).
                FormItem::Action {
                    id,
                    label: title,
                    kind,
                    tool,
                    ..
                } => {
                    out = flush(&mut run, out);
                    let known = tool
                        .as_ref()
                        .is_none_or(|t| self.processing.registry.get(t).is_some());
                    let read_only =
                        self.entry_locked(e) || e.screen.prepared.locked_by_state(&e.before);
                    let can = known
                        && !e.preview
                        && e.members.is_empty()
                        && !(kind.writes() && read_only);
                    let glyph = match kind {
                        kentos_commands::ActionKind::Tool => Icon::Play,
                        kentos_commands::ActionKind::Recompute => Icon::Redo,
                        kentos_commands::ActionKind::Step => Icon::ChevronRight,
                        kentos_commands::ActionKind::Show => Icon::Target,
                    };
                    let button = iced::widget::button(
                        row![
                            icon(glyph).size(12.0),
                            label::body(e.words_of(id, Some(title)).unwrap_or_default())
                        ]
                        .spacing(6)
                        .align_y(Center),
                    )
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::secondary)
                    .on_press_maybe(can.then(|| e.msg(Event::RunAction(id.clone()))));
                    out = out.push(match tool.as_ref().filter(|_| !known) {
                        Some(t) => row![
                            button,
                            label::caption(format!("“{t}” kayıtlı bir araç değil.")).style(muted)
                        ]
                        .spacing(8)
                        .align_y(Center)
                        .into(),
                        None => Element::from(button),
                    });
                }
                FormItem::Note { id, text } => {
                    out = flush(&mut run, out);
                    let text = e.note_in(id).unwrap_or_else(|| {
                        e.state(id)
                            .and_then(|s| s.text.clone())
                            .unwrap_or_else(|| text.clone())
                    });
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
                    // A mirror is shown, never written (docs/adr/0437); nor is a value the bound table always makes
                    // itself (docs/adr/0754 §3).
                    let read_only = locked
                        || f.mirror
                        || field.made_always()
                        || state.is_some_and(|s| s.read_only);
                    let words = e
                        .words_of(&f.id, f.label.as_ref())
                        .unwrap_or_else(|| field.label().to_owned());
                    let control = entry_control(e, f, field, read_only);
                    // A list's chosen object, its columns live, and the values past the page
                    // (docs/adr/0439 §1, §2).
                    let control: Element<'a, Message> = match state.map(|s| (&s.preview, s.more)) {
                        Some((preview, more)) if preview.is_some() || more.is_some() => {
                            let mut col = Column::new().spacing(3).push(control);
                            if let Some(p) = preview {
                                let line: Vec<String> = p
                                    .iter()
                                    .filter(|v| !v.value.is_empty())
                                    .map(|v| format!("{}: {}", v.label, v.value))
                                    .collect();
                                if !line.is_empty() {
                                    col = col.push(label::caption(line.join("  ·  ")).style(muted));
                                }
                            }
                            if let Some(n) = more.filter(|_| !read_only) {
                                col = col.push(
                                    label::caption(format!(
                                        "Listede {n} değer daha var; aramak için yazın."
                                    ))
                                    .style(muted),
                                );
                            }
                            col.into()
                        }
                        _ => control,
                    };
                    // A number's unit after it (docs/adr/0438 §4).
                    let control: Element<'a, Message> = match &field.unit {
                        Some(unit) => Row::new()
                            .spacing(6)
                            .align_y(Center)
                            .push(container(control).width(Fill))
                            .push(label::caption(unit.clone()).style(muted))
                            .into(),
                        None => control,
                    };
                    // What the listed value held means, said under it (docs/adr/0438 §5).
                    let meaning = e
                        .after
                        .get(&field.name)
                        .and_then(|v| field.values.as_ref()?.iter().find(|c| c.code == *v))
                        .and_then(|c| c.description.clone());
                    // An overridable calculation typed over: ↺ gives it back to its calculation.
                    let control: Element<'a, Message> = if f.overridable
                        && f.calculate.is_some()
                        && !read_only
                        && !e.follows.contains(&f.field)
                    {
                        let back = kentos_ui::widget::tip(
                            iced::widget::button(icon(Icon::Undo).size(12.0))
                                .padding(kentos_ui::theme::metrics::icon_small(12.0))
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
                    // Haritadan seç (docs/adr/0759 §1): a list from another layer takes its value from
                    // an object clicked on the map.
                    let control: Element<'a, Message> =
                        match (!read_only).then(|| e.map_pick_of(&f.id)).flatten() {
                            Some(_) => {
                                let pick = kentos_ui::widget::tip(
                                    iced::widget::button(icon(Icon::Target).size(12.0))
                                        .padding(kentos_ui::theme::metrics::icon_small(12.0))
                                        .style(style::button::tool(false))
                                        .on_press(e.msg(Event::PickOnMap(f.id.clone()))),
                                    kentos_ui::widget::Tip::new("Haritadan seç").body(
                                        "Değeri, listenin katmanından bir nesneye tıklayarak verin",
                                    ),
                                    iced::widget::tooltip::Position::Top,
                                );
                                Row::new()
                                    .spacing(4)
                                    .align_y(Center)
                                    .push(container(control).width(Fill))
                                    .push(pick)
                                    .into()
                            }
                            None => control,
                        };
                    // Alanlara not: the field's note for the next step (docs/adr/0442 §4).
                    let control: Element<'a, Message> = if e.reviewing && !f.mirror {
                        let key = f.field.clone();
                        let note = e.field_notes.get(&f.field).map_or("", String::as_str);
                        Column::new()
                            .spacing(3)
                            .push(control)
                            .push(
                                kentos_ui::widget::text_field("Bu alana not", note).on_input(
                                    move |text| e.msg(Event::FieldNote(key.clone(), text)),
                                ),
                            )
                            .into()
                    } else {
                        control
                    };
                    // Leaving the field asks its checks kept for leaving (docs/adr/0760 §2).
                    let control: Element<'a, Message> = if read_only || e.preview {
                        control
                    } else {
                        let id = f.id.clone();
                        kentos_ui::widget::focus_watch(control)
                            .on_focus(move |on| e.msg(Event::Focus(id.clone(), on)))
                            .into()
                    };
                    let said = said_note(e, &f.field);
                    form = form.field(words, control);
                    if state.is_some_and(|s| s.required) {
                        form = form.required();
                    }
                    match e.issue_of(field.key()).filter(|_| !f.mirror) {
                        Some(issue) if issue.severity == FormSeverity::Error => {
                            form = form.error(Some(issue.message.clone()));
                        }
                        Some(issue) => form = form.warning(Some(issue.message.clone())),
                        // The last step's note on it, as the reviewer wrote it.
                        None if said.is_some() => {
                            form = form.warning(said.map(|n| format!("İnceleyenin notu: {n}")));
                        }
                        None => {
                            if let Some(help) = e.help_in(&f.id).or(f.help.clone()) {
                                form = form.help(help);
                            } else if let Some(meaning) = meaning {
                                form = form.help(meaning);
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
                    let text = e.note_in(id).unwrap_or_else(|| {
                        e.state(id)
                            .and_then(|s| s.text.clone())
                            .unwrap_or_else(|| text.clone())
                    });
                    form = form.row(container(label::caption(text).style(muted)).width(Fill));
                }
                FormItem::Group { .. }
                | FormItem::Related { .. }
                | FormItem::Attachments { .. }
                | FormItem::Location { .. }
                | FormItem::Divider { .. }
                | FormItem::Action { .. } => {
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
        accept: kentos_commands::AttachmentAccept,
        min: Option<u32>,
        max: Option<u32>,
    ) -> Element<'a, Message> {
        use kentos_commands::{AttachmentAccept, AttachmentKind};
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
                words::ghost(
                    if e.facts.is_some() {
                        "Bilgiyi gizle"
                    } else {
                        "Bilgi"
                    },
                    (!e.preview && !shown.is_empty()).then(|| e.msg(Event::InspectAttachments))
                ),
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
            // What it is, as its bytes say (docs/adr/0442 §7).
            if let Some(f) = e.facts.as_ref().and_then(|all| all.get(v.index)) {
                col = col.push(
                    container(label::caption(facts_line(f)).style(muted)).padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: 22.0,
                    }),
                );
            }
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
        let uids = e.related_now(id);
        let mut col = Column::new().spacing(6).push(
            row![
                icon(Icon::Link).size(13.0),
                label::strong(title.unwrap_or("İlişkili kayıtlar").to_owned()),
                label::caption(format!("{}", uids.len())).style(muted),
            ]
            .spacing(6)
            .align_y(Center),
        );
        // Its bounds' issue under its title, before a long list (docs/adr/0444 §1).
        if let Some(issue) = e.issues.iter().find(|i| i.item.as_deref() == Some(id)) {
            col = col.push(words::text_line(Kind::Error, issue.message.clone()));
        }
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
        // A table of the other layer's fields, its numbers totalled (docs/adr/0425), or a card for each
        // (docs/adr/0440 §6), a page at a time.
        // A bar for each value of the first column (docs/adr/0441 §3).
        if let (
            Some(d),
            Some(FormItem::Related {
                layer,
                field,
                view: kentos_commands::RelatedView::Chart,
                columns,
                ..
            }),
        ) = (doc, crate::forms::designer::find(&e.body.items, id))
        {
            col = col.push(related_chart(d, layer, field, columns, &uids));
            return self.related_footer(e, id, col, live, candidates);
        }
        if let (
            Some(d),
            Some(FormItem::Related {
                layer,
                field,
                view:
                    view @ (kentos_commands::RelatedView::Table | kentos_commands::RelatedView::Cards),
                columns,
                ..
            }),
        ) = (doc, crate::forms::designer::find(&e.body.items, id))
        {
            let page = e
                .related_pages
                .get(id)
                .copied()
                .unwrap_or(0)
                .min(uids.len().saturating_sub(1) / RELATED_PAGE);
            let shown_uids: Vec<String> = uids
                .iter()
                .skip(page * RELATED_PAGE)
                .take(RELATED_PAGE)
                .cloned()
                .collect();
            col = col.push(if *view == kentos_commands::RelatedView::Cards {
                related_cards(d, layer, field, columns, &shown_uids, e, id, live)
            } else {
                related_table(d, layer, field, columns, &uids, &shown_uids, e, id, live)
            });
            if uids.len() > RELATED_PAGE {
                let pages = uids.len().div_ceil(RELATED_PAGE);
                let first = page * RELATED_PAGE + 1;
                let last = (first + RELATED_PAGE - 1).min(uids.len());
                let go = |to: usize, glyph: Icon| {
                    iced::widget::button(icon(glyph).size(12.0))
                        .padding(kentos_ui::theme::metrics::icon_small(12.0))
                        .style(style::button::tool(false))
                        .on_press_maybe(
                            (to != page && to < pages)
                                .then(|| e.msg(Event::RelatedPage(id.to_owned(), to))),
                        )
                };
                col = col.push(
                    row![
                        go(page.saturating_sub(1), Icon::ChevronLeft),
                        label::caption(format!("{first}–{last} / {}", uids.len())).style(muted),
                        go(page + 1, Icon::ChevronRight),
                    ]
                    .spacing(6)
                    .align_y(Center),
                );
            }
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
            Control::Press(FIRST_ISSUE | NEXT_ISSUE) => Some(msg(Event::FirstIssue)),
            Control::Press(DISCARD) => Some(msg(Event::Discard)),
            Control::Press(KEEP_EDITING) => Some(msg(Event::KeepEditing)),
            Control::Press(SAVE_AND_CLOSE) => Some(msg(Event::Save)),
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

/// The item a value chosen (not typed) for the field `key` was given on (docs/adr/0760 §2): a list,
/// a check box, a slider or a colour finishes the field as it is chosen, as a browser's change event
/// fires on a selection at once; a typed one is left when the focus leaves it.
fn chosen_item(items: &[FormItem], fields: &[LayerField], key: &str) -> Option<String> {
    for item in items {
        if let FormItem::Field(f) = item
            && f.field == key
            && !f.mirror
        {
            let field = fields.iter().find(|l| l.key() == key)?;
            let typed = matches!(
                kentos_forms::widget_of(f, field),
                FormWidget::Text
                    | FormWidget::TextArea
                    | FormWidget::Number
                    | FormWidget::Search
                    | FormWidget::Date
            );
            return (!typed).then(|| f.id.clone());
        }
        if let Some(id) = chosen_item(item.items(), fields, key) {
            return Some(id);
        }
    }
    None
}

/// A field's control by its widget; read-only shows the value without an edit.
/// An attachment's facts on one line: its type, size, moment and place, and its digest's start.
fn facts_line(f: &kentos_commands::AttachmentFacts) -> String {
    if f.missing {
        return "Bulunamadı: dosya ya da kitaplıktaki fotoğraf yok.".into();
    }
    let mut parts: Vec<String> = Vec::new();
    parts.extend(f.mime.clone());
    if let Some(n) = f.size {
        parts.push(if n < 1024 {
            format!("{n} B")
        } else if n < 1024 * 1024 {
            format!("{:.1} KB", n as f64 / 1024.0).replace('.', ",")
        } else {
            format!("{:.1} MB", n as f64 / (1024.0 * 1024.0)).replace('.', ",")
        });
    }
    if let Some(at) = &f.taken {
        parts.push(format!("çekim {}", at.replacen('T', " ", 1)));
    }
    if let (Some(lat), Some(lon)) = (f.latitude, f.longitude) {
        parts.push(format!("{lat:.5}, {lon:.5}"));
    }
    if let Some(h) = &f.sha256 {
        parts.push(format!("SHA-256 {}…", &h[..12.min(h.len())]));
    }
    if parts.is_empty() {
        "Adres; indirilmedi.".into()
    } else {
        parts.join(" · ")
    }
}

/// The note the record's last workflow step left on a field (docs/adr/0442 §4), from the workflow's
/// notes field as the record holds it (the window's values aside).
fn said_note(e: &Entry, key: &str) -> Option<String> {
    let notes = e.screen.prepared.workflow()?.notes.as_ref()?;
    let name = &e.fields.iter().find(|f| f.key() == notes)?.name;
    let text = e.before.get(name)?;
    let map: BTreeMap<String, String> = serde_json::from_str(text).ok()?;
    map.get(key).cloned()
}

/// The item's placeholder (a made value said, docs/adr/0753 §2), else how the value is written.
fn or_shape<'a>(placeholder: &'a str, shape: &'a str) -> &'a str {
    if placeholder.is_empty() {
        shape
    } else {
        placeholder
    }
}

fn entry_control<'a>(
    e: &'a Entry,
    item: &'a kentos_commands::FormField,
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
                    kentos_commands::display_value(field, v)
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
        e.said(&item.id)
            .and_then(|w| w.placeholder.clone())
            .or_else(|| item.placeholder.clone())
            // A new record's value its table works out (docs/adr/0753 §2), and any record's the table always makes
            // (docs/adr/0754 §3): left empty, said.
            .or_else(|| {
                (e.new_record || field.made_always())
                    .then(|| crate::layer_fields::made_said(field.column.as_ref()))
                    .flatten()
            })
            .unwrap_or_default()
    };
    // What the item's choice filter offers for the record; every listed value without one.
    let offered = e.state(&item.id).and_then(|s| s.choices.as_ref());
    // A list from another layer offers its values for the record (docs/adr/0404 §2).
    // A paged list's whole list where it is searched; its page elsewhere (docs/adr/0439 §1).
    let more = e.state(&item.id).and_then(|s| s.more);
    let paged = match kentos_forms::widget_of(item, field) {
        FormWidget::Search => true,
        FormWidget::List => more.is_some(),
        _ => false,
    };
    let options = e
        .whole
        .get(&item.id)
        .filter(|_| paged)
        .or_else(|| e.state(&item.id).and_then(|s| s.options.as_ref()));
    let looked: Option<Vec<(String, String)>> = item.lookup.as_ref().map(|_| {
        options
            .map(|o| {
                o.iter()
                    .map(|c| (c.code.clone(), c.label.clone()))
                    .collect()
            })
            .unwrap_or_default()
    });
    // A value's columns from its layer, said beside its label (docs/adr/0439 §2).
    let details: BTreeMap<&str, &str> = options
        .map(|o| {
            o.iter()
                .filter_map(|c| Some((c.code.as_str(), c.description.as_deref()?)))
                .collect()
        })
        .unwrap_or_default();
    // A retired value, or one outside its days (docs/adr/0438 §5), only for the record holding it
    // (docs/adr/0418).
    let listed = field.offered_values_on(
        e.after.get(&field.name).map_or("", String::as_str),
        e.moment.as_deref(),
    );
    let choices: Vec<(String, String)> = match &field.values {
        _ if looked.is_some() => looked.clone().unwrap_or_default(),
        Some(list) if !list.is_empty() => listed
            .iter()
            .filter(|c| offered.is_none_or(|o| o.contains(&c.code)))
            // A label in the chosen language; the code is the list's own (docs/adr/0443 §5).
            .map(|c| {
                let said = e
                    .said(&item.id)
                    .and_then(|w| w.choices.get(&c.code))
                    .cloned();
                (c.code.clone(), said.unwrap_or_else(|| c.label.clone()))
            })
            .collect(),
        _ if field.kind == LayerFieldKind::Boolean => vec![
            ("true".into(), "Evet".into()),
            ("false".into(), "Hayır".into()),
        ],
        _ => Vec::new(),
    };
    // The value's code: as typed, a label's code, or the record's.
    let code = kentos_commands::check_value(
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
    // A list from another layer is not known before the record: its segmented choice is a list. A
    // long list is searched (docs/adr/0439 §6): its values are not all laid out.
    let widget = match kentos_forms::widget_of(item, field) {
        FormWidget::Segmented if looked.is_some() => FormWidget::List,
        FormWidget::List if choices.len() > LONG_LIST || more.is_some() => FormWidget::Search,
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
                let toggle = set(next.join(kentos_commands::MULTIPLE_JOIN));
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
                            (!read_only).then(|| set(next.join(kentos_commands::MULTIPLE_JOIN))),
                        ),
                        label::body(kentos_commands::display_value(field, h)).style(muted),
                    ]
                    .spacing(8)
                    .align_y(Center),
                );
            }
            col.into()
        }
        // The held codes as chips, one more added from the list (docs/adr/0431).
        FormWidget::Chips => {
            let held: Vec<String> = field
                .codes_of(&code)
                .into_iter()
                .map(str::to_owned)
                .collect();
            // The list's order, a held code no longer offered after them.
            let ordered = |codes: &[&str]| -> String {
                let mut out: Vec<&str> = choices
                    .iter()
                    .map(|(c, _)| c.as_str())
                    .filter(|c| codes.contains(c))
                    .collect();
                out.extend(
                    codes
                        .iter()
                        .filter(|c| !choices.iter().any(|(x, _)| x == *c)),
                );
                out.join(kentos_commands::MULTIPLE_JOIN)
            };
            let mut chips = Row::new().spacing(4);
            for h in &held {
                let words = choices.iter().find(|(c, _)| c == h).map_or_else(
                    || kentos_commands::display_value(field, h),
                    |(_, l)| l.clone(),
                );
                let rest: Vec<&str> = held
                    .iter()
                    .filter(|x| *x != h)
                    .map(String::as_str)
                    .collect();
                let face = row![label::caption(words)].spacing(4).align_y(Center);
                let face = if read_only {
                    face
                } else {
                    face.push(icon(crate::icons::from_web(Some("close"))).size(10.0))
                };
                chips = chips.push(
                    button(face)
                        .padding([2, 8])
                        .style(style::button::chip(true))
                        .on_press_maybe((!read_only).then(|| set(ordered(&rest)))),
                );
            }
            let mut col = Column::new().spacing(6).push(chips.wrap());
            if !read_only {
                let free: Vec<(String, String)> = choices
                    .iter()
                    .filter(|(c, _)| !held.contains(c))
                    .cloned()
                    .collect();
                if !free.is_empty() {
                    let held_now = held.clone();
                    let set_add = set.clone();
                    let codes: Vec<String> = free.iter().map(|(c, _)| c.clone()).collect();
                    let order: Vec<String> = choices.iter().map(|(c, _)| c.clone()).collect();
                    let add = Select::new(
                        free.iter().map(|(_, l)| Choice::new(l.clone())),
                        None,
                        move |i| {
                            let mut next: Vec<&str> = held_now.iter().map(String::as_str).collect();
                            next.push(codes[i].as_str());
                            set_add(ordered_codes(&next, &order))
                        },
                    )
                    .placeholder("Ekle…");
                    col = col.push(container(add).width(220));
                }
            }
            col.into()
        }
        // The list as a tree (docs/adr/0437): what a label holds before its last “ / ” is its branch.
        FormWidget::Tree if !read_only => {
            let mut col = Column::new().spacing(2);
            let mut last: Option<String> = None;
            for (c, words) in &choices {
                let (branch, leaf) = match words.rsplit_once(" / ") {
                    Some((b, l)) => (Some(b.to_owned()), l.to_owned()),
                    None => (None, words.clone()),
                };
                if branch != last {
                    if let Some(b) = &branch {
                        col = col.push(container(label::caption(b.clone()).style(muted)).padding(
                            iced::Padding {
                                top: 4.0,
                                ..iced::Padding::ZERO
                            },
                        ));
                    }
                    last = branch.clone();
                }
                let indent = if branch.is_some() { 14.0 } else { 0.0 };
                col = col.push(row![
                    space::horizontal().width(indent),
                    button(label::body(leaf))
                        .padding([3, 8])
                        .width(Fill)
                        .style(style::button::row(*c == code))
                        .on_press(set(c.clone())),
                ]);
            }
            container(col)
                .padding(4)
                .width(Fill)
                .style(style::container::field_box)
                .into()
        }
        // A long list searched by typing (docs/adr/0431): the labels holding what is typed.
        FormWidget::Search if !read_only => {
            let query = e.queries.get(&key).cloned();
            let shown = query
                .clone()
                .unwrap_or_else(|| at.map_or_else(|| value.clone(), |i| choices[i].1.clone()));
            // What was searched once typing paused, a page at a time; a value's columns searched
            // with its label (docs/adr/0439 §2, §6).
            let searched = e.searched.get(&key).filter(|_| query.is_some());
            let least = item.min_search.unwrap_or(1).max(1) as usize;
            let short = searched.is_some_and(|q| !q.is_empty() && q.chars().count() < least);
            let page = SEARCH_PAGE * e.pages.get(&key).copied().unwrap_or(1);
            let texts: Vec<String> = choices
                .iter()
                .map(|(c, l)| match details.get(c.as_str()) {
                    Some(d) => format!("{l} {d}"),
                    None => l.clone(),
                })
                .collect();
            // The labels holding what is typed first, then the values whose columns hold it.
            let labels: Vec<String> = choices.iter().map(|(_, l)| l.clone()).collect();
            let found: Vec<usize> = match searched {
                _ if short => Vec::new(),
                Some(q) if !q.is_empty() => {
                    let mut out: Vec<usize> = kentos_forms::suggestions_for(&labels, q, page + 1)
                        .into_iter()
                        .filter_map(|s| labels.iter().position(|x| std::ptr::eq(x, s)))
                        .collect();
                    if out.len() <= page {
                        for s in kentos_forms::suggestions_for(&texts, q, page + 1) {
                            if let Some(i) = texts.iter().position(|x| std::ptr::eq(x, s))
                                && !out.contains(&i)
                            {
                                out.push(i);
                            }
                            if out.len() > page {
                                break;
                            }
                        }
                    }
                    out
                }
                _ => (0..choices.len().min(page + 1)).collect(),
            };
            let more = found.len() > page;
            let found: Vec<usize> = found.into_iter().take(page).collect();
            let picked: Vec<String> = found.iter().map(|&i| choices[i].0.clone()).collect();
            let shown_count = picked.len();
            let k = key.clone();
            let km = key.clone();
            let on_pick = set.clone();
            let mut rows: Vec<kentos_ui::widget::suggest::Suggestion<'a>> = found
                .iter()
                .map(|&i| {
                    let s = kentos_ui::widget::suggest::Suggestion::new(choices[i].1.clone());
                    match details.get(choices[i].0.as_str()) {
                        Some(d) => s.detail((*d).to_owned()),
                        None => s,
                    }
                })
                .collect();
            if more {
                rows.push(kentos_ui::widget::suggest::Suggestion::offer(
                    "Daha fazla göster",
                ));
            }
            let mut suggest = kentos_ui::widget::Suggest::new(shown.as_str(), placeholder.as_str())
                .id(input_id(&key))
                .on_input(move |t| e.msg(Event::Query(k.clone(), t)))
                .suggestions(rows)
                .on_pick(move |i| {
                    if i >= shown_count {
                        e.msg(Event::More(km.clone()))
                    } else {
                        on_pick(picked.get(i).cloned().unwrap_or_default())
                    }
                });
            if short {
                suggest = suggest.note(format!("Aramak için en az {least} harf yazın."));
            } else if query.is_some() && e.pending.contains_key(&key) {
                suggest = suggest.note("Aranıyor…");
            }
            suggest.into()
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
                choices.iter().map(|(c, l)| match details.get(c.as_str()) {
                    Some(d) => Choice::new(l.clone()).detail((*d).to_owned()),
                    None => Choice::new(l.clone()),
                }),
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
        // An instant (docs/adr/0629 §2): its date and time in the picker, its offset from UTC kept; an empty one
        // takes the device's (docs/adr/0751).
        FormWidget::Text | FormWidget::Date
            if !read_only && field.kind == LayerFieldKind::Instant =>
        {
            let (at, offset) = instant_parts(&code);
            let set_at = set.clone();
            let written = offset.clone();
            DatePicker::date_time(at, move |d: Option<DateTime>| {
                set_at(d.map_or_else(String::new, |d| instant_text(&d, &written)))
            })
            .anchor(
                kentos_ui::widget::text_field(
                    or_shape(&placeholder, "YYYY-AA-GGTSS:DD+03:00"),
                    &value,
                )
                .id(input_id(&key))
                .on_input(set),
            )
            .clearable(!field.required)
            .now(now_at(&offset))
            .into()
        }
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
            .anchor(
                kentos_ui::widget::text_field(or_shape(&placeholder, "GG.AA.YYYY SS:DD"), &value)
                    .id(input_id(&key))
                    .on_input(set),
            )
            .clearable(!field.required)
            .now(device_now())
            .into()
        }
        FormWidget::Date if !read_only && field.kind == LayerFieldKind::Time => {
            let set_at = set.clone();
            TimePicker::new(Time::parse(&code), move |t: Option<Time>| {
                set_at(t.map_or_else(String::new, |t| {
                    format!("{:02}:{:02}:{:02}", t.hour(), t.minute(), t.second())
                }))
            })
            .anchor(
                kentos_ui::widget::text_field(or_shape(&placeholder, "SS:DD"), &value)
                    .id(input_id(&key))
                    .on_input(set),
            )
            .clearable(!field.required)
            .now(device_now())
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
            .anchor(
                kentos_ui::widget::text_field(or_shape(&placeholder, "YYYY-AA-GG"), &value)
                    .id(input_id(&key))
                    .on_input(set),
            )
            .clearable(!field.required)
            .now(device_now())
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
        // A read-only text of several lines shows them all (a workflow's log, docs/adr/0429).
        FormWidget::TextArea if read_only => container(label::body(value.clone()))
            .padding([5, 8])
            .width(Fill)
            .style(style::container::field_box)
            .into(),
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
            let input = kentos_ui::widget::text_field(&placeholder, &shown).id(input_id(&key));
            if read_only {
                input.into()
            } else {
                input.on_input(set).on_submit(e.msg(Event::Save)).into()
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
    let input = kentos_ui::widget::text_field(
        if placeholder.is_empty() {
            "#RRGGBB"
        } else {
            &placeholder
        },
        value,
    )
    .id(input_id(key))
    .on_input(set);
    row![picker, input].spacing(8).align_y(Center).into()
}

/// An instant's date and time of day and its offset from UTC (`Z`, `+03:00`): the offset an empty one takes is the
/// device's (docs/adr/0751).
fn instant_parts(code: &str) -> (Option<DateTime>, String) {
    let local = || {
        let east = kentos_interaction::time_version::LOCAL_OFFSET
            .load(std::sync::atomic::Ordering::Relaxed);
        offset_text(east)
    };
    let Some((date, rest)) = code.split_once('T') else {
        return (None, local());
    };
    // The time of day, a fraction dropped, then the offset.
    let clock: String = rest.chars().take(8).collect();
    let mut tail = &rest[clock.len().min(rest.len())..];
    if let Some(after) = tail.strip_prefix('.') {
        tail = after.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    let offset = if tail.is_empty() {
        local()
    } else {
        tail.to_owned()
    };
    let at = parse_date(date).and_then(|d| Some(DateTime::new(d, Time::parse(&clock)?)));
    (at, offset)
}

/// Now on the device's clock: a date's or a time's Şimdi and today, they have no zone (docs/adr/0751; the picker's
/// own is UTC's).
fn device_now() -> DateTime {
    let east =
        kentos_interaction::time_version::LOCAL_OFFSET.load(std::sync::atomic::Ordering::Relaxed);
    DateTime::now(east / 60)
}

/// Now at an instant's offset (`Z`, `+03:00`): the moment Şimdi writes is the present one.
fn now_at(offset: &str) -> DateTime {
    DateTime::now(offset_minutes(offset).unwrap_or(0))
}

/// An instant's offset (`Z`, `±SS:DD`) in minutes east of UTC.
fn offset_minutes(offset: &str) -> Option<i32> {
    if offset == "Z" {
        return Some(0);
    }
    let sign = match offset.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (h, m) = offset[1..].split_once(':')?;
    Some(sign * (h.parse::<i32>().ok()? * 60 + m.parse::<i32>().ok()?))
}

/// Seconds east of UTC as an instant's offset: `Z` at none, else `±SS:DD`.
fn offset_text(east: i32) -> String {
    if east == 0 {
        return "Z".into();
    }
    let sign = if east < 0 { '-' } else { '+' };
    let m = east.unsigned_abs() / 60;
    format!("{sign}{:02}:{:02}", m / 60, m % 60)
}

/// An instant's text from a picked date and time and `offset`.
fn instant_text(d: &DateTime, offset: &str) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{offset}",
        d.date.year(),
        d.date.month(),
        d.date.day(),
        d.time.hour(),
        d.time.minute(),
        d.time.second()
    )
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
    all: &[String],
    uids: &[String],
    e: &'a Entry,
    id: &str,
    live: bool,
) -> Element<'a, Message> {
    let fields: Vec<kentos_commands::LayerField> = d
        .model
        .layers()
        .get(layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let shown: Vec<kentos_commands::LayerField> = if columns.is_empty() {
        fields
            .iter()
            .filter(|f| f.key() != link)
            .take(4)
            .cloned()
            .collect()
    } else {
        columns
            .iter()
            .filter_map(|c| fields.iter().find(|f| f.key() == c).cloned())
            .collect()
    };
    let numeric = |f: &kentos_commands::LayerField| {
        matches!(f.kind, LayerFieldKind::Integer | LayerFieldKind::Decimal)
    };
    let cell = |text: String, f: Option<&kentos_commands::LayerField>| -> Element<'a, Message> {
        let t = label::body(text);
        if f.is_some_and(numeric) {
            container(t)
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .into()
        } else {
            container(t).width(Fill).into()
        }
    };
    let mut head = Row::new().spacing(8);
    for f in &shown {
        let words = label::caption(f.label().to_owned()).style(muted);
        head = head.push(if numeric(f) {
            container(words)
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Right)
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
    // The totals are of every related object, not only the page's.
    for uid in all {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        for (i, f) in shown.iter().enumerate() {
            let raw = x.base().attrs.get(&f.name).map_or("", String::as_str);
            if let (Some(t), Ok(v)) = (totals[i].as_mut(), raw.trim().parse::<f64>()) {
                *t += v;
            }
        }
    }
    for uid in uids {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        let mut line = Row::new().spacing(8).align_y(Center);
        for f in &shown {
            let raw = x.base().attrs.get(&f.name).cloned().unwrap_or_default();
            line = line.push(cell(kentos_commands::display_value(f, &raw), Some(f)));
        }
        let tool = |glyph: Icon, tip: &'static str, on: Option<Message>| {
            kentos_ui::widget::tip(
                iced::widget::button(icon(glyph).size(13.0))
                    .padding(kentos_ui::theme::metrics::icon_small(13.0))
                    .style(style::button::tool(false))
                    .on_press_maybe(on),
                kentos_ui::widget::Tip::new(tip),
                iced::widget::tooltip::Position::Top,
            )
        };
        line = line.push(
            row![
                tool(
                    Icon::Open,
                    "Aç",
                    live.then(|| e.msg(Event::OpenRelated(uid.clone())))
                ),
                tool(
                    Icon::Target,
                    "Çizimde göster",
                    live.then(|| e.msg(Event::ShowObject(uid.clone())))
                ),
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
            foot = foot.push(cell(
                text,
                Some(f).filter(|f| f.kind == LayerFieldKind::Decimal),
            ));
        }
        foot = foot.push(space::horizontal().width(84));
        rows = rows.push(container(foot).padding(iced::Padding {
            top: 4.0,
            ..iced::Padding::ZERO
        }));
    }
    rows.into()
}

/// The related objects as bars (docs/adr/0441 §3): each value of the first column (else the first field
/// that is not the link) with how many objects hold it, the most first; an empty value last.
fn related_chart<'a>(
    d: &crate::document::Document,
    layer: &str,
    link: &str,
    columns: &[String],
    uids: &[String],
) -> Element<'a, Message> {
    let fields: Vec<kentos_commands::LayerField> = d
        .model
        .layers()
        .get(layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let by = columns
        .first()
        .and_then(|c| fields.iter().find(|f| f.key() == c))
        .or_else(|| fields.iter().find(|f| f.key() != link))
        .cloned();
    let Some(by) = by else {
        return label::caption("Grafik için alt katmanın bir alanı gerekir.")
            .style(muted)
            .into();
    };
    let mut counts: Vec<(String, usize)> = Vec::new();
    for uid in uids {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        let v = x.base().attrs.get(&by.name).cloned().unwrap_or_default();
        match counts.iter_mut().find(|(k, _)| *k == v) {
            Some((_, n)) => *n += 1,
            None => counts.push((v, 1)),
        }
    }
    counts.sort_by(|a, b| {
        (a.0.is_empty(), std::cmp::Reverse(a.1), &a.0).cmp(&(
            b.0.is_empty(),
            std::cmp::Reverse(b.1),
            &b.0,
        ))
    });
    let most = counts.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1) as f32;
    let mut rows = Column::new()
        .spacing(4)
        .push(label::caption(format!("{} sayısı", by.label())).style(muted));
    for (value, n) in counts {
        let words = if value.is_empty() {
            "(boş)".to_owned()
        } else {
            kentos_commands::display_value(&by, &value)
        };
        let share = n as f32 / most;
        rows = rows.push(
            row![
                container(label::caption(words)).width(140),
                kentos_ui::widget::progress::bar(Some(share))
                    .width(Fill)
                    .height(10.0),
                container(label::caption(n.to_string()))
                    .width(40)
                    .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }
    rows.into()
}

/// The related objects as cards (docs/adr/0440 §6): each its columns' labels and values, as people
/// read them, with Aç, Çizimde göster and Bağı kopar.
#[allow(clippy::too_many_arguments)]
fn related_cards<'a>(
    d: &crate::document::Document,
    layer: &str,
    link: &str,
    columns: &[String],
    uids: &[String],
    e: &'a Entry,
    id: &str,
    live: bool,
) -> Element<'a, Message> {
    let fields: Vec<kentos_commands::LayerField> = d
        .model
        .layers()
        .get(layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let shown: Vec<kentos_commands::LayerField> = if columns.is_empty() {
        fields
            .iter()
            .filter(|f| f.key() != link)
            .take(4)
            .cloned()
            .collect()
    } else {
        columns
            .iter()
            .filter_map(|c| fields.iter().find(|f| f.key() == c).cloned())
            .collect()
    };
    let mut cards = Column::new().spacing(6);
    for uid in uids {
        let Some(x) = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| d.model.slot_of(u))
            .and_then(|s| d.model.get(s))
        else {
            continue;
        };
        let mut body = Column::new().spacing(2);
        for f in &shown {
            let raw = x.base().attrs.get(&f.name).cloned().unwrap_or_default();
            body = body.push(
                row![
                    container(label::caption(f.label().to_owned()).style(muted)).width(120),
                    label::body(kentos_commands::display_value(f, &raw)),
                ]
                .spacing(8),
            );
        }
        let tool = |glyph: Icon, tip: &'static str, on: Option<Message>| {
            kentos_ui::widget::tip(
                iced::widget::button(icon(glyph).size(13.0))
                    .padding(kentos_ui::theme::metrics::icon_small(13.0))
                    .style(style::button::tool(false))
                    .on_press_maybe(on),
                kentos_ui::widget::Tip::new(tip),
                iced::widget::tooltip::Position::Top,
            )
        };
        let tools = Row::new()
            .spacing(2)
            .push(tool(
                Icon::Open,
                "Aç",
                live.then(|| e.msg(Event::OpenRelated(uid.clone()))),
            ))
            .push(tool(
                Icon::Target,
                "Çizimde göster",
                live.then(|| e.msg(Event::ShowObject(uid.clone()))),
            ))
            .push(tool(
                Icon::Close,
                "Bağı kopar",
                live.then(|| e.msg(Event::Unlink(id.to_owned(), uid.clone()))),
            ));
        cards = cards.push(
            container(
                row![container(body).width(Fill), tools]
                    .spacing(8)
                    .align_y(iced::alignment::Vertical::Top),
            )
            .padding(8)
            .width(Fill)
            .style(style::container::field_box),
        );
    }
    cards.into()
}

/// `codes` in the list's order (`list`), a held code the list no longer offers after them.
fn ordered_codes(codes: &[&str], list: &[String]) -> String {
    let mut out: Vec<&str> = Vec::new();
    for c in codes {
        if !out.contains(c) {
            out.push(c);
        }
    }
    let rank = |c: &str| list.iter().position(|x| x == c).unwrap_or(usize::MAX);
    out.sort_by_key(|c| rank(c));
    out.join(kentos_commands::MULTIPLE_JOIN)
}

#[cfg(test)]
pub(crate) fn ordered_for_tests(codes: &[&str], list: &[&str]) -> String {
    let list: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
    ordered_codes(codes, &list)
}

/// A language tag's name as its speakers write it (docs/adr/0443 §5); none: the form's own, Türkçe.
pub(crate) fn language_name(tag: Option<&str>) -> String {
    let Some(tag) = tag else {
        return "Türkçe".to_owned();
    };
    let lang = tag.split('-').next().unwrap_or(tag);
    let name = match lang {
        "en" => "English",
        "de" => "Deutsch",
        "fr" => "Français",
        "es" => "Español",
        "it" => "Italiano",
        "pt" => "Português",
        "nl" => "Nederlands",
        "ru" => "Русский",
        "uk" => "Українська",
        "ar" => "العربية",
        "fa" => "فارسی",
        "ku" | "kmr" => "Kurdî",
        "az" => "Azərbaycanca",
        "el" => "Ελληνικά",
        "bg" => "Български",
        "ka" => "ქართული",
        "ja" => "日本語",
        "zh" => "中文",
        _ => return tag.to_owned(),
    };
    if lang == tag {
        name.to_owned()
    } else {
        format!("{name} ({tag})")
    }
}

#[cfg(test)]
mod tests {
    use super::{instant_parts, instant_text, offset_minutes, offset_text};
    use kentos_ui::attribute::{Date, DateTime, Time};

    #[test]
    fn an_instant_parts_its_date_and_time_and_keeps_its_offset() {
        let at = |y, m, d, h, mi, s| {
            Some(DateTime::new(
                Date::new(y, m, d).expect("a date"),
                Time::new(h, mi, s).expect("a time"),
            ))
        };
        // Canonical texts (docs/adr/0629 §2): seconds always, a zero offset `Z`, a fraction's trailing zeros dropped.
        assert_eq!(
            instant_parts("2026-10-28T16:05:12+03:00"),
            (at(2026, 10, 28, 16, 5, 12), "+03:00".into())
        );
        assert_eq!(
            instant_parts("2026-03-14T09:30:00Z"),
            (at(2026, 3, 14, 9, 30, 0), "Z".into())
        );
        assert_eq!(
            instant_parts("2026-03-14T09:30:00.125-05:30"),
            (at(2026, 3, 14, 9, 30, 0), "-05:30".into()),
            "the fraction dropped"
        );
        // What the picker writes back keeps the offset the value had.
        let (picked, offset) = instant_parts("2026-10-28T16:05:12+03:00");
        assert_eq!(
            instant_text(&picked.expect("a moment"), &offset),
            "2026-10-28T16:05:12+03:00"
        );
        assert_eq!(
            instant_text(
                &DateTime::new(
                    Date::new(1869, 1, 1).expect("a date"),
                    Time::new(0, 0, 0).expect("a time")
                ),
                "Z"
            ),
            "1869-01-01T00:00:00Z"
        );
    }

    #[test]
    fn an_empty_or_unread_instant_takes_the_devices_offset() {
        let east = kentos_interaction::time_version::LOCAL_OFFSET
            .load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(instant_parts(""), (None, offset_text(east)));
        assert_eq!(instant_parts("2026-13-01Tsabah"), (None, offset_text(east)));
        assert_eq!(offset_text(0), "Z");
        assert_eq!(offset_text(3 * 3600), "+03:00");
        assert_eq!(offset_text(-(5 * 3600 + 30 * 60)), "-05:30");
        assert_eq!(offset_text(5 * 3600 + 45 * 60), "+05:45");
        // Şimdi at the value's own offset: the offsets read back as minutes.
        assert_eq!(offset_minutes("Z"), Some(0));
        assert_eq!(offset_minutes("+03:00"), Some(180));
        assert_eq!(offset_minutes("-05:30"), Some(-330));
        assert_eq!(offset_minutes("03:00"), None);
        for east in [0, 3 * 3600, -(5 * 3600 + 30 * 60), 14 * 3600] {
            assert_eq!(offset_minutes(&offset_text(east)), Some(east / 60));
        }
    }
}

#[cfg(test)]
mod left_tests {
    use super::Event;
    use crate::app::App;

    fn said(app: &App, key: &str) -> Option<String> {
        app.form_entry
            .as_ref()
            .and_then(|e| e.issue_of(key))
            .map(|i| i.message.clone())
    }

    fn event(app: &mut App, event: Event) {
        crate::form_scenes::entry(app, event);
    }

    /// The survey form on B-4, twelve storeys and a short note typed.
    fn typed() -> App {
        let mut app = crate::files_testing::app_with_drawing();
        crate::form_scenes::left_checks(&mut app);
        assert_eq!(
            app.form_entry.as_ref().and_then(|e| e.value_of("Kat")),
            Some("9".into()),
            "B-4, nine storeys: {:?}",
            crate::files_testing::last_said(&app)
        );
        event(&mut app, Event::Value("Kat".into(), "12".into()));
        event(&mut app, Event::Value("Not".into(), "kısa".into()));
        app
    }

    #[test]
    fn a_check_kept_for_leaving_waits_until_the_field_is_left() {
        let mut app = typed();
        assert_eq!(said(&app, "Kat"), None, "typing: not yet");
        assert_eq!(said(&app, "Not"), None);
        event(&mut app, Event::Focus("kat".into(), true));
        assert_eq!(said(&app, "Kat"), None, "entering says nothing");
        event(&mut app, Event::Focus("kat".into(), false));
        assert_eq!(
            said(&app, "Kat").as_deref(),
            Some("10 kattan yüksek binada ruhsat ayrıca denetlenir.")
        );
        assert_eq!(said(&app, "Not"), None, "the note not left");
        event(&mut app, Event::Focus("not".into(), false));
        assert_eq!(
            said(&app, "Not").as_deref(),
            Some("Not en az 10 karakter olmalı.")
        );
        // Typed again after leaving: the field stays left, the check answers the new value.
        event(
            &mut app,
            Event::Value("Not".into(), "Kolonlarda çatlak var.".into()),
        );
        assert_eq!(said(&app, "Not"), None);
        event(&mut app, Event::Value("Kat".into(), "8".into()));
        assert_eq!(said(&app, "Kat"), None);
    }

    #[test]
    fn saving_asks_the_checks_of_fields_never_left() {
        let mut app = typed();
        let _ = app.update(crate::app::Message::FormEntry(Event::Save));
        let e = app
            .form_entry
            .as_ref()
            .expect("an error keeps the window open");
        assert!(
            e.issues
                .iter()
                .any(|i| i.message == "Not en az 10 karakter olmalı."),
            "{:?}",
            e.issues
        );
    }

    #[test]
    fn a_value_chosen_from_a_list_leaves_its_field_at_once() {
        let mut app = typed();
        event(&mut app, Event::Value("Durum".into(), "kotu".into()));
        let e = app.form_entry.as_ref().expect("the form");
        assert!(e.left.contains("durum"), "a list's choice leaves it");
        assert!(!e.left.contains("kat"), "a typed field waits for the focus");
    }
}

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

use crate::gis;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use iced::widget::{Column, Row, button, container, row, scrollable, space, text_editor};
use iced::{Center, Element, Fill, Task};
use kentos_commands::FormsFill;
use kentos_commands::{
    FormBody, FormIssue, FormItem, FormItemState, FormSeverity, FormWidget, FormsSubmit,
    LayerField, LayerFieldKind, RelatedAction,
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

#[path = "entry_sources.rs"]
mod sources;
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
    /// Opened from a related item that offers reading its children, not changing them
    /// (docs/adr/0762 §1): nothing is written.
    pub(crate) read_only: bool,
    /// A related child whose Sil asks first (docs/adr/0762 §4): its item and id.
    removing: Option<(String, String)>,
    /// How the sources of the form's lists stand (docs/adr/0763), the question asked last and whether it runs.
    sources: Vec<kentos_commands::FormSource>,
    sources_asked: u64,
    sources_busy: bool,
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
    /// The lists' sources as a question (its number) found them (docs/adr/0763).
    SourcesAnswered(u64, Vec<kentos_commands::FormSource>),
    /// Yeniden dene: the sources asked again.
    SourcesAgain,
    /// A source's way out, by its layer.
    SourceWay(String, kentos_commands::FormSourceRetry),
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
    /// A related object's form opens (docs/adr/0404 §4): the item and the object; read-only when
    /// the item offers no change (docs/adr/0762 §1).
    OpenRelated(String, String),
    /// A related child's Sil (docs/adr/0762 §4): asked in its row, then deleted; Vazgeç forgets it.
    DeleteRelated(String, String),
    ConfirmDelete,
    KeepChild,
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
    Message::Gis(gis::Event::FormEntry(event))
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
            Message::Gis(gis::Event::FormPreview(event))
        } else {
            Message::Gis(gis::Event::FormEntry(event))
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
        app.gis
            .form_entry
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
            app.gis.form_entry.as_ref().and_then(|e| e.value_of("Kat")),
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
        let _ = app.update(crate::app::Message::Gis(crate::gis::Event::FormEntry(
            Event::Save,
        )));
        let e = app
            .gis
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
        let e = app.gis.form_entry.as_ref().expect("the form");
        assert!(e.left.contains("durum"), "a list's choice leaves it");
        assert!(!e.left.contains("kat"), "a typed field waits for the focus");
    }
}

mod opening;

mod steps;

mod view;

mod items;
use items::*;

mod controls;
use controls::*;

mod related;
#[cfg(test)]
pub(crate) use related::ordered_for_tests;
use related::*;

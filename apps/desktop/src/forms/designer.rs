//! Formlar (docs/adr/0401 §1, §4, §9): the project's forms and their designer.
//!
//! ```text
//! ┌ Formlar ────────────────────────────────────────────────────────────────┐
//! │ Formlar          │ [Sekme] [Grup] [Not]  [↑] [↓] [Sil]                   │
//! │ ● Bina denetimi  │ ▾ Genel                │ Öğe: Kat                      │
//! │   Yol formu      │     Durum *            │ Etiket   [Kat sayısı      ]   │
//! │ [+ Yeni form]    │     Kat ƒ              │ Öğe türü [Sayı          ▾]   │
//! │ Katmanın alanları│   ▾ Denetim            │ ☐ Zorunlu  ☐ Salt okunur      │
//! │   + Tarih        │       Onaylı           │ Görünürlük [Kat >= 1   ] [ε]  │
//! │                  │ Sorunlar: ⚠ …                                          │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │ Ad [Bina denetimi]  Taslak 3. sürüm · son yayın 1   ☐ Kuralları uygula   │
//! │                          [Formu sil]   [Vazgeç] [Yayınla] [Kaydet]       │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the chosen form's draft; the preflight (`kentos_forms::preflight`)
//! is asked of the copy as it changes. Kaydet writes the copy (`cad.forms.edit`'s `update`, and
//! `rename` when the name changed) and closes the window; Yayınla writes it and publishes it,
//! the window stays; Vazgeç forgets the copy and closes. Kuralları uygula and Formu sil write at
//! once. Another form is chosen only when the copy is saved or given up.
//!
//! The copy keeps its own history (docs/adr/0403 §1): Geri al and Yinele (Ctrl+Z, Ctrl+Y) step
//! through it, typing into one text runs as one step; Kaydet or another form forgets it. Items are
//! dragged in the tree before, after or into a tab or group (§2); a drop the form's shape does not
//! take is said and does nothing.

use iced::Task;
use kentos_contracts::{
    FormBody, FormCheck, FormField, FormIssue, FormItem, FormWidget, FormsEdit, FormsEdited,
    FormsOp, LayerField, ProjectForm,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, forms_edit};

use crate::app::{App, Dialog, Message};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Formlar";
pub(super) const SAVE: &str = "Kaydet";
pub(super) const PUBLISH: &str = "Yayınla";
pub(super) const CANCEL: &str = "Vazgeç";
pub(super) const REMOVE_FORM: &str = "Formu sil";
pub(super) const NEW_FORM: &str = "Yeni form";
pub(super) const CREATE: &str = "Oluştur";
pub(super) const ENFORCE: &str = "Kuralları uygula";
pub(super) const NAME: &str = "Ad";
pub(super) const ADD_TAB: &str = "Sekme";
pub(super) const ADD_GROUP: &str = "Grup";
pub(super) const ADD_NOTE: &str = "Not";
pub(super) const UP: &str = "Yukarı";
pub(super) const DOWN: &str = "Aşağı";
pub(super) const REMOVE_ITEM: &str = "Öğeyi sil";
pub(super) const ADD_RELATED: &str = "İlişkili";
pub(super) const ADD_ATTACHMENTS: &str = "Ekler";
pub(super) const ADD_LOCATION: &str = "Konum";
pub(super) const AUDIT: &str = "Kayıtları denetle";
pub(super) const RECOMPUTE: &str = "Hesapları yenile";
pub(super) const EXPORT: &str = "Dışa aktar…";
pub(super) const COPY_ITEM: &str = "Öğeyi kopyala";
pub(super) const PASTE_ITEM: &str = "Yapıştır";
pub(super) const IMPORT: &str = "İçe aktar…";
pub(super) const UNDO: &str = "Geri al";
pub(super) const PREVIEW: &str = "Önizleme";
pub(super) const CHANGES: &str = "Değişiklikler";
pub(super) const FLOW: &str = "İş akışı";
pub(super) const PROPERTIES: &str = "Özellikler";
pub(super) const REDO: &str = "Yinele";
/// The steps the copy's history keeps.
const HISTORY: usize = 200;

/// A rule of an item the window edits (the expression builder writes it back).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    Visible(String),
    Required(String),
    ReadOnly(String),
    Check(String, usize),
    /// A field's default as an expression (docs/adr/0402 §1).
    Default(String),
    /// A field's calculation.
    Calculate(String),
    /// What a listed field offers (`@secenek` is each listed value).
    ChoiceFilter(String),
    /// The filter of a field's list from another layer (docs/adr/0404 §1).
    LookupFilter(String),
}

impl Rule {
    pub(crate) fn item(&self) -> &str {
        match self {
            Rule::Visible(i)
            | Rule::Required(i)
            | Rule::ReadOnly(i)
            | Rule::Check(i, _)
            | Rule::Default(i)
            | Rule::Calculate(i)
            | Rule::ChoiceFilter(i)
            | Rule::LookupFilter(i) => i,
        }
    }

    /// The rule's place as the preflight names it.
    pub(super) fn path(&self) -> String {
        match self {
            Rule::Visible(_) => "visibleWhen".into(),
            Rule::Required(_) => "requiredWhen".into(),
            Rule::ReadOnly(_) => "readOnlyWhen".into(),
            Rule::Check(_, i) => format!("checks/{i}"),
            Rule::Default(_) => "defaultExpr".into(),
            Rule::Calculate(_) => "calculate".into(),
            Rule::ChoiceFilter(_) => "choiceFilter".into(),
            Rule::LookupFilter(_) => "lookup.filter".into(),
        }
    }
}

/// A text of an item the inspector edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Text {
    Label,
    Help,
    Placeholder,
    Default,
    Note,
    CheckMessage(usize),
    /// An attachments item's fewest and most (docs/adr/0407 §1).
    AttachMin,
    AttachMax,
}

/// A new form being made: its name and layer.
#[derive(Debug, Clone, Default)]
pub struct NewForm {
    pub(super) name: String,
    pub(super) layer: Option<String>,
}

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    /// The chosen form.
    pub(crate) chosen: Option<String>,
    /// The copy of its draft the window edits, and the name typed.
    pub(crate) body: Option<FormBody>,
    pub(crate) name: String,
    /// The chosen item.
    pub(crate) item: Option<String>,
    pub(crate) dirty: bool,
    pub(crate) new_form: Option<NewForm>,
    /// The copy before each step, and the steps taken back (docs/adr/0403 §1).
    pub(crate) undo: Vec<FormBody>,
    pub(crate) redo: Vec<FormBody>,
    /// The text being typed into: its steps run as one.
    pub(crate) typing: Option<String>,
    /// The tabs and groups folded in the tree.
    pub(crate) folded: std::collections::BTreeSet<String>,
    /// The third column shows the preview instead of the chosen item's properties (§3).
    pub(crate) previewing: bool,
    /// Why the preview cannot be shown (the copy's preflight error).
    pub(crate) preview_error: Option<String>,
    /// The name a new scenario takes (docs/adr/0420).
    pub(crate) scenario_name: String,
    /// The third column shows what changed since the last release (docs/adr/0427).
    pub(crate) changes: bool,
    /// The third column shows the records' workflow (docs/adr/0428).
    pub(crate) flow: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Senaryolar (docs/adr/0420): the preview's values kept as a scenario, its name typed, one removed.
    SaveScenario,
    ScenarioName(String),
    RemoveScenario(usize),
    Choose(String),
    NewForm,
    NewName(String),
    NewLayer(String),
    Create,
    CancelNew,
    SelectItem(String),
    AddTab,
    AddGroup,
    AddNote,
    AddField(String),
    /// A related item (docs/adr/0404 §4) where the chosen item is.
    AddRelated,
    /// An attachments item (docs/adr/0407 §1) where the chosen item is.
    AddAttachments,
    /// Konum (docs/adr/0433): the record's place.
    AddLocation,
    /// What an attachments item counts and adds.
    Accept(kentos_contracts::AttachmentAccept),
    /// A text field offers the layer's values (docs/adr/0407 §3).
    Suggest(bool),
    /// Yansıma (docs/adr/0437): the chosen field item shown again, read-only, right after it.
    AddMirror,
    /// Kopyala and Yapıştır (docs/adr/0437): the chosen item with what it holds, into this or another form.
    CopyItem,
    PasteItem,
    /// A field's list from another layer: its layer, none for the field's own list.
    LookupLayer(Option<String>),
    /// The list's key (`$uid`: the object's persistent id) and its label's field.
    LookupKey(String),
    LookupLabel(Option<String>),
    /// The chain: a match added, its two fields, one removed.
    AddMatch,
    MatchField(usize, String),
    MatchRecord(usize, String),
    RemoveMatch(usize),
    /// A related item's layer and field.
    RelatedLayer(String),
    RelatedField(String),
    /// The chosen item shown in a subtype, or no longer (docs/adr/0426): its visibility rule rewritten.
    ShowInKind(String),
    /// A related item shown as a list or a table (docs/adr/0425).
    RelatedView(kentos_contracts::RelatedView),
    /// A field of the other layer taken into the table's columns, or out of them.
    RelatedColumn(String),
    Move(i32),
    /// An item dragged onto another in the tree, by their places in the tree's order.
    Drop(usize, usize, kentos_ui::widget::tree_view::Place),
    /// A tab or group folded or unfolded in the tree.
    Fold(String),
    Undo,
    Redo,
    /// The third column: the properties (`false`) or the preview.
    Preview(bool),
    /// The third column: what changed since the last release (docs/adr/0427).
    Changes,
    /// The third column: the records' workflow (docs/adr/0428).
    Flow,
    /// İş akışı: made (with the first listed field and its first two states) or taken away.
    WorkflowAdd,
    WorkflowRemove,
    WorkflowField(String),
    WorkflowInitial(String),
    WorkflowLocked(String),
    /// A draft state taken in or out (docs/adr/0432).
    WorkflowDraft(String),
    /// The text field each step appends its line to (docs/adr/0429); none: no log.
    WorkflowLog(Option<String>),
    StepAdd,
    StepRemove(usize),
    StepLabel(usize, String),
    StepFrom(usize, String),
    StepTo(usize, String),
    StepValid(usize, bool),
    StepReason(usize, Option<String>),
    /// Hesapları yenile on the chosen form (docs/adr/0421).
    Recompute,
    /// Dışa aktar… and İçe aktar… (docs/adr/0435): the form as a package file.
    Export,
    Exported(Option<std::path::PathBuf>),
    Import,
    Imported(Option<(String, String)>),
    /// Kayıtları denetle on the chosen form (docs/adr/0406 §1).
    Audit,
    RemoveItem,
    Text(Text, String),
    Widget(Option<FormWidget>),
    Required(bool),
    ReadOnly(bool),
    /// A field's value is not repeated on the layer.
    Unique(bool),
    /// The calculation may be typed over (docs/adr/0417).
    Overridable(bool),
    /// A new record starts with the session's last value.
    ReuseLast(bool),
    /// A check's failure is a warning, not an error.
    CheckWarning(usize, bool),
    /// When a check is asked (docs/adr/0438 §2): none as the value changes.
    CheckMoment(usize, Option<kentos_contracts::CheckMoment>),
    /// A group starts closed.
    Collapsed(bool),
    /// The fields a record hides are emptied (the form's setting).
    ClearHidden(bool),
    /// Adım adım (docs/adr/0437).
    Steps(bool),
    Columns(u8),
    RuleText(Rule, String),
    Builder(Rule),
    AddCheck,
    RemoveCheck(usize),
    Name(String),
    Save,
    Publish,
    Enforce(bool),
    RemoveForm,
    Cancel,
}

pub(super) fn msg(event: Event) -> Message {
    Message::Forms(event)
}

// ── The body's edits: pure, on the window's copy ─────────────────────────

/// The item of `id`, anywhere in `items`.
pub fn find<'a>(items: &'a [FormItem], id: &str) -> Option<&'a FormItem> {
    items.iter().find_map(|i| {
        if i.id() == id {
            Some(i)
        } else {
            find(i.items(), id)
        }
    })
}

fn find_mut<'a>(items: &'a mut [FormItem], id: &str) -> Option<&'a mut FormItem> {
    for i in items.iter_mut() {
        if i.id() == id {
            return Some(i);
        }
        if let Some(found) = i.items_mut().and_then(|xs| find_mut(xs, id)) {
            return Some(found);
        }
    }
    None
}

/// The list holding `id` and its place there.
fn holder_mut<'a>(
    items: &'a mut Vec<FormItem>,
    id: &str,
) -> Option<(&'a mut Vec<FormItem>, usize)> {
    if let Some(at) = items.iter().position(|i| i.id() == id) {
        return Some((items, at));
    }
    for i in items.iter_mut() {
        if let Some(found) = i.items_mut().and_then(|xs| holder_mut(xs, id)) {
            return Some(found);
        }
    }
    None
}

/// Every item id of the body.
fn ids(items: &[FormItem]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(xs: &[FormItem], out: &mut Vec<String>) {
        for i in xs {
            out.push(i.id().to_owned());
            walk(i.items(), out);
        }
    }
    walk(items, &mut out);
    out
}

/// The keys the body's field items name.
pub(super) fn bound(items: &[FormItem]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(xs: &[FormItem], out: &mut Vec<String>) {
        for i in xs {
            if let FormItem::Field(f) = i {
                out.push(f.field.clone());
            }
            walk(i.items(), out);
        }
    }
    walk(items, &mut out);
    out
}

fn has_tabs(body: &FormBody) -> bool {
    body.items
        .first()
        .is_some_and(|i| matches!(i, FormItem::Tab { .. }))
}

/// Where a new item goes: into the chosen tab or group, after the chosen field or note, else the
/// first tab or the top.
fn place(body: &mut FormBody, chosen: Option<&str>, item: FormItem) {
    if let Some(id) = chosen {
        let container = matches!(
            find(&body.items, id),
            Some(FormItem::Tab { .. } | FormItem::Group { .. })
        );
        if container {
            if let Some(xs) = find_mut(&mut body.items, id).and_then(FormItem::items_mut) {
                xs.push(item);
                return;
            }
        } else if let Some((xs, at)) = holder_mut(&mut body.items, id) {
            xs.insert(at + 1, item);
            return;
        }
    }
    match body.items.first_mut() {
        Some(tab @ FormItem::Tab { .. }) => tab.items_mut().expect("a tab holds items").push(item),
        _ => body.items.push(item),
    }
}

/// A copied item as `body` takes it (docs/adr/0437): every id made fresh in `body`, its fields bound to
/// `to`'s by key, else by name; a tab only into a form of tabs, another item only into one.
fn pasted(
    body: &FormBody,
    item: FormItem,
    from: &[kentos_contracts::LayerField],
    to: &[kentos_contracts::LayerField],
) -> Result<FormItem, String> {
    let tabbed = has_tabs(body);
    if matches!(item, FormItem::Tab { .. }) && !tabbed && !body.items.is_empty() {
        return Err("Sekme yalnız sekmeli bir forma yapıştırılır; önce bir sekme ekleyin.".into());
    }
    let mut taken = ids(&body.items);
    fn fresh(item: &mut FormItem, taken: &mut Vec<String>) {
        let id = kentos_forms::item_id_of(item.id(), taken);
        taken.push(id.clone());
        match item {
            FormItem::Tab { id: own, items, .. } | FormItem::Group { id: own, items, .. } => {
                *own = id;
                for i in items {
                    fresh(i, taken);
                }
            }
            FormItem::Field(f) => f.id = id,
            FormItem::Note { id: own, .. }
            | FormItem::Related { id: own, .. }
            | FormItem::Attachments { id: own, .. }
            | FormItem::Location { id: own, .. } => *own = id,
        }
    }
    let mut item = item;
    fresh(&mut item, &mut taken);
    let pairs: Vec<kentos_forms::field_refs::Rebind> = from
        .iter()
        .filter_map(|f| {
            let t = to
                .iter()
                .find(|t| t.key() == f.key())
                .or_else(|| to.iter().find(|t| t.name == f.name))?;
            Some(kentos_forms::field_refs::Rebind {
                from_key: f.key().to_owned(),
                from_name: f.name.clone(),
                to_key: t.key().to_owned(),
                to_name: t.name.clone(),
            })
        })
        .collect();
    let mut holder = FormBody {
        revision: 1,
        items: vec![item],
        clear_hidden: false,
        tests: Vec::new(),
        workflow: None,
        steps: false,
    };
    kentos_forms::field_refs::rebind_body(&mut holder, &pairs);
    Ok(holder.items.remove(0))
}

/// A new tab: the first one gathers what the top had (“Genel”).
fn add_tab(body: &mut FormBody) -> String {
    let mut taken = ids(&body.items);
    if !has_tabs(body) && !body.items.is_empty() {
        let id = kentos_forms::item_id_of("genel", &taken);
        taken.push(id.clone());
        let items = std::mem::take(&mut body.items);
        body.items.push(FormItem::Tab {
            id,
            label: "Genel".into(),
            items,
            visible_when: None,
        });
    }
    let n = body.items.len() + 1;
    let label = format!("Sekme {n}");
    let id = kentos_forms::item_id_of(&label, &taken);
    body.items.push(FormItem::Tab {
        id: id.clone(),
        label,
        items: Vec::new(),
        visible_when: None,
    });
    id
}

/// Moves the item `by` places in its list.
fn shift(body: &mut FormBody, id: &str, by: i32) {
    if let Some((xs, at)) = holder_mut(&mut body.items, id) {
        let to = at as i64 + i64::from(by);
        if (0..xs.len() as i64).contains(&to) {
            xs.swap(at, to as usize);
        }
    }
}

/// Moves `source` before, after or into `target` (docs/adr/0403 §2), or says why the form's shape
/// does not take it.
pub(super) fn move_item(
    body: &mut FormBody,
    source: &str,
    target: &str,
    place: kentos_ui::widget::tree_view::Place,
) -> Result<(), &'static str> {
    use kentos_ui::widget::tree_view::Place;
    if source == target {
        return Err("Öğe kendi üstüne bırakılamaz.");
    }
    let moved = find(&body.items, source).ok_or("Taşınan öğe formda yok.")?;
    if find(moved.items(), target).is_some() {
        return Err("Bir sekme ya da grup kendi içine taşınamaz.");
    }
    let is_tab = matches!(moved, FormItem::Tab { .. });
    let tabbed = has_tabs(body);
    let target_item = find(&body.items, target).ok_or("Hedef öğe formda yok.")?;
    let target_top = body.items.iter().any(|i| i.id() == target);
    match place {
        Place::Into => {
            if !matches!(target_item, FormItem::Tab { .. } | FormItem::Group { .. }) {
                return Err("Öğe yalnız bir sekmenin ya da grubun içine bırakılır.");
            }
            if is_tab {
                return Err("Sekme yalnız en üstte, sekmeler arasında olur.");
            }
        }
        Place::Before | Place::After => {
            if is_tab && !target_top {
                return Err("Sekme yalnız en üstte, sekmeler arasında olur.");
            }
            if tabbed && !is_tab && target_top {
                return Err("Sekmeli formda öğeler bir sekmenin içinde olur.");
            }
        }
    }
    let item = {
        let (xs, at) = holder_mut(&mut body.items, source).ok_or("Taşınan öğe formda yok.")?;
        xs.remove(at)
    };
    match place {
        Place::Into => {
            find_mut(&mut body.items, target)
                .and_then(FormItem::items_mut)
                .ok_or("Hedef öğe formda yok.")?
                .push(item);
        }
        Place::Before | Place::After => {
            let (xs, at) = holder_mut(&mut body.items, target).ok_or("Hedef öğe formda yok.")?;
            xs.insert(if place == Place::After { at + 1 } else { at }, item);
        }
    }
    Ok(())
}

/// The id of the item at `index` in the tree's order (depth first).
pub(crate) fn id_at(body: &FormBody, index: usize) -> Option<String> {
    ids(&body.items).into_iter().nth(index)
}

/// Two copies of a form's draft are the same form, whatever their revision.
fn same_form(a: &FormBody, b: &FormBody) -> bool {
    a.items == b.items && a.clear_hidden == b.clear_hidden
}

fn remove(body: &mut FormBody, id: &str) {
    if let Some((xs, at)) = holder_mut(&mut body.items, id) {
        xs.remove(at);
    }
}

/// The issues of the body over the layer's fields.
impl App {
    /// The issues of the copy over the layer's fields, the project's `@` values and the other layers
    /// it reads known (docs/adr/0404).
    pub(super) fn issues_of(&self, body: &FormBody, fields: &[LayerField]) -> Vec<FormIssue> {
        let world = self
            .document
            .as_ref()
            .map(|d| d.model.form_world(body))
            .unwrap_or_default();
        kentos_forms::preflight_in(body, fields, &self.expression_variables(), &world)
    }

    /// Formlar (`data.forms`): the window, on `form` or the first.
    pub(crate) fn open_forms(&mut self, form: Option<String>) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let mut w = Window::default();
        let chosen = form.or_else(|| self.project_forms().first().map(|f| f.id.clone()));
        if let Some(id) = chosen {
            self.choose_into(&mut w, &id);
        }
        self.forms_window = Some(w);
        self.dialog = Some(Dialog::Forms);
    }

    /// Formlar… on a layer: its first form, or a new one for it.
    pub(crate) fn open_forms_for_layer(&mut self, layer: &str) {
        let form = self
            .project_forms()
            .iter()
            .find(|f| f.layer == layer)
            .map(|f| f.id.clone());
        let new = form.is_none();
        self.open_forms(form);
        if new && let Some(w) = self.forms_window.as_mut() {
            w.new_form = Some(NewForm {
                name: String::new(),
                layer: Some(layer.to_owned()),
            });
        }
    }

    fn choose_into(&self, w: &mut Window, id: &str) {
        if let Some(f) = self.project_forms().iter().find(|f| f.id == id) {
            w.chosen = Some(f.id.clone());
            w.body = Some(f.draft.clone());
            w.name = f.name.clone();
            w.item = None;
            w.dirty = false;
            w.new_form = None;
            w.undo.clear();
            w.redo.clear();
            w.typing = None;
        }
    }

    /// The chosen form's layer's subtype field (its name) and its subtypes' codes, in their order.
    pub(crate) fn forms_sorter(&self) -> Option<(String, Vec<String>)> {
        let form = self.chosen_form()?;
        let node = self.document.as_ref()?.model.layers().get(&form.layer)?;
        let subtypes = node.subtypes.as_ref()?;
        let sorter = subtypes.sorter(&node.fields)?;
        Some((
            sorter.name.clone(),
            subtypes.types.iter().map(|t| t.code.clone()).collect(),
        ))
    }

    pub(crate) fn chosen_form(&self) -> Option<&ProjectForm> {
        let id = self.forms_window.as_ref()?.chosen.as_deref()?;
        self.project_forms().iter().find(|f| f.id == id)
    }

    /// Hesapları yenile (docs/adr/0421): `cad.forms.recompute` through the form's latest release, its
    /// draft when it has none; one undo step.
    fn recompute_form(&mut self, form: &ProjectForm) {
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = kentos_contracts::FormsRecompute {
            form: form.id.clone(),
            draft: form.releases.is_empty(),
            moment: None,
            user: None,
            expected_revision: None,
        };
        let result = kentos_native_application::forms_recompute::execute(
            &mut ExecutionContext::new(&mut doc.model),
            input,
        );
        if let Some(done) = self.answered(result) {
            let said = match (done.changed.len(), done.failed.len()) {
                (0, 0) => format!(
                    "“{}” formunun hesapları güncel: {} kayıt değişmedi.",
                    form.name, done.checked
                ),
                (n, 0) => format!(
                    "“{}” formuyla {n} / {} kaydın hesabı yenilendi.",
                    form.name, done.checked
                ),
                (n, f) => format!(
                    "“{}” formuyla {n} / {} kaydın hesabı yenilendi; {f} kayıt hesabı sığmadığı için olduğu gibi kaldı.",
                    form.name, done.checked
                ),
            };
            let level = if done.failed.is_empty() {
                Level::Success
            } else {
                Level::Warn
            };
            self.say(level, said);
        }
    }

    /// `cad.forms.edit` on the open drawing.
    pub(crate) fn edit_forms(&mut self, edit: FormsOp) -> Option<FormsEdited> {
        if let Some(locked) = self.forms_locked() {
            self.warn(locked);
            return None;
        }
        let doc = self.document.as_mut()?;
        let result = forms_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            FormsEdit {
                edit,
                expected_revision: None,
            },
        );
        self.answered(result)
    }

    /// Writes the window's copy and its name; whether everything went in.
    fn save_form(&mut self) -> bool {
        let Some(w) = self.forms_window.clone() else {
            return false;
        };
        let (Some(id), Some(body)) = (w.chosen.clone(), w.body.clone()) else {
            return false;
        };
        let Some(form) = self.chosen_form().cloned() else {
            return false;
        };
        if w.dirty
            && self
                .edit_forms(FormsOp::Update {
                    id: id.clone(),
                    body: FormBody {
                        revision: form.draft.revision,
                        ..body
                    },
                })
                .is_none()
        {
            return false;
        }
        if w.name.trim() != form.name
            && self
                .edit_forms(FormsOp::Rename {
                    id: id.clone(),
                    name: w.name.clone(),
                })
                .is_none()
        {
            return false;
        }
        // The copy follows the draft as it was kept.
        let mut fresh = w;
        self.choose_into(&mut fresh, &id);
        fresh.item = self.forms_window.as_ref().and_then(|w| w.item.clone());
        self.forms_window = Some(fresh);
        true
    }

    /// An event of the window, its copy's change kept in the history (docs/adr/0403 §1).
    pub(crate) fn forms_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Undo => return self.forms_history(true),
            Event::Redo => return self.forms_history(false),
            Event::Export => return self.export_form(),
            Event::Exported(path) => {
                self.form_exported(path);
                return Task::none();
            }
            Event::Import => return self.import_form(),
            Event::Imported(read) => {
                self.form_imported(read);
                return Task::none();
            }
            _ => {}
        }
        let typing = match &event {
            Event::Text(which, _) => Some(format!(
                "{:?}:{which:?}",
                self.forms_window.as_ref().and_then(|w| w.item.clone())
            )),
            Event::RuleText(rule, _) => Some(format!("{rule:?}")),
            _ => None,
        };
        let before = self
            .forms_window
            .as_ref()
            .and_then(|w| Some((w.chosen.clone(), w.body.clone()?)));
        let task = self.forms_step(event);
        self.refresh_preview();
        if let Some(w) = self.forms_window.as_mut() {
            match before {
                Some((chosen, old))
                    if w.dirty
                        && w.chosen == chosen
                        && w.body.as_ref().is_some_and(|b| !same_form(b, &old)) =>
                {
                    if typing.is_none() || w.typing != typing {
                        w.undo.push(old);
                        if w.undo.len() > HISTORY {
                            w.undo.remove(0);
                        }
                    }
                    w.redo.clear();
                    w.typing = typing;
                }
                _ => {
                    if typing.is_none() {
                        w.typing = None;
                    }
                }
            }
        }
        task
    }

    /// Geri al (`back`) or Yinele on the copy.
    fn forms_history(&mut self, back: bool) -> Task<Message> {
        let saved = self.chosen_form().map(|f| f.draft.clone());
        let Some(w) = self.forms_window.as_mut() else {
            return Task::none();
        };
        let Some(now) = w.body.clone() else {
            return Task::none();
        };
        let (from, to) = if back {
            (&mut w.undo, &mut w.redo)
        } else {
            (&mut w.redo, &mut w.undo)
        };
        let Some(then) = from.pop() else {
            return Task::none();
        };
        to.push(now);
        w.dirty = saved.as_ref().is_none_or(|s| !same_form(s, &then));
        if w.item
            .as_deref()
            .is_some_and(|id| find(&then.items, id).is_none())
        {
            w.item = None;
        }
        w.body = Some(then);
        w.typing = None;
        self.refresh_preview();
        Task::none()
    }

    /// The preview follows the copy (docs/adr/0403 §3).
    fn refresh_preview(&mut self) {
        let Some(w) = self.forms_window.as_ref() else {
            self.form_preview = None;
            return;
        };
        let (true, Some(body), Some(form)) = (w.previewing, w.body.clone(), w.chosen.clone())
        else {
            self.form_preview = None;
            return;
        };
        let error = self.rebuild_preview(&body, &form).err();
        if error.is_some() {
            self.form_preview = None;
        }
        if let Some(w) = self.forms_window.as_mut() {
            w.preview_error = error;
        }
    }

    fn forms_step(&mut self, event: Event) -> Task<Message> {
        let Some(mut w) = self.forms_window.clone() else {
            return Task::none();
        };
        let fields = w
            .chosen
            .as_deref()
            .and_then(|id| self.project_forms().iter().find(|f| f.id == id))
            .map(|f| self.layer_fields_of(&f.layer))
            .unwrap_or_default();
        let chosen_item = w.item.clone();
        let edited = |w: &mut Window, change: &dyn Fn(&mut FormBody)| {
            if let Some(body) = w.body.as_mut() {
                change(body);
                w.dirty = true;
            }
        };
        match event {
            Event::Choose(id) => {
                if w.dirty && w.chosen.as_deref() != Some(id.as_str()) {
                    self.say(
                        Level::Warn,
                        "Formdaki değişiklikler kaydedilmedi: önce Kaydet’e ya da Vazgeç’e basın.",
                    );
                    return Task::none();
                }
                self.choose_into(&mut w, &id);
            }
            Event::NewForm => {
                let layer = self
                    .document
                    .as_ref()
                    .map(|d| d.model.layers().active().to_owned());
                w.new_form = Some(NewForm {
                    name: String::new(),
                    layer,
                });
            }
            Event::NewName(text) => {
                if let Some(n) = w.new_form.as_mut() {
                    n.name = text;
                }
            }
            Event::NewLayer(layer) => {
                if let Some(n) = w.new_form.as_mut() {
                    n.layer = Some(layer);
                }
            }
            Event::CancelNew => w.new_form = None,
            Event::Create => {
                let Some(n) = w.new_form.clone() else {
                    return Task::none();
                };
                let Some(layer) = n.layer else {
                    self.warn("Formun katmanını seçin.");
                    return Task::none();
                };
                let name = if n.name.trim().is_empty() {
                    format!("{} formu", self.form_layer_name(&layer))
                } else {
                    n.name.clone()
                };
                if let Some(done) = self.edit_forms(FormsOp::Create {
                    id: None,
                    name,
                    layer,
                    body: None,
                }) && let Some(made) = done.form
                {
                    self.say(
                        Level::Success,
                        format!("“{}” formu katmanın alanlarından oluşturuldu.", made.name),
                    );
                    self.choose_into(&mut w, &made.id);
                }
            }
            Event::SelectItem(id) => w.item = Some(id),
            Event::AddTab => {
                let mut made = None;
                if let Some(body) = w.body.as_mut() {
                    made = Some(add_tab(body));
                    w.dirty = true;
                }
                w.item = made;
            }
            Event::AddGroup => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("grup", &taken);
                let item = FormItem::Group {
                    id: id.clone(),
                    label: Some("Grup".into()),
                    columns: None,
                    visible_when: None,
                    collapsed: false,
                    items: Vec::new(),
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddNote => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("not", &taken);
                let item = FormItem::Note {
                    id: id.clone(),
                    text: "Açıklama".into(),
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddField(key) => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let name = fields
                    .iter()
                    .find(|f| f.key() == key)
                    .map_or(key.as_str(), |f| f.name.as_str());
                let id = kentos_forms::item_id_of(name, &taken);
                let mut field = FormField::new(id.clone(), key.clone());
                field.required = fields.iter().any(|f| f.key() == key && f.required);
                let item = FormItem::Field(field);
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddRelated => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("iliskili", &taken);
                let item = FormItem::Related {
                    id: id.clone(),
                    label: Some("İlişkili kayıtlar".into()),
                    layer: String::new(),
                    field: String::new(),
                    visible_when: None,
                    view: kentos_contracts::RelatedView::List,
                    columns: Vec::new(),
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddAttachments => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("ekler", &taken);
                let item = FormItem::Attachments {
                    id: id.clone(),
                    label: Some("Ekler".into()),
                    accept: kentos_contracts::AttachmentAccept::Any,
                    min: None,
                    max: None,
                    visible_when: None,
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::AddLocation => {
                let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                let id = kentos_forms::item_id_of("konum", &taken);
                let item = FormItem::Location {
                    id: id.clone(),
                    label: Some("Konum".into()),
                    visible_when: None,
                };
                edited(&mut w, &|b| place(b, chosen_item.as_deref(), item.clone()));
                w.item = Some(id);
            }
            Event::Accept(to) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Attachments { accept, .. }) =
                            find_mut(&mut b.items, &id)
                        {
                            *accept = to;
                        }
                    });
                }
            }
            Event::CopyItem => {
                if let Some(item) = chosen_item
                    .as_deref()
                    .and_then(|id| w.body.as_ref().and_then(|b| find(&b.items, id)))
                {
                    self.form_clipboard = Some((item.clone(), fields.clone()));
                    self.say(
                        Level::Info,
                        "Öğe kopyalandı; bu ya da başka bir formda Yapıştır'a basın.",
                    );
                }
            }
            Event::PasteItem => {
                if let Some((item, from)) = self.form_clipboard.clone()
                    && let Some(body) = w.body.as_ref()
                {
                    match pasted(body, item, &from, &fields) {
                        Ok(item) => {
                            let id = item.id().to_owned();
                            let tab = matches!(item, FormItem::Tab { .. });
                            edited(&mut w, &|b| {
                                if tab {
                                    b.items.push(item.clone());
                                } else {
                                    place(b, chosen_item.as_deref(), item.clone());
                                }
                            });
                            w.item = Some(id);
                        }
                        Err(why) => self.warn(why),
                    }
                }
            }
            Event::AddMirror => {
                if let Some(id) = chosen_item.clone() {
                    let taken = w.body.as_ref().map(|b| ids(&b.items)).unwrap_or_default();
                    let source = w.body.as_ref().and_then(|b| match find(&b.items, &id) {
                        Some(FormItem::Field(f)) if !f.mirror => Some(f.clone()),
                        _ => None,
                    });
                    if let Some(f) = source {
                        let mirror_id =
                            kentos_forms::item_id_of(&format!("{}-yansima", f.id), &taken);
                        let mut m =
                            kentos_contracts::FormField::new(mirror_id.clone(), f.field.clone());
                        m.label = f.label.clone();
                        m.mirror = true;
                        let item = FormItem::Field(m);
                        edited(&mut w, &|b| place(b, Some(id.as_str()), item.clone()));
                        w.item = Some(mirror_id);
                    }
                }
            }
            Event::Suggest(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.suggest = on;
                        }
                    });
                }
            }
            Event::LookupLayer(layer) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.lookup = layer.clone().map(|layer| kentos_contracts::FormLookup {
                                layer,
                                key: kentos_contracts::LOOKUP_UID.into(),
                                label: None,
                                matches: Vec::new(),
                                filter: None,
                            });
                        }
                    });
                }
            }
            Event::LookupKey(key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                        {
                            l.key = key.clone();
                        }
                    });
                }
            }
            Event::LookupLabel(label) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                        {
                            l.label = label.clone();
                        }
                    });
                }
            }
            Event::AddMatch => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                        {
                            l.matches.push(kentos_contracts::LookupMatch {
                                field: String::new(),
                                record: String::new(),
                            });
                        }
                    });
                }
            }
            Event::MatchField(i, key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(m) = f.lookup.as_mut().and_then(|l| l.matches.get_mut(i))
                        {
                            m.field = key.clone();
                        }
                    });
                }
            }
            Event::MatchRecord(i, key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(m) = f.lookup.as_mut().and_then(|l| l.matches.get_mut(i))
                        {
                            m.record = key.clone();
                        }
                    });
                }
            }
            Event::RemoveMatch(i) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(l) = f.lookup.as_mut()
                            && i < l.matches.len()
                        {
                            l.matches.remove(i);
                        }
                    });
                }
            }
            Event::RelatedLayer(layer) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related {
                            layer: l, field, ..
                        }) = find_mut(&mut b.items, &id)
                        {
                            *l = layer.clone();
                            field.clear();
                        }
                        if let Some(FormItem::Related { columns, .. }) = find_mut(&mut b.items, &id)
                        {
                            columns.clear();
                        }
                    });
                }
            }
            Event::ShowInKind(code) => {
                if let (Some(id), Some(sorter)) = (chosen_item.clone(), self.forms_sorter()) {
                    let rule = Rule::Visible(id);
                    let now = w.body.as_ref().and_then(|b| rule_text(b, &rule));
                    if let Some(mut kinds) = kinds_of_rule(now.as_deref(), &sorter.0) {
                        if let Some(i) = kinds.iter().position(|k| *k == code) {
                            kinds.remove(i);
                        } else {
                            kinds.push(code);
                            kinds.sort_by_key(|k| sorter.1.iter().position(|c| c == k));
                        }
                        let text = rule_of_kinds(&kinds, &sorter.0);
                        edited(&mut w, &|b| set_rule(b, &rule, &text));
                    }
                }
            }
            Event::RelatedView(view) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related { view: v, .. }) = find_mut(&mut b.items, &id)
                        {
                            *v = view;
                        }
                    });
                }
            }
            Event::RelatedColumn(key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related { columns, .. }) = find_mut(&mut b.items, &id)
                        {
                            if let Some(i) = columns.iter().position(|c| *c == key) {
                                columns.remove(i);
                            } else if columns.len() < kentos_contracts::MOST_RELATED_COLUMNS {
                                columns.push(key.clone());
                            }
                        }
                    });
                }
            }
            Event::RelatedField(key) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Related { field, .. }) = find_mut(&mut b.items, &id) {
                            *field = key.clone();
                        }
                    });
                }
            }
            Event::Move(by) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| shift(b, &id, by));
                }
            }
            Event::Drop(from, to, place) => {
                let ids = w.body.as_ref().map(|b| (id_at(b, from), id_at(b, to)));
                if let Some((Some(source), Some(target))) = ids
                    && let Some(body) = w.body.as_mut()
                {
                    let mut moved = body.clone();
                    match move_item(&mut moved, &source, &target, place) {
                        Ok(()) => {
                            *body = moved;
                            w.dirty = true;
                            w.item = Some(source);
                        }
                        Err(why) => self.say(Level::Warn, why),
                    }
                }
            }
            Event::Fold(id) => {
                if !w.folded.remove(&id) {
                    w.folded.insert(id);
                }
            }
            Event::Undo
            | Event::Redo
            | Event::Export
            | Event::Exported(_)
            | Event::Import
            | Event::Imported(_) => {}
            Event::Audit => {
                let form = w.chosen.clone();
                if w.dirty {
                    self.say(
                        Level::Warn,
                        "Kayıtlar formun kaydedilmiş hâliyle denetlenir: önce Kaydet’e basın.",
                    );
                    self.forms_window = Some(w);
                    return Task::none();
                }
                self.forms_window = None;
                self.form_preview = None;
                self.dialog = None;
                return self.open_form_audit(form);
            }
            Event::Recompute => {
                if w.dirty {
                    self.say(
                        Level::Warn,
                        "Hesaplar formun kaydedilmiş hâliyle yenilenir: önce Kaydet’e basın.",
                    );
                } else if let Some(form) = self.chosen_form().cloned() {
                    self.forms_window = Some(w);
                    self.recompute_form(&form);
                    return Task::none();
                }
            }
            Event::Changes => {
                w.changes = true;
                w.flow = false;
                w.previewing = false;
                self.form_preview = None;
            }
            Event::Flow => {
                w.flow = true;
                w.changes = false;
                w.previewing = false;
                self.form_preview = None;
            }
            Event::WorkflowAdd => {
                let layer = self
                    .chosen_form()
                    .map(|f| f.layer.clone())
                    .unwrap_or_default();
                let fields = self.layer_fields_of(&layer);
                let listed = fields
                    .iter()
                    .find(|f| f.values.as_ref().is_some_and(|v| v.len() >= 2) && !f.holds_many());
                match listed {
                    None => self.say(
                        Level::Warn,
                        "İş akışı için katmanda iki ya da daha çok değerli, tek değerli bir liste alanı gerekir (Taslak, Gönderildi, Onaylandı gibi).",
                    ),
                    Some(f) => {
                        let codes: Vec<String> =
                            f.values.iter().flatten().map(|c| c.code.clone()).collect();
                        let workflow = kentos_contracts::FormWorkflow {
                            field: f.key().to_owned(),
                            initial: codes[0].clone(),
                            locked: Vec::new(),
                            steps: vec![kentos_contracts::WorkflowStep {
                                label: "Gönder".into(),
                                from: vec![codes[0].clone()],
                                to: codes[1].clone(),
                                valid: true,
                                reason: None,
                            }],
                            log: None,
                            drafts: Vec::new(),
                        };
                        edited(&mut w, &|b| b.workflow = Some(workflow.clone()));
                    }
                }
            }
            Event::WorkflowRemove => edited(&mut w, &|b| b.workflow = None),
            Event::WorkflowField(key) => {
                let layer = self
                    .chosen_form()
                    .map(|f| f.layer.clone())
                    .unwrap_or_default();
                let first = self
                    .layer_fields_of(&layer)
                    .iter()
                    .find(|f| f.key() == key)
                    .and_then(|f| f.values.as_ref()?.first().map(|c| c.code.clone()))
                    .unwrap_or_default();
                edited(&mut w, &|b| {
                    if let Some(wf) = b.workflow.as_mut() {
                        wf.field = key.clone();
                        wf.initial = first.clone();
                        wf.locked.clear();
                        for s in &mut wf.steps {
                            s.from.clear();
                            s.to = first.clone();
                        }
                    }
                });
            }
            Event::WorkflowInitial(code) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    wf.initial = code.clone();
                }
            }),
            Event::WorkflowLocked(code) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    toggle(&mut wf.locked, &code);
                }
            }),
            Event::WorkflowDraft(code) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    toggle(&mut wf.drafts, &code);
                }
            }),
            Event::WorkflowLog(key) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    wf.log = key.clone();
                }
            }),
            Event::StepAdd => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut() {
                    let n = wf.steps.len() + 1;
                    wf.steps.push(kentos_contracts::WorkflowStep {
                        label: format!("Adım {n}"),
                        from: Vec::new(),
                        to: wf.initial.clone(),
                        valid: false,
                        reason: None,
                    });
                }
            }),
            Event::StepRemove(i) => edited(&mut w, &|b| {
                if let Some(wf) = b.workflow.as_mut()
                    && i < wf.steps.len()
                {
                    wf.steps.remove(i);
                }
            }),
            Event::StepLabel(i, text) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    s.label = text.clone();
                }
            }),
            Event::StepFrom(i, code) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    toggle(&mut s.from, &code);
                }
            }),
            Event::StepTo(i, code) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    s.to = code.clone();
                }
            }),
            Event::StepValid(i, on) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    s.valid = on;
                }
            }),
            Event::StepReason(i, key) => edited(&mut w, &|b| {
                if let Some(s) = b.workflow.as_mut().and_then(|wf| wf.steps.get_mut(i)) {
                    s.reason = key.clone();
                }
            }),
            Event::Preview(on) => {
                w.changes = false;
                w.flow = false;
                w.previewing = on;
                if !on {
                    self.form_preview = None;
                }
            }
            Event::RemoveItem => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| remove(b, &id));
                    w.item = None;
                }
            }
            Event::Text(which, text) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| set_text(b, &id, &which, &text));
                }
            }
            Event::Widget(widget) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.widget = widget;
                        }
                    });
                }
            }
            Event::Required(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.required = on;
                        }
                    });
                }
            }
            Event::ReadOnly(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.read_only = on;
                        }
                    });
                }
            }
            Event::Unique(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.unique = on;
                        }
                    });
                }
            }
            Event::ScenarioName(text) => w.scenario_name = text,
            Event::RemoveScenario(i) => {
                edited(&mut w, &|b| {
                    if i < b.tests.len() {
                        b.tests.remove(i);
                    }
                });
            }
            Event::SaveScenario => {
                let typed = self.form_preview.as_ref().map(|e| e.typed());
                let form = self.chosen_form().cloned();
                if let (Some((values, new)), Some(form), Some(body), Some(doc)) =
                    (typed, form, w.body.clone(), self.document.as_ref())
                {
                    let name = match w.scenario_name.trim() {
                        "" => format!("Senaryo {}", body.tests.len() + 1),
                        n => n.to_owned(),
                    };
                    match kentos_native_application::forms_test::prepared_for(
                        &doc.model, &form, &body,
                    ) {
                        Ok(prepared) if !body.tests.iter().any(|t| t.name == name) => {
                            let test =
                                kentos_forms::scenario::capture(&prepared, &name, values, new);
                            edited(&mut w, &|b| b.tests.push(test.clone()));
                            w.scenario_name.clear();
                        }
                        Ok(_) => self.say(
                            Level::Warn,
                            format!("“{name}” adlı bir senaryo var; başka bir ad verin."),
                        ),
                        Err(why) => self.say(Level::Warn, why),
                    }
                }
            }
            Event::Overridable(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.overridable = on;
                        }
                    });
                }
            }
            Event::ReuseLast(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.reuse_last = on;
                        }
                    });
                }
            }
            Event::CheckWarning(i, on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(c) = f.checks.get_mut(i)
                        {
                            c.warning = on;
                        }
                    });
                }
            }
            Event::CheckMoment(i, on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && let Some(c) = f.checks.get_mut(i)
                        {
                            c.on = on;
                        }
                    });
                }
            }
            Event::Collapsed(on) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Group { collapsed, .. }) = find_mut(&mut b.items, &id)
                        {
                            *collapsed = on;
                        }
                    });
                }
            }
            Event::ClearHidden(on) => {
                edited(&mut w, &|b| b.clear_hidden = on);
            }
            Event::Steps(on) => {
                edited(&mut w, &|b| b.steps = on);
            }
            Event::Columns(n) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Group { columns, .. }) = find_mut(&mut b.items, &id) {
                            *columns = (n == 2).then_some(2);
                        }
                    });
                }
            }
            Event::RuleText(rule, text) => {
                edited(&mut w, &|b| set_rule(b, &rule, &text));
            }
            Event::Builder(rule) => {
                let value = w
                    .body
                    .as_ref()
                    .and_then(|b| rule_text(b, &rule))
                    .unwrap_or_default();
                let layer = self.chosen_form().map(|f| f.layer.clone());
                self.forms_window = Some(w);
                return match layer {
                    Some(layer) => self.open_builder_for_form(rule, &layer, &value),
                    None => Task::none(),
                };
            }
            Event::AddCheck => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id) {
                            f.checks.push(FormCheck {
                                rule: String::new(),
                                message: String::new(),
                                warning: false,
                                on: None,
                            });
                        }
                    });
                }
            }
            Event::RemoveCheck(i) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| {
                        if let Some(FormItem::Field(f)) = find_mut(&mut b.items, &id)
                            && i < f.checks.len()
                        {
                            f.checks.remove(i);
                        }
                    });
                }
            }
            Event::Name(text) => {
                w.name = text;
            }
            Event::Save => {
                self.forms_window = Some(w);
                let name = self.forms_window.as_ref().map(|w| w.name.trim().to_owned());
                if self.save_form() {
                    self.forms_window = None;
                    self.dialog = None;
                    self.say(
                        Level::Success,
                        format!("“{}” formu kaydedildi.", name.unwrap_or_default()),
                    );
                }
                return Task::none();
            }
            Event::Publish => {
                self.forms_window = Some(w);
                if !self.save_form() {
                    return Task::none();
                }
                let id = self
                    .forms_window
                    .as_ref()
                    .and_then(|w| w.chosen.clone())
                    .unwrap_or_default();
                if let Some(done) = self.edit_forms(FormsOp::Publish {
                    id: id.clone(),
                    note: None,
                }) && let Some(f) = done.form
                {
                    let version = f.releases.last().map_or(0, |r| r.version);
                    let said = if done.changed {
                        format!("“{}” formu yayınlandı: {version}. sürüm.", f.name)
                    } else {
                        format!(
                            "“{}” formunun taslağı son yayınla aynı ({version}. sürüm).",
                            f.name
                        )
                    };
                    self.say(Level::Success, said);
                }
                return Task::none();
            }
            Event::Enforce(on) => {
                if let Some(id) = w.chosen.clone()
                    && self.edit_forms(FormsOp::Enforce { id, on }).is_some()
                {
                    let said = if on {
                        "Formun son yayını katmanın öznitelik yazmalarında uygulanacak."
                    } else {
                        "Formun kuralları artık yalnız formla yazarken uygulanır."
                    };
                    self.say(Level::Success, said);
                }
            }
            Event::RemoveForm => {
                if let Some(id) = w.chosen.clone() {
                    let name = w.name.clone();
                    if self.edit_forms(FormsOp::Remove { id }).is_some() {
                        self.say(Level::Success, format!("“{name}” formu silindi."));
                        w = Window::default();
                        if let Some(first) = self.project_forms().first().map(|f| f.id.clone()) {
                            self.choose_into(&mut w, &first);
                        }
                    }
                }
            }
            Event::Cancel => {
                self.forms_window = None;
                self.form_preview = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.forms_window = Some(w);
        Task::none()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn forms_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.forms_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Fill(NAME, text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(PUBLISH) => Some(msg(Event::Publish)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW_FORM) => Some(msg(Event::NewForm)),
            Control::Press(CREATE) => Some(msg(Event::Create)),
            Control::Press(ADD_TAB) => Some(msg(Event::AddTab)),
            Control::Press(ADD_GROUP) => Some(msg(Event::AddGroup)),
            Control::Press(ADD_NOTE) => Some(msg(Event::AddNote)),
            Control::Press(ADD_RELATED) => Some(msg(Event::AddRelated)),
            Control::Press(ADD_ATTACHMENTS) => Some(msg(Event::AddAttachments)),
            Control::Press(AUDIT) => Some(msg(Event::Audit)),
            Control::Press(ENFORCE) => self.chosen_form().map(|f| msg(Event::Enforce(!f.enforce))),
            Control::Press(UNDO) => Some(msg(Event::Undo)),
            Control::Press(REDO) => Some(msg(Event::Redo)),
            Control::Press(words) => {
                if let Some(f) = self.project_forms().iter().find(|f| f.name == words) {
                    Some(msg(Event::Choose(f.id.clone())))
                } else if let Some(id) = w.body.as_ref().and_then(|b| item_named(&b.items, words)) {
                    Some(msg(Event::SelectItem(id)))
                } else {
                    return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok"));
                }
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }

    /// The window's keys (docs/adr/0403 §1): Ctrl+Z and Ctrl+Y step through the copy's history,
    /// Ctrl+S is Kaydet.
    pub(crate) fn forms_key(&mut self, press: &crate::keys::KeyPress) -> Option<Task<Message>> {
        match crate::keys::chord(press).as_deref() {
            Some("Ctrl+Z") => Some(self.forms_event(Event::Undo)),
            Some("Ctrl+Y" | "Ctrl+Shift+Z") => Some(self.forms_event(Event::Redo)),
            Some("Ctrl+S") => Some(self.forms_event(Event::Save)),
            // The chosen item moved with the keyboard (docs/adr/0437), as Yukarı and Aşağı move it.
            Some("Alt+Up") if self.forms_window.as_ref().is_some_and(|w| w.item.is_some()) => {
                Some(self.forms_event(Event::Move(-1)))
            }
            Some("Alt+Down") if self.forms_window.as_ref().is_some_and(|w| w.item.is_some()) => {
                Some(self.forms_event(Event::Move(1)))
            }
            _ => None,
        }
    }

    /// The expression builder's Tamam: a rule's text back into the window's copy.
    pub(crate) fn form_rule_written(&mut self, rule: Rule, text: String) -> Task<Message> {
        self.forms_event(Event::RuleText(rule, text))
    }
}

/// The item whose id or label is `words`.
fn item_named(items: &[FormItem], words: &str) -> Option<String> {
    items.iter().find_map(|i| {
        let label = match i {
            FormItem::Tab { label, .. } => Some(label.as_str()),
            FormItem::Group { label, .. } => label.as_deref(),
            FormItem::Field(f) => f.label.as_deref().or(Some(f.field.as_str())),
            FormItem::Note { .. } => None,
            FormItem::Related { label, .. }
            | FormItem::Attachments { label, .. }
            | FormItem::Location { label, .. } => label.as_deref(),
        };
        if i.id() == words || label == Some(words) {
            Some(i.id().to_owned())
        } else {
            item_named(i.items(), words)
        }
    })
}

fn set_text(body: &mut FormBody, id: &str, which: &Text, text: &str) {
    let some = |t: &str| (!t.trim().is_empty()).then(|| t.to_owned());
    match find_mut(&mut body.items, id) {
        Some(FormItem::Tab { label, .. }) if *which == Text::Label => *label = text.to_owned(),
        Some(FormItem::Group { label, .. }) if *which == Text::Label => *label = some(text),
        Some(FormItem::Related { label, .. }) | Some(FormItem::Location { label, .. })
            if *which == Text::Label =>
        {
            *label = some(text)
        }
        Some(FormItem::Attachments {
            label, min, max, ..
        }) => {
            // A count that is not a whole number leaves the bound off.
            let count = || text.trim().parse::<u32>().ok();
            match which {
                Text::Label => *label = some(text),
                Text::AttachMin => *min = count(),
                Text::AttachMax => *max = count(),
                _ => {}
            }
        }
        Some(FormItem::Note { text: t, .. }) if *which == Text::Note => *t = text.to_owned(),
        Some(FormItem::Field(f)) => match which {
            Text::Label => f.label = some(text),
            Text::Help => f.help = some(text),
            Text::Placeholder => f.placeholder = some(text),
            Text::Default => f.default = some(text),
            Text::CheckMessage(i) => {
                if let Some(c) = f.checks.get_mut(*i) {
                    c.message = text.to_owned();
                }
            }
            _ => {}
        },
        _ => {}
    }
}

/// `code` in `list`, or out of it.
fn toggle(list: &mut Vec<String>, code: &str) {
    if let Some(i) = list.iter().position(|c| c == code) {
        list.remove(i);
    } else {
        list.push(code.to_owned());
    }
}

/// The subtypes a visibility rule shows the item in, when it is the one the designer writes for them
/// (`[Tür] içinde ('a', 'b')`, docs/adr/0426); none for another rule; empty for no rule.
pub(crate) fn kinds_of_rule(rule: Option<&str>, sorter: &str) -> Option<Vec<String>> {
    let Some(rule) = rule.map(str::trim).filter(|r| !r.is_empty()) else {
        return Some(Vec::new());
    };
    let inner = rule
        .strip_prefix(&format!("[{sorter}] içinde ("))?
        .strip_suffix(')')?;
    inner
        .split(", ")
        .map(|c| {
            c.strip_prefix('\'')
                .and_then(|c| c.strip_suffix('\''))
                .filter(|c| !c.contains('\''))
                .map(str::to_owned)
        })
        .collect()
}

/// The visibility rule that shows an item in `kinds` (none: always).
pub(crate) fn rule_of_kinds(kinds: &[String], sorter: &str) -> String {
    if kinds.is_empty() {
        return String::new();
    }
    let codes: Vec<String> = kinds.iter().map(|c| format!("'{c}'")).collect();
    format!("[{sorter}] içinde ({})", codes.join(", "))
}

fn rule_text(body: &FormBody, rule: &Rule) -> Option<String> {
    match (find(&body.items, rule.item())?, rule) {
        (FormItem::Group { visible_when, .. }, Rule::Visible(_))
        | (FormItem::Tab { visible_when, .. }, Rule::Visible(_)) => visible_when.clone(),
        (FormItem::Related { visible_when, .. }, Rule::Visible(_))
        | (FormItem::Attachments { visible_when, .. }, Rule::Visible(_))
        | (FormItem::Location { visible_when, .. }, Rule::Visible(_)) => visible_when.clone(),
        (FormItem::Field(f), Rule::LookupFilter(_)) => f.lookup.as_ref()?.filter.clone(),
        (FormItem::Field(f), Rule::Visible(_)) => f.visible_when.clone(),
        (FormItem::Field(f), Rule::Required(_)) => f.required_when.clone(),
        (FormItem::Field(f), Rule::ReadOnly(_)) => f.read_only_when.clone(),
        (FormItem::Field(f), Rule::Check(_, i)) => f.checks.get(*i).map(|c| c.rule.clone()),
        (FormItem::Field(f), Rule::Default(_)) => f.default_expr.clone(),
        (FormItem::Field(f), Rule::Calculate(_)) => f.calculate.clone(),
        (FormItem::Field(f), Rule::ChoiceFilter(_)) => f.choice_filter.clone(),
        _ => None,
    }
}

fn set_rule(body: &mut FormBody, rule: &Rule, text: &str) {
    let value = (!text.trim().is_empty()).then(|| text.to_owned());
    match (find_mut(&mut body.items, rule.item()), rule) {
        (Some(FormItem::Group { visible_when, .. }), Rule::Visible(_))
        | (Some(FormItem::Tab { visible_when, .. }), Rule::Visible(_)) => *visible_when = value,
        (Some(FormItem::Related { visible_when, .. }), Rule::Visible(_))
        | (Some(FormItem::Attachments { visible_when, .. }), Rule::Visible(_))
        | (Some(FormItem::Location { visible_when, .. }), Rule::Visible(_)) => {
            *visible_when = value
        }
        (Some(FormItem::Field(f)), Rule::LookupFilter(_)) => {
            if let Some(l) = f.lookup.as_mut() {
                l.filter = value;
            }
        }
        (Some(FormItem::Field(f)), Rule::Visible(_)) => f.visible_when = value,
        (Some(FormItem::Field(f)), Rule::Required(_)) => f.required_when = value,
        (Some(FormItem::Field(f)), Rule::ReadOnly(_)) => f.read_only_when = value,
        (Some(FormItem::Field(f)), Rule::Default(_)) => f.default_expr = value,
        (Some(FormItem::Field(f)), Rule::Calculate(_)) => f.calculate = value,
        (Some(FormItem::Field(f)), Rule::ChoiceFilter(_)) => f.choice_filter = value,
        (Some(FormItem::Field(f)), Rule::Check(_, i)) => {
            if let Some(c) = f.checks.get_mut(*i) {
                c.rule = text.to_owned();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> FormBody {
        kentos_forms::generate(&[
            LayerField::new("Durum", kentos_contracts::LayerFieldKind::Text),
            LayerField::new("Kat", kentos_contracts::LayerFieldKind::Integer),
        ])
    }

    #[test]
    fn a_first_tab_gathers_the_top_and_items_go_where_the_chosen_one_is() {
        let mut b = body();
        let tab = add_tab(&mut b);
        assert_eq!(ids(&b.items), ["genel", "durum", "kat", tab.as_str()]);
        place(
            &mut b,
            Some("durum"),
            FormItem::Note {
                id: "n".into(),
                text: "x".into(),
            },
        );
        assert_eq!(ids(&b.items), ["genel", "durum", "n", "kat", tab.as_str()]);
        place(
            &mut b,
            Some(&tab),
            FormItem::Note {
                id: "m".into(),
                text: "y".into(),
            },
        );
        assert_eq!(ids(b.items[1].items()), ["m"]);
        shift(&mut b, "kat", -1);
        assert_eq!(ids(b.items[0].items()), ["durum", "kat", "n"]);
        remove(&mut b, "kat");
        assert_eq!(bound(&b.items), ["Durum"]);
        assert!(kentos_contracts::body_problem(&b).is_none());
    }

    /// docs/adr/0403 §2: the drops the form's shape takes, and the ones it does not.
    #[test]
    fn items_are_moved_by_the_form_s_shape() {
        use kentos_ui::widget::tree_view::Place;
        let mut b = body();
        let second = add_tab(&mut b);
        let group = FormItem::Group {
            id: "g".into(),
            label: None,
            columns: None,
            visible_when: None,
            collapsed: false,
            items: Vec::new(),
        };
        place(&mut b, Some("genel"), group);
        // genel[durum, kat, g], second[]
        assert_eq!(move_item(&mut b, "kat", "g", Place::Into), Ok(()));
        assert_eq!(
            ids(&b.items),
            ["genel", "durum", "g", "kat", second.as_str()]
        );
        assert_eq!(move_item(&mut b, "durum", &second, Place::Into), Ok(()));
        assert_eq!(
            ids(&b.items),
            ["genel", "g", "kat", second.as_str(), "durum"]
        );
        assert_eq!(move_item(&mut b, &second, "genel", Place::Before), Ok(()));
        assert_eq!(ids(&b.items)[0], second);
        assert!(
            move_item(&mut b, "g", "kat", Place::Into).is_err(),
            "into a field"
        );
        assert!(
            move_item(&mut b, "g", "kat", Place::After).is_err(),
            "into itself"
        );
        assert!(
            move_item(&mut b, "kat", "genel", Place::After).is_err(),
            "beside a tab"
        );
        assert!(
            move_item(&mut b, "genel", "g", Place::Into).is_err(),
            "a tab into a group"
        );
        assert!(
            move_item(&mut b, "genel", "kat", Place::Before).is_err(),
            "a tab deep"
        );
        assert!(kentos_contracts::body_problem(&b).is_none());
        assert_eq!(id_at(&b, 0).as_deref(), Some(second.as_str()));
    }

    #[test]
    fn a_rule_is_written_and_read_back_and_an_empty_one_is_no_rule() {
        let mut b = body();
        let rule = Rule::Visible("kat".into());
        set_rule(&mut b, &rule, "Durum = 'iyi'");
        assert_eq!(rule_text(&b, &rule).as_deref(), Some("Durum = 'iyi'"));
        set_rule(&mut b, &rule, "  ");
        assert_eq!(rule_text(&b, &rule), None);
    }
}

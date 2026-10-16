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
            | Rule::ChoiceFilter(i) => i,
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
}

#[derive(Debug, Clone)]
pub enum Event {
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
    Move(i32),
    RemoveItem,
    Text(Text, String),
    Widget(Option<FormWidget>),
    Required(bool),
    ReadOnly(bool),
    /// A field's value is not repeated on the layer.
    Unique(bool),
    /// A new record starts with the session's last value.
    ReuseLast(bool),
    /// A check's failure is a warning, not an error.
    CheckWarning(usize, bool),
    /// A group starts closed.
    Collapsed(bool),
    /// The fields a record hides are emptied (the form's setting).
    ClearHidden(bool),
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
        });
    }
    let n = body.items.len() + 1;
    let label = format!("Sekme {n}");
    let id = kentos_forms::item_id_of(&label, &taken);
    body.items.push(FormItem::Tab {
        id: id.clone(),
        label,
        items: Vec::new(),
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

fn remove(body: &mut FormBody, id: &str) {
    if let Some((xs, at)) = holder_mut(&mut body.items, id) {
        xs.remove(at);
    }
}

/// The issues of the body over the layer's fields.
pub fn issues_of(body: &FormBody, fields: &[LayerField]) -> Vec<FormIssue> {
    kentos_forms::preflight(body, fields, &[])
}

impl App {
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
        }
    }

    pub(super) fn chosen_form(&self) -> Option<&ProjectForm> {
        let id = self.forms_window.as_ref()?.chosen.as_deref()?;
        self.project_forms().iter().find(|f| f.id == id)
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

    pub(crate) fn forms_event(&mut self, event: Event) -> Task<Message> {
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
            Event::Move(by) => {
                if let Some(id) = chosen_item.clone() {
                    edited(&mut w, &|b| shift(b, &id, by));
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
            Control::Press(ENFORCE) => self.chosen_form().map(|f| msg(Event::Enforce(!f.enforce))),
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

fn rule_text(body: &FormBody, rule: &Rule) -> Option<String> {
    match (find(&body.items, rule.item())?, rule) {
        (FormItem::Group { visible_when, .. }, Rule::Visible(_)) => visible_when.clone(),
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
        (Some(FormItem::Group { visible_when, .. }), Rule::Visible(_)) => *visible_when = value,
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

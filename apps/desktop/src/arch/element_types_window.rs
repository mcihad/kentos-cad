//! Eleman tipleri (docs/adr/8106 §5; plan/bim-plan.md BIM-12; `arch.elementTypes`): the project's column, beam, slab,
//! roof, railing and stair types, each with the elements following it counted (those keeping a value of their own,
//! those at an older version). The chosen type's values are typed in the project's unit; before Uygula the plan of
//! `cad.elementtypes.edit` says what the change does. Yeni tip ▾ starts a type of a kind or makes one from the selected
//! element (Tipe çevir: önce çiz, beğenince tipe çevir); Sil leaves its elements with their values; Aynı tiptekileri seç
//! selects its elements. The selection's Tipe bağla, Tipten ayır, Tipe uygula, Tipe döndür and Tipi yeniden uygula
//! (Öznitelikler's Tip section) come here too. Every write is one undo step.
//!
//! ```text
//! ┌ Eleman tipleri ─────────────────────────────────────────────────────────────────────────────┐
//! │ Kolon, kiriş, döşeme, çatı, korkuluk ve merdiven tipleri …                                     │
//! │ ┌ Tip      Tür     Eleman  İstisna  Eski ┐│ Kolon tipi · sürüm 2                               │
//! │ │ S1       Kolon       17        1     1 ││ Ad          [S1            ]                       │
//! │ │ K1       Kiriş        8        0     0 ││ En          [0,30   ] m                            │
//! │ └────────────────────────────────────────┘│ Boy         [0,50   ] m    …                       │
//! │                                            │ Uygula: 16 eleman yeni değerleri alır, 1 …        │
//! │ [Yeni tip ▾] [Sil] [Aynı tiptekileri seç]                              [Uygula]  [Kapat]   │
//! └──────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::{Column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, ElementType, ElementTypeKind, ElementTypeRefused, ElementTypeReport, ElementTypesEdit,
    ElementTypesEdited, ElementTypesOperation, Entity, TypeField, fields_off_type, type_link,
};
use kentos_domain::Slot;
use kentos_interaction::{Format, Level};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography::scaled;
use kentos_ui::widget::table::{self, Column as Col, Row, Table};
use kentos_ui::widget::{Choice, Dialog as Frame, Form, ListDetails, Menu, MenuButton, Select, text_field};
use kentos_ui::{label, style};
use serde_json::{Map, Value};
use crate::arch::ArchState;

use crate::app::{App, Message};
use crate::arch;
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Eleman tipleri";
const LEAD: &str = "Kolon, kiriş, döşeme, çatı, korkuluk ve merdiven tipleri. Tip değişince onu izleyen elemanlar yeni \
                    değerleri alır; elemanın kendi değeri (istisna) korunur, kilitli katmandaki eski sürümde kalır.";
pub const APPLY: &str = "Uygula";
pub const NEW: &str = "Yeni tip";
/// Yeni tip ▾'s last item: Tipe çevir, the selected element's values a type and the element following it.
pub const FROM: &str = "Seçili elemandan";
pub const REMOVE: &str = "Sil";
pub const SAME: &str = "Aynı tiptekileri seç";
const CLOSE: &str = "Kapat";
const BODY: f32 = 440.0;
const LIST: f32 = 420.0;

/// How a type field is typed in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edit {
    /// A length in the project's unit; required or not (empty: none).
    Length { required: bool },
    /// A whole number; empty: none.
    Count,
    /// Words; empty: none.
    Text,
    /// Evet or Hayır.
    Switch,
    /// One of the kind's words.
    Choice,
    /// Shown, changed on an element and pushed to the type (Tipe uygula): layers, legs, an edge.
    Shown,
}

fn edit_of(kind: ElementTypeKind, f: &TypeField) -> Edit {
    use ElementTypeKind as K;
    match (kind, f.key) {
        (_, "width" | "height" | "thickness" | "tread") => Edit::Length { required: f.required },
        (_, "depth" | "gutter" | "snowGuard" | "gap" | "landing" | "post") => Edit::Length { required: false },
        (_, "winders") => Edit::Count,
        (_, "material") => Edit::Text,
        (_, "fill" | "numbering") => Edit::Switch,
        (K::Column, "shape" | "member")
        | (K::Beam, "role" | "member")
        | (K::Slab, "role")
        | (K::Railing, "infill")
        | (K::Stair, "railings") => Edit::Choice,
        _ => Edit::Shown,
    }
}

/// The words a choice field takes, each with the value it writes (none: the field's absence, its default).
fn choices(kind: ElementTypeKind, key: &str) -> Vec<(&'static str, Option<Value>)> {
    use ElementTypeKind as K;
    let to = |v: Value| Some(v);
    let member = |m: kentos_commands::Member| serde_json::to_value(m).ok();
    match (kind, key) {
        (K::Column, "member") => vec![("Kolon", None), ("Dikme", member(kentos_commands::Member::post()))],
        (K::Beam, "member") => vec![("Kiriş", None), ("Kayıt", member(kentos_commands::Member::rail()))],
        (K::Column, "shape") => kentos_commands::ColumnShape::ALL
            .iter()
            .map(|s| (s.label(), (*s != kentos_commands::ColumnShape::Rect).then(|| Value::from(s.name()))))
            .collect(),
        (K::Beam, "role") => kentos_commands::BeamRole::ALL
            .iter()
            .map(|r| {
                (r.label(), (*r != kentos_commands::BeamRole::Beam).then(|| serde_json::to_value(r).unwrap_or(Value::Null)))
            })
            .collect(),
        (K::Slab, "role") => kentos_commands::SlabRole::ALL
            .iter()
            .map(|r| {
                (r.label(), (*r != kentos_commands::SlabRole::Floor).then(|| serde_json::to_value(r).unwrap_or(Value::Null)))
            })
            .collect(),
        (K::Railing, "infill") => kentos_commands::RailingInfill::ALL
            .iter()
            .map(|i| {
                (
                    i.label(),
                    (*i != kentos_commands::RailingInfill::default()).then(|| serde_json::to_value(i).unwrap_or(Value::Null)),
                )
            })
            .collect(),
        (K::Stair, "railings") => std::iter::once(("Yok", None))
            .chain(kentos_commands::Railings::ALL.iter().map(|r| (r.words(), to(Value::from(r.as_str())))))
            .collect(),
        _ => Vec::new(),
    }
}

/// What a shown field holds, in words.
fn shown_words(key: &str, v: Option<&Value>, f: &Format) -> String {
    let Some(v) = v else {
        return "Yok".into();
    };
    match key {
        "legs" => v
            .as_array()
            .map(|l| l.iter().filter_map(Value::as_f64).map(|m| f.length_bare(m)).collect::<Vec<_>>().join(" / "))
            .unwrap_or_default(),
        "layers" => match v {
            Value::Array(l) => format!("{} katman", l.len()),
            Value::Object(o) => format!(
                "{} katman",
                o.values().map(|x| x.as_array().map_or(usize::from(!x.is_null()), Vec::len)).sum::<usize>()
            ),
            _ => "Var".into(),
        },
        "member" => "Dikme ya da kayıt".into(),
        "covering" => "Kaplama".into(),
        "materialId" => v.as_str().map_or_else(|| "Var".into(), str::to_owned),
        _ => "Var".into(),
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Window {
    /// The chosen type's id.
    pub chosen: Option<String>,
    pub name: String,
    /// Each typed field's words, by its key.
    pub typed: Vec<(&'static str, String)>,
    /// Each chosen field's (a choice's, a switch's) value picked, by its key; none: the field's absence.
    pub picked: Vec<(&'static str, Option<Value>)>,
    /// What the change typed would do; its refusal's words when refused.
    pub preview: Option<Result<ElementTypesEdited, String>>,
    pub said: Option<(Kind, String)>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(String),
    Name(String),
    Value(&'static str, String),
    Pick(&'static str, Option<Value>),
    New(ElementTypeKind),
    FromSelection,
    Apply,
    Remove,
    SelectSame,
    Close,
    /// The selection's elements follow the type (Öznitelikler's Tip).
    Assign(String),
    Detach,
    Push,
    Reset,
    Refresh,
}

fn msg(e: Event) -> Message {
    Message::from(arch::Event::ElementTypes(e))
}

/// A type's use: its elements, those keeping a value of their own, those at an older version.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Use {
    pub elements: usize,
    pub own: usize,
    pub behind: usize,
}

/// The use of the type `t` in `doc`.
pub(crate) fn use_of(doc: &kentos_domain::Document, t: &ElementType) -> Use {
    let mut u = Use::default();
    for e in doc.entities() {
        let Some(l) = type_link(e).filter(|l| l.id == t.id) else { continue };
        u.elements += 1;
        if l.version < t.version {
            u.behind += 1;
            if !l.overrides.is_empty() {
                u.own += 1;
            }
        } else if !fields_off_type(e, t).is_empty() {
            u.own += 1;
        }
    }
    u
}

/// The window's report line for a change's plan.
pub(crate) fn report_words(r: &ElementTypeReport) -> String {
    let followed = r.applied.len().saturating_sub(r.kept.len());
    let mut parts = vec![format!("{followed} eleman yeni değerleri alır")];
    if !r.kept.is_empty() {
        parts.push(format!("{} eleman değişen alanda kendi değerini korur", r.kept.len()));
    }
    if !r.skipped.is_empty() {
        parts.push(format!("{} eleman kilitli katmanda eski sürümde kalır", r.skipped.len()));
    }
    if !r.refused.is_empty() {
        parts.push(format!("{} eleman geçersiz olur, eski sürümde kalır", r.refused.len()));
    }
    parts.join("; ")
}

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait ElementTypesWindowApp {
    fn element_types_format(&self) -> Option<Format>;
    fn open_element_types(&mut self);
    fn element_types_choose(&mut self, id: &str);
    fn element_types_chosen(&self) -> Option<ElementType>;
    fn element_types_change(&self) -> Option<Result<Map<String, Value>, String>>;
    fn element_types_preview(&mut self);
    fn element_types_command(&mut self, operation: ElementTypesOperation) -> Option<ElementTypesEdited>;
    fn selected_typeable(&self) -> Vec<(String, ElementTypeKind, Option<String>)>;
    fn element_type_name(&self, kind: ElementTypeKind, wish: Option<&str>) -> String;
    fn element_types_said(&mut self, kind: Kind, words: String);
    fn element_types_event(&mut self, event: Event) -> Task<Message>;
    fn element_types_apply(&mut self) -> Task<Message>;
    fn select_same_element_type(&mut self);
    fn element_types_view(&self) -> Element<'_, Message>;
    fn element_types_table<'a>(&'a self, w: &'a Window, types: &'a [ElementType]) -> Element<'a, Message>;
    fn element_types_details<'a>(&'a self, w: &'a Window, f: &Format) -> Element<'a, Message>;
    fn element_types_control(&self, control: Control<'_>) -> Result<Option<Message>, String>;
}

impl ElementTypesWindowApp for App {
    fn element_types_format(&self) -> Option<Format>{
        self.document.as_ref().map(|d| Format::of(d.model.settings()))
    }

    /// Eleman tipleri (`arch.elementTypes`).
    fn open_element_types(&mut self){
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return;
        };
        // The selected element's type is chosen first, else the first type.
        let chosen = self
            .selection
            .ids()
            .iter()
            .find_map(|&s| doc.model.get(s).and_then(type_link).map(|l| l.id.clone()))
            .or_else(|| doc.model.element_types().first().map(|t| t.id.clone()));
        let mut w = self.modules.arch_mut().element_types.take().unwrap_or_default();
        w.said = None;
        self.modules.arch_mut().element_types = Some(w);
        if let Some(id) = chosen {
            self.element_types_choose(&id);
        }
        self.dialog = Some(crate::arch::dialog(crate::arch::ArchDialogKind::ElementTypes));
    }

    fn element_types_choose(&mut self, id: &str){
        let Some(f) = self.element_types_format() else { return };
        let Some(t) = self.document.as_ref().and_then(|d| d.model.element_type(id).cloned()) else { return };
        let Some(w) = self.modules.arch_mut().element_types.as_mut() else { return };
        w.chosen = Some(t.id.clone());
        w.name = t.name.clone();
        w.typed = t
            .kind
            .fields()
            .iter()
            .filter(|x| matches!(edit_of(t.kind, x), Edit::Length { .. } | Edit::Count | Edit::Text))
            .map(|x| {
                let v = t.values.get(x.key);
                let words = match edit_of(t.kind, x) {
                    Edit::Length { .. } => v.and_then(Value::as_f64).map_or_else(String::new, |m| f.length_bare(m)),
                    Edit::Count => v.and_then(Value::as_u64).map_or_else(String::new, |n| n.to_string()),
                    _ => v.and_then(Value::as_str).map_or_else(String::new, str::to_owned),
                };
                (x.key, words)
            })
            .collect();
        w.picked.clear();
        w.preview = None;
    }

    fn element_types_chosen(&self) -> Option<ElementType>{
        let id = self.modules.arch().element_types.as_ref()?.chosen.clone()?;
        self.document.as_ref()?.model.element_type(&id).cloned()
    }

    /// The values typed against the chosen type: the fields that changed (null: taken away), or why one cannot be read.
    fn element_types_change(&self) -> Option<Result<Map<String, Value>, String>>{
        let w = self.modules.arch().element_types.as_ref()?;
        let t = self.element_types_chosen()?;
        let f = self.element_types_format()?;
        let mut out = Map::new();
        for (key, typed) in &w.typed {
            let Some(field) = t.kind.field(key) else { continue };
            let now = t.values.get(*key);
            let typed = typed.trim();
            let next: Option<Value> = match edit_of(t.kind, field) {
                Edit::Length { required } => {
                    if typed.is_empty() {
                        if required {
                            return Some(Err(format!("{} boş olamaz; bir uzunluk yazın.", field.label)));
                        }
                        None
                    } else {
                        match kentos_geometry_core::tools::point_text::parse_length(typed) {
                            Some(l) => {
                                let m = l.metres(|n| f.to_metres(n));
                                if !(m.is_finite() && m > 0.0) {
                                    return Some(Err(format!("{} sıfırdan büyük olmalı.", field.label)));
                                }
                                // The typed words of the value it has: no change.
                                if now.and_then(Value::as_f64).is_some_and(|v| f.length_bare(v) == typed) {
                                    continue;
                                }
                                Some(Value::from(m))
                            }
                            None => return Some(Err(format!("{} okunmadı: “{typed}”.", field.label))),
                        }
                    }
                }
                Edit::Count if typed.is_empty() => None,
                Edit::Count => match typed.parse::<u32>() {
                    Ok(n) => Some(Value::from(n)),
                    Err(_) => return Some(Err(format!("{} bir tam sayı olmalı: “{typed}”.", field.label))),
                },
                _ if typed.is_empty() => None,
                _ => Some(Value::from(typed)),
            };
            if next.as_ref() != now {
                out.insert((*key).to_owned(), next.unwrap_or(Value::Null));
            }
        }
        for (key, picked) in &w.picked {
            if picked.as_ref() != t.values.get(*key) {
                out.insert((*key).to_owned(), picked.clone().unwrap_or(Value::Null));
            }
        }
        Some(Ok(out))
    }

    fn element_types_preview(&mut self){
        let change = self.element_types_change();
        let Some(t) = self.element_types_chosen() else { return };
        let preview = match change {
            None => None,
            Some(Err(why)) => Some(Err(why)),
            Some(Ok(values)) if values.is_empty() => None,
            Some(Ok(values)) => self.document.as_ref().map(|doc| {
                let input = ElementTypesEdit {
                    operation: ElementTypesOperation::Update { kind: t.id.clone(), values },
                    on_refused: ElementTypeRefused::Stop,
                    expected_revision: None,
                };
                match kentos_native_application::element_types_edit::plan_in(&doc.model, &input) {
                    CommandResult::Completed { output, .. } => Ok(output),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        Err(error.message)
                    }
                    _ => Err("Önizleme yapılamadı.".into()),
                }
            }),
        };
        if let Some(w) = self.modules.arch_mut().element_types.as_mut() {
            w.preview = preview;
        }
    }

    /// `cad.elementtypes.edit` over the drawing; its answer, or none with the refusal said.
    fn element_types_command(&mut self, operation: ElementTypesOperation) -> Option<ElementTypesEdited>{
        let doc = self.document.as_mut()?;
        let result = kentos_native_application::element_types_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            ElementTypesEdit { operation, on_refused: ElementTypeRefused::Stop, expected_revision: None },
        );
        match result {
            CommandResult::Completed { output, warnings, .. } => {
                for w in warnings {
                    self.warn(w.message);
                }
                Some(output)
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                let message = error.message;
                match self.modules.arch_mut().element_types.as_mut().filter(|_| self.dialog.is_some()) {
                    Some(w) => w.said = Some((Kind::Error, message)),
                    None => self.warn(message),
                }
                None
            }
            _ => None,
        }
    }

    /// The selected elements a type serves, each its persistent id and the type it follows.
    fn selected_typeable(&self) -> Vec<(String, ElementTypeKind, Option<String>)>{
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        self.selection
            .ids()
            .iter()
            .filter_map(|&s| {
                let e = doc.model.get(s)?;
                let kind = ElementTypeKind::of(e)?;
                Some((doc.model.uid(s)?.to_string(), kind, type_link(e).map(|l| l.id.clone())))
            })
            .collect()
    }

    /// A free name for a new `kind` type: “Kolon 1”, “Kolon 2” ….
    fn element_type_name(&self, kind: ElementTypeKind, wish: Option<&str>) -> String{
        let taken: Vec<String> = self
            .document
            .as_ref()
            .map(|d| d.model.element_types().iter().filter(|t| t.kind == kind).map(|t| t.name.to_lowercase()).collect())
            .unwrap_or_default();
        if let Some(w) = wish.map(str::trim).filter(|w| kentos_commands::element_type_name_ok(w))
            && !taken.contains(&w.to_lowercase())
        {
            return w.to_owned();
        }
        (1..).map(|n| format!("{} {n}", kind.label())).find(|n| !taken.contains(&n.to_lowercase())).unwrap_or_default()
    }

    fn element_types_said(&mut self, kind: Kind, words: String){
        match self.modules.arch_mut().element_types.as_mut().filter(|_| self.dialog.is_some()) {
            Some(w) => w.said = Some((kind, words)),
            None => self.say(if kind == Kind::Error { Level::Warn } else { Level::Success }, words),
        }
    }

    fn element_types_event(&mut self, event: Event) -> Task<Message>{
        match event {
            Event::Choose(id) => self.element_types_choose(&id),
            Event::Name(t) => {
                if let Some(w) = self.modules.arch_mut().element_types.as_mut() {
                    w.name = t;
                }
            }
            Event::Value(key, t) => {
                if let Some(w) = self.modules.arch_mut().element_types.as_mut() {
                    match w.typed.iter_mut().find(|(k, _)| *k == key) {
                        Some(x) => x.1 = t,
                        None => w.typed.push((key, t)),
                    }
                }
                self.element_types_preview();
            }
            Event::Pick(key, v) => {
                if let Some(w) = self.modules.arch_mut().element_types.as_mut() {
                    match w.picked.iter_mut().find(|(k, _)| *k == key) {
                        Some(x) => x.1 = v,
                        None => w.picked.push((key, v)),
                    }
                }
                self.element_types_preview();
            }
            Event::New(kind) => {
                let name = self.element_type_name(kind, None);
                let values = match serde_json::to_value(probe_values(kind)) {
                    Ok(Value::Object(m)) => m,
                    _ => Map::new(),
                };
                let op = ElementTypesOperation::Define {
                    name: name.clone(),
                    id: None,
                    element: Some(kind),
                    copy_of: None,
                    from: None,
                    values,
                };
                if let Some(done) = self.element_types_command(op)
                    && let Some(id) = done.id
                {
                    self.element_types_choose(&id);
                    self.element_types_said(Kind::Ok, format!("{name} tipi eklendi; değerlerini yazıp Uygula'ya basın."));
                }
            }
            Event::FromSelection => {
                let Some((uid, kind, _)) = self.selected_typeable().into_iter().next() else {
                    self.element_types_said(
                        Kind::Error,
                        "Önce bir kolon, kiriş, döşeme, çatı, korkuluk ya da merdiven seçin: tipi onun değerlerinden yapılır.".into(),
                    );
                    return Task::none();
                };
                // A column's own type text names the type when it is free.
                let wish = self.document.as_ref().and_then(|d| {
                    let slot = d.model.slot_of(uid.parse().ok()?)?;
                    match d.model.get(slot)? {
                        Entity::Column(c) => c.column.kind.clone(),
                        _ => None,
                    }
                });
                let name = self.element_type_name(kind, wish.as_deref());
                let op = ElementTypesOperation::Define {
                    name: name.clone(),
                    id: None,
                    element: None,
                    copy_of: None,
                    from: Some(uid),
                    values: Map::new(),
                };
                if let Some(done) = self.element_types_command(op)
                    && let Some(id) = done.id
                {
                    if self.modules.arch().element_types.is_none() {
                        self.modules.arch_mut().element_types = Some(Window::default());
                    }
                    self.element_types_choose(&id);
                    self.dialog = Some(crate::arch::dialog(crate::arch::ArchDialogKind::ElementTypes));
                    self.element_types_said(Kind::Ok, format!("{name} tipi seçili {} elemandan yapıldı; eleman onu izliyor.", kind.words()));
                }
            }
            Event::Apply => return self.element_types_apply(),
            Event::Remove => {
                let Some(t) = self.element_types_chosen() else { return Task::none() };
                let u = self.document.as_ref().map(|d| use_of(&d.model, &t)).unwrap_or_default();
                let op = ElementTypesOperation::Remove { kind: t.id.clone(), detach: true };
                if self.element_types_command(op).is_some() {
                    let first = self.document.as_ref().and_then(|d| d.model.element_types().first().map(|x| x.id.clone()));
                    if let Some(w) = self.modules.arch_mut().element_types.as_mut() {
                        *w = Window::default();
                    }
                    if let Some(id) = first {
                        self.element_types_choose(&id);
                    }
                    self.element_types_said(
                        Kind::Ok,
                        format!("{} tipi silindi; {} eleman değerleriyle kaldı, tip izlemiyor.", t.name, u.elements),
                    );
                }
            }
            Event::SelectSame => self.select_same_element_type(),
            Event::Close => {
                self.dialog = None;
            }
            Event::Assign(id) => {
                let Some(t) = self.document.as_ref().and_then(|d| d.model.element_type(&id).cloned()) else {
                    return Task::none();
                };
                let uids: Vec<String> =
                    self.selected_typeable().into_iter().filter(|(_, k, _)| *k == t.kind).map(|(u, _, _)| u).collect();
                if uids.is_empty() {
                    self.warn(format!("Seçimde {} yok; {} tipi yalnız {} elemanlara verilir.", t.kind.words(), t.name, t.kind.words()));
                    return Task::none();
                }
                let n = uids.len();
                if self.element_types_command(ElementTypesOperation::Assign { kind: id, uids }).is_some() {
                    self.say(Level::Success, format!("{n} {} {} tipini izliyor.", t.kind.words(), t.name));
                }
            }
            Event::Detach | Event::Reset | Event::Refresh => {
                let uids: Vec<String> =
                    self.selected_typeable().into_iter().filter(|(_, _, t)| t.is_some()).map(|(u, _, _)| u).collect();
                if uids.is_empty() {
                    self.warn("Seçimde bir tipi izleyen eleman yok.");
                    return Task::none();
                }
                let n = uids.len();
                let (op, done) = match event {
                    Event::Detach => (ElementTypesOperation::Detach { uids }, "tipten ayrıldı; değerleri kaldı"),
                    Event::Reset => (ElementTypesOperation::Reset { uids, fields: Vec::new() }, "tipine döndü"),
                    _ => (ElementTypesOperation::Refresh { uids }, "tipinin son sürümünü aldı"),
                };
                if self.element_types_command(op).is_some() {
                    self.say(Level::Success, format!("{n} eleman {done}."));
                }
            }
            Event::Push => {
                let typed: Vec<String> =
                    self.selected_typeable().into_iter().filter(|(_, _, t)| t.is_some()).map(|(u, _, _)| u).collect();
                let [uid] = typed.as_slice() else {
                    self.warn("Tipe uygula tek elemanla çalışır: değerleri tipin olacak elemanı seçin.");
                    return Task::none();
                };
                if let Some(done) = self.element_types_command(ElementTypesOperation::Push { uid: uid.clone(), fields: Vec::new() }) {
                    self.say(Level::Success, format!("Elemanın kendi değerleri tipin oldu; {}.", report_words(&done.report)));
                }
            }
        }
        Task::none()
    }

    fn element_types_apply(&mut self) -> Task<Message>{
        let Some(t) = self.element_types_chosen() else { return Task::none() };
        let name = self.modules.arch().element_types.as_ref().map(|w| w.name.trim().to_owned()).unwrap_or_default();
        let change = self.element_types_change();
        let mut said = Vec::new();
        if !name.is_empty() && name != t.name {
            if self.element_types_command(ElementTypesOperation::Rename { kind: t.id.clone(), name: name.clone() }).is_none() {
                return Task::none();
            }
            said.push(format!("adı {name}"));
        }
        match change {
            Some(Err(why)) => {
                self.element_types_said(Kind::Error, why);
                return Task::none();
            }
            Some(Ok(values)) if !values.is_empty() => {
                match self.element_types_command(ElementTypesOperation::Update { kind: t.id.clone(), values }) {
                    Some(done) => said.push(report_words(&done.report)),
                    None => return Task::none(),
                }
            }
            _ => {}
        }
        self.element_types_choose(&t.id);
        let shown = if name.is_empty() { t.name.clone() } else { name };
        if said.is_empty() {
            self.element_types_said(Kind::Info, "Değişen bir değer yok.".into());
        } else {
            self.element_types_said(Kind::Ok, format!("{shown} tipi değişti: {}.", said.join("; ")));
        }
        Task::none()
    }

    /// Aynı tiptekileri seç (`arch.elementSameType`): every element of the selected one's type (else the window's
    /// chosen type), selected.
    fn select_same_element_type(&mut self){
        let from_selection = self.selected_typeable().into_iter().find_map(|(_, _, t)| t);
        let Some(id) = from_selection.or_else(|| self.modules.arch().element_types.as_ref().and_then(|w| w.chosen.clone())) else {
            self.warn("Önce bir tipi izleyen eleman seçin: aynı tiptekiler onun tipiyle seçilir.");
            return;
        };
        let Some(doc) = self.document.as_ref() else { return };
        let name = doc.model.element_type(&id).map_or_else(|| id.clone(), |t| t.name.clone());
        let slots: Vec<Slot> = doc
            .model
            .entities()
            .filter(|e| type_link(e).is_some_and(|l| l.id == id))
            .map(|e| Slot(e.base().id))
            .collect();
        let n = slots.len();
        self.selection.set(slots);
        self.element_types_said(Kind::Ok, format!("{name} tipinde {n} eleman seçildi."));
    }

    fn element_types_view(&self) -> Element<'_, Message>{
        let (Some(w), Some(doc), Some(f)) = (&self.modules.arch().element_types, self.document.as_ref(), self.element_types_format())
        else {
            return space::horizontal().into();
        };
        let types = doc.model.element_types();
        let body: Element<'_, Message> = if types.is_empty() {
            label::caption(
                "Projede eleman tipi yok. Yeni tip ▾ ile bir türün tipini başlatın ya da çizdiğiniz bir elemanı seçip \
                 Seçili elemandan ile tipe çevirin.",
            )
            .into()
        } else {
            ListDetails::new(self.element_types_table(w, types), self.element_types_details(w, &f))
                .list_width(LIST)
                .height(BODY)
                .into()
        };
        let chosen = self.element_types_chosen();
        let mut foot = row![
            new_menu(),
            words::secondary(REMOVE, chosen.as_ref().map(|_| msg(Event::Remove))),
            words::secondary(SAME, chosen.as_ref().map(|_| msg(Event::SelectSame))),
        ]
        .spacing(8)
        .align_y(Center);
        foot = match &w.said {
            Some((kind, said)) => foot.push(container(words::text_line(*kind, said.clone())).width(Fill)),
            None => foot.push(space::horizontal().width(Fill)),
        };
        let foot = foot
            .push(words::primary(APPLY, chosen.as_ref().map(|_| msg(Event::Apply))))
            .push(words::secondary(CLOSE, Some(msg(Event::Close))));
        Frame::new(TITLE).push(label::caption(LEAD)).push(body).footer(foot).width(1020.0).modal(msg(Event::Close))
    }

    fn element_types_table<'a>(&'a self, w: &'a Window, types: &'a [ElementType]) -> Element<'a, Message>{
        let Some(doc) = self.document.as_ref() else { return space::horizontal().into() };
        let mut t = Table::new([
            Col::new("Tip").width(Fill),
            Col::new("Tür").width(scaled(76.0)),
            Col::new("Eleman").width(scaled(56.0)).align_right(),
            Col::new("İstisna").width(scaled(56.0)).align_right(),
            Col::new("Eski").width(scaled(44.0)).align_right(),
        ]);
        let mut order: Vec<&ElementType> = types.iter().collect();
        order.sort_by_key(|x| x.kind);
        for x in order {
            let u = use_of(&doc.model, x);
            t = t.push(
                Row::new([
                    table::text(x.name.clone()).into(),
                    table::text(x.kind.label()).into(),
                    table::number(u.elements.to_string()).into(),
                    table::number(u.own.to_string()).into(),
                    table::number(u.behind.to_string()).into(),
                ])
                .selected(w.chosen.as_deref() == Some(x.id.as_str()))
                .on_press(msg(Event::Choose(x.id.clone()))),
            );
        }
        t.into()
    }

    fn element_types_details<'a>(&'a self, w: &'a Window, f: &Format) -> Element<'a, Message>{
        let (Some(t), Some(doc)) = (self.element_types_chosen(), self.document.as_ref()) else {
            return label::caption("Bir tip seçin.").into();
        };
        let unit = f.length_unit_label();
        let mut form = Form::new()
            .label_width(scaled(140.0))
            .section(format!("{} tipi · sürüm {}", t.kind.label(), t.version))
            .field("Ad", text_field("Tipin adı", &w.name).on_input(|x| msg(Event::Name(x))));
        for field in t.kind.fields() {
            let key = field.key;
            let typed = || w.typed.iter().find(|(k, _)| *k == key).map(|(_, x)| x.as_str()).unwrap_or("");
            let cell: Element<'a, Message> = if field.named {
                label::body(format!("{} (tipin adı)", t.name)).into()
            } else {
                match edit_of(t.kind, field) {
                    Edit::Length { .. } => row![
                        text_field("", typed()).on_input(move |x| msg(Event::Value(key, x))).width(scaled(110.0)),
                        label::body(unit),
                    ]
                    .spacing(6)
                    .align_y(Center)
                    .into(),
                    Edit::Count => {
                        text_field("", typed()).on_input(move |x| msg(Event::Value(key, x))).width(scaled(110.0)).into()
                    }
                    Edit::Text => text_field("", typed()).on_input(move |x| msg(Event::Value(key, x))).width(Fill).into(),
                    Edit::Switch => {
                        let on = now_of(w, &t, key).and_then(|v| v.as_bool()).unwrap_or(false);
                        let words = if key == "fill" { ["Dolu", "Boş"] } else { ["Evet", "Hayır"] };
                        Select::new(words.iter().map(|x| Choice::new(*x)), Some(usize::from(!on)), move |i| {
                            msg(Event::Pick(key, (i == 0).then_some(Value::Bool(true))))
                        })
                        .searchable(false)
                        .into()
                    }
                    Edit::Choice => {
                        let list = choices(t.kind, key);
                        let now = now_of(w, &t, key);
                        let at = list.iter().position(|(_, v)| *v == now);
                        let values: Vec<Option<Value>> = list.iter().map(|(_, v)| v.clone()).collect();
                        Select::new(list.iter().map(|(x, _)| Choice::new(*x)), at, move |i| {
                            msg(Event::Pick(key, values.get(i).cloned().flatten()))
                        })
                        .searchable(false)
                        .into()
                    }
                    Edit::Shown => label::body(shown_words(key, t.values.get(key), f)).into(),
                }
            };
            form = form.field(field.label, cell);
        }
        let u = use_of(&doc.model, &t);
        form = form.field(
            "Kullanım",
            label::body(format!("{} eleman izliyor · {} istisna · {} eski sürümde", u.elements, u.own, u.behind)),
        );
        let preview: Element<'a, Message> = match &w.preview {
            None => label::caption(
                "Değeri değiştirip Uygula'ya basın. Katmanlar, kollar ve kenar bir elemanda değiştirilip Öznitelikler'in \
                 Tipe uygula'sıyla tipe verilir.",
            )
            .into(),
            Some(Ok(done)) => words::text_line(Kind::Info, format!("Uygula: {}.", report_words(&done.report))),
            Some(Err(why)) => words::text_line(Kind::Warn, why.clone()),
        };
        let out = Column::new().spacing(10).push(form).push(preview);
        scrollable(container(out).padding(iced::Padding::ZERO.right(14))).height(Fill).into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    fn element_types_control(&self, control: Control<'_>) -> Result<Option<Message>, String>{
        let Some(w) = &self.modules.arch().element_types else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        let chosen = self.element_types_chosen();
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(SAME) => Some(msg(Event::SelectSame)),
            Control::Pick(NEW, FROM) => Some(msg(Event::FromSelection)),
            Control::Pick(NEW, kind) => {
                let k = ElementTypeKind::ALL.into_iter().find(|k| k.label() == kind).ok_or_else(|| format!("Yeni tip ▾'de {kind} yok"))?;
                Some(msg(Event::New(k)))
            }
            Control::Fill("Ad", x) => Some(msg(Event::Name(x.to_owned()))),
            Control::Fill(label, x) => {
                let t = chosen.ok_or_else(|| "Seçili tip yok".to_owned())?;
                let f = t.kind.fields().iter().find(|f| f.label == label).ok_or_else(|| format!("“{label}” alanı yok"))?;
                Some(msg(Event::Value(f.key, x.to_owned())))
            }
            Control::Row(n) if n >= 1 => {
                let doc = self.document.as_ref().ok_or("Açık çizim yok")?;
                let mut order: Vec<&ElementType> = doc.model.element_types().iter().collect();
                order.sort_by_key(|x| x.kind);
                let x = order.get(n - 1).ok_or_else(|| format!("{n}. satır yok"))?;
                let _ = w;
                Some(msg(Event::Choose(x.id.clone())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A chosen field's value as the window shows it: the one picked, else the type's.
fn now_of(w: &Window, t: &ElementType, key: &str) -> Option<Value> {
    match w.picked.iter().find(|(k, _)| *k == key) {
        Some((_, v)) => v.clone(),
        None => t.values.get(key).cloned(),
    }
}

/// A new type's starting values: a plain element of its kind's (the window's fields then take the office's).
fn probe_values(kind: ElementTypeKind) -> Value {
    match kind {
        ElementTypeKind::Column => serde_json::json!({ "width": 0.3, "depth": 0.3 }),
        ElementTypeKind::Beam => serde_json::json!({ "width": 0.25, "height": 0.5 }),
        ElementTypeKind::Slab => serde_json::json!({ "thickness": 0.15 }),
        ElementTypeKind::Roof => serde_json::json!({}),
        ElementTypeKind::Railing => serde_json::json!({ "height": 1.1, "width": 0.05 }),
        ElementTypeKind::Stair => serde_json::json!({ "width": 1.2, "tread": 0.27 }),
    }
}

fn new_menu<'a>() -> Element<'a, Message> {
    let face = container(
        row![
            label::body(NEW),
            icon(kentos_ui::icon::Icon::ChevronDown).size(12.0).tone(kentos_ui::icon::Tone::Muted),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding(kentos_ui::theme::metrics::action())
    .style(style::container::field_box);
    MenuButton::new(face, move || {
        let mut menu = Menu::new().header("Türü seçin");
        for kind in ElementTypeKind::ALL {
            menu = menu.item(kind.label(), msg(Event::New(kind)));
        }
        menu.separator().item(FROM, msg(Event::FromSelection)).detail("Tipe çevir: değerleri tipin olur")
    })
    .into()
}

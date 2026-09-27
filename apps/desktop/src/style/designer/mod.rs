//! Sembol tasarımcısı on the desktop (docs/STYLE.md §7, docs/adr/0094; the
//! web's `ui/style/SymbolDesigner.ts`, `layerForms.ts` and
//! `designerFields.ts`): a symbol is a stack of layers. The stack is on the
//! left (a layer that places markers shows the marker's own layers under
//! it), the live preview on sample geometry in the middle, the chosen
//! layer's properties on the right. The designer edits a draft with its own
//! undo (Ctrl+Z, Ctrl+Y and the list's buttons); Kaydet writes it to the
//! library. A symbol inside a layer style is edited in the same window and
//! handed back with Uygula; it never reaches the library.
//!
//! The model (layer types, new layers, summaries, patches, the list's
//! edits) is `kentos_native_style::designer`, held with the web's to
//! `fixtures/style/v1/designer.json`.

mod foot;
mod form;
mod list;
pub(crate) mod numbers;
mod parts;
#[cfg(test)]
mod screens;
#[cfg(test)]
mod tests;
mod update;
mod view;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use iced::widget::Id;
use kentos_native_style::designer::{
    self as model, Draft, LayerPath, Patch, apply_patch, layer_at,
};
use kentos_native_style::library::Source;
use kentos_native_style::preview::Geometry;
use kentos_native_style::renderer::GeometryClass;
use serde_json::Value;

use crate::app::{Dialog, Message};
use crate::style::layer_style::SetAt;
use crate::style::thumbs::Thumbs;

pub(crate) use update::{Opening, focused};

/// Steps the designer's undo keeps.
const HISTORY: usize = 100;
/// Typing into one field within this is one undo step.
const MERGE: Duration = Duration::from_secs(1);
/// The preview's pictures kept: one per edit, so only the last few.
const PICTURES: usize = 8;

pub(crate) fn ev(e: Event) -> Message {
    Message::Designer(Box::new(e))
}

/// A field's widget id: the window gives it the keyboard and finds it by it.
pub(crate) fn field_id(key: &str) -> Id {
    Id::from(format!("sdes:{key}"))
}

/// Where the edited symbol goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// A symbol of Kitaplığım or of the project; None: a new one not saved yet.
    Library { id: Option<String>, source: Source },
    /// A symbol inside a layer style's slot (`title`: the slot's name): Uygula hands it back.
    Slot {
        at: SetAt,
        class: GeometryClass,
        title: String,
    },
}

/// A change a form makes.
#[derive(Clone, Debug)]
pub struct Edit {
    /// The layer it is made on.
    pub at: LayerPath,
    /// The field it came from: its text is kept by this key, and typing in it
    /// within a second is one undo step.
    pub key: String,
    /// The text as typed, kept while it differs from the value it gives.
    pub text: Option<String>,
    /// What the layer takes; None while the text is not a value.
    pub patch: Option<Patch>,
    /// The field the keyboard goes to after the change (an ƒ switched on: its expression).
    pub focus: Option<String>,
}

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    /// Vazgeç, Esc and the backdrop: closes, or asks first about changes.
    Close,
    /// The question's answers: close without saving, stay, save and close.
    Discard,
    Stay,
    SaveAndClose,
    /// Kaydet, or Uygula for a layer style's symbol.
    Save,
    Undo,
    Redo,
    /// A row of the list pressed.
    Select(LayerPath),
    /// ↑ or ↓ with no field holding the keyboard: the row above or below.
    Step(bool),
    /// Katman ekle: a layer type, into the symbol or into the marker of a layer.
    Add(&'static str, Option<usize>),
    Up,
    Down,
    Duplicate,
    Remove,
    /// A row's check box: the layer drawn or not.
    Enabled(LayerPath, bool),
    Sample(Geometry),
    /// − and + (a factor), 1:1, and the wheel over the preview (up: closer).
    Zoom(f64),
    RealSize,
    Wheel(f32),
    Name(String),
    Path(String),
    Edit(Edit),
    /// Enter in a field: its text gives way to the value's own.
    Settle(String),
    /// ↑ or ↓ with a field holding the keyboard (Shift: ten steps).
    Arrow {
        up: bool,
        shift: bool,
        focus: Focus,
    },
    /// Dosya al… for an image field: an SVG, PNG or JPEG into Kitaplığım.
    ImportAsset(String),
    /// Yeni çizim… and Düzenle… for an image field: the SVG editor, whose Kaydet gives the field its drawing.
    DrawSvg(String, Option<String>),
    AssetPicked(String, Option<(String, Vec<u8>)>),
}

/// Which field holds the keyboard: its id, and whether any does.
#[derive(Clone, Debug, Default)]
pub struct Focus {
    pub id: Option<Id>,
    pub any: bool,
}

/// The designer while it is open.
pub struct Designer {
    pub target: Target,
    pub draft: Draft,
    /// The draft as saved, or as opened: what the title's • compares with.
    saved: Draft,
    pub selected: LayerPath,
    pub sample: Geometry,
    pub px_per_mm: f64,
    past: Vec<Draft>,
    future: Vec<Draft>,
    last: Option<(String, Instant)>,
    /// Fields' texts as typed, by key, while they differ from their values.
    pub typed: HashMap<String, String>,
    /// The category field as typed (“Ana / Alt”).
    pub path_text: String,
    said: Option<(String, bool)>,
    /// Closing asked about the unsaved changes.
    pub asking: bool,
    /// The window under it, to go back to.
    under: Option<Dialog>,
    /// The preview's own pictures.
    pub thumbs: Thumbs,
}

impl Designer {
    fn new(target: Target, draft: Draft, under: Option<Dialog>) -> Designer {
        let sample = model::geometries(model::type_of(&draft.symbol))
            .first()
            .map_or(Geometry::Area, |(g, _)| *g);
        Designer {
            target,
            path_text: draft.path.join(" / "),
            saved: draft.clone(),
            draft,
            selected: LayerPath::Top(0),
            sample,
            px_per_mm: model::zoom::START,
            past: Vec::new(),
            future: Vec::new(),
            last: None,
            typed: HashMap::new(),
            said: None,
            asking: false,
            under,
            thumbs: Thumbs::with_limit(PICTURES),
        }
    }

    /// The symbol's kind: fill, line or marker.
    pub fn kind(&self) -> &str {
        model::type_of(&self.draft.symbol)
    }

    /// The slot's name when the symbol lives in a layer style.
    pub fn inline(&self) -> Option<&str> {
        match &self.target {
            Target::Slot { title, .. } => Some(title),
            Target::Library { .. } => None,
        }
    }

    pub fn dirty(&self) -> bool {
        self.draft != self.saved
    }

    pub fn title(&self) -> String {
        model::title(self.kind(), self.inline(), self.dirty())
    }

    pub fn layer(&self) -> Option<&Value> {
        layer_at(&self.draft.symbol, self.selected)
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// What the footer says last, and whether it warns.
    pub fn said(&self) -> Option<&(String, bool)> {
        self.said.as_ref()
    }

    pub(crate) fn say(&mut self, text: impl Into<String>, warn: bool) {
        self.said = Some((text.into(), warn));
    }

    /// Records the draft before a change. Typing into one field within a
    /// second is one step (`merge`); the list's edits are a step each (the
    /// web's merged two layers added quickly too).
    fn snapshot(&mut self, key: &str, merge: bool) {
        let now = Instant::now();
        if let Some((last, at)) = &mut self.last
            && merge
            && last == key
            && now.duration_since(*at) < MERGE
        {
            *at = now;
            return;
        }
        self.last = Some((key.to_owned(), now));
        self.past.push(self.draft.clone());
        if self.past.len() > HISTORY {
            self.past.remove(0);
        }
        self.future.clear();
    }

    /// A draft from the history: the fields show its values again.
    fn restore(&mut self, draft: Draft) {
        self.path_text = draft.path.join(" / ");
        self.draft = draft;
        if self.layer().is_none() {
            self.selected = LayerPath::Top(0);
        }
        self.last = None;
        self.typed.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.past.pop() {
            self.future.push(self.draft.clone());
            self.restore(prev);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.future.pop() {
            self.past.push(self.draft.clone());
            self.restore(next);
        }
    }

    /// A list edit: recorded, done, the chosen row moved, the typed texts dropped.
    fn list_edit(&mut self, key: &str, edit: impl FnOnce(&mut Value) -> Option<LayerPath>) {
        let mut symbol = self.draft.symbol.clone();
        let Some(chosen) = edit(&mut symbol) else {
            return;
        };
        self.snapshot(key, false);
        self.draft.symbol = symbol;
        self.selected = chosen;
        self.typed.clear();
    }

    /// A form's change on its layer.
    pub(crate) fn edit(&mut self, e: Edit) {
        if let Some(text) = e.text {
            self.typed.insert(e.key.clone(), text);
        }
        let Some(patch) = e.patch else {
            return;
        };
        let Some(layer) = layer_at(&self.draft.symbol, e.at).cloned() else {
            return;
        };
        let changed = apply_patch(&layer, &patch);
        if changed == layer {
            return;
        }
        self.snapshot(&format!("{:?}:{}", e.at, e.key), true);
        model::put_layer(&mut self.draft.symbol, e.at, changed);
    }

    /// The draft as the window leaves it for the library: its name and category.
    fn saved_as(&self) -> (String, Vec<String>) {
        model::saved_as(&self.draft.name, &self.draft.path)
    }

    /// ↑ or ↓ in a number field: a step from what it shows.
    fn arrow(&mut self, up: bool, shift: bool, id: &Id) {
        let Some(layer) = self.layer().cloned() else {
            return;
        };
        let Some((key, spec)) = numbers::KEYS.iter().find_map(|k| {
            (field_id(k) == *id)
                .then(|| numbers::spec_of(&layer, k).map(|s| (*k, s)))
                .flatten()
        }) else {
            return;
        };
        let text = self
            .typed
            .get(key)
            .cloned()
            .unwrap_or_else(|| numbers::text_of(numbers::shown(&layer, key)));
        let v = numbers::stepped(&spec, &text, up, shift);
        self.edit(Edit {
            at: self.selected,
            key: key.to_owned(),
            text: Some(numbers::text_of(v)),
            patch: Some(numbers::write(&layer, key, v)),
            focus: None,
        });
    }
}

/// The symbol a layer style's slot starts editing from: its own, the library
/// symbol it links to (a copy, kept in the style), the layer's plain look it
/// shows, or the class's default.
pub(crate) fn slot_symbol(
    library: &kentos_native_style::StyleLibrary,
    symbol: Option<&Value>,
    simple: Option<&Value>,
    class: GeometryClass,
) -> Option<Value> {
    use kentos_native_style::renderer::ref_id;
    let resolve = |s: &Value| match ref_id(s) {
        Some(id) => library.symbol(id).cloned(),
        None => Some(s.clone()),
    };
    match symbol {
        Some(s) => resolve(s),
        None => Some(
            simple
                .and_then(resolve)
                .unwrap_or_else(|| model::default_for(class)),
        ),
    }
}

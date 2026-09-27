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

mod form;
mod numbers;
mod parts;
#[cfg(test)]
mod screens;
#[cfg(test)]
mod tests;
mod view;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use iced::Task;
use iced::widget::Id;
use kentos_native_style::designer::{
    self as model, Draft, LayerPath, Patch, apply_patch, layer_at, new_draft, texts,
};
use kentos_native_style::file::{sanitize_svg, svg_asset, validate_symbol};
use kentos_native_style::library::{ItemKind, Source, new_item_id};
use kentos_native_style::preview::Geometry;
use kentos_native_style::renderer::GeometryClass;
use serde_json::{Map, Value, json};

use crate::app::{App, Dialog, Message};
use crate::style::layer_style::SetAt;
use crate::style::thumbs::Thumbs;

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
    fn edit(&mut self, e: Edit) {
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

/// Where a symbol of the library is designed from Stil yöneticisi.
pub(crate) struct Opening {
    pub id: Option<String>,
    pub kind: &'static str,
    pub path: Option<Vec<String>>,
    pub source: Source,
}

impl App {
    /// The designer on a library symbol (a user or project one) or on a new
    /// symbol of a kind, over the window that opened it (`openSymbolDesigner`).
    pub(crate) fn open_designer(&mut self, o: Opening) {
        let draft = match &o.id {
            Some(id) => match self.styles.library.get(id) {
                Some((item, source))
                    if item.kind() == ItemKind::Symbol && source.editable() =>
                {
                    Draft {
                        name: item.name().to_owned(),
                        path: item.path().into_iter().map(str::to_owned).collect(),
                        symbol: item.symbol().cloned().unwrap_or(Value::Null),
                    }
                }
                _ => {
                    self.warn(texts::CANNOT_EDIT);
                    return;
                }
            },
            None => new_draft(o.kind, o.path.as_deref()),
        };
        let source = match &o.id {
            Some(id) => self
                .styles
                .library
                .get(id)
                .map_or(o.source, |(_, source)| source),
            None => o.source,
        };
        let target = Target::Library { id: o.id, source };
        self.styles.designer = Some(Designer::new(target, draft, self.dialog));
        self.dialog = Some(Dialog::SymbolDesigner);
    }

    /// The designer on a symbol of a layer style's slot: Uygula hands it back.
    pub(crate) fn open_slot_designer(
        &mut self,
        at: SetAt,
        class: GeometryClass,
        title: String,
        symbol: Value,
    ) {
        let draft = Draft {
            name: title.clone(),
            path: Vec::new(),
            symbol,
        };
        let target = Target::Slot { at, class, title };
        self.styles.designer = Some(Designer::new(target, draft, self.dialog));
        self.dialog = Some(Dialog::SymbolDesigner);
    }

    /// The window goes; the one under it comes back when it is still there.
    fn close_designer(&mut self) {
        let under = self.styles.designer.take().and_then(|d| d.under);
        self.dialog = under.filter(|d| match d {
            Dialog::StyleManager => self.styles.manager.is_some(),
            Dialog::LayerStyle => self.styles.layer_style.is_some(),
            _ => true,
        });
    }

    /// Esc, Vazgeç or the backdrop (`confirmClose`): a question open is
    /// answered “stay”; changes are asked about; else the window closes.
    /// Sets the dialog itself (`close_dialog` has taken it).
    pub(crate) fn designer_close_request(&mut self) {
        let Some(d) = &mut self.styles.designer else {
            return;
        };
        if d.asking {
            d.asking = false;
            self.dialog = Some(Dialog::SymbolDesigner);
            return;
        }
        if d.dirty() {
            d.asking = true;
            self.dialog = Some(Dialog::SymbolDesigner);
            return;
        }
        self.close_designer();
    }

    /// Ctrl+Z, Ctrl+Y (Ctrl+Shift+Z) and ↑ ↓ while the designer is on top.
    pub(crate) fn designer_key(
        &mut self,
        press: &crate::keys::KeyPress,
    ) -> Option<Task<Message>> {
        use iced::keyboard::key::Named;
        let d = self.styles.designer.as_ref()?;
        if d.asking {
            return None;
        }
        let ctrl = press.modifiers.control() || press.modifiers.command();
        match press.named() {
            Some(key @ (Named::ArrowUp | Named::ArrowDown)) if !ctrl => {
                let up = key == Named::ArrowUp;
                let shift = press.modifiers.shift();
                return Some(
                    iced::advanced::widget::operate(focused())
                        .map(move |focus| ev(Event::Arrow { up, shift, focus })),
                );
            }
            _ => {}
        }
        match crate::keys::chord(press).as_deref() {
            Some("Ctrl+Z") => Some(self.designer_event(Event::Undo)),
            Some("Ctrl+Y" | "Ctrl+Shift+Z") => Some(self.designer_event(Event::Redo)),
            _ => None,
        }
    }

    pub(crate) fn designer_event(&mut self, event: Event) -> Task<Message> {
        let Some(d) = &mut self.styles.designer else {
            return Task::none();
        };
        match event {
            Event::Close => {
                self.dialog.take();
                self.designer_close_request();
            }
            Event::Discard => self.close_designer(),
            Event::Stay => d.asking = false,
            Event::SaveAndClose => {
                d.asking = false;
                if self.save_design() {
                    self.close_designer();
                }
            }
            Event::Save => {
                if self.save_design()
                    && matches!(
                        self.styles.designer.as_ref().map(|d| &d.target),
                        Some(Target::Slot { .. })
                    )
                {
                    self.close_designer();
                }
            }
            Event::Undo => d.undo(),
            Event::Redo => d.redo(),
            Event::Select(p) => {
                if d.selected != p {
                    d.selected = p;
                    d.typed.clear();
                }
            }
            Event::Step(up) => {
                let rows = view::rows(&d.draft.symbol);
                if let Some(i) = rows.iter().position(|p| *p == d.selected) {
                    let j = if up {
                        i.saturating_sub(1)
                    } else {
                        (i + 1).min(rows.len() - 1)
                    };
                    if rows[j] != d.selected {
                        d.selected = rows[j];
                        d.typed.clear();
                    }
                }
            }
            Event::Add(t, parent) => {
                d.list_edit("add", |s| Some(model::add_layer(s, t, parent)));
            }
            Event::Up | Event::Down => {
                let delta = if matches!(event, Event::Up) { -1 } else { 1 };
                let at = d.selected;
                d.list_edit("move", |s| model::move_layer(s, at, delta));
            }
            Event::Duplicate => {
                let at = d.selected;
                d.list_edit("dup", |s| model::duplicate_layer(s, at));
            }
            Event::Remove => {
                let at = d.selected;
                if !model::can_remove(&d.draft.symbol, at) {
                    d.say(texts::LAST_LAYER, true);
                } else {
                    d.list_edit("remove", |s| model::remove_layer(s, at));
                }
            }
            Event::Enabled(p, on) => {
                let before = d.selected;
                d.list_edit("enabled", |s| {
                    model::set_enabled(s, p, on);
                    Some(before)
                });
            }
            Event::Sample(g) => d.sample = g,
            Event::Zoom(f) => d.px_per_mm = model::zoomed(d.px_per_mm, f),
            Event::RealSize => d.px_per_mm = model::zoom::REAL,
            Event::Wheel(dy) => {
                if dy != 0.0 {
                    let f = if dy > 0.0 {
                        model::zoom::STEP
                    } else {
                        1.0 / model::zoom::STEP
                    };
                    d.px_per_mm = model::zoomed(d.px_per_mm, f);
                }
            }
            Event::Name(text) => {
                d.snapshot("name", true);
                d.draft.name = text;
            }
            Event::Path(text) => {
                d.snapshot("path", true);
                d.draft.path = model::path_of(&text);
                d.path_text = text;
            }
            Event::Edit(e) => {
                let focus = e.focus.clone();
                d.edit(e);
                if let Some(key) = focus {
                    return iced::widget::operation::focus(field_id(&key));
                }
            }
            // Enter ends the field's step too: what is typed next is another.
            Event::Settle(key) => {
                d.typed.remove(&key);
                d.last = None;
            }
            Event::Arrow { up, shift, focus } => match focus.id {
                Some(id) => d.arrow(up, shift, &id),
                // No field has the keyboard: the list's row above or below.
                None if !focus.any => return self.designer_event(Event::Step(up)),
                None => {}
            },
            Event::ImportAsset(key) => return pick_asset_file(key),
            Event::AssetPicked(_, None) => {}
            Event::AssetPicked(key, Some((name, bytes))) => self.take_asset(&key, &name, &bytes),
        }
        Task::none()
    }

    /// Kaydet (`save`): the symbol checked, then written to the library (the
    /// project's part into the drawing), or handed back to its layer style.
    /// Whether it was.
    fn save_design(&mut self) -> bool {
        let Some(d) = &mut self.styles.designer else {
            return false;
        };
        let issues = validate_symbol(&d.draft.symbol, "sembol");
        if let Some(first) = issues.first() {
            d.say(texts::not_saved(first, issues.len() - 1), true);
            return false;
        }
        let symbol = d.draft.symbol.clone();
        let (name, path) = d.saved_as();
        let (id, source) = match d.target.clone() {
            Target::Slot { at, class, .. } => {
                d.saved = d.draft.clone();
                if let Some(window) = &mut self.styles.layer_style {
                    window.put_symbol(&at, class, Some(symbol));
                }
                return true;
            }
            Target::Library { id, source } => (id, source),
        };
        if !self.writable(source) {
            self.designer_say(
                "Açık çizim yok: projenin kitaplığına ancak bir çizim açıkken kaydedilir.",
                true,
            );
            return false;
        }
        let written = match &id {
            Some(id) => {
                let mut patch = Map::new();
                patch.insert("name".into(), Value::from(name.clone()));
                patch.insert("path".into(), json!(path));
                patch.insert("symbol".into(), symbol);
                self.styles.library.update(id, &patch).map(|_| id.clone())
            }
            None => {
                let new = new_item_id(if source == Source::Project { "p" } else { "u" });
                let item = json!({
                    "kind": "symbol", "id": new, "name": name, "path": path, "symbol": symbol,
                });
                self.styles.library.add(source, item).map(|_| new)
            }
        };
        match written {
            Ok(saved) => {
                self.library_changed(source);
                if let Some(d) = &mut self.styles.designer {
                    d.target = Target::Library {
                        id: Some(saved.clone()),
                        source,
                    };
                    d.saved = d.draft.clone();
                    d.say(texts::saved(&name), false);
                }
                // Stil yöneticisi shows it where it now is (`onSaved`).
                self.reveal_in_manager(&saved);
                true
            }
            Err(e) => {
                self.designer_say(e, true);
                false
            }
        }
    }

    /// The designer's footer says it.
    fn designer_say(&mut self, text: impl Into<String>, warn: bool) {
        if let Some(d) = &mut self.styles.designer {
            d.say(text, warn);
        }
    }

    /// A picked file for an image field: an SVG drawing (cleaned) or a PNG or
    /// JPEG picture into Kitaplığım, and the field takes it (`importSvg`).
    fn take_asset(&mut self, key: &str, name: &str, bytes: &[u8]) {
        let asset = match crate::style::manager::files::raster_asset(name, bytes) {
            Some(Ok(asset)) => asset,
            Some(Err(why)) => {
                self.designer_say(why, true);
                return;
            }
            None => {
                let text = String::from_utf8_lossy(bytes);
                let clean = sanitize_svg(&text);
                if !clean.to_ascii_lowercase().contains("<svg") {
                    self.designer_say(format!("“{name}” bir SVG çizimi değil."), true);
                    return;
                }
                let stem = name
                    .strip_suffix(".svg")
                    .or_else(|| name.strip_suffix(".SVG"))
                    .unwrap_or(name);
                svg_asset(stem, &["Çizimlerim".to_owned()], &clean, &new_item_id("a"))
            }
        };
        match self.styles.library.add(Source::User, asset) {
            Ok(item) => {
                self.library_changed(Source::User);
                if let Some(d) = &mut self.styles.designer {
                    d.say(format!("“{}” Kitaplığım'a eklendi.", item.name()), false);
                    let at = d.selected;
                    d.edit(Edit {
                        at,
                        key: key.to_owned(),
                        text: None,
                        patch: Some(model::set(key, Value::from(item.id()))),
                        focus: None,
                    });
                }
            }
            Err(e) => self.designer_say(e, true),
        }
    }
}

/// Dosya al…: an SVG drawing or a PNG or JPEG picture from the computer.
fn pick_asset_file(key: String) -> Task<Message> {
    Task::perform(
        async {
            let file = rfd::AsyncFileDialog::new()
                .set_title("Çizim ya da görüntü al")
                .add_filter("SVG, PNG, JPEG", &["svg", "png", "jpg", "jpeg"])
                .pick_file()
                .await?;
            let name = file.file_name();
            let bytes = file.read().await;
            Some((name, bytes))
        },
        move |picked| ev(Event::AssetPicked(key.clone(), picked)),
    )
}

/// Finds the field holding the keyboard: its id when it has one, and
/// whether any does (a field without an id still keeps ↑ ↓ from the list).
fn focused() -> impl iced::advanced::widget::Operation<Focus> {
    use iced::advanced::widget::Operation;
    use iced::advanced::widget::operation::{Focusable, Outcome};
    use iced::Rectangle;

    struct Find(Focus);

    impl Operation<Focus> for Find {
        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            if state.is_focused() {
                self.0.any = true;
                if let Some(id) = id {
                    self.0.id = Some(id.clone());
                }
            }
        }

        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Focus>)) {
            operate(self);
        }

        fn finish(&self) -> Outcome<Focus> {
            Outcome::Some(self.0.clone())
        }
    }

    Find(Focus::default())
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

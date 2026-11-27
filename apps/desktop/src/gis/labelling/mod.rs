//! Etiketler (docs/adr/0212 §4; the web's `ui/labels/LabelsDialog.ts` and
//! `app/labelCommands.ts`).
//!
//! - The window (`layer.labels`): Katman (the active one, or the one the layer
//!   tree's menu named); Etiketleme: Tek etiket, Kurallı (an ordered list of
//!   classes, each its name, its condition and its own style) or Yok; the
//!   style in five tabs (Metin, Yerleşim, Biçim, Sığdırma, Öncelik, form.rs);
//!   Engel: the layer's objects as obstacles to every label (their weight, an
//!   area's inside or only its outline). Uygula writes through
//!   `cad.layers.labels` (one undo step “Etiketler”) and leaves the window
//!   open; Kaydet writes and closes; Vazgeç closes.
//! - Sabit etiketleri vurgula (`view.pinnedLabels`) and Yerleşmeyen etiketleri
//!   göster (`view.unplacedLabels`): the user's preferences
//!   (`graphics.pinnedLabels`, `graphics.unplacedLabels`); the drawing does
//!   not change (app.rs).

mod candidates;
mod examples;
pub(crate) mod form;
mod rows;

use crate::gis;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, Row, button, column, container, row, scrollable, text, text_editor};
use iced::{Bottom, Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, LabelClass, LabelObstacle, LabelPlacement, LabelStyle, LabelsMode, LabelsReport,
    LayerLabels, LayerNodeType, LayersLabels, ObstacleKind, default_label, layer_labels_problem,
    style_problem,
};
use kentos_interaction::Level;
use kentos_native_application::label_texts::compile_label_expression;
use kentos_native_application::{ExecutionContext, labels_report, layers_labels};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tabs::{Tab as TabFace, Tabs};
use kentos_ui::{label, style};

pub use form::Tab;
use form::{Edit, Host, Key};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Etiketler";
const LAYER: &str = "Katman";
const LABELLING: &str = "Etiketleme";
const APPLY: &str = "Uygula";
const CANCEL: &str = "Vazgeç";
/// Applies and closes (the second round's T27): the project file is written by Projeyi kaydet, not here.
const SAVE: &str = "Tamam";
const REPORT: &str = "Denetle";
const REPORT_AT: &str = "Denetle ölçeği";
/// The template's field picker (docs/adr/9145).
const ADD_FIELD: &str = "Alan ekle";
/// The most classes a layer's labelling has (`LABEL_CLASSES_MAX`).
const CLASSES_MAX: usize = 64;
/// Why a command has no layer to label.
const NO_LAYER: &str = "Etiketleme katmanın olur: etkin katman bir grup ya da servisten çizilen bir katman. Nesneleri olan bir katmanı etkin yapın.";
/// The classes column's width and its rows' name column (the web's 220 and 44 px).
const CLASSES_WIDTH: f32 = 220.0;
const CLASS_NAME: f32 = 44.0;

/// The labelling's three modes, in the window's order, with their words and hints.
const MODES: [(LabelsMode, &str, &str); 3] = [
    (
        LabelsMode::Single,
        "Tek etiket",
        "Katmanın bütün nesneleri tek bir stille etiketlenir.",
    ),
    (
        LabelsMode::Rules,
        "Kurallı",
        "Sıralı sınıflar: koşulu tutan her sınıf nesneyi kendi stiliyle etiketler.",
    ),
    (
        LabelsMode::Off,
        "Yok",
        "Katman etiketlenmez; nesneleri yine de diğer etiketlere engel olabilir.",
    ),
];

/// An expression field the builder (ε) edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// The style's text (Metin › İfade).
    Text,
    /// The selected class's condition.
    When,
}

/// The window: the layer, its labelling as edited and what it gives.
pub struct Window {
    layer: String,
    mode: LabelsMode,
    /// The single label's style.
    single: LabelStyle,
    /// The rules' classes (at least one, the single style's copy when the layer has none).
    classes: Vec<LabelClass>,
    selected: usize,
    obstacle: Option<LabelObstacle>,
    tab: Tab,
    /// What is typed, by field, until the window rebuilds the field.
    typed: HashMap<Key, String>,
    invalid: HashSet<Key>,
    /// The abbreviation dictionary's editor (Sığdırma › Sözlük).
    dictionary: text_editor::Content,
    /// What the summary says, each line with its kind, and whether Kaydet may write.
    pub(crate) lines: Vec<(Kind, String)>,
    ready: bool,
    /// Where Denetle counts (docs/adr/0636 §7): its place in [`App::report_scopes`]; 0, the view at its scale.
    report_at: usize,
    /// The fields the template's picker offers (docs/adr/9145): the layer's declared ones, then its objects' attributes.
    fields: Vec<String>,
    /// A data layer's first rows read from its source (docs/adr/9186), the examples' objects; why not; none while read.
    rows: Option<Result<Arc<kentos_domain::Document>, String>>,
    /// The read asked last: an earlier one's late answer is dropped.
    rows_asked: u64,
    /// The examples' first text: the preview draws it in the style (docs/adr/9186).
    sample: Option<String>,
}

/// The data layers' row reads asked, one number each.
static ROWS_ASKED: AtomicU64 = AtomicU64::new(1);

/// The preview's caption.
const PREVIEW: &str = "Haritada böyle görünür (gerçek boyunda)";

/// A scale and ground Denetle counts at: its words, the paper scale (none: the view's) and the ground (none: the view).
pub(crate) struct ReportScope {
    pub words: String,
    pub paper: Option<u32>,
    pub ground: Option<[f64; 4]>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The window opened on a layer at a tab (Yerleşim ayarlarını aç): the label tools' way on from a data layer's
    /// label (the second round's T24).
    OpenAt(String, Tab),
    Layer(usize),
    Mode(LabelsMode),
    Class(usize),
    Add,
    Copy,
    Up,
    Down,
    Remove,
    Tab(Tab),
    Edit(Edit),
    Type(Key, String),
    Dictionary(text_editor::Action),
    Builder(Field),
    /// The builder's text back.
    Expression(Field, String),
    Obstacle(bool),
    ObstacleKind(ObstacleKind),
    Apply,
    Save,
    /// Denetle: the labels the engine places for the layer in the view, counted (docs/adr/0533 §6).
    Report,
    /// Denetle ölçeği: where Denetle counts (docs/adr/0636 §7).
    ReportAt(usize),
    /// Alan ekle: the picker's field at this place written into the template (docs/adr/9145).
    Field(usize),
    /// A data layer's first rows read off the interface's thread (docs/adr/9186), by the read asked.
    Rows(u64, Result<Arc<kentos_domain::Document>, String>),
    Cancel,
}

pub(crate) fn msg(event: Event) -> Message {
    Message::Gis(gis::Event::Labelling(event))
}

/// A new class's name: “Sınıf n”, the first free.
fn free_name(classes: &[LabelClass]) -> String {
    (classes.len() + 1..)
        .map(|n| format!("Sınıf {n}"))
        .find(|name| !classes.iter().any(|c| c.name == *name))
        .unwrap_or_default()
}

impl Window {
    /// The style the form edits: the single label's or the selected class's.
    fn current(&self) -> &LabelStyle {
        match self.mode {
            LabelsMode::Rules => &self.classes[self.selected].style,
            _ => &self.single,
        }
    }

    fn current_mut(&mut self) -> &mut LabelStyle {
        match self.mode {
            LabelsMode::Rules => &mut self.classes[self.selected].style,
            _ => &mut self.single,
        }
    }

    /// What `cad.layers.labels` is given: Kurallı and Yok their labelling, Tek etiket its style and,
    /// with an obstacle, a labelling that holds it (none: the layer's goes).
    fn input(&self) -> LayersLabels {
        let labelling = |mode, classes| LayerLabels {
            mode,
            classes,
            obstacle: self.obstacle.clone(),
        };
        let (label, labels) = match self.mode {
            LabelsMode::Rules => (
                None,
                Some(Some(labelling(LabelsMode::Rules, self.classes.clone()))),
            ),
            LabelsMode::Off => (None, Some(Some(labelling(LabelsMode::Off, Vec::new())))),
            LabelsMode::Single => (
                Some(Some(self.single.clone())),
                Some(
                    self.obstacle
                        .is_some()
                        .then(|| labelling(LabelsMode::Single, Vec::new())),
                ),
            ),
        };
        LayersLabels {
            layer: self.layer.clone(),
            label,
            labels,
            expected_revision: None,
        }
    }

    /// The fields rebuilt from the style: what is typed in the form's fields goes (the class's
    /// name and condition and the obstacle's weight stay, with `keep`), and the dictionary is read anew.
    fn rebuild(&mut self, keep: bool) {
        self.typed
            .retain(|k, _| keep && matches!(k, Key::Name | Key::When | Key::Weight));
        self.invalid
            .retain(|k| keep && matches!(k, Key::Name | Key::When | Key::Weight));
        self.dictionary = text_editor::Content::with_text(&form::dictionary_text(self.current()));
    }
}

impl App {
    /// The layers a labelling may go on: those that hold objects, by id.
    fn label_layers(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        doc.model
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| l.service.is_none())
            .map(|l| l.id.clone())
            .collect()
    }

    /// The layer a command means: the one named, else the active one; a group or a service layer is none.
    fn label_layer_at_hand(&self, layer: Option<String>) -> Option<String> {
        let doc = self.document.as_ref()?;
        let layers = doc.model.layers();
        let id = layer.unwrap_or_else(|| layers.active().to_owned());
        layers
            .get(&id)
            .filter(|n| n.kind == LayerNodeType::Layer && n.service.is_none())
            .map(|n| n.id.clone())
    }

    /// What a layer holds (the second round's T25): its table's geometry, else most of its objects' kind (“point”,
    /// “polyline”, “polygon”); none when it is not known.
    fn labelling_kind(&self, layer: &str) -> Option<&'static str> {
        let doc = self.document.as_ref()?;
        let node = doc.model.layers().get(layer)?;
        if let Some(t) = node.data.as_ref().and_then(|d| d.geometry.as_ref()).map(|g| g.type_name.to_ascii_uppercase()) {
            return if t.contains("POINT") {
                Some("point")
            } else if t.contains("LINE") || t.contains("CURVE") {
                Some("polyline")
            } else if t.contains("POLYGON") || t.contains("SURFACE") {
                Some("polygon")
            } else {
                None
            };
        }
        let objects: Vec<&str> = doc.model.by_layer(layer).take(200).map(|e| e.kind()).collect();
        ["point", "polyline", "polygon"]
            .into_iter()
            .find(|k| objects.iter().filter(|o| **o == *k).count() * 2 > objects.len() && !objects.is_empty())
    }

    /// A point layer's symbol colour for the preview (T25); none for another layer.
    fn labelling_point_symbol(&self, layer: &str) -> Option<iced::Color> {
        if self.labelling_kind(layer) != Some("point") {
            return None;
        }
        let node = self.document.as_ref()?.model.layers().get(layer)?;
        let palette = crate::viewport::palette(self.canvas());
        Some(palette.resolve(&node.style.color).map_or_else(|| crate::view::hex_color(&node.style.color), crate::view::rgba_color))
    }

    /// The style a layer's single label starts from: its own, else its objects' most common kind's default.
    fn starting_label(&self, layer: &str) -> LabelStyle {
        let Some(doc) = &self.document else {
            return LabelStyle::default();
        };
        if let Some(own) = doc
            .model
            .layers()
            .get(layer)
            .and_then(|n| n.style.label.clone())
        {
            return own;
        }
        // A data layer's rows are its source's: its table's geometry says what it labels (the second round's T25).
        let table_kind = doc
            .model
            .layers()
            .get(layer)
            .and_then(|n| n.data.as_ref())
            .and_then(|d| d.geometry.as_ref())
            .map(|g| g.type_name.to_ascii_uppercase())
            .and_then(|t| {
                if t.contains("POINT") {
                    Some("point")
                } else if t.contains("LINE") || t.contains("CURVE") {
                    Some("polyline")
                } else if t.contains("POLYGON") || t.contains("SURFACE") {
                    Some("polygon")
                } else {
                    None
                }
            });
        let mut counts: Vec<(&'static str, usize)> = table_kind.map(|k| vec![(k, 1)]).unwrap_or_default();
        for e in doc.model.by_layer(layer).take(2000) {
            let kind = e.kind();
            match counts.iter_mut().find(|(k, _)| *k == kind) {
                Some((_, n)) => *n += 1,
                None => counts.push((kind, 1)),
            }
        }
        // The most common kind, the first seen of equals (the web's stable sort).
        let mut most: Option<(&str, usize)> = None;
        for (kind, n) in counts {
            if most.is_none_or(|(_, m)| n > m) {
                most = Some((kind, n));
            }
        }
        // A new point label goes around its symbol, its edge taken into account (T25); a saved one stays as it is.
        most.and_then(|(kind, _)| {
            default_label(kind).map(|mut s| {
                if kind == "point" {
                    s.point = Some(kentos_commands::PointLabelMode::Around);
                }
                s
            })
        })
            .unwrap_or_else(|| LabelStyle {
                placement: LabelPlacement::Center,
                size: 10.0,
                ..LabelStyle::default()
            })
    }

    /// The window over a layer, its labelling as the drawing has it.
    fn labelling_window(&self, layer: String, tab: Tab) -> Window {
        let node = self.document.as_ref().and_then(|d| d.model.layers().get(&layer));
        let labels = node.and_then(|n| n.style.labels.clone());
        // A layer with no labelling and no label style labels nothing (docs/adr/9311): the window opens on Yok.
        let unset = if node.is_some_and(|n| n.style.label.is_some()) {
            LabelsMode::Single
        } else {
            LabelsMode::Off
        };
        let single = self.starting_label(&layer);
        let mut classes = labels
            .as_ref()
            .map(|l| l.classes.clone())
            .unwrap_or_default();
        if classes.is_empty() {
            classes = vec![LabelClass {
                name: "Sınıf 1".to_owned(),
                when: None,
                style: single.clone(),
            }];
        }
        let fields = self
            .document
            .as_ref()
            .map(|d| candidates::picker_fields(&d.model, &layer))
            .unwrap_or_default();
        let mut w = Window {
            layer,
            mode: labels.as_ref().map_or(unset, |l| l.mode),
            single,
            classes,
            selected: 0,
            obstacle: labels.and_then(|l| l.obstacle),
            tab,
            typed: HashMap::new(),
            invalid: HashSet::new(),
            dictionary: text_editor::Content::new(),
            lines: Vec::new(),
            ready: false,
            report_at: self.gis.labelling_report_at,
            fields,
            rows: None,
            rows_asked: 0,
            sample: None,
        };
        w.rebuild(false);
        self.labelling_check(&mut w);
        w
    }

    /// A data layer's first rows asked of its source off the interface's thread (docs/adr/9186); nothing for a drawing
    /// layer, whose objects are in the drawing.
    fn ask_rows(&self, w: &mut Window) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(node) = doc.model.layers().get(&w.layer).filter(|n| n.data.is_some()).cloned() else {
            return Task::none();
        };
        let asked = ROWS_ASKED.fetch_add(1, Ordering::Relaxed);
        w.rows_asked = asked;
        w.rows = None;
        let settings = doc.model.settings().clone();
        let styles = doc.model.styles().clone();
        iced_runtime::task::blocking(move |mut out: iced::futures::channel::mpsc::Sender<Message>| {
            let read = rows::read(&node, &settings, &styles).map(Arc::new);
            let done = msg(Event::Rows(asked, read));
            let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, done));
        })
    }

    /// Etiketler (`layer.labels`): the window over `layer`, else the active layer; a data layer's first rows asked for
    /// its examples.
    pub(crate) fn open_labelling(&mut self, layer: Option<String>) -> Task<Message> {
        let Some(id) = self.label_layer_at_hand(layer) else {
            self.warn(NO_LAYER);
            return Task::none();
        };
        let mut w = self.labelling_window(id, Tab::Text);
        let task = self.ask_rows(&mut w);
        self.labelling_check(&mut w);
        self.gis.labelling = Some(w);
        self.dialog = Some(Dialog::Gis(gis::GisDialogKind::Labelling));
        task
    }

    /// The labelling checked as the contract does (its rules, its expressions); Kaydet and Uygula follow it.
    fn labelling_check(&self, w: &mut Window) {
        let given = w.input();
        let mut said: Vec<String> = Vec::new();
        let unit = self.reading_unit();
        let compiled = |source: &Option<String>| {
            source
                .as_deref()
                .and_then(|s| compile_label_expression(s, unit).err())
        };
        if let Some(Some(label)) = &given.label {
            if let Some(p) = style_problem(label) {
                said.push(format!("Etiketin stili: {p}."));
            }
            if let Some(e) = compiled(&label.text) {
                said.push(format!("Etiketin metni: {e}"));
            }
        }
        if let Some(Some(labels)) = &given.labels {
            if let Some(p) = layer_labels_problem(labels) {
                said.push(format!("Etiketleme: {p}."));
            }
            for c in &labels.classes {
                if let Some(e) = compiled(&c.when) {
                    said.push(format!("“{}” sınıfının koşulu: {e}", c.name));
                }
                if let Some(e) = compiled(&c.style.text) {
                    said.push(format!("“{}” sınıfının metni: {e}", c.name));
                }
            }
        }
        w.ready = said.is_empty();
        if !w.ready {
            w.lines = said.into_iter().take(3).map(|s| (Kind::Warn, s)).collect();
            return;
        }
        let Some(doc) = &self.document else {
            return;
        };
        let n = doc.model.by_layer(&w.layer).count();
        let bound = doc.model.layers().get(&w.layer).is_some_and(|l| l.data.is_some());
        let what = match w.mode {
            LabelsMode::Rules => format!("{} sınıf", w.classes.len()),
            LabelsMode::Off => "etiketsiz".to_owned(),
            LabelsMode::Single => "tek etiket".to_owned(),
        };
        let obstacle = w
            .obstacle
            .as_ref()
            .map(|o| format!(", engel (ağırlık {})", o.weight))
            .unwrap_or_default();
        let name = doc
            .model
            .layers()
            .get(&w.layer)
            .map_or_else(|| w.layer.clone(), |n| n.name.clone());
        // A data layer's rows are in its source: the drawing holds none of them (docs/adr/9186).
        let mut lines = vec![(
            Kind::Ok,
            if bound {
                format!("{name}: veri katmanı, satırları kaynağında; {what}{obstacle}.")
            } else {
                format!("{name}: {n} nesne, {what}{obstacle}.")
            },
        )];
        w.sample = None;
        // The text the layer's first objects take as typed, and a template's field the layer does not have
        // (docs/adr/9145).
        if w.mode != LabelsMode::Off
            && let Some(node) = self.draft_layer(w)
        {
            // The examples on the same data (docs/adr/9164): the first and the longest text, the empty ones, the scales
            // it shows at with the view's, its letters on paper. A data layer's on its source's first rows (docs/adr/9186).
            let scale = self.viewport.camera.upright().scale;
            if bound {
                match &w.rows {
                    None => lines.push((Kind::Info, "Örnekler için kaynağın ilk satırları okunuyor…".to_owned())),
                    Some(Err(why)) => lines.push((Kind::Warn, format!("Örnek satırlar okunamadı: {why}"))),
                    Some(Ok(sample)) => {
                        let mut plain = node.clone();
                        plain.data = None;
                        let read = sample.by_layer(&node.id).count();
                        lines.push((Kind::Info, format!("Örnekler tablonun ilk {read} satırından.")));
                        lines.extend(examples::example_lines(sample, &plain, read, scale));
                        w.sample = examples::texts(sample, &plain).first.map(|(text, _)| text);
                    }
                }
            } else {
                lines.extend(examples::example_lines(&doc.model, &node, n, scale));
                w.sample = examples::texts(&doc.model, &node).first.map(|(text, _)| text);
            }
            let unknown = candidates::unknown_fields(&doc.model, &node);
            if !unknown.is_empty() {
                lines.push((Kind::Warn, candidates::unknown_words(&unknown, &w.fields)));
            }
        }
        w.lines = lines;
    }

    /// The layer as the window would write it: the drawing's node with the window's labelling.
    fn draft_layer(&self, w: &Window) -> Option<kentos_commands::LayerNode> {
        let doc = self.document.as_ref()?;
        let mut node = doc.model.layers().get(&w.layer)?.clone();
        let given = w.input();
        if let Some(label) = given.label {
            node.style.label = label;
        }
        if let Some(labels) = given.labels {
            node.style.labels = labels;
        }
        Some(node)
    }

    pub(crate) fn labelling_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::OpenAt(layer, tab) => {
                let task = self.open_labelling(Some(layer));
                if let Some(w) = self.gis.labelling.as_mut() {
                    w.tab = tab;
                }
                return task;
            }
            _ => {}
        }
        let Some(mut w) = self.gis.labelling.take() else {
            return Task::none();
        };
        match event {
            Event::Layer(i) => {
                if let Some(id) = self.label_layers().get(i).cloned() {
                    w = self.labelling_window(id, w.tab);
                    let task = self.ask_rows(&mut w);
                    self.labelling_check(&mut w);
                    self.gis.labelling = Some(w);
                    return task;
                }
            }
            Event::Rows(asked, read) => {
                if asked == w.rows_asked {
                    w.rows = Some(read);
                }
            }
            Event::Mode(mode) => {
                w.mode = mode;
                w.rebuild(false);
            }
            Event::Class(i) => {
                if i < w.classes.len() {
                    w.selected = i;
                    w.rebuild(false);
                }
            }
            Event::Add if w.classes.len() < CLASSES_MAX => {
                let name = free_name(&w.classes);
                w.classes.push(LabelClass {
                    name,
                    when: None,
                    style: w.single.clone(),
                });
                w.selected = w.classes.len() - 1;
                w.rebuild(false);
            }
            Event::Copy if w.classes.len() < CLASSES_MAX => {
                let mut copy = w.classes[w.selected].clone();
                copy.name = free_name(&w.classes);
                w.classes.insert(w.selected + 1, copy);
                w.selected += 1;
                w.rebuild(false);
            }
            Event::Up if w.selected > 0 => {
                w.classes.swap(w.selected - 1, w.selected);
                w.selected -= 1;
                w.rebuild(false);
            }
            Event::Down if w.selected + 1 < w.classes.len() => {
                w.classes.swap(w.selected, w.selected + 1);
                w.selected += 1;
                w.rebuild(false);
            }
            Event::Remove if w.classes.len() > 1 => {
                w.classes.remove(w.selected);
                w.selected = w.selected.min(w.classes.len() - 1);
                w.rebuild(false);
            }
            Event::Add | Event::Copy | Event::Up | Event::Down | Event::Remove => {}
            Event::Tab(tab) => {
                w.tab = tab;
                w.rebuild(true);
            }
            Event::Edit(edit) => {
                form::apply(w.current_mut(), edit);
                w.rebuild(true);
            }
            Event::Type(key, typed) => {
                let read = match key {
                    Key::Name => {
                        let c = &mut w.classes[w.selected];
                        c.name = typed.trim().to_owned();
                        true
                    }
                    Key::When => {
                        let c = &mut w.classes[w.selected];
                        let t = typed.trim();
                        c.when = (!t.is_empty()).then(|| t.to_owned());
                        true
                    }
                    Key::Weight => match (w.obstacle.as_mut(), typed.trim().parse::<f64>()) {
                        (Some(o), Ok(v)) if v.is_finite() && (1.0..=10.0).contains(&v.round()) => {
                            o.weight = v.round() as u8;
                            true
                        }
                        _ => false,
                    },
                    _ => form::typed(w.current_mut(), key, &typed),
                };
                w.typed.insert(key, typed);
                if read {
                    w.invalid.remove(&key);
                } else {
                    w.invalid.insert(key);
                }
            }
            Event::Dictionary(action) => {
                let edits = action.is_edit();
                w.dictionary.perform(action);
                if edits {
                    let text = w.dictionary.text();
                    form::dictionary(w.current_mut(), &text);
                }
            }
            Event::Builder(field) => {
                let value = match field {
                    Field::Text => w.current().text.clone().unwrap_or_default(),
                    Field::When => w.classes[w.selected].when.clone().unwrap_or_default(),
                };
                let layer = w.layer.clone();
                self.gis.labelling = Some(w);
                return self.open_builder_for_labels(field, &layer, &value);
            }
            Event::Expression(field, text) => {
                match field {
                    Field::Text => w.current_mut().text = Some(text),
                    Field::When => {
                        let t = text.trim();
                        w.classes[w.selected].when = (!t.is_empty()).then(|| t.to_owned());
                    }
                }
                w.rebuild(false);
            }
            Event::Obstacle(on) => {
                w.obstacle = on.then_some(LabelObstacle {
                    weight: 5,
                    kind: None,
                });
                w.typed.remove(&Key::Weight);
                w.invalid.remove(&Key::Weight);
            }
            Event::ObstacleKind(kind) => {
                if let Some(o) = &mut w.obstacle {
                    o.kind = (kind == ObstacleKind::Boundary).then_some(ObstacleKind::Boundary);
                }
            }
            Event::Apply | Event::Save => {
                self.labelling_check(&mut w);
                let save = matches!(event, Event::Save);
                if w.ready && self.write_labelling(&w, save) && save {
                    self.dialog = None;
                    return Task::none();
                }
            }
            Event::Report => {
                w.lines = self.labelling_report(&w);
                self.gis.labelling = Some(w);
                return Task::none();
            }
            Event::ReportAt(i) => {
                w.report_at = i;
                self.gis.labelling_report_at = i;
                w.lines = self.labelling_report(&w);
                self.gis.labelling = Some(w);
                return Task::none();
            }
            Event::Field(i) => {
                if let Some(name) = w.fields.get(i).cloned() {
                    let now = w
                        .typed
                        .get(&Key::Template)
                        .cloned()
                        .unwrap_or_else(|| form::value_text(w.current(), Key::Template));
                    let typed = if now.trim().is_empty() {
                        format!("{{{name}}}")
                    } else {
                        format!("{now} {{{name}}}")
                    };
                    form::typed(w.current_mut(), Key::Template, &typed);
                    w.typed.insert(Key::Template, typed);
                    w.invalid.remove(&Key::Template);
                }
            }
            Event::Cancel => {
                self.dialog = None;
                return Task::none();
            }
            Event::OpenAt(..) => {}
        }
        self.labelling_check(&mut w);
        self.gis.labelling = Some(w);
        Task::none()
    }

    /// Denetle: how the engine places this layer's labels in the view at its scale, as `cad.labels.report` counts
    /// them (docs/adr/0533 §6), with what a bound layer's loaded part means for them. It counts the labelling as the
    /// window has it, not yet written, with the drawing's texts as the screen sizes them keeping the labels off them
    /// (14 November 2026: it counted the drawing's labelling, so a change counted only after Kaydet, and no text).
    fn labelling_report(&mut self, w: &Window) -> Vec<(Kind, String)> {
        let camera = self.viewport.camera.upright();
        let scopes = self.report_scopes();
        let scope = scopes.get(w.report_at).or(scopes.first());
        let texts = self.label_size();
        let Some(doc) = self.document.as_mut() else {
            return Vec::new();
        };
        let view = camera.visible_bounds();
        let paper = scope.and_then(|s| s.paper);
        let scale = paper.map_or(camera.scale, labels_report::paper_px_per_metre);
        let mut window = scope
            .and_then(|s| s.ground)
            .unwrap_or([view.min_x, view.min_y, view.max_x, view.max_y]);
        // A view far out counted at a paper scale is a page past what the engine places in (100 million pixels): its
        // middle is counted and the summary says so. Denetle answered only that the window was too big.
        let pixels = (window[2] - window[0]) * scale * (window[3] - window[1]) * scale;
        let middle = (pixels > labels_report::MOST_PIXELS).then(|| {
            let k = (0.98 * labels_report::MOST_PIXELS / pixels).sqrt();
            let (x, y) = ((window[0] + window[2]) / 2.0, (window[1] + window[3]) / 2.0);
            let (half_w, half_h) = (
                (window[2] - window[0]) / 2.0 * k,
                (window[3] - window[1]) / 2.0 * k,
            );
            window = [x - half_w, y - half_h, x + half_w, y + half_h];
            (2.0 * half_w, 2.0 * half_h)
        });
        let input = LabelsReport {
            px_per_metre: paper.is_none().then_some(camera.scale),
            paper_scale: paper,
            window: Some(window),
            layers: Some(vec![w.layer.clone()]),
        };
        let draft_node = match layers_labels::plan(&ExecutionContext::new(&mut doc.model), &w.input()) {
            CommandResult::Completed { output, .. } => output.node,
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => return vec![(Kind::Warn, error.message)],
            _ => return Vec::new(),
        };
        // A data layer's labels are placed on the rows the view read, in a document of their own (data_labels.rs,
        // docs/adr/9186): counted there, the layer without its binding. Before the view holds them, the window says so
        // and its examples' rows give the candidates.
        let bound = draft_node.data.is_some();
        let mut draft_node = draft_node;
        draft_node.data = None;
        let target: &mut kentos_domain::Document = if bound {
            match self.gis.data_labels.iter_mut().find(|d| d.layer == w.layer) {
                Some(d) => d.doc_mut(),
                None => {
                    let mut lines = vec![(
                        Kind::Info,
                        "Veri katmanının etiketleri görünümün okuduğu satırlara yerleşir; görünüm satırları okuyunca Denetle \
                         onları sayar. Etiketleme kapalıysa önce Uygula ile açın."
                            .to_owned(),
                    )];
                    if let Some(Ok(sample)) = &w.rows {
                        let found = candidates::count(sample, &draft_node, window, scale);
                        lines.extend(candidates::lines(&found, scale, true, 0));
                    }
                    return lines;
                }
            }
        } else {
            &mut doc.model
        };
        // The bands round the parts are the ones the drawing is drawn with: no file is read on this thread. A data
        // layer's read has none.
        let bands = if bound { Vec::new() } else { self.bands.given(target) };
        let draft = labels_report::Draft {
            layer: Some(&draft_node),
            texts: Some(texts),
        };
        let mut lines = match labels_report::execute_with(
            &mut ExecutionContext::new(target),
            input,
            &bands,
            draft,
        ) {
            CommandResult::Completed { output, .. } => {
                let t = &output.totals;
                // Nothing counted is no success: the candidates' lines below say why (docs/adr/9145).
                let mut lines = vec![(
                    if t.placed + t.unplaced + t.hidden == 0 {
                        Kind::Info
                    } else if t.collisions == 0 && t.unplaced == 0 {
                        Kind::Ok
                    } else {
                        Kind::Warn
                    },
                    format!(
                        "Görünümde {} etiket yerleşti, {} yerleşemedi, {} gizli; {} elle sabit, {} zorla konmuş.",
                        t.placed, t.unplaced, t.hidden, t.pinned, t.forced
                    ),
                )];
                lines.push((
                    if t.collisions == 0 && t.repeats == 0 {
                        Kind::Ok
                    } else {
                        Kind::Warn
                    },
                    format!(
                        "{} etiket çifti gerçekten örtüşüyor, {} etiket yineleniyor.",
                        t.collisions, t.repeats
                    ),
                ));
                // The candidates before the engine, a sample of their text and each reason apart (docs/adr/9145).
                let found = candidates::count(target, &draft_node, window, scale);
                lines.extend(candidates::lines(
                    &found,
                    scale,
                    t.placed + t.unplaced + t.hidden == 0,
                    t.unplaced,
                ));
                if output.layers.iter().any(|l| l.partial) {
                    lines.push((
                        Kind::Info,
                        "Katman bağlı tablonun yalnız yüklenen parçasını tutuyor: etiketleri o parçanınkidir.".to_owned(),
                    ));
                }
                // The part's neighbours and the pins parked in the binding (docs/adr/0578).
                for l in &output.layers {
                    match (l.band, l.neighbours) {
                        (Some(_), Some(n)) => lines.push((
                            Kind::Info,
                            format!(
                                "Parçanın çevresindeki {n} satır engel olarak yer tutuyor (nesne değiller; çizilmez, etiketlenmez)."
                            ),
                        )),
                        (Some(_), None) => lines.push((
                            Kind::Warn,
                            "Parçanın çevresindeki komşular okunamadı ya da henüz okunuyor: kenardaki etiketler yalnız parçanın nesnelerine göre yerleşir."
                                .to_owned(),
                        )),
                        _ => {}
                    }
                    if !l.parked.is_empty() {
                        let pins: u32 = l.parked.iter().map(|p| p.pins).sum();
                        lines.push((
                            Kind::Info,
                            format!(
                                "{} nesnenin {pins} etiket iğnesi park edilmiş: satırları parçanın dışında; alana girince geri verilir.",
                                l.parked.len()
                            ),
                        ));
                    }
                }
                lines
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => vec![(Kind::Warn, error.message)],
            _ => Vec::new(),
        };
        if let (Some((wide, high)), Some(n)) = (middle, paper) {
            lines.push((
                Kind::Info,
                format!(
                    "Görünüm 1:{n} ölçekte bir sayfaya sığmayacak kadar geniş: ortasındaki {} × {} m'lik kısım sayıldı.",
                    wide.round(),
                    high.round()
                ),
            ));
        }
        lines
    }

    /// Where Denetle may count (docs/adr/0636 §7): the view at its scale, the view at the drawing's scale on paper, and
    /// each map of the sheet in front at its own scale and ground.
    pub(crate) fn report_scopes(&self) -> Vec<ReportScope> {
        let mut out = vec![ReportScope {
            words: "Görünüm, ekrandaki ölçeğiyle".into(),
            paper: None,
            ground: None,
        }];
        if let Some(doc) = &self.document {
            let n = doc
                .model
                .settings()
                .plot_scale
                .round()
                .clamp(1.0, 1_000_000.0) as u32;
            out.push(ReportScope {
                words: format!("Görünüm, çizim ölçeğinde (1:{n})"),
                paper: Some(n),
                ground: None,
            });
        }
        let book = self.sheets.book();
        for p in self
            .sheets
            .display_list()
            .iter()
            .flat_map(|l| l.prims.iter())
        {
            let kentos_sheet::display::Prim::Map(m) = p else {
                continue;
            };
            let Some(extent) = m.extent else {
                continue;
            };
            let n = m.view.scale.round().clamp(1.0, 1_000_000.0) as u32;
            let name = book
                .item(&m.item)
                .map_or(m.item.as_str(), |i| i.name.as_str());
            out.push(ReportScope {
                words: format!("Paftanın haritası: {name} (1:{n})"),
                paper: Some(n),
                ground: Some(extent),
            });
        }
        out
    }

    /// Writes the labelling through `cad.layers.labels` and says what it did, once (the second round's T27): applied, on
    /// the map and in the open project; Tamam after Uygula with nothing changed closes without a word. Whether it went
    /// (or nothing changed).
    fn write_labelling(&mut self, w: &Window, closing: bool) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let model = &mut doc.model;
        let name = model
            .layers()
            .get(&w.layer)
            .map_or_else(|| w.layer.clone(), |n| n.name.clone());
        match layers_labels::execute(&mut ExecutionContext::new(model), w.input()) {
            CommandResult::Completed { output, .. } => {
                if output.changed {
                    self.announce(
                        Level::Success,
                        format!("“{name}” katmanının etiketlemesi uygulandı."),
                    );
                } else if !closing {
                    self.announce(
                        Level::Info,
                        format!("“{name}” katmanının etiketlemesi zaten böyle."),
                    );
                }
                true
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                false
            }
            _ => false,
        }
    }

    pub(crate) fn labelling_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.gis.labelling else {
            return text("").into();
        };
        let layers = self.label_layers();
        let layer = Select::new(
            layers.iter().map(|id| Choice::new(self.layer_path(id))),
            layers.iter().position(|id| *id == w.layer),
            |i| msg(Event::Layer(i)),
        );
        let mode = form::choice(
            &MODES.map(|(_, words, _)| words),
            MODES.iter().position(|(m, _, _)| *m == w.mode).unwrap_or(0),
            |i| msg(Event::Mode(MODES[i].0)),
        )
        .hints(MODES.map(|(_, _, hint)| hint));
        let scopes = self.report_scopes();
        let at = Select::new(
            scopes.iter().map(|s| Choice::new(s.words.clone())),
            Some(w.report_at.min(scopes.len().saturating_sub(1))),
            |i| msg(Event::ReportAt(i)),
        );
        let head = row![
            container(words::field(LAYER, layer, None)).width(Fill),
            words::field(LABELLING, mode, None),
            container(words::field(REPORT_AT, at, None)).width(260),
        ]
        .spacing(16)
        .align_y(Bottom);

        let host = Host {
            style: w.current(),
            typed: &w.typed,
            invalid: &w.invalid,
            dictionary: &w.dictionary,
            view_scale: self.viewport.camera.screen_scale(),
            fields: &w.fields,
            kind: self.labelling_kind(&w.layer),
        };
        let tabs = Tabs::new(
            form::TABS.map(|(_, words)| TabFace::new(words).closable(false)),
            form::TABS
                .iter()
                .position(|(t, _)| *t == w.tab)
                .unwrap_or(0),
            |i| msg(Event::Tab(form::TABS[i].0)),
        );
        let form_rows: Element<'_, Message> = if w.mode == LabelsMode::Off {
            container(label::muted(
                "Katman etiketlenmez. Nesnelerinin diğer etiketlere engel olması için aşağıdaki Engel’i açın.",
            ))
            .padding([24, 0])
            .into()
        } else {
            Column::with_children(form::rows(w.tab, &host))
                .spacing(8)
                .padding(iced::Padding {
                    right: 12.0,
                    ..iced::Padding::ZERO
                })
                .into()
        };
        let style_column = column![
            tabs,
            scrollable(form_rows)
                .direction(style::field::body_scrollbar())
                .height(Fill),
        ]
        .spacing(10)
        .width(Fill);
        let mut body = Row::new().spacing(16).height(Fill);
        if w.mode == LabelsMode::Rules {
            body = body
                .push(self.labelling_classes(w))
                .push(kentos_ui::widget::vertical_divider());
        }
        body = body.push(style_column);

        let lines = w
            .lines
            .iter()
            .map(|(kind, line)| words::text_line(*kind, line.clone()))
            .collect();
        let mut frame = Frame::new(TITLE).push(head).fill(body);
        // The example as the map draws it (docs/adr/9186): the first object's text in the current class's look.
        if let Some(text) = w.sample.clone().filter(|_| w.mode != LabelsMode::Off) {
            let colors = crate::labels::colors(self.canvas(), &crate::viewport::palette(self.canvas()))
                .in_mode(self.color_mode());
            let font = self
                .document
                .as_ref()
                .and_then(|d| d.model.settings().drawing_font)
                .unwrap_or(kentos_commands::DrawingFont::Barlow);
            frame = frame.push(words::field(
                PREVIEW,
                crate::labels::example(text, w.current().clone(), colors, font, self.labelling_point_symbol(&w.layer)),
                None,
            ));
        }
        frame
            .push(labelling_obstacle(w))
            .push(words::summary(lines))
            .action(words::secondary(REPORT, Some(msg(Event::Report))))
            .action(words::secondary(APPLY, w.ready.then(|| msg(Event::Apply))))
            .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .action(words::primary(SAVE, w.ready.then(|| msg(Event::Save))))
            .width(860.0)
            .max_height(640.0)
            .modal(msg(Event::Cancel))
    }

    /// Kurallı's classes: the list, its tools and the selected class's name and condition.
    fn labelling_classes<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let list = w
            .classes
            .iter()
            .enumerate()
            .fold(Column::new(), |list, (i, c)| {
                let mut face = Column::new().spacing(1).push(label::body(c.name.as_str()));
                if let Some(when) = &c.when {
                    face = face.push(label::mono_caption(when.as_str()));
                }
                list.push(
                    button(face)
                        .on_press(msg(Event::Class(i)))
                        .width(Fill)
                        .padding([5, 8])
                        .style(style::button::row(i == w.selected)),
                )
            });
        let tool = |glyph: &'static str, caption: &'static str, on: bool, event: Event| {
            kentos_ui::widget::tip(
                button(icon(from_web(Some(glyph))).size(16.0))
                    .style(style::button::ghost)
                    .padding(kentos_ui::theme::metrics::icon_small(16.0))
                    .on_press_maybe(on.then(|| msg(event))),
                kentos_ui::widget::Tip::new(caption),
                iced::widget::tooltip::Position::Top,
            )
        };
        let n = w.classes.len();
        let tools = row![
            tool("plus", "Sınıf ekle", n < CLASSES_MAX, Event::Add),
            tool("copy", "Sınıfın kopyası", n < CLASSES_MAX, Event::Copy),
            tool("chevronUp", "Yukarı", w.selected > 0, Event::Up),
            tool("chevronDown", "Aşağı", w.selected + 1 < n, Event::Down),
            tool("trash", "Sınıfı sil", n > 1, Event::Remove),
        ]
        .spacing(2);
        let c = &w.classes[w.selected];
        let name_text = w
            .typed
            .get(&Key::Name)
            .cloned()
            .unwrap_or_else(|| c.name.clone());
        let when_text = w
            .typed
            .get(&Key::When)
            .cloned()
            .unwrap_or_else(|| c.when.clone().unwrap_or_default());
        let field = |key: Key, text: String, placeholder: &str| {
            kentos_ui::widget::text_field(placeholder, &text)
                .on_input(move |t| msg(Event::Type(key, t)))
        };
        let builder = kentos_ui::widget::tip(
            button(icon(from_web(Some("expression"))).size(16.0))
                .style(style::button::ghost)
                .padding(kentos_ui::theme::metrics::icon_small(16.0))
                .on_press(msg(Event::Builder(Field::When))),
            kentos_ui::widget::Tip::new("İfade oluşturucu…"),
            iced::widget::tooltip::Position::Top,
        );
        column![
            label::caption("Sınıflar"),
            // The list takes the room left (the second round's T22): its tools, Ad and Koşul stay in sight at the
            // window's first size.
            container(
                scrollable(list)
                    .direction(style::field::body_scrollbar())
                    .height(Fill)
            )
            .height(Fill)
            .width(Fill)
            .style(style::container::field_box),
            tools,
            form::line("Ad", CLASS_NAME, field(Key::Name, name_text, ""), None),
            form::line(
                "Koşul",
                CLASS_NAME,
                row![field(Key::When, when_text, "her nesne"), builder]
                    .spacing(4)
                    .align_y(Center),
                None,
            ),
        ]
        .spacing(6)
        .width(typography::scaled(CLASSES_WIDTH))
        .height(Fill)
        .into()
    }

    /// The window's controls by their words (a trace's `dialog` step): Katman by its path,
    /// Etiketleme's modes, Alan ekle by the field's name, Uygula, Kaydet, Vazgeç.
    pub(crate) fn labelling_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.gis.labelling else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Fill(LAYER, words) => {
                let layers = self.label_layers();
                let i = layers
                    .iter()
                    .position(|id| self.layer_path(id) == words)
                    .ok_or_else(|| format!("“{TITLE}” penceresinde “{words}” katmanı yok"))?;
                Some(msg(Event::Layer(i)))
            }
            Control::Press(words) if MODES.iter().any(|(_, m, _)| *m == words) => MODES
                .iter()
                .find(|(_, m, _)| *m == words)
                .map(|(mode, _, _)| msg(Event::Mode(*mode))),
            Control::Fill(ADD_FIELD, words) => {
                let i = w
                    .fields
                    .iter()
                    .position(|f| f == words)
                    .ok_or_else(|| format!("“{TITLE}” penceresinde “{words}” alanı yok"))?;
                Some(msg(Event::Field(i)))
            }
            Control::Press(APPLY) => w.ready.then(|| msg(Event::Apply)),
            Control::Press(REPORT) => Some(msg(Event::Report)),
            Control::Press(SAVE) => w.ready.then(|| msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// Engel: the layer's objects as obstacles to the labels, their weight and kind.
fn labelling_obstacle(w: &Window) -> Element<'_, Message> {
    let on = w.obstacle.is_some();
    let mut parts = Row::new()
        .spacing(12)
        .align_y(Center)
        .push(label::strong("Engel"))
        .push(words::check(
            on,
            "Nesneleri diğer etiketlere engel",
            Some(msg(Event::Obstacle(!on))),
        ));
    let mut out = Column::new().spacing(4);
    if let Some(o) = &w.obstacle {
        let weight_text = w
            .typed
            .get(&Key::Weight)
            .cloned()
            .unwrap_or_else(|| o.weight.to_string());
        let weight = kentos_ui::widget::text_field("", &weight_text)
            .on_input(|t| msg(Event::Type(Key::Weight, t)))
            .width(typography::scaled(56.0))
            .style(style::field::validated(w.invalid.contains(&Key::Weight)));
        let kinds = [ObstacleKind::Interior, ObstacleKind::Boundary];
        let kind = form::choice(
            &["Alanın içi", "Yalnız sınırı"],
            usize::from(o.kind == Some(ObstacleKind::Boundary)),
            move |i| msg(Event::ObstacleKind(kinds[i])),
        )
        .hints([
            "Alanın içine etiket konmaz.",
            "Etiket alanın içine konabilir, sınırını kesmez.",
        ]);
        parts = parts
            .push(
                row![label::body("Ağırlık"), weight, label::caption("1–10")]
                    .spacing(6)
                    .align_y(Center),
            )
            .push(kind);
        out = out.push(parts).push(label::caption(
            "Önceliği ağırlıktan küçük etiket engeli örtemez; büyüğü örtebilir ama başka yeri yeğler.",
        ));
    } else {
        out = out.push(parts);
    }
    column![kentos_ui::widget::horizontal_divider(), out]
        .spacing(10)
        .into()
}

#[cfg(test)]
#[path = "no_label_tests.rs"]
mod no_label_tests;

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;

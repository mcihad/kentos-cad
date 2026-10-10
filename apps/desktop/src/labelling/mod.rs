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

mod form;

use std::collections::{HashMap, HashSet};

use iced::widget::{Column, Row, button, column, container, row, scrollable, text, text_editor};
use iced::{Bottom, Center, Element, Fill, Length, Task};
use kentos_contracts::{
    CommandResult, LabelClass, LabelObstacle, LabelPlacement, LabelStyle, LabelsMode, LayerLabels,
    LayerNodeType, LayersLabels, ObstacleKind, default_label, layer_labels_problem, style_problem,
};
use kentos_interaction::Level;
use kentos_native_application::label_texts::compile_label_expression;
use kentos_native_application::{ExecutionContext, layers_labels};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tabs::{Tab as TabFace, Tabs};
use kentos_ui::widget::{Dialog as Frame, overlay};
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
const SAVE: &str = "Kaydet";
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
        "Katman etiketlenmez; nesneleri yine de öbür etiketlere engel olabilir.",
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
    lines: Vec<(Kind, String)>,
    ready: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
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
    Cancel,
}

pub(crate) fn msg(event: Event) -> Message {
    Message::Labelling(event)
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
        let mut counts: Vec<(&'static str, usize)> = Vec::new();
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
        most.and_then(|(kind, _)| default_label(kind))
            .unwrap_or_else(|| LabelStyle {
                placement: LabelPlacement::Center,
                size: 10.0,
                ..LabelStyle::default()
            })
    }

    /// The window over a layer, its labelling as the drawing has it.
    fn labelling_window(&self, layer: String, tab: Tab) -> Window {
        let labels = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .and_then(|n| n.style.labels.clone());
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
        let mut w = Window {
            layer,
            mode: labels.as_ref().map_or(LabelsMode::Single, |l| l.mode),
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
        };
        w.rebuild(false);
        self.labelling_check(&mut w);
        w
    }

    /// Etiketler (`layer.labels`): the window over `layer`, else the active layer.
    pub(crate) fn open_labelling(&mut self, layer: Option<String>) {
        let Some(id) = self.label_layer_at_hand(layer) else {
            self.warn(NO_LAYER);
            return;
        };
        let w = self.labelling_window(id, Tab::Text);
        self.labelling = Some(w);
        self.dialog = Some(Dialog::Labelling);
    }

    /// The labelling checked as the contract does (its rules, its expressions); Kaydet and Uygula follow it.
    fn labelling_check(&self, w: &mut Window) {
        let given = w.input();
        let mut said: Vec<String> = Vec::new();
        let compiled = |source: &Option<String>| {
            source
                .as_deref()
                .and_then(|s| compile_label_expression(s).err())
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
        w.lines = vec![(Kind::Ok, format!("{name}: {n} nesne, {what}{obstacle}."))];
    }

    pub(crate) fn labelling_event(&mut self, event: Event) -> Task<Message> {
        let Some(mut w) = self.labelling.take() else {
            return Task::none();
        };
        match event {
            Event::Layer(i) => {
                if let Some(id) = self.label_layers().get(i).cloned() {
                    w = self.labelling_window(id, w.tab);
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
                self.labelling = Some(w);
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
                if w.ready && self.write_labelling(&w) && save {
                    self.dialog = None;
                    return Task::none();
                }
            }
            Event::Cancel => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.labelling_check(&mut w);
        self.labelling = Some(w);
        Task::none()
    }

    /// Writes the labelling through `cad.layers.labels` and says what it did; whether it went (or
    /// nothing changed).
    fn write_labelling(&mut self, w: &Window) -> bool {
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
                    self.say(
                        Level::Success,
                        format!("“{name}” katmanının etiketlemesi yazıldı."),
                    );
                } else {
                    self.say(
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
        let Some(w) = &self.labelling else {
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
        let head = row![
            container(words::field(LAYER, layer, None)).width(Fill),
            words::field(LABELLING, mode, None),
        ]
        .spacing(16)
        .align_y(Bottom);

        let host = Host {
            style: w.current(),
            typed: &w.typed,
            invalid: &w.invalid,
            dictionary: &w.dictionary,
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
                "Katman etiketlenmez. Nesnelerinin öbür etiketlere engel olması için aşağıdaki Engel’i açın.",
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
        overlay::modal(
            Frame::new(TITLE)
                .push(head)
                .fill(body)
                .push(labelling_obstacle(w))
                .push(words::summary(lines))
                .action(words::secondary(APPLY, w.ready.then(|| msg(Event::Apply))))
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, w.ready.then(|| msg(Event::Save))))
                .width(860.0)
                .max_height(640.0),
            msg(Event::Cancel),
        )
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
                    .padding(4)
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
            kentos_ui::widget::focus_ring(
                iced::widget::text_input(placeholder, &text)
                    .on_input(move |t| msg(Event::Type(key, t)))
                    .padding([5, 8])
                    .size(typography::body())
                    .style(style::field::input),
            )
        };
        let builder = kentos_ui::widget::tip(
            button(icon(from_web(Some("expression"))).size(16.0))
                .style(style::button::ghost)
                .padding(4)
                .on_press(msg(Event::Builder(Field::When))),
            kentos_ui::widget::Tip::new("İfade oluşturucu…"),
            iced::widget::tooltip::Position::Top,
        );
        column![
            label::caption("Sınıflar"),
            container(
                scrollable(list)
                    .direction(style::field::body_scrollbar())
                    .height(Length::Shrink)
            )
            .height(typography::scaled(200.0))
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
        .into()
    }

    /// The window's controls by their words (a trace's `dialog` step): Katman by its path,
    /// Etiketleme's modes, Uygula, Kaydet, Vazgeç.
    pub(crate) fn labelling_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.labelling else {
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
            Control::Press(APPLY) => w.ready.then(|| msg(Event::Apply)),
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
            "Nesneleri öbür etiketlere engel",
            Some(msg(Event::Obstacle(!on))),
        ));
    let mut out = Column::new().spacing(4);
    if let Some(o) = &w.obstacle {
        let weight_text = w
            .typed
            .get(&Key::Weight)
            .cloned()
            .unwrap_or_else(|| o.weight.to_string());
        let weight = kentos_ui::widget::focus_ring(
            iced::widget::text_input("", &weight_text)
                .on_input(|t| msg(Event::Type(Key::Weight, t)))
                .padding([5, 8])
                .width(typography::scaled(56.0))
                .size(typography::body())
                .style(style::field::validated(w.invalid.contains(&Key::Weight))),
        );
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

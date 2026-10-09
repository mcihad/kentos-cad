//! Katman süzgeci (docs/adr/0211 §4; the web's `ui/layers/LayerFilterDialog.ts`
//! and `app/layerFilterCommands.ts`).
//!
//! - The window (`layer.filter`): Katman (the active one, or the one the
//!   layer tree's menu named), İfade (İfadeyle seç's language; ε opens the
//!   expression builder on the layer's objects), the object list (Seçimden
//!   al: the layer's selected objects; Listeyi kaldır) and, as they change,
//!   how many of the layer's objects pass or why the condition does not
//!   compile. Kaydet writes through `cad.layers.filter` (one step “Katman
//!   süzgeci”); Süzgeci kaldır takes the filter away.
//! - Seçimden süzgeç (`layer.filterFromSelection`): each layer of the
//!   selected objects (only the one the tree's menu named) gets the list of
//!   its selected objects, its condition kept; one step “Seçimden süzgeç”.
//! - Süzgeci kaldır (`layer.filterClear`).
//! - A new object its layer's filter leaves out is said once for the change
//!   (the store collects them, `Spatial::take_hidden_new`).
//!
//! A filter is the layer's view, as its eye: a locked layer takes one.

use std::time::Duration;

use iced::widget::{Column, button, container, row, text, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{CommandResult, EntityId, LayerFilter, LayerNodeType, LayersFilter};
use kentos_domain::Uuid;
use kentos_interaction::Level;
use kentos_native_application::layer_filter::compile_filter;
use kentos_native_application::{ExecutionContext, layers_filter};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Katman süzgeci";
/// Seçimden süzgeç's undo step.
pub const FROM_SELECTION: &str = "Seçimden süzgeç";
const LAYER: &str = "Katman";
const EXPRESSION: &str = "İfade";
const SAVE: &str = "Kaydet";
const REMOVE: &str = "Süzgeci kaldır";
const CANCEL: &str = "Vazgeç";
const TAKE: &str = "Seçimden al";
const DROP: &str = "Listeyi kaldır";
const HINT: &str = "İfadeyle seç’in dili: uyan nesneler görünür. $sıra ve $ölçek kullanılamaz.";
const PLACEHOLDER: &str = "Nitelik = 'Arsa' ve $alan > 500";
/// Why a command has no layer to filter.
const NO_LAYER: &str = "Süzgeç katmanın olur: etkin katman bir grup ya da servisten çizilen bir katman. Nesneleri olan bir katmanı etkin yapın.";

/// A larger layer's objects are counted once typing rests this long; a smaller one's as it is typed.
const PREVIEW_DELAY: Duration = Duration::from_millis(150);
const COUNT_AT_ONCE: usize = 20_000;

/// The expression field, focused when the window opens.
fn expression_id() -> iced::widget::Id {
    iced::widget::Id::new("katman-suzgeci-ifade")
}

/// The window: the layer, the filter as chosen and what it gives.
#[derive(Debug, Clone)]
pub struct Window {
    layer: String,
    expression: String,
    objects: Vec<EntityId>,
    /// What the summary says, each line with its kind, and whether Kaydet may write.
    lines: Vec<(Kind, String)>,
    ready: bool,
    /// The condition does not compile: the field shows it.
    invalid: bool,
    /// Counting's turn: a count asked before the typing rested is dropped.
    turn: u64,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(usize),
    Expression(String),
    /// The count asked when typing rested, by its turn.
    Count(u64),
    Builder,
    Take,
    Drop,
    Save,
    Remove,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::LayerFilter(event)
}

impl Window {
    /// The filter as chosen; none when it keeps nothing (no condition, no list).
    fn chosen(&self) -> Option<LayerFilter> {
        let e = self.expression.trim();
        (!e.is_empty() || !self.objects.is_empty()).then(|| LayerFilter {
            expression: (!e.is_empty()).then(|| e.to_owned()),
            objects: self.objects.clone(),
        })
    }
}

/// What a filter keeps, said in a few words: ““…” ve 12 nesnelik liste”.
pub(crate) fn filter_text(f: &LayerFilter) -> String {
    let list = (!f.objects.is_empty()).then(|| format!("{} nesnelik liste", f.objects.len()));
    match (&f.expression, list) {
        (Some(e), Some(l)) => format!("“{e}” ve {l}"),
        (Some(e), None) => format!("“{e}”"),
        (None, Some(l)) => l,
        (None, None) => String::new(),
    }
}

impl App {
    /// The layers a filter may go on: those that hold objects, by id.
    fn filter_layers(&self) -> Vec<String> {
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
    fn filter_layer_at_hand(&self, layer: Option<String>) -> Option<String> {
        let doc = self.document.as_ref()?;
        let layers = doc.model.layers();
        let id = layer.unwrap_or_else(|| layers.active().to_owned());
        layers
            .get(&id)
            .filter(|n| n.kind == LayerNodeType::Layer && n.service.is_none())
            .map(|n| n.id.clone())
    }

    /// The layer's selected objects' persistent ids.
    fn selected_on(&self, layer: &str) -> Vec<EntityId> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        self.selection
            .ids()
            .iter()
            .filter(|s| {
                doc.model
                    .get(**s)
                    .is_some_and(|e| e.base().layer_id == layer)
            })
            .filter_map(|s| doc.model.uid(*s).map(|u| EntityId(*u.as_bytes())))
            .collect()
    }

    /// Katman süzgeci (`layer.filter`): the window over `layer`, else the active layer.
    pub(crate) fn open_layer_filter(&mut self, layer: Option<String>) -> Task<Message> {
        let Some(id) = self.filter_layer_at_hand(layer) else {
            self.warn(NO_LAYER);
            return Task::none();
        };
        let own = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&id))
            .and_then(|n| n.filter.clone())
            .unwrap_or_default();
        let mut w = Window {
            layer: id,
            expression: own.expression.unwrap_or_default(),
            objects: own.objects,
            lines: Vec::new(),
            ready: false,
            invalid: false,
            turn: 0,
        };
        self.layer_filter_check(&mut w, true);
        self.layer_filter = Some(w);
        self.dialog = Some(Dialog::LayerFilter);
        iced::widget::operation::focus(expression_id())
    }

    /// The filter checked at once (Kaydet follows it as it is typed); with `count`, how many of the
    /// layer's objects pass counted too.
    fn layer_filter_check(&self, w: &mut Window, count: bool) {
        w.invalid = false;
        let Some(f) = w.chosen() else {
            w.ready = false;
            w.lines = vec![(
                Kind::Info,
                "Bir ifade yazın ya da seçimden bir liste alın. Süzgeci kaldırmak için Süzgeci kaldır."
                    .to_owned(),
            )];
            return;
        };
        if let Some(problem) = f.problem() {
            w.ready = false;
            w.lines = vec![(Kind::Warn, format!("Süzgeç: {problem}."))];
            return;
        }
        let compiled = match compile_filter(&f) {
            Ok(c) => c,
            Err(why) => {
                w.ready = false;
                w.invalid = true;
                w.lines = vec![(Kind::Error, format!("İfade: {why}"))];
                return;
            }
        };
        w.ready = true;
        if !count {
            return;
        }
        let Some(doc) = &self.document else {
            return;
        };
        let (passed, total) = layers_filter::count(&doc.model, &w.layer, Some(&compiled));
        let mut lines = vec![(
            if passed > 0 { Kind::Ok } else { Kind::Warn },
            format!(
                "{passed} / {total} nesne süzgeçten geçiyor{}.",
                if passed > 0 {
                    ""
                } else {
                    "; katmanda hiçbir nesne görünmeyecek"
                }
            ),
        )];
        let known = w
            .objects
            .iter()
            .filter(|id| doc.model.slot_of(Uuid::from_bytes(id.0)).is_some())
            .count();
        if known < w.objects.len() {
            lines.push((
                Kind::Info,
                format!(
                    "Listedeki {} nesne çizimde yok (silinmiş olabilir); listede kalır.",
                    w.objects.len() - known
                ),
            ));
        }
        w.lines = lines;
    }

    pub(crate) fn layer_filter_event(&mut self, event: Event) -> Task<Message> {
        let Some(mut w) = self.layer_filter.take() else {
            return Task::none();
        };
        let mut later = Task::none();
        match event {
            Event::Layer(i) => {
                let Some(id) = self.filter_layers().get(i).cloned() else {
                    self.layer_filter = Some(w);
                    return Task::none();
                };
                let own = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.layers().get(&id))
                    .and_then(|n| n.filter.clone())
                    .unwrap_or_default();
                w.layer = id;
                w.expression = own.expression.unwrap_or_default();
                w.objects = own.objects;
                self.layer_filter_check(&mut w, true);
            }
            Event::Expression(text) => {
                w.expression = text;
                w.turn += 1;
                let large = self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.model.by_layer(&w.layer).nth(COUNT_AT_ONCE).is_some());
                self.layer_filter_check(&mut w, !large);
                if large && w.ready {
                    later = crate::hover_card::after(PREVIEW_DELAY, msg(Event::Count(w.turn)));
                }
            }
            Event::Count(turn) => {
                if turn == w.turn {
                    self.layer_filter_check(&mut w, true);
                }
            }
            Event::Builder => {
                self.layer_filter = Some(w);
                return self.open_builder_for_layer_filter();
            }
            Event::Take => {
                w.objects = self.selected_on(&w.layer);
                self.layer_filter_check(&mut w, true);
            }
            Event::Drop => {
                w.objects.clear();
                self.layer_filter_check(&mut w, true);
            }
            Event::Save => {
                self.layer_filter_check(&mut w, true);
                if let Some(f) = w.chosen().filter(|_| w.ready)
                    && self.write_filter(&w.layer, Some(f))
                {
                    self.dialog = None;
                    return Task::none();
                }
            }
            Event::Remove => {
                self.write_filter(&w.layer, None);
                self.dialog = None;
                return Task::none();
            }
            Event::Cancel => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.layer_filter = Some(w);
        later
    }

    /// Writes a layer's filter (none: takes it away) through `cad.layers.filter` and says what it did;
    /// whether it was written.
    fn write_filter(&mut self, layer: &str, filter: Option<LayerFilter>) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let model = &mut doc.model;
        let name = model
            .layers()
            .get(layer)
            .map_or_else(|| layer.to_owned(), |n| n.name.clone());
        let had = model
            .layers()
            .get(layer)
            .is_some_and(|n| n.filter.is_some());
        let removing = filter.is_none();
        let input = LayersFilter {
            layer: layer.to_owned(),
            filter,
            expected_revision: None,
        };
        match layers_filter::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed { output, .. } => {
                let (level, said) = match (removing, output.changed) {
                    (true, _) if !had => (Level::Info, format!("“{name}” katmanının süzgeci yok.")),
                    (true, _) => (
                        Level::Success,
                        format!(
                            "“{name}” katmanının süzgeci kaldırıldı; {} nesnesinin hepsi görünür.",
                            output.total
                        ),
                    ),
                    (false, false) => (
                        Level::Info,
                        format!("“{name}” katmanının süzgeci zaten böyle."),
                    ),
                    (false, true) => (
                        Level::Success,
                        format!(
                            "“{name}” katmanının süzgeci yazıldı: {} / {} nesne görünür.",
                            output.passed, output.total
                        ),
                    ),
                };
                self.say(level, said);
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

    /// Süzgeci kaldır (`layer.filterClear`): the named layer's filter, else the active layer's.
    pub(crate) fn clear_layer_filter(&mut self, layer: Option<String>) {
        match self.filter_layer_at_hand(layer) {
            Some(id) => {
                self.write_filter(&id, None);
            }
            None => self.warn(NO_LAYER),
        }
    }

    /// Seçimden süzgeç (`layer.filterFromSelection`, docs/adr/0211 §1): each layer of the selected objects
    /// (only `only`, the layer tree's menu's) gets the list of its selected objects; a condition it has stays.
    /// One undo step, all layers or none.
    pub(crate) fn filter_from_selection(&mut self, only: Option<String>) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let mut by_layer: Vec<(String, Vec<EntityId>)> = Vec::new();
        for slot in self.selection.ids() {
            let Some(e) = doc.model.get(*slot) else {
                continue;
            };
            let layer = &e.base().layer_id;
            if only.as_ref().is_some_and(|o| o != layer) {
                continue;
            }
            let Some(uid) = doc.model.uid(*slot) else {
                continue;
            };
            let id = EntityId(*uid.as_bytes());
            match by_layer.iter_mut().find(|(l, _)| l == layer) {
                Some((_, list)) => list.push(id),
                None => by_layer.push((layer.clone(), vec![id])),
            }
        }
        if by_layer.is_empty() {
            self.warn("Seçili nesne yok; önce süzgeçte kalacak nesneleri seçin.");
            return;
        }
        let inputs: Vec<LayersFilter> = by_layer
            .iter()
            .map(|(layer, objects)| LayersFilter {
                layer: layer.clone(),
                filter: Some(LayerFilter {
                    expression: doc
                        .model
                        .layers()
                        .get(layer)
                        .and_then(|n| n.filter.as_ref())
                        .and_then(|f| f.expression.clone()),
                    objects: objects.clone(),
                }),
                expected_revision: None,
            })
            .collect();
        let names: Vec<String> = by_layer
            .iter()
            .map(|(layer, objects)| {
                let name = doc
                    .model
                    .layers()
                    .get(layer)
                    .map_or_else(|| layer.clone(), |n| n.name.clone());
                format!("“{name}” ({})", objects.len())
            })
            .collect();
        let model = &mut doc.model;
        for input in &inputs {
            if let CommandResult::Failed { error } | CommandResult::Conflict { error } =
                layers_filter::validate(&ExecutionContext::new(model), input)
            {
                self.warn(error.message);
                return;
            }
        }
        let group = model.begin_group(FROM_SELECTION);
        let mut changed = 0;
        for input in inputs {
            if let CommandResult::Completed { output, .. } =
                layers_filter::execute(&mut ExecutionContext::new(model), input)
                && output.changed
            {
                changed += 1;
            }
        }
        model.end_group(group);
        let names = names.join(", ");
        if changed == 0 {
            self.say(
                Level::Info,
                format!("{names} katmanlarının süzgeci zaten bu nesneler."),
            );
        } else {
            self.say(
                Level::Success,
                format!(
                    "Süzgeç seçimden yazıldı: {names}. Yalnız bu nesneler görünür; Süzgeci kaldır ile hepsi döner."
                ),
            );
        }
    }

    /// The new objects their layer's filter leaves out, said once for the change (docs/adr/0211 §3).
    pub(crate) fn say_hidden_new(&mut self) {
        let hidden = self.spatial.take_hidden_new();
        if hidden.is_empty() {
            return;
        }
        let Some(doc) = &self.document else {
            return;
        };
        let lines: Vec<String> = hidden
            .into_iter()
            .map(|(layer, n)| {
                let name = doc
                    .model
                    .layers()
                    .get(&layer)
                    .map_or_else(|| layer.clone(), |n| n.name.clone());
                format!(
                    "“{name}” katmanının süzgecinden geçmeyen {n} yeni nesne görünmüyor; Süzgeç… ya da Süzgeci kaldır."
                )
            })
            .collect();
        for line in lines {
            self.warn(line);
        }
    }

    /// The expression field's text, which the builder edits.
    pub(crate) fn layer_filter_expression(&self) -> Option<(String, String)> {
        self.layer_filter
            .as_ref()
            .map(|w| (w.layer.clone(), w.expression.clone()))
    }

    pub(crate) fn layer_filter_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.layer_filter, &self.document) else {
            return text("").into();
        };
        let layers = self.filter_layers();
        let layer_choices: Vec<Choice> = layers
            .iter()
            .map(|id| Choice::new(self.layer_path(id)))
            .collect();
        let layer = Select::new(
            layer_choices,
            layers.iter().position(|id| *id == w.layer),
            |i| msg(Event::Layer(i)),
        );
        let input = text_input(PLACEHOLDER, &w.expression)
            .id(expression_id())
            .size(typography::body())
            .padding([5, 8])
            .on_input(|t| msg(Event::Expression(t)))
            .on_submit_maybe(w.ready.then(|| msg(Event::Save)))
            .style(style::field::validated(w.invalid));
        let builder = kentos_ui::widget::tip(
            button(icon(from_web(Some("expression"))).size(16.0))
                .style(style::button::ghost)
                .padding(4)
                .on_press(msg(Event::Builder)),
            kentos_ui::widget::Tip::new("İfade oluşturucu…"),
            iced::widget::tooltip::Position::Top,
        );
        let here = self.selected_on(&w.layer).len();
        let small = |glyph: &'static str, caption: String, on: Option<Message>| {
            button(
                row![icon(from_web(Some(glyph))).size(14.0), label::body(caption)]
                    .spacing(6)
                    .align_y(Center),
            )
            .on_press_maybe(on)
            .padding([3, 10])
            .style(style::button::secondary)
        };
        let list = row![
            container(label::muted(if w.objects.is_empty() {
                "Liste yok: ifadeye uyan bütün nesneler".to_owned()
            } else {
                format!("Seçimden: {} nesne", w.objects.len())
            }))
            .width(Fill),
            small(
                "layerFilterSelection",
                if here > 0 {
                    format!("{TAKE} ({here})")
                } else {
                    TAKE.to_owned()
                },
                (here > 0).then(|| msg(Event::Take)),
            ),
            small(
                "close",
                DROP.to_owned(),
                (!w.objects.is_empty()).then(|| msg(Event::Drop)),
            ),
        ]
        .spacing(6)
        .align_y(Center);
        let lines = w
            .lines
            .iter()
            .map(|(kind, line)| words::text_line(*kind, line.clone()))
            .collect();
        let has_filter = doc
            .model
            .layers()
            .get(&w.layer)
            .is_some_and(|n| n.filter.is_some());
        let body = Column::new()
            .spacing(12)
            .push(container(words::field(LAYER, layer, None)).width(Fill))
            .push(words::field(
                EXPRESSION,
                row![container(input).width(Fill), builder]
                    .spacing(4)
                    .align_y(Center),
                Some(HINT.to_owned()),
            ))
            .push(words::field("Nesne listesi", list, None))
            .push(words::summary(lines));
        overlay::modal(
            Frame::new(TITLE)
                .push(body)
                .action(words::secondary(
                    REMOVE,
                    has_filter.then(|| msg(Event::Remove)),
                ))
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, w.ready.then(|| msg(Event::Save))))
                .width(640.0),
            msg(Event::Cancel),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): Katman by its path, İfade's text
    /// (`fill`), Seçimden al, Listeyi kaldır, Kaydet, Süzgeci kaldır, Vazgeç.
    pub(crate) fn layer_filter_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.layer_filter else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Fill(LAYER, words) => {
                let layers = self.filter_layers();
                let i = layers
                    .iter()
                    .position(|id| self.layer_path(id) == words)
                    .ok_or_else(|| format!("“{TITLE}” penceresinde “{words}” katmanı yok"))?;
                Some(msg(Event::Layer(i)))
            }
            Control::Fill(EXPRESSION, words) => Some(msg(Event::Expression(words.to_owned()))),
            Control::Press(SAVE) => w.ready.then(|| msg(Event::Save)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(TAKE) => Some(msg(Event::Take)),
            Control::Press(DROP) => Some(msg(Event::Drop)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

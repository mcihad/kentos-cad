//! A layer's style classes in Katmanlar (docs/adr/9105 §2–§4): a row under the layer for each category and Diğer
//! değerler, each graduated class, each rule (its children deeper), with a box that switches it on and off
//! (`cad.layers.set` `classVisible`, the tree's own change as the eye is) and its symbol's colour. The row's menu:
//! Tümünü göster, Tümünü gizle, Tersine çevir; Bu sınıfın nesnelerini seç; Öznitelik tablosunda göster; Renk ▸ (the
//! class's symbol in that colour, a style change: `cad.layers.renderer`, an undo step); Sembolü düzenle… (Katman
//! stili). Nesne sayısını göster (the layer's menu, `classCounts`) adds `[n]` to the layer and to each class: a drawing
//! layer's objects counted here, a data layer's rows read from its source off the interface's thread, again when its
//! style or its rows change.

use std::collections::HashMap;

use iced::Task;
use kentos_commands::{LayerNode, LayerSetOperation, LayersSet};
use kentos_native_application::ExecutionContext;
use kentos_native_application::style_classes::{ClassReader, StyleClass, style_classes};
use kentos_ui::widget::context_menu::Menu;
use kentos_ui::widget::tree_view::Node;
use serde_json::Value;

use crate::app::{App, Message};
use crate::document::Document;

/// The most rows of a data layer Bu sınıfın nesnelerini seç takes, as Seç does.
const SELECT_MOST: usize = 10_000;

/// A layer's counts as read: each class's (or why it could not be counted) and the layer's; or why none was read.
pub type CountsAnswer = Result<(Vec<Result<u64, String>>, u64), String>;

/// A layer's counts: what they were asked for (the style and the data as they were), and the answer.
#[derive(Debug, Default)]
struct Counted {
    key: String,
    answer: Option<CountsAnswer>,
}

/// The counted layers, by id.
#[derive(Debug, Default)]
pub(crate) struct LayerClasses {
    counts: HashMap<String, Counted>,
}

/// What the class rows and their menu say.
#[derive(Debug, Clone)]
pub enum Event {
    /// A class's box: the layer and the class's id.
    Toggle(String, String),
    /// Tümünü göster (true) or Tümünü gizle (false).
    All(String, bool),
    /// Tersine çevir.
    Invert(String),
    /// Nesne sayısını göster turned over.
    Counts(String),
    /// Bu sınıfın nesnelerini seç: the layer and the class's place.
    Select(String, usize),
    /// Öznitelik tablosunda göster.
    Table(String, usize),
    /// Renk ▸: the class's symbol in this colour.
    Color(String, usize, String),
    /// Sembolü düzenle…: Katman stili on the layer.
    Symbol(String),
    /// A class's row pressed: the layer and the class's place. A second press soon after opens the class's symbol in
    /// the symbol designer over Katman stili (docs/adr/9110 §3).
    Pressed(String, usize),
    /// A data layer's counts read: the layer, what they were asked for, the answer.
    Counted(String, String, CountsAnswer),
    /// A data layer's class's rows read for Seç: the layer, the class's label, the keys and whether there were more.
    Keys(String, String, Result<(Vec<i64>, bool), String>),
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::LayerClasses(e))
}

/// The classes of a layer's style.
fn classes_of(node: &LayerNode) -> Vec<StyleClass> {
    node.style.renderer.as_ref().map(style_classes).unwrap_or_default()
}

/// What a layer's counts depend on: its style, and its rows (a data layer's writes) or the drawing (a drawing layer's
/// objects).
fn count_key(doc: &Document, node: &LayerNode) -> String {
    let style = node.style.renderer.as_ref().map(Value::to_string).unwrap_or_default();
    let filter = node.filter.as_ref().and_then(|f| f.expression.clone()).unwrap_or_default();
    match &node.data {
        Some(d) => format!("{}|{}|{}|{}", style, d.address, d.table, doc.model.data_written()),
        None => format!("{}|{}|{}", style, filter, doc.model.generation()),
    }
}

/// The symbol a class shows: its library symbol when it names one.
fn resolved<'v>(doc: &'v Document, symbol: &'v Value) -> &'v Value {
    match kentos_native_style::renderer::ref_id(symbol) {
        Some(id) => doc
            .model
            .styles()
            .items
            .iter()
            .find(|i| i.get("id").and_then(Value::as_str) == Some(id))
            .and_then(|i| i.get("symbol"))
            .unwrap_or(symbol),
        None => symbol,
    }
}

/// A symbol's main colour: its first layer's fill, line or mark colour.
fn main_color(symbol: &Value) -> Option<String> {
    for layer in symbol.get("layers").and_then(Value::as_array).into_iter().flatten() {
        let found = match layer.get("type").and_then(Value::as_str).unwrap_or_default() {
            "simpleFill" | "hatchFill" | "gradientFill" | "simpleLine" | "text" => {
                layer.get("color").and_then(Value::as_str).map(str::to_owned)
            }
            "shape" | "svg" => layer
                .get("fill")
                .or_else(|| layer.get("stroke"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            _ => layer
                .get("marker")
                .and_then(|m| main_color(m).or_else(|| m.get("layers").and_then(|_| main_color(m)))),
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// The symbol slot that speaks for a class on the layer: an area's fill, a line's line, a point's mark; else the first
/// it has.
fn slot_of(node: &LayerNode, symbols: &Value) -> Option<&'static str> {
    let wanted = match node.geometry {
        Some(kentos_commands::LayerGeometry::Area) => Some("fill"),
        Some(kentos_commands::LayerGeometry::Line) => Some("line"),
        Some(kentos_commands::LayerGeometry::Point) => Some("marker"),
        _ => None,
    };
    wanted
        .filter(|w| symbols.get(*w).is_some_and(|s| !s.is_null()))
        .or_else(|| ["fill", "line", "marker"].into_iter().find(|k| symbols.get(*k).is_some_and(|s| !s.is_null())))
}

impl App {
    /// The colour drawn beside a class: its symbol's main colour.
    fn class_color(&self, doc: &Document, node: &LayerNode, class: &StyleClass) -> Option<iced::Color> {
        let symbols = class.symbols.as_ref()?;
        let symbol = resolved(doc, symbols.get(slot_of(node, symbols)?)?);
        main_color(symbol).map(|c| self.drawing_color(&c))
    }

    /// What a counted layer's counts were last asked for (a test hands a data layer's answer in itself).
    #[cfg(test)]
    pub(crate) fn class_count_key(&self, layer: &str) -> Option<String> {
        self.gis.layer_classes.counts.get(layer).map(|c| c.key.clone())
    }

    /// `[n]` for a counted layer and its classes: none while it is not counted or its counts are not read yet.
    pub(crate) fn class_total(&self, node: &LayerNode) -> Option<u64> {
        if node.class_counts != Some(true) {
            return None;
        }
        match &self.gis.layer_classes.counts.get(&node.id)?.answer {
            Some(Ok((_, total))) => Some(*total),
            _ => None,
        }
    }

    fn class_count(&self, node: &LayerNode, at: usize) -> Option<Result<u64, String>> {
        if node.class_counts != Some(true) {
            return None;
        }
        match &self.gis.layer_classes.counts.get(&node.id)?.answer {
            Some(Ok((counts, _))) => counts.get(at).cloned(),
            Some(Err(why)) => Some(Err(why.clone())),
            None => None,
        }
    }

    /// A class's row under its layer.
    pub(crate) fn class_node<'a>(
        &'a self,
        doc: &'a Document,
        node: &'a LayerNode,
        at: usize,
        parent_visible: bool,
    ) -> Node<'a, Message> {
        let classes = classes_of(node);
        let Some(class) = classes.get(at) else {
            return Node::new("");
        };
        let counted = self.class_count(node, at);
        let label = match &counted {
            Some(Ok(n)) => format!("{} [{}]", class.label, crate::view::thousands(*n as f64)),
            _ => class.label.clone(),
        };
        let mut tip = class
            .expression
            .as_ref()
            .map_or_else(|| "Katmanın bütün nesneleri".to_owned(), |e| format!("Koşul: {e}"));
        if let Some(Err(why)) = &counted {
            tip = format!("{tip}\nSayılamadı: {why}");
        }
        if !class.enabled {
            tip = format!("{tip}\nKapalı: nesneleri çizilmez");
        }
        let face: iced::Element<'_, Message> = match self.class_color(doc, node, class) {
            Some(c) => match node.geometry {
                Some(g @ (kentos_commands::LayerGeometry::Line | kentos_commands::LayerGeometry::Point)) => {
                    kentos_ui::icon::icon(crate::layer_types::glyph(Some(g))).size(14.0).color(c).into()
                }
                _ => kentos_ui::widget::swatch(c),
            },
            None => kentos_ui::icon::icon(crate::icons::from_web(Some("layerStyle")))
                .size(12.0)
                .tone(kentos_ui::icon::Tone::Muted)
                .into(),
        };
        let layer = node.id.clone();
        let id = class.id.clone();
        Node::new(label)
            .tip(kentos_ui::widget::Tip::new(tip))
            .icon(face)
            .check(class.enabled, message(Event::Toggle(layer.clone(), id)))
            .on_press(message(Event::Pressed(layer, at)))
            .muted(!parent_visible || !class.enabled)
            .menu(move |_| self.class_menu(node, at))
    }

    /// A class row's menu.
    fn class_menu(&self, node: &LayerNode, at: usize) -> Menu<Message> {
        use kentos_ui::icon::Icon;
        let id = node.id.clone();
        let icon = |name: &str| crate::icons::from_web(Some(name));
        let class = classes_of(node).into_iter().nth(at);
        let colors = crate::ribbon_panels::DRAW_COLORS.iter().fold(Menu::new(), |menu, (name, value)| {
            menu.item(*name, message(Event::Color(id.clone(), at, (*value).to_owned())))
                .swatch(self.drawing_color(value))
        });
        Menu::new()
            .item("Tümünü göster", message(Event::All(id.clone(), true)))
            .icon(Icon::Eye)
            .item("Tümünü gizle", message(Event::All(id.clone(), false)))
            .icon(Icon::EyeOff)
            .item("Tersine çevir", message(Event::Invert(id.clone())))
            .icon(Icon::InvertSelection)
            .separator()
            .item("Bu sınıfın nesnelerini seç", message(Event::Select(id.clone(), at)))
            .icon(icon("select"))
            .item(
                "Öznitelik tablosunda göster",
                class
                    .as_ref()
                    .and_then(|c| c.expression.as_ref())
                    .map(|_| message(Event::Table(id.clone(), at))),
            )
            .icon(icon("table"))
            .separator()
            .submenu("Renk", colors)
            .icon(icon("color"))
            .item("Sembolü düzenle…", message(Event::Symbol(id.clone())))
            .icon(icon("layerStyle"))
            .separator()
            .check(
                "Nesne sayısını göster",
                node.class_counts == Some(true),
                message(Event::Counts(id)),
            )
    }

    /// A class's symbol in the symbol designer over Katman stili (docs/adr/9110 §3): the window opens on the layer's
    /// style, the designer on the class's slot for the layer's geometry; Kaydet puts the symbol back in the window and
    /// Tamam writes the style (one undo step), as from the window itself.
    fn edit_class_symbol(&mut self, layer: String, at: usize) -> Task<Message> {
        use kentos_native_style::renderer::GeometryClass as G;
        let opened = self.update(Message::LayerStyle(crate::style::layer_style::Event::Open(Some(layer.clone()))));
        let Some(node) = self.document.as_ref().and_then(|d| d.model.layers().get(&layer)).cloned() else {
            return opened;
        };
        let Some(class) = classes_of(&node).into_iter().nth(at) else {
            return opened;
        };
        let Some(window) = self.styles.layer_style.as_ref() else {
            return opened;
        };
        let geometry = match node.geometry {
            Some(kentos_commands::LayerGeometry::Point) => G::Marker,
            Some(kentos_commands::LayerGeometry::Line) => G::Line,
            Some(kentos_commands::LayerGeometry::Area) => G::Fill,
            _ => window.classes.first().copied().unwrap_or(G::Fill),
        };
        let Some((slot, set)) = window.slot_of_class(&class.id) else {
            return opened;
        };
        let Some(symbol) = set.and_then(|s| s.get(geometry)).or_else(|| window.simple.get(geometry)).cloned() else {
            return opened;
        };
        let designer = self.update(Message::LayerStyle(crate::style::layer_style::Event::Design(
            slot,
            geometry,
            class.label,
            symbol,
        )));
        Task::batch([opened, designer])
    }

    pub(crate) fn layer_classes_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Toggle(layer, id) => {
                let Some(on) = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.layers().get(&layer))
                    .and_then(|n| classes_of(n).into_iter().find(|c| c.id == id))
                    .map(|c| !c.enabled)
                else {
                    return Task::none();
                };
                self.switch_classes(&layer, LayerSetOperation::ClassVisible, Some(vec![id]), Some(on));
            }
            Event::All(layer, on) => self.switch_classes(&layer, LayerSetOperation::ClassVisible, None, Some(on)),
            Event::Invert(layer) => self.switch_classes(&layer, LayerSetOperation::ClassInvert, None, None),
            Event::Counts(layer) => {
                let on = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.layers().get(&layer))
                    .is_some_and(|n| n.class_counts != Some(true));
                self.switch_classes(&layer, LayerSetOperation::ClassCounts, None, Some(on));
            }
            Event::Select(layer, at) => return self.select_class(&layer, at),
            Event::Table(layer, at) => return self.class_in_table(&layer, at),
            Event::Color(layer, at, color) => self.class_colored(&layer, at, &color),
            Event::Pressed(layer, at) => {
                // Its own key: a press on the layer's row and then on its class is not a double one.
                let key = format!("{layer}\u{0}{at}");
                let now = std::time::Instant::now();
                let double = self
                    .last_layer_press
                    .as_ref()
                    .is_some_and(|(last, t)| *last == key && now.duration_since(*t) <= crate::layering::DOUBLE_CLICK);
                self.last_layer_press = (!double).then_some((key, now));
                self.row_chosen(&layer);
                if double {
                    return self.edit_class_symbol(layer, at);
                }
            }
            Event::Symbol(layer) => {
                return self.update(Message::LayerStyle(crate::style::layer_style::Event::Open(Some(layer))));
            }
            Event::Counted(layer, key, answer) => {
                let counted = self.gis.layer_classes.counts.entry(layer).or_default();
                if counted.key == key {
                    counted.answer = Some(answer);
                }
            }
            Event::Keys(layer, label, answer) => match answer {
                Ok((keys, more)) => {
                    let n = keys.len();
                    // The class's rows are the selection now, as Bu sınıfın nesnelerini seç says.
                    self.gis.features.data.selected.clear();
                    self.gis.features.data.select_rows(&layer, keys);
                    let words = if more {
                        format!("“{label}”: ilk {} satır seçildi (daha fazlası var).", crate::view::thousands(n as f64))
                    } else {
                        format!("“{label}”: {} satır seçildi.", crate::view::thousands(n as f64))
                    };
                    self.output(words);
                }
                Err(why) => self.warn(why),
            },
        }
        Task::none()
    }

    /// `cad.layers.set` on a layer's classes; its refusal said.
    fn switch_classes(&mut self, layer: &str, operation: LayerSetOperation, classes: Option<Vec<String>>, value: Option<bool>) {
        let mut input = LayersSet::new(operation, Some(layer.to_owned()));
        input.classes = classes;
        input.value = value;
        self.set_layers(input);
    }

    /// Bu sınıfın nesnelerini seç: a drawing layer's objects selected at once, a data layer's rows read off the thread
    /// and selected in its table (outlined on the map).
    fn select_class(&mut self, layer: &str, at: usize) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(node) = doc.model.layers().get(layer).cloned() else {
            return Task::none();
        };
        let label = classes_of(&node).get(at).map(|c| c.label.clone()).unwrap_or_default();
        if node.data.is_some() {
            let settings = doc.model.settings().clone();
            let id = layer.to_owned();
            return crate::db_connections::off_thread(
                move || {
                    kentos_native_application::layer_classes::data_class_keys(&node, &settings, at, SELECT_MOST, &|| false)
                },
                move |answer| message(Event::Keys(id, label, answer)),
            );
        }
        let Some(reader) = node.style.renderer.as_ref().and_then(|r| {
            ClassReader::new(r, kentos_native_application::expression_unit::reading_unit(doc.model.settings()))
        }) else {
            return Task::none();
        };
        let list: Vec<&kentos_commands::Entity> = doc.model.by_layer(layer).collect();
        let shapes: Vec<_> = if reader.reads_geometry() {
            list.iter().map(|e| kentos_native_application::geometry::shape(e)).collect()
        } else {
            Vec::new()
        };
        let names = |id: &str| doc.model.layers().get(id).map(|n| n.name.clone()).unwrap_or_default();
        match reader.members(&list, &names, &|i| shapes.get(i), at) {
            Ok(taken) => {
                let slots: Vec<kentos_domain::Slot> = list
                    .iter()
                    .zip(taken)
                    .filter(|(_, t)| *t)
                    .map(|(e, _)| kentos_domain::Slot(e.base().id))
                    .collect();
                let n = slots.len();
                self.selection.set(slots);
                self.output(format!("“{label}”: {} nesne seçildi.", crate::view::thousands(n as f64)));
            }
            Err(why) => self.warn(format!("Sınıfın koşulu okunamadı: {why}")),
        }
        Task::none()
    }

    /// Öznitelik tablosunda göster: the table on the layer, its İfade süzgeci the class's condition.
    fn class_in_table(&mut self, layer: &str, at: usize) -> Task<Message> {
        let Some(node) = self.document.as_ref().and_then(|d| d.model.layers().get(layer)).cloned() else {
            return Task::none();
        };
        let Some(expression) = classes_of(&node).get(at).and_then(|c| c.expression.clone()) else {
            return Task::none();
        };
        self.gis.features.layer = Some(layer.to_owned());
        self.open_feature_table();
        if node.data.is_some() {
            self.gis.features.filter = expression;
            self.data_table_event(crate::features::data::DataEvent::Apply)
        } else {
            self.features_event(crate::features::Event::Filter(expression))
        }
    }

    /// Renk ▸: the class's symbol in `color` (a library symbol taken into the class first), written as a style change.
    fn class_colored(&mut self, layer: &str, at: usize, color: &str) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(node) = doc.model.layers().get(layer) else {
            return;
        };
        let Some(mut renderer) = node.style.renderer.clone() else {
            return;
        };
        let Some(class) = classes_of(node).into_iter().nth(at) else {
            return;
        };
        let Some(symbols) = class.symbols.as_ref() else {
            return self.warn("Bu kural yalnız alt kurallarını toplar; kendi sembolü yok.".to_owned());
        };
        let Some(slot) = slot_of(node, symbols) else {
            return;
        };
        let Some(symbol) = symbols.get(slot) else {
            return;
        };
        let colored = kentos_native_style::thematic::with_color(resolved(doc, symbol), color);
        let Some(holder) = symbols_mut(&mut renderer, &class.id) else {
            return;
        };
        holder[slot] = colored;
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = kentos_native_application::layers_renderer::execute(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::LayersRenderer {
                layer: layer.to_owned(),
                renderer: Some(renderer),
                expected_revision: None,
            },
        );
        self.answered(result);
    }

    /// After every message: the counted layers' counts asked again when their style or their data changed (a data
    /// layer's off the thread).
    pub(crate) fn layer_classes_tasks(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.gis.layer_classes.counts.clear();
            return Task::none();
        };
        let mut counted: Vec<&LayerNode> = Vec::new();
        fn walk<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
            for n in nodes {
                if n.class_counts == Some(true) {
                    out.push(n);
                }
                walk(&n.children, out);
            }
        }
        walk(doc.model.layers().nodes(), &mut counted);
        let counts = &mut self.gis.layer_classes.counts;
        counts.retain(|id, _| counted.iter().any(|n| n.id == *id));
        let mut tasks = Vec::new();
        for node in counted {
            let key = count_key(doc, node);
            if counts.get(&node.id).is_some_and(|c| c.key == key) {
                continue;
            }
            counts.insert(
                node.id.clone(),
                Counted {
                    key: key.clone(),
                    answer: None,
                },
            );
            if node.data.is_some() {
                let node = node.clone();
                let settings = doc.model.settings().clone();
                let id = node.id.clone();
                tasks.push(crate::db_connections::off_thread(
                    move || kentos_native_application::layer_classes::data_counts(&node, &settings, &|| false),
                    move |answer| message(Event::Counted(id, key, answer)),
                ));
            } else {
                let answer = kentos_native_application::layer_classes::drawing_counts(&doc.model, node);
                if let Some(c) = counts.get_mut(&node.id) {
                    c.answer = Some(Ok(answer));
                }
            }
        }
        Task::batch(tasks)
    }
}

/// The symbol set of the class `id` in the renderer's JSON.
fn symbols_mut<'v>(renderer: &'v mut Value, id: &str) -> Option<&'v mut Value> {
    if id == kentos_native_application::style_classes::OTHER {
        return renderer.get_mut("other");
    }
    if let Some(i) = id.strip_prefix('c').and_then(|n| n.parse::<usize>().ok()) {
        return renderer.get_mut("categories")?.get_mut(i)?.get_mut("symbols");
    }
    if let Some(i) = id.strip_prefix('g').and_then(|n| n.parse::<usize>().ok()) {
        return renderer.get_mut("classes")?.get_mut(i)?.get_mut("symbols");
    }
    let path: Vec<usize> = id.strip_prefix('r')?.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let (first, rest) = path.split_first()?;
    let mut rule = renderer.get_mut("rules")?.get_mut(*first)?;
    for i in rest {
        rule = rule.get_mut("children")?.get_mut(*i)?;
    }
    rule.get_mut("symbols")
}

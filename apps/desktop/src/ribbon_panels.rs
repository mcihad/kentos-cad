//! The ribbon's own panels, which the web draws itself (`ui/ribbon/panels.ts`'s
//! builtin panels with the fields of `ui/ribbon/fields.ts`; docs/adr/0089):
//!
//! - **Katmanlar** (Giriş): the active layer, a drop-down of the tree's
//!   layers under their groups, with their colours and object counts (a
//!   locked layer cannot be chosen); Yeni katman, Yeni grup, Katmanları
//!   göster and Katman paneli.
//! - **Özellikler** (Giriş): the colour, line type and weight new objects
//!   take (“Katmana göre” at first; kept for the session, as on the web) and
//!   the project's plot scale.
//! - **Seçim** (the contextual Seçim tab): how many objects are selected and
//!   of which kinds; Seçime yakınlaştır, Seçimi kaldır, Seçimi ters çevir.
//!
//! Each steps down with the window as the web's does: the fields shorten, the
//! buttons keep only their icons, then the panel folds into one button whose
//! menu holds the same choices. The colour goes into the tools' draft
//! (`Draft::color`) and so is explicit in every drawing command's input
//! (CMD-07); the line type and weight are kept for the session, which no tool
//! reads yet, on the web either.

use iced::widget::{column, container, row, text};
use iced::{Center, Color, Element, Fill};
use kentos_contracts::{LayerNode, LayerNodeType, LineType};
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Menu;
use kentos_ui::widget::ribbon::{Button, Choice, Group, Level};

use crate::app::{App, Message};
use crate::catalog::{Item, Panel as RibbonPanel};
use crate::document::Document;

use menus::{
    LayerLine, color_menu, kinds_column, layer_menu, line_type_menu, scale_menu, weight_menu,
};

/// The colours new objects can take (the web's `DRAW_COLORS`). `ink` is CAD
/// colour 7: black, drawn white on a dark drawing.
pub(crate) const DRAW_COLORS: [(&str, &str); 8] = [
    ("Siyah", "ink"),
    ("Kırmızı", "#E5484D"),
    ("Sarı", "#F2C94C"),
    ("Yeşil", "#5FBF77"),
    ("Camgöbeği", "#4CC3D9"),
    ("Mavi", "#4F8EF7"),
    ("Eflatun", "#C86DD7"),
    ("Gri", "#8C9AAA"),
];

/// Line types with the web's names (`LINE_TYPE_LABEL`, model/layers.ts).
pub(crate) const LINE_TYPES: [(LineType, &str); 4] = [
    (LineType::Continuous, "Sürekli"),
    (LineType::Dashed, "Kesikli"),
    (LineType::Dashdot, "Noktalı kesik"),
    (LineType::Dotted, "Noktalı"),
];

/// Plot line weights in mm (the web's `LINE_WEIGHTS`).
pub(crate) const LINE_WEIGHTS: [f64; 6] = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7];

/// The plot scales the ribbon offers (the web's `PLOT_SCALES`).
pub(super) const PLOT_SCALES: [f64; 5] = [500.0, 1000.0, 2000.0, 5000.0, 25000.0];

/// What the fields' choices ask; the active layer goes through the layer
/// tree's own event (`layering::Event::Activate`).
#[derive(Debug, Clone)]
pub enum Event {
    Color(Option<&'static str>),
    LineType(Option<LineType>),
    Weight(Option<f64>),
    Scale(f64),
}

/// A line weight as the web writes it: “0.25 mm” (the display rule, docs/adr/0149).
pub(crate) fn weight_text(weight: f64) -> String {
    format!("{} mm", kentos_interaction::fixed(weight, 2))
}

/// The current colour's name, “Katmana göre” without one.
fn color_text(color: Option<&str>) -> &'static str {
    color
        .and_then(|value| DRAW_COLORS.iter().find(|(_, v)| *v == value))
        .map_or("Katmana göre", |(name, _)| name)
}

fn line_type_text(line_type: Option<LineType>) -> &'static str {
    line_type
        .and_then(|t| LINE_TYPES.iter().find(|(lt, _)| *lt == t))
        .map_or("Katmana göre", |(_, name)| name)
}

fn weight_value_text(weight: Option<f64>) -> String {
    weight.map_or_else(|| "Katmana göre".to_owned(), weight_text)
}

impl App {
    /// A field's choice: the session's current properties, or the project's
    /// plot scale (an edit of the drawing, as on the web; never an undo step).
    pub(crate) fn ribbon_panel_event(&mut self, event: Event) {
        match event {
            Event::Color(color) => self.draft.color = color,
            Event::LineType(line_type) => self.new_line_type = line_type,
            // New objects drawn with lines take it (docs/adr/0139).
            Event::Weight(weight) => self.draft.line_weight = weight,
            Event::Scale(scale) => {
                if let Some(doc) = &mut self.document {
                    let mut settings = doc.settings().clone();
                    settings.plot_scale = scale;
                    doc.model.set_settings(settings);
                }
            }
        }
    }

    /// A panel of the web's ribbon that the web draws itself: its group, or
    /// none for one the desktop does not know.
    pub(crate) fn builtin_group(
        &self,
        name: &str,
        panel: &RibbonPanel,
    ) -> Option<Group<'static, Message>> {
        let group = match name {
            "layers" => self.layers_group(panel)?,
            "properties" => self.properties_group(panel)?,
            "selection" => self.selection_group(panel),
            _ => return None,
        };
        let group = group.icon(panel.icon).keep(panel.keep);
        Some(
            match panel
                .launcher
                .as_ref()
                .and_then(|l| crate::view::launch(l).map(|message| (message, l.title)))
            {
                Some((message, title)) => group.launcher(message, title),
                None => group,
            },
        )
    }

    /// A small command button of these panels, as the ribbon's own.
    fn small(&self, id: &'static str) -> Option<Button<'static, Message>> {
        self.ribbon_button(&Item::Command {
            id,
            size: crate::catalog::Size::Small,
        })
    }

    /// Katmanlar: the active layer's field over two rows of two buttons.
    fn layers_group(&self, panel: &RibbonPanel) -> Option<Group<'static, Message>> {
        let doc = self.document.as_ref()?;
        let menu = self.layer_menu_items(doc);
        let layers = doc.model.layers();
        let active = layers.get(layers.active());
        let value = active.map_or_else(String::new, |n| n.name.clone());
        let swatch = active.map(|n| self.drawing_color(&n.style.color));
        let buttons: Vec<Button<'static, Message>> = [
            "layer.new",
            "layer.newGroup",
            "layer.showAll",
            "view.rightPanel",
        ]
        .into_iter()
        .filter_map(|id| self.small(id))
        .collect();
        let s = typography::scaled;
        let field_width = move |level: Level| if level >= 2 { s(150.0) } else { s(204.0) };
        let row_width = |pair: &[Button<'static, Message>], icons: bool| {
            pair.iter()
                .map(|b| b.clone().icon_only(icons).measure())
                .sum::<f32>()
                + ROW_GAP * pair.len().saturating_sub(1) as f32
        };
        let widths = [0, 1, 2].map(|level: Level| {
            let icons = level >= 2;
            buttons
                .chunks(2)
                .map(|pair| row_width(pair, icons))
                .fold(field_width(level), f32::max)
                + STACK_PAD * 2.0
        });
        let field_menu = menu.clone();
        let (field_value, field_swatch) = (value.clone(), swatch);
        let view_buttons = buttons.clone();
        let view = move |level: Level| -> Element<'static, Message> {
            let menu = field_menu.clone();
            let field = Choice::new(field_value.clone(), move || layer_menu(&menu))
                .swatch(field_swatch)
                .width(field_width(level))
                .tip(kentos_ui::widget::Tip::new("Etkin katman").body(
                    "Yeni nesnelerin çizildiği katman. Kilitli katman seçilemez; kilidi Katmanlar panelinden açın.",
                ));
            let mut stack = column![field].spacing(3);
            for pair in view_buttons.chunks(2) {
                stack = stack.push(pair.iter().fold(row![].spacing(ROW_GAP), |row, b| {
                    row.push(b.clone().icon_only(level >= 2))
                }));
            }
            container(stack).padding([0.0, STACK_PAD]).into()
        };
        let folded_buttons = buttons;
        let folded = move || {
            let menu = menu.clone();
            let title = format!("Etkin katman: {value}");
            folded_buttons.iter().fold(
                Menu::new().submenu(title, layer_menu(&menu)).separator(),
                |m, b| b.menu_entry(m),
            )
        };
        Some(Group::new(panel.label).stepped(widths, view, folded))
    }

    /// The layers as the field's menu lists them (the web's `layerField`):
    /// every group a header with its path, every layer a choice with its
    /// colour and object count, a locked one not to be chosen.
    fn layer_menu_items(&self, doc: &Document) -> Vec<LayerLine> {
        let layers = doc.model.layers();
        let mut out = Vec::new();
        let mut stack: Vec<&LayerNode> = layers.nodes().iter().rev().collect();
        while let Some(node) = stack.pop() {
            match node.kind {
                LayerNodeType::Group => {
                    out.push(LayerLine::Header(crate::properties::layer_path(
                        layers, &node.id,
                    )));
                    stack.extend(node.children.iter().rev());
                }
                LayerNodeType::Layer => out.push(LayerLine::Layer {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    color: self.drawing_color(&node.style.color),
                    count: doc.count_below(node),
                    active: layers.active() == node.id,
                    locked: layers.is_locked(&node.id),
                }),
            }
        }
        out
    }

    /// Özellikler: the colour, line type and weight for new objects, and the
    /// plot scale beside them.
    fn properties_group(&self, panel: &RibbonPanel) -> Option<Group<'static, Message>> {
        let doc = self.document.as_ref()?;
        let color = self.draft.color;
        let color_swatch = color.map(|value| self.drawing_color(value));
        let swatches: Vec<Color> = DRAW_COLORS
            .iter()
            .map(|(_, value)| self.drawing_color(value))
            .collect();
        let (line_type, weight) = (self.new_line_type, self.draft.line_weight);
        let scale = doc.settings().plot_scale;
        let s = typography::scaled;
        let fields_width = move |level: Level| match level {
            0 => s(188.0),
            1 => s(162.0),
            _ => s(132.0),
        };
        let scale_width = move |level: Level| if level >= 2 { s(112.0) } else { s(134.0) };
        let widths = [0, 1, 2]
            .map(|level: Level| fields_width(level) + scale_width(level) + 6.0 + STACK_PAD * 4.0);
        let menus = {
            let swatches = swatches.clone();
            move || {
                (
                    color_menu(color, &swatches),
                    line_type_menu(line_type),
                    weight_menu(weight),
                    scale_menu(scale),
                )
            }
        };
        let view_menus = menus.clone();
        let view = move |level: Level| -> Element<'static, Message> {
            let width = fields_width(level);
            let m = view_menus.clone();
            let colors = Choice::new(color_text(color), move || m().0)
                .label("Renk")
                .swatch(color_swatch)
                .width(width)
                .tip(
                    kentos_ui::widget::Tip::new("Renk")
                        .body("Yeni nesnelerin rengi; “Katmana göre” katmanın rengini kullanır."),
                );
            let m = view_menus.clone();
            let types = Choice::new(line_type_text(line_type), move || m().1)
                .label("Tip")
                .width(width)
                .tip(
                    kentos_ui::widget::Tip::new("Çizgi tipi")
                        .body("Yeni nesnelerin çizgi tipi; “Katmana göre” katmanınkini kullanır."),
                );
            let m = view_menus.clone();
            let weights = Choice::new(weight_value_text(weight), move || m().2)
                .label("Kalınlık")
                .width(width)
                .tip(kentos_ui::widget::Tip::new("Çizgi kalınlığı").body(
                    "Yeni nesnelerin çizim kalınlığı; “Katmana göre” katmanınkini kullanır.",
                ));
            let m = view_menus.clone();
            let scales = Choice::new(format!("1:{scale}"), move || m().3)
                .label("Ölçek")
                .width(scale_width(level))
                .tip(kentos_ui::widget::Tip::new("Çizim ölçeği").body(
                    "Projenin çizim ölçeği: yazı boyları ve semboller buna göre çizilir. Proje ayarıdır; proje dosyasıyla saklanır.",
                ));
            row![
                container(column![colors, types, weights].spacing(3)).padding([0.0, STACK_PAD]),
                container(column![scales].spacing(3)).padding([0.0, STACK_PAD]),
            ]
            .spacing(6)
            .into()
        };
        let folded = move || {
            let (colors, types, weights, scales) = menus();
            Menu::new()
                .header("Yeni nesnelerin özellikleri")
                .submenu(format!("Renk: {}", color_text(color)), colors)
                .submenu(format!("Çizgi tipi: {}", line_type_text(line_type)), types)
                .submenu(
                    format!("Çizgi kalınlığı: {}", weight_value_text(weight)),
                    weights,
                )
                .separator()
                .submenu(format!("Çizim ölçeği: 1:{scale}"), scales)
        };
        Some(Group::new(panel.label).stepped(widths, view, folded))
    }

    /// Seçim: the count and the kinds of the selection, then its commands
    /// (the web's selection panel: the kinds give way when narrow).
    fn selection_group(&self, panel: &RibbonPanel) -> Group<'static, Message> {
        let count = self.selection.len();
        let kinds = self.selection_kinds();
        let buttons: Vec<Button<'static, Message>> = [
            "view.zoomSelection",
            "edit.deselect",
            "edit.invertSelection",
        ]
        .into_iter()
        .filter_map(|id| self.small(id))
        .collect();
        let s = typography::scaled;
        let (total_width, kinds_width) = (s(76.0), s(112.0));
        let column_width = |icons: bool| {
            buttons
                .iter()
                .map(|b| b.clone().icon_only(icons).measure())
                .fold(0.0, f32::max)
        };
        let widths = [0, 1, 2].map(|level: Level| {
            let kinds = if level >= 2 { 0.0 } else { kinds_width + 6.0 };
            total_width + kinds + 6.0 + column_width(level >= 2)
        });
        let view_buttons = buttons.clone();
        let view_kinds = kinds.clone();
        let view = move |level: Level| -> Element<'static, Message> {
            let accent = |theme: &iced::Theme| text::Style {
                color: Some(Tokens::of(theme).accent_hover),
            };
            let total = container(
                column![
                    text(count.to_string())
                        .font(typography::ui_strong())
                        .size(s(19.0))
                        .style(accent),
                    label::caption("nesne seçili").size(typography::caption() - 1.0),
                ]
                .spacing(2)
                .align_x(Center),
            )
            .width(total_width)
            .height(Fill)
            .center_y(Fill)
            .align_x(Center);
            let buttons = view_buttons.iter().fold(column![].spacing(1), |col, b| {
                col.push(b.clone().icon_only(level >= 2))
            });
            let mut body = row![total].spacing(6).height(Fill);
            if level < 2 {
                body = body.push(kinds_column(&view_kinds, kinds_width));
            }
            body.push(container(buttons).height(Fill).center_y(Fill))
                .into()
        };
        let folded_buttons = buttons;
        let folded = move || {
            folded_buttons.iter().fold(
                Menu::new().header(format!("{count} nesne seçili")),
                |m, b| b.menu_entry(m),
            )
        };
        Group::new(panel.label).stepped(widths, view, folded)
    }

    /// The selection's kinds, the most first: (how many, the kind's name in
    /// lower case, as the web's `toLocaleLowerCase('tr-TR')`).
    fn selection_kinds(&self) -> Vec<(usize, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut kinds: Vec<(&str, usize)> = Vec::new();
        for slot in self.selection.ids() {
            if let Some(e) = doc.model.get(*slot) {
                match kinds.iter_mut().find(|(k, _)| *k == e.kind()) {
                    Some((_, n)) => *n += 1,
                    None => kinds.push((e.kind(), 1)),
                }
            }
        }
        // Most first; the first seen first among equals (the web's stable sort).
        kinds.sort_by_key(|k| std::cmp::Reverse(k.1));
        kinds
            .into_iter()
            .map(|(kind, n)| {
                (
                    n,
                    crate::layer_tree::lower_tr(crate::selecting::kind_title(kind)),
                )
            })
            .collect()
    }
}

/// The gap between buttons in a row (the web's `.rpanel__row`).
const ROW_GAP: f32 = 2.0;
/// A stack's side padding (the web's `.rpanel__stack`).
const STACK_PAD: f32 = 2.0;

mod menus;
#[cfg(test)]
mod tests;

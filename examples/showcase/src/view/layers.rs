//! Katmanlar paneli: gruplar, katmanlar ve alt katmanlar ağaç tablo olarak.
//!
//! Grupların ve katmanların onay kutusu kendi görünürlüğüdür; üst grubu
//! gizli olduğu için çizilmeyen düğümler sönük yazılır. Bazı alt katmanları
//! gizli olan görünür katmanın kutusu karışıktır. Her düğüme sağ tıklanınca
//! kendi bağlam menüsü açılır.

use iced::widget::{button, column, container, row, slider, space, tooltip};
use iced::{Center, Color, Element, Fill, Right, border};

use kentos_rc::icon::{Icon, Tone, icon};
use kentos_rc::label;
use kentos_rc::spatial::LayerKind;
use kentos_rc::style;
use kentos_rc::widget::tree_view::{self, Check, Node, TreeView};
use kentos_rc::widget::{Tip, swatch, tip};

use crate::app::Showcase;
use crate::layer_tree::{Entry, NodeId};
use crate::message::Message;

impl Showcase {
    /// Katman ağacı ve altında aktif katmanın opaklığı.
    pub(super) fn layer_panel(&self) -> Element<'_, Message> {
        let tree = TreeView::new([
            tree_view::Column::new("Ad").width(Fill),
            tree_view::Column::new("Öğe").width(28).align_right(),
            tree_view::Column::new("").width(20),
        ])
        .extend(
            self.layer_tree
                .roots
                .iter()
                .map(|entry| self.tree_node(*entry)),
        )
        .height(Fill);

        column![tree, self.opacity_footer()].into()
    }

    /// Katmanlar panelinin başlığındaki "tümünü genişlet/daralt" düğmeleri.
    pub(super) fn layer_panel_actions(&self) -> Element<'_, Message> {
        let action = |glyph: Icon, description: &'static str, message: Message| {
            tip(
                button(icon(glyph).size(13.0))
                    .on_press(message)
                    .padding([0, 3])
                    .style(style::button::subtle),
                Tip::new(description),
                tooltip::Position::Bottom,
            )
        };

        row![
            action(
                Icon::ExpandAll,
                "Tümünü genişlet",
                Message::TreeExpandAll(None, true)
            ),
            action(
                Icon::CollapseAll,
                "Tümünü daralt",
                Message::TreeExpandAll(None, false)
            ),
        ]
        .spacing(2)
        .into()
    }

    fn tree_node(&self, entry: Entry) -> Node<'_, Message> {
        match entry {
            Entry::Group(group) => self.group_node(group),
            Entry::Layer(layer) => self.layer_node(layer),
        }
    }

    fn group_node(&self, index: usize) -> Node<'_, Message> {
        let group = &self.layer_tree.groups[index];
        let id = NodeId::Group(index);
        let count: usize = self
            .layer_tree
            .layers_in(index)
            .into_iter()
            .filter_map(|layer| self.layers.get(layer))
            .map(|layer| layer.features.len())
            .sum();

        let mut node = Node::new(group.name.as_str())
            .icon(icon(Icon::Folder).size(14.0).tone(Tone::Muted))
            .check(group.visible, Message::TreeChecked(id, !group.visible))
            .expanded(group.expanded, Message::TreeToggled(id))
            .cells([count_cell(count), zoom_button(id)])
            .selected(self.layer_tree.selected == Some(id))
            .muted(!self.layer_tree.is_shown(Entry::Group(index)))
            .on_press(Message::TreeSelected(id))
            .menu(move |_| self.group_menu(index));

        if group.expanded {
            node = node.extend(group.children.iter().map(|child| self.tree_node(*child)));
        }

        node
    }

    fn layer_node(&self, index: usize) -> Node<'_, Message> {
        let layer = &self.layers[index];
        let id = NodeId::Layer(index);
        let own = self.layer_tree.visible[index];
        let sublayers_hidden = layer.sublayers.iter().any(|sublayer| !sublayer.visible);

        // Görünür katmanın bazı alt katmanları gizliyse kutu karışıktır;
        // tıklamak alt katmanların hepsini gösterir.
        let check = match (own, sublayers_hidden) {
            (false, _) => Check::Unchecked,
            (true, true) => Check::Mixed,
            (true, false) => Check::Checked,
        };

        let mut node = Node::new(layer.name.as_str())
            .icon(symbol(layer.kind, layer.color))
            .check(check, Message::TreeChecked(id, check != Check::Checked))
            .cells([count_cell(layer.features.len()), zoom_button(id)])
            .selected(self.layer_tree.selected == Some(id))
            .muted(!layer.visible)
            .on_press(Message::TreeSelected(id))
            .menu(move |_| self.layer_menu(index));

        if !layer.sublayers.is_empty() {
            let expanded = self.layer_tree.expanded[index];

            node = node.expanded(expanded, Message::TreeToggled(id));

            if expanded {
                node = node.extend(
                    (0..layer.sublayers.len()).map(|sublayer| self.sublayer_node(index, sublayer)),
                );
            }
        }

        node
    }

    fn sublayer_node(&self, layer_index: usize, index: usize) -> Node<'_, Message> {
        let layer = &self.layers[layer_index];
        let sublayer = &layer.sublayers[index];
        let id = NodeId::Sublayer(layer_index, index);

        Node::new(sublayer.name.as_str())
            .icon(swatch(sublayer.color))
            .check(
                sublayer.visible,
                Message::TreeChecked(id, !sublayer.visible),
            )
            .cells([
                count_cell(layer.sublayer_features(index).count()),
                zoom_button(id),
            ])
            .selected(self.layer_tree.selected == Some(id))
            .muted(!(layer.visible && sublayer.visible))
            .on_press(Message::TreeSelected(id))
            .menu(move |_| self.sublayer_menu(layer_index, index))
    }

    /// Aktif katmanın opaklık kaydırıcısı.
    fn opacity_footer(&self) -> Element<'_, Message> {
        let Some(layer) = self.layers.get(self.active_layer) else {
            return space::vertical().height(0).into();
        };

        let index = self.active_layer;

        container(
            row![
                label::muted("Opaklık").width(58),
                slider(0.0..=1.0, layer.opacity, move |value| {
                    Message::LayerOpacity(index, value)
                })
                .step(0.05_f32),
                label::mono_caption(format!("{:>3.0}%", layer.opacity * 100.0))
                    .style(style::text::default)
                    .width(38)
                    .align_x(Right),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .padding([6, 10])
        .width(Fill)
        .into()
    }
}

/// Katmanın lejant simgesi: nokta katmanında dolu daire, çizgi ve alan
/// katmanında türünün ikonu; katmanın renginde.
fn symbol<'a>(kind: LayerKind, color: Color) -> Element<'a, Message> {
    let symbol: Element<'a, Message> = match kind {
        LayerKind::Point => container(space::horizontal())
            .width(9)
            .height(9)
            .style(move |_theme| container::Style {
                background: Some(color.into()),
                border: border::rounded(4.5),
                ..container::Style::default()
            })
            .into(),
        LayerKind::Line => icon(Icon::Polyline).size(14.0).color(color).into(),
        LayerKind::Polygon => icon(Icon::Polygon).size(14.0).color(color).into(),
    };

    container(symbol).center(14).into()
}

fn count_cell<'a>(count: usize) -> Element<'a, Message> {
    label::mono_caption(count.to_string()).into()
}

/// Düğümün öğelerine yakınlaştıran küçük düğme.
fn zoom_button<'a>(node: NodeId) -> Element<'a, Message> {
    tip(
        button(icon(Icon::Target).size(13.0))
            .on_press(Message::ZoomToNode(node))
            .padding([0, 4])
            .style(style::button::subtle),
        Tip::new("Yakınlaştır"),
        tooltip::Position::Left,
    )
}

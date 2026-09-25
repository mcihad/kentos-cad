//! Bağlam menüleri: katman ağacındaki düğümler, model alanı ve öznitelik
//! tablosu satırları için.
//!
//! Menüler her açılışta durumdan kurulur; yapılamayan komutlar devre dışı
//! gösterilir, gizlenmez. Böylece menünün düzeni değişmez ve kullanıcı
//! komutun neden kullanılamadığını görür.

use iced::Point;

use kentos_rc::icon::Icon;
use kentos_rc::spatial::{FeatureRef, SelectionMode, Tool, format, query};
use kentos_rc::widget::Menu;

use crate::app::{DRAWING_LAYER, Showcase};
use crate::layer_tree::NodeId;
use crate::message::{Message, QueryPurpose, Setting};

/// Opaklık alt menüsündeki değerler.
const OPACITIES: [f32; 4] = [1.0, 0.75, 0.5, 0.25];

impl Showcase {
    /// Katman düğümünün menüsü.
    pub(super) fn layer_menu(&self, index: usize) -> Menu<Message> {
        let Some(layer) = self.layers.get(index) else {
            return Menu::new();
        };

        let id = NodeId::Layer(index);
        let own = self.layer_tree.visible.get(index).copied().unwrap_or(false);
        let has_features = !layer.features.is_empty();

        let opacity = OPACITIES.into_iter().fold(Menu::new(), |menu, value| {
            menu.check(
                format!("%{:.0}", value * 100.0),
                (layer.opacity - value).abs() < 0.01,
                Message::LayerOpacity(index, value),
            )
        });

        let mut menu = Menu::new()
            .header(layer.name.clone())
            .item(
                "Katmana yakınlaştır",
                has_features.then_some(Message::ZoomToLayer(index)),
            )
            .icon(Icon::Target)
            .item("Öznitelik tablosunu aç", Message::OpenTable(index))
            .icon(Icon::Table)
            .separator()
            .item(
                "Tümünü seç",
                has_features.then_some(Message::SelectNode(id)),
            )
            .icon(Icon::SelectAll)
            .item(
                "Öznitelikle seç…",
                Message::QueryOpenedFor(QueryPurpose::Select, index),
            )
            .icon(Icon::Filter)
            .separator()
            .item(
                if own { "Gizle" } else { "Göster" },
                Message::TreeChecked(id, !own),
            )
            .icon(if own { Icon::EyeOff } else { Icon::Eye })
            .item("Yalnızca bu katmanı göster", Message::ShowOnly(id))
            .icon(Icon::Eye)
            .submenu("Opaklık", opacity)
            .icon(Icon::Contrast)
            .item("Stil…", Message::StyleOpened(index))
            .icon(Icon::Drop)
            .item("Özellikler…", Message::PropertiesOpened(index))
            .icon(Icon::Properties)
            .item("Yeniden adlandır", Message::RenameStarted(id))
            .icon(Icon::Type)
            .shortcut("F2");

        if !layer.sublayers.is_empty() {
            let expanded = self
                .layer_tree
                .expanded
                .get(index)
                .copied()
                .unwrap_or(false);

            menu = menu
                .submenu(
                    "Alt katmanlar",
                    Menu::new()
                        .item("Hepsini göster", Message::SublayersShown(index, true))
                        .icon(Icon::Eye)
                        .item("Hepsini gizle", Message::SublayersShown(index, false))
                        .icon(Icon::EyeOff)
                        .separator()
                        .item(
                            if expanded {
                                "Ağaçta daralt"
                            } else {
                                "Ağaçta genişlet"
                            },
                            Message::TreeToggled(id),
                        ),
                )
                .icon(Icon::Properties);
        }

        if index == DRAWING_LAYER {
            menu = menu
                .separator()
                .item(
                    "Çizimleri temizle",
                    has_features.then_some(Message::ClearDrawings),
                )
                .icon(Icon::Eraser)
                .danger();
        }

        menu
    }

    /// Grup düğümünün menüsü.
    pub(super) fn group_menu(&self, index: usize) -> Menu<Message> {
        let Some(group) = self.layer_tree.groups.get(index) else {
            return Menu::new();
        };

        let id = NodeId::Group(index);
        let has_features = !self.node_features(id).is_empty();

        Menu::new()
            .header(group.name.clone())
            .item(
                "Gruba yakınlaştır",
                has_features.then_some(Message::ZoomToNode(id)),
            )
            .icon(Icon::Target)
            .item(
                "Gruptaki öğeleri seç",
                has_features.then_some(Message::SelectNode(id)),
            )
            .icon(Icon::SelectAll)
            .separator()
            .item(
                if group.visible { "Gizle" } else { "Göster" },
                Message::TreeChecked(id, !group.visible),
            )
            .icon(if group.visible {
                Icon::EyeOff
            } else {
                Icon::Eye
            })
            .item("Yalnızca bu grubu göster", Message::ShowOnly(id))
            .icon(Icon::Eye)
            .item("Yeniden adlandır", Message::RenameStarted(id))
            .icon(Icon::Type)
            .shortcut("F2")
            .separator()
            .item("Tümünü genişlet", Message::TreeExpandAll(Some(index), true))
            .icon(Icon::ExpandAll)
            .item("Tümünü daralt", Message::TreeExpandAll(Some(index), false))
            .icon(Icon::CollapseAll)
    }

    /// Alt katman düğümünün menüsü.
    pub(super) fn sublayer_menu(&self, layer_index: usize, index: usize) -> Menu<Message> {
        let Some(layer) = self.layers.get(layer_index) else {
            return Menu::new();
        };
        let Some(sublayer) = layer.sublayers.get(index) else {
            return Menu::new();
        };

        let id = NodeId::Sublayer(layer_index, index);
        let has_features = layer.sublayer_features(index).next().is_some();

        Menu::new()
            .header(format!("{}: {}", layer.name, sublayer.name))
            .item(
                "Alt katmana yakınlaştır",
                has_features.then_some(Message::ZoomToNode(id)),
            )
            .icon(Icon::Target)
            .item(
                "Öğelerini seç",
                has_features.then_some(Message::SelectNode(id)),
            )
            .icon(Icon::SelectAll)
            .separator()
            .item(
                if sublayer.visible { "Gizle" } else { "Göster" },
                Message::TreeChecked(id, !sublayer.visible),
            )
            .icon(if sublayer.visible {
                Icon::EyeOff
            } else {
                Icon::Eye
            })
            .item("Yalnızca bu alt katmanı göster", Message::ShowOnly(id))
            .icon(Icon::Eye)
    }

    /// Model alanının menüsü: tıklanan noktanın koordinatı, altındaki öğe,
    /// seçim, araçlar ve görünüm.
    pub(super) fn map_menu(&self, position: Point) -> Menu<Message> {
        let location = self.viewport.unproject(position);
        let has_selection = !self.selection.is_empty();
        let drawings = self.selection.count_in(DRAWING_LAYER);

        let mut menu = Menu::new().header(format::decimal(location));

        let hit = query::hit_test(&self.layers, &self.viewport, position)
            .and_then(|reference| Some((reference, reference.resolve(&self.layers)?)));

        if let Some((reference, (layer, feature))) = hit {
            let selected = self.selection.contains(&reference);

            menu = menu
                .item(
                    format!("{} öğesini seç", layer.label(feature)),
                    Message::SelectFeature(reference, SelectionMode::New),
                )
                .icon(Icon::Select)
                .item(
                    if selected {
                        "Seçimden çıkar"
                    } else {
                        "Seçime ekle"
                    },
                    Message::SelectFeature(
                        reference,
                        if selected {
                            SelectionMode::Remove
                        } else {
                            SelectionMode::Add
                        },
                    ),
                )
                .shortcut(if selected { "Ctrl+tık" } else { "Shift+tık" })
                .item("Öğeye yakınlaştır", Message::FocusFeature(reference))
                .icon(Icon::Target)
                .separator();
        }

        let tools = Tool::NAVIGATION
            .into_iter()
            .fold(Menu::new(), |menu, tool| {
                menu.check(tool.label(), self.tool == tool, Message::ToolSelected(tool))
            })
            .separator();
        let tools = Tool::DRAWING.into_iter().fold(tools, |menu, tool| {
            menu.check(tool.label(), self.tool == tool, Message::ToolSelected(tool))
        });

        let view = Menu::new()
            .item("Tümünü gör", Message::FitAll)
            .icon(Icon::ZoomExtents)
            .item("Yakınlaştır", Message::ZoomIn)
            .icon(Icon::ZoomIn)
            .item("Uzaklaştır", Message::ZoomOut)
            .icon(Icon::ZoomOut)
            .item("Başlangıç görünümü", Message::ResetView)
            .icon(Icon::Home)
            .separator()
            .check("Izgara", self.options.grid, Message::Toggle(Setting::Grid))
            .shortcut("F7")
            .check(
                "Yakalama",
                self.options.snap,
                Message::Toggle(Setting::Snap),
            )
            .shortcut("F3")
            .check(
                "Etiketler",
                self.options.labels,
                Message::Toggle(Setting::Labels),
            );

        menu.item("Koordinatı kopyala", Message::CopyCoordinates(location))
            .icon(Icon::Copy)
            .item("Buraya ortala", Message::CenterAt(location))
            .icon(Icon::Pan)
            .separator()
            .item(
                "Seçime odakla",
                has_selection.then_some(Message::FocusSelection),
            )
            .icon(Icon::Target)
            .item(
                "Seçimi kaldır",
                has_selection.then_some(Message::ClearSelection),
            )
            .icon(Icon::ClearSelection)
            .shortcut("Esc")
            .item(
                "Öznitelikle seç…",
                Message::QueryOpened(QueryPurpose::Select),
            )
            .icon(Icon::Filter)
            .separator()
            .submenu("Araç", tools)
            .icon(self.tool.icon())
            .submenu("Görünüm", view)
            .icon(Icon::ZoomExtents)
            .separator()
            .item(
                "Seçili çizimleri sil",
                (drawings > 0).then_some(Message::DeleteSelection),
            )
            .icon(Icon::Eraser)
            .shortcut("Del")
            .danger()
    }

    /// Öznitelik tablosu satırının menüsü.
    pub(super) fn row_menu(&self, reference: FeatureRef) -> Menu<Message> {
        let Some((layer, feature)) = reference.resolve(&self.layers) else {
            return Menu::new();
        };

        let selected = self.selection.contains(&reference);

        Menu::new()
            .header(layer.label(feature))
            .item("Seç", Message::SelectFeature(reference, SelectionMode::New))
            .icon(Icon::Select)
            .item(
                if selected {
                    "Seçimden çıkar"
                } else {
                    "Seçime ekle"
                },
                Message::SelectFeature(
                    reference,
                    if selected {
                        SelectionMode::Remove
                    } else {
                        SelectionMode::Add
                    },
                ),
            )
            .shortcut("Ctrl+tık")
            .item("Öğeye yakınlaştır", Message::FocusFeature(reference))
            .icon(Icon::Target)
            .separator()
            .item("Satırı kopyala", Message::CopyRow(reference))
            .icon(Icon::Copy)
            .separator()
            .item("Tümünü seç", Message::SelectAll)
            .icon(Icon::SelectAll)
            .shortcut("Ctrl+A")
            .item(
                "Seçimi kaldır",
                (!self.selection.is_empty()).then_some(Message::ClearSelection),
            )
            .icon(Icon::ClearSelection)
    }
}

//! Yan paneller: katmanlar ve özellikler (nesne inceleyici). Ölçüm, harita
//! üstündeki kayan pencerededir.

use iced::widget::{button, column, container, row, space, tooltip};
use iced::{Center, Element, Fill, FillPortion};

use kentos_rc::attribute::{DateTime, FieldKind, ObjectId, text};
use kentos_rc::icon::{Icon, icon};
use kentos_rc::label;
use kentos_rc::spatial::{Geometry, format};
use kentos_rc::style;
use kentos_rc::widget::{Dock, Inspector, Panel, Tip, swatch, tip};

use crate::app::{DOCK_WIDTH, Showcase, TIME_ZONE};
use crate::message::{DockPanel, Message};

impl Showcase {
    pub(super) fn dock(&self) -> Element<'_, Message> {
        let details =
            Panel::new("Özellikler", self.inspector_panel()).meta(match self.selection.len() {
                0 => "Seçim yok".to_owned(),
                count => format!("{count} öğe seçili"),
            });

        Dock::new(self.dock.width)
            .resizable(DOCK_WIDTH, Message::DockResized)
            .on_resize_end(Message::DockResizeEnded)
            .push(
                Panel::new("Katmanlar", self.layer_panel())
                    .meta(format!(
                        "{} katman, {} grup",
                        self.layers.len(),
                        self.layer_tree.groups.len()
                    ))
                    .trailing(self.layer_panel_actions())
                    .collapsible(
                        self.dock.layers_collapsed,
                        Message::PanelToggled(DockPanel::Layers),
                    )
                    .height(FillPortion(5)),
            )
            .push(
                details
                    .collapsible(
                        self.dock.details_collapsed,
                        Message::PanelToggled(DockPanel::Details),
                    )
                    .height(FillPortion(6))
                    .scrollable(),
            )
            .into()
    }

    /// Birincil öğenin nesne inceleyicisi: başlıkta adı ve seçimde gezinme,
    /// altında türlerine göre düzenlenebilir öznitelikler ve geometri.
    fn inspector_panel(&self) -> Element<'_, Message> {
        let Some((primary, (layer, feature))) = self
            .selection
            .primary()
            .and_then(|primary| Some((primary, primary.resolve(&self.layers)?)))
        else {
            return container(
                column![
                    label::muted(
                        "Özelliklerini görmek ve düzenlemek için haritada bir öğeye tıklayın \
                         ya da öznitelik tablosundan bir satır seçin."
                    ),
                    label::caption(
                        "Shift ile seçime ekler, Ctrl ile seçimden çıkarırsınız. Seç aracında \
                         sürükleyerek pencereyle seçebilirsiniz."
                    ),
                ]
                .spacing(8),
            )
            .padding(12)
            .width(Fill)
            .into();
        };

        let mut title = row![label::title(layer.label(feature)).width(Fill)]
            .spacing(2)
            .align_y(Center);

        if let Some((position, total)) = self.selection.position()
            && total > 1
        {
            title = title
                .push(step_button(Icon::ChevronLeft, "Önceki", false))
                .push(label::mono_caption(format!("{position}/{total}")))
                .push(step_button(Icon::ChevronRight, "Sonraki", true));
        }

        title = title.push(
            button(label::caption("Odakla").style(style::text::default))
                .on_press(Message::FocusFeature(primary))
                .padding([2, 8])
                .style(style::button::flat),
        );

        let subtitle = row![
            swatch(layer.color),
            label::muted(layer.name.as_str()),
            space::horizontal(),
            label::mono_caption(format!("OBJECTID {}", feature.id)),
        ]
        .spacing(6)
        .align_y(Center);

        let mut header = column![title, subtitle].spacing(4).padding([8, 10]);

        if self.selection.len() > 1 {
            let summary: Vec<String> = self
                .layers
                .iter()
                .enumerate()
                .filter_map(|(index, layer)| {
                    let count = self.selection.count_in(index);
                    (count > 0).then(|| format!("{} {count}", layer.name))
                })
                .collect();

            header = header.push(label::caption(format!("Seçimde: {}", summary.join(", "))));
        }

        let mut inspector = Inspector::new(&self.inspector, Message::Inspector)
            .now(DateTime::now(TIME_ZONE))
            .category("Öznitelikler");

        for (index, field) in layer.schema.iter().enumerate() {
            let value = feature.value(index);

            inspector = match &field.kind {
                FieldKind::Object { target } => {
                    inspector.object(index, field, value, self.candidates(target), true)
                }
                _ => inspector.field(index, field, value),
            };
        }

        let geometry = &feature.geometry;

        inspector = inspector
            .category("Geometri")
            .fixed("Tür", geometry.label())
            .figure("Köşe sayısı", geometry.vertices().len().to_string());

        inspector = match geometry {
            Geometry::Point(location) => inspector.figure("Konum", format::decimal(*location)),
            Geometry::Line(_) => inspector
                .figure("Uzunluk", format::distance(geometry.length_meters()))
                .figure("Merkez", center(feature)),
            Geometry::Polygon(_) => inspector
                .figure("Çevre", format::distance(geometry.length_meters()))
                .figure("Merkez", center(feature)),
        };

        column![header, inspector].width(Fill).into()
    }

    /// Başvuru alanının adayları: hedef katmanın öğeleri, ada göre sıralı.
    pub(super) fn candidates(&self, target: &str) -> Vec<(ObjectId, String)> {
        let Some(layer) = self.layers.iter().find(|layer| layer.name == target) else {
            return Vec::new();
        };

        let mut candidates: Vec<(ObjectId, String)> = layer
            .features
            .iter()
            .map(|feature| (feature.id, layer.label(feature)))
            .collect();

        candidates.sort_by(|(_, a), (_, b)| text::compare(a, b));
        candidates
    }
}

/// Seçimde önceki ya da sonraki öğeye geçen küçük düğme.
fn step_button<'a>(glyph: Icon, description: &'a str, forward: bool) -> Element<'a, Message> {
    tip(
        button(icon(glyph).size(12.0))
            .on_press(Message::SelectionStep(forward))
            .padding([2, 3])
            .style(style::button::flat),
        Tip::new(description),
        tooltip::Position::Bottom,
    )
}

/// Öğeyi kapsayan kutunun merkezi.
fn center(feature: &kentos_rc::spatial::Feature) -> String {
    feature
        .bounds()
        .map_or_else(String::new, |bounds| format::decimal(bounds.center()))
}

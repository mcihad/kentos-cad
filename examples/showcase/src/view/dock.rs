//! Yan paneller: katmanlar, özellikler (veya ölçüm) ve öğe tablosu.

use iced::widget::{button, checkbox, column, container, row, slider, space, tooltip};
use iced::{Center, Element, Fill, FillPortion, Right};

use kentos_rc::icon::{Icon, icon};
use kentos_rc::label;
use kentos_rc::spatial::{FeatureRef, Tool, format};
use kentos_rc::style;
use kentos_rc::widget::table::{self, Table};
use kentos_rc::widget::{Dock, Panel, PropertyGrid, Tip, swatch, tip};

use crate::app::Showcase;
use crate::message::Message;

const DOCK_WIDTH: f32 = 332.0;

impl Showcase {
    pub(super) fn dock(&self) -> Element<'_, Message> {
        let (title, meta, details) = if self.tool == Tool::Measure {
            (
                "Ölçüm",
                format!("{} nokta", self.measurement.points().len()),
                self.measurement_properties(),
            )
        } else {
            (
                "Özellikler",
                if self.selection.is_some() {
                    "1 öğe seçili".to_owned()
                } else {
                    "Seçim yok".to_owned()
                },
                self.selection_properties(),
            )
        };

        let record_count = self
            .layers
            .get(self.active_layer)
            .map_or(0, |layer| layer.features.len());

        Dock::new(DOCK_WIDTH)
            .push(
                Panel::new("Katmanlar", self.layer_table())
                    .meta(format!("{} katman", self.layers.len())),
            )
            .push(
                Panel::new(title, details)
                    .meta(meta)
                    .height(FillPortion(4))
                    .scrollable(),
            )
            .push(
                Panel::new("Öğe tablosu", self.feature_table())
                    .meta(format!(
                        "{}, {record_count} kayıt",
                        self.active_layer_name()
                    ))
                    .height(FillPortion(5)),
            )
            .into()
    }

    /// Katman tablosu ve altında etkin katmanın opaklığı.
    fn layer_table(&self) -> Element<'_, Message> {
        let rows = self.layers.iter().enumerate().map(|(index, layer)| {
            let name = label::body(layer.name.as_str()).style(if layer.visible {
                style::text::default
            } else {
                style::text::muted
            });

            table::Row::new([
                checkbox(layer.visible)
                    .size(13.0)
                    .on_toggle(move |visible| Message::LayerVisibility(index, visible))
                    .into(),
                swatch(layer.color),
                name.into(),
                label::caption(layer.kind.label()).into(),
                label::mono_caption(layer.features.len().to_string()).into(),
                label::mono_caption(format!("{:.0}%", layer.opacity * 100.0)).into(),
                tip(
                    button(icon(Icon::Target).size(13.0))
                        .on_press(Message::ZoomToLayer(index))
                        .padding([0, 4])
                        .style(style::button::subtle),
                    Tip::new("Katmana sığdır"),
                    tooltip::Position::Left,
                ),
            ])
            .selected(index == self.active_layer)
            .on_press(Message::LayerActivated(index))
        });

        let layers = Table::new([
            table::Column::new("").width(13),
            table::Column::new("").width(11),
            table::Column::new("Ad").width(Fill),
            table::Column::new("Tür").width(44),
            table::Column::new("Öğe").width(24).align_right(),
            table::Column::new("Opak.").width(38).align_right(),
            table::Column::new("").width(20),
        ])
        .extend(rows);

        let opacity: Element<'_, Message> = match self.layers.get(self.active_layer) {
            Some(layer) => {
                let index = self.active_layer;

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
                .align_y(Center)
                .into()
            }
            None => space::vertical().height(0).into(),
        };

        column![layers, container(opacity).padding([6, 10]).width(Fill)].into()
    }

    /// Seçili öğenin genel bilgileri ve öznitelikleri.
    fn selection_properties(&self) -> Element<'_, Message> {
        let Some((layer, feature)) = self
            .selection
            .and_then(|selection| selection.resolve(&self.layers))
        else {
            return container(label::muted(
                "Özelliklerini görmek için haritada bir öğeye tıklayın (Seç aracı) \
                 veya öğe tablosundan bir satır seçin.",
            ))
            .padding(12)
            .width(Fill)
            .into();
        };

        let mut grid = PropertyGrid::new()
            .category("Genel")
            .property("Katman", layer.name.as_str())
            .property("Geometri", feature.geometry.label())
            .figure("Köşe sayısı", feature.geometry.vertices().len().to_string());

        if let Some(bounds) = feature.bounds() {
            grid = grid.figure("Merkez", format::decimal(bounds.center()));
        }

        if !feature.properties.is_empty() {
            grid = feature
                .properties
                .iter()
                .fold(grid.category("Öznitelikler"), |grid, (key, value)| {
                    grid.property(key.as_str(), value.as_str())
                });
        }

        column![
            row![
                label::title(feature.name.as_str()),
                space::horizontal(),
                button(label::caption("Odakla").style(style::text::default))
                    .on_press(Message::FocusSelection)
                    .padding([2, 8])
                    .style(style::button::flat),
            ]
            .align_y(Center)
            .padding([8, 10]),
            grid,
        ]
        .width(Fill)
        .into()
    }

    /// Ölç aracında toplam uzunluk ve kenarlar.
    fn measurement_properties(&self) -> Element<'_, Message> {
        let measurement = &self.measurement;

        let mut grid = PropertyGrid::new()
            .category("Genel")
            .figure("Nokta sayısı", measurement.points().len().to_string())
            .figure("Kenar sayısı", measurement.segment_count().to_string())
            .property(
                "Yakalama",
                if self.options.snap {
                    "Açık"
                } else {
                    "Kapalı"
                },
            );

        if measurement.segment_count() > 0 {
            grid = measurement.segments().enumerate().fold(
                grid.category("Kenarlar"),
                |grid, (index, meters)| {
                    grid.figure(format!("Kenar {}", index + 1), format::distance(meters))
                },
            );
        }

        let hint = if measurement.is_empty() {
            "Sol tıkla nokta ekleyin; sağ tık ölçümü temizler."
        } else {
            "Toplam uzunluk"
        };

        column![
            column![
                label::caption(hint),
                row![
                    label::figure(format::distance(measurement.total_meters())).style(|theme| {
                        iced::widget::text::Style {
                            color: Some(kentos_rc::spatial::model_space::Style::of(theme).measure),
                        }
                    }),
                    space::horizontal(),
                    button(label::caption("Temizle").style(style::text::default))
                        .on_press_maybe(
                            (!measurement.is_empty()).then_some(Message::ClearMeasurement),
                        )
                        .padding([2, 8])
                        .style(style::button::flat),
                ]
                .align_y(Center),
            ]
            .spacing(2)
            .padding([8, 10]),
            grid,
        ]
        .width(Fill)
        .into()
    }

    /// Etkin katmanın öğeleri: ad ve ilk iki özniteliğin özeti.
    fn feature_table(&self) -> Element<'_, Message> {
        let Some(layer) = self.layers.get(self.active_layer) else {
            return space::vertical().into();
        };

        let rows = layer.features.iter().enumerate().map(|(index, feature)| {
            let reference = FeatureRef::new(self.active_layer, index);
            let selected = self.selection == Some(reference);

            let summary = feature
                .properties
                .iter()
                .filter(|(key, _)| key != "Tür")
                .map(|(_, value)| value.as_str())
                .take(2)
                .collect::<Vec<_>>()
                .join(", ");

            table::Row::new([
                label::body(feature.name.as_str()).into(),
                label::caption(summary)
                    .style(if selected {
                        style::text::default
                    } else {
                        style::text::muted
                    })
                    .into(),
            ])
            .selected(selected)
            .on_press(Message::FeatureSelected(reference))
        });

        Table::new([
            table::Column::new("Ad").width(Fill),
            table::Column::new("Özet").align_right(),
        ])
        .extend(rows)
        .height(Fill)
        .empty("Bu katmanda öğe yok. Çizim araçlarıyla ekleyebilirsiniz.")
        .into()
    }
}

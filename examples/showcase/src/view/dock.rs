//! Yuva: kenarlardaki paneller (katmanlar, özellikler, öznitelik tablosu,
//! görevler) ve ortadaki harita. Ölçüm, koordinata git ve katman stili
//! harita üstündeki kayan pencerelerdedir.

use iced::widget::{button, column, container, row, space, tooltip};
use iced::{Center, Element, Fill};

use kentos_rc::attribute::{DateTime, FieldKind, ObjectId, text};
use kentos_rc::icon::{Icon, icon};
use kentos_rc::label;
use kentos_rc::spatial::{Geometry, format};
use kentos_rc::style;
use kentos_rc::widget::progress;
use kentos_rc::widget::{
    DockSpace, Inspector, Pane, Task, TaskList, Tip, horizontal_divider, swatch, tip,
};

use crate::app::{Showcase, TIME_ZONE};
use crate::jobs::JobState;
use crate::message::{DockPanel, Message};

impl Showcase {
    /// Ortadaki içeriğin çevresinde yuvadaki paneller. Panelin gövdesi
    /// yalnızca görünürken kurulur; başlığın sağında kısa bilgisi ve
    /// eylemleri durur.
    pub(super) fn dock_space<'a>(
        &'a self,
        center: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        DockSpace::new(center, &self.docks, Message::Dock, move |panel| {
            let pane = Pane::new(panel.title(), move || self.panel_body(panel)).icon(panel.icon());

            match panel {
                DockPanel::Layers => pane.actions(
                    row![
                        label::caption(format!(
                            "{} katman, {} grup",
                            self.layers.len(),
                            self.layer_tree.groups.len()
                        )),
                        self.layer_panel_actions(),
                    ]
                    .spacing(8)
                    .align_y(Center),
                ),
                DockPanel::Details => pane
                    .actions(label::caption(match self.selection.len() {
                        0 => "Seçim yok".to_owned(),
                        count => format!("{count} öğe seçili"),
                    }))
                    .scrollable(),
                DockPanel::Table => pane.actions(label::caption(self.table_meta())),
                DockPanel::Tasks => {
                    let active = self.jobs.iter().filter(|job| job.is_active()).count();
                    let meta = match (active, self.jobs.failed()) {
                        (0, 0) => String::new(),
                        (0, failed) => format!("{failed} başarısız"),
                        (active, _) => format!("{active} etkin"),
                    };

                    pane.actions(label::caption(meta)).scrollable()
                }
            }
        })
        .into()
    }

    fn panel_body(&self, panel: DockPanel) -> Element<'_, Message> {
        match panel {
            DockPanel::Layers => self.layer_panel(),
            DockPanel::Details => self.inspector_panel(),
            DockPanel::Table => self.attribute_table(),
            DockPanel::Tasks => self.tasks_panel(),
        }
    }

    /// Arka plandaki işler: süren, sıradaki ve biten işler; iptal, yeniden
    /// deneme ve listeden kaldırma.
    fn tasks_panel(&self) -> Element<'_, Message> {
        if self.jobs.is_empty() {
            return container(label::caption(
                "Süren iş yok. Dışa aktarma uygulama menüsünden, dizin oluşturma Yönet \
                 sekmesinden başlar.",
            ))
            .padding([12, 12])
            .into();
        }

        let tasks = self.jobs.iter().map(|job| {
            let state = match &job.state {
                JobState::Queued => progress::State::Queued,
                JobState::Running => progress::State::Running(job.progress()),
                JobState::Done => progress::State::Done,
                JobState::Failed(_) => progress::State::Failed,
                JobState::Cancelled => progress::State::Cancelled,
            };

            Task::new(job.title())
                .detail(job.detail())
                .state(state)
                .on_cancel(Message::JobCancelled(job.id))
                .on_retry(Message::JobRetried(job.id))
                .on_dismiss(Message::JobDismissed(job.id))
        });

        let mut body = column![TaskList::new().extend(tasks)];

        if self.jobs.has_finished() {
            body = body.push(horizontal_divider()).push(
                container(
                    button(label::caption("Bitenleri kaldır").style(style::text::default))
                        .on_press(Message::JobsCleared)
                        .padding([2, 8])
                        .style(style::button::flat),
                )
                .padding([6, 8]),
            );
        }

        body.into()
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

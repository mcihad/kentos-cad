//! Harita üstündeki kayan araç pencereleri: ölçüm, koordinata git ve
//! katman stili.
//!
//! Pencereler arkadaki işi kilitlemez: ölçüm sürerken haritaya tıklanır,
//! stil değişiklikleri haritaya anında yansır.

use iced::widget::text::Wrapping;
use iced::widget::{
    Column, button, column, container, pick_list, row, scrollable, slider, space, text_input,
    tooltip,
};
use iced::{Center, Color, Element, Fill, Right, Theme};

use kentos_rc::icon::{Icon, icon};
use kentos_rc::label;
use kentos_rc::spatial::{LayerKind, LonLat, format, model_space};
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::widget::progress::{self, Task, TaskList};
use kentos_rc::widget::{Segmented, Tip, ToolWindow, horizontal_divider, tip};

use super::LayerChoice;
use crate::app::Showcase;
use crate::jobs::JobState;
use crate::message::{Keyword, Message, Pane};

/// Katman stili penceresindeki hazır renkler: örnek verinin renkleri ve
/// harita zemininde okunan birkaç ton.
const COLORS: [Color; 10] = [
    Color::from_rgb(0.96, 0.35, 0.38),
    Color::from_rgb(0.96, 0.62, 0.25),
    Color::from_rgb(0.89, 0.76, 0.35),
    Color::from_rgb(0.55, 0.86, 0.26),
    Color::from_rgb(0.21, 0.77, 0.71),
    Color::from_rgb(0.29, 0.62, 0.96),
    Color::from_rgb(0.18, 0.56, 0.60),
    Color::from_rgb(0.69, 0.52, 0.97),
    Color::from_rgb(0.89, 0.47, 0.72),
    Color::from_rgb(0.85, 0.86, 0.88),
];

/// Çizgi kalınlıkları (piksel).
const STROKES: [u8; 4] = [1, 2, 3, 4];

/// Form satırlarındaki etiket sütununun genişliği (12 piksellik metne göre).
const LABEL_WIDTH: f32 = 56.0;

impl Showcase {
    pub(super) fn pane(&self, pane: Pane) -> ToolWindow<'_, Message> {
        match pane {
            Pane::Measure => self.measure_pane(),
            Pane::GoTo => self.go_to_pane(),
            Pane::Style => self.style_pane(),
            Pane::Tasks => self.tasks_pane(),
        }
    }

    /// Arka plandaki işler: süren, sıradaki ve biten işler; iptal, yeniden
    /// deneme ve listeden kaldırma.
    fn tasks_pane(&self) -> ToolWindow<'_, Message> {
        let active = self.jobs.iter().filter(|job| job.is_active()).count();

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

        let body: Element<'_, Message> = if self.jobs.is_empty() {
            container(label::caption(
                "Süren iş yok. Dışa aktarma uygulama menüsünden, dizin oluşturma Yönet \
                 sekmesinden başlar.",
            ))
            .padding([12, 12])
            .into()
        } else {
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
        };

        let meta = match (active, self.jobs.failed()) {
            (0, 0) => String::new(),
            (0, failed) => format!("{failed} başarısız"),
            (active, _) => format!("{active} etkin"),
        };

        let window = ToolWindow::new(Pane::Tasks.title(), body)
            .icon(Pane::Tasks.icon())
            .width(Pane::Tasks.width())
            .scrollable()
            .resizable();

        if meta.is_empty() {
            window
        } else {
            window.meta(meta)
        }
    }

    /// Ölç aracının sonuçları: toplam uzunluk, eylemler ve kenarlar.
    fn measure_pane(&self) -> ToolWindow<'_, Message> {
        let measurement = &self.measurement;
        let total = format::distance(measurement.total_meters());
        let empty = measurement.is_empty();

        let actions = row![
            action(
                Icon::Undo,
                "Son noktayı geri al",
                (!empty).then_some(Message::Keyword(Keyword::Undo))
            ),
            action(
                Icon::Eraser,
                "Ölçümü temizle",
                (!empty).then_some(Message::ClearMeasurement)
            ),
            action(
                Icon::Copy,
                "Panoya kopyala",
                (measurement.segment_count() > 0).then_some(Message::CopyMeasurement)
            ),
        ]
        .spacing(2);

        let summary = row![
            column![
                label::caption("Toplam uzunluk"),
                label::figure(total.clone()).style(measure_color),
            ]
            .spacing(2)
            .width(Fill),
            actions,
        ]
        .align_y(Center);

        let mut body = column![summary].spacing(10).padding([10, 12]);

        body = if measurement.segment_count() > 0 {
            let mut running = 0.0;
            let rows = measurement.segments().enumerate().map(|(index, meters)| {
                running += meters;

                row![
                    label::mono_caption((index + 1).to_string()).width(typography::scaled(22.0)),
                    label::mono(format::distance(meters))
                        .width(Fill)
                        .align_x(Right),
                    label::mono(format::distance(running))
                        .style(style::text::muted)
                        .width(Fill)
                        .align_x(Right),
                ]
                .spacing(8)
                .into()
            });

            let header = row![
                label::caption("#").width(typography::scaled(22.0)),
                label::caption("Kenar").width(Fill).align_x(Right),
                label::caption("Toplam").width(Fill).align_x(Right),
            ]
            .spacing(8);

            body.push(
                column![
                    header,
                    horizontal_divider(),
                    scrollable(Column::with_children(rows).spacing(4))
                        .direction(style::field::thin_scrollbar())
                        .spacing(4),
                ]
                .spacing(5),
            )
        } else {
            body.push(label::caption(if empty {
                "Haritada ölçülecek ilk noktayı tıklayın. Her tıklama bir kenar ekler; sağ tık ölçümü temizler."
            } else {
                "Kenarı tamamlamak için ikinci noktayı tıklayın."
            }))
        };

        let window = ToolWindow::new(Pane::Measure.title(), body)
            .icon(Pane::Measure.icon())
            .width(Pane::Measure.width())
            .resizable();

        if empty { window } else { window.meta(total) }
    }

    /// Koordinata git: enlem ve boylam yazılır; görünüm ortalanır ya da
    /// süren çizime nokta eklenir.
    fn go_to_pane(&self) -> ToolWindow<'_, Message> {
        let latitude = self.go_to.latitude();
        let longitude = self.go_to.longitude();
        let location = self.go_to.location();

        let field =
            |name: &'static str, value: &str, invalid: bool, on_input: fn(String) -> Message| {
                row![
                    label::muted(name).width(typography::scaled(LABEL_WIDTH)),
                    text_input("ondalık derece", value)
                        .on_input(on_input)
                        .on_submit(Message::GoToCentered)
                        .font(typography::mono())
                        .size(typography::body())
                        .padding([3, 6])
                        .style(style::field::validated(invalid)),
                ]
                .spacing(8)
                .align_y(Center)
            };

        let mut form = column![
            field(
                "Enlem",
                &self.go_to.latitude,
                latitude.is_err(),
                Message::GoToLatitude
            ),
            field(
                "Boylam",
                &self.go_to.longitude,
                longitude.is_err(),
                Message::GoToLongitude
            ),
        ]
        .spacing(6);

        for error in [latitude.err(), longitude.err()].into_iter().flatten() {
            form = form.push(
                row![
                    icon(Icon::Warning)
                        .size(12.0)
                        .tone(kentos_rc::icon::Tone::Danger),
                    label::caption(error).style(style::text::danger),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }

        let drawing = self.tool.takes_points();

        let actions = row![
            button(label::text("Git").align_x(Center).width(Fill))
                .on_press_maybe(location.map(|_| Message::GoToCentered))
                .width(Fill)
                .padding([3, 10])
                .style(style::button::primary),
            tip(
                button(label::text("Nokta ekle").align_x(Center).width(Fill))
                    .on_press_maybe(location.filter(|_| drawing).map(|_| Message::GoToPlaced))
                    .width(Fill)
                    .padding([3, 10])
                    .style(style::button::secondary),
                Tip::new("Nokta ekle").body(if drawing {
                    "Koordinatı süren çizime ya da ölçüme nokta olarak ekler."
                } else {
                    "Önce şeritten bir çizim aracı ya da Ölç'ü seçin."
                }),
                tooltip::Position::Bottom,
            ),
        ]
        .spacing(6);

        let readouts = column![
            self.readout(
                "İmleç",
                self.cursor.or(self.last_cursor),
                self.cursor.is_some()
            ),
            self.readout("Merkez", Some(self.viewport.center), true),
        ]
        .spacing(2);

        ToolWindow::new(
            Pane::GoTo.title(),
            column![form, actions, horizontal_divider(), readouts]
                .spacing(10)
                .padding([10, 12]),
        )
        .icon(Pane::GoTo.icon())
        .width(Pane::GoTo.width())
    }

    /// Koordinat satırı: ad, ondalık koordinat ve kopyalama düğmesi. Canlı
    /// olmayan değer soluk yazılır.
    fn readout<'a>(
        &self,
        name: &'a str,
        location: Option<LonLat>,
        live: bool,
    ) -> Element<'a, Message> {
        let value: Element<'a, Message> = match location {
            Some(location) => label::mono_caption(format::decimal(location))
                .style(if live {
                    style::text::default
                } else {
                    style::text::muted
                })
                .wrapping(Wrapping::None)
                .into(),
            None => label::caption("haritada değil").into(),
        };

        row![
            label::caption(name).width(typography::scaled(LABEL_WIDTH)),
            container(value).width(Fill).clip(true),
            action(
                Icon::Copy,
                "Koordinatı kopyala",
                location.map(Message::CopyCoordinates)
            ),
        ]
        .spacing(8)
        .align_y(Center)
        .into()
    }

    /// Aktif katmanın rengi, opaklığı ve çizgi kalınlığı; değişiklikler
    /// haritaya anında yansır.
    fn style_pane(&self) -> ToolWindow<'_, Message> {
        let index = self.active_layer;

        let Some(layer) = self.layers.get(index) else {
            return ToolWindow::new(Pane::Style.title(), space::vertical().height(0));
        };

        let choices = self.layer_choices();
        let current = choices.get(index).cloned();

        let picker = pick_list(choices, current, |choice: LayerChoice<'_>| {
            Message::LayerActivated(choice.index)
        })
        .font(typography::ui())
        .text_size(typography::body())
        .padding([2, 8])
        .width(Fill)
        .style(style::field::pick_list)
        .menu_style(style::field::menu);

        let swatches = COLORS.chunks(5).map(|colors| {
            row(colors.iter().map(|&color| {
                let selected = same_color(layer.color, color);

                button(
                    container(space::horizontal())
                        .width(16)
                        .height(16)
                        .style(style::container::swatch(color)),
                )
                .on_press(Message::LayerColor(index, color))
                .padding(2)
                .style(style::button::swatch(selected))
                .into()
            }))
            .spacing(4)
            .into()
        });

        let mut form = column![
            form_row("Katman", picker),
            form_row("Renk", Column::with_children(swatches).spacing(4)),
            form_row(
                "Opaklık",
                row![
                    slider(
                        0.0..=1.0,
                        layer.opacity,
                        move |value| Message::LayerOpacity(index, value)
                    )
                    .step(0.05_f32),
                    label::mono_caption(format!("{:>3.0}%", layer.opacity * 100.0))
                        .style(style::text::default)
                        .width(38)
                        .align_x(Right),
                ]
                .spacing(8)
                .align_y(Center),
            ),
        ]
        .spacing(8);

        if layer.kind != LayerKind::Point {
            let stroke = STROKES
                .into_iter()
                .find(|&width| (f32::from(width) - layer.stroke_width).abs() < 0.26)
                .unwrap_or(0);

            form = form.push(form_row(
                "Kalınlık",
                Segmented::new(STROKES, stroke, move |width| {
                    Message::LayerStroke(index, f32::from(width))
                })
                .width(iced::Fill),
            ));
        }

        if !layer.sublayers.is_empty() {
            form = form.push(label::caption(
                "Alt katmanlar kendi renkleriyle çizilir; bu renk hiçbir alt katmana uymayan öğeler içindir.",
            ));
        }

        ToolWindow::new(Pane::Style.title(), form.padding([10, 12]))
            .icon(Pane::Style.icon())
            .width(Pane::Style.width())
    }
}

/// Ölçüm renginde metin: haritadaki ölçüm çizgisiyle aynı.
fn measure_color(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(model_space::Style::of(theme).measure),
    }
}

/// Pencerelerdeki küçük ikon düğmesi ve ipucu.
fn action<'a>(
    glyph: Icon,
    description: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    tip(
        button(icon(glyph).size(14.0))
            .on_press_maybe(on_press)
            .padding(4)
            .style(style::button::ghost),
        Tip::new(description),
        tooltip::Position::Bottom,
    )
}

/// Form satırı: solda ad, sağda denetim.
fn form_row<'a>(name: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        label::muted(name).width(typography::scaled(LABEL_WIDTH)),
        container(control).width(Fill),
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

/// İki renk aynı mı (kayan nokta yuvarlamasına dayanıklı).
fn same_color(a: Color, b: Color) -> bool {
    a.into_rgba8() == b.into_rgba8()
}

//! The plot scale and the annotations that follow it (docs/adr/0205 §3, §4;
//! the web's `app/annotationScale.ts` and `ui/settings/PlotScaleDialog.ts`):
//! a scale chosen on the ribbon or typed in Ölçek yaz…, and Proje ayarları'
//! Kaydet, give the texts, leaders, dimensions and tables at the old general
//! height the new one in one undo step “Yazı yüksekliklerini uydur”
//! (`kentos_interaction::annotation_scale`); what was changed by hand stays;
//! those on a locked layer are counted.

use iced::widget::{Id, column, operation, row, text_input};
use iced::{Center, Element, Task};
use kentos_contracts::ScaleChange;
use kentos_interaction::Level;
use kentos_interaction::annotation_scale::follow_annotation_change;
use kentos_project::wizard::scale_text;
use kentos_ui::theme::typography;
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::screen_scale::typed_scale;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Çizim ölçeği";
/// The largest scale denominator that may be typed.
pub const MAX_PLOT_SCALE: f64 = 1_000_000.0;
const FIELD: &str = "cizim-olcegi";

#[derive(Debug, Clone)]
pub enum Event {
    /// Ölçek yaz…: the window opens with the project's scale.
    Open,
    Edit(String),
    /// Tamam or Enter: a scale that holds is set.
    Submit,
    Close,
}

fn event(e: Event) -> Message {
    Message::PlotScale(e)
}

/// A typed plot scale: a whole number from 1 to `MAX_PLOT_SCALE` (1:N, N,
/// dots between its digits); none for anything else (the web's `typedPlotScale`).
pub fn typed_plot_scale(text: &str) -> Option<f64> {
    typed_scale(text).filter(|n| *n <= MAX_PLOT_SCALE)
}

impl App {
    /// The annotations at the old general height given the new one, in one
    /// step; how many followed and how many a locked layer kept are said.
    pub(crate) fn follow_annotations(&mut self, change: ScaleChange) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let followed = follow_annotation_change(&mut doc.model, &change);
        if let Some(why) = followed.refused {
            self.say(Level::Warn, why);
        }
        if followed.written > 0 {
            let place = if change.from_scale == change.to_scale {
                String::new()
            } else {
                format!(
                    " ({} → {})",
                    scale_text(change.from_scale),
                    scale_text(change.to_scale)
                )
            };
            self.say(
                Level::Success,
                format!(
                    "Yazı yükseklikleri uyduruldu{place}: {} nesne yeni boyunda.",
                    followed.written
                ),
            );
        }
        if followed.locked > 0 {
            self.say(
                Level::Warn,
                format!(
                    "Kilitli katmanlardaki {} nesnenin yüksekliği değişmedi; kilidi açıp ölçeği yeniden seçebilirsiniz.",
                    followed.locked
                ),
            );
        }
    }

    /// The project's plot scale set to `scale` (the ribbon's Ölçek, Ölçek
    /// yaz…): an edit of the project, never an undo step; its annotations
    /// follow in one.
    pub(crate) fn set_plot_scale(&mut self, scale: f64) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let mut settings = doc.settings().clone();
        let from = settings.plot_scale;
        if !(scale.is_finite() && scale > 0.0) || from == scale {
            return;
        }
        let heights = settings.annotation.clone().unwrap_or_default();
        settings.plot_scale = scale;
        doc.model.set_settings(settings);
        self.follow_annotations(ScaleChange {
            from_scale: from,
            to_scale: scale,
            from: heights.clone(),
            to: heights,
        });
    }

    pub(crate) fn plot_scale_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Open => {
                let Some(doc) = &self.document else {
                    self.output("Açık çizim yok.");
                    return Task::none();
                };
                self.plot_scale_field = Some(kentos_processing::text::js_number(
                    doc.settings().plot_scale,
                ));
                self.dialog = Some(Dialog::PlotScale);
                return Task::batch([
                    operation::focus(Id::new(FIELD)),
                    operation::select_all(Id::new(FIELD)),
                ]);
            }
            Event::Edit(t) => self.plot_scale_field = Some(t),
            Event::Submit => {
                let typed = self.plot_scale_field.as_deref().and_then(typed_plot_scale);
                if let Some(n) = typed {
                    self.plot_scale_field = None;
                    self.dialog = None;
                    self.set_plot_scale(n);
                }
            }
            Event::Close => {
                self.plot_scale_field = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Ölçek yaz…'s window: 1: and the denominator, and what it will do.
    pub(crate) fn plot_scale_view(&self) -> Element<'_, Message> {
        let text = self.plot_scale_field.as_deref().unwrap_or_default();
        let now = self
            .document
            .as_ref()
            .map_or(1000.0, |d| d.settings().plot_scale);
        let typed = typed_plot_scale(text);
        let field = text_input("N", text)
            .id(Id::new(FIELD))
            .on_input(|t| event(Event::Edit(t)))
            .on_submit(event(Event::Submit))
            .padding([5, 8])
            .size(typography::body())
            .font(typography::mono())
            .width(typography::from_default(200.0))
            .style(style::field::validated(typed.is_none()));
        let status = match typed {
            None => format!(
                "1 ile {} arasında bir tam sayı yazın.",
                crate::view::thousands(MAX_PLOT_SCALE)
            ),
            Some(n) if n == now => format!("Şimdi {}.", scale_text(now)),
            Some(n) => format!(
                "{} → {}: genel boydaki yazılar yeni ölçeğe uyar.",
                scale_text(now),
                scale_text(n)
            ),
        };
        overlay::modal(
            Frame::new(TITLE)
                .push(
                    column![
                        row![label::mono("1:"), kentos_ui::widget::focus_ring(field)]
                            .spacing(8)
                            .align_y(Center),
                        label::caption(status).style(style::text::muted),
                    ]
                    .spacing(10),
                )
                .action(words::secondary("Vazgeç", Some(event(Event::Close))))
                .action(words::primary(
                    "Tamam",
                    typed.is_some().then(|| event(Event::Submit)),
                ))
                .width(380.0),
            event(Event::Close),
        )
    }
}

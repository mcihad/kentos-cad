//! Değişikliğin etkisi (docs/adr/0314 §4): what changing a layer's look or a line type would reach, from the
//! read-only `cad.styles.impact`: the objects that take it, those keeping values of their own, those a locked layer
//! keeps, the block pieces and the inserts showing them, the layer states that bring an old look back, the sheet maps
//! that draw it otherwise, and the external references (none). Nesneleri seç chooses the objects that take it.
//! Katmanlar's right-click menu and Çizgi tipleri open it.

use iced::widget::{Column, row, text};
use iced::{Element, Task};
use kentos_commands::{CommandResult, ImpactTarget, StyleImpact, StylesImpact};
use kentos_native_application::{ExecutionContext, styles_impact};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;
use kentos_interaction::Level;

/// The window's title, which a trace names it by.
pub const IMPACT_TITLE: &str = "Değişikliğin etkisi";
const CHOOSE: &str = "Nesneleri seç";
const CLOSE: &str = "Kapat";

#[derive(Debug, Clone)]
pub struct Window {
    pub target: ImpactTarget,
    pub impact: StyleImpact,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(ImpactTarget, String),
    Choose,
    Close,
}

fn msg(event: Event) -> Message {
    Message::StyleImpact(event)
}

impl App {
    pub(crate) fn style_impact_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(target, id) => {
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let input = StylesImpact { target, id };
                // Read-only: plan writes nothing.
                match styles_impact::plan(&ExecutionContext::new(&mut doc.model), &input) {
                    CommandResult::Completed { output, .. } => {
                        self.style_impact = Some(Window {
                            target,
                            impact: output,
                        });
                        self.dialog = Some(Dialog::StyleImpact);
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => self.warn(error.message),
                    _ => {}
                }
            }
            Event::Choose => {
                let Some(w) = &self.style_impact else {
                    return Task::none();
                };
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let slots: Vec<_> = w
                    .impact
                    .uids
                    .iter()
                    .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                let n = slots.len();
                self.selection.set(slots);
                self.say(Level::Info, format!("{n} nesne seçildi."));
                self.style_impact = None;
                self.dialog = None;
            }
            Event::Close => {
                self.style_impact = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn style_impact_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.style_impact else {
            return text("").into();
        };
        let i = &w.impact;
        let what = match w.target {
            ImpactTarget::Layer => "Katman",
            ImpactTarget::LineType => "Çizgi tipi",
            ImpactTarget::TextStyle => "Yazı stili",
            ImpactTarget::DimensionStyle => "Ölçü stili",
        };
        let line = |name: &str, value: String| -> Element<'_, Message> {
            row![
                label::caption(name.to_owned()).width(220.0),
                label::body(value).width(iced::Fill)
            ]
            .spacing(8)
            .into()
        };
        let own = if i.own.is_empty() {
            "yok".to_owned()
        } else {
            i.own
                .iter()
                .map(|o| format!("{} {}", o.count, styles_impact::property_word(&o.property)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut rows = Column::new()
            .spacing(6)
            .push(line("Değişikliği alan nesneler", i.follow.to_string()))
            .push(line("Kendi değerini koruyanlar", own));
        if !matches!(w.target, ImpactTarget::Layer | ImpactTarget::LineType) {
            rows = rows.push(line("Kilitli katmanda kalanlar", i.locked.to_string()));
        }
        if w.target == ImpactTarget::LineType {
            rows = rows.push(line(
                "Deseni kullanan katmanlar",
                i.layers.len().to_string(),
            ));
        }
        rows = rows
            .push(line(
                "Blok parçaları",
                format!(
                    "{} parça, {} blok; {} yerleştirmede görünür",
                    i.block_pieces,
                    i.blocks.len(),
                    i.inserts
                ),
            ))
            .push(line(
                "Eski görünüşü saklayan durumlar",
                if i.layer_states.is_empty() {
                    "yok".to_owned()
                } else {
                    i.layer_states.join(", ")
                },
            ))
            .push(line(
                "Geçersiz kılan pafta haritaları",
                i.sheet_maps.len().to_string(),
            ))
            .push(line(
                "Dış başvurular",
                format!("{} (KentOS dış başvuru taşımaz)", i.external_references),
            ));
        // What the rows do not say: a layer's look reaches a locked layer too; a style's leaves it.
        let note = match w.target {
            ImpactTarget::Layer | ImpactTarget::LineType => {
                "Katmanın görünüşü kilitli katmandaki nesnelere de yansır."
            }
            _ => "Kilitli katmandaki nesneler stil değişince olduğu gibi kalır.",
        };
        let body = Column::new()
            .spacing(12)
            .push(label::caption(format!("{what}: {}", i.name)).style(style::text::muted))
            .push(rows)
            .push(words::summary(vec![words::text_line(Kind::Ok, note)]));
        overlay::modal(
            Frame::new(IMPACT_TITLE)
                .push(body)
                .action(words::secondary(
                    CHOOSE,
                    (i.follow > 0).then(|| msg(Event::Choose)),
                ))
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(560.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn style_impact_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.style_impact.is_none() {
            return Err(format!("{IMPACT_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CHOOSE) => Some(msg(Event::Choose)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{IMPACT_TITLE}” penceresinde {other} yok")),
        })
    }
}

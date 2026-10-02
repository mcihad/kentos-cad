use iced::widget::{button, column, container, row, space, text};
use iced::{Element, Fill, Length};

use kentos_ui::style;
use kentos_ui::theme::{motion, typography};
use kentos_ui::widget::ai::{AiActivity, AiConversation, AiQuestion, AiThinking, AiToolCall};
use kentos_ui::widget::{Responsive, Switch};

use crate::ai::{Event, OUTPUT, Scenario};
use crate::app::Showcase;
use crate::message::Message;

fn event(event: Event) -> Message {
    Message::AiStudio(event)
}

impl Showcase {
    pub(super) fn ai_page(&self) -> Vec<Element<'_, Message>> {
        let studio = &self.ai_studio;
        let mut scenarios = row![
            text("Akışı dene")
                .size(typography::body())
                .style(style::text::muted)
        ]
        .spacing(8)
        .align_y(iced::Center);
        for scenario in Scenario::ALL {
            scenarios = scenarios.push(
                button(text(scenario.label()).size(typography::body()))
                    .on_press(event(Event::Scenario(scenario)))
                    .padding([6, 11])
                    .style(style::button::segment(studio.scenario == scenario)),
            );
        }
        scenarios = scenarios.push(space::horizontal()).push(
            Switch::new(motion::reduced(), |reduced| {
                event(Event::ReducedMotion(reduced))
            })
            .label("Hareketi azalt"),
        );
        let stage = Responsive::new(move |bounds| {
            let conversation =
                AiConversation::new(&studio.conversation, |e| event(Event::Conversation(e)))
                    .title("KentOS Asistan")
                    .model("Etkileşimli akış örneği")
                    .attachments(true)
                    .output_id(OUTPUT)
                    .suggestion(
                        "Parselleri incele",
                        event(Event::Scenario(Scenario::Stream)),
                    )
                    .suggestion("Bir soru sor", event(Event::Scenario(Scenario::Question)))
                    .height(typography::scaled(550.0));
            let rail = column![
                row![
                    AiActivity::new(true).size(30.0),
                    column![
                        text("Canlı bileşenler")
                            .font(typography::ui_strong())
                            .size(typography::body() + 3.0),
                        text("Tek başına da kullanılabilir.")
                            .size(typography::caption())
                            .style(style::text::muted)
                    ]
                    .spacing(3)
                ]
                .spacing(9)
                .align_y(iced::Center),
                text("Düşünme özeti")
                    .font(typography::ui_strong())
                    .size(typography::body()),
                AiThinking::new(&studio.thought).on_toggle(event(Event::Thought)),
                text("Araç çalıştırma")
                    .font(typography::ui_strong())
                    .size(typography::body()),
                AiToolCall::new(&studio.tool).on_toggle(event(Event::Tool)),
                text("Kullanıcıya soru")
                    .font(typography::ui_strong())
                    .size(typography::body()),
                AiQuestion::new(&studio.question, |e| event(Event::Question(e))),
            ]
            .spacing(12)
            .width(Fill);
            if bounds.width < typography::scaled(900.0) {
                column![conversation, rail].spacing(22).into()
            } else {
                row![
                    container(conversation).width(Length::FillPortion(2)),
                    container(rail).width(Length::FillPortion(1))
                ]
                .spacing(22)
                .align_y(iced::Top)
                .into()
            }
        })
        .height(Length::Shrink);
        let mut speed = row![
            text("Karakter akışı")
                .size(typography::caption())
                .style(style::text::muted)
        ]
        .spacing(6)
        .align_y(iced::Center);
        for (index, label) in ["Yavaş", "Normal", "Hızlı"].iter().enumerate() {
            speed = speed.push(
                button(text(*label).size(typography::caption()))
                    .on_press(event(Event::Speed(index)))
                    .padding([5, 9])
                    .style(style::button::segment(index == studio.speed)),
            );
        }
        speed = speed.push(space::horizontal()).push(
            button(text("Örneği sıfırla").size(typography::caption()))
                .on_press(event(Event::Reset))
                .padding([5, 9])
                .style(style::button::flat),
        );
        let mut details = column![text("Bu sayfa örnek verilerle akış üretir. Metin akışı, soru yanıtlama, araç onayı, durdurma ve yeniden denemeyi deneyebilirsiniz.").size(typography::body()).style(style::text::muted)].spacing(6);
        if !studio.notice.is_empty() {
            details = details.push(
                text(studio.notice.as_str())
                    .size(typography::body())
                    .style(style::text::default),
            );
        }
        vec![scenarios.into(), stage.into(), speed.into(), details.into()]
    }
}

//! The Asistan panel (docs/adr/8056 §4): the agent the chat talks to, the chat, and the header's Yeni sohbet and
//! Yapay zekâ ayarları. With no agent set up yet the panel says how to add one.

use iced::widget::{button, column, container, row, tooltip};
use iced::{Element, Fill};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::ai::AiConversation;
use kentos_ui::widget::{Choice, Select, Tip, tip};
use kentos_ui::{label, style};

use super::{Event, msg};
use crate::app::{App, Message};
use crate::exchange::words;

/// What an empty chat offers, sent as it is.
const SUGGESTIONS: [&str; 3] = [
    "Bu çizimde neler var? Kısaca özetle.",
    "Etkin katmana köşesi (0, 0) olan 6 × 4 m bir dikdörtgen alan çiz.",
    "Katmanları ve her birindeki nesne sayısını listele.",
];

impl App {
    /// The panel's header: Yeni sohbet and Yapay zekâ ayarları.
    pub(crate) fn assistant_actions(&self) -> Element<'_, Message> {
        let action = |glyph: Icon, title: &'static str, body: &'static str, event: Event| {
            tip(
                button(icon(glyph).size(14.0))
                    .on_press(msg(event))
                    .padding(kentos_ui::theme::metrics::icon_small(14.0))
                    .style(style::button::ghost),
                Tip::new(title).body(body),
                tooltip::Position::Bottom,
            )
        };
        row![
            action(
                Icon::DocumentNew,
                "Yeni sohbet",
                "Sohbeti ve ajana söyleneni unutur; çizimde yapılanlar kalır.",
                Event::New
            ),
            action(
                Icon::Properties,
                "Yapay zekâ ayarları",
                "Ajanlar, modeller ve API anahtarları (Uygulama ayarları › Yapay zekâ).",
                Event::Settings
            ),
        ]
        .spacing(2)
        .into()
    }

    /// The panel's body.
    pub(crate) fn assistant_view(&self) -> Element<'_, Message> {
        let a = &self.assistant;
        let agents = a.store.list();
        let Some(agent) = a.agent() else {
            let first = column![
                label::strong("Kendi yapay zekâ ajanınız"),
                label::caption(
                    "Asistan, çizimi sizin seçtiğiniz bir yapay zekâ sağlayıcısıyla (Anthropic, OpenAI, Gemini, \
                     DeepSeek, Kimi, GLM, Qwen, Mistral, xAI, OpenRouter ya da bu bilgisayardaki bir sunucu) ve \
                     kendi API anahtarınızla okur ve çizer. Yazan her komutu önce planlar; silen ya da değiştiren \
                     bir yazmayı onayınıza sunar."
                )
                .style(style::text::muted),
                words::primary("Ajan ekle", Some(msg(Event::Settings))),
            ]
            .spacing(10);
            return container(first).padding(14).width(Fill).into();
        };
        let mut body = column![].spacing(6).height(Fill);
        if agents.len() > 1 {
            let chosen = agents.iter().position(|p| p.id == agent.id);
            let picker = Select::new(
                agents.iter().map(|p| {
                    Choice::new(p.name.clone()).detail(format!("{} · {}", p.preset().name, p.model))
                }),
                chosen,
                |i| msg(Event::Choose(i)),
            )
            .searchable(false);
            body = body.push(container(picker).padding([6, 8]).width(Fill));
        }
        let mut chat = AiConversation::new(&a.chat, |e| msg(Event::Chat(e)))
            .title(&a.shown.0)
            .model(&a.shown.1)
            .output_id("asistan-sohbet");
        for text in SUGGESTIONS {
            chat = chat.suggestion(text, msg(Event::Suggest(text)));
        }
        body.push(chat).into()
    }
}

//! The status bar (docs/adr/0800 §3): on the left whether anything runs
//! (“Hazır”, “2 uygulama çalışıyor”); on the right, honestly, that updates
//! are not checked, and the launcher's version.

use iced::widget::{Space, column, container, row, space, text};
use iced::{Alignment, Element, Fill, Length};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::horizontal_divider;

use crate::app::{Launcher, Message};
use crate::view::style::{self, px};

pub fn bar(launcher: &Launcher) -> Element<'_, Message> {
    let running = launcher.processes.apps_running();
    let (said, busy) = match running {
        0 => ("Hazır".to_owned(), false),
        1 => ("1 uygulama çalışıyor".to_owned(), true),
        n => (format!("{n} uygulama çalışıyor"), true),
    };
    let dot = px(8.0);
    let lamp = container(Space::new())
        .width(dot)
        .height(dot)
        .style(move |theme: &iced::Theme| {
            let t = Tokens::of(theme);
            style::dot(if busy { t.accent_hover } else { t.success })(theme)
        });
    let words = |w: String| text(w).size(typography::caption());
    let content = row![
        lamp,
        words(said),
        space::horizontal(),
        words("Güncelleme denetimi yok".to_owned()),
        container(Space::new())
            .width(1)
            .height(px(12.0))
            .style(|theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(Tokens::of(theme).border)),
                ..container::Style::default()
            }),
        words(format!("KentOS Launcher {}", env!("CARGO_PKG_VERSION"))),
    ]
    .spacing(px(10.0))
    .align_y(Alignment::Center);
    column![
        horizontal_divider(),
        container(content)
            .padding([0.0, px(16.0)])
            .width(Fill)
            .center_y(Length::Fixed(px(style::STATUS)))
            .style(style::status_bar),
    ]
    .into()
}

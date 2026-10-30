//! The status line under the sheet (docs/adr/0803): on the left whether
//! anything runs (“Hazır”, “2 uygulama çalışıyor”), on the right the launcher's
//! version. What is not checked is said where it matters, in the update centre.

use iced::widget::{Space, container, row, space, text};
use iced::{Alignment, Element, Fill, Length, Padding};
use kentos_ui::theme::{Tokens, typography};

use crate::app::{Launcher, Message};
use crate::view::style::{self, px};

pub fn bar(launcher: &Launcher) -> Element<'_, Message> {
    let running = launcher.processes.apps_running();
    let (said, busy) = match running {
        0 => ("Hazır".to_owned(), false),
        1 => ("1 uygulama çalışıyor".to_owned(), true),
        n => (format!("{n} uygulama çalışıyor"), true),
    };
    let dot = px(6.0);
    let lamp = container(Space::new())
        .width(dot)
        .height(dot)
        .style(move |theme: &iced::Theme| {
            let t = Tokens::of(theme);
            style::dot(if busy { t.accent_hover } else { t.success })(theme)
        });
    let words = |w: String| text(w).size(typography::caption()).style(style::soft);
    container(
        row![
            lamp,
            words(said),
            space::horizontal(),
            words(format!("KentOS Launcher {}", env!("CARGO_PKG_VERSION"))),
        ]
        .spacing(px(8.0))
        .align_y(Alignment::Center),
    )
    .padding(Padding {
        left: px(style::PAGE_X) + px(style::INSET),
        right: px(style::MARGIN) + px(style::PAGE_X) + px(style::INSET),
        ..Padding::ZERO
    })
    .width(Fill)
    .center_y(Length::Fixed(px(style::STATUS)))
    .into()
}

//! The notices' panel (docs/adr/0800 §4), under the bell: newest first, each
//! with its sentence, the file it is about and when; one by one or all
//! cleared. A click outside closes it.

use iced::widget::{
    Column, Space, button, column, container, mouse_area, opaque, row, scrollable, space, stack,
    text,
};
use iced::{Alignment, Element, Fill, Length, Padding, mouse};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::theme::typography;

use crate::app::{Launcher, Message};
use crate::recent::ago;
use crate::view::style::{self, px};

pub fn panel(launcher: &Launcher) -> Element<'_, Message> {
    let mut list = Column::new().spacing(px(2.0));
    for notice in launcher.notices.newest_first() {
        let mut words =
            column![text(notice.text.as_str()).size(typography::body())].spacing(px(3.0));
        if let Some(detail) = &notice.detail {
            words = words.push(label::mono_caption(detail.as_str()));
        }
        words = words.push(label::caption(ago(notice.at, launcher.now)));
        list = list.push(
            container(
                row![
                    icon(notice.severity.icon())
                        .size(px(16.0))
                        .tone(notice.severity.tone()),
                    words.width(Fill),
                    button(icon(Icon::Close).size(px(12.0)))
                        .padding(px(4.0))
                        .style(kentos_ui::style::button::flat)
                        .on_press(Message::DismissNotice(notice.id)),
                ]
                .spacing(px(10.0))
                .align_y(Alignment::Start),
            )
            .padding([px(8.0), px(10.0)]),
        );
    }
    let body: Element<'_, Message> = if launcher.notices.is_empty() {
        container(label::muted("Bildirim yok."))
            .padding([px(14.0), px(12.0)])
            .into()
    } else {
        scrollable(list)
            .direction(kentos_ui::style::field::thin_scrollbar())
            .height(Length::Shrink)
            .into()
    };
    let mut clear = button(text("Tümünü temizle").size(typography::caption()))
        .padding([px(3.0), px(8.0)])
        .style(style::link);
    if !launcher.notices.is_empty() {
        clear = clear.on_press(Message::ClearNotices);
    }
    let panel = container(
        column![
            row![label::strong("Bildirimler"), space::horizontal(), clear]
                .align_y(Alignment::Center)
                .padding([px(8.0), px(12.0)]),
            kentos_ui::widget::horizontal_divider(),
            container(body).max_height(px(420.0)),
        ]
        .width(px(400.0)),
    )
    .style(kentos_ui::style::container::popover);

    let dismiss = mouse_area(container(Space::new()).width(Fill).height(Fill))
        .interaction(mouse::Interaction::Idle)
        .on_press(Message::CloseNotices);
    let placed = container(opaque(panel))
        .width(Fill)
        .height(Fill)
        .align_right(Fill)
        .padding(Padding {
            top: px(style::PAGE_Y) + px(46.0),
            right: px(style::PAGE_X),
            ..Padding::ZERO
        });
    stack![dismiss, placed].into()
}

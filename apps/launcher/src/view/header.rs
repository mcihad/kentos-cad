//! The page's header (docs/adr/0800 §3): its title and what it is for, the
//! search on the pages that have something to search, the bell of the
//! notices with a dot while any is unread.

use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, row, space, stack, text};
use iced::{Alignment, Element, Fill, Length, Padding};
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::theme::{metrics, typography};
use kentos_ui::widget::{InputField, Tip, tip};

use crate::app::{Launcher, Message, Page, SEARCH};
use crate::glyphs;
use crate::view::glyph;
use crate::view::style::{self, px};

pub fn header(launcher: &Launcher) -> Element<'_, Message> {
    let (title, note) = match launcher.page {
        Page::Apps => ("Uygulamalarım", "Tüm KentOS araçları, tek merkezde."),
        Page::Recent => (
            "Son dosyalar",
            "KentOS programlarında açılan ve kaydedilen çizimler.",
        ),
        Page::Updates => ("Güncellemeler", "Kurulu uygulamalar ve sürümleri."),
        Page::Downloads => ("İndirmeler", "Launcher'ın indirdikleri."),
        Page::Settings => ("Ayarlar", "Launcher'ın görünüşü, davranışı ve klasörleri."),
    };
    let heading = column![
        text(title).size(px(28.0)).font(typography::ui_strong()),
        label::muted(note),
    ]
    .spacing(px(4.0));

    // The search takes what the title leaves, at most its own width.
    let search: Option<Element<'_, Message>> = match launcher.page {
        Page::Apps => Some(
            InputField::new("Uygulama ara", &launcher.query)
                .on_input(Message::Search)
                .on_submit(Message::SearchSubmit)
                .icon(Icon::Search)
                .clear(Message::Search(String::new()))
                .id(SEARCH)
                .into(),
        ),
        Page::Recent => Some(
            InputField::new("Dosya ara", &launcher.file_query)
                .on_input(Message::FileSearch)
                .icon(Icon::Search)
                .clear(Message::FileSearch(String::new()))
                .id(SEARCH)
                .into(),
        ),
        _ => None,
    };
    let room: Element<'_, Message> = match search {
        Some(search) => container(container(search).width(Fill).max_width(px(320.0)))
            .align_right(Fill)
            .into(),
        None => space::horizontal().into(),
    };
    row![heading, room, bell(launcher)]
        .spacing(px(12.0))
        .height(Length::Fixed(height()))
        .align_y(Alignment::Center)
        .into()
}

/// The header's height: the title and its note, whole pixels, with the
/// controls beside them centred on whole pixels.
fn height() -> f32 {
    const LINE: f32 = 1.3;
    let words = (px(28.0) * LINE + px(4.0) + typography::body() * LINE).ceil();
    style::around(metrics::control(), words.max(metrics::control()))
}

/// The bell: opens the notices; a dot while any is unread.
fn bell(launcher: &Launcher) -> Element<'_, Message> {
    let side = metrics::control();
    let unread = launcher.notices.unread();
    let button = button(
        container(glyph(glyphs::BELL, px(18.0)))
            .center_x(Length::Fixed(side))
            .center_y(Length::Fixed(side)),
    )
    .padding(0)
    .style(kentos_ui::style::button::flat)
    .on_press(Message::ToggleNotices);
    let words = match (launcher.notices.len(), unread) {
        (0, _) => "Bildirim yok".to_owned(),
        (n, 0) => format!("Bildirimler: {n}"),
        (n, u) => format!("Bildirimler: {n}, {u} okunmamış"),
    };
    let bell = tip(button, Tip::new(words), Position::Bottom);
    if unread == 0 {
        return bell;
    }
    let dot = px(9.0);
    stack![
        bell,
        container(
            container(Space::new())
                .width(dot)
                .height(dot)
                .style(style::bell_dot)
        )
        .padding(Padding {
            top: px(3.0),
            left: side - dot - px(3.0),
            ..Padding::ZERO
        })
    ]
    .into()
}

//! The page's header (docs/adr/0803): its title in a large, light hand and
//! what is on it; the search on the pages that have something to search,
//! Uygulama ekle on the apps' page and the bell of the notices with a dot
//! while any is unread.

use iced::widget::text::Wrapping;
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, row, space, stack, text};
use iced::{Alignment, Element, Fill, Length, Padding};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::theme::{metrics, typography};
use kentos_ui::widget::{InputField, Tip, fallback, tip};

use crate::app::{Launcher, Message, Page, SEARCH};
use crate::cards::Counts;
use crate::glyphs;
use crate::view::glyph;
use crate::view::style::{self, px};

/// The title's size at the default text size.
const TITLE: f32 = 28.0;

pub fn header(launcher: &Launcher) -> Element<'_, Message> {
    let (title, note): (&str, String) = match launcher.page {
        Page::Apps => (
            "Uygulamalarım",
            Counts::of(launcher.registry.apps.iter().map(|a| launcher.status(a))).sentence(),
        ),
        Page::Recent => (
            "Son dosyalar",
            "KentOS programlarında açılan ve kaydedilen çizimler.".to_owned(),
        ),
        Page::Updates => (
            "Güncellemeler",
            "Kurulu uygulamalar ve sürümleri.".to_owned(),
        ),
        Page::Downloads => ("İndirmeler", "Launcher'ın indirdikleri.".to_owned()),
        Page::Settings => (
            "Ayarlar",
            "Launcher'ın görünüşü, davranışı ve klasörleri.".to_owned(),
        ),
    };
    let heading = column![
        text(title).size(px(TITLE)).font(typography::ui()),
        label::muted(note),
    ]
    .spacing(px(2.0));

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
        Some(search) => container(container(search).width(Fill).max_width(px(260.0)))
            .align_right(Fill)
            .into(),
        None => space::horizontal().into(),
    };
    let mut line = row![heading, room]
        .spacing(px(12.0))
        .height(Length::Fixed(height()))
        .align_y(Alignment::Center);
    if launcher.page == Page::Apps {
        line = line.push(fallback(add(true), add(false)));
    }
    container(line.push(bell(launcher)))
        .padding([0.0, px(style::INSET)])
        .into()
}

/// Uygulama ekle; where the header is narrow, only its mark (its words in the tip).
fn add<'a>(words: bool) -> Element<'a, Message> {
    let side = metrics::control();
    if words {
        button(
            container(
                row![
                    icon(Icon::Plus).size(px(14.0)),
                    label::body("Uygulama ekle").wrapping(Wrapping::None)
                ]
                .spacing(px(6.0))
                .align_y(Alignment::Center),
            )
            .center_y(Length::Fixed(side)),
        )
        .height(Length::Fixed(side))
        .padding([0.0, px(10.0)])
        .style(style::quiet)
        .on_press(Message::AddApp)
        .into()
    } else {
        tip(
            button(
                container(icon(Icon::Plus).size(px(14.0)))
                    .center_x(Length::Fixed(side))
                    .center_y(Length::Fixed(side)),
            )
            .padding(0)
            .style(style::quiet)
            .on_press(Message::AddApp),
            Tip::new("Uygulama ekle"),
            Position::Bottom,
        )
    }
}

/// The header's height: the title and its note, whole pixels, with the
/// controls beside them centred on whole pixels.
fn height() -> f32 {
    const LINE: f32 = 1.3;
    let words = (px(TITLE) * LINE + px(2.0) + typography::body() * LINE).ceil();
    style::around(metrics::control(), words.max(metrics::control()))
}

/// The bell: opens the notices; a dot while any is unread.
fn bell(launcher: &Launcher) -> Element<'_, Message> {
    let side = metrics::control();
    let unread = launcher.notices.unread();
    let button = button(
        container(glyph(glyphs::BELL, px(16.0)))
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

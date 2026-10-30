//! The navigation (docs/adr/0800 §3): KentOS's mark and name, the pages,
//! Ayarlar at the bottom. The selected page has the accent's soft ground
//! and a bar at its edge; Güncellemeler carries a count only when there is
//! something to update, which there is not while no update service exists.

use iced::widget::{Space, button, column, container, row, space, text};
use iced::{Alignment, Element, Fill, Length};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{brand, typography};
use kentos_ui::widget::brand_mark;

use crate::app::{Launcher, Message, Page};
use crate::glyphs;
use crate::view::glyph;
use crate::view::style::{self, px};

pub fn sidebar(launcher: &Launcher) -> Element<'_, Message> {
    let mode = launcher.mode();
    let word = px(24.0);
    let mark = row![
        brand_mark(px(30.0)),
        row![
            text("Kent").size(word).font(typography::ui_strong()),
            text("OS")
                .size(word)
                .font(typography::ui_strong())
                .color(brand(mode).text),
        ],
    ]
    .spacing(px(10.0))
    .align_y(Alignment::Center);

    let pages = column![
        item(
            launcher.page,
            Page::Apps,
            Mark::Svg(glyphs::APPS),
            "Uygulamalar",
            0
        ),
        item(
            launcher.page,
            Page::Recent,
            Mark::Icon(Icon::Clock),
            "Son dosyalar",
            0
        ),
        // No update service yet (docs/adr/0800 §6): nothing to count.
        item(
            launcher.page,
            Page::Updates,
            Mark::Icon(Icon::Retry),
            "Güncellemeler",
            0
        ),
        item(
            launcher.page,
            Page::Downloads,
            Mark::Icon(Icon::Import),
            "İndirmeler",
            0
        ),
    ]
    .spacing(px(4.0));

    container(
        column![
            container(mark).padding([px(6.0), px(10.0)]),
            Space::new().height(px(18.0)),
            pages,
            space::vertical(),
            item(
                launcher.page,
                Page::Settings,
                Mark::Svg(glyphs::SETTINGS),
                "Ayarlar",
                0
            ),
        ]
        .height(Fill),
    )
    .width(px(style::SIDEBAR))
    .height(Fill)
    .padding([px(18.0), px(12.0)])
    .style(style::sidebar)
    .into()
}

/// An item's mark: KentOS UI's icon or a glyph of the web's grid.
enum Mark {
    Icon(Icon),
    Svg(&'static str),
}

fn item(current: Page, page: Page, mark: Mark, label: &str, count: usize) -> Element<'_, Message> {
    let selected = current == page;
    let size = px(18.0);
    let mark: Element<'_, Message> = match mark {
        Mark::Icon(i) => icon(i).size(size).into(),
        Mark::Svg(markup) => glyph(markup, size),
    };
    let name = if selected {
        text(label).font(typography::ui_strong())
    } else {
        text(label).font(typography::ui())
    }
    .size(px(15.0));
    let bar: Element<'_, Message> = if selected {
        container(Space::new())
            .width(3)
            .height(px(18.0))
            .style(style::nav_bar)
            .into()
    } else {
        Space::new().width(3).into()
    };
    let mut content = row![bar, mark, name]
        .spacing(px(10.0))
        .align_y(Alignment::Center);
    if count > 0 {
        content = content.push(space::horizontal()).push(
            container(
                text(count.to_string())
                    .size(typography::caption())
                    .font(typography::ui_strong()),
            )
            .padding([1.0, px(7.0)])
            .style(kentos_ui::style::container::count),
        );
    }
    button(container(content).width(Fill).center_y(Fill))
        .width(Fill)
        .height(Length::Fixed(px(style::NAV_ITEM)))
        .padding([0.0, px(8.0)])
        .style(style::nav(selected))
        .on_press(Message::Page(page))
        .into()
}

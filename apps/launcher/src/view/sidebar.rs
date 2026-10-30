//! The navigation (docs/adr/0803): KentOS's mark and name, the pages, Ayarlar
//! at the bottom, on the table. The selected page is a layer of the table with
//! its mark in the accent; under them a fragment of a zoning plan fades in and
//! out, in hairlines, where the table is empty.

use std::sync::LazyLock;

use iced::widget::{Space, button, column, container, row, space, stack, svg, text};
use iced::{Alignment, ContentFit, Element, Fill, FillPortion, Length, Padding};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Mode, Tokens, brand, typography};
use kentos_ui::widget::brand_mark;

use crate::app::{Launcher, Message, Page};
use crate::glyphs;
use crate::view::style::{self, px};

/// The drawing, from `scripts/launcher/art.py`.
static PLAN: LazyLock<svg::Handle> = LazyLock::new(|| {
    svg::Handle::from_memory(include_bytes!("../../assets/art/imar.svg").as_slice())
});

/// The sidebar's padding across and at the top and bottom; its mark's height and
/// what is under it. The plan starts where the pages end.
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 14.0;
const MARK: f32 = 26.0;
const UNDER_MARK: f32 = 22.0;
const ITEM_GAP: f32 = 2.0;
const PAGES: f32 = 4.0;

pub fn sidebar(launcher: &Launcher) -> Element<'_, Message> {
    let mode = launcher.mode();
    let word = px(18.0);
    let mark = row![
        brand_mark(px(MARK)),
        row![
            text("Kent").size(word).font(typography::ui_strong()),
            text("OS")
                .size(word)
                .font(typography::ui())
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
            "Uygulamalar"
        ),
        item(
            launcher.page,
            Page::Recent,
            Mark::Icon(Icon::Clock),
            "Son dosyalar"
        ),
        // No update service yet (docs/adr/0800 §6): nothing to count.
        item(
            launcher.page,
            Page::Updates,
            Mark::Icon(Icon::Retry),
            "Güncellemeler"
        ),
        item(
            launcher.page,
            Page::Downloads,
            Mark::Icon(Icon::Import),
            "İndirmeler"
        ),
    ]
    .spacing(px(ITEM_GAP));

    let content = container(
        column![
            container(mark).padding([0.0, px(8.0)]),
            Space::new().height(px(UNDER_MARK)),
            pages,
            space::vertical(),
            item(
                launcher.page,
                Page::Settings,
                Mark::Svg(glyphs::SETTINGS),
                "Ayarlar"
            ),
        ]
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .padding([px(PAD_Y), px(PAD_X)]);

    let above = px(PAD_Y)
        + px(MARK)
        + px(UNDER_MARK)
        + PAGES * px(style::NAV_ITEM)
        + (PAGES - 1.0) * px(ITEM_GAP);
    let below = px(PAD_Y) + px(style::NAV_ITEM);
    // The table over everything but the plan's middle: it fades in and out
    // whatever the window's height, and what the software renderer would leave
    // of a picture larger than its place stays out of sight.
    let veil = column![
        container(Space::new())
            .width(Fill)
            .height(above)
            .style(style::table),
        container(Space::new())
            .width(Fill)
            .height(FillPortion(3))
            .style(style::fade(true)),
        Space::new().height(FillPortion(2)),
        container(Space::new())
            .width(Fill)
            .height(px(72.0))
            .style(style::fade(false)),
        container(Space::new())
            .width(Fill)
            .height(below)
            .style(style::table),
    ];
    let plan = container(
        svg(PLAN.clone())
            .width(Fill)
            .height(Fill)
            .content_fit(ContentFit::Cover)
            .opacity(if mode == Mode::Light { 0.26 } else { 0.22 })
            .style(|theme: &iced::Theme, _| svg::Style {
                color: Some(Tokens::of(theme).text),
            }),
    )
    .width(Fill)
    .height(Fill)
    .padding(Padding {
        top: above,
        bottom: below,
        ..Padding::ZERO
    });

    container(stack![plan, veil, content])
        .width(px(style::SIDEBAR))
        .height(Fill)
        .into()
}

/// An item's mark: KentOS UI's icon or a glyph of the web's grid.
enum Mark {
    Icon(Icon),
    Svg(&'static str),
}

fn item(current: Page, page: Page, mark: Mark, label: &str) -> Element<'_, Message> {
    let selected = current == page;
    let size = px(16.0);
    let tone = if selected {
        Tone::Highlight
    } else {
        Tone::Inherit
    };
    let mark = match mark {
        Mark::Icon(i) => icon(i),
        Mark::Svg(markup) => icon(Icon::Svg(markup)),
    }
    .size(size)
    .tone(tone);
    let name = if selected {
        text(label).font(typography::ui_strong())
    } else {
        text(label).font(typography::ui())
    }
    .size(typography::body());
    let content = row![mark, name]
        .spacing(px(10.0))
        .align_y(Alignment::Center);
    button(container(content).width(Fill).center_y(Fill))
        .width(Fill)
        .height(Length::Fixed(px(style::NAV_ITEM)))
        .padding([0.0, px(8.0)])
        .style(style::nav(selected))
        .on_press(Message::Page(page))
        .into()
}

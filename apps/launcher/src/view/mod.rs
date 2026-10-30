//! The launcher's window (docs/adr/0803): the navigation on the table, the
//! sheet with the page's header and content, the status line under it; the
//! notices and the dialogs over them.

mod app_menu;
mod apps;
mod dialog;
mod header;
#[cfg(test)]
mod interaction_tests;
mod notices;
mod pages;
mod plates;
mod sidebar;
mod status;
pub mod style;
mod tables;

use iced::widget::{column, container, row, stack, svg};
use iced::{Element, Fill, Length, Padding};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::overlay;

use crate::app::{Dialog, Launcher, Message, Page};
use crate::glyphs::Glyph;
use crate::manifest::{self, Manifest};
use crate::tones::Tone;
use style::px;

/// The window.
pub fn view(launcher: &Launcher) -> Element<'_, Message> {
    let body: Element<'_, Message> = match launcher.page {
        Page::Apps => apps::page(launcher),
        Page::Recent => pages::recent(launcher),
        Page::Updates => pages::updates(launcher),
        Page::Downloads => pages::downloads(launcher),
        Page::Settings => pages::settings(launcher),
    };
    let content = column![header::header(launcher), body]
        .spacing(px(style::SECTION))
        .max_width(px(style::PAGE_MOST))
        .height(Fill);
    let sheet = container(content)
        .center_x(Fill)
        .height(Fill)
        .padding(Padding {
            top: px(style::PAGE_TOP),
            right: px(style::PAGE_X),
            bottom: px(style::PAGE_BOTTOM),
            left: px(style::PAGE_X),
        })
        .style(style::sheet);
    let main = column![
        container(sheet).width(Fill).height(Fill).padding(Padding {
            top: px(style::MARGIN),
            right: px(style::MARGIN),
            ..Padding::ZERO
        }),
        status::bar(launcher),
    ]
    .width(Fill)
    .height(Fill);
    let window = row![sidebar::sidebar(launcher), main].height(Fill);
    let mut layers = stack![
        container(window)
            .width(Fill)
            .height(Fill)
            .style(style::table)
    ];
    if launcher.notices_open {
        layers = layers.push(notices::panel(launcher));
    }
    match &launcher.dialog {
        Some(Dialog::Edit(draft)) => {
            layers = layers.push(overlay::blocking(dialog::edit(launcher, draft)));
        }
        Some(Dialog::Remove { id, name }) => {
            layers = layers.push(overlay::modal(
                dialog::remove(id, name),
                Message::CloseDialog,
            ));
        }
        None => {}
    }
    layers.into()
}

/// A glyph of the web's grid, drawn in its element's text colour.
pub fn glyph<'a>(markup: &'static str, size: f32) -> Element<'a, Message> {
    icon(Icon::Svg(markup)).size(size).into()
}

/// A tile of `side` pixels: the tone's ground and the glyph in its ink.
pub fn tile<'a>(glyph: Glyph, tone: Tone, side: f32) -> Element<'a, Message> {
    let paint = tone.paint();
    // The glyph a little over half the tile, a whole number of pixels, centred on whole pixels.
    let mark = style::centred_in(side, (side * 0.56).round());
    container(icon(Icon::Svg(glyph.svg())).size(mark).color(paint.ink))
        .center_x(Length::Fixed(side))
        .center_y(Length::Fixed(side))
        .style(style::tile(paint, side))
        .into()
}

/// An app's tile: its glyph on its tone, or its own picture file as it is.
pub fn app_tile<'a>(launcher: &'a Launcher, app: &'a Manifest, side: f32) -> Element<'a, Message> {
    match &app.icon {
        manifest::Icon::Glyph(glyph) => tile(*glyph, app.tone, side),
        manifest::Icon::File(path) => {
            let is_svg = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
            if is_svg && path.is_file() {
                svg(svg::Handle::from_path(path))
                    .width(side)
                    .height(side)
                    .into()
            } else if let Some(handle) = launcher.pictures.ready_file(path) {
                iced::widget::image(handle.clone())
                    .width(side)
                    .height(side)
                    .content_fit(iced::ContentFit::Cover)
                    .border_radius(style::tile_radius(side))
                    .into()
            } else {
                tile(Glyph::default(), app.tone, side)
            }
        }
    }
}

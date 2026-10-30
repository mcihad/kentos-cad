//! The sheet's tables (docs/adr/0803): a section's head, hairlines between
//! rows and the rows of the recent drawings and of the installed apps, single
//! lines in columns, as a map book's index. Nothing around them.

use std::path::Path;

use iced::widget::text::Wrapping;
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, row, space, text};
use iced::{Alignment, Element, Fill, Length, alignment::Horizontal};
use kentos_launch::Module;
use kentos_ui::label;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Tip, tip};

use crate::app::{Launcher, Message, Page};
use crate::cards::Status;
use crate::glyphs::Glyph;
use crate::manifest::Manifest;
use crate::recent::Row;
use crate::resolve::Found;
use crate::tones::Tone;
use crate::view::style::{self, px};
use crate::view::{app_tile, tile};

/// A section's head: its title and, when there is one, the page that shows
/// the whole of it; both words in line with the rows under them.
pub fn head<'a>(title: &'a str, whole: Option<Page>) -> Element<'a, Message> {
    let word_pad = px(6.0);
    let mut line = row![label::heading(title), space::horizontal()].align_y(Alignment::Center);
    if let Some(page) = whole {
        line = line.push(
            button(text("Tümünü gör").size(typography::caption()))
                .padding([px(2.0), word_pad])
                .style(style::aside)
                .on_press(Message::Page(page)),
        );
    }
    container(line)
        .padding(iced::Padding {
            left: px(style::INSET),
            right: if whole.is_some() {
                px(style::INSET) - word_pad
            } else {
                px(style::INSET)
            },
            ..iced::Padding::ZERO
        })
        .height(px(style::HEAD))
        .center_y(Length::Fixed(px(style::HEAD)))
        .into()
}

/// The hairline between two rows, in line with what they say.
pub fn rule<'a>() -> Element<'a, Message> {
    container(
        container(Space::new())
            .width(Fill)
            .height(1)
            .style(style::hairline),
    )
    .padding([0.0, px(style::INSET)])
    .width(Fill)
    .into()
}

/// `rows` of a table one under the other, a hairline between each two.
pub fn lines<'a>(rows: impl IntoIterator<Item = Element<'a, Message>>) -> Element<'a, Message> {
    let mut list = iced::widget::Column::new();
    for (i, line) in rows.into_iter().enumerate() {
        if i > 0 {
            list = list.push(rule());
        }
        list = list.push(line);
    }
    list.width(Fill).into()
}

/// How high `count` rows are with their hairlines.
pub fn lines_height(count: usize) -> f32 {
    count as f32 * px(style::ROW) + count.saturating_sub(1) as f32
}

/// A module's tile on a table's row.
fn module_tile<'a>(module: Option<Module>, dim: bool, side: f32) -> Element<'a, Message> {
    let (glyph, tone) = match module {
        Some(Module::Architecture) => (Glyph::Mimari, Tone::Purple),
        Some(Module::Map) => (Glyph::Harita, Tone::Blue),
        Some(Module::Gis) => (Glyph::Cbs, Tone::Green),
        None => (Glyph::Klasor, Tone::Gray),
    };
    tile(glyph, if dim { Tone::Gray } else { tone }, side)
}

/// What each column of the recent drawings gets of a row `width` wide: the
/// name's, the folder's and the information's room (a narrow row gives up the
/// information, then the folder).
fn recent_columns(width: f32) -> (f32, f32, f32) {
    let rest = width - 2.0 * px(style::INSET) - when_width() - 2.0 * px(16.0);
    let rest = rest - px(style::ROW_TILE) - px(10.0);
    if rest >= px(520.0) {
        (
            (rest * 0.36).floor(),
            (rest * 0.32).floor(),
            (rest * 0.32).floor(),
        )
    } else if rest >= px(320.0) {
        ((rest * 0.55).floor(), (rest * 0.45).floor(), 0.0)
    } else {
        (rest.floor(), 0.0, 0.0)
    }
}

/// The “ne zaman” column's width.
fn when_width() -> f32 {
    px(92.0)
}

/// A recent drawing on a row `width` wide: its module's tile, its name, its
/// folder, what the program wrote about it, when; a file no longer there
/// dimmed and said, and not to be opened.
pub fn recent_row<'a>(
    launcher: &'a Launcher,
    index: usize,
    row_of: &'a Row,
    width: f32,
) -> Element<'a, Message> {
    let module = row_of.module.map_or("KentOS", Module::title);
    let dim = row_of.missing;
    let (name_room, folder_room, info_room) = recent_columns(width);
    let faded = |theme: &iced::Theme| iced::widget::text::Style {
        color: Some(Tokens::of(theme).disabled()),
    };
    let name = style::fit(&row_of.name, typography::body(), name_room, true).into_owned();
    let name = if dim {
        label::strong(name).style(faded)
    } else {
        label::strong(name)
    }
    .wrapping(Wrapping::None);
    let mut line = row![
        module_tile(row_of.module, dim, px(style::ROW_TILE)),
        container(name).width(Length::Fixed(name_room)),
    ]
    .spacing(px(10.0))
    .align_y(Alignment::Center);
    if folder_room > 0.0 {
        let folder =
            style::fit(&row_of.folder, typography::caption(), folder_room, false).into_owned();
        line = line.push(
            container(
                label::caption(folder)
                    .style(if dim {
                        faded
                    } else {
                        kentos_ui::style::text::muted
                    })
                    .wrapping(Wrapping::None),
            )
            .width(Length::Fixed(folder_room + px(6.0))),
        );
    }
    if info_room > 0.0 {
        let said: Element<'a, Message> = if dim {
            label::caption("Dosya yerinde yok")
                .font(typography::ui_strong())
                .style(kentos_ui::style::text::warning)
                .wrapping(Wrapping::None)
                .into()
        } else {
            // The program writes “334 nesne · Yerel”; the sheet reads it as a phrase.
            let info = row_of.info.replace(" · ", ", ");
            label::caption(style::fit(&info, typography::caption(), info_room, false).into_owned())
                .style(style::soft)
                .wrapping(Wrapping::None)
                .into()
        };
        line = line.push(container(said).width(Length::Fixed(info_room)));
    }
    line = line.push(space::horizontal()).push(
        label::caption(row_of.ago.as_str())
            .width(Length::Fixed(when_width()))
            .align_x(Horizontal::Right)
            .style(if dim {
                faded
            } else {
                kentos_ui::style::text::muted
            })
            .wrapping(Wrapping::None),
    );
    let spec = launcher.recent_spec(row_of);
    let mut button = button(container(line).center_y(Fill))
        .width(Fill)
        .height(Length::Fixed(px(style::ROW)))
        .padding([0.0, px(style::INSET)])
        .style(style::row);
    if !dim && spec.is_ok() {
        button = button.on_press(Message::OpenRecent(index));
    }
    let hint = if dim {
        Tip::new("Dosya yerinde yok").body(format!(
            "{} taşınmış ya da silinmiş olabilir; KentOS'ta Aç ile yeni yerinden açın.",
            row_of.path.display()
        ))
    } else {
        match spec {
            Ok(spec) => {
                Tip::new(format!("{module} ile aç")).detail(command(&spec.program, &spec.args))
            }
            Err(why) => Tip::new("Açılamaz").body(why),
        }
    };
    tip(button, hint, Position::Bottom)
}

/// The command line a tip shows: the program's name and its arguments.
fn command(program: &Path, args: &[std::ffi::OsString]) -> String {
    let name = program.file_name().map_or_else(
        || program.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut line = name;
    for arg in args {
        let arg = arg.to_string_lossy();
        if arg.contains(' ') {
            line.push_str(&format!(" \"{arg}\""));
        } else {
            line.push(' ');
            line.push_str(&arg);
        }
    }
    line
}

/// An installed app's version on a short row (the update centre): its tile,
/// its name, its version.
pub fn version_row<'a>(launcher: &'a Launcher, app: &'a Manifest) -> Element<'a, Message> {
    let version = app
        .version
        .as_deref()
        .map_or_else(|| "—".to_owned(), |v| format!("v{v}"));
    container(
        row![
            app_tile(launcher, app, px(style::ROW_TILE)),
            label::body(app.name.as_str()).wrapping(Wrapping::None),
            space::horizontal(),
            label::caption(version).style(style::soft),
        ]
        .spacing(px(10.0))
        .align_y(Alignment::Center),
    )
    .padding([0.0, px(style::INSET)])
    .width(Fill)
    .center_y(Length::Fixed(px(style::ROW)))
    .into()
}

/// An app on the page of the installed ones, on a row `width` wide: its tile,
/// its name, where it runs from, its version and its state.
pub fn installed_row<'a>(
    launcher: &'a Launcher,
    app: &'a Manifest,
    width: f32,
) -> Element<'a, Message> {
    let status = launcher.status(app);
    let place = match launcher.found.get(&app.id) {
        Some(Found::Program(spec)) => spec.program.display().to_string(),
        Some(Found::Url(url)) => url.clone(),
        Some(Found::Missing(_)) => "Programı bulunamadı".to_owned(),
        Some(Found::Soon) | None => "Henüz çıkmadı".to_owned(),
    };
    let version = app
        .version
        .as_deref()
        .map_or_else(|| "—".to_owned(), |v| format!("v{v}"));
    let tone = move |theme: &iced::Theme| {
        let t = Tokens::of(theme);
        iced::widget::text::Style {
            color: Some(match status {
                Status::Installed => t.muted,
                Status::Running(_) => t.accent_hover,
                Status::Missing => t.warning,
                Status::Web => t.info,
                Status::Soon => t.faint,
            }),
        }
    };
    let side = px(style::ROW_TILE);
    let name_width = px(170.0);
    let version_width = px(72.0);
    let state_width = px(96.0);
    let room = width
        - 2.0 * px(style::INSET)
        - side
        - name_width
        - version_width
        - state_width
        - 4.0 * px(12.0);
    container(
        row![
            app_tile(launcher, app, side),
            label::strong(app.name.as_str())
                .wrapping(Wrapping::None)
                .width(Length::Fixed(name_width)),
            label::caption(style::fit(&place, typography::caption(), room, false).into_owned())
                .style(style::soft)
                .wrapping(Wrapping::None)
                .width(Fill),
            label::caption(version)
                .style(kentos_ui::style::text::muted)
                .wrapping(Wrapping::None)
                .width(Length::Fixed(version_width)),
            label::caption(status.word())
                .style(tone)
                .wrapping(Wrapping::None)
                .width(Length::Fixed(state_width))
                .align_x(Horizontal::Right),
        ]
        .spacing(px(12.0))
        .align_y(Alignment::Center),
    )
    .padding([0.0, px(style::INSET)])
    .width(Fill)
    .center_y(Length::Fixed(px(style::ROW)))
    .into()
}

/// A short sentence where a table has no rows yet.
pub fn empty_line<'a>(words: &'a str) -> Element<'a, Message> {
    container(label::muted(words))
        .padding([0.0, px(style::INSET)])
        .width(Fill)
        .center_y(Length::Fixed(px(style::ROW)))
        .into()
}

/// A column of a head and what is under it, a section of the page.
pub fn section<'a>(
    title: &'a str,
    whole: Option<Page>,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![head(title, whole), body.into()].width(Fill).into()
}

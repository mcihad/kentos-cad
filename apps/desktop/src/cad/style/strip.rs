//! The tool strip over a style window's canvas (SVG çizim düzenleyicisi, Sembol tasarımcısı): one
//! surface with a rule under it, flat buttons that show their edge only under the pointer, switches
//! tinted while on, menus that look like the buttons beside them with a chevron, and thin dividers
//! between the groups. Every control is the inline height, so the strip reads as one bar; on a
//! narrow window it wraps, group after group.

use iced::widget::{Row, button, container, row, space, tooltip};
use iced::{Border, Center, Element, Fill, Theme};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, metrics, shape, typography};
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};

use crate::app::Message;

/// An icon's size on the strip.
const GLYPH: f32 = 14.0;

/// The strip: its items in one row (wrapping on a narrow window) on the toolbar's surface.
pub fn strip<'a>(items: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    surface(
        Row::with_children(items)
            .spacing(1)
            .align_y(Center)
            .wrap()
            .vertical_spacing(4),
    )
}

/// A row laid out by its window (a chooser left, the zoom right) on the strip's surface: the panel
/// headers' ground, so the strip reads apart from the canvas under it and the panels beside it.
pub fn surface<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding([4, 6])
        .width(Fill)
        .style(|t: &Theme| container::Style {
            background: Some(iced::Background::Color(Tokens::of(t).header)),
            ..container::Style::default()
        })
        .into()
}

/// The rule under a strip (a strip sits over a canvas or a picture).
pub fn rule<'a>() -> Element<'a, Message> {
    kentos_ui::widget::horizontal_divider().into()
}

/// A gap between two groups: a short vertical rule with room on both sides.
pub fn divider<'a>() -> Element<'a, Message> {
    container(
        container(space())
            .width(1)
            .height(typography::scaled(18.0))
            .style(|t: &Theme| container::Style {
                background: Some(iced::Background::Color(Tokens::of(t).border)),
                ..container::Style::default()
            }),
    )
    .padding([0, 3])
    .into()
}

fn face<'a>(glyph: Option<Icon>, words: Option<&str>, chevron: bool) -> Element<'a, Message> {
    let mut r = Row::new().spacing(6).align_y(Center);
    if let Some(g) = glyph {
        r = r.push(icon(g).size(GLYPH));
    }
    if let Some(w) = words {
        r = r.push(label::body(w.to_owned()).wrapping(iced::widget::text::Wrapping::None));
    }
    if chevron {
        r = r.push(icon(Icon::ChevronDown).size(11.0).tone(Tone::Muted));
    }
    let pad = if words.is_some() { [0, 7] } else { [0, 0] };
    let c = container(r)
        .padding(pad)
        .height(metrics::inline())
        .align_y(Center);
    if words.is_none() && !chevron {
        c.center_x(metrics::inline() + 4.0).into()
    } else {
        c.into()
    }
}

/// The control's tip: “Ad: what it does” is the tip's title over its body, as on the ribbon.
fn tipped<'a>(content: impl Into<Element<'a, Message>>, words: &str) -> Element<'a, Message> {
    let tip_text = match words.split_once(": ") {
        Some((title, body)) => Tip::new(title.to_owned()).body(body.to_owned()),
        None => Tip::new(words.to_owned()),
    };
    tip(content, tip_text, tooltip::Position::Bottom)
}

/// A button with words only (“1:1”).
pub fn text_button<'a>(words: &str, help: &str, press: Option<Message>) -> Element<'a, Message> {
    tipped(
        button(face(None, Some(words), false))
            .padding(0)
            .style(style::button::flat)
            .on_press_maybe(press),
        help,
    )
}

/// A square button with its icon only; its words are its tip.
pub fn icon_button<'a>(glyph: Icon, help: &str, press: Option<Message>) -> Element<'a, Message> {
    tipped(
        button(face(Some(glyph), None, false))
            .padding(0)
            .style(style::button::flat)
            .on_press_maybe(press),
        help,
    )
}

/// A switch with its icon only; its words are its tip.
pub fn icon_toggle<'a>(glyph: Icon, help: &str, on: bool, press: Message) -> Element<'a, Message> {
    tipped(
        button(face(Some(glyph), None, false))
            .padding(0)
            .style(style::button::tool(on))
            .on_press(press),
        help,
    )
}

/// A button that opens its menu under it; the chevron says so.
pub fn menu<'a>(
    glyph: Icon,
    words: &str,
    help: &str,
    menu: impl Fn() -> Menu<Message> + 'a,
) -> Element<'a, Message> {
    tipped(
        MenuButton::new(face(Some(glyph), Some(words), true), menu),
        help,
    )
}

/// A switch with its options' menu beside it, joined (Kenet ▾); its words are its tip.
pub fn split_toggle<'a>(
    glyph: Icon,
    help: &str,
    on: bool,
    press: Message,
    menu_help: &str,
    menu: impl Fn() -> Menu<Message> + 'a,
) -> Element<'a, Message> {
    let chevron = container(icon(Icon::ChevronDown).size(11.0).tone(Tone::Muted))
        .center_x(typography::scaled(16.0))
        .center_y(metrics::inline());
    row![
        icon_toggle(glyph, help, on, press),
        tipped(MenuButton::new(chevron, menu), menu_help),
    ]
    .spacing(0)
    .align_y(Center)
    .into()
}

/// A value the strip shows (the zoom): figures in a sunken box, the width of `room` characters.
pub fn readout<'a>(text: String, help: &str, room: f32) -> Element<'a, Message> {
    tipped(
        container(label::caption(text).font(typography::mono()))
            .center_x(typography::scaled(room))
            .center_y(metrics::inline() - 2.0)
            .style(|t: &Theme| {
                let k = Tokens::of(t);
                container::Style {
                    background: Some(iced::Background::Color(k.field)),
                    border: Border {
                        color: k.border,
                        width: 1.0,
                        radius: shape::radius(3.0).into(),
                    },
                    text_color: Some(k.muted),
                    ..container::Style::default()
                }
            }),
        help,
    )
}

//! The designer's foot and its question (the web's footer and `askUnsaved`):
//! name and category, or the note on a layer style's symbol; what the window
//! said; Vazgeç and Kaydet (Uygula); and the question on closing with changes.

use iced::widget::{Row, button, column, container, row, space, text_input};
use iced::{Center, Element, Fill, Length, Theme};
use kentos_native_style::designer::texts;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};

use super::{Designer, Event, ev, field_id};
use crate::app::Message;

/// The foot: name and category (or the note on a layer style's symbol), what
/// the window said, Vazgeç and Kaydet or Uygula.
pub(super) fn footer(d: &Designer) -> Element<'_, Message> {
    let mut foot = Row::new().spacing(10).align_y(Center);
    if d.inline().is_some() {
        foot = foot.push(label::caption(texts::INLINE).style(style::text::muted));
    } else {
        let field = |id: &str, hint: &str, value: &str, width: f32, on: fn(String) -> Event| {
            text_input(hint, value)
                .id(field_id(id))
                .on_input(move |t| ev(on(t)))
                .padding([4, 7])
                .size(typography::body())
                .style(style::field::validated(false))
                .width(Length::Fixed(typography::from_default(width)))
        };
        foot = foot
            .push(
                row![
                    label::body("Ad").style(style::text::muted),
                    field("name", "", &d.draft.name, 190.0, Event::Name)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .push(
                row![
                    label::body("Kategori").style(style::text::muted),
                    field("path", "Ana / Alt", &d.path_text, 230.0, Event::Path)
                ]
                .spacing(6)
                .align_y(Center),
            );
    }
    let status: Element<'_, Message> = match d.said() {
        Some((text, warn)) => {
            let warn = *warn;
            row![
                icon(if warn { Icon::Warning } else { Icon::Check })
                    .size(14.0)
                    .tone(if warn { Tone::Warning } else { Tone::Success }),
                label::body(text.clone()).style(move |t: &Theme| iced::widget::text::Style {
                    color: Some(if warn {
                        Tokens::of(t).warning
                    } else {
                        Tokens::of(t).muted
                    }),
                }),
            ]
            .spacing(6)
            .align_y(Center)
            .into()
        }
        None => space().into(),
    };
    let save_words = if d.inline().is_some() {
        "Uygula"
    } else {
        "Kaydet"
    };
    foot.push(container(status).width(Fill))
        .push(
            button(label::body("Vazgeç"))
                .padding([5, 14])
                .style(style::button::secondary)
                .on_press(ev(Event::Close)),
        )
        .push(
            button(
                row![
                    icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
                    label::body(save_words).style(style::text::on_accent)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([5, 14])
            .style(style::button::primary)
            .on_press(ev(Event::Save)),
        )
        .into()
}

/// Closing with changes (the web's `askUnsaved`, DESIGN.md §7.9.1): the
/// leaving answer aside on the left, Vazgeç, and saving first in weight.
pub(super) fn question(d: &Designer) -> Element<'_, Message> {
    let inline = d.inline();
    let (title, state, save, without) = if inline.is_some() {
        (
            "Uygulanmamış değişiklikler",
            "uygulanmamış",
            "Uygula",
            "Uygulamadan",
        )
    } else {
        (
            "Kaydedilmemiş değişiklikler",
            "kaydedilmemiş",
            "Kaydet",
            "Kaydetmeden",
        )
    };
    let name = inline.map_or_else(|| d.saved_as().0, str::to_owned);
    let symbol = container(icon(Icon::Warning).size(18.0).tone(Tone::Warning))
        .center_x(36)
        .center_y(36)
        .style(|t: &Theme| container::Style {
            background: Some(iced::Background::Color(
                Tokens::of(t).warning.scale_alpha(0.14),
            )),
            border: iced::border::rounded(18.0),
            ..container::Style::default()
        });
    let text = column![
        label::title(title),
        label::body(format!(
            "“{name}” içinde {state} değişiklikler var. Pencere kapanırsa bu değişiklikler kaybolur."
        )),
    ]
    .spacing(6)
    .width(Fill);
    let answer = |words: String, e: Event| {
        button(label::body(words))
            .padding([5, 14])
            .style(style::button::secondary)
            .on_press(ev(e))
    };
    let actions = row![
        answer(format!("{without} kapat"), Event::Discard),
        space::horizontal(),
        answer("Vazgeç".to_owned(), Event::Stay),
        button(label::body(format!("{save} ve kapat")).style(style::text::on_accent))
            .padding([5, 14])
            .style(style::button::primary)
            .on_press(ev(Event::SaveAndClose)),
    ]
    .spacing(6)
    .align_y(Center);
    container(column![row![symbol, text].spacing(14), actions].spacing(18))
        .width(typography::scaled(480.0))
        .padding(18)
        .style(style::container::popover)
        .into()
}

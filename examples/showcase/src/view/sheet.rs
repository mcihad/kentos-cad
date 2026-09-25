//! Model ve düzen sekmeleri; düzenin kâğıt paftası.
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │   ┌──────────────────────────────────────┐   │
//! │   │ ┌──────────────────────────────────┐ │   │
//! │   │ │ harita çerçevesi                 │ │   │
//! │   │ └──────────────────────────────────┘ │   │
//! │   │ KentOS CAD │ Pafta │ Kâğıt │ Katman  │   │
//! │   └──────────────────────────────────────┘   │
//! ├ Model │ Düzen 1 │ Düzen 2 │ + ───────────────┤
//! ```

use iced::widget::text::Wrapping;
use iced::widget::{column, container, responsive, row, space, themer};
use iced::{Border, Element, Fill, Shadow, Size, Theme, Vector};

use kentos_rc::label;
use kentos_rc::spatial::model_space::{Backdrop, Style};
use kentos_rc::spatial::{ModelSpace, Tool};
use kentos_rc::theme::{self, Mode, Tokens, typography};
use kentos_rc::widget::{Tab, Tabs};

use crate::app::Showcase;
use crate::message::Message;
use crate::sheets::Sheet;

/// Kâğıt: A3 yatay, milimetre.
const PAPER: Size = Size::new(420.0, 297.0);
/// Kâğıdın çevresindeki boşluk; kâğıdın kenar payı, kâğıt genişliğine
/// oranla.
const DESK: f32 = 28.0;
const MARGIN: f32 = 0.03;

impl Showcase {
    /// Model ve düzen sekmeleri, harita alanının altında. Etkin sekme
    /// üstündeki alana bağlanır: model alanında harita zemininin, düzende
    /// masanın rengini alır.
    pub(super) fn sheet_tabs(&self) -> Element<'_, Message> {
        let backdrop = self.backdrop;
        let model = self.sheets.current() == 0;

        let tabs = std::iter::once(Tab::new("Model").closable(false)).chain(
            self.sheets
                .iter()
                .map(|sheet| Tab::new(sheet.name.as_str())),
        );

        Tabs::new(tabs, self.sheets.current(), Message::SheetSelected)
            .on_close(Message::SheetClosed)
            .on_reorder(Message::SheetMoved)
            .on_new(Message::SheetAdded)
            .bottom()
            .content(move |theme| {
                if model {
                    Style::with(backdrop, theme).background
                } else {
                    Tokens::of(theme).surface
                }
            })
            .into()
    }

    /// Düzen: masanın ortasında, alana sığan A3 kâğıt. Kâğıt her temada
    /// beyazdır; üzerindeki yazılar aydınlık temayla çizilir. Harita
    /// çerçevesinde gezinilir, seçim ve çizim model alanındadır.
    pub(super) fn sheet_view<'a>(&'a self, sheet: &'a Sheet) -> Element<'a, Message> {
        let paper_theme = theme::theme(Mode::Light, self.accent);

        responsive(move |size| {
            let room = Size::new(
                (size.width - DESK * 2.0).max(PAPER.width / 2.0),
                (size.height - DESK * 2.0).max(PAPER.height / 2.0),
            );
            let scale = (room.width / PAPER.width).min(room.height / PAPER.height);

            let frame = container(
                ModelSpace::new(sheet.viewport, &self.layers, Message::SheetView)
                    .backdrop(Backdrop::Paper)
                    .tool(Tool::Pan),
            )
            .padding(1)
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| container::Style {
                border: Border {
                    color: Tokens::of(theme).text,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            });

            let width = (PAPER.width * scale).floor();
            let margin = (width * MARGIN).clamp(8.0, 20.0).round();

            let paper = container(column![frame, self.title_block(sheet)].spacing(margin / 2.0))
                .padding(margin)
                .width(width)
                .height((PAPER.height * scale).floor())
                .style(|theme: &Theme| {
                    let t = Tokens::of(theme);

                    container::Style {
                        background: Some(t.field.into()),
                        shadow: Shadow {
                            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35),
                            offset: Vector::new(0.0, 3.0),
                            blur_radius: 14.0,
                        },
                        ..container::Style::default()
                    }
                });

            container(
                themer(Some(paper_theme.clone()), paper).text_color(|theme| Tokens::of(theme).text),
            )
            .center(Fill)
            .style(kentos_rc::style::container::surface)
            .into()
        })
        .into()
    }

    /// Paftanın antet kutusu: çizim, pafta, kâğıt ve program; tek satır.
    fn title_block<'a>(&'a self, sheet: &'a Sheet) -> Element<'a, Message> {
        let cell = |content: String| {
            container(
                label::caption(content)
                    .style(kentos_rc::style::text::default)
                    .wrapping(Wrapping::None),
            )
            .padding([3, 8])
            .clip(true)
        };

        // Antedin çizgileri çerçeveyle aynı, yazı renginde.
        let line = || {
            container(space::horizontal())
                .width(1)
                .height(Fill)
                .style(|theme: &Theme| container::Style {
                    background: Some(Tokens::of(theme).text.into()),
                    ..container::Style::default()
                })
        };

        container(
            row![
                cell("Türkiye örnek verisi".to_owned()).width(Fill),
                line(),
                cell(sheet.name.clone()),
                line(),
                cell("A3 yatay".to_owned()),
                line(),
                container(
                    label::caption("KentOS CAD")
                        .font(typography::ui_strong())
                        .style(kentos_rc::style::text::default)
                )
                .padding([3, 8]),
            ]
            .height(iced::Shrink),
        )
        .width(Fill)
        .style(|theme: &Theme| container::Style {
            border: Border {
                color: Tokens::of(theme).text,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
    }
}

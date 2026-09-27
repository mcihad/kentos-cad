//! How Lejant looks (the web's `LegendDialog` and its `leg__*` rules): the
//! two choices at the top; the layers, each with its check box, name and
//! count, and its rows with their pictures (only the lines in view are
//! built); the footer with the count or the last word, Kapat and “PNG
//! olarak kaydet”. The window has the web's size (760 × 760 at most, 88 % of
//! the window's height).

use iced::widget::{button, column, container, responsive, row, space, stack};
use iced::{Center, Element, Fill, Length, Theme};
use kentos_native_style::legend::texts;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog, VirtualList, horizontal_divider, overlay};

use super::{Event, Line, ev, lines};
use crate::app::{App, Message};
use crate::exchange::words::check;
use crate::style::thumbs::Look;

const WIDTH: f32 = 760.0;
const HEIGHT: f32 = 760.0;
/// Room kept on the right for the list's scroll bar.
const SCROLLBAR: f32 = 14.0;
/// A line of the list: a layer's heading or a row with its 56 × 32 picture.
const LINE: f32 = 40.0;
const PICTURE: (f32, f32) = (56.0, 32.0);

impl App {
    /// Lejant over the drawing.
    pub(crate) fn legend_view(&self) -> Element<'_, Message> {
        if self.styles.legend.is_none() {
            return space().into();
        }
        responsive(move |room| {
            let width = typography::from_default(WIDTH).min(room.width - 60.0);
            let height = typography::from_default(HEIGHT).min(room.height * 0.88);
            overlay::modal(self.legend_window(width, height), ev(Event::Close))
        })
        .into()
    }

    fn legend_window(&self, width: f32, height: f32) -> Element<'_, Message> {
        let (Some(window), Some(doc)) = (&self.styles.legend, &self.document) else {
            return space().into();
        };
        let lib = &self.styles.library;
        let groups = window.groups(&doc.model, lib);
        let options = row![
            check(
                window.visible_only,
                texts::VISIBLE_ONLY,
                Some(ev(Event::VisibleOnly(!window.visible_only)))
            ),
            check(
                window.headings,
                texts::HEADINGS,
                Some(ev(Event::Headings(!window.headings)))
            ),
        ]
        .spacing(18)
        .align_y(Center);
        let list: Element<'_, Message> = if groups.is_empty() {
            container(label::body(texts::NOTHING).style(style::text::muted))
                .padding([18, 0])
                .into()
        } else {
            let all = lines(&groups);
            let palette = self.style_palette();
            let images = self.styles.images.clone();
            let thumbs = &self.styles.thumbs;
            let left = window.left.clone();
            let groups = groups.clone();
            let line_h = typography::from_default(LINE).max(PICTURE.1 + 8.0);
            VirtualList::new(all.len(), line_h, move |i| {
                let look = Look {
                    palette: &palette,
                    library: lib,
                    images: &images,
                };
                match all[i] {
                    Line::Head(g) => {
                        let group = &groups[g];
                        let on = !left.contains(&group.layer_id);
                        let toggle = ev(Event::Layer(group.layer_id.clone(), !on));
                        let face = row![
                            check_box(
                                if on { Check::Checked } else { Check::Unchecked },
                                Some(toggle.clone())
                            ),
                            label::strong(group.layer_name.clone()),
                            space::horizontal(),
                            label::mono_caption(group.entries.len().to_string())
                                .style(style::text::muted),
                        ]
                        .spacing(8)
                        .align_y(Center);
                        let head = button(container(face).height(Fill).center_y(Fill))
                            .on_press(toggle)
                            .padding(iced::Padding {
                                left: 4.0,
                                right: SCROLLBAR,
                                ..iced::Padding::ZERO
                            })
                            .height(Fill)
                            .style(style::button::ghost);
                        // A hairline above every layer but the first (the web's group border).
                        let line: Element<'_, Message> = if g == 0 {
                            space().height(1).into()
                        } else {
                            horizontal_divider().into()
                        };
                        column![line, head].height(Fill).into()
                    }
                    Line::Entry(g, e) => {
                        let group = &groups[g];
                        let entry = &group.entries[e];
                        let picture: Element<'_, Message> = match &entry.symbol {
                            Some(symbol) => thumbs.picture(symbol, None, PICTURE, None, &look),
                            None => space().width(PICTURE.0).height(PICTURE.1).into(),
                        };
                        let framed = container(picture)
                            .padding(1)
                            .style(style::container::bordered);
                        let entry_row = row![framed, label::body(entry.label.clone())]
                            .spacing(12)
                            .align_y(Center);
                        let body = container(entry_row)
                            .padding(iced::Padding {
                                left: 30.0,
                                ..iced::Padding::ZERO
                            })
                            .height(Fill)
                            .center_y(Fill);
                        if left.contains(&group.layer_id) {
                            // Left out: the rows fade, as the web's (opacity 0.35).
                            stack![
                                body,
                                container(space())
                                    .width(Fill)
                                    .height(Fill)
                                    .style(|t: &Theme| container::Style {
                                        background: Some(
                                            Tokens::of(t).popover.scale_alpha(0.65).into()
                                        ),
                                        ..container::Style::default()
                                    })
                            ]
                            .into()
                        } else {
                            body.into()
                        }
                    }
                }
            })
            .height(Fill)
            .into()
        };
        let status: Element<'_, Message> = match window.said() {
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
            None => label::body(texts::rows(window.rows(&groups)))
                .style(style::text::muted)
                .into(),
        };
        let save = button(
            row![
                icon(crate::icons::from_web(Some("export")))
                    .size(14.0)
                    .tone(Tone::OnAccent),
                label::body(texts::SAVE).style(style::text::on_accent)
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([5, 14])
        .style(style::button::primary)
        .on_press_maybe((!window.saving && !groups.is_empty()).then(|| ev(Event::Save)));
        let footer = row![
            container(status).width(Fill),
            button(label::body(texts::CLOSE))
                .padding([5, 14])
                .style(style::button::secondary)
                .on_press(ev(Event::Close)),
            save,
        ]
        .spacing(8)
        .align_y(Center);
        let body = column![options, container(list).height(Fill)]
            .spacing(12)
            .height(Fill);
        Dialog::new(texts::TITLE)
            .push(container(body).height(Length::Fill))
            .push(footer)
            .width(typography::unscaled(width))
            .max_height(typography::unscaled(height))
            .into()
    }
}

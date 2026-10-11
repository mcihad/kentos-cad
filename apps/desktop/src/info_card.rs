//! The Bilgi tool's card (docs/adr/0225 §5; the web's `ui/shell/InfoCard.ts`):
//! the object `App::info_card` names, beside the drawing's point it was
//! clicked at and following the view, never outside the drawing. Its layer's
//! card's title, rows and text; the object's photos (a click opens the
//! viewer) and other attachments; its layer's actions as buttons. A broken
//! template's error is said under the title. × closes it, as Esc and a click
//! on nothing do. The card's body is the Bilgi kartı window's preview too.

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, button, column, container, row};
use iced::{Center, Element, Fill, Length, Padding, Point, Vector};
use kentos_domain::Slot;
use kentos_expression::template::TargetKind;
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::{Pairs, beside};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::attachments::{self, AttachmentView};
use crate::icons::from_web;
use crate::object_info::{ObjectAction, ObjectCard};
use crate::photo_view::{self, Fit};

/// The card's width, as the web's.
const WIDTH: f32 = 300.0;
const GAP: Vector = Vector::new(14.0, 14.0);

#[derive(Debug, Clone)]
pub enum Event {
    Close,
    /// One of the object's attachments, opened (a photo in the viewer).
    Attachment(Slot, usize),
    /// One of its layer's actions, by its place in the list.
    Action(Slot, usize),
    /// An action's target to the system's clipboard (Öznitelikler's Kopyala).
    Copy(String),
}

fn msg(e: Event) -> Message {
    Message::InfoCard(e)
}

/// The card's body: the title (with the close button when `close` is given),
/// the error, the rows, the text, the attachments of `slot` and the actions.
pub(crate) fn card_body<'a>(
    app: &'a App,
    card: &ObjectCard,
    shown: Option<(Slot, Vec<AttachmentView>)>,
    actions: &[ObjectAction],
    attachment_count: usize,
) -> Element<'a, Message> {
    let size = typography::caption();
    let shown_preview = shown.is_none();
    let mut head = Row::new().spacing(8).push(
        container(label::strong(card.title.clone()).wrapping(Wrapping::WordOrGlyph)).width(Fill),
    );
    if shown.is_some() {
        head = head.push(
            button(icon(from_web(Some("close"))).size(14.0))
                .on_press(msg(Event::Close))
                .padding(2)
                .style(style::button::ghost),
        );
    }
    let mut body = Column::new().spacing(8).push(head);
    if let Some(error) = &card.error {
        body = body.push(label::caption(error.clone()).style(style::text::warning));
    }
    if !card.rows.is_empty() {
        let rows = card.rows.iter().fold(
            Pairs::new().spacing_x(10.0).spacing_y(2.0),
            |rows, (k, v)| {
                rows.push(
                    label::caption(k.clone()).wrapping(Wrapping::None),
                    label::body(if v.is_empty() {
                        "—".to_owned()
                    } else {
                        v.clone()
                    })
                    .size(size)
                    .wrapping(Wrapping::WordOrGlyph),
                )
            },
        );
        body = body.push(rows);
    }
    if let Some(text) = &card.text {
        body = body.push(
            label::body(text.clone())
                .size(size)
                .wrapping(Wrapping::WordOrGlyph),
        );
    }
    match shown {
        Some((slot, views)) if card.attachments => {
            let (photos, others): (Vec<_>, Vec<_>) =
                views.into_iter().partition(|v| v.key.is_some());
            if !photos.is_empty() {
                let mut strip = Row::new().spacing(6);
                for v in photos {
                    strip = strip.push(
                        button(photo_view::photo(
                            app.photo_picture(v.key.as_deref()),
                            84.0,
                            63.0,
                            Fit::Cover,
                        ))
                        .on_press(msg(Event::Attachment(slot, v.index)))
                        .padding(0)
                        .style(style::button::ghost),
                    );
                }
                body = body.push(
                    iced::widget::scrollable(strip).direction(
                        iced::widget::scrollable::Direction::Horizontal(
                            iced::widget::scrollable::Scrollbar::new()
                                .width(4)
                                .scroller_width(4),
                        ),
                    ),
                );
            }
            for v in others {
                body = body.push(
                    button(
                        row![
                            icon(from_web(Some(attachments::kind_icon(v.kind)))).size(14.0),
                            label::body(v.attachment.name.clone()).size(size),
                        ]
                        .spacing(6)
                        .align_y(Center),
                    )
                    .on_press(msg(Event::Attachment(slot, v.index)))
                    .padding([2, 6])
                    .style(style::button::ghost),
                );
            }
        }
        None if card.attachments && attachment_count > 0 => {
            body = body.push(label::caption(format!("{attachment_count} ek")));
        }
        _ => {}
    }
    // The window's preview: each action's name and its target filled for the object, as text.
    if card.actions && !actions.is_empty() && shown_preview {
        for a in actions {
            let target = a.error.clone().or(a.target.clone()).unwrap_or_default();
            body = body.push(
                iced::widget::rich_text![
                    iced::widget::span(format!("{}: ", a.name)).font(typography::ui_strong()),
                    iced::widget::span(target).font(typography::ui()),
                ]
                .size(size)
                .wrapping(Wrapping::WordOrGlyph)
                .on_link_click(|_: ()| msg(Event::Close)),
            );
        }
        return body.into();
    }
    if card.actions && !actions.is_empty() {
        let slot = app.info_card.map(|(s, _)| s);
        let mut buttons = Row::new().spacing(6);
        for (i, a) in actions.iter().enumerate() {
            let glyph = if a.kind == TargetKind::File {
                "document"
            } else {
                "externalOpen"
            };
            let on = match (slot, a.error.is_none()) {
                (Some(slot), true) => Some(msg(Event::Action(slot, i))),
                _ => None,
            };
            let face = row![
                icon(from_web(Some(glyph))).size(14.0),
                label::body(a.name.clone()).size(size)
            ]
            .spacing(6)
            .align_y(Center);
            buttons = buttons.push(
                button(face)
                    .on_press_maybe(on)
                    .padding([3, 8])
                    .style(style::button::secondary),
            );
        }
        body = body.push(
            iced::widget::scrollable(buttons).direction(
                iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .scroller_width(4),
                ),
            ),
        );
    }
    body.into()
}

impl App {
    /// The card over the drawing, when the Bilgi tool shows one.
    pub(crate) fn info_card_view(&self) -> Option<Element<'_, Message>> {
        let (slot, at) = self.info_card?;
        let card = self.slot_card(slot)?;
        let actions = if card.actions {
            self.slot_actions(slot)
        } else {
            Vec::new()
        };
        let views = self.attachment_views(slot);
        let count = views.len();
        let body = card_body(self, &card, Some((slot, views)), &actions, count);
        let [x, y] = self.viewport.camera.world_to_screen(at);
        let framed = container(column![body])
            .width(Length::Fixed(typography::from_default(WIDTH)))
            .padding(Padding::from([10, 12]))
            .style(style::container::popover);
        Some(beside(
            framed,
            Point::new(x.round() as f32, y.round() as f32),
            GAP,
        ))
    }

    pub(crate) fn info_card_event(&mut self, e: Event) -> iced::Task<Message> {
        match e {
            Event::Close => self.info_card = None,
            Event::Attachment(slot, index) => {
                return self.attachment_event(attachments::Event::Open(slot, index));
            }
            Event::Action(slot, index) => {
                if let Some(a) = self.slot_actions(slot).get(index).cloned() {
                    self.run_action(&a);
                }
            }
            Event::Copy(text) => {
                self.say(
                    kentos_interaction::Level::Info,
                    "Eylemin hedefi panoya kopyalandı.",
                );
                return iced::clipboard::write(text);
            }
        }
        iced::Task::none()
    }

    /// After every message: the card goes with the Bilgi tool and with its object.
    pub(crate) fn follow_info_card(&mut self) {
        let Some((slot, _)) = self.info_card else {
            return;
        };
        let gone = self
            .document
            .as_ref()
            .is_none_or(|d| d.model.get(slot).is_none());
        if gone || self.session.tool_id() != kentos_interaction::identify::ID {
            self.info_card = None;
        }
    }
}

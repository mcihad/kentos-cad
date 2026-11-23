//! The Bilgi tool's card (docs/adr/0225 §5; the web's `ui/shell/InfoCard.ts`):
//! the object `App::info_card` names, beside the drawing's point it was
//! clicked at and following the view, never outside the drawing. Its layer's
//! card's title, rows and text; the object's photos (a click opens the
//! viewer) and other attachments; its layer's actions as buttons. A broken
//! template's error is said under the title. × closes it, as Esc and a click
//! on nothing do. The card's body is the Bilgi kartı window's preview too.
//! A data layer's row (data_info.rs, docs/adr/9101 §6) has the same card:
//! its columns as read from the source, its layer's actions, no attachments.

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
    /// One of a data layer's row's actions, by its place in the list.
    RowAction(usize),
    /// An action's target to the system's clipboard (Öznitelikler's Kopyala).
    Copy(String),
    /// The raster's value under the click shown (true), or the object or the row again (docs/adr/9137).
    Raster(bool),
    /// Sonraki: the next of the objects and the data layers' rows under the click (docs/adr/9145, 9167).
    Next,
}

/// The words of the hits under a click: how many, and which is shown.
pub(crate) fn hits_words(shown: usize, count: usize) -> String {
    format!("Bu noktada {count} nesne · {shown}/{count}")
}

/// The words of the objects and the data layers' rows under a click (docs/adr/9167): how many of each, and which of
/// them all is shown.
pub(crate) fn place_words(shown: usize, objects: usize, rows: usize) -> String {
    if rows == 0 {
        return hits_words(shown, objects);
    }
    format!("Bu noktada {objects} nesne, {rows} veri satırı · {shown}/{}", objects + rows)
}

fn msg(e: Event) -> Message {
    Message::InfoCard(e)
}

/// What a card is of.
pub(crate) enum Shown {
    /// The Bilgi kartı window's preview: no close button, the actions as their targets.
    Preview,
    /// An object of the drawing, with its attachments.
    Object(Slot, Vec<AttachmentView>),
    /// A data layer's row: no attachments, its actions by their place.
    Row,
}

/// The card's body: the title (with the close button but in the preview),
/// the error, the rows, the text, the attachments of an object and the actions.
pub(crate) fn card_body<'a>(
    app: &'a App,
    card: &ObjectCard,
    shown: Shown,
    actions: &[ObjectAction],
    attachment_count: usize,
) -> Element<'a, Message> {
    let size = typography::caption();
    let shown_preview = matches!(shown, Shown::Preview);
    let row = matches!(shown, Shown::Row);
    let mut head = Row::new().spacing(8).push(
        container(label::strong(card.title.clone()).wrapping(Wrapping::WordOrGlyph)).width(Fill),
    );
    if !shown_preview {
        head = head.push(
            button(icon(from_web(Some("close"))).size(14.0))
                .on_press(msg(Event::Close))
                .padding(kentos_ui::theme::metrics::icon_small(14.0))
                .style(style::button::ghost),
        );
    }
    let mut body = Column::new().spacing(8).push(head);
    if let Some((line, waits)) = &card.source {
        let l = label::caption(line.clone()).wrapping(Wrapping::WordOrGlyph);
        body = body.push(if *waits {
            l.style(style::text::warning)
        } else {
            l.style(style::text::muted)
        });
    }
    if let Some(error) = &card.error {
        body = body.push(label::caption(error.clone()).style(style::text::warning));
    }
    if !card.rows.is_empty() {
        let rows = card.rows.iter().fold(
            Pairs::new().spacing_x(10.0).spacing_y(2.0),
            |rows, (k, v)| {
                // A value never written, muted as the table draws it (docs/adr/9188).
                let null = v == crate::features::texts::NULL;
                let value = label::body(if v.is_empty() {
                    "—".to_owned()
                } else {
                    v.clone()
                })
                .size(size)
                .wrapping(Wrapping::WordOrGlyph);
                let value = if null { value.style(style::text::muted) } else { value };
                rows.push(label::caption(k.clone()).wrapping(Wrapping::None), value)
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
        Shown::Object(slot, views) if card.attachments => {
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
        Shown::Preview if card.attachments && attachment_count > 0 => {
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
                (_, true) if row => Some(msg(Event::RowAction(i))),
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
                    .padding(kentos_ui::theme::metrics::action_small())
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
    /// The card over the drawing, when the Bilgi tool shows one: an object's, or a data layer's row's.
    pub(crate) fn info_card_view(&self) -> Option<Element<'_, Message>> {
        // The switch between the object (or the row) and the raster's value under the same click (docs/adr/9137).
        let switch = |words: String, to: bool| {
            button(label::caption(words))
                .on_press(msg(Event::Raster(to)))
                .padding(kentos_ui::theme::metrics::action_small())
                .style(style::button::secondary)
        };
        let raster = self.gis.data_info.showing_raster.then(|| self.raster_card()).flatten();
        let (body, at): (Element<'_, Message>, _) = match (raster, self.info_card) {
            (Some((card, at)), _) => {
                let mut body = column![card_body(self, &card, Shown::Row, &[], 0)].spacing(8);
                if self.info_card.is_some() || self.gis.data_info.card.is_some() {
                    body = body.push(switch("Nesneye dön".to_owned(), false));
                }
                (body.into(), at)
            }
            (None, Some((slot, at))) => {
                let card = self.slot_card(slot)?;
                let actions = if card.actions {
                    self.slot_actions(slot)
                } else {
                    Vec::new()
                };
                let views = self.attachment_views(slot);
                let count = views.len();
                let body = card_body(self, &card, Shown::Object(slot, views), &actions, count);
                // Overlapping objects and data rows under the click: how many, which, and the next (docs/adr/9145, 9167).
                (self.with_place(body), at)
            }
            (None, None) => {
                let (card, actions, at) = self.data_card()?;
                (self.with_place(card_body(self, &card, Shown::Row, &actions, 0)), at)
            }
        };
        // Beside an object or a row, a raster under the same click offers its value.
        let body: Element<'_, Message> = match self.gis.data_info.cells.first() {
            Some(cell) if !self.gis.data_info.showing_raster => {
                column![body, switch(format!("Raster değeri: {}", cell.layer), true)].spacing(8).into()
            }
            _ => body,
        };
        let [x, y] = self.viewport.camera.world_to_screen(at);
        let framed = container(column![body])
            .width(Length::Fixed(typography::from_default(WIDTH)))
            .padding(Padding::from([10, 12]))
            .style(style::container::popover);
        Some(beside(
            iced::widget::mouse_area(framed).interaction(iced::mouse::Interaction::Idle),
            Point::new(x.round() as f32, y.round() as f32),
            GAP,
        ))
    }

    /// Where the card is among the click's objects and the data layers' rows under it (docs/adr/9145, 9167): its place
    /// from 0 and how many there are; none when it is none of them.
    pub(crate) fn info_place(&self) -> Option<(usize, usize)> {
        let hits = &self.gis.data_info.hits;
        let rows = &self.gis.data_info.rows;
        let total = hits.len() + rows.len();
        match (self.info_card, &self.gis.data_info.card) {
            (Some((slot, _)), _) => hits.iter().position(|s| *s == slot).map(|i| (i, total)),
            (None, Some(f)) => {
                rows.iter().position(|r| r.layer == f.layer && r.key == f.key).map(|j| (hits.len() + j, total))
            }
            (None, None) => None,
        }
    }

    /// `body` with the card's place under it and Sonraki, when the click has more than it (docs/adr/9145, 9167).
    fn with_place<'a>(&self, body: Element<'a, Message>) -> Element<'a, Message> {
        let Some((i, total)) = self.info_place().filter(|(_, t)| *t > 1) else {
            return body;
        };
        let objects = self.gis.data_info.hits.len();
        column![
            body,
            row![
                container(label::caption(place_words(i + 1, objects, total - objects))).width(Fill),
                button(label::caption("Sonraki"))
                    .on_press(msg(Event::Next))
                    .padding(kentos_ui::theme::metrics::action_small())
                    .style(style::button::secondary),
            ]
            .spacing(8)
            .align_y(Center),
        ]
        .spacing(8)
        .into()
    }

    pub(crate) fn info_card_event(&mut self, e: Event) -> iced::Task<Message> {
        match e {
            Event::Raster(to) => self.gis.data_info.showing_raster = to && !self.gis.data_info.cells.is_empty(),
            Event::Next => {
                // The next of the objects still in the drawing and the rows, round to the first (docs/adr/9167).
                let Some((i, total)) = self.info_place() else {
                    return iced::Task::none();
                };
                let hits = self.gis.data_info.hits.clone();
                let rows = self.gis.data_info.rows.clone();
                let at = self.info_card.map(|(_, at)| at).or_else(|| self.gis.data_info.card.as_ref().map(|f| f.at));
                for k in 1..total {
                    let p = (i + k) % total;
                    if let Some(slot) = hits.get(p).copied() {
                        if self.document.as_ref().is_some_and(|d| d.model.get(slot).is_some())
                            && let Some(at) = at
                        {
                            self.info_card = Some((slot, at));
                            self.gis.data_info.card = None;
                            self.with_tool(|s, cx| s.info_shown(slot, cx));
                            break;
                        }
                    } else if let Some(row) = rows.get(p - hits.len()) {
                        let object = row.object.clone();
                        self.info_card = None;
                        self.gis.data_info.card = Some(row.clone());
                        self.with_tool(|s, cx| s.data_found(Some(&object), cx));
                        break;
                    }
                }
            }
            Event::Close => {
                let row = self.info_card.is_none() && self.gis.data_info.card.is_some();
                self.info_card = None;
                self.gis.data_info.card = None;
                self.gis.data_info.cells.clear();
                self.gis.data_info.showing_raster = false;
                self.gis.data_info.hits.clear();
                self.gis.data_info.rows.clear();
                if row {
                    self.with_tool(|s, cx| s.data_found(None, cx));
                }
            }
            Event::Attachment(slot, index) => {
                return self.attachment_event(attachments::Event::Open(slot, index));
            }
            Event::Action(slot, index) => {
                if let Some(a) = self.slot_actions(slot).get(index).cloned() {
                    self.run_action(&a);
                }
            }
            Event::RowAction(index) => {
                if let Some(a) = self.data_card().and_then(|(_, actions, _)| actions.into_iter().nth(index)) {
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

    /// After every message: the card goes with the Bilgi tool and with its object (a row's, with its layer).
    pub(crate) fn follow_info_card(&mut self) {
        // The raster's cells go with the Bilgi tool.
        if self.session.tool_id() != kentos_interaction::identify::ID {
            self.gis.data_info.cells.clear();
            self.gis.data_info.showing_raster = false;
        }
        if self.gis.data_info.card.is_some() {
            let gone = self.document.as_ref().is_none_or(|d| {
                self.gis.data_info
                    .card
                    .as_ref()
                    .is_none_or(|f| d.model.layers().get(&f.layer).is_none_or(|n| n.data.is_none()))
            });
            let identify = self.session.tool_id() == kentos_interaction::identify::ID;
            if gone || !identify {
                self.gis.data_info.card = None;
            }
            if gone && identify {
                self.with_tool(|s, cx| s.data_found(None, cx));
            }
        }
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

#[cfg(test)]
#[path = "info_card_tests.rs"]
mod info_card_tests;

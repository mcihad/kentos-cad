//! The bonds in the way of a refused write (docs/adr/0790 §2): a grip's drag, Duvarı kaydır, Duvarın mesafesi, Boy or
//! an opening's move the architectural bonds refused leaves a card over the drawing with the refusal, each bond (the
//! values it keeps and the write would make) and the way through: “Bağı kaldır: d1” when one bond alone lets the write
//! through, “Bu iki bağı kaldır” (or üç) when the smallest set does together. The button writes one
//! `cad.constraints.edit` `remove` (one undo step); the bonds stay marked in place while the card is open. A write that
//! goes through, or Kapat, takes it away.

use iced::widget::{Column, Row, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_interaction::{bond_refusal, wall_distance};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::{label, style};

use crate::app::{App, Message};

#[derive(Clone, Debug)]
pub enum Event {
    /// Takes the bonds of the way through away.
    Remove,
    Close,
}

fn msg(e: Event) -> Message {
    Message::BondNotice(e)
}

impl App {
    pub(crate) fn bond_notice_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Close => self.bonds_refused = None,
            Event::Remove => {
                let Some(ids) = self
                    .bonds_refused
                    .as_ref()
                    .and_then(|e| bond_refusal::way_through(&e.bonds))
                    .map(|(ids, _)| ids)
                else {
                    return Task::none();
                };
                if self
                    .with_tool(|_, cx| bond_refusal::remove(ids, cx))
                    .unwrap_or(false)
                {
                    self.bonds_refused = None;
                }
            }
        }
        Task::none()
    }

    /// The bonds of the refusal marked where they stand while its card is open.
    pub(crate) fn bonds_refused_preview(&self) -> Option<kentos_interaction::Preview> {
        let error = self.bonds_refused.as_ref()?;
        let doc = self.document.as_ref()?;
        Some(wall_distance::bonds_in_way(&error.bonds, &doc.model))
    }

    /// What the drawing shows over its objects: the session's preview with the refusal's bonds marked while its card
    /// is open (unless the preview marks bonds of its own), else Mimari denetim's.
    pub(crate) fn drawing_preview(
        &self,
        format: &kentos_interaction::Format,
    ) -> Option<kentos_interaction::Preview> {
        let marks = self.bonds_refused_preview();
        match (self.session.preview(format), marks) {
            (Some(mut p), Some(m)) if !p.labels.iter().any(|l| l.text.starts_with('×')) => {
                p.strokes.extend(m.strokes);
                p.markers.extend(m.markers);
                p.labels.extend(m.labels);
                Some(p)
            }
            (Some(p), _) => Some(p),
            (None, Some(m)) => Some(m),
            (None, None) => self.arch_audit_preview(),
        }
    }

    /// The card over the drawing, while a refusal's bonds are told.
    pub(crate) fn bond_notice_view(&self) -> Option<Element<'_, Message>> {
        let error = self.bonds_refused.as_ref()?;
        let lines = wall_distance::refusal_lines(error, &self.format());
        let mut words = Column::new().spacing(2);
        for (i, line) in lines.into_iter().enumerate() {
            words = words.push(if i == 0 {
                label::strong(line)
            } else {
                label::caption(line)
            });
        }
        let mut actions = Row::new().spacing(6);
        if let Some((_, said)) = bond_refusal::way_through(&error.bonds) {
            actions = actions.push(
                button(label::body(said))
                    .on_press(msg(Event::Remove))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::primary),
            );
        }
        actions = actions.push(
            button(icon(Icon::Close).size(14.0))
                .on_press(msg(Event::Close))
                .padding(kentos_ui::theme::metrics::icon_small(14.0))
                .style(style::button::ghost),
        );
        let card = container(
            row![
                icon(Icon::Warning).size(18.0).tone(Tone::Danger),
                container(words).width(Fill),
                actions
            ]
            .spacing(12)
            .align_y(Center),
        )
        .padding([10, 14])
        .max_width(820.0)
        .style(style::container::popover);
        Some(container(card).padding(12).center_x(Fill).into())
    }
}

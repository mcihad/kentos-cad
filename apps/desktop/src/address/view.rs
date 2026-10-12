//! Adres's view (docs/adr/0229 §13): the bar (Kaynak, the address box with
//! Ara, Konumdan adres; Git, Nokta ekle and Taşı for the chosen row), the
//! candidates' table and the status line; with no source to ask, the two
//! windows that make one.

use iced::widget::{Column, button, container, row};
use iced::{Background, Center, Element, Fill, Length};
use kentos_processing::address::{LOCATOR, PROVIDER};
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::search_box::SearchBox;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, Table};
use kentos_ui::widget::{Elided, Tip, horizontal_divider, tip};

use super::{ADDRESS_FIELD, COLUMNS, Event, Question, texts};
use crate::app::{App, Message};
use crate::exchange::words;
use crate::icons::from_web;

fn msg(e: Event) -> Message {
    Message::Address(e)
}

/// A bar button: its icon and words, its hint above it.
fn bar_button<'a>(
    glyph: &str,
    words: &'a str,
    hint: &'a str,
    on: Option<Message>,
) -> Element<'a, Message> {
    tip(
        button(
            row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                .spacing(6)
                .align_y(Center),
        )
        .style(style::button::secondary)
        .padding([4, 10])
        .on_press_maybe(on),
        Tip::new(words).body(hint),
        iced::widget::tooltip::Position::Top,
    )
}

/// A table cell: one line, cut with an ellipsis.
fn cell<'a>(words: String, strong: bool) -> Element<'a, Message> {
    Elided::new(words)
        .size(typography::body())
        .font(if strong {
            typography::ui_strong()
        } else {
            typography::ui()
        })
        .style(move |theme: &iced::Theme| iced::widget::text::Style {
            color: Some(Tokens::of(theme).text),
        })
        .width(Fill)
        .into()
}

impl App {
    /// The Adres tab: its bar over the table, the status line under it.
    pub(crate) fn address_tab(&self) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::muted("Açık çizim yok."))
                .padding(12)
                .into();
        };
        let format = kentos_interaction::Format::of(doc.settings());
        let a = &self.address;
        let sources = self.address_sources();
        let chosen = self.address_source();

        // Kaynak: the project's address data, then the providers, those that cannot be asked dimmed with why.
        let mut choices: Vec<Choice> = Vec::new();
        let mut ids: Vec<Option<String>> = Vec::new();
        let locals: Vec<_> = sources
            .iter()
            .filter(|s| s.id.starts_with(LOCATOR))
            .collect();
        let onlines: Vec<_> = sources
            .iter()
            .filter(|s| s.id.starts_with(PROVIDER))
            .collect();
        if !locals.is_empty() {
            choices.push(Choice::header("Projenin adres verisi"));
            ids.push(None);
            for s in &locals {
                let short = s.name.split_once(": ").map_or(s.name.as_str(), |(_, n)| n);
                choices.push(Choice::new(short).icon(from_web(Some("addressData"))));
                ids.push(Some(s.id.clone()));
            }
        }
        choices.push(Choice::header("Çevrimiçi sağlayıcılar"));
        ids.push(None);
        for s in &onlines {
            let mut c = Choice::new(s.name.clone()).icon(from_web(Some("addressProviders")));
            if let Some(why) = &s.why {
                c = c.disabled().detail(why.clone());
            }
            choices.push(c);
            ids.push(Some(s.id.clone()));
        }
        let selected = chosen
            .as_ref()
            .and_then(|c| ids.iter().position(|i| i.as_deref() == Some(c.id.as_str())));
        let pick_ids = ids.clone();
        let now = chosen.as_ref().map(|c| c.id.clone()).unwrap_or_default();
        // A header is not chosen: the source stays.
        let source = Select::new(choices, selected, move |i| {
            msg(Event::Source(
                pick_ids
                    .get(i)
                    .cloned()
                    .flatten()
                    .unwrap_or_else(|| now.clone()),
            ))
        })
        .placeholder("Kaynak yok")
        .action(
            from_web(Some("addressData")),
            texts::DATA,
            msg(Event::OpenData),
        )
        .action(
            from_web(Some("addressProviders")),
            texts::PROVIDERS,
            msg(Event::OpenProviders),
        );

        let words = SearchBox::new(a.text.clone(), texts::PLACEHOLDER, |t| msg(Event::Text(t)))
            .id(iced::widget::Id::new(ADDRESS_FIELD))
            .on_enter(msg(Event::Find))
            .fill()
            .height(28.0);
        let asked_address = matches!(a.question, Some(Question::Address(_)))
            && a.answerer.as_ref().is_none_or(|w| w.keeps_none.is_none());
        let some_row = a.current.is_some();
        let one_point = matches!(self.selection.ids(), [slot]
            if matches!(doc.model.get(*slot), Some(kentos_contracts::Entity::Point(_))));
        // The question: where, what, Ara; and Konumdan adres.
        let bar = row![
            label::caption(texts::SOURCE),
            container(source).width(Length::Fixed(typography::scaled(200.0))),
            container(tip(
                words,
                Tip::new(texts::SEARCH).body(texts::SEARCH_HINT),
                iced::widget::tooltip::Position::Top,
            ))
            .width(Fill),
            button(label::body(texts::FIND))
                .style(style::button::primary)
                .padding([4, 14])
                .on_press_maybe(chosen.as_ref().map(|_| msg(Event::Find))),
            bar_button(
                "addressReverse",
                texts::REVERSE,
                texts::REVERSE_HINT,
                chosen.as_ref().map(|_| msg(Event::Reverse)),
            ),
        ]
        .spacing(8)
        .align_y(Center);
        let bar = container(bar).padding([6, 10]).width(Fill);
        // The chosen row's actions, beside the status line.
        let actions = row![
            bar_button(
                "target",
                texts::GO,
                texts::GO_HINT,
                some_row.then(|| msg(Event::Go))
            ),
            bar_button(
                "point",
                texts::ADD,
                texts::ADD_HINT,
                (some_row && asked_address).then(|| msg(Event::AddPoint)),
            ),
            bar_button(
                "move",
                texts::MOVE,
                texts::MOVE_HINT,
                (some_row && asked_address && one_point).then(|| msg(Event::MovePoint)),
            ),
        ]
        .spacing(8)
        .align_y(Center);

        // No source to ask: the two windows that make one.
        let banner = chosen.is_none().then(|| {
            container(
                row![
                    label::body(texts::NO_SOURCE).width(Fill),
                    bar_button(
                        "addressData",
                        texts::DATA,
                        texts::DATA,
                        Some(msg(Event::OpenData))
                    ),
                    bar_button(
                        "addressProviders",
                        texts::PROVIDERS,
                        texts::PROVIDERS,
                        Some(msg(Event::OpenProviders)),
                    ),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .padding([6, 10])
            .width(Fill)
            .style(|theme: &iced::Theme| container::Style {
                background: Some(Background::Color(
                    Tokens::of(theme).accent.scale_alpha(0.12),
                )),
                ..container::Style::default()
            })
        });

        // A place's address has a distance where an address looked for has a score.
        let place = matches!(a.question, Some(Question::Place(_)));
        let columns = COLUMNS.iter().enumerate().map(|(i, title)| {
            let title = if i == 3 && place {
                texts::DISTANCE
            } else {
                *title
            };
            let width = match i {
                0 => Length::Fixed(typography::scaled(52.0)),
                1 => Length::FillPortion(10),
                2 => Length::FillPortion(3),
                3 => Length::Fixed(typography::scaled(72.0)),
                4 => Length::FillPortion(4),
                5 => Length::FillPortion(3),
                _ => Length::FillPortion(4),
            };
            let column = TableColumn::new(title).width(width);
            if i == 0 || i == 3 {
                column.align_right()
            } else {
                column
            }
        });
        let rows: Vec<[String; 6]> = a
            .rows
            .iter()
            .map(|m| self.address_cells(m, &format))
            .collect();
        let current = a.current;
        let count = rows.len();
        let table = Table::new(columns)
            .virtualized(count, move |i| {
                let c = &rows[i];
                TableLine::new([
                    label::muted((i + 1).to_string()).into(),
                    cell(c[0].clone(), i == 0),
                    cell(c[1].clone(), false),
                    label::body(c[2].clone()).into(),
                    cell(c[3].clone(), false),
                    cell(c[4].clone(), false),
                    cell(c[5].clone(), false),
                ])
                .selected(current == Some(i))
                .current(current == Some(i))
                .on_press(msg(Event::Press(i)))
            })
            .reveal(current)
            .empty(texts::EMPTY.to_owned());

        let status: Element<'_, Message> = match &a.said {
            Some((kind, words)) => words::text_line(*kind, words.clone()),
            None => label::muted(self.address_status_line()).into(),
        };
        let mut out = Column::new().push(bar);
        if let Some(banner) = banner {
            out = out.push(banner);
        }
        out.push(horizontal_divider())
            .push(table)
            .push(horizontal_divider())
            .push(
                container(
                    row![container(status).width(Fill), actions]
                        .spacing(10)
                        .align_y(Center),
                )
                .padding([4, 10])
                .width(Fill),
            )
            .width(Fill)
            .height(Fill)
            .into()
    }
}

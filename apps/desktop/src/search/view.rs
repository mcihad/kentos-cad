//! Arama's view (docs/adr/0178; the web's `SearchPanel`): the bar (the
//! search box and Katman, the count with Hepsini seç and Göster; the fields
//! asked for with Öznitelik's list, and the three options), Koordinata
//! git's bar (the place typed with Git, the place marked with İşareti
//! kaldır) and the table of what was found.

use iced::widget::{Column, button, container, row, space};
use iced::{Background, Center, Element, Fill, Length};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::search_box::SearchBox;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, SortOrder, Table};
use kentos_ui::widget::{Elided, Tip, horizontal_divider, tip};

use super::{COLUMNS, Event, Field, Results, SEARCH_FIELD, count_text, field_cell, texts};
use crate::app::{App, Message};
use crate::exchange::words;
use crate::icons::from_web;

fn msg(e: Event) -> Message {
    Message::Search(e)
}

/// What the bar's second line needs in one row, logical pixels at the
/// default type size: Ara:, its four buttons and Öznitelik's list, then the
/// three options.
const FIELDS_ONE_ROW: f32 = 1080.0;

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

/// A field button of the second line: pressed while the field is asked for.
fn chip<'a>(on: bool, field: Field, words: &'a str, hint: &'a str) -> Element<'a, Message> {
    tip(
        button(label::body(words))
            .padding([3, 10])
            .style(style::button::chip(on))
            .on_press(msg(Event::Field(field))),
        Tip::new(words).body(hint),
        iced::widget::tooltip::Position::Top,
    )
}

/// A table cell: one line, cut with an ellipsis, in `tone`.
fn cell<'a>(words: String, quiet: bool, strong: bool) -> Element<'a, Message> {
    Elided::new(words)
        .size(typography::body())
        .font(if strong {
            typography::ui_strong()
        } else {
            typography::ui()
        })
        .style(move |theme: &iced::Theme| {
            let t = Tokens::of(theme);
            iced::widget::text::Style {
                color: Some(if quiet { t.muted } else { t.text }),
            }
        })
        .width(Fill)
        .into()
}

impl App {
    /// The Arama tab: its bar over the table.
    pub(crate) fn data_tab(&self) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::muted(texts::NO_DATA)).padding(12).into();
        };
        let rows = self.data_rows(doc);
        let format = kentos_interaction::Format::of(doc.settings());
        let panel = &self.search;
        let place = self.data_place();
        let mark = self.data_mark_label();

        let bar = self.data_bar(&rows);
        let banner = (place.is_some() || mark.is_some()).then(|| {
            let mut line = row![].spacing(28).align_y(Center);
            if let Some(p) = place {
                line = line.push(
                    row![
                        label::caption(texts::COORDINATE),
                        label::strong(format.point(p)),
                        bar_button("target", texts::GO, texts::GO_HINT, Some(msg(Event::Go))),
                    ]
                    .spacing(10)
                    .align_y(Center),
                );
            }
            if let Some((_, said)) = &mark {
                line = line.push(
                    row![
                        label::caption(texts::MARK),
                        label::strong(said.clone()),
                        bar_button(
                            "markClear",
                            texts::UNMARK,
                            texts::UNMARK_HINT,
                            Some(msg(Event::Unmark)),
                        ),
                    ]
                    .spacing(10)
                    .align_y(Center),
                );
            }
            container(line)
                .padding([4, 10])
                .width(Fill)
                .style(|theme: &iced::Theme| container::Style {
                    background: Some(Background::Color(
                        Tokens::of(theme).accent.scale_alpha(0.12),
                    )),
                    ..container::Style::default()
                })
        });

        let columns = COLUMNS.iter().enumerate().map(|(i, (title, key))| {
            let order = key.filter(|k| panel.sort == Some(k)).map(|_| {
                if panel.descending {
                    SortOrder::Descending
                } else {
                    SortOrder::Ascending
                }
            });
            let width = match i {
                0 => Length::Fixed(typography::scaled(56.0)),
                1 => Length::FillPortion(5),
                2 => Length::FillPortion(3),
                3 => Length::FillPortion(4),
                _ => Length::FillPortion(8),
            };
            let column = TableColumn::new(*title)
                .width(width)
                .sortable(order, msg(Event::Sort(i)));
            if i == 0 { column.align_right() } else { column }
        });
        let model = &doc.model;
        let selection = &self.selection;
        let found = rows.found.rows.clone();
        let slots = rows.slots.clone();
        let index = rows.index.clone();
        let subset = rows.subset.clone();
        let count = found.len();
        let table = Table::new(columns)
            .virtualized(count, move |i| {
                let row = &found[i];
                let at = subset[row.record as usize];
                let record = &index.records[at];
                let layer_id = index.layer_ids[at].as_str();
                let hidden = !model.layers().is_visible(layer_id);
                let locked = model.layers().is_locked(layer_id);
                let slot = slots[i];
                let mut layer = row![].spacing(4).align_y(Center).width(Fill);
                if hidden {
                    layer = layer.push(icon(from_web(Some("eyeOff"))).size(12.0).tone(Tone::Muted));
                } else if locked {
                    layer = layer.push(icon(Icon::Lock).size(12.0).tone(Tone::Muted));
                }
                layer = layer.push(cell(record.layer.clone(), hidden, false));
                TableLine::new([
                    label::muted((i + 1).to_string()).into(),
                    layer.into(),
                    cell(record.kind.clone(), hidden, false),
                    cell(field_cell(row), true, false),
                    cell(row.value.clone(), hidden, true),
                ])
                .selected(selection.contains(slot))
                // Every selected row alike, as the web shows them: no primary one.
                .current(false)
                .on_press(msg(Event::Press(i)))
            })
            .reveal(rows.first_selected)
            .empty(if !panel.text.trim().is_empty() && !rows.asks {
                texts::NO_FIELDS.to_owned()
            } else if panel.text.trim().is_empty() {
                format.axes_text(texts::NO_QUERY)
            } else if rows.index.records.is_empty() {
                texts::NO_DATA.to_owned()
            } else {
                texts::no_match(panel.text.trim())
            });
        let mut out = Column::new().push(bar);
        if let Some(banner) = banner {
            out = out.push(banner);
        }
        out.push(horizontal_divider())
            .push(table)
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// The bar: the words and the layer with the count and its buttons; the
    /// fields asked for and the options (a narrow panel takes the options to
    /// a third line rather than cut them, as the web's bar wraps).
    fn data_bar<'a>(&'a self, rows: &Results) -> Element<'a, Message> {
        let panel = &self.search;
        let some = !self.selection.is_empty();
        // The words: as wide as their place, the box's own width ran under Katman.
        // The coordinate order is the open drawing's type's (Y,X, X,Y in a CAD project).
        let format = self.format();
        let words = SearchBox::new(
            panel.text.clone(),
            format.axes_text(texts::PLACEHOLDER),
            |t| msg(Event::Text(t)),
        )
        .id(iced::widget::Id::new(SEARCH_FIELD))
        .on_enter(msg(Event::Submit))
        .fill()
        .height(28.0);
        let mut layers = vec![Choice::new(texts::ALL_LAYERS)];
        layers.extend(
            rows.layers
                .iter()
                .map(|(_, name, n)| Choice::new(format!("{name} ({n})"))),
        );
        let chosen = panel
            .layer
            .as_ref()
            .and_then(|l| rows.layers.iter().position(|(id, _, _)| id == l))
            .map_or(0, |i| i + 1);
        let ids: Vec<String> = rows.layers.iter().map(|(id, _, _)| id.clone()).collect();
        let layer = Select::new(layers, Some(chosen), move |i| {
            msg(Event::Layer(if i == 0 {
                None
            } else {
                ids.get(i - 1).cloned()
            }))
        });
        let count = if rows.found.total > 0 || !panel.text.trim().is_empty() {
            count_text(rows.found.rows.len(), rows.found.total as usize)
        } else {
            String::new()
        };
        let first = row![
            container(tip(
                words,
                Tip::new(texts::SEARCH).body(format.axes_text(texts::SEARCH_HINT)),
                iced::widget::tooltip::Position::Top,
            ))
            .width(Length::Fixed(typography::scaled(250.0))),
            container(layer).width(Length::Fixed(typography::scaled(180.0))),
            space::horizontal(),
            label::muted(count),
            bar_button(
                "selectAll",
                texts::SELECT_ALL,
                texts::SELECT_ALL_HINT,
                (rows.found.total > 0).then(|| msg(Event::SelectAll)),
            ),
            bar_button(
                "zoomSelection",
                texts::SHOW,
                texts::SHOW_HINT,
                some.then(|| msg(Event::Show)),
            ),
        ]
        .spacing(10)
        .align_y(Center);

        // Ara: the four fields, and which attribute.
        let f = &panel.fields;
        let mut names = vec![Choice::new(texts::ALL_ATTRS)];
        names.extend(rows.names.iter().map(Choice::new));
        let named = f
            .attr_name
            .as_ref()
            .and_then(|n| rows.names.iter().position(|x| x == n))
            .map_or(0, |i| i + 1);
        let list = rows.names.clone();
        let attr: Element<'_, Message> = if f.attrs {
            Select::new(names, Some(named), move |i| {
                msg(Event::AttrName(if i == 0 {
                    None
                } else {
                    list.get(i - 1).cloned()
                }))
            })
            .into()
        } else {
            container(
                row![
                    label::muted(
                        f.attr_name
                            .clone()
                            .unwrap_or_else(|| texts::ALL_ATTRS.to_owned())
                    ),
                    space::horizontal(),
                    icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
                ]
                .align_y(Center),
            )
            .padding([4, 10])
            .style(style::container::field_box)
            .into()
        };
        let fields = row![
            label::caption(texts::FIELDS),
            chip(f.label, Field::Label, texts::LABEL_FIELD, texts::LABEL_HINT),
            chip(f.text, Field::Text, texts::TEXT_FIELD, texts::TEXT_HINT),
            chip(f.block, Field::Block, texts::BLOCK_FIELD, texts::BLOCK_HINT),
            chip(f.attrs, Field::Attrs, texts::ATTRS_FIELD, texts::ATTRS_HINT),
            container(attr).width(Length::Fixed(typography::scaled(150.0))),
        ]
        .spacing(6)
        .align_y(Center);
        let nothing_selected = !some && !panel.only_selected;
        let options = row![
            words::check(
                panel.match_case,
                texts::MATCH_CASE,
                Some(msg(Event::MatchCase(!panel.match_case))),
            ),
            tip(
                words::check(
                    panel.whole_word,
                    texts::WHOLE_WORD,
                    Some(msg(Event::WholeWord(!panel.whole_word))),
                ),
                Tip::new(texts::WHOLE_WORD).body(texts::WHOLE_WORD_HINT),
                iced::widget::tooltip::Position::Top,
            ),
            words::check(
                panel.only_selected,
                texts::ONLY_SELECTED,
                (!nothing_selected).then(|| msg(Event::OnlySelected(!panel.only_selected))),
            ),
        ]
        .spacing(18)
        .align_y(Center);

        let one_row = self.viewport.bounds.width <= 0.0
            || self.viewport.bounds.width >= typography::from_default(FIELDS_ONE_ROW);
        let second: Element<'_, Message> = if one_row {
            row![fields, options].spacing(18).align_y(Center).into()
        } else {
            iced::widget::column![fields, options].spacing(6).into()
        };
        container(iced::widget::column![first, second].spacing(6))
            .padding([6, 10])
            .width(Fill)
            .into()
    }
}

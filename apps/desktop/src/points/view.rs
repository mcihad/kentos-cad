//! Noktalar's view (docs/adr/0153 §1–§4; the web's `PointTable`): the bar
//! (search, layer, only the selected, Bağlı çizgiler izler, the count, Satır
//! ekle, Sil, Göster) over the table; a value cell double-clicked holds the
//! editor's field (`cell.rs`), the draft row after the points.

use iced::widget::{button, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Length};
use kentos_contracts::Entity;
use kentos_interaction::Format;
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::search_box::SearchBox;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, SortOrder, Table};
use kentos_ui::widget::{Tip, tip};

use super::edit::{EditColumn, cell_text};
use super::{COLUMNS, Event, Rows, Target, Walk, cell, layer_name, texts};
use crate::app::{App, Message};
use crate::icons::from_web;

fn msg(e: Event) -> Message {
    Message::Points(e)
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

/// What the bar needs in one row, logical pixels at the default type size
/// (measured 1 037): the filters (search 200, layer 170, the two switches)
/// and the count with the three buttons, their gaps and margins.
const BAR_ONE_ROW: f32 = 1060.0;

impl App {
    /// The bar: the filters on the left, the count and the buttons on the
    /// right; a narrow panel takes the right group to a second line, to the
    /// right, rather than cut it (the web's bar wraps the same way). The
    /// panel is as wide as the drawing area above it.
    fn points_bar<'a>(
        search: String,
        rows: &Rows,
        layer: Option<String>,
        only_selected: bool,
        follow: bool,
        some: bool,
        width: f32,
    ) -> Element<'a, Message> {
        let names: Vec<String> = rows.layers.iter().map(|(name, _)| name.clone()).collect();
        let labels: Vec<String> = rows
            .layers
            .iter()
            .map(|(name, n)| format!("{name} ({n})"))
            .collect();
        let chosen = match &layer {
            None => 0,
            Some(l) => names.iter().position(|name| name == l).map_or(0, |i| i + 1),
        };
        let count = format!("{} / {} nokta", rows.shown.len(), rows.points.len());
        {
            let search = SearchBox::new(search.clone(), texts::SEARCH, |t| msg(Event::Search(t)))
                .height(28.0);
            let mut choices = vec![Choice::new(texts::ALL_LAYERS)];
            choices.extend(labels.iter().map(Choice::new));
            let names = names.clone();
            let layer = Select::new(choices, Some(chosen), move |i| {
                msg(Event::Layer(if i == 0 {
                    None
                } else {
                    names.get(i - 1).cloned()
                }))
            });
            let only = Switch::new(only_selected, |on| msg(Event::OnlySelected(on)))
                .label(texts::ONLY_SELECTED);
            let follow = tip(
                Switch::new(follow, |on| msg(Event::Follow(on))).label(texts::FOLLOW),
                Tip::new(texts::FOLLOW).body(
                    "Nokta taşınınca ya da kotu değişince, o yerde köşesi olan çizgi, çoklu çizgi ve alanların köşeleri de izler.",
                ),
                iced::widget::tooltip::Position::Top,
            );
            let filters = row![
                container(search).width(Length::Fixed(200.0)),
                container(layer).width(Length::Fixed(170.0)),
                only,
                follow,
            ]
            .spacing(10)
            .align_y(Center);
            let actions = row![
                label::muted(count.clone()),
                bar_button(
                    "plus",
                    texts::ADD,
                    "Tablonun sonunda yeni satır: Ad, Y, X, Z ve Kod yazılır, Enter etkin katmana yazar ve sonrakini açar.",
                    Some(msg(Event::AddRow)),
                ),
                bar_button(
                    "erase",
                    texts::REMOVE,
                    "Seçili noktaları siler",
                    some.then(|| msg(Event::Remove)),
                ),
                bar_button(
                    "zoomSelection",
                    texts::SHOW,
                    "Seçili noktalara yakınlaştırır",
                    some.then(|| msg(Event::Show)),
                ),
            ]
            .spacing(10)
            .align_y(Center);
            // Before the area reports its size, one row.
            let one_row = width <= 0.0 || width >= typography::from_default(BAR_ONE_ROW);
            let bar: Element<'_, Message> = if one_row {
                row![filters, space::horizontal(), actions]
                    .align_y(Center)
                    .into()
            } else {
                iced::widget::column![filters, row![space::horizontal(), actions]]
                    .spacing(6)
                    .into()
            };
            container(bar).padding([6, 10]).width(Fill).into()
        }
    }

    /// The Noktalar tab: its bar over the table.
    pub(crate) fn points_tab(&self) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::muted(texts::NONE)).padding(12).into();
        };
        let rows = self.point_rows(doc);
        let format = Format::of(doc.model.settings());
        let panel = &self.points;

        let bar = Self::points_bar(
            panel.search.clone(),
            &rows,
            panel.layer.clone(),
            panel.only_selected,
            panel.follow,
            !self.selection.is_empty(),
            self.viewport.bounds.width,
        );

        let columns = COLUMNS
            .iter()
            .enumerate()
            .map(|(i, (title, key, numeric, _))| {
                let order = key.filter(|k| panel.sort == Some(k)).map(|_| {
                    if panel.descending {
                        SortOrder::Descending
                    } else {
                        SortOrder::Ascending
                    }
                });
                let width = match i {
                    0 => Length::Fixed(56.0),
                    1 | 5 | 6 => Length::FillPortion(2),
                    _ => Length::FillPortion(3),
                };
                let column = TableColumn::new(*title)
                    .width(width)
                    .sortable(order, msg(Event::Sort(i)));
                if *numeric {
                    column.align_right()
                } else {
                    column
                }
            });
        let model = &doc.model;
        let selection = &self.selection;
        let shown = rows.shown.clone();
        let editing = panel.editing;
        let text = panel.text.clone();
        let draft = panel.draft.clone();
        let count = shown.len() + usize::from(draft.is_some());
        // The cell at row `i`, column `j`: the editor's field when it is edited, its value else.
        let cell =
            move |i: usize, j: usize, value: String, target: Target| -> Element<'_, Message> {
                let (_, _, numeric, col) = COLUMNS[j];
                if let (Some(col), Some((t, c))) = (col, editing)
                    && t == target
                    && c == col
                {
                    return cell::edit_box(
                        &text,
                        numeric,
                        |t| msg(Event::Input(t)),
                        msg(Event::Finish(Some(Walk::Down))),
                        msg(Event::Finish(None)),
                        msg(Event::Cancel),
                    );
                }
                // The draft's cells in the secondary colour, its number cell (Yeni) in the accent.
                let draft_row = target == Target::Draft;
                let shown: Element<'_, Message> = match (j, draft_row) {
                    (0, true) => label::strong(value).style(style::text::accent).into(),
                    (0, false) => label::muted(value).into(),
                    (_, true) if numeric => label::mono(value).style(style::text::muted).into(),
                    (_, true) => label::muted(value).into(),
                    (1, false) => label::strong(value).into(),
                    _ if numeric => label::mono(value).into(),
                    _ => label::body(value).into(),
                };
                if j == 0 && !draft_row {
                    return mouse_area(shown)
                        .on_double_click(msg(Event::Zoom(i)))
                        .into();
                }
                match col {
                    Some(_) => mouse_area(shown)
                        .on_double_click(msg(Event::Edit(i, j)))
                        .into(),
                    None => shown,
                }
            };
        let table = Table::new(columns)
            .virtualized(count, move |i| {
                let Some(&slot) = shown.get(i) else {
                    // The draft after the points.
                    let d = draft.clone().unwrap_or_default();
                    let values = [
                        texts::DRAFT.to_owned(),
                        d.name,
                        d.east,
                        d.north,
                        d.z,
                        d.code,
                        String::new(),
                    ];
                    return TableLine::new(
                        values
                            .into_iter()
                            .enumerate()
                            .map(|(j, v)| cell(i, j, v, Target::Draft)),
                    )
                    .selected(true)
                    .current(false);
                };
                let Some(Entity::Point(p)) = model.get(slot) else {
                    return TableLine::new([label::muted("").into()]);
                };
                let value = |col: EditColumn| match col {
                    EditColumn::East => format.coord(p.p.x),
                    EditColumn::North => format.coord(p.p.y),
                    EditColumn::Z => p.z.map(|z| format.length_bare(z)).unwrap_or_default(),
                    _ => cell_text(p, col),
                };
                let values = [
                    (i + 1).to_string(),
                    value(EditColumn::Name),
                    value(EditColumn::East),
                    value(EditColumn::North),
                    value(EditColumn::Z),
                    value(EditColumn::Code),
                    layer_name(model, &p.base.layer_id),
                ];
                TableLine::new(
                    values
                        .into_iter()
                        .enumerate()
                        .map(|(j, v)| cell(i, j, v, Target::Point(slot))),
                )
                .selected(selection.contains(slot))
                // Every selected row alike, as the web shows them: no primary one.
                .current(false)
                .on_press(msg(Event::Press(i)))
            })
            .reveal(match editing {
                Some((Target::Draft, _)) => Some(rows.shown.len()),
                Some((Target::Point(s), _)) => rows.shown.iter().position(|&x| x == s),
                None => rows.first_selected,
            })
            .empty(if rows.points.is_empty() {
                texts::NONE
            } else {
                texts::NO_MATCH
            });
        iced::widget::column![bar, kentos_ui::widget::horizontal_divider(), table]
            .width(Fill)
            .height(Fill)
            .into()
    }
}

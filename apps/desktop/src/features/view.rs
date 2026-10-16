//! Öznitelik tablosu's view (docs/adr/0199 §4; the web's `FeatureTable`):
//! the bar (Katman ▾, Ara, Göster ▾, İfade süzgeci and its ε, the count,
//! Seçime yakınlaş, Alanlar…) over the table; a value cell double-clicked
//! holds its editor (a list for a value list and yes or no, else Noktalar's
//! field), a refused value its reason on hover.

use iced::widget::text::Wrapping;
use iced::widget::{button, container, mouse_area, row, space, text_input};
use iced::{Center, Color, Element, Fill, Length, Theme};
use kentos_contracts::{LayerField, LayerFieldKind, check_value};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::search_box::SearchBox;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, SortOrder, Table};
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};

use super::{Event, Rows, Show, texts};
use crate::app::{App, Message};
use crate::icons::from_web;
use crate::points::cell;

fn msg(e: Event) -> Message {
    Message::Features(e)
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

/// What the bar needs in one row, logical pixels at the default type size:
/// the layer (220), the search (180), Göster (190), the filter (220 and its
/// ε), the count and the three buttons, their gaps and margins.
const BAR_ONE_ROW: f32 = 1370.0;

/// A list editor's choices for a field: none, then its codes by their labels,
/// or yes and no; with the one written now.
fn choices_of(f: &LayerField, value: &str) -> (Vec<Choice>, Vec<String>, Option<usize>) {
    let pairs: Vec<(String, String)> = match &f.values {
        Some(list) if !list.is_empty() => list
            .iter()
            .map(|c| (c.code.clone(), c.label.clone()))
            .collect(),
        _ => vec![
            ("true".to_owned(), "Evet".to_owned()),
            ("false".to_owned(), "Hayır".to_owned()),
        ],
    };
    let mut codes = vec![String::new()];
    let mut choices = vec![Choice::new(texts::EMPTY)];
    for (code, label) in pairs {
        codes.push(code);
        choices.push(Choice::new(label));
    }
    let now = check_value(f, value).unwrap_or_default();
    let at = codes.iter().position(|c| *c == now);
    (choices, codes, at)
}

impl App {
    fn features_bar<'a>(&self, rows: &Rows, width: f32) -> Element<'a, Message> {
        let panel = &self.features;
        let ids: Vec<String> = rows.layers.iter().map(|(id, _, _)| id.clone()).collect();
        let chosen = rows
            .layer
            .as_ref()
            .and_then(|l| ids.iter().position(|id| id == l));
        let layer = Select::new(
            rows.layers
                .iter()
                .map(|(_, path, n)| Choice::new(format!("{path} ({n})"))),
            chosen,
            move |i| msg(Event::Layer(ids[i].clone())),
        )
        .placeholder(texts::NO_LAYERS);
        let search = SearchBox::new(panel.search.clone(), texts::SEARCH, |t| {
            msg(Event::Search(t))
        })
        .fill()
        .height(28.0);
        let show = Select::new(
            Show::ALL.iter().map(|s| Choice::new(s.label())),
            Show::ALL.iter().position(|s| *s == panel.show),
            |i| msg(Event::Show(Show::ALL[i])),
        );
        let filter = text_input(texts::FILTER, &panel.filter)
            .on_input(|t| msg(Event::Filter(t)))
            .size(typography::body())
            .padding([4, 8])
            .style(style::field::validated(rows.filter_error.is_some()));
        let hint = rows
            .filter_error
            .clone()
            .unwrap_or_else(|| texts::FILTER_HINT.to_owned());
        let filter = tip(
            row![
                container(filter).width(Length::Fixed(220.0)),
                button(icon(from_web(Some("expression"))).size(16.0))
                    .style(style::button::subtle)
                    .padding(4)
                    .on_press(msg(Event::Builder)),
            ]
            .spacing(2)
            .align_y(Center),
            Tip::new(texts::FILTER).body(hint),
            iced::widget::tooltip::Position::Top,
        );
        // Each label stays with its control; in a narrow panel the controls
        // wrap, the filter last (the web's `ftable` bar).
        let filters = row![
            row![
                container(label::muted(texts::LAYER)).padding([0, 2]),
                container(layer).width(Length::Fixed(220.0)),
            ]
            .spacing(8)
            .align_y(Center),
            container(search).width(Length::Fixed(180.0)),
            row![
                container(label::muted(texts::SHOW)).padding([0, 2]),
                container(show).width(Length::Fixed(150.0)),
            ]
            .spacing(8)
            .align_y(Center),
            filter,
        ]
        .spacing(8)
        .align_y(Center)
        .wrap()
        .vertical_spacing(6);
        let count = format!("{} / {}", rows.shown.len(), rows.slots.len());
        // Dışa aktar ▾ (docs/adr/0223 §3): the rows shown, as the table shows them.
        let face = container(
            row![
                icon(from_web(Some("export"))).size(14.0),
                label::body(texts::EXPORT),
                icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::container::field_box);
        let export = tip(
            MenuButton::new(face, || {
                crate::table_write::items(Menu::new(), |f| msg(Event::Export(f)))
            }),
            Tip::new(texts::EXPORT).body(texts::EXPORT_HINT),
            iced::widget::tooltip::Position::Top,
        );
        let actions = row![
            label::muted(count),
            bar_button(
                "zoomSelection",
                texts::ZOOM,
                texts::ZOOM_HINT,
                (!self.selection.is_empty()).then(|| msg(Event::ZoomSelection)),
            ),
            export,
            bar_button(
                "tableRowBelow",
                texts::ADD_RECORD,
                texts::ADD_RECORD_HINT,
                rows.layer.is_some().then(|| msg(Event::AddRecord)),
            ),
            bar_button(
                "layerFields",
                texts::FIELDS,
                texts::FIELDS_HINT,
                rows.layer.is_some().then(|| msg(Event::Fields)),
            ),
        ]
        .spacing(10)
        .align_y(Center);
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

    /// The Tablo tab: its bar over the table.
    pub(crate) fn features_tab(&self) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::muted(texts::NO_LAYERS)).padding(12).into();
        };
        let rows = self.feature_rows(doc);
        let panel = &self.features;
        let bar = self.features_bar(&rows, self.viewport.bounds.width);
        let mut columns = vec![
            TableColumn::new("Sıra")
                .width(Length::Fixed(56.0))
                .align_right()
                .sortable(None, msg(Event::Sort(0))),
        ];
        for (j, c) in rows.table.columns.iter().enumerate() {
            let order = (rows.sorted == Some(j)).then_some(if panel.descending {
                SortOrder::Descending
            } else {
                SortOrder::Ascending
            });
            let title = match &c.field {
                Some(f) if f.required => format!("{} *", c.label),
                _ => c.label.clone(),
            };
            let width = if c.key.is_none() {
                Length::Fixed(110.0)
            } else {
                Length::Fixed(140.0)
            };
            let column = TableColumn::new(title)
                .width(width)
                .sortable(order, msg(Event::Sort(j + 1)));
            columns.push(if c.order == "number" {
                column.align_right()
            } else {
                column
            });
        }
        let model = &doc.model;
        let selection = &self.selection;
        let editing = panel.editing;
        let edit_text = panel.text.clone();
        let table_rows = rows.table.rows.clone();
        let problems = rows.table.problems.clone();
        let table_columns = rows.table.columns.clone();
        let shown = rows.shown.clone();
        let slots = rows.slots.clone();
        let count = shown.len();
        // Koşullu biçim (docs/adr/0413 §4): each object's cells' looks, the Tür column the row's.
        let looks: Vec<Vec<kentos_contracts::CellLook>> = if rows.styles.is_empty() {
            Vec::new()
        } else {
            (0..rows.slots.len())
                .map(|at| {
                    std::iter::once(rows.look(at, None))
                        .chain(
                            table_columns
                                .iter()
                                .map(|c| rows.look(at, c.key.as_deref())),
                        )
                        .collect()
                })
                .collect()
        };
        let min_width = 56.0
            + table_columns
                .iter()
                .map(|c| if c.key.is_none() { 110.0 } else { 140.0 })
                .sum::<f32>()
            + 8.0 * table_columns.len() as f32;
        let table = Table::new(columns)
            .horizontal()
            .min_width(min_width)
            .virtualized(count, move |i| {
                let at = shown[i];
                let slot = slots[at];
                let line = &table_rows[at];
                let look_of = |j: usize| looks.get(at).and_then(|l| l.get(j)).cloned();
                let row_look = look_of(0);
                let mut cells: Vec<Element<'_, Message>> = vec![
                    mouse_area(label::muted((i + 1).to_string()))
                        .on_double_click(msg(Event::Zoom(i)))
                        .into(),
                ];
                for (j, c) in table_columns.iter().enumerate() {
                    let value = line.cells[j].shown.clone();
                    if editing == Some((slot, j)) {
                        let field = c.field.as_ref();
                        let listed = field.is_some_and(|f| {
                            f.kind == LayerFieldKind::Boolean
                                || f.values.as_ref().is_some_and(|v| !v.is_empty())
                        });
                        if let (true, Some(f)) = (listed, field) {
                            let raw = model
                                .get(slot)
                                .and_then(|e| c.key.as_ref().and_then(|k| e.base().attrs.get(k)))
                                .cloned()
                                .unwrap_or_default();
                            let (choices, codes, at) = choices_of(f, &raw);
                            cells.push(
                                Select::new(choices, at, move |k| {
                                    msg(Event::Choose(codes[k].clone()))
                                })
                                .borderless()
                                .into(),
                            );
                        } else {
                            let numeric = c.order == "number";
                            cells.push(cell::edit_box(
                                &edit_text,
                                numeric,
                                |t| msg(Event::Input(t)),
                                msg(Event::Enter),
                                msg(Event::Finish(None)),
                                msg(Event::Cancel),
                            ));
                        }
                        continue;
                    }
                    let problem = problems.get(&(at, j)).cloned();
                    let shown: Element<'_, Message> = match (&problem, c.key.is_none()) {
                        // A value its field refuses (an empty required one too): the
                        // warning tint over the cell, as the web's `ftable__bad`.
                        (Some(_), _) => container(
                            label::body(value)
                                .style(style::text::warning)
                                .wrapping(Wrapping::None),
                        )
                        .width(Fill)
                        .height(Fill)
                        .align_y(Center)
                        .padding([0, 4])
                        .style(|t: &Theme| container::Style {
                            background: Some(iced::Background::Color(Color {
                                a: 0.16,
                                ..Tokens::of(t).warning
                            })),
                            ..container::Style::default()
                        })
                        .into(),
                        // One line in a cell: a long value is cut at the cell's edge, not wrapped
                        // under itself.
                        (None, true) => label::muted(value).wrapping(Wrapping::None).into(),
                        (None, false) if c.order == "number" => {
                            label::mono(value).wrapping(Wrapping::None).into()
                        }
                        (None, false) => label::body(value).wrapping(Wrapping::None).into(),
                    };
                    // The rules' look over the cell: the row's background is the line's tint, a
                    // field's own background the cell's (docs/adr/0413 §4).
                    let shown: Element<'_, Message> =
                        match look_of(j + 1).filter(|l| !l.is_plain() && problem.is_none()) {
                            Some(look) => styled_cell(
                                &line.cells[j].shown,
                                Face {
                                    muted: c.key.is_none(),
                                    number: c.order == "number",
                                },
                                look,
                                row_look.as_ref().and_then(|r| r.background.clone()),
                            ),
                            None => shown,
                        };
                    let shown: Element<'_, Message> = match problem {
                        Some(p) => tip(
                            shown,
                            Tip::new("Kurala uymuyor").body(p),
                            iced::widget::tooltip::Position::Top,
                        ),
                        None => shown,
                    };
                    cells.push(if c.key.is_some() {
                        mouse_area(shown)
                            .on_double_click(msg(Event::Edit(i, j)))
                            .into()
                    } else {
                        shown
                    });
                }
                TableLine::new(cells)
                    .tint(
                        row_look
                            .as_ref()
                            .and_then(|l| l.background.as_deref())
                            .and_then(hex_colour),
                    )
                    .selected(selection.contains(slot))
                    .current(false)
                    .on_press(msg(Event::Press(i)))
            })
            .reveal(match editing {
                Some((s, _)) => rows.shown.iter().position(|&a| rows.slots[a] == s),
                None => rows.first_selected,
            })
            .empty(if rows.layer.is_none() {
                texts::NO_LAYERS
            } else if rows.slots.is_empty() {
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

/// How a column shows its values: Tür muted, numbers in figures.
struct Face {
    muted: bool,
    number: bool,
}

fn hex_colour(hex: &str) -> Option<Color> {
    let v = u32::from_str_radix(hex.strip_prefix('#')?, 16).ok()?;
    Some(Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// The text that reads on a background the rules gave but no colour: dark on light, light on dark.
pub(crate) fn readable_on(background: Color) -> Color {
    let lum = 0.2126 * background.r + 0.7152 * background.g + 0.0722 * background.b;
    if lum > 0.55 {
        Color::from_rgb8(0x1F, 0x23, 0x28)
    } else {
        Color::WHITE
    }
}

/// A cell by the table's rules (docs/adr/0413 §4): its own background when a field's rule gives one
/// other than the row's, its text's colour (readable on the background when none is given) and weight.
fn styled_cell<'a>(
    value: &str,
    face: Face,
    look: kentos_contracts::CellLook,
    row_background: Option<String>,
) -> Element<'a, Message> {
    let own = look
        .background
        .as_deref()
        .filter(|b| Some(*b) != row_background.as_deref())
        .and_then(hex_colour);
    let under = own.or_else(|| look.background.as_deref().and_then(hex_colour));
    let colour = look
        .color
        .as_deref()
        .and_then(hex_colour)
        .or_else(|| under.map(readable_on));
    let font = match (face.number, look.bold) {
        (true, true) => typography::mono_strong(),
        (true, false) => typography::mono(),
        (false, true) => typography::ui_strong(),
        (false, false) => typography::ui(),
    };
    let text = if face.muted && colour.is_none() {
        label::muted(value.to_owned())
    } else {
        label::body(value.to_owned())
    }
    .font(font)
    .wrapping(Wrapping::None);
    let text = match colour {
        Some(c) => text.style(move |_: &Theme| iced::widget::text::Style { color: Some(c) }),
        None => text,
    };
    match own {
        None => text.into(),
        Some(background) => container(text)
            .width(Fill)
            .height(Fill)
            .align_x(if face.number {
                iced::alignment::Horizontal::Right
            } else {
                iced::alignment::Horizontal::Left
            })
            .align_y(Center)
            .padding([0, 4])
            .style(move |_: &Theme| container::Style {
                background: Some(iced::Background::Color(background)),
                ..container::Style::default()
            })
            .into(),
    }
}

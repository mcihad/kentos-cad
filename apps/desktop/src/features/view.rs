//! Öznitelik tablosu's view (docs/adr/0199 §4; the web's `FeatureTable`):
//! the bar (Katman ▾, Ara, Göster ▾, İfade süzgeci and its ε, the count,
//! Seçime yakınlaş, Alanlar…) over the table; a value cell double-clicked
//! holds its editor (a list for a value list and yes or no, else Noktalar's
//! field), a refused value its reason on hover.

use iced::widget::text::Wrapping;
use iced::widget::{button, container, mouse_area, row, space, text_input};
use iced::{Center, Color, Element, Fill, Length, Theme};
use kentos_commands::{LayerField, LayerFieldKind, check_value};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::search_box::SearchBox;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, SortOrder, Table};
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};

use super::outgoing::{Analyse, ColumnsEvent, words as out_words};
use super::{Event, Rows, Show, texts};
use crate::app::{App, Message};
use crate::icons::from_web;
use crate::points::cell;

fn msg(e: Event) -> Message {
    Message::Features(e)
}

/// A bar button: its icon alone, without a frame; its words and hint above it.
fn bar_button<'a>(
    glyph: &str,
    words: &'a str,
    hint: &'a str,
    on: Option<Message>,
) -> Element<'a, Message> {
    tip(
        button(icon(from_web(Some(glyph))).size(16.0))
            .style(style::button::ghost)
            .padding(5)
            .on_press_maybe(on),
        Tip::new(words).body(hint),
        iced::widget::tooltip::Position::Top,
    )
}

/// A bar menu: its icon and a small ▾, without a frame; `badge` beside the icon (how many are hidden).
pub(super) fn bar_menu<'a>(
    glyph: Icon,
    badge: Option<String>,
    words: String,
    hint: &'a str,
    menu: impl Fn() -> Menu<Message> + 'a,
) -> Element<'a, Message> {
    let mut face = row![icon(glyph).size(16.0)].spacing(3).align_y(Center);
    if let Some(badge) = badge {
        face = face.push(label::caption(badge).style(style::text::accent));
    }
    face = face.push(icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted));
    tip(
        MenuButton::new(container(face).padding(5), menu),
        Tip::new(words).body(hint),
        iced::widget::tooltip::Position::Top,
    )
}

/// A thin upright line between the bar's groups.
fn bar_gap<'a>() -> Element<'a, Message> {
    container(kentos_ui::widget::vertical_divider())
        .height(Length::Fixed(18.0))
        .padding([0, 4])
        .into()
}

/// What the bar needs in one row, logical pixels at the default type size: Tablo | Form, the layer (250), the
/// search (170), Göster (150), the filter (200 and its ε), the eight buttons, their gaps and margins.
const BAR_ONE_ROW: f32 = 1240.0;

/// A list editor's choices for a field: none, then its codes by their labels,
/// or yes and no; with the one written now.
fn choices_of(
    f: &LayerField,
    value: &str,
    offered: Vec<kentos_commands::FieldChoice>,
) -> (Vec<Choice>, Vec<String>, Option<usize>) {
    let pairs: Vec<(String, String)> = match &f.values {
        // A retired value only for the record holding it (docs/adr/0418), narrowed to the record's
        // subtype (docs/adr/0422).
        Some(list) if !list.is_empty() => offered.into_iter().map(|c| (c.code, c.label)).collect(),
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
    /// Katman ▾: the layers, then the project's join views (docs/adr/0527).
    pub(crate) fn layer_select<'a>(&self, rows: &Rows) -> Select<'a, Message> {
        let views: Vec<(String, String)> = self
            .document
            .as_ref()
            .map(|d| {
                d.model
                    .settings()
                    .join_views
                    .iter()
                    .map(|v| (v.id.clone(), v.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let ids: Vec<String> = rows.layers.iter().map(|(id, _, _)| id.clone()).collect();
        let n = ids.len();
        let chosen = match self.features.join.view.as_deref() {
            Some(v) => views.iter().position(|(id, _)| id == v).map(|k| n + k),
            None => rows
                .layer
                .as_ref()
                .and_then(|l| ids.iter().position(|id| id == l)),
        };
        let choices = rows
            .layers
            .iter()
            .map(|(_, path, c)| Choice::new(format!("{path} ({c})")))
            .chain(
                views
                    .iter()
                    .map(|(_, name)| Choice::new(format!("{} {name}", super::joins::words::MARK))),
            )
            .collect::<Vec<_>>();
        Select::new(choices, chosen, move |i| {
            if i < n {
                msg(Event::Layer(ids[i].clone()))
            } else {
                msg(Event::View(views[i - n].0.clone()))
            }
        })
        .placeholder(texts::NO_LAYERS)
        .inline()
        .prefix(texts::LAYER)
    }

    fn features_bar<'a>(&self, rows: &Rows, width: f32) -> Element<'a, Message> {
        let panel = &self.features;
        let layer = self.layer_select(rows);
        let search = SearchBox::new(panel.search.clone(), texts::SEARCH, |t| {
            msg(Event::Search(t))
        })
        .fill()
        .height(28.0);
        let show = Select::new(
            Show::ALL.iter().map(|s| Choice::new(s.label())),
            Show::ALL.iter().position(|s| *s == panel.show),
            |i| msg(Event::Show(Show::ALL[i])),
        )
        .inline()
        .prefix(texts::SHOW);
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
                container(filter).width(Length::Fixed(200.0)),
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
        let pane = kentos_ui::widget::segmented::Segmented::new(
            [super::Pane::Table, super::Pane::Form],
            self.features.pane,
            |p| msg(Event::Pane(p)),
        )
        .compact();
        // The labels are inside their boxes (the web's `ftable` bar); in a narrow panel the controls wrap, the
        // filter last.
        let filters = row![
            pane,
            container(layer).width(Length::Fixed(250.0)),
            container(search).width(Length::Fixed(170.0)),
            container(show).width(Length::Fixed(150.0)),
        ]
        .spacing(8)
        .align_y(Center)
        .wrap()
        .vertical_spacing(6);
        let has_layer = rows.layer.is_some();
        let on_layer = |m: Event| has_layer.then(|| msg(m));
        // Sütunlar ▾ (docs/adr/0546 §1): which columns the table shows; a hidden one is in no copy or file either.
        let hidden = self.hidden_columns(rows.layer.as_deref());
        let named: Vec<(String, String)> = rows
            .table
            .columns
            .iter()
            .filter_map(|c| Some((c.key.clone()?, c.label.clone())))
            .collect();
        let hidden_count = named.iter().filter(|(k, _)| hidden.contains(k)).count();
        let columns_menu = bar_menu(
            from_web(Some("columns")),
            (hidden_count > 0).then(|| hidden_count.to_string()),
            if hidden_count == 0 {
                out_words::COLUMNS.to_owned()
            } else {
                format!("{} ({hidden_count} gizli)", out_words::COLUMNS)
            },
            out_words::COLUMNS_HINT,
            move || {
                let mut menu = Menu::new().header(out_words::COLUMNS_HEADER);
                for (key, label) in &named {
                    menu = menu
                        .check(
                            label.clone(),
                            !hidden.contains(key),
                            Some(msg(Event::Columns(ColumnsEvent::Toggle(key.clone())))),
                        )
                        .stay();
                }
                menu.separator().item(
                    out_words::SHOW_ALL,
                    Some(msg(Event::Columns(ColumnsEvent::ShowAll))),
                )
            },
        );
        // Dışa aktar ▾ (docs/adr/0223 §3): the rows shown, as the table shows them.
        let export = bar_menu(
            from_web(Some("export")),
            None,
            texts::EXPORT.to_owned(),
            texts::EXPORT_HINT,
            || crate::table_write::items(Menu::new(), |f| msg(Event::Export(f))),
        );
        // İncele ▾ (docs/adr/0546 §2): İstatistik and Grafik over the rows the table shows, each where the
        // program has its tab (docs/adr/0802).
        let statistics = self.shows_bottom_tab(crate::bottom::BottomTab::Statistics);
        let chart = self.shows_bottom_tab(crate::bottom::BottomTab::Chart);
        let analyse = bar_menu(
            from_web(Some("chartBar")),
            None,
            out_words::ANALYSE.to_owned(),
            out_words::ANALYSE_HINT,
            move || {
                let mut menu = Menu::new();
                if !has_layer {
                    return menu;
                }
                if statistics {
                    menu = menu
                        .item(
                            out_words::STATISTICS,
                            Some(msg(Event::Analyse(Analyse::Statistics))),
                        )
                        .icon(from_web(Some("chartHistogram")));
                }
                if chart {
                    menu = menu
                        .item(out_words::CHART, Some(msg(Event::Analyse(Analyse::Chart))))
                        .icon(from_web(Some("chartPie")));
                }
                menu
            },
        );
        // ⋯: what a table is rarely asked for: a record, a copy of the selected, values pasted, the fields.
        let selected = !self.selection.is_empty();
        let more = bar_menu(
            Icon::More,
            None,
            texts::MORE.to_owned(),
            texts::MORE_HINT,
            move || {
                Menu::new()
                    .item(texts::ADD_RECORD, has_layer.then(|| msg(Event::AddRecord)))
                    .icon(from_web(Some("tableRowBelow")))
                    .detail(texts::ADD_RECORD_HINT)
                    .item(texts::DUPLICATE, selected.then(|| msg(Event::Duplicate)))
                    .icon(from_web(Some("copy")))
                    .detail(texts::DUPLICATE_HINT)
                    .item(
                        texts::PASTE,
                        has_layer.then(|| msg(Event::Write(super::write::WriteEvent::Paste))),
                    )
                    .icon(from_web(Some("paste")))
                    .detail(texts::PASTE_HINT)
                    .separator()
                    .item(texts::FIELDS, has_layer.then(|| msg(Event::Fields)))
                    .icon(from_web(Some("layerFields")))
                    .detail(texts::FIELDS_HINT)
            },
        );
        let actions = row![
            bar_button(
                "zoomSelection",
                texts::ZOOM,
                texts::ZOOM_HINT,
                selected.then(|| msg(Event::ZoomSelection)),
            ),
            columns_menu,
            bar_gap(),
            bar_button(
                "fieldCalc",
                texts::CALCULATE,
                texts::CALCULATE_HINT,
                on_layer(Event::Write(super::write::WriteEvent::Open)),
            ),
            bar_button(
                "tableGridAll",
                out_words::COPY,
                out_words::COPY_HINT,
                on_layer(Event::Copy),
            ),
            export,
            analyse,
            bar_gap(),
            more,
        ]
        .spacing(2)
        .align_y(Center);
        let one_row = width <= 0.0 || width >= typography::from_default(BAR_ONE_ROW);
        let bar: Element<'_, Message> = if one_row {
            row![filters, filter, space::horizontal(), actions]
                .spacing(8)
                .align_y(Center)
                .into()
        } else {
            // In a narrower panel the filter and the buttons share the second row.
            iced::widget::column![
                filters,
                row![filter, space::horizontal(), actions].align_y(Center)
            ]
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
        if let Some(v) = self.join_view(doc) {
            let picker = container(self.layer_select(&rows))
                .width(Length::Fixed(260.0))
                .into();
            return self.joins_tab(doc, v, picker);
        }
        let bar = self.features_bar(&rows, self.viewport.bounds.width);
        // A bound layer's table: its source, state and the one way out (docs/adr/0546 §3).
        let bar: Element<'_, Message> = match self.features_source_line(&rows) {
            Some(line) => iced::widget::column![bar, line]
                .spacing(2)
                .padding(iced::Padding::ZERO.bottom(4))
                .into(),
            None => bar,
        };
        // Kaynağın tümü: the source's rows instead of the working copy's (docs/adr/0591 §4).
        if let Some(layer) = rows.layer.as_deref()
            && self.whole_link(Some(layer)).is_some()
        {
            return iced::widget::column![
                bar,
                kentos_ui::widget::horizontal_divider(),
                self.whole_table(layer),
                self.whole_foot()
            ]
            .width(Fill)
            .height(Fill)
            .into();
        }
        if panel.pane == super::Pane::Form {
            return iced::widget::column![
                bar,
                kentos_ui::widget::horizontal_divider(),
                self.form_pane(&rows)
            ]
            .width(Fill)
            .height(Fill)
            .into();
        }
        let mut columns = vec![
            TableColumn::new("Sıra")
                .width(Length::Fixed(56.0))
                .align_right()
                .sortable(None, msg(Event::Sort(0))),
        ];
        for &j in &rows.visible {
            let c = &rows.table.columns[j];
            let order = (rows.sorted == Some(j)).then_some(if panel.descending {
                SortOrder::Descending
            } else {
                SortOrder::Ascending
            });
            let title = match &c.field {
                Some(f) if f.required => format!("{} *", c.label),
                _ => c.label.clone(),
            };
            // Wider columns for the kinds whose values are long (docs/adr/0629 §3): a 19-digit integer, a time stamp
            // with its offset, a UUID, JSON.
            let width = match (&c.key, c.field.as_ref().map(|f| f.kind)) {
                (None, _) => Length::Fixed(110.0),
                (_, Some(kentos_commands::LayerFieldKind::BigInt)) => Length::Fixed(170.0),
                (_, Some(kentos_commands::LayerFieldKind::Instant)) => Length::Fixed(240.0),
                (_, Some(kentos_commands::LayerFieldKind::Uuid)) => Length::Fixed(290.0),
                (_, Some(kentos_commands::LayerFieldKind::Json)) => Length::Fixed(220.0),
                _ => Length::Fixed(140.0),
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
        let looks: Vec<Vec<kentos_commands::CellLook>> = if rows.styles.is_empty() {
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
        let visible = rows.visible.clone();
        let min_width = 56.0
            + visible
                .iter()
                .map(|&j| {
                    if table_columns[j].key.is_none() {
                        110.0
                    } else {
                        140.0
                    }
                })
                .sum::<f32>()
            + 8.0 * visible.len() as f32;
        let table = Table::new(columns)
            .height(Fill)
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
                for &j in &visible {
                    let c = &table_columns[j];
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
                            let offered = model.get(slot).map_or_else(Vec::new, |e| {
                                let node = model.layers().get(&e.base().layer_id);
                                kentos_commands::offered_on(
                                    node.and_then(|n| n.subtypes.as_ref()),
                                    node.map_or(&[][..], |n| n.contingents.as_slice()),
                                    node.map_or(&[][..], |n| n.fields.as_slice()),
                                    f,
                                    |n| e.base().attrs.get(n).cloned(),
                                )
                            });
                            let (choices, codes, at) = choices_of(f, &raw, offered);
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
        iced::widget::column![
            bar,
            kentos_ui::widget::horizontal_divider(),
            table,
            self.features_foot(&rows)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// The line under the rows, muted: how many are shown of how many, how many are selected.
    fn features_foot<'a>(&self, rows: &Rows) -> Element<'a, Message> {
        let selected = rows
            .slots
            .iter()
            .filter(|s| self.selection.contains(**s))
            .count();
        let mut words = format!(
            "{} / {} kayıt gösteriliyor · {selected} seçili",
            rows.shown.len(),
            rows.slots.len()
        );
        if let Some(why) = &rows.filter_error {
            words.push_str(&format!(" · süzgeç okunamadı: {why}"));
        }
        foot_line(label::caption(words).style(style::text::muted).into(), None)
    }
}

/// The table's foot: a thin line over it, its words on the left, `end` on the right.
pub(crate) fn foot_line<'a>(
    words: Element<'a, Message>,
    end: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut line = row![container(words).width(Fill)]
        .spacing(8)
        .align_y(Center);
    if let Some(end) = end {
        line = line.push(end);
    }
    iced::widget::column![
        kentos_ui::widget::horizontal_divider(),
        container(line).padding([3, 10]).width(Fill)
    ]
    .into()
}

impl App {
    /// The Form view (docs/adr/0416): the rows shown as a list of records, the chosen one's form
    /// beside it, written through the form as Formla düzenle writes.
    fn form_pane<'a>(&'a self, rows: &super::Rows) -> Element<'a, Message> {
        let chosen = self
            .form_entry
            .as_ref()
            .filter(|e| e.docked)
            .and_then(|e| e.uid_slot(self));
        let mut list = iced::widget::Column::new().spacing(1).padding(2);
        for (i, &at) in rows.shown.iter().enumerate().take(MOST_LISTED) {
            let slot = rows.slots[at];
            let line = &rows.table.rows[at];
            // The record's name: its label, else its first value, else its place.
            let name = self
                .document
                .as_ref()
                .and_then(|d| d.model.get(slot))
                .and_then(|e| e.base().label.clone())
                .filter(|l| !l.trim().is_empty())
                .or_else(|| {
                    rows.table
                        .columns
                        .iter()
                        .zip(&line.cells)
                        .find(|(c, cell)| c.key.is_some() && !cell.shown.trim().is_empty())
                        .map(|(_, cell)| cell.shown.clone())
                })
                .unwrap_or_else(|| format!("Kayıt {}", i + 1));
            let kind = line
                .cells
                .first()
                .map(|c| c.shown.clone())
                .unwrap_or_default();
            let face = iced::widget::column![
                label::body(name).wrapping(Wrapping::None),
                label::caption(kind).style(style::text::muted),
            ]
            .spacing(1);
            list = list.push(
                button(face)
                    .on_press(msg(Event::FormRow(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(chosen == Some(slot))),
            );
        }
        if rows.shown.len() > MOST_LISTED {
            list = list.push(
                container(label::caption(format!(
                    "İlk {MOST_LISTED} kayıt listelendi; aramayla ya da süzgeçle daraltın."
                )))
                .padding([4, 8]),
            );
        }
        let left = container(iced::widget::scrollable(list).height(Fill))
            .width(Length::Fixed(typography::scaled(260.0)))
            .height(Fill)
            .style(style::container::field_box);
        let right: Element<'a, Message> = match self.docked_entry_view() {
            Some(form) => form,
            None => {
                let layer = rows.layer.as_deref().unwrap_or_default();
                let words = if rows.shown.is_empty() {
                    "Gösterilen kayıt yok.".to_owned()
                } else if self.form_of_layer(layer).is_none() {
                    "Bu katmanın formu yok: Veri › Formlar'dan katmanın alanlarından bir form oluşturun.".to_owned()
                } else {
                    "Soldan bir kayıt seçin.".to_owned()
                };
                container(label::muted(words)).padding(16).into()
            }
        };
        iced::widget::row![left, container(right).width(Fill).height(Fill)]
            .spacing(8)
            .padding([6, 8])
            .height(Fill)
            .into()
    }
}

/// The Form view lists at most this many records (a list beyond it says so).
const MOST_LISTED: usize = 2000;

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
    look: kentos_commands::CellLook,
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

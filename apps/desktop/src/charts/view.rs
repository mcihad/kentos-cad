//! Grafik's view (docs/adr/0227 §8; the web's `ChartPanel`): the bar (the
//! six kinds at the left; Grafik | Tablo, Panoya kopyala and Kaydet ▾ at the
//! right) over the settings column (Kaynak, Göster, the kind's settings and
//! the notes under them) and the chart, or its table.

use iced::widget::{button, canvas, column, container, row, scrollable, space, stack};
use iced::{Center, Element, Fill, Length};
use kentos_geometry_core::ops::charts::data::Kind;
use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Sort, Spec};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, Table};
use kentos_ui::widget::{Menu, MenuButton, Tip, horizontal_divider, tip, vertical_divider};

use super::canvas::{Chart, Overlay};
use super::data::layer_list;
use super::now::ColumnInfo;
use super::{BINS, Event, KINDS, Picture, Setting, View, kind_info, texts};
use crate::app::{App, Message};
use crate::features::Show;
use crate::icons::from_web;

fn msg(e: Event) -> Message {
    Message::Charts(e)
}

fn set(s: Setting) -> Message {
    msg(Event::Set(s))
}

/// The settings column's width (the web's 260 px).
const SETTINGS_WIDTH: f32 = 260.0;

/// A kind as a segment.
#[derive(Clone, Copy, PartialEq, Eq)]
struct KindChoice(ChartKind);

impl std::fmt::Display for KindChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(kind_info(self.0).title)
    }
}

/// A setting's row: its name over its control.
fn setting<'a>(title: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![
        label::muted(title).size(typography::caption()),
        control.into()
    ]
    .spacing(2)
    .into()
}

/// A list that cannot be opened now: its value, muted, in a field's box.
fn shut<'a>(text: impl Into<String>) -> Element<'a, Message> {
    container(label::muted(text.into()))
        .padding([5, 8])
        .width(Fill)
        .style(style::container::field_box)
        .into()
}

/// A list of columns: its labels (a geometry's marked), the one chosen,
/// and none first when it may be left out.
fn columns_pick<'a>(
    list: &[&ColumnInfo],
    current: Option<&str>,
    none: bool,
    on: impl Fn(Option<String>) -> Message + 'a,
) -> Element<'a, Message> {
    let mut keys: Vec<Option<String>> = Vec::new();
    let mut choices = Vec::new();
    if none {
        keys.push(None);
        choices.push(Choice::new(texts::NONE));
    }
    for c in list {
        keys.push(Some(c.key.clone()));
        let choice = Choice::new(c.label.clone());
        choices.push(if c.key.starts_with('$') {
            choice.detail(texts::GEOMETRY)
        } else {
            choice
        });
    }
    let chosen = keys.iter().position(|k| k.as_deref() == current);
    Select::new(choices, chosen, move |i| on(keys[i].clone())).into()
}

/// The aggregates a kind offers, as a list.
fn aggregate_pick<'a>(allowed: &[Aggregate], current: Aggregate) -> Element<'a, Message> {
    let list: Vec<Aggregate> = Aggregate::ALL
        .into_iter()
        .filter(|a| allowed.contains(a))
        .collect();
    let chosen = list.iter().position(|a| *a == current);
    let choices: Vec<Choice> = list.iter().map(|a| Choice::new(a.label())).collect();
    Select::new(choices, chosen, move |i| set(Setting::Aggregate(list[i]))).into()
}

impl App {
    /// The Grafik tab.
    pub(crate) fn chart_tab(&self) -> Element<'_, Message> {
        self.chart_refresh();
        let now = self.charts.chart.borrow();
        let spec = now.spec.clone();
        let bar = self.chart_bar(spec.as_ref().map(|s| s.kind), now.table.is_some());
        let settings = self.chart_settings(
            spec.as_ref(),
            &now.columns,
            now.error.as_deref(),
            &now.notes,
        );
        let main: Element<'_, Message> = if self.chart_table_view() {
            self.chart_numbers(now.table.clone(), now.error.clone())
        } else {
            let scatter = now.kind == Some(ChartKind::Scatter);
            let stamp = now.stamp;
            let scale = self.viewport.scale_factor() as f32;
            let selects = !self.charting_table();
            stack![
                canvas(Chart {
                    panel: &self.charts,
                    stamp,
                    scale,
                })
                .width(Fill)
                .height(Fill),
                canvas(Overlay {
                    panel: &self.charts,
                    scatter,
                    selects,
                })
                .width(Fill)
                .height(Fill),
            ]
            .into()
        };
        drop(now);
        let body = row![
            container(
                scrollable(settings)
                    .direction(style::field::body_scrollbar())
                    .height(Fill)
            )
            .width(Length::Fixed(typography::from_default(SETTINGS_WIDTH)))
            .height(Fill)
            .style(style::container::surface),
            vertical_divider(),
            container(main).width(Fill).height(Fill),
        ];
        column![bar, horizontal_divider(), body]
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// The bar: the kinds, then Grafik | Tablo, Panoya kopyala and Kaydet ▾.
    fn chart_bar<'a>(&self, kind: Option<ChartKind>, has_table: bool) -> Element<'a, Message> {
        let kinds = Segmented::new(
            KINDS.iter().map(|k| KindChoice(k.kind)),
            KindChoice(kind.unwrap_or(ChartKind::Bar)),
            |k| msg(Event::Kind(k.0)),
        )
        .icons(KINDS.iter().map(|k| from_web(Some(k.icon))))
        .hints(KINDS.iter().map(|k| k.description))
        .icon_only();
        let views = Segmented::new([View::Chart, View::Table], self.charts.view, |v| {
            msg(Event::View(v))
        });
        let copy = tip(
            button(
                row![
                    icon(from_web(Some("copy"))).size(14.0),
                    label::body(texts::COPY)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .style(style::button::secondary)
            .padding([4, 10])
            .on_press_maybe(has_table.then(|| msg(Event::Copy))),
            Tip::new(texts::COPY).body(texts::COPY_HINT),
            iced::widget::tooltip::Position::Top,
        );
        let face = container(
            row![
                icon(from_web(Some("export"))).size(14.0),
                label::body(texts::SAVE),
                icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::container::field_box);
        let save = tip(
            MenuButton::new(face, || {
                Menu::new()
                    .item(texts::PNG, Some(msg(Event::Save(Picture::Png))))
                    .icon(from_web(Some("exportPng")))
                    .item(texts::SVG, Some(msg(Event::Save(Picture::Svg))))
                    .icon(from_web(Some("exportSvg")))
            }),
            Tip::new(texts::SAVE).body(texts::SAVE_HINT),
            iced::widget::tooltip::Position::Top,
        );
        container(
            row![kinds, space::horizontal(), views, copy, save]
                .spacing(10)
                .align_y(Center),
        )
        .padding([6, 10])
        .width(Fill)
        .into()
    }

    /// Kaynak and (for a layer) Göster, the column's first rows.
    fn chart_source<'a>(&self) -> Vec<Element<'a, Message>> {
        let Some(doc) = &self.document else {
            return vec![setting(texts::SOURCE, shut(texts::NO_LAYERS))];
        };
        let layers = layer_list(self, doc);
        let table = self.charting_table();
        let current = self.chart_layer();
        let mut ids: Vec<Option<String>> =
            layers.iter().map(|(id, _, _)| Some(id.clone())).collect();
        // The list counts each layer's objects; the closed box names the layer (the web's).
        let mut choices: Vec<Choice> = layers
            .iter()
            .map(|(_, path, n)| Choice::new(format!("{path} ({n})")).shown(path.clone()))
            .collect();
        if let Some(t) = &self.charts.table {
            ids.push(None);
            choices.push(Choice::new(texts::table(&t.title)).icon(from_web(Some("table"))));
        }
        let chosen = if table {
            ids.iter().position(Option::is_none)
        } else {
            ids.iter().position(|id| id.is_some() && *id == current)
        };
        let source = Select::new(choices, chosen, move |i| match &ids[i] {
            Some(id) => msg(Event::Layer(id.clone())),
            None => msg(Event::TableSource),
        })
        .placeholder(texts::NO_LAYERS);
        let mut rows = vec![setting(texts::SOURCE, source)];
        if !table {
            let show = Select::new(
                Show::WITH_TABLE.iter().map(|s| Choice::new(s.label())),
                Show::WITH_TABLE.iter().position(|s| *s == self.charts.show),
                |i| msg(Event::Show(Show::WITH_TABLE[i])),
            );
            rows.push(setting(texts::SHOW, show));
        }
        rows
    }

    /// The kind's settings: a column's list, an aggregate, an order; the notes under them.
    fn chart_settings<'a>(
        &self,
        spec: Option<&Spec>,
        columns: &[ColumnInfo],
        error: Option<&str>,
        notes: &[String],
    ) -> Element<'a, Message> {
        let mut rows = self.chart_source();
        let numbers: Vec<&ColumnInfo> = columns.iter().filter(|c| c.kind == Kind::Number).collect();
        let all: Vec<&ColumnInfo> = columns.iter().collect();
        let label = |k: Option<&str>| {
            columns
                .iter()
                .find(|c| Some(c.key.as_str()) == k)
                .map_or(texts::NONE.to_owned(), |c| c.label.clone())
        };
        if let Some(spec) = spec {
            match spec.kind {
                ChartKind::Bar | ChartKind::Pie => {
                    rows.push(setting(
                        texts::CATEGORY,
                        columns_pick(&all, spec.category.as_deref(), false, |k| {
                            set(Setting::Category(k.unwrap_or_default()))
                        }),
                    ));
                    let allowed: &[Aggregate] = if spec.kind == ChartKind::Pie {
                        &[Aggregate::Count, Aggregate::Sum]
                    } else {
                        &[
                            Aggregate::Count,
                            Aggregate::Sum,
                            Aggregate::Mean,
                            Aggregate::Min,
                            Aggregate::Max,
                            Aggregate::Median,
                        ]
                    };
                    rows.push(setting(
                        texts::AGGREGATE,
                        aggregate_pick(allowed, spec.aggregate),
                    ));
                    let value = if spec.aggregate == Aggregate::Count || numbers.is_empty() {
                        shut(if spec.aggregate == Aggregate::Count {
                            "—".to_owned()
                        } else {
                            label(spec.value.as_deref())
                        })
                    } else {
                        columns_pick(&numbers, spec.value.as_deref(), false, |k| {
                            set(Setting::Value(k.unwrap_or_default()))
                        })
                    };
                    rows.push(setting(texts::VALUE, value));
                    if spec.kind == ChartKind::Bar {
                        rows.push(setting(
                            texts::SORT,
                            Select::new(
                                Sort::ALL.iter().map(|s| Choice::new(s.label())),
                                Sort::ALL.iter().position(|s| *s == spec.sort),
                                |i| set(Setting::Sort(Sort::ALL[i])),
                            ),
                        ));
                    }
                }
                ChartKind::Histogram => {
                    rows.push(setting(
                        texts::VALUE,
                        self.numbers_pick(&numbers, spec.value.as_deref(), &label, Setting::Value),
                    ));
                    let bins = spec.bins.unwrap_or(0);
                    rows.push(setting(
                        texts::BINS,
                        Select::new(
                            BINS.iter().map(|b| {
                                Choice::new(if *b == 0 {
                                    texts::AUTO.to_owned()
                                } else {
                                    b.to_string()
                                })
                            }),
                            BINS.iter().position(|b| *b == bins),
                            |i| set(Setting::Bins((BINS[i] > 0).then_some(BINS[i]))),
                        ),
                    ));
                    rows.push(
                        Switch::new(spec.guides, |on| set(Setting::Guides(on)))
                            .label(texts::GUIDES)
                            .into(),
                    );
                }
                ChartKind::Box => {
                    rows.push(setting(
                        texts::VALUE,
                        self.numbers_pick(&numbers, spec.value.as_deref(), &label, Setting::Value),
                    ));
                    rows.push(setting(
                        texts::GROUP,
                        columns_pick(&all, spec.group.as_deref(), true, |k| {
                            set(Setting::Group(k))
                        }),
                    ));
                }
                ChartKind::Scatter => {
                    rows.push(setting(
                        texts::X,
                        self.numbers_pick(&numbers, spec.x.as_deref(), &label, Setting::X),
                    ));
                    rows.push(setting(
                        texts::Y,
                        self.numbers_pick(
                            &numbers,
                            spec.y.first().map(String::as_str),
                            &label,
                            Setting::Y,
                        ),
                    ));
                    rows.push(
                        Switch::new(spec.trend, |on| set(Setting::Trend(on)))
                            .label(texts::TREND)
                            .into(),
                    );
                }
                ChartKind::Line => {
                    rows.push(setting(
                        texts::X,
                        self.numbers_pick(&numbers, spec.x.as_deref(), &label, Setting::X),
                    ));
                    rows.push(setting(texts::Y, y_pick(&numbers, &spec.y, &label)));
                    let series = if spec.y.len() > 1 {
                        shut(label(spec.series.as_deref()))
                    } else {
                        columns_pick(&all, spec.series.as_deref(), true, |k| {
                            set(Setting::Series(k))
                        })
                    };
                    rows.push(setting(texts::SERIES, series));
                    rows.push(setting(
                        texts::AGGREGATE,
                        aggregate_pick(&Aggregate::ALL, spec.aggregate),
                    ));
                }
            }
        }
        // The notes: why it cannot be drawn, what was left out, a source without numbers.
        let mut said: Vec<String> = error.map(str::to_owned).into_iter().collect();
        said.extend(notes.iter().cloned());
        if spec.is_some() && numbers.is_empty() {
            said.push(texts::NO_NUMBERS.to_owned());
        }
        for n in said {
            rows.push(
                row![
                    icon(from_web(Some("warning")))
                        .size(14.0)
                        .tone(Tone::Warning),
                    label::muted(n).size(typography::caption()),
                ]
                .spacing(6)
                .into(),
            );
        }
        column(rows).spacing(8).padding([8, 12]).width(Fill).into()
    }

    /// A list of number columns, or its value shut while there are none.
    fn numbers_pick<'a>(
        &self,
        numbers: &[&ColumnInfo],
        current: Option<&str>,
        label: &dyn Fn(Option<&str>) -> String,
        make: fn(String) -> Setting,
    ) -> Element<'a, Message> {
        if numbers.is_empty() {
            return shut(label(current));
        }
        columns_pick(numbers, current, false, move |k| {
            set(make(k.unwrap_or_default()))
        })
    }

    /// Tablo: the chart's numbers (numbers at the right).
    fn chart_numbers<'a>(
        &self,
        table: Option<kentos_geometry_core::ops::charts::model::Table>,
        error: Option<String>,
    ) -> Element<'a, Message> {
        let Some(t) = table else {
            return container(label::muted(error.unwrap_or_default()))
                .padding(12)
                .into();
        };
        let numeric: Vec<bool> = (0..t.columns.len())
            .map(|j| {
                !t.rows.is_empty()
                    && t.rows.iter().all(|r| {
                        r.get(j).is_none_or(|c| {
                            c.is_empty() || {
                                let s = c.strip_prefix('-').unwrap_or(c);
                                let (a, b) = s.split_once('.').unwrap_or((s, "0"));
                                !a.is_empty()
                                    && a.bytes().all(|b| b.is_ascii_digit())
                                    && !b.is_empty()
                                    && b.bytes().all(|b| b.is_ascii_digit())
                            }
                        })
                    })
            })
            .collect();
        // The columns share the room (the web's full-width table).
        let columns = t.columns.iter().zip(&numeric).map(|(c, n)| {
            let column = TableColumn::new(c.clone()).width(Length::Fill);
            if *n { column.align_right() } else { column }
        });
        let rows = t.rows.clone();
        let count = rows.len();
        Table::new(columns)
            .virtualized(count, move |i| {
                let cells = rows[i]
                    .iter()
                    .zip(&numeric)
                    .map(|(c, n)| -> Element<'_, Message> {
                        if *n {
                            label::mono(c.clone()).into()
                        } else {
                            label::body(c.clone()).into()
                        }
                    });
                TableLine::new(cells)
            })
            .into()
    }
}

/// Çizgi's Y: a menu of the number columns, each ticked or not; it stays
/// open while columns are taken (at most eight).
fn y_pick<'a>(
    numbers: &[&ColumnInfo],
    ys: &[String],
    label: &dyn Fn(Option<&str>) -> String,
) -> Element<'a, Message> {
    if numbers.is_empty() {
        return shut(texts::NONE);
    }
    let shown = if ys.is_empty() {
        texts::NONE.to_owned()
    } else {
        ys.iter()
            .map(|k| label(Some(k)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let face = container(
        row![
            label::body(typography::elide(&shown, typography::body(), 200.0).into_owned())
                .width(Fill),
            icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([5, 8])
    .width(Fill)
    .style(style::container::field_box);
    let items: Vec<(String, String, bool)> = numbers
        .iter()
        .map(|c| (c.key.clone(), c.label.clone(), ys.contains(&c.key)))
        .collect();
    MenuButton::new(face, move || {
        items.iter().fold(Menu::new(), |m, (key, title, on)| {
            m.check(title.clone(), *on, Some(set(Setting::YToggle(key.clone()))))
                .stay()
        })
    })
    .edge()
    .into()
}

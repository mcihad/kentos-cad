//! İstatistik's view (docs/adr/0227 §8; the web's `StatisticsPanel`): the
//! bar (Katman ▾, Göster ▾, the count; Grafik and Panoya kopyala at the
//! right) over the summary table and the chosen field's statistics.

use iced::widget::{button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length};
use kentos_ui::icon::{Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, Table};
use kentos_ui::widget::{Tip, horizontal_divider, tip, vertical_divider};

use super::source::texts as whole_texts;
use super::{Event, StatsNow, summary_cells, texts};
use crate::app::{App, Message};
use crate::charts::data::layer_list;
use crate::features::Show;
use crate::icons::from_web;

fn msg(e: Event) -> Message {
    Message::Statistics(e)
}

/// The detail pane's width (the web's 320 px).
const DETAIL_WIDTH: f32 = 320.0;

/// The summary table's columns' widths: the field's name, its kind, the
/// numbers (in the interface's typeface, as the web's); all nine fit beside
/// the field's statistics at 1440 px, a narrower window scrolls them.
const WIDTHS: [f32; 9] = [132.0, 64.0, 40.0, 36.0, 64.0, 72.0, 72.0, 72.0, 78.0];

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

impl App {
    /// The İstatistik tab.
    pub(crate) fn statistics_tab(&self) -> Element<'_, Message> {
        self.stats_refresh();
        let now = self.charts.stats.borrow();
        let bar = self.statistics_bar(&now);
        let table = self.summary_table(&now);
        let detail = match self.whole_pane() {
            Some(whole) => whole,
            None => detail_pane(&now),
        };
        let detail = match self.scope_row() {
            Some(scope) => column![scope, horizontal_divider(), detail].into(),
            None => detail,
        };
        drop(now);
        let body = row![
            container(table).width(Fill).height(Fill),
            vertical_divider(),
            container(
                scrollable(detail)
                    .direction(style::field::body_scrollbar())
                    .height(Fill)
            )
            .width(Length::Fixed(typography::from_default(DETAIL_WIDTH)))
            .height(Fill)
            .style(style::container::surface),
        ];
        column![bar, horizontal_divider(), body]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn statistics_bar<'a>(&self, now: &StatsNow) -> Element<'a, Message> {
        let (layers, current) = match &self.document {
            Some(doc) => (layer_list(self, doc), self.chart_layer()),
            None => (Vec::new(), None),
        };
        let ids: Vec<String> = layers.iter().map(|(id, _, _)| id.clone()).collect();
        let chosen = current.and_then(|l| ids.iter().position(|id| *id == l));
        let layer = Select::new(
            layers
                .iter()
                .map(|(_, path, n)| Choice::new(format!("{path} ({n})")).shown(path.clone())),
            chosen,
            move |i| msg(Event::Layer(ids[i].clone())),
        )
        .placeholder(texts::NO_LAYERS);
        let show = Select::new(
            Show::WITH_TABLE.iter().map(|s| Choice::new(s.label())),
            Show::WITH_TABLE.iter().position(|s| *s == self.charts.show),
            |i| msg(Event::Show(Show::WITH_TABLE[i])),
        );
        let filters = row![
            container(label::muted(texts::LAYER)).padding([0, 2]),
            container(layer).width(Length::Fixed(180.0)),
            container(label::muted(texts::SHOW)).padding([0, 2]),
            container(show).width(Length::Fixed(140.0)),
            label::muted(texts::objects(now.rows.len())),
        ]
        .spacing(8)
        .align_y(Center);
        let field = self.charts.field.clone().filter(|f| now.keys.contains(f));
        let actions = row![
            bar_button(
                "chart",
                texts::CHART,
                texts::CHART_HINT,
                field.map(|_| msg(Event::Chart)),
            ),
            bar_button(
                "copy",
                texts::COPY,
                texts::COPY_HINT,
                Some(msg(Event::Copy))
            ),
        ]
        .spacing(10)
        .align_y(Center);
        container(row![filters, space::horizontal(), actions].align_y(Center))
            .padding([6, 10])
            .width(Fill)
            .into()
    }

    /// The summary table: a row a field, its numbers at the right; “…” while it is counted.
    fn summary_table<'a>(&self, now: &StatsNow) -> Element<'a, Message> {
        let columns = texts::COLUMNS
            .iter()
            .zip(WIDTHS)
            .enumerate()
            .map(|(j, (t, w))| {
                let c = TableColumn::new(*t).width(Length::Fixed(w));
                if j >= 2 { c.align_right() } else { c }
            });
        let lines: Vec<[String; 9]> = now
            .summaries
            .iter()
            .zip(&now.labels)
            .map(|(s, label)| match s {
                Some(s) => summary_cells(s),
                None => [
                    label.clone(),
                    String::new(),
                    "…".to_owned(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
            })
            .collect();
        let keys = now.keys.clone();
        let chosen = self.charts.field.clone();
        let count = lines.len();
        let empty = if self.chart_layer().is_none() {
            texts::NO_LAYERS
        } else {
            texts::NONE
        };
        Table::new(columns)
            .horizontal()
            .min_width(WIDTHS.iter().sum::<f32>() + 8.0 * WIDTHS.len() as f32)
            .virtualized(if now.objects == 0 { 0 } else { count }, move |i| {
                let key = keys[i].clone();
                let geometry = key.starts_with('$');
                let cells = lines[i]
                    .iter()
                    .enumerate()
                    .map(|(j, c)| -> Element<'_, Message> {
                        match j {
                            0 if geometry => tip(
                                label::strong(c.clone()),
                                Tip::new(texts::GEOMETRY),
                                iced::widget::tooltip::Position::Top,
                            ),
                            0 => label::strong(c.clone()).into(),
                            1 => label::muted(c.clone()).into(),
                            _ => label::body(c.clone()).into(),
                        }
                    });
                TableLine::new(cells)
                    .selected(chosen.as_deref() == Some(key.as_str()))
                    .on_press(msg(Event::Field(key)))
            })
            .empty(empty)
            .into()
    }
}

impl App {
    /// Kapsam, over the detail pane of a bound layer: Çalışma kopyası or Kaynağın tümü.
    fn scope_row<'a>(&self) -> Option<Element<'a, Message>> {
        self.stats_bound()?;
        let scope = Select::new(
            [Choice::new(whole_texts::LOADED), Choice::new(whole_texts::WHOLE)],
            Some(usize::from(self.charts.whole.on)),
            |i| msg(Event::Scope(i == 1)),
        );
        Some(tip(
            row![
                label::muted(whole_texts::SCOPE),
                container(scope).width(Fill),
            ]
            .spacing(8)
            .align_y(Center)
            .padding([6, 12]),
            Tip::new(whole_texts::SCOPE).body(whole_texts::SCOPE_HINT),
            iced::widget::tooltip::Position::Top,
        ))
    }

    /// Kaynağın tümü's pane, while that scope is chosen for a bound layer and a field: the source's answer, or
    /// that it is being read (with Durdur).
    fn whole_pane<'a>(&self) -> Option<Element<'a, Message>> {
        let w = &self.charts.whole;
        if !w.on || self.stats_bound().is_none() {
            return None;
        }
        let field = self.charts.field.clone()?;
        let note = |t: String| -> Element<'a, Message> {
            container(label::muted(t).size(typography::caption()))
                .padding([10, 12])
                .into()
        };
        if field.starts_with('$') {
            return Some(note(whole_texts::GEOMETRY.to_owned()));
        }
        let head = row![
            label::heading(field.clone()),
            label::caption(whole_texts::WHOLE),
        ]
        .spacing(8)
        .align_y(iced::Alignment::End);
        if w.running.is_some() {
            return Some(
                column![
                    head,
                    row![
                        label::muted(whole_texts::READING),
                        space::horizontal(),
                        bar_button(
                            "close",
                            whole_texts::STOP,
                            whole_texts::STOP,
                            Some(msg(Event::WholeStop)),
                        ),
                    ]
                    .align_y(Center),
                ]
                .spacing(8)
                .padding([8, 12])
                .into(),
            );
        }
        let answer = match &w.answer {
            None => return Some(note(whole_texts::READING.to_owned())),
            Some(Err(why)) => return Some(note(why.clone())),
            Some(Ok(a)) => a,
        };
        let line = |k: &'static str, v: String| -> Element<'a, Message> {
            row![label::muted(k).width(Fill), label::body(v)]
                .spacing(12)
                .into()
        };
        let mut lines = vec![line(
            whole_texts::RECORDS,
            answer.total.unwrap_or_default().to_string(),
        )];
        if let Some(st) = answer.statistics.first() {
            lines.push(line(whole_texts::READ, st.read.to_string()));
            lines.push(line(whole_texts::SKIPPED, st.skipped.to_string()));
            for (k, v) in [
                (whole_texts::SUM, &st.sum),
                (whole_texts::MEAN, &st.mean),
                (whole_texts::MIN, &st.min),
                (whole_texts::MAX, &st.max),
                (whole_texts::STD, &st.std),
            ] {
                if let Some(v) = v {
                    lines.push(line(k, v.clone()));
                }
            }
        }
        Some(
            column![
                head,
                column(lines).spacing(2),
                horizontal_divider(),
                label::muted(whole_texts::handed(answer.read)).size(typography::caption()),
            ]
            .spacing(6)
            .padding([8, 12])
            .width(Fill)
            .into(),
        )
    }
}

/// The chosen field's statistics and its values; a value row selects its objects.
fn detail_pane<'a>(now: &StatsNow) -> Element<'a, Message> {
    let Some(d) = &now.detail else {
        return container(label::muted(texts::PICK).size(typography::caption()))
            .padding([10, 12])
            .into();
    };
    let head = row![
        label::heading(d.summary.label.clone()),
        label::caption(d.summary.kind.clone()),
    ]
    .spacing(8)
    .align_y(iced::Alignment::End);
    let stats = column(d.stats.iter().map(|s| {
        row![
            label::muted(s.label.clone()).width(Fill),
            label::body(s.value.clone()),
        ]
        .spacing(12)
        .into()
    }))
    .spacing(2);
    let mut values: Vec<Element<'a, Message>> = d
        .values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let marked = v.mark != kentos_geometry_core::ops::charts::fields::Mark::Value;
            let text = if marked {
                label::caption(v.text.clone())
            } else {
                label::body(v.text.clone())
            };
            button(
                row![
                    container(text).width(Fill).clip(true),
                    label::body(v.count.to_string()),
                    container(label::caption(format!("%{}", v.share)))
                        .width(Length::Fixed(56.0))
                        .align_x(iced::alignment::Horizontal::Right),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .padding([0, 4])
            .width(Fill)
            .height(Length::Fixed(typography::from_default(24.0)))
            .style(style::button::row(false))
            .on_press(msg(Event::Value(i)))
            .into()
        })
        .collect();
    if d.more > 0 {
        values.push(
            container(label::caption(texts::more(d.more)))
                .padding([4, 4])
                .into(),
        );
    }
    let notes = d.notes.iter().map(|n| {
        row![
            icon(from_web(Some("warning")))
                .size(14.0)
                .tone(Tone::Warning),
            label::muted(n.clone()).size(typography::caption()),
        ]
        .spacing(6)
        .into()
    });
    column![
        head,
        stats,
        horizontal_divider(),
        label::muted(texts::VALUES).size(typography::caption()),
        column(values),
    ]
    .extend(notes)
    .spacing(6)
    .padding([8, 12])
    .width(Fill)
    .into()
}

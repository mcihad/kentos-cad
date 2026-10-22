//! Öznitelik tablosu's Kaynağın tümü for a bound layer (docs/adr/0591 §4; plan/gis.md CBS-20): the table shows the
//! layer's source itself, every row of it, read a page at a time by `cad.sources.query` (`scope: source`) on a
//! thread of its own with Durdur; the drawing does not wait for it.
//!
//! - A header press orders the rows: a number, date or true/false column by the source, a text one here (at most
//!   200 000 records); Sıra goes back to the source's key order. The İfade süzgeci is the question's filter: its
//!   part the source can check goes to it.
//! - Under the rows Sonraki sayfa reads the next page from where the last one ended (the answer's `next`), of the
//!   same file (`expectedStamp`): a page neither repeats nor skips a row. A file changed meanwhile is said, and the
//!   first page read again.
//! - The rows are the source's, read only here: a row the working copy holds selects its object when pressed; one it
//!   does not hold is muted. Edits are made in Çalışma kopyası.
//! - The line over the rows says how many were read of how many, who ordered them and where the filter was checked.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Task;
use iced::futures::channel::oneshot;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Length};
use kentos_commands::{
    CommandResult, QueryOrder, QueryScope, SourceLink, SourcesQueried, SourcesQuery, js_trim,
};
use kentos_domain::Uuid;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, SortOrder, Table};
use kentos_ui::widget::{Tip, tip};
use kentos_ui::{label, style};

use super::{Event, texts};
use crate::app::{App, Message};

/// The words.
pub mod words {
    pub const WHOLE: &str = "Kaynağın tümü";
    pub const WHOLE_HINT: &str = "Tablo bağlı kaynağın bütün kayıtlarını kaynaktan sayfa sayfa okur; sıralama ve \
         süzgeç kaynağa gider. Kayıtlar burada salt okunur; düzenleme Çalışma kopyası'nda yapılır.";
    pub const LOADED: &str = "Çalışma kopyası";
    pub const LOADED_HINT: &str = "Tablo katmanın çizimdeki nesnelerini gösterir.";
    pub const MORE: &str = "Sonraki sayfa";
    pub const READING: &str = "Kaynak okunuyor…";
    pub const STOP: &str = "Durdur";
    pub const STOPPED: &str = "Kaynağın okunması durduruldu.";
    pub const NO_ROWS: &str = "Kaynakta soruya uyan kayıt yok.";
    pub const NOT_HELD: &str =
        "Bu kayıt çalışma kopyasında değil: katman tablonun yalnız bir parçasını tutuyor.";
    pub const CHANGED: &str =
        "Kaynak ilk sayfa okunduğundan beri değişti; ilk sayfa yeniden okundu.";
    pub const BY_KEY: &str = "anahtar sırasıyla";
    pub const BY_SOURCE: &str = "kaynakta sıralandı";
    pub const BY_HERE: &str = "burada sıralandı (en çok 200 000 kayıt)";
    pub const FILTER_SENT: &str = "süzgeç kaynakta";
    pub const FILTER_BOTH: &str = "süzgeç kaynakta ve burada";
    pub const FILTER_HERE: &str = "süzgeç burada";
    pub fn read(shown: usize, total: Option<u64>) -> String {
        match total {
            Some(t) => format!("{shown} / {t} kayıt okundu"),
            None => format!("{shown} kayıt okundu"),
        }
    }
}

/// How many rows a page asks for.
pub(crate) const PAGE: u32 = 500;

/// Kaynağın tümü's state in the tab.
#[derive(Default)]
pub(crate) struct WholePane {
    /// The bound layer shown whole (none: the working copy).
    pub layer: Option<String>,
    /// The ordered column and which way (none: the source's key order).
    pub sort: Option<String>,
    pub descending: bool,
    /// The question's number: an answer to an older one is dropped.
    ticket: u64,
    pub running: Option<Arc<AtomicBool>>,
    /// The question last asked: its number, whether it goes after the rows read, and the input.
    pub asked: Option<(u64, bool, SourcesQuery)>,
    /// The pages read so far, joined, and the last page's answer (its next, plan, total and stamp).
    pub rows: Vec<kentos_commands::QueriedRow>,
    pub last: Option<SourcesQueried>,
    /// The first page's total and stamp.
    total: Option<u64>,
    stamp: Option<String>,
    pub problem: Option<String>,
}

/// Messages of Kaynağın tümü.
#[derive(Debug, Clone, PartialEq)]
pub enum WholeEvent {
    /// Kaynağın tümü or Çalışma kopyası.
    Toggle(bool),
    /// A header pressed, by the shown columns (0: Sıra).
    Sort(usize),
    More,
    Stop,
    /// A row pressed.
    Press(usize),
    /// A page's answer: the question's number, whether it goes after the rows read, and the answer (a code and
    /// words when refused).
    Answered(u64, bool, Box<Result<SourcesQueried, (String, String)>>),
}

fn msg(e: WholeEvent) -> Message {
    Message::Features(Event::Whole(e))
}

impl App {
    /// The bound layer the table shows whole, while it is the layer shown and bound.
    pub(crate) fn whole_link(&self, layer: Option<&str>) -> Option<SourceLink> {
        let layer = layer?;
        if self.features.whole.layer.as_deref() != Some(layer) {
            return None;
        }
        self.document
            .as_ref()?
            .model
            .settings()
            .sources
            .iter()
            .find(|s| s.layer == layer)
            .cloned()
    }

    /// The columns shown: the layer's fields (a bound layer has its table's columns), the reader's hidden left out;
    /// a layer without fields shows the keys the rows have, in the order first met.
    pub(crate) fn whole_columns(&self, layer: &str) -> Vec<String> {
        let hidden = self.features.hidden.get(layer);
        let fields: Vec<String> = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(layer))
            .map(|n| n.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        let mut out: Vec<String> = if fields.is_empty() {
            let mut seen = Vec::new();
            for r in &self.features.whole.rows {
                for k in r.attrs.keys() {
                    if !seen.contains(k) {
                        seen.push(k.clone());
                    }
                }
            }
            seen
        } else {
            fields
        };
        out.retain(|c| hidden.is_none_or(|h| !h.contains(c)));
        out
    }

    /// Asks a page: the first (`more` false) or the one after the rows read.
    pub(crate) fn whole_ask(&mut self, more: bool) -> Task<Message> {
        let Some(layer) = self.features.whole.layer.clone() else {
            return Task::none();
        };
        let (Some(link), Some(doc)) = (self.whole_link(Some(&layer)), self.document.as_ref())
        else {
            return Task::none();
        };
        let p = &mut self.features.whole;
        if let Some(stop) = p.running.take() {
            stop.store(true, Ordering::Relaxed);
        }
        let cursor = if more {
            match p.last.as_ref().and_then(|l| l.next.clone()) {
                Some(next) => Some(next),
                None => return Task::none(),
            }
        } else {
            p.rows.clear();
            p.last = None;
            p.total = None;
            p.stamp = None;
            None
        };
        p.problem = None;
        p.ticket += 1;
        let ticket = p.ticket;
        let filter = js_trim(&self.features.filter).to_owned();
        let input = SourcesQuery {
            layer: Some(link.layer.clone()),
            scope: Some(QueryScope::Source),
            filter: (!filter.is_empty()).then_some(filter),
            limit: Some(PAGE),
            count: Some(!more),
            order: p.sort.clone().map(|field| QueryOrder {
                field,
                descending: p.descending,
            }),
            cursor,
            expected_stamp: if more { p.stamp.clone() } else { None },
            ..SourcesQuery::default()
        };
        let settings = doc.model.settings().clone();
        let revision = doc.model.revision().to_string();
        let stop = Arc::new(AtomicBool::new(false));
        p.running = Some(stop.clone());
        p.asked = Some((ticket, more, input.clone()));
        let (done, wait) = oneshot::channel();
        std::thread::spawn(move || {
            let answer = kentos_native_application::sources_query::ask_source(
                &settings,
                link.kind,
                &link.path,
                &link.table,
                &link.layer,
                &input,
                revision,
                &|| stop.load(Ordering::Relaxed),
            );
            let _ = done.send(match answer {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                    Err((error.code, error.message))
                }
                CommandResult::Cancelled => Err(("cancelled".into(), words::STOPPED.into())),
                other => Err((String::new(), format!("{other:?}"))),
            });
        });
        Task::perform(wait, move |answer| {
            msg(WholeEvent::Answered(
                ticket,
                more,
                Box::new(
                    answer.unwrap_or_else(|_| Err(("cancelled".into(), words::STOPPED.into()))),
                ),
            ))
        })
    }

    /// The filter changed while the table shows the source: the first page is asked again.
    pub(crate) fn whole_filtered(&mut self) -> Task<Message> {
        if self.features.whole.layer.is_some() {
            return self.whole_ask(false);
        }
        Task::none()
    }

    pub(crate) fn whole_event(&mut self, e: WholeEvent) -> Task<Message> {
        match e {
            WholeEvent::Toggle(on) => {
                self.whole_stop();
                let p = &mut self.features.whole;
                p.rows.clear();
                p.last = None;
                p.problem = None;
                if !on {
                    p.layer = None;
                    return Task::none();
                }
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let layer = self.feature_layer(doc);
                self.features.whole.layer = layer;
                self.features.whole.sort = None;
                self.features.whole.descending = false;
                return self.whole_ask(false);
            }
            WholeEvent::Sort(column) => {
                let Some(layer) = self.features.whole.layer.clone() else {
                    return Task::none();
                };
                let columns = self.whole_columns(&layer);
                let p = &mut self.features.whole;
                if column == 0 {
                    (p.sort, p.descending) = (None, false);
                } else if let Some(c) = columns.get(column - 1) {
                    (p.sort, p.descending) = super::next_sort(&p.sort, p.descending, c);
                }
                return self.whole_ask(false);
            }
            WholeEvent::More => return self.whole_ask(true),
            WholeEvent::Stop => {
                self.whole_stop();
                self.say(kentos_interaction::Level::Info, words::STOPPED);
            }
            WholeEvent::Press(i) => {
                let (Some(doc), Some(r)) = (&self.document, self.features.whole.rows.get(i)) else {
                    return Task::none();
                };
                if let Some(slot) = r
                    .uid
                    .parse::<Uuid>()
                    .ok()
                    .and_then(|u| doc.model.slot_of(u))
                {
                    self.selection.set(vec![slot]);
                }
            }
            WholeEvent::Answered(ticket, more, answer) => {
                if ticket != self.features.whole.ticket {
                    return Task::none();
                }
                self.features.whole.running = None;
                match *answer {
                    Ok(page) => {
                        let p = &mut self.features.whole;
                        if !more {
                            p.total = page.total;
                            p.stamp = page.stamp.clone();
                        }
                        p.rows.extend(page.rows.iter().cloned());
                        p.last = Some(page);
                    }
                    // The file changed between pages: its first page again, said.
                    Err((code, _)) if code == "source_changed" && more => {
                        self.say(kentos_interaction::Level::Warn, words::CHANGED);
                        return self.whole_ask(false);
                    }
                    Err((code, _)) if code == "cancelled" => {}
                    Err((_, why)) => {
                        self.features.whole.problem = Some(why.clone());
                        self.say(kentos_interaction::Level::Warn, why);
                    }
                }
            }
        }
        Task::none()
    }

    fn whole_stop(&mut self) {
        if let Some(stop) = self.features.whole.running.take() {
            stop.store(true, Ordering::Relaxed);
        }
    }

    /// Kaynağın tümü or Çalışma kopyası on the source line.
    pub(crate) fn whole_switch<'a>(&self, layer: &str) -> Element<'a, Message> {
        let on = self.features.whole.layer.as_deref() == Some(layer);
        let (text, hint, style): (_, _, fn(&iced::Theme, button::Status) -> button::Style) = if on {
            (words::LOADED, words::LOADED_HINT, style::button::secondary)
        } else {
            (words::WHOLE, words::WHOLE_HINT, style::button::secondary)
        };
        tip(
            button(label::caption(text))
                .on_press(msg(WholeEvent::Toggle(!on)))
                .padding([2, 8])
                .style(style),
            Tip::new(text).body(hint),
            iced::widget::tooltip::Position::Top,
        )
    }

    /// The line over the rows: how many were read, who ordered them, where the filter was checked.
    fn whole_report(&self) -> String {
        let p = &self.features.whole;
        let mut out = vec![words::read(p.rows.len(), p.total)];
        if let Some(plan) = p.last.as_ref().map(|l| &l.plan) {
            out.push(
                match plan.order.as_str() {
                    "column" => words::BY_SOURCE,
                    "here" => words::BY_HERE,
                    _ => words::BY_KEY,
                }
                .to_owned(),
            );
            let filtered = plan.local.iter().any(|l| l == "filter");
            match (&plan.pushed, filtered && !plan.exact) {
                (Some(_), false) => out.push(words::FILTER_SENT.into()),
                (Some(_), true) => out.push(words::FILTER_BOTH.into()),
                (None, _) if filtered => out.push(words::FILTER_HERE.into()),
                _ => {}
            }
        }
        out.join(" · ")
    }

    /// The table on the source: the report, the rows and Sonraki sayfa.
    pub(crate) fn whole_table<'a>(&'a self, layer: &str) -> Element<'a, Message> {
        let p = &self.features.whole;
        let columns_shown = self.whole_columns(layer);
        let mut columns = vec![
            TableColumn::new("Sıra")
                .width(Length::Fixed(56.0))
                .align_right()
                .sortable(None, msg(WholeEvent::Sort(0))),
        ];
        for (j, c) in columns_shown.iter().enumerate() {
            let order = (p.sort.as_deref() == Some(c.as_str())).then_some(if p.descending {
                SortOrder::Descending
            } else {
                SortOrder::Ascending
            });
            columns.push(
                TableColumn::new(c.clone())
                    .width(Length::Fixed(140.0))
                    .sortable(order, msg(WholeEvent::Sort(j + 1))),
            );
        }
        let min_width = 56.0 + 148.0 * columns_shown.len() as f32;
        let held: Vec<bool> = {
            let doc = self.document.as_ref();
            p.rows
                .iter()
                .map(|r| {
                    doc.zip(r.uid.parse::<Uuid>().ok())
                        .is_some_and(|(d, u)| d.model.slot_of(u).is_some())
                })
                .collect()
        };
        let rows = &p.rows;
        let shown = columns_shown.clone();
        let table = Table::new(columns)
            .horizontal()
            .min_width(min_width)
            .virtualized(rows.len(), move |i| {
                let r = &rows[i];
                let mut cells: Vec<Element<'_, Message>> =
                    vec![label::muted((i + 1).to_string()).into()];
                for c in &shown {
                    let value = r
                        .attrs
                        .get(c)
                        .cloned()
                        .unwrap_or_else(|| texts::EMPTY.to_owned());
                    cells.push(if held[i] {
                        label::body(value).wrapping(Wrapping::None).into()
                    } else {
                        label::muted(value).wrapping(Wrapping::None).into()
                    });
                }
                TableLine::new(cells).on_press(msg(WholeEvent::Press(i)))
            })
            .empty(if p.running.is_some() {
                words::READING
            } else {
                words::NO_ROWS
            });
        let mut foot = row![label::muted(self.whole_report())]
            .spacing(8)
            .align_y(Center);
        if let Some(why) = &p.problem {
            foot = foot.push(label::body(why.clone()).style(style::text::warning));
        }
        foot = foot.push(space::horizontal());
        if p.running.is_some() {
            foot = foot.push(label::muted(words::READING)).push(
                button(label::caption(words::STOP))
                    .on_press(msg(WholeEvent::Stop))
                    .padding([2, 8])
                    .style(style::button::secondary),
            );
        } else if p.last.as_ref().is_some_and(|l| l.next.is_some()) {
            foot = foot.push(
                button(label::caption(words::MORE))
                    .on_press(msg(WholeEvent::More))
                    .padding([2, 8])
                    .style(style::button::secondary),
            );
        }
        let notice = tip(
            mouse_area(label::caption("ⓘ").style(style::text::muted)),
            Tip::new(words::WHOLE).body(words::NOT_HELD),
            iced::widget::tooltip::Position::Top,
        );
        column![
            container(row![foot, notice].spacing(8).align_y(Center))
                .padding([4, 10])
                .width(Fill),
            kentos_ui::widget::horizontal_divider(),
            table
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}

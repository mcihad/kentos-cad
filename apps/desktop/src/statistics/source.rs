//! İstatistik's Kapsam for a bound layer (docs/adr/0581 §6): Çalışma kopyası
//! counts the drawing's objects (the tab's own work), Kaynağın tümü asks the
//! layer's source for the chosen field's statistics over its every row
//! (`cad.sources.query` with `scope: source`), on a thread of its own with
//! Durdur; the drawing does not wait for it. The answer is kept for the layer
//! and the field it was asked for.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Task;
use iced::futures::channel::oneshot;
use kentos_commands::{CommandResult, QueryScope, SourcesQueried, SourcesQuery};
use kentos_interaction::Level;

use super::Event;
use crate::app::{App, Message};

/// The words.
pub mod texts {
    pub const SCOPE: &str = "Kapsam";
    pub const LOADED: &str = "Çalışma kopyası";
    pub const WHOLE: &str = "Kaynağın tümü";
    pub const SCOPE_HINT: &str = "Çalışma kopyası çizimdeki nesneleri sayar; Kaynağın tümü seçilen alanı bağlı \
         kaynağın bütün kayıtlarında, kaynağa sorarak sayar.";
    pub const READING: &str = "Kaynak okunuyor…";
    pub const STOP: &str = "Durdur";
    pub const STOPPED: &str = "Kaynağın istatistiği durduruldu.";
    pub const GEOMETRY: &str =
        "Geometriden gelen alanlar kaynağa sorulmaz; Çalışma kopyası kapsamında sayılır.";
    pub const RECORDS: &str = "Kayıt";
    pub const READ: &str = "Değer";
    pub const SKIPPED: &str = "Sayı değil";
    pub const SUM: &str = "Toplam";
    pub const MEAN: &str = "Ortalama";
    pub const MIN: &str = "En az";
    pub const MAX: &str = "En çok";
    pub const STD: &str = "Std. sapma";
    pub fn handed(n: u64) -> String {
        format!("Kaynak {n} satır verdi.")
    }
}

/// What Kaynağın tümü asked and was told.
#[derive(Default)]
pub(crate) struct Whole {
    /// Kapsam: Kaynağın tümü.
    pub on: bool,
    /// The layer and the field last asked (or being asked).
    pub asked: Option<(String, String)>,
    /// The question running: its stop.
    pub running: Option<Arc<AtomicBool>>,
    /// The last answer, for `asked`.
    pub answer: Option<Result<SourcesQueried, String>>,
}

impl App {
    /// The chosen layer's source, when it is bound to one.
    pub(crate) fn stats_bound(&self) -> Option<kentos_commands::SourceLink> {
        let doc = self.document.as_ref()?;
        let layer = self.chart_layer()?;
        doc.model
            .settings()
            .sources
            .iter()
            .find(|s| s.layer == layer)
            .cloned()
    }

    /// Kaynağın tümü's answer for the chosen layer and field, asked when it is not yet.
    pub(crate) fn stats_whole(&mut self) -> Task<Message> {
        if !self.charts.whole.on {
            return Task::none();
        }
        let (Some(link), Some(field), Some(doc)) = (
            self.stats_bound(),
            self.charts.field.clone(),
            self.document.as_ref(),
        ) else {
            return Task::none();
        };
        if field.starts_with('$') {
            return Task::none();
        }
        let want = (link.layer.clone(), field.clone());
        if self.charts.whole.asked.as_ref() == Some(&want) {
            return Task::none();
        }
        if let Some(stop) = self.charts.whole.running.take() {
            stop.store(true, Ordering::Relaxed);
        }
        let settings = doc.model.settings().clone();
        let revision = doc.model.revision().to_string();
        let stop = Arc::new(AtomicBool::new(false));
        self.charts.whole.running = Some(stop.clone());
        self.charts.whole.asked = Some(want.clone());
        self.charts.whole.answer = None;
        let input = SourcesQuery {
            layer: Some(link.layer.clone()),
            scope: Some(QueryScope::Source),
            fields: Some(vec![field.clone()]),
            limit: Some(0),
            count: Some(true),
            statistics: Some(vec![field]),
            ..SourcesQuery::default()
        };
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
                CommandResult::Failed { error } => Err(error.message),
                CommandResult::Cancelled => Err(texts::STOPPED.to_owned()),
                other => Err(format!("{other:?}")),
            });
        });
        Task::perform(wait, move |answer| {
            Message::Statistics(Event::WholeAnswered(
                want.0.clone(),
                want.1.clone(),
                Box::new(answer.unwrap_or_else(|_| Err(texts::STOPPED.to_owned()))),
            ))
        })
    }

    pub(crate) fn stats_whole_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Scope(on) => {
                self.charts.whole.on = on;
                if !on {
                    self.stats_whole_stop();
                }
                return self.stats_whole();
            }
            Event::WholeStop => {
                self.stats_whole_stop();
                self.say(Level::Info, texts::STOPPED);
            }
            Event::WholeAnswered(layer, field, answer) => {
                if self.charts.whole.asked.as_ref() == Some(&(layer, field)) {
                    self.charts.whole.running = None;
                    if let Err(why) = answer.as_ref() {
                        self.say(Level::Warn, why.clone());
                    }
                    self.charts.whole.answer = Some(*answer);
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn stats_whole_stop(&mut self) {
        if let Some(stop) = self.charts.whole.running.take() {
            stop.store(true, Ordering::Relaxed);
        }
        self.charts.whole.asked = None;
        self.charts.whole.answer = None;
    }
}

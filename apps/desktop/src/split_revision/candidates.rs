//! An explicit candidate search runs once on reading copies. Closing a revision or changing the drawing cancels it.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use iced::futures::channel::oneshot;
use iced::widget::{column, container, row};
use iced::{Element, Fill, Task};
use kentos_commands::{CommandResult, DrawingMatchSearch};
use kentos_native_application::drawing_compare::{self, MatchSuggestions};
use kentos_ui::widget::{
    Dialog as Frame,
    select::{Choice, Select},
    table::{self, Table},
};
use kentos_ui::{label, style};

use super::Revision;
use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::split_view;

type Token = (u64, u64, u64, u64);

// A cancelled search can still be finishing its current comparison. Its lease remains held until the worker
// exits, even if the view that owned it has gone. Rapid reopen/retry cannot accumulate background workers.
static SEARCH_ACTIVE: AtomicBool = AtomicBool::new(false);
struct SearchLease;
impl SearchLease {
    fn acquire() -> Option<Self> {
        SEARCH_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .ok()
            .map(|_| Self)
    }
}
impl Drop for SearchLease {
    fn drop(&mut self) {
        SEARCH_ACTIVE.store(false, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(usize),
    Radius(usize, usize),
    Start(usize),
    Stop(usize),
    Ready(Token, Result<MatchSuggestions, String>),
}

fn msg(event: Event) -> Message {
    Message::Split(split_view::Event::Candidates(event))
}

impl Revision {
    pub(super) fn clear_candidates(&mut self) {
        if let Some(stop) = self.candidate_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        self.candidate_epoch = self.candidate_epoch.wrapping_add(1);
        self.candidate_result = None;
    }

    fn token(&self) -> Option<Token> {
        let (session, generation) = self.for_doc?;
        Some((session, generation, self.doc.session, self.candidate_epoch))
    }
}

impl Drop for Revision {
    fn drop(&mut self) {
        if let Some(stop) = &self.candidate_stop {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

impl App {
    fn candidate_revision(&self, i: usize) -> Option<&Revision> {
        self.split.as_ref()?.others.get(i)?.revision.as_ref()
    }

    fn candidate_revision_mut(&mut self, i: usize) -> Option<&mut Revision> {
        self.split.as_mut()?.others.get_mut(i)?.revision.as_mut()
    }

    pub(crate) fn revision_candidates_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(i) => {
                if self.candidate_revision(i).is_some() {
                    self.dialog = Some(Dialog::RevisionMatches(i));
                }
            }
            Event::Radius(i, at) => {
                if let Some(r) = self.candidate_revision_mut(i) {
                    r.clear_candidates();
                    r.candidate_radius = [0.001, 0.01, 0.1].get(at).copied().unwrap_or(0.01);
                }
            }
            Event::Stop(i) => {
                if let Some(r) = self.candidate_revision(i)
                    && let Some(stop) = &r.candidate_stop
                {
                    stop.store(true, Ordering::Relaxed);
                }
            }
            Event::Ready(token, answer) => {
                let now = self
                    .document
                    .as_ref()
                    .map(|d| (d.session, d.model.generation()));
                if now != Some((token.0, token.1)) {
                    return Task::none();
                }
                if let Some(r) = self.split.as_mut().and_then(|s| {
                    s.others
                        .iter_mut()
                        .filter_map(|o| o.revision.as_mut())
                        .find(|r| r.token() == Some(token))
                }) {
                    r.candidate_stop = None;
                    r.candidate_result = Some(answer);
                }
            }
            Event::Start(i) => {
                let running = self.split.as_ref().is_some_and(|s| {
                    s.others.iter().any(|o| {
                        o.revision
                            .as_ref()
                            .is_some_and(|r| r.candidate_stop.is_some())
                    })
                });
                if running {
                    return Task::none();
                }
                let Some(lease) = SearchLease::acquire() else {
                    self.warn(
                        "Önceki aday araması sonlanıyor; tamamlandığında yeniden arayabilirsiniz.",
                    );
                    return Task::none();
                };
                let Some(now) = self.document.as_ref().map(|d| d.model.reading_copy()) else {
                    return Task::none();
                };
                let Some(r) = self.candidate_revision_mut(i) else {
                    return Task::none();
                };
                r.clear_candidates();
                let Some(token) = r.token() else {
                    return Task::none();
                };
                let old = r.doc.model.reading_copy();
                let stop = Arc::new(AtomicBool::new(false));
                r.candidate_stop = Some(stop.clone());
                let ask = DrawingMatchSearch {
                    search: r.candidate_radius,
                    key: None,
                    old_uids: None,
                    new_uids: None,
                };
                let (send, receive) = oneshot::channel();
                let worker = std::thread::Builder::new()
                    .name("cad-revision-candidates".into())
                    .spawn(move || {
                        let _lease = lease;
                        let answer =
                            match drawing_compare::suggest_matches(&old, &now, &ask, || {
                                stop.load(Ordering::Relaxed)
                            }) {
                                CommandResult::Completed { output, .. } => Ok(output),
                                CommandResult::Failed { error }
                                | CommandResult::Conflict { error } => Err(error.message),
                                CommandResult::Cancelled => Err("Arama durduruldu.".into()),
                                _ => Err("Aday araması tamamlanamadı.".into()),
                            };
                        let _ = send.send(answer);
                    });
                if let Err(error) = worker {
                    if let Some(r) = self.candidate_revision_mut(i) {
                        r.candidate_stop = None;
                        r.candidate_result =
                            Some(Err(format!("Aday araması başlatılamadı: {error}")));
                    }
                    return Task::none();
                }
                return Task::perform(receive, move |answer| {
                    msg(Event::Ready(
                        token,
                        answer.unwrap_or_else(|_| Err("Aday araması sonlandı.".into())),
                    ))
                });
            }
        }
        Task::none()
    }

    pub(crate) fn revision_candidates_view(&self, i: usize) -> Element<'_, Message> {
        let Some(r) = self.candidate_revision(i) else {
            return Frame::new("Eşleme adayları")
                .push(label::body("Revizyon görünümü kapandı."))
                .action(words::primary("Kapat", Some(Message::DialogClosed)))
                .modal(Message::DialogClosed);
        };
        let choices = [
            Choice::new("1 mm".to_owned()),
            Choice::new("10 mm".to_owned()),
            Choice::new("100 mm".to_owned()),
        ];
        let at = [0.001, 0.01, 0.1]
            .iter()
            .position(|v| *v == r.candidate_radius);
        let mut body = column![
            label::body("Kalıcı kimliği farklı nesneler için öneriler. Her iki sürümdeki eklenen ve silinen nesneler kapsam içindedir. Birden çok karşılığı olan satırlar belirsizdir; kimlik kararı için incelenmelidir."),
            label::caption("Geometri aynılığı 1 mm toleransla değerlendirilir."),
            row![label::caption("Arama uzaklığı"), container(Select::new(choices, at, move |at| msg(Event::Radius(i, at)))).width(150.0)].spacing(12),
        ].spacing(12);
        let running = r.candidate_stop.is_some();
        if running {
            body = body.push(label::body("Eşleme adayları aranıyor…"));
        }
        if let Some(answer) = &r.candidate_result {
            match answer {
                Err(why) => body = body.push(label::body(why.clone()).style(style::text::warning)),
                Ok(found) => {
                    let ambiguous = found.candidates.iter().filter(|c| c.ambiguous).count();
                    body = body.push(label::body(format!(
                        "{} öneri · {} belirsiz{}",
                        found.candidates.len(),
                        ambiguous,
                        if found.truncated {
                            " · Liste sınırlandı"
                        } else {
                            ""
                        }
                    )));
                    let columns = [
                        table::Column::new("Eski nesne").width(145.0),
                        table::Column::new("Yeni nesne").width(145.0),
                        table::Column::new("Uzaklık (mm)")
                            .width(100.0)
                            .align_right(),
                        table::Column::new("Geometri").width(105.0),
                        table::Column::new("Eşleme").width(105.0),
                    ];
                    let rows: Vec<table::Row<'_, Message>> = found
                        .candidates
                        .iter()
                        .map(|c| {
                            let old = object_label(&r.doc.model, &c.old_uid);
                            let new = self
                                .document
                                .as_ref()
                                .map_or_else(|| "—".into(), |d| object_label(&d.model, &c.new_uid));
                            table::Row::new(
                                [
                                    old,
                                    new,
                                    c.distance.map_or_else(
                                        || "—".into(),
                                        |d| format!("{:.3}", d * 1000.0),
                                    ),
                                    if c.same_geometry {
                                        "Aynı".into()
                                    } else {
                                        "Farklı".into()
                                    },
                                    if c.ambiguous {
                                        "Belirsiz".into()
                                    } else {
                                        "Tek aday".into()
                                    },
                                ]
                                .into_iter()
                                .map(|s| label::caption(s).into()),
                            )
                        })
                        .collect();
                    body = body.push(
                        container(
                            Table::new(columns)
                                .extend(rows)
                                .min_width(650.0)
                                .height(240.0),
                        )
                        .width(Fill)
                        .style(style::container::field_box),
                    );
                }
            }
        }
        Frame::new("Eşleme adayları")
            .push(body)
            .action(words::secondary("Kapat", Some(Message::DialogClosed)))
            .action(words::primary(
                if running { "Durdur" } else { "Adayları ara" },
                Some(msg(if running {
                    Event::Stop(i)
                } else {
                    Event::Start(i)
                })),
            ))
            .width(760.0)
            .modal(Message::DialogClosed)
    }
}

fn object_label(doc: &kentos_domain::Document, id: &str) -> String {
    let entity = kentos_domain::Uuid::parse_str(id)
        .ok()
        .and_then(|uid| doc.slot_of(uid))
        .and_then(|s| doc.get(s));
    entity.map_or_else(
        || id.chars().take(8).collect(),
        |e| {
            format!(
                "{} · #{}",
                match e.kind() {
                    "line" => "Çizgi",
                    "polyline" => "Çoklu çizgi",
                    "polygon" => "Alan",
                    "hatch" => "Tarama",
                    "insert" => "Blok",
                    "dimension" => "Ölçü",
                    "circle" => "Daire",
                    "text" => "Yazı",
                    _ => "Nesne",
                },
                e.base().id
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::split_view::Source;

    fn app() -> App {
        let mut app = crate::files_testing::app_with_drawing();
        let baseline = Document::of_model(app.document.as_ref().unwrap().model.clone());
        app.show_revision(0, "baseline".into(), baseline, Source::Drawing);
        app
    }

    #[test]
    fn only_one_candidate_worker_lease_can_exist_until_it_is_dropped() {
        let first = SearchLease::acquire().expect("first search");
        assert!(SearchLease::acquire().is_none());
        drop(first);
        assert!(SearchLease::acquire().is_some());
    }

    #[test]
    fn changed_scope_and_closed_revision_cancel_the_worker() {
        let mut app = app();
        let r = app.candidate_revision_mut(0).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        r.candidate_stop = Some(stop.clone());
        let stale = r.token().unwrap();
        let _ = app.revision_candidates_event(Event::Stop(0));
        assert!(stop.load(Ordering::Relaxed));
        stop.store(false, Ordering::Relaxed);
        app.candidate_revision_mut(0).unwrap().clear_candidates();
        assert!(stop.load(Ordering::Relaxed));
        let _ = app.revision_candidates_event(Event::Ready(
            stale,
            Ok(MatchSuggestions {
                candidates: vec![],
                truncated: false,
            }),
        ));
        assert!(
            app.candidate_revision(0)
                .unwrap()
                .candidate_result
                .is_none()
        );
        let stop = Arc::new(AtomicBool::new(false));
        app.candidate_revision_mut(0).unwrap().candidate_stop = Some(stop.clone());
        app.close_view(0);
        assert!(stop.load(Ordering::Relaxed));
    }

    #[test]
    fn result_uses_the_revision_identity_when_a_preceding_view_is_closed() {
        let mut app = app();
        app.add_view();
        let baseline = Document::of_model(app.document.as_ref().unwrap().model.clone());
        app.show_revision(1, "second".into(), baseline, Source::Drawing);
        let token = app.candidate_revision(1).unwrap().token().unwrap();
        app.dialog = Some(Dialog::RevisionMatches(1));
        app.close_view(0);
        assert!(matches!(app.dialog, Some(Dialog::RevisionMatches(0))));
        let _ = app.revision_candidates_event(Event::Ready(
            token,
            Ok(MatchSuggestions {
                candidates: vec![],
                truncated: false,
            }),
        ));
        assert!(
            app.candidate_revision(0)
                .unwrap()
                .candidate_result
                .is_some()
        );
    }
}

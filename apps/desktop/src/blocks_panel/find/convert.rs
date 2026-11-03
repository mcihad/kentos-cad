//! Review chosen repeats before one cancellable background conversion. The drawing remains unchanged
//! until the worker's result is committed against the same session/revision/generation.
use super::{Event as Parent, msg as parent_msg};
use crate::{
    app::{App, Dialog, Message},
    exchange::words::{self, Kind},
    traces::Control,
};
use iced::{
    Element, Subscription, Task,
    widget::{column, text, text_input},
};
use kentos_commands::{
    BlocksConvert, BlocksConvertPlan, BlocksConverted, CallEnvelope, CallOp, CommandResult,
};
use kentos_native_application::jobs::Job;
use kentos_ui::{label, widget::Dialog as Frame};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(super) struct Window {
    input: BlocksConvert,
    busy: bool,
    token: u64,
    cancel: Arc<AtomicBool>,
    plan: Option<BlocksConvertPlan>,
    refused: Option<String>,
    job: Option<Job>,
    details: bool,
}
impl std::fmt::Debug for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepeatConversion")
            .field("input", &self.input)
            .field("busy", &self.busy)
            .finish()
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(j) = &mut self.job {
            j.cancel();
        }
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Name(String),
    Review,
    Ready(u64, Box<CommandResult<BlocksConvertPlan>>),
    Apply,
    Tick,
    Copy,
    Details,
}
fn msg(e: Event) -> Message {
    parent_msg(Parent::Conversion(e))
}
impl App {
    pub(in crate::blocks_panel) fn block_convert_open(&mut self) -> Task<Message> {
        let (Some(doc), Some(w)) = (&self.document, self.blocks_panel.find.as_mut()) else {
            return Task::none();
        };
        if doc.session != w.session || doc.model.generation() != w.generation || w.busy {
            return Task::none();
        }
        let Some(output) = &w.output else {
            return Task::none();
        };
        let groups: Vec<_> = output
            .matches
            .iter()
            .zip(&w.chosen)
            .filter(|(_, on)| **on)
            .map(|(m, _)| m.members.clone())
            .collect();
        if groups.is_empty() {
            return Task::none();
        }
        let name =
            kentos_commands::blocks::free_name(doc.model.blocks().iter().map(|b| b.name.as_str()));
        w.conversion = Some(Window {
            input: BlocksConvert {
                name,
                source: output.source.clone(),
                groups,
                rotations: Some(w.rotations),
                mirrors: Some(w.mirrors),
                scales: Some(w.scales),
                tolerance: Some(w.tolerance),
                expected_revision: Some(w.revision.clone()),
            },
            busy: false,
            token: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            plan: None,
            refused: None,
            job: None,
            details: false,
        });
        self.dialog = Some(Dialog::BlockFind);
        self.block_convert_event(Event::Review)
    }
    pub(crate) fn block_convert_subscription(&self) -> Subscription<Message> {
        if self
            .blocks_panel
            .find
            .as_ref()
            .and_then(|w| w.conversion.as_ref())
            .is_some_and(|w| w.job.is_some())
        {
            iced::window::frames().map(|_| msg(Event::Tick))
        } else {
            Subscription::none()
        }
    }
    pub(in crate::blocks_panel) fn block_convert_event(&mut self, event: Event) -> Task<Message> {
        let (Some(doc), Some(parent)) = (self.document.as_mut(), self.blocks_panel.find.as_mut())
        else {
            return Task::none();
        };
        let current = doc.session == parent.session && doc.model.generation() == parent.generation;
        let Some(w) = parent.conversion.as_mut() else {
            return Task::none();
        };
        if !current {
            w.cancel.store(true, Ordering::Relaxed);
            if let Some(j) = &mut w.job {
                j.cancel();
            }
            w.job = None;
            w.busy = false;
            w.plan = None;
            w.refused = Some(
                "Çizim değişti. Eski inceleme uygulanmadı; pencereyi kapatıp aramayı yenileyin."
                    .into(),
            );
            return Task::none();
        }
        match event {
            Event::Name(name) if !w.busy => {
                w.input.name = name;
                w.plan = None;
                w.refused = None;
            }
            Event::Review if !w.busy && !w.input.name.trim().is_empty() => {
                w.cancel.store(true, Ordering::Relaxed);
                w.cancel = Arc::new(AtomicBool::new(false));
                w.token = super::NEXT.fetch_add(1, Ordering::Relaxed);
                w.busy = true;
                w.plan = None;
                w.refused = None;
                let token = w.token;
                let input = w.input.clone();
                let model = doc.model.clone();
                let cancel = Arc::clone(&w.cancel);
                return crate::db_connections::off_thread(
                    move || {
                        kentos_native_application::blocks_convert::inspect(&model, &input, || {
                            cancel.load(Ordering::Relaxed)
                        })
                    },
                    move |result| msg(Event::Ready(token, Box::new(result))),
                );
            }
            Event::Ready(token, result)
                if token == w.token && !w.cancel.load(Ordering::Relaxed) =>
            {
                w.busy = false;
                match *result {
                    CommandResult::Completed { output, .. } => w.plan = Some(output),
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => w.refused = Some(error.message),
                    _ => w.refused = Some("İnceleme durduruldu; çizim değiştirilmedi.".into()),
                }
            }
            Event::Details => w.details = !w.details,
            Event::Copy => {
                if let Some(plan) = &w.plan {
                    return iced::clipboard::write(
                        serde_json::to_string_pretty(plan).expect("conversion plan"),
                    );
                }
            }
            Event::Apply if !w.busy && w.plan.is_some() => {
                w.busy = true;
                w.refused = None;
                w.job = Some(Job::start(
                    format!("repeat-convert-{}", w.token),
                    &doc.model,
                    CallEnvelope {
                        command: kentos_commands::CAD_BLOCKS_CONVERT.into(),
                        version: None,
                        op: CallOp::Execute,
                        input: serde_json::to_value(&w.input).expect("conversion input"),
                        request_id: None,
                        expected: None,
                        relax_constraints: false,
                    },
                ));
            }
            Event::Tick => {
                let Some(job) = &mut w.job else {
                    return Task::none();
                };
                let status = job.status(&mut doc.model);
                if !status.phase.finished() {
                    return Task::none();
                }
                w.job = None;
                w.busy = false;
                let Some(answer) = status.answer else {
                    return Task::none();
                };
                match answer.result {
                    CommandResult::Completed { output, .. } => {
                        let Ok(output) = serde_json::from_value::<BlocksConverted>(output) else {
                            w.refused =
                                Some("Dönüşüm cevabı okunamadı. Geçmişi kontrol edin.".into());
                            return Task::none();
                        };
                        let slots: Vec<_> = output
                            .inserts
                            .iter()
                            .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                            .filter_map(|u| doc.model.slot_of(u))
                            .collect();
                        self.selection.set(slots);
                        self.close_block_find();
                        self.say(kentos_interaction::Level::Success,format!("{} yerleştirme oluşturuldu; {} ölçü bağı korundu. Tek adımda geri alınır.",output.inserts.len(),output.dimensions.len()));
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => w.refused = Some(error.message),
                    _ => w.refused = Some("Dönüşüm durduruldu; çizim değiştirilmedi.".into()),
                }
            }
            _ => {}
        }
        Task::none()
    }
    pub(in crate::blocks_panel) fn block_convert_view(&self) -> Element<'_, Message> {
        let Some(parent) = self.blocks_panel.find.as_ref() else {
            return text("").into();
        };
        let Some(w) = parent.conversion.as_ref() else {
            return text("").into();
        };
        let current = parent.current(self);
        let mut body = column![
            label::body(format!(
                "{} aday · Her aday {} parça",
                w.input.groups.len(),
                w.input.source.len()
            )),
            words::field(
                "Yeni blok adı",
                text_input("Blok adı", &w.input.name)
                    .on_input_maybe((!w.busy && current).then_some(|s| msg(Event::Name(s)))),
                None
            ),
            label::caption(format!(
                "Eşleşme toleransı: {} mm. Proje birimi ve çizim ölçeği korunur.",
                format!(
                    "{:.6}",
                    w.input
                        .tolerance
                        .unwrap_or(kentos_commands::RECOGNITION_TOLERANCE)
                        * 1000.
                )
                .trim_end_matches('0')
                .trim_end_matches('.')
                .replace('.', ",")
            ))
        ]
        .spacing(12);
        if !current {
            body = body.push(words::text_line(
                Kind::Warn,
                "Çizim değişti. Kapatıp aramayı yenileyin.",
            ));
        }
        if let Some(why) = &w.refused {
            body = body.push(words::text_line(Kind::Error, why.clone()));
        }
        if w.busy {
            body = body.push(label::body(if w.job.is_some() {
                "Dönüştürülüyor… Durdur ile yazılmadan iptal edebilirsiniz."
            } else {
                "Geometri, çizim sırası ve ilişkiler denetleniyor…"
            }));
        }
        if let Some(plan) = &w.plan {
            body=body.push(words::text_line(Kind::Ok,format!("{} aday tek bir blok tanımını kullanacak.",plan.placements.len())))
                .push(label::body(format!("{} çizim parçası → {} yerleştirme. {} ölçü noktası bağlı kalacak.",plan.mapping.len(),plan.placements.len(),plan.dimensions.len())))
                .push(label::caption("Katman, görünüş ve ilişkiler doğrulandı. Her adayın ilk nesne kimliği korunur; diğer parçalar tanımın içinde olur. Tek adımda geri alabilirsiniz."));
            let max = plan
                .dimensions
                .iter()
                .map(|d| d.deviation)
                .fold(0.0_f64, f64::max);
            if !plan.dimensions.is_empty() {
                body = body.push(label::caption(if max == 0. {
                    "Ölçü noktaları yer değiştirmeyecek.".to_owned()
                } else {
                    format!("En büyük ölçü noktası sapması: {:.6} mm", max * 1000.)
                        .replace('.', ",")
                }));
            }
            body = body.push(words::secondary(
                if w.details {
                    "Parça eşlemesini gizle"
                } else {
                    "Parça eşlemesini göster"
                },
                (!w.busy).then(|| msg(Event::Details)),
            ));
            if w.details {
                for map in plan.mapping.iter().take(8) {
                    let slot = |u: &str| {
                        self.document
                            .as_ref()
                            .and_then(|d| {
                                kentos_domain::Uuid::parse_str(u)
                                    .ok()
                                    .and_then(|u| d.model.slot_of(u))
                            })
                            .map_or(0, |s| s.0)
                    };
                    body = body.push(label::caption(format!(
                        "#{} → #{} / Parça {}",
                        slot(&map.from),
                        slot(&map.insert),
                        map.path[0] + 1
                    )));
                }
                if plan.mapping.len() > 8 {
                    body = body.push(label::caption(format!(
                        "{} eşlemenin tamamı inceleme kaydındadır.",
                        plan.mapping.len()
                    )));
                }
                body = body.push(words::secondary(
                    "İnceleme kaydını kopyala",
                    (!w.busy).then(|| msg(Event::Copy)),
                ));
            }
        }
        Frame::new("Tekrarları bloğa dönüştür")
            .scroll(body)
            .action(words::secondary(
                if w.busy { "Durdur" } else { "Vazgeç" },
                Some(parent_msg(Parent::CloseOptions)),
            ))
            .action(if w.plan.is_some() {
                words::primary("Dönüştür", (current && !w.busy).then(|| msg(Event::Apply)))
            } else {
                words::primary(
                    "Dönüşümü incele",
                    (current && !w.busy && !w.input.name.trim().is_empty())
                        .then(|| msg(Event::Review)),
                )
            })
            .width(660.)
            .max_height(700.)
            .modal(parent_msg(Parent::CloseOptions))
    }
    pub(in crate::blocks_panel) fn block_convert_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let event = match control {
            Control::Press("Vazgeç" | "Durdur") => {
                return Ok(Some(parent_msg(Parent::CloseOptions)));
            }
            Control::Press("Parça eşlemesini göster" | "Parça eşlemesini gizle") => {
                Event::Details
            }
            Control::Press("İnceleme kaydını kopyala") => Event::Copy,
            Control::Press("Dönüşümü incele") => Event::Review,
            Control::Press("Dönüştür") => Event::Apply,
            Control::Fill("Yeni blok adı", name) => Event::Name(name.into()),
            _ => return Ok(None),
        };
        Ok(Some(msg(event)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ready() -> App {
        let mut app = super::super::tests::test_app();
        app.open_block_find();
        super::super::tests::complete(&mut app);
        let _ = app.block_convert_open();
        finish_review(&mut app);
        app
    }
    fn finish_review(app: &mut App) {
        let w = app
            .blocks_panel
            .find
            .as_ref()
            .unwrap()
            .conversion
            .as_ref()
            .unwrap();
        let token = w.token;
        let input = w.input.clone();
        let result = kentos_native_application::blocks_convert::inspect(
            &app.document.as_ref().unwrap().model,
            &input,
            || false,
        );
        let _ = app.block_convert_event(Event::Ready(token, Box::new(result)));
    }
    #[test]
    fn review_details_and_editing_name_do_not_write_and_old_plan_cannot_apply() {
        let mut app = ready();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let w = app
            .blocks_panel
            .find
            .as_ref()
            .unwrap()
            .conversion
            .as_ref()
            .unwrap();
        assert!(w.plan.is_some());
        assert!(!w.details);
        let _ = app.block_convert_event(Event::Details);
        assert!(
            app.blocks_panel
                .find
                .as_ref()
                .unwrap()
                .conversion
                .as_ref()
                .unwrap()
                .details
        );
        let _ = app.block_convert_event(Event::Name("Başka ad".into()));
        let _ = app.block_convert_event(Event::Apply);
        let w = app
            .blocks_panel
            .find
            .as_ref()
            .unwrap()
            .conversion
            .as_ref()
            .unwrap();
        assert!(w.plan.is_none());
        assert!(w.job.is_none());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }
    #[test]
    fn closing_review_and_pending_write_cancels_the_worker_without_committing_or_late_results() {
        let mut app = ready();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let w = app
            .blocks_panel
            .find
            .as_ref()
            .unwrap()
            .conversion
            .as_ref()
            .unwrap();
        let cancel = Arc::clone(&w.cancel);
        let token = w.token;
        let result = kentos_native_application::blocks_convert::inspect(
            &app.document.as_ref().unwrap().model,
            &w.input,
            || false,
        );
        let _ = app.block_convert_event(Event::Apply);
        assert!(
            app.blocks_panel
                .find
                .as_ref()
                .unwrap()
                .conversion
                .as_ref()
                .unwrap()
                .job
                .is_some()
        );
        app.block_find_close_options();
        assert!(cancel.load(Ordering::Relaxed));
        let _ = app.block_convert_event(Event::Ready(token, Box::new(result)));
        assert!(app.blocks_panel.find.as_ref().unwrap().conversion.is_none());
        assert_eq!(app.dialog, None);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }
    #[test]
    fn changed_drawing_refuses_the_previously_reviewed_conversion() {
        let mut app = ready();
        let doc = &mut app.document.as_mut().unwrap().model;
        let mut changed = doc.get(kentos_domain::Slot(1)).unwrap().clone();
        if let kentos_commands::Entity::Line(l) = &mut changed {
            l.b.x += 0.1;
        }
        doc.update(kentos_domain::Slot(1), changed);
        let before = doc.to_snapshot_v2();
        let _ = app.block_convert_event(Event::Apply);
        let w = app
            .blocks_panel
            .find
            .as_ref()
            .unwrap()
            .conversion
            .as_ref()
            .unwrap();
        assert!(w.plan.is_none());
        assert!(w.job.is_none());
        assert!(w.refused.as_ref().unwrap().contains("Çizim değişti"));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }
    #[test]
    fn write_job_commits_once_and_selects_retained_insert_identities() {
        let mut app = ready();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.block_convert_event(Event::Apply);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while app.blocks_panel.find.is_some() && std::time::Instant::now() < deadline {
            let _ = app.block_convert_event(Event::Tick);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(app.blocks_panel.find.is_none());
        assert_eq!(app.dialog, None);
        assert_eq!(
            app.selection.ids(),
            [kentos_domain::Slot(1), kentos_domain::Slot(2)]
        );
        let doc = &mut app.document.as_mut().unwrap().model;
        assert_eq!(doc.blocks().len(), 1);
        assert_eq!(doc.undo_labels().len(), 1);
        assert_eq!(
            doc.undo().as_deref(),
            Some(kentos_native_application::blocks_convert::LABEL)
        );
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

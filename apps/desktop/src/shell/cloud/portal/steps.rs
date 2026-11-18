//! A dossier's steps in Portal dosyaları (docs/adr/9001 §4): the buttons are the steps the server allows the reader
//! now; each asks its question before it is taken, as the web portal's do (a refusal and a deficiency list need their
//! words, a decision is said to be the portal's record and not the official letter), and goes with the status the
//! person saw: another's step in between is the server's conflict, never an overwrite. A KentOS project file of the
//! dossier is opened as a new cloud project in the reader's space, then in the drawing.

use iced::Task;
use iced::widget::text_editor;
use kentos_cloud::ApiFailure;
use kentos_commands::{
    DossierAction, DossierActionRequest, DossierDetail, DossierFileImport, DossierFileImported,
    DossierSide, DossierStatus, ProjectStorage,
};
use kentos_domain::Uuid;

use super::{Event, Said, msg, plan};
use crate::app::{App, Dialog, Message, Then};

/// What a question is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    Step(DossierAction),
    /// A KentOS project file to open as a project: its id and name.
    Import {
        file: String,
        name: String,
    },
}

/// A question over the window, with what is typed into it.
pub struct Asking {
    pub question: Question,
    pub note: text_editor::Content,
    /// Sorumlu ata's choice, among the dossier's assignees.
    pub assignee: Option<usize>,
    /// What the note field is missing, said under it.
    pub missing: Option<String>,
    /// Why the server refused it.
    pub error: Option<String>,
    /// Its request on its way: the question waits for the answer.
    pub busy: bool,
    /// The dossier it is about and the status the person saw (the step's `expectedStatus`).
    pub dossier: String,
    pub seen: DossierStatus,
}

/// A project a file of the dossier became.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Made {
    pub tenant: Uuid,
    pub project: Uuid,
    pub name: String,
}

impl App {
    pub(super) fn portal_step_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Step(action) => self.portal_ask(action),
            Event::AskNote(action) => {
                if let Some(a) = self.portal_asking().filter(|a| !a.busy) {
                    a.note.perform(action);
                    a.missing = None;
                    a.error = None;
                }
                Task::none()
            }
            Event::AskAssignee(i) => {
                if let Some(a) = self.portal_asking().filter(|a| !a.busy) {
                    a.assignee = Some(i);
                    a.error = None;
                }
                Task::none()
            }
            Event::AskConfirm => self.portal_confirm(),
            Event::AskClose => {
                self.close_portal();
                Task::none()
            }
            Event::Acted { id, result } => self.portal_acted(id, result),
            Event::Import(file) => {
                let Some(p) = self.cloud.portal.as_mut() else {
                    return Task::none();
                };
                let Some(d) = p.chosen_detail() else {
                    return Task::none();
                };
                let Some(f) = super::package::all_files(d)
                    .find(|f| f.id == file)
                    .filter(|f| plan::importable(d, f))
                else {
                    return Task::none();
                };
                let question = Question::Import {
                    file: f.id.clone(),
                    name: f.name.clone(),
                };
                let (dossier, seen) = (d.summary.id.clone(), d.summary.status);
                p.asking = Some(asking(question, dossier, seen));
                Task::none()
            }
            Event::Imported { id, result } => self.portal_imported(id, result),
            Event::OpenMade => {
                let Some(made) = self.cloud.portal.as_mut().and_then(|p| p.made.take()) else {
                    return Task::none();
                };
                self.portal_open_made(made)
            }
            _ => Task::none(),
        }
    }

    fn portal_asking(&mut self) -> Option<&mut Asking> {
        self.cloud.portal.as_mut()?.asking.as_mut()
    }

    /// A step's button: İncelemeye al goes at once, the others ask first.
    fn portal_ask(&mut self, action: DossierAction) -> Task<Message> {
        let Some(p) = self.cloud.portal.as_mut().filter(|p| !p.working()) else {
            return Task::none();
        };
        let Some(d) = p.chosen_detail().filter(|d| d.actions.contains(&action)) else {
            return Task::none();
        };
        let (dossier, seen) = (d.summary.id.clone(), d.summary.status);
        p.asking = Some(asking(Question::Step(action), dossier, seen));
        if action == DossierAction::StartReview {
            return self.portal_confirm();
        }
        Task::none()
    }

    /// The question's confirming button: the step with its note, or the file opened as a project.
    fn portal_confirm(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let Some(p) = self.cloud.portal.as_mut() else {
            return Task::none();
        };
        let Some(d) = p.detail.clone() else {
            return Task::none();
        };
        let Some(a) = p
            .asking
            .as_mut()
            .filter(|a| !a.busy && a.dossier == d.summary.id)
        else {
            return Task::none();
        };
        let Some(dossier) = crate::cloud::uuid(&a.dossier) else {
            return Task::none();
        };
        let note = a.note.text().trim().to_owned();
        match a.question.clone() {
            Question::Step(action) => {
                let ask = plan::ask_of(action, &d);
                if let Some((plan::Note::Required, label)) = ask.as_ref().and_then(|q| q.note)
                    && note.is_empty()
                {
                    a.missing = Some(plan::note_missing(label));
                    return Task::none();
                }
                let assignee = match action {
                    DossierAction::Assign => {
                        let Some(who) = a.assignee.and_then(|i| d.assignees.get(i)) else {
                            a.error = Some(
                                "Bir sorumlu seçin; listede dosyalara bakabilen üyeler var."
                                    .to_owned(),
                            );
                            return Task::none();
                        };
                        Some(who.user_id.clone())
                    }
                    _ => None,
                };
                a.busy = true;
                a.error = None;
                p.busy = Some(id);
                let step = DossierActionRequest {
                    action,
                    expected_status: a.seen,
                    note: (!note.is_empty()).then_some(note),
                    assignee_id: assignee,
                    attachments: Vec::new(),
                };
                Task::perform(client.dossier_step(dossier, step), move |result| {
                    msg(Event::Acted {
                        id,
                        result: result.map(Box::new),
                    })
                })
            }
            Question::Import { file, .. } => {
                let Some(file) = crate::cloud::uuid(&file) else {
                    return Task::none();
                };
                // A member opens it in the organisation's space, a professional in their own.
                let into = DossierFileImport {
                    tenant_id: (d.side == DossierSide::Municipality)
                        .then(|| d.organization.tenant_id.clone()),
                };
                a.busy = true;
                a.error = None;
                p.busy = Some(id);
                Task::perform(client.dossier_import(dossier, file, into), move |result| {
                    msg(Event::Imported { id, result })
                })
            }
        }
    }

    /// A step's answer: the dossier as it is now, said; or the server's refusal in the question.
    fn portal_acted(
        &mut self,
        id: u64,
        result: Result<Box<DossierDetail>, ApiFailure>,
    ) -> Task<Message> {
        let Some(p) = self.cloud.portal.as_mut().filter(|p| p.busy == Some(id)) else {
            return Task::none();
        };
        p.busy = None;
        match result {
            Ok(d) => {
                let done = match p.asking.take().map(|a| a.question) {
                    Some(Question::Step(action)) => plan::ask_of(action, &d)
                        .map(|q| q.done.to_owned())
                        .unwrap_or_else(|| done_at_once(action).to_owned()),
                    _ => String::new(),
                };
                if p.chosen.as_deref() == Some(d.summary.id.as_str()) {
                    p.detail = Some(*d);
                }
                if !done.is_empty() {
                    p.said = Some(Said::Done(done));
                }
                // Its row in the list says the new status.
                self.portal_list()
            }
            Err(f) => {
                let conflict = f.status == 409;
                match p.asking.as_mut() {
                    Some(a) => {
                        a.busy = false;
                        a.error = Some(refusal(&f));
                    }
                    None => p.said = Some(Said::Error(refusal(&f))),
                }
                // Another's step came first: the page is read again, so that what is asked next is what is there.
                if conflict {
                    return Task::batch([self.portal_list(), self.portal_detail()]);
                }
                Task::none()
            }
        }
    }

    /// A file became a project: it opens in the drawing (the drawing on screen asked about first, leaving.rs).
    fn portal_imported(
        &mut self,
        id: u64,
        result: Result<DossierFileImported, ApiFailure>,
    ) -> Task<Message> {
        let Some(p) = self.cloud.portal.as_mut().filter(|p| p.busy == Some(id)) else {
            return Task::none();
        };
        p.busy = None;
        match result {
            Ok(made) => {
                p.asking = None;
                let (Some(tenant), Some(project)) = (
                    crate::cloud::uuid(&made.tenant_id),
                    crate::cloud::uuid(&made.project_id),
                ) else {
                    p.said = Some(Said::Error(
                        "Proje açıldı ama sunucunun verdiği kimlik okunamadı; Bulut projeleri'nden açın.".to_owned(),
                    ));
                    return Task::none();
                };
                p.said = Some(Said::Done(format!(
                    "“{}” projesi oluşturuldu; çizim alanında açılıyor.",
                    made.name
                )));
                self.portal_open_made(Made {
                    tenant,
                    project,
                    name: made.name,
                })
            }
            Err(f) => {
                if let Some(a) = p.asking.as_mut() {
                    a.busy = false;
                    a.error = Some(refusal(&f));
                }
                Task::none()
            }
        }
    }

    /// Opens the project a file became; the window steps aside for the open's progress and comes back when the
    /// drawing on screen is not left (leaving.rs).
    fn portal_open_made(&mut self, made: Made) -> Task<Message> {
        if let Some(p) = self.cloud.portal.as_mut() {
            p.made = Some(made.clone());
        }
        self.cloud.open_hint = Some((made.name, ProjectStorage::File));
        if self.dialog == Some(Dialog::Portal) {
            self.dialog = None;
        }
        self.leave(Then::OpenCloud {
            tenant: made.tenant,
            project: made.project,
        })
    }
}

fn asking(question: Question, dossier: String, seen: DossierStatus) -> Asking {
    Asking {
        question,
        note: text_editor::Content::new(),
        assignee: None,
        missing: None,
        error: None,
        busy: false,
        dossier,
        seen,
    }
}

/// What is said of a step that asks no question.
fn done_at_once(action: DossierAction) -> &'static str {
    match action {
        DossierAction::StartReview => "Başvuru incelemeye alındı.",
        DossierAction::Assign => "Sorumlu atandı.",
        _ => "Adım atıldı.",
    }
}

/// A refusal in the server's own words (a stale step's says what the dossier is now); no answer says how to go on.
pub(super) fn refusal(f: &ApiFailure) -> String {
    if f.message.trim().is_empty() {
        "İşlem tamamlanamadı; bağlantınızı denetleyip yeniden deneyin.".to_owned()
    } else {
        f.message.clone()
    }
}

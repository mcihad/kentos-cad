//! A dossier's messages in Portal dosyaları (docs/adr/9001 §6): the box to write to the other side while the dossier is
//! open, the message sent and the box emptied, or the refusal said and what was written kept.

use iced::Task;
use iced::widget::text_editor;
use kentos_cloud::ApiFailure;
use kentos_commands::{DossierMessageView, MessagePost};

use super::steps::refusal;
use super::{Event, Said, msg};
use crate::app::{App, Message};

impl App {
    pub(super) fn portal_message_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Draft(action) => {
                if let Some(p) = self.cloud.portal.as_mut().filter(|p| p.busy.is_none()) {
                    p.draft.perform(action);
                }
                Task::none()
            }
            Event::Send => self.portal_send(),
            Event::Sent { id, result } => self.portal_sent(id, result),
            _ => Task::none(),
        }
    }

    /// Gönder: the message goes to the other side.
    fn portal_send(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let Some(p) = self.cloud.portal.as_mut().filter(|p| !p.working()) else {
            return Task::none();
        };
        let body = p.draft.text().trim().to_owned();
        let Some(dossier) = p
            .chosen_detail()
            .filter(|d| d.can_message)
            .and_then(|d| crate::cloud::uuid(&d.summary.id))
        else {
            return Task::none();
        };
        if body.is_empty() {
            p.said = Some(Said::Error(
                "İleti boş; göndermek için önce yazın.".to_owned(),
            ));
            return Task::none();
        }
        p.busy = Some(id);
        let post = MessagePost {
            body,
            attachments: Vec::new(),
        };
        Task::perform(client.dossier_message(dossier, post), move |result| {
            msg(Event::Sent { id, result })
        })
    }

    fn portal_sent(
        &mut self,
        id: u64,
        result: Result<DossierMessageView, ApiFailure>,
    ) -> Task<Message> {
        let Some(p) = self.cloud.portal.as_mut().filter(|p| p.busy == Some(id)) else {
            return Task::none();
        };
        p.busy = None;
        match result {
            Ok(_) => {
                p.draft = text_editor::Content::new();
                p.said = Some(Said::Done("İleti gönderildi.".to_owned()));
                self.portal_detail()
            }
            Err(f) => {
                p.said = Some(Said::Error(format!(
                    "İleti gönderilemedi: {} Yazdığınız ileti duruyor; yeniden deneyin.",
                    refusal(&f)
                )));
                Task::none()
            }
        }
    }
}

//! Saving a file project (docs/adr/0041, 0031, 0040, 0043): Kaydet writes
//! the drawing into verified `.kcad` v2 bytes in the save worker
//! (saving.rs), then kentos-cloud uploads them and commits them as the
//! project's next revision, based on the revision the drawing stands on.
//!
//! - Without a connection the save is kept in the project's local copy
//!   (“Kaydedildi (bu cihazda) · gönderilecek”) and goes when the
//!   connection returns; a save the server does not answer is kept the same
//!   way. After a save reaches the server the copy holds the new revision.
//! - When someone else saved meanwhile nothing is written, and a window
//!   offers the choices: open the server's newest revision (after a
//!   question: the changes here are dropped), save the drawing as a
//!   separate file project “… (kopya)”, or leave it for now. A save kept on
//!   this device is never dropped without that choice.

use std::sync::Arc;

use iced::Task;
use iced::futures::channel::mpsc;
use iced::futures::stream;
use kentos_cloud::{ApiFailure, Opened, Revision, Source as Kept, conflicting_revision};
use kentos_contracts::{FileCommitted, ProjectStorage};

use crate::app::{App, Dialog, Message, Then};
use crate::cloud::{Event, Once};
use crate::saving::{self, Stage, Target};

/// A file revision refused because another one came first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileConflict {
    /// The drawing it belongs to.
    pub session: u64,
    /// The server's newest revision, and the one the drawing stands on.
    pub server: u64,
    pub based_on: u64,
}

impl App {
    /// Kaydet on a cloud project: a file project's next revision; a
    /// database project's changes go at once (its autosave).
    pub(crate) fn save_cloud(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(source) = doc.cloud_source() else {
            return Task::none();
        };
        if source.storage() == ProjectStorage::Database {
            return self.send_now();
        }
        let name = source.info.name.clone();
        if !source.can_write() {
            let text = if source.archived() {
                format!(
                    "“{name}” arşivlenmiş: salt okunurdur, buluta kaydedilmez. Çizimi saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
                )
            } else {
                format!(
                    "“{name}” projesinde kaydetme yetkiniz yok (feature.write). Çizimi saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
                )
            };
            self.warn(text);
            return Task::none();
        }
        if self.cloud.signed_in().is_none() && self.cloud.held.is_none() {
            self.warn("Bulut oturumu açık değil; kaydetmek için önce giriş yapın (Buluta giriş).");
            return Task::none();
        }
        if !doc.dirty() && source.revision.is_some() {
            self.output(format!(
                "“{name}” buluta kaydedildi; kaydedilecek değişiklik yok."
            ));
            return Task::none();
        }
        let target = Target::Cloud {
            tenant: source.tenant,
            project: source.project,
            based_on: source.revision.as_ref().map_or(0, |r| r.number),
            name,
        };
        self.start_save(target)
    }

    /// The drawing's bytes are ready: they go up as the next revision, or,
    /// without a connection, into this device's copy.
    pub(crate) fn upload_revision(&mut self, id: u64, bytes: Once<Vec<u8>>) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let keeps = self.cloud.held.is_some();
        let Some(s) = self.saving.as_mut().filter(|s| s.id == id) else {
            return Task::none();
        };
        let Target::Cloud {
            tenant,
            project,
            based_on,
            ..
        } = s.target.clone()
        else {
            return Task::none();
        };
        let Some(bytes) = bytes.take() else {
            return Task::none();
        };
        let Some(client) = client else {
            return self.keep_on_device(id, bytes, based_on);
        };
        (s.stage, s.fraction) = Stage::Uploading {
            done: 0,
            total: bytes.len() as u64,
        }
        .describe();
        // Kept for this device's copy, should the server not answer.
        if keeps {
            s.bytes = Some(bytes.clone());
        }
        // In parts; a cut connection goes on from what the server has (docs/adr/0045).
        let (tell, told) = mpsc::unbounded();
        let progress: kentos_cloud::Progress = Arc::new(move |done, total| {
            let stage = Stage::Uploading { done, total };
            let _ = tell.unbounded_send(Message::Saving(saving::Event::Progress { id, stage }));
        });
        let save = kentos_cloud::saving::save_revision_watched(
            &client,
            tenant,
            project,
            bytes,
            based_on,
            kentos_cloud::saving::PART,
            Some(progress),
        );
        let done = stream::once(async move {
            crate::cloud::msg(Event::FileSaved {
                id,
                result: save.await,
            })
        });
        let (task, handle) = Task::stream(stream::select(told, done)).abortable();
        s.upload = Some(handle.abort_on_drop());
        task
    }

    /// Keeps a save in the project's copy until the connection returns.
    fn keep_on_device(&mut self, id: u64, bytes: Vec<u8>, based_on: u64) -> Task<Message> {
        let Some(s) = self.saving.take_if(|s| s.id == id) else {
            return Task::none();
        };
        let name = match &s.target {
            Target::Cloud { name, .. } => name.clone(),
            Target::File(_) => return Task::none(),
        };
        let same = self
            .document
            .as_ref()
            .is_some_and(|d| d.session == s.session);
        let Some(held) = self.cloud.held.as_mut().filter(|h| h.session == s.session) else {
            self.say(
                kentos_interaction::Level::Error,
                format!("“{name}” kaydedilemedi: sunucuya ulaşılamıyor ve projenin bu cihazda kopyası yok. Çizim kaydedilmemiş sayılıyor."),
            );
            return Task::none();
        };
        match held.replica.keep_save(&bytes, based_on) {
            Ok(()) => {
                held.kept_save = true;
                if same && let Some(doc) = self.document.as_mut() {
                    doc.model.mark_saved(s.revision);
                }
                self.warn(format!(
                    "“{name}” bu cihazda kaydedildi; sunucuya ulaşılamadı. Bağlantı gelince revizyon {based_on} üstüne gönderilecek."
                ));
                if same {
                    self.recovery_saved();
                }
            }
            Err(e) => self.say(
                kentos_interaction::Level::Error,
                format!("“{name}” kaydedilemedi: {e} Çizim kaydedilmemiş sayılıyor."),
            ),
        }
        Task::none()
    }

    pub(crate) fn file_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::FileSaved { id, result } => return self.file_saved(id, result),
            Event::KeptSent {
                session,
                based_on,
                result,
            } => return self.kept_sent(session, based_on, result),
            Event::OpenLatest => {
                // The changes here are dropped: asked first (leaving.rs).
                self.dialog = None;
                return self.leave(Then::Reopen);
            }
            Event::SaveCopy => {
                let Some(c) = self.cloud.file_conflict.clone() else {
                    return Task::none();
                };
                self.dialog = None;
                return self.save_copy(c);
            }
            _ => {}
        }
        Task::none()
    }

    fn file_saved(&mut self, id: u64, result: Result<FileCommitted, ApiFailure>) -> Task<Message> {
        if self.saving.as_ref().is_none_or(|s| s.id != id) {
            return Task::none();
        }
        // No answer: the save is kept on this device and goes when the connection returns.
        if let Err(failure) = &result
            && failure.status == 0
            && failure.transient()
        {
            self.went_offline();
            let (bytes, based_on) = match self.saving.as_mut() {
                Some(s) => (
                    s.bytes.take(),
                    match &s.target {
                        Target::Cloud { based_on, .. } => *based_on,
                        Target::File(_) => 0,
                    },
                ),
                None => (None, 0),
            };
            if let Some(bytes) = bytes {
                return self.keep_on_device(id, bytes, based_on);
            }
        }
        let Some(s) = self.saving.take_if(|s| s.id == id) else {
            return Task::none();
        };
        let Target::Cloud { based_on, name, .. } = &s.target else {
            return Task::none();
        };
        let same = self
            .document
            .as_ref()
            .is_some_and(|d| d.session == s.session);
        match result {
            Ok(committed) => {
                let number = committed.revision.parse::<u64>().unwrap_or(0);
                let mut later = "";
                if same && let Some(doc) = self.document.as_mut() {
                    doc.model.mark_saved(s.revision);
                    if doc.dirty() {
                        later = " Kayıt sürerken yapılan değişiklikler kaydedilmedi.";
                    }
                    if let Some(source) = doc.cloud_source_mut() {
                        source.revision = Some(Revision {
                            number,
                            sha256: committed.sha256.clone(),
                        });
                    }
                }
                self.cloud.file_conflict = None;
                self.say(
                    kentos_interaction::Level::Success,
                    format!("“{name}” buluta kaydedildi: revizyon {number}.{later}"),
                );
                if same {
                    self.recovery_saved();
                    // This save supersedes one kept on this device; the copy holds the new revision.
                    self.copy_saved(s.revision);
                }
                return self.came_online();
            }
            Err(failure) => {
                if let Some(server) = conflicting_revision(&failure) {
                    self.cloud.file_conflict = Some(FileConflict {
                        session: s.session,
                        server,
                        based_on: *based_on,
                    });
                    self.warn(format!(
                        "“{name}” kaydedilmedi: siz açtıktan sonra başka biri revizyon {server} olarak kaydetti. Hiçbir şeyin üzerine yazılmadı."
                    ));
                    if same {
                        self.dialog = Some(Dialog::FileConflict);
                    }
                } else {
                    self.say(
                        kentos_interaction::Level::Error,
                        format!(
                            "“{name}” buluta kaydedilemedi: {} Çizim kaydedilmemiş sayılıyor.",
                            failure.message
                        ),
                    );
                    if failure.signed_out() {
                        self.cloud.me = None;
                    }
                }
            }
        }
        Task::none()
    }

    /// A save kept on this device reached the server, or met a newer revision.
    fn kept_sent(
        &mut self,
        session: u64,
        based_on: u64,
        result: Result<FileCommitted, ApiFailure>,
    ) -> Task<Message> {
        let Some(held) = self.cloud.held.as_mut().filter(|h| h.session == session) else {
            return Task::none();
        };
        held.sending = None;
        let name = self
            .document
            .as_ref()
            .map_or(String::new(), |d| d.name().to_owned());
        match result {
            Ok(committed) => {
                let number = committed.revision.parse::<u64>().unwrap_or(0);
                if let Err(e) = held.replica.clear_save() {
                    self.warn(format!("Bu cihazda bekleyen kayıt silinemedi ({e}); sonraki açılışta yeniden gönderilmeye çalışılır, sunucu iki kez yazmaz."));
                }
                if let Some(held) = self.cloud.held.as_mut() {
                    held.kept_save = false;
                }
                if let Some(source) = self.document.as_mut().and_then(|d| d.cloud_source_mut()) {
                    source.revision = Some(Revision {
                        number,
                        sha256: committed.sha256.clone(),
                    });
                }
                self.say(
                    kentos_interaction::Level::Success,
                    format!(
                        "“{name}” için bu cihazda bekleyen kayıt gönderildi: revizyon {number}."
                    ),
                );
                let clean = self.document.as_ref().is_some_and(|d| !d.dirty());
                if clean {
                    let revision = self.document.as_ref().map_or(0, |d| d.model.revision());
                    self.copy_saved(revision);
                }
                Task::none()
            }
            Err(failure) => {
                if let Some(server) = conflicting_revision(&failure) {
                    // Both are kept: the server's newest stays, the save here waits for the choice.
                    self.cloud.file_conflict = Some(FileConflict {
                        session,
                        server,
                        based_on,
                    });
                    self.warn(format!(
                        "“{name}” için bu cihazda bekleyen kayıt gönderilmedi: arada başka biri revizyon {server} olarak kaydetti. İki iş de duruyor; seçin."
                    ));
                    self.dialog = Some(Dialog::FileConflict);
                } else if failure.status == 0 && failure.transient() {
                    self.went_offline();
                } else {
                    self.warn(format!(
                        "“{name}” için bu cihazda bekleyen kayıt gönderilemedi: {} Kayıt bu cihazda duruyor.",
                        failure.message
                    ));
                }
                Task::none()
            }
        }
    }

    /// After a save reached the server: a save kept here is superseded, and the copy holds the revision.
    fn copy_saved(&mut self, revision: u64) {
        let (Some(held), Some(doc)) = (self.cloud.held.as_mut(), self.document.as_ref()) else {
            return;
        };
        let Some(source) = doc.cloud_source().filter(|_| held.session == doc.session) else {
            return;
        };
        if held.kept_save && held.replica.clear_save().is_ok() {
            held.kept_save = false;
        }
        // The copy holds the drawing as it was saved; one that changed since is the draft's.
        if doc.model.revision() != revision {
            return;
        }
        let opened = Opened {
            tenant: source.tenant,
            project: source.project,
            info: source.info.clone(),
            document: doc.model.clone(),
            source: Kept::File {
                revision: source.revision.clone(),
            },
        };
        if let Err(e) = held.replica.reset(&opened) {
            let text = format!("{e} Proje bağlantısız açılırsa önceki revizyonla açılır.");
            self.warn(text);
        }
    }

    /// “Ayrı proje olarak kaydet”: the drawing as a new file project “… (kopya)”
    /// in the same workspace; the upload window shows its progress (upload.rs).
    fn save_copy(&mut self, conflict: FileConflict) -> Task<Message> {
        let Some(source) = self
            .document
            .as_ref()
            .filter(|d| d.session == conflict.session)
            .and_then(|d| d.cloud_source())
        else {
            return Task::none();
        };
        let tenant = source.info.tenant_id.clone();
        let name = format!("{} (kopya)", source.info.name);
        self.open_upload();
        // Straight up when the account may open projects in that workspace; else the window asks where.
        let chosen = self.cloud.upload.as_mut().is_some_and(|u| {
            u.from_conflict = true;
            u.choose(&tenant, &name, ProjectStorage::File)
        });
        if chosen {
            self.upload_submit()
        } else {
            Task::none()
        }
    }
}

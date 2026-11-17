//! A dossier's package in Portal dosyaları (docs/adr/9001 §5): the open cloud project's present state goes into the
//! package being prepared (a file project's newest revision, so unsaved work is saved first; a database project's
//! snapshot of now, once its edits reached the server), a pending file is taken out, and a file of the dossier is
//! saved to disk checked against its SHA-256.

use std::path::PathBuf;

use iced::Task;
use kentos_cloud::ApiFailure;
use kentos_commands::{DossierFileView, FromProject, ProjectPermission, ProjectStorage};
use kentos_domain::Uuid;

use super::steps::refusal;
use super::{Event, Said, msg};
use crate::app::{App, Message, Picker};
use crate::cloud::actions::Settle;
use crate::cloud::copy::Link;

/// The cloud project open in the drawing, as the package sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenProject {
    pub tenant: Uuid,
    pub project: Uuid,
    pub name: String,
    pub storage: ProjectStorage,
    /// A file project's revision the drawing stands on.
    pub revision: Option<u64>,
    /// A file project's drawing has changes not saved as a revision.
    pub unsaved: bool,
    /// Why it cannot go into a package now; none when it can.
    pub refused: Option<String>,
}

impl OpenProject {
    /// Whether this revision of it is in the package being prepared already: a file project's revision is exact (a
    /// database project's state is the server's now, so it may go in again).
    pub fn in_package(&self, d: &kentos_commands::DossierDetail) -> bool {
        let Some(revision) = self
            .revision
            .filter(|_| self.storage == ProjectStorage::File)
        else {
            return false;
        };
        let (project, revision) = (self.project.to_string(), revision.to_string());
        d.pending
            .iter()
            .filter_map(|f| f.source.as_ref())
            .any(|s| s.project_id == project && s.revision == revision)
    }
}

/// Why the open file project's revision is not added again.
pub const IN_PACKAGE: &str =
    "Projenin bu revizyonu pakette; yeni bir revizyon kaydedince yeniden eklenebilir.";

impl App {
    /// The cloud project open in the drawing, when there is one.
    pub(crate) fn portal_open_project(&self) -> Option<OpenProject> {
        let doc = self.document.as_ref()?;
        let source = doc.cloud_source()?;
        let storage = source.storage();
        let name = source.info.name.clone();
        let refused = if !source
            .info
            .access
            .permissions
            .contains(&ProjectPermission::Download)
        {
            Some(format!(
                "“{name}” projesinde indirme yetkiniz yok (project.download); pakete eklenemez. Projenin sahibinden \
                 yetki isteyin."
            ))
        } else if self.cloud.link == Link::Offline {
            Some("Sunucuya ulaşılamıyor; bağlantı gelince pakete ekleyin.".to_owned())
        } else {
            None
        };
        Some(OpenProject {
            tenant: source.tenant,
            project: source.project,
            name,
            storage,
            revision: source.revision.as_ref().map(|r| r.number),
            unsaved: storage == ProjectStorage::File && doc.dirty(),
            refused,
        })
    }

    pub(super) fn portal_package_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::AddOpen => self.portal_add_open(),
            Event::SaveOpen => self.save_cloud(),
            Event::Added { id, result } => self.portal_added(id, result),
            Event::Remove(file) => self.portal_remove(file),
            Event::Removed { id, name, result } => {
                let Some(p) = self.cloud.portal.as_mut().filter(|p| p.busy == Some(id)) else {
                    return Task::none();
                };
                p.busy = None;
                p.said = Some(match result {
                    Ok(()) => Said::Done(format!("“{name}” paketten çıkarıldı.")),
                    Err(f) => {
                        Said::Error(format!("“{name}” paketten çıkarılamadı: {}", refusal(&f)))
                    }
                });
                self.portal_detail()
            }
            Event::Download(file) => self.portal_download(file),
            Event::DownloadTo { id, path } => self.portal_download_to(id, path),
            Event::Downloaded { id, result } => {
                let Some(p) = self.cloud.portal.as_mut().filter(|p| p.busy == Some(id)) else {
                    return Task::none();
                };
                p.busy = None;
                p.said = Some(match result {
                    Ok((path, size)) => Said::Done(format!(
                        "“{}” kaydedildi ({}).",
                        path.display(),
                        crate::cloud::words::size_text(size)
                    )),
                    Err(f) => Said::Error(format!("Dosya indirilemedi: {}", refusal(&f))),
                });
                Task::none()
            }
            Event::Draft(_) | Event::Send | Event::Sent { .. } => self.portal_message_event(event),
            _ => Task::none(),
        }
    }

    /// Açık projeyi ekle: a database project's edits go first (actions.rs `Settle`), then its present state.
    fn portal_add_open(&mut self) -> Task<Message> {
        let Some(open) = self.portal_open_project() else {
            return Task::none();
        };
        let Some(p) = self.cloud.portal.as_mut().filter(|p| !p.working()) else {
            return Task::none();
        };
        let Some(d) = p.chosen_detail().filter(|d| d.can_add_files) else {
            return Task::none();
        };
        if open.in_package(d) {
            p.said = Some(Said::Info(IN_PACKAGE.to_owned()));
            return Task::none();
        }
        if let Some(why) = open.refused {
            p.said = Some(Said::Error(why));
            return Task::none();
        }
        if open.unsaved {
            p.said = Some(Said::Error(format!(
                "“{}” çiziminde kaydedilmemiş değişiklikler var; pakete projenin buluttaki son kaydı girer. Önce \
                 Kaydet'e basın, sonra ekleyin.",
                open.name
            )));
            return Task::none();
        }
        let waits = self.cloud.live.as_ref().is_some_and(|l| !l.sync.all_sent());
        if waits {
            if let Some(p) = self.cloud.portal.as_mut() {
                p.adding = true;
                p.said = Some(Said::Info(format!(
                    "“{}” projesinin değişiklikleri sunucuya gönderiliyor; bitince pakete eklenecek.",
                    open.name
                )));
            }
            self.cloud.settling = Some(Settle::Portal);
            return self.flush();
        }
        self.portal_add_send()
    }

    /// The open database project's edits went (or stopped going): its present state goes in, or it is said why not.
    pub(crate) fn portal_add_settled(&mut self, sent: bool) -> Task<Message> {
        let Some(p) = self.cloud.portal.as_mut().filter(|p| p.adding) else {
            return Task::none();
        };
        p.adding = false;
        if !sent {
            p.said = Some(Said::Error(
                "Açık projenin değişiklikleri sunucuya gönderilemedi; pakete eklenmedi. Bağlantıyı denetleyip \
                 yeniden deneyin."
                    .to_owned(),
            ));
            return Task::none();
        }
        self.portal_add_send()
    }

    fn portal_add_send(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let (Some(client), Some(open)) =
            (self.cloud.signed_in().cloned(), self.portal_open_project())
        else {
            return Task::none();
        };
        let Some(p) = self.cloud.portal.as_mut() else {
            return Task::none();
        };
        let Some(dossier) = p
            .chosen_detail()
            .filter(|d| d.can_add_files)
            .and_then(|d| crate::cloud::uuid(&d.summary.id))
        else {
            return Task::none();
        };
        p.busy = Some(id);
        p.said = Some(Said::Info(format!("“{}” pakete ekleniyor…", open.name)));
        let from = FromProject {
            tenant_id: open.tenant.to_string(),
            project_id: open.project.to_string(),
            doc_type: None,
        };
        Task::perform(client.dossier_add_project(dossier, from), move |result| {
            msg(Event::Added { id, result })
        })
    }

    fn portal_added(
        &mut self,
        id: u64,
        result: Result<DossierFileView, ApiFailure>,
    ) -> Task<Message> {
        let Some(p) = self.cloud.portal.as_mut().filter(|p| p.busy == Some(id)) else {
            return Task::none();
        };
        p.busy = None;
        match result {
            Ok(f) => {
                let revision = f
                    .source
                    .as_ref()
                    .map(|s| format!(" (revizyon {})", s.revision))
                    .unwrap_or_default();
                p.said = Some(Said::Done(format!(
                    "“{}”{revision} pakete eklendi; gönderene kadar yalnız siz görürsünüz.",
                    f.name
                )));
                self.portal_detail()
            }
            Err(f) => {
                p.said = Some(Said::Error(format!(
                    "Proje pakete eklenemedi: {}",
                    refusal(&f)
                )));
                Task::none()
            }
        }
    }

    /// Paketten çıkar: a pending file goes, at once (it was not sent; the web's too).
    fn portal_remove(&mut self, file: String) -> Task<Message> {
        let id = self.cloud.next_id();
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let Some(p) = self.cloud.portal.as_mut().filter(|p| !p.working()) else {
            return Task::none();
        };
        let Some(d) = p.chosen_detail() else {
            return Task::none();
        };
        let Some(f) = d
            .pending
            .iter()
            .find(|f| f.id == file && f.state == "pending")
        else {
            return Task::none();
        };
        let (Some(dossier), Some(file_id)) =
            (crate::cloud::uuid(&d.summary.id), crate::cloud::uuid(&f.id))
        else {
            return Task::none();
        };
        let name = f.name.clone();
        p.busy = Some(id);
        Task::perform(
            client.dossier_remove_file(dossier, file_id),
            move |result| msg(Event::Removed { id, name, result }),
        )
    }

    /// İndir…: where first (the system's save window), then the bytes.
    fn portal_download(&mut self, file: String) -> Task<Message> {
        let id = self.cloud.next_id();
        let Some(p) = self.cloud.portal.as_mut().filter(|p| !p.working()) else {
            return Task::none();
        };
        let Some(d) = p.chosen_detail() else {
            return Task::none();
        };
        let Some(name) = all_files(d)
            .find(|f| f.id == file)
            .map(|f| crate::cloud::catalog_actions::safe_name(&f.name))
        else {
            return Task::none();
        };
        p.busy = Some(id);
        p.download = Some(file);
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return Task::done(msg(Event::DownloadTo {
                id,
                path: Some(path),
            }));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Dosyayı kaydet")
                    .set_file_name(name)
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            move |path| msg(Event::DownloadTo { id, path }),
        )
    }

    fn portal_download_to(&mut self, id: u64, path: Option<PathBuf>) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(p) = self.cloud.portal.as_mut().filter(|p| p.busy == Some(id)) else {
            return Task::none();
        };
        let file = p.download.take();
        let dossier = p.chosen.clone();
        let (Some(path), Some(client), Some(file), Some(dossier)) = (path, client, file, dossier)
        else {
            // The save window was closed: nothing happens.
            p.busy = None;
            return Task::none();
        };
        let (Some(dossier), Some(file)) = (crate::cloud::uuid(&dossier), crate::cloud::uuid(&file))
        else {
            p.busy = None;
            return Task::none();
        };
        p.said = Some(Said::Info("Dosya indiriliyor…".to_owned()));
        let fetch = async move {
            let got = client.dossier_file(dossier, file, None).await?;
            let size = got.bytes.len();
            crate::cloud::catalog_actions::write_file(&path, &got.bytes).map_err(|e| {
                ApiFailure::new(
                    0,
                    "local",
                    format!(
                        "“{}” yazılamadı: {e}. Klasörün yazılabilir olduğunu ve diskte yer olduğunu denetleyin.",
                        path.display()
                    ),
                )
            })?;
            Ok((path, size))
        };
        Task::perform(fetch, move |result| msg(Event::Downloaded { id, result }))
    }
}

/// Every file of a dossier the reader sees: the packages sent, the one being prepared and the messages' attachments.
pub fn all_files(d: &kentos_commands::DossierDetail) -> impl Iterator<Item = &DossierFileView> {
    d.rounds
        .iter()
        .flat_map(|r| &r.files)
        .chain(&d.pending)
        .chain(d.messages.iter().flat_map(|m| &m.attachments))
}

//! The catalog's actions on the selected project (docs/adr/0086; the web's
//! CatalogDialog `actions`, ProjectActions.ts and lifecycle.ts, docs/adr/0028):
//! the favourite, archiving and unarchiving, the trash, restoring from it,
//! removing for good, and a `.kcad` download. Each is one product command
//! with its own idempotency key; the server checks the rights and answers,
//! and a refusal is said in its own words. Archiving, the trash and removing
//! for good ask first (plan.rs has the questions).
//!
//! When the project is the one open here, its unsent edits go first (the
//! web's `settle`); archiving it leaves the drawing read-only, the trash
//! leaves the project with the drawing on screen, and unarchiving opens it
//! again so its saving resumes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use iced::Task;
use iced::task::Handle;
use kentos_cloud::saving::envelope;
use kentos_cloud::{ApiFailure, Progress};
use kentos_contracts::{
    CatalogView, EmptyInput, PROJECT_ARCHIVE, PROJECT_ARCHIVE_VERSION, PROJECT_FAVORITE,
    PROJECT_FAVORITE_VERSION, PROJECT_PURGE, PROJECT_PURGE_VERSION, PROJECT_RESTORE,
    PROJECT_RESTORE_VERSION, PROJECT_TRASH, PROJECT_TRASH_VERSION, PROJECT_UNARCHIVE,
    PROJECT_UNARCHIVE_VERSION, ProjectCatalogChange, ProjectFavorite, ProjectPurge, ProjectPurged,
    ProjectState, ProjectStorage, ProjectSummary,
};
use kentos_domain::Uuid;
use kentos_interaction::Level;
use serde_json::json;

use crate::app::{App, Message, Picker, Then};
use crate::cloud::actions::{Settle, reason};
use crate::cloud::catalog::{List, Said, place_of};
use crate::cloud::plan::{self, DetailAction, lines};
use crate::cloud::{Event, local_time::Zone, uuid, words};

/// An action on the selected project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Favorite,
    Archive,
    Unarchive,
    Trash,
    Restore,
    Purge,
    Download,
}

impl Act {
    /// The pane's action it is, whose plan says whether it can be taken.
    fn detail(self) -> Option<DetailAction> {
        match self {
            Self::Archive => Some(DetailAction::Archive),
            Self::Unarchive => Some(DetailAction::Unarchive),
            Self::Trash => Some(DetailAction::Trash),
            Self::Purge => Some(DetailAction::Purge),
            Self::Download => Some(DetailAction::Download),
            Self::Favorite | Self::Restore => None,
        }
    }
}

/// What a download fetches: the project itself (a database project's
/// snapshot, a file project's newest revision), a file project's revision,
/// or a checkpoint's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetch {
    Project,
    Revision(String),
    Checkpoint(String),
}

/// A download: what it fetches, its name (the save window's and the
/// lines'), and the SHA-256 a list gave for it, checked besides the server's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub fetch: Fetch,
    pub name: String,
    pub listed: Option<String>,
    /// Revizyonla karşılaştır (docs/adr/0210 §8): the point's name; the file goes to a
    /// place of the app's own and opens as Veri karşılaştır's Eski veri, not to a place chosen.
    pub compare: Option<String>,
}

/// The action on its way: which, on what, and its request.
pub struct Acting {
    pub id: u64,
    pub act: Act,
    pub project: ProjectSummary,
    /// A download's: what it fetches, and how far it is, in words and 0–1.
    pub download: Option<Download>,
    pub progress: Option<(String, f32)>,
    _request: Option<Handle>,
}

/// What an action's request answered.
#[derive(Debug, Clone)]
pub enum Acted {
    Changed(Box<ProjectCatalogChange>),
    Purged(ProjectPurged),
    /// The download was written: where, how big, which revision.
    Downloaded {
        path: PathBuf,
        size: usize,
        revision: Option<String>,
    },
}

impl App {
    /// A pane button, the favourite toggle or the trash's main button.
    pub(crate) fn catalog_act(&mut self, act: Act) -> Task<Message> {
        let opening = self.cloud.opening.is_some();
        let Some(p) = self
            .cloud
            .catalog
            .as_ref()
            .and_then(|c| c.picked().cloned())
        else {
            return Task::none();
        };
        let open = self.is_open_project(&p.id);
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        if opening || c.busy() {
            return Task::none();
        }
        // What the plan turns off stays off, whatever sent it.
        let allowed = match act.detail() {
            Some(d) => plan::detail_plan(&p, open)
                .actions
                .iter()
                .any(|a| a.id == d && a.why.is_none()),
            None if act == Act::Restore => {
                plan::primary_plan(CatalogView::Trash, Some(&p)).enabled
                    && c.list == List::View(CatalogView::Trash)
            }
            None => p.state != ProjectState::Trashed,
        };
        if !allowed {
            return Task::none();
        }
        match act {
            Act::Archive | Act::Trash | Act::Purge => {
                c.asking = Some((act, p));
                Task::none()
            }
            Act::Download => {
                let name = p.name.clone();
                self.catalog_download(
                    p,
                    Download {
                        fetch: Fetch::Project,
                        name,
                        listed: None,
                        compare: None,
                    },
                )
            }
            _ => self.catalog_send(act, p),
        }
    }

    /// The question over the window, answered.
    pub(crate) fn catalog_answer(&mut self, yes: bool) -> Task<Message> {
        let Some((act, p)) = self.cloud.catalog.as_mut().and_then(|c| c.asking.take()) else {
            return Task::none();
        };
        if !yes {
            return Task::none();
        }
        // The open project's unsent edits go first (the web's `settle`).
        if matches!(act, Act::Archive | Act::Trash)
            && self.is_open_project(&p.id)
            && self.cloud.live.is_some()
        {
            self.cloud.settling = Some(Settle::Catalog(act, Box::new(p)));
            return self.flush();
        }
        self.catalog_send(act, p)
    }

    /// The question's words (plan.rs).
    pub(crate) fn catalog_question(&self) -> Option<(Act, plan::Question)> {
        let (act, p) = self.cloud.catalog.as_ref()?.asking.as_ref()?;
        let question = match act {
            Act::Trash => {
                let days = self
                    .cloud
                    .catalog
                    .as_ref()
                    .map(|c| c.retention)
                    .filter(|d| *d > 0);
                plan::trash_question(
                    &p.name,
                    &place_of(self, p),
                    days,
                    self.is_open_project(&p.id),
                )
            }
            Act::Purge => plan::purge_question(&p.name),
            Act::Archive => plan::archive_question(&p.name),
            _ => return None,
        };
        Some((*act, question))
    }

    /// Sends the command of `act` on `p`.
    pub(crate) fn catalog_send(&mut self, act: Act, p: ProjectSummary) -> Task<Message> {
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&p.tenant_id), uuid(&p.id)) else {
            return Task::none();
        };
        let id = self.cloud.next_id();
        let command = |name: &str, version: u32, input: serde_json::Value| {
            envelope(
                tenant,
                project,
                name,
                version,
                Uuid::new_v4(),
                BTreeMap::new(),
                input,
            )
        };
        let empty = || serde_json::to_value(EmptyInput {}).unwrap_or(json!({}));
        let changed = |request| {
            Task::perform(
                client.command::<ProjectCatalogChange>(request),
                move |result| {
                    crate::cloud::msg(Event::CatalogActed {
                        id,
                        result: result.map(|c| Acted::Changed(Box::new(c))),
                    })
                },
            )
        };
        let task = match act {
            Act::Favorite => changed(command(
                PROJECT_FAVORITE,
                PROJECT_FAVORITE_VERSION,
                serde_json::to_value(ProjectFavorite {
                    favorite: !p.favorite,
                })
                .unwrap_or(json!({})),
            )),
            Act::Archive => changed(command(PROJECT_ARCHIVE, PROJECT_ARCHIVE_VERSION, empty())),
            Act::Unarchive => changed(command(
                PROJECT_UNARCHIVE,
                PROJECT_UNARCHIVE_VERSION,
                empty(),
            )),
            Act::Trash => changed(command(PROJECT_TRASH, PROJECT_TRASH_VERSION, empty())),
            Act::Restore => changed(command(PROJECT_RESTORE, PROJECT_RESTORE_VERSION, empty())),
            Act::Purge => {
                // The server wants the project's name as the confirmation.
                let request = command(
                    PROJECT_PURGE,
                    PROJECT_PURGE_VERSION,
                    serde_json::to_value(ProjectPurge {
                        confirm_name: p.name.clone(),
                    })
                    .unwrap_or(json!({})),
                );
                Task::perform(client.command::<ProjectPurged>(request), move |result| {
                    crate::cloud::msg(Event::CatalogActed {
                        id,
                        result: result.map(Acted::Purged),
                    })
                })
            }
            Act::Download => return Task::none(),
        };
        let (task, handle) = task.abortable();
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.acting = Some(Acting {
                id,
                act,
                project: p,
                download: None,
                progress: None,
                _request: Some(handle.abort_on_drop()),
            });
        }
        task
    }

    /// An action's answer: the lines, the open project, the list again.
    pub(crate) fn catalog_acted(
        &mut self,
        id: u64,
        result: Result<Acted, ApiFailure>,
    ) -> Task<Message> {
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        if c.acting.as_ref().is_none_or(|a| a.id != id) {
            return Task::none();
        }
        let Some(Acting {
            act,
            project: p,
            download,
            ..
        }) = c.acting.take()
        else {
            return Task::none();
        };
        let name = p.name.clone();
        let compare = download.as_ref().and_then(|d| d.compare.clone());
        let download_name = download.map(|d| d.name);
        let open = self.is_open_project(&p.id);
        // Revizyonla karşılaştır (docs/adr/0210 §8): the file opens as Veri karşılaştır's Eski veri.
        if let (Some(point), Ok(Acted::Downloaded { path, .. })) = (&compare, &result) {
            if let Some(c) = self.cloud.catalog.as_mut() {
                c.status = None;
            }
            let path = path.clone();
            self.compare_with_file(point, &path);
            return Task::none();
        }
        let failure = match result {
            Ok(done) => return self.catalog_done(act, p, done, open, download_name),
            Err(failure) => failure,
        };
        let why = reason(&failure);
        match act {
            // The questions' actions say their failure in the log (the web's ProjectActions).
            Act::Trash => self.error(lines::trash_failed(&name, &why)),
            Act::Purge => self.error(lines::purge_failed(&name, &why)),
            Act::Archive => self.error(lines::archive_failed(&name, &why)),
            // A download's too, and its line goes (the web's `downloadKcad`).
            Act::Download => {
                self.error(format!(
                    "“{}.kcad” indirilemedi: {}",
                    safe_name(&download_name.unwrap_or(name)),
                    failure.message
                ));
                if let Some(c) = self.cloud.catalog.as_mut() {
                    c.status = None;
                }
            }
            _ => {
                if let Some(c) = self.cloud.catalog.as_mut() {
                    c.status = Some(Said::error(why));
                }
            }
        }
        Task::none()
    }

    fn catalog_done(
        &mut self,
        act: Act,
        p: ProjectSummary,
        done: Acted,
        open: bool,
        download_name: Option<String>,
    ) -> Task<Message> {
        let zone = Zone::system();
        let name = p.name.clone();
        let say = |app: &mut App, text: String| {
            if let Some(c) = app.cloud.catalog.as_mut() {
                c.status = Some(Said::info(text));
            }
        };
        match (act, done) {
            (Act::Favorite, Acted::Changed(change)) => {
                let favorite = change.project.favorite;
                let line = if favorite {
                    lines::favorite_added(&name)
                } else {
                    lines::favorite_removed(&name)
                };
                let Some(c) = self.cloud.catalog.as_mut() else {
                    return Task::none();
                };
                // Taken out of “Favoriler”, it leaves the list; anywhere else its row changes in place.
                if c.list == List::View(CatalogView::Favorites) && !favorite {
                    say(self, line);
                    return self.catalog_load(false);
                }
                if let Some(row) = c.projects.iter_mut().find(|x| x.id == change.project.id) {
                    *row = change.project;
                }
                say(self, line);
                Task::none()
            }
            (Act::Archive, Acted::Changed(_)) => {
                if open {
                    self.archived_here();
                } else {
                    self.say(Level::Success, lines::archived(&name));
                }
                say(self, lines::archived_status(&name));
                self.catalog_load(false)
            }
            (Act::Unarchive, Acted::Changed(change)) => {
                self.say(Level::Success, lines::unarchived(&name));
                say(self, lines::unarchived_status(&name));
                let list = self.catalog_load(false);
                // Unarchived, the open project opens again: its saving resumes
                // and edits kept on this device come back (the web's `unarchive`).
                if open && change.changed {
                    self.cloud.reopen_keeps_catalog = true;
                    return Task::batch([list, self.leave(Then::Reopen)]);
                }
                list
            }
            (Act::Trash, Acted::Changed(change)) => {
                if open {
                    // The open project's own words: the drawing stays on screen.
                    self.detach_cloud();
                    let until = change
                        .project
                        .purge_after
                        .as_deref()
                        .and_then(|t| crate::cloud::local_time::day(t, zone))
                        .map_or_else(String::new, |d| {
                            format!(" ({d} tarihine kadar geri yüklenebilir)")
                        });
                    self.output(format!(
                        "“{name}” bulut projesi çöp kutusuna taşındı{until}. Çizim ekranda kaldı; saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin."
                    ));
                } else {
                    self.say(
                        Level::Success,
                        lines::trashed(&name, change.project.purge_after.as_deref(), zone),
                    );
                }
                say(self, lines::trashed_status(&name));
                self.catalog_load(false)
            }
            (Act::Restore, Acted::Changed(_)) => {
                self.say(Level::Success, lines::restored(&name));
                say(self, lines::restored_status(&name));
                self.catalog_load(false)
            }
            (Act::Purge, Acted::Purged(gone)) => {
                self.say(Level::Success, lines::purged(&gone.name, &gone.objects));
                say(self, lines::purged_status(&name));
                self.catalog_load(false)
            }
            (
                Act::Download,
                Acted::Downloaded {
                    path,
                    size,
                    revision,
                },
            ) => {
                let file = path
                    .file_name()
                    .map_or_else(|| name.clone(), |f| f.to_string_lossy().into_owned());
                let revision = revision.map_or_else(String::new, |r| format!(", revizyon {r}"));
                self.say(
                    Level::Success,
                    format!("“{file}” indirildi: {}{revision}.", words::size_text(size)),
                );
                say(self, lines::downloaded(&download_name.unwrap_or(name)));
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// “.kcad olarak indir”: where first, then the bytes with their progress,
    /// checked against the server's SHA-256, then written (the web's `downloadKcad`).
    pub(crate) fn catalog_download(&mut self, p: ProjectSummary, d: Download) -> Task<Message> {
        let id = self.cloud.next_id();
        let file = format!("{}.kcad", safe_name(&d.name));
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        if c.acting.is_some() {
            return Task::none();
        }
        c.acting = Some(Acting {
            id,
            act: Act::Download,
            project: p,
            download: Some(d),
            progress: None,
            _request: None,
        });
        // A point to compare goes to a file of the app's own (read, then removed).
        let compare = c
            .acting
            .as_ref()
            .and_then(|a| a.download.as_ref())
            .is_some_and(|d| d.compare.is_some());
        if compare {
            let path = std::env::temp_dir().join(format!(
                "kentos-karsilastir-{}-{id}.kcad",
                std::process::id()
            ));
            return Task::done(crate::cloud::msg(Event::CatalogDownloadTo {
                id,
                path: Some(path),
            }));
        }
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return Task::done(crate::cloud::msg(Event::CatalogDownloadTo {
                id,
                path: Some(path),
            }));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(".kcad olarak indir")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .set_file_name(file)
                    .save_file()
                    .await?;
                let path = file.path().to_path_buf();
                Some(if path.extension().is_some_and(|e| e == "kcad") {
                    path
                } else {
                    path.with_extension("kcad")
                })
            },
            move |path| crate::cloud::msg(Event::CatalogDownloadTo { id, path }),
        )
    }

    /// The place was chosen (or not): the bytes come.
    pub(crate) fn catalog_download_to(&mut self, id: u64, path: Option<PathBuf>) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        let Some(a) = c.acting.as_mut().filter(|a| a.id == id) else {
            return Task::none();
        };
        let (Some(path), Some(client)) = (path, client) else {
            // The save window was closed: nothing happens.
            c.acting = None;
            c.status = None;
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&a.project.tenant_id), uuid(&a.project.id))
        else {
            c.acting = None;
            return Task::none();
        };
        let (sender, receiver) = iced::futures::channel::mpsc::unbounded::<(u64, u64)>();
        let progress: Progress = Arc::new(move |done, total| {
            let _ = sender.unbounded_send((done, total));
        });
        let storage = a.project.storage;
        let (what, listed) = a.download.as_ref().map_or((Fetch::Project, None), |d| {
            (d.fetch.clone(), d.listed.clone())
        });
        let fetch = async move {
            let got = if let Fetch::Revision(revision) = &what {
                let Ok(revision) = revision.parse::<u64>() else {
                    return Err(ApiFailure::new(0, "local", "Revizyon numarası okunamadı."));
                };
                client
                    .download_revision(tenant, project, revision, Some(progress))
                    .await?
            } else if let Fetch::Checkpoint(checkpoint) = &what {
                let Some(checkpoint) = uuid(checkpoint) else {
                    return Err(ApiFailure::new(
                        0,
                        "local",
                        "Kontrol noktasının kimliği okunamadı.",
                    ));
                };
                client
                    .checkpoint_file(tenant, project, checkpoint, Some(progress))
                    .await?
            } else if storage == ProjectStorage::File {
                // A file project's newest revision.
                let revs = client.file_revisions(tenant, project).await?;
                let Some(current) = revs.current.as_deref().and_then(|r| r.parse::<u64>().ok())
                else {
                    return Err(ApiFailure::new(
                        0,
                        "local",
                        "Projenin henüz kaydedilmiş revizyonu yok; indirilecek dosya yok.",
                    ));
                };
                client
                    .download_revision(tenant, project, current, Some(progress))
                    .await?
            } else {
                // A database project as one file of one moment.
                client.snapshot(tenant, project, Some(progress)).await?
            };
            // The list's SHA-256 too, besides the server's (the web's `verifyDownload`).
            if listed.as_ref().is_some_and(|want| *want != got.sha256) {
                return Err(ApiFailure::new(
                    0,
                    "local",
                    "İndirilen dosya sunucudakiyle aynı değil (SHA-256 tutmuyor); dosya kullanılmadı. Bağlantınızı denetleyip yeniden deneyin.",
                ));
            }
            let size = got.bytes.len();
            write_file(&path, &got.bytes).map_err(|e| {
                ApiFailure::new(0, "local", format!(
                    "“{}” yazılamadı: {e}. Klasörün yazılabilir olduğunu ve diskte yer olduğunu denetleyin.",
                    path.display()
                ))
            })?;
            Ok(Acted::Downloaded {
                path,
                size,
                revision: got.revision,
            })
        };
        let progress = iced::Task::run(receiver, move |(done, total)| {
            crate::cloud::msg(Event::CatalogDownloadProgress { id, done, total })
        });
        let (fetch, handle) = Task::perform(fetch, move |result| {
            crate::cloud::msg(Event::CatalogActed { id, result })
        })
        .abortable();
        a._request = Some(handle.abort_on_drop());
        a.progress = Some(("İndiriliyor…".to_owned(), 0.0));
        Task::batch([fetch, progress])
    }

    /// How far a download is (the web's `İndiriliyor: 1,2 MB / 3,4 MB`).
    pub(crate) fn catalog_download_progress(&mut self, id: u64, done: u64, total: u64) {
        if let Some(a) = self
            .cloud
            .catalog
            .as_mut()
            .and_then(|c| c.acting.as_mut())
            .filter(|a| a.id == id)
        {
            let of = if total > 0 {
                format!(" / {}", words::size_text(total as usize))
            } else {
                String::new()
            };
            let fraction = if total > 0 {
                (done as f32 / total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            a.progress = Some((
                format!("İndiriliyor: {}{of}", words::size_text(done as usize)),
                fraction,
            ));
        }
    }

    /// The open project archived from here: read-only, said once (the web's
    /// `projectArchived(open, true)`); the notice of a project archived by
    /// someone else is not shown for it.
    fn archived_here(&mut self) {
        let Some(source) = self.document.as_mut().and_then(|d| d.cloud_source_mut()) else {
            return;
        };
        source.info.state = ProjectState::Archived;
        let name = source.info.name.clone();
        self.cloud.archived_by_me = true;
        self.output(format!(
            "“{name}” arşivlendi: salt okunur. Değiştirmek için arşivden çıkarın; çizim ekranda kalıyor."
        ));
    }
}

/// A file name without what file systems refuse (the web's `safe`).
fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c < ' ' {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        "proje".to_owned()
    } else {
        cleaned.to_owned()
    }
}

/// Writes the file whole or not at all: a temporary file beside it, then renamed.
fn write_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let dir = path.parent().unwrap_or(std::path::Path::new("."));
    let name = path
        .file_name()
        .map_or_else(|| "indirilen".into(), |n| n.to_string_lossy().into_owned());
    let temp = dir.join(format!(".{name}.{}.indiriliyor", std::process::id()));
    let written = (|| {
        let mut f = std::fs::File::create(&temp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&temp, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_download_is_named_as_the_web_names_it() {
        assert_eq!(safe_name("Ada 101"), "Ada 101");
        assert_eq!(safe_name("a/b:c*d?\"e<f>g|h"), "a_b_c_d__e_f_g_h");
        assert_eq!(safe_name("  "), "proje");
        assert_eq!(safe_name("x\u{1}y"), "x_y");
    }
}

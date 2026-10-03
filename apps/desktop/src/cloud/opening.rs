//! A cloud project opened into the drawing (docs/adr/0041, 0043; the web's
//! `CloudSession.open`). Its local copy is locked first (another KentOS
//! window may have it). Online, kentos-cloud reads the project off the UI
//! thread — a database project page by page, a file project's newest
//! revision checked against its SHA-256 — and builds and checks the drawing
//! (docs/adr/0040); the copy is then rewritten from it. Without a
//! connection, or when the server does not answer, the project opens from
//! the copy. Its progress shows in the catalog, or in a window of its own;
//! Vazgeç or Esc stops it. The drawing on screen is replaced in one step,
//! only by the latest open, and only if it did not change meanwhile
//! (CLAUDE.md §21.2). A database project's device draft is put back at once;
//! a file project's save kept on this device waits to be sent.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use iced::Task;
use iced::futures::channel::mpsc;
use iced::futures::stream;
use iced::task::Handle;
use kentos_cloud::{
    Loaded, Opened, ProjectSync, Replica, Revision, Source as Kept, given_back_text,
};
use kentos_contracts::ProjectStorage;
use kentos_domain::Uuid;

use crate::app::{App, Dialog, Message};
use crate::cloud::copy::{Held, Link};
use crate::cloud::{Event, Live, Once, words};
use crate::document::{CloudSource, Document};

/// An open under way.
pub struct Opening {
    pub id: u64,
    pub name: String,
    /// How it is kept, when known (the progress counts objects or bytes).
    pub storage: Option<ProjectStorage>,
    pub stage: String,
    pub fraction: Option<f32>,
    pub started: Instant,
    /// Read from this device's copy, without the server.
    pub offline: bool,
    /// The drawing on screen when it began: which one, and its revision.
    was: Option<(u64, u64)>,
    /// The project's copy, locked for this program; in a task while it is read or written.
    replica: Option<Replica>,
    /// The catalog stays on screen once it is open (the open project again,
    /// unarchived in the catalog).
    keep_catalog: bool,
    /// A file project's newest revision, chosen over the drawing on screen:
    /// never this device's copy instead (it is not the newest), and its
    /// failure says so (revisions.rs `texts::open_failed`).
    pub newest: bool,
    _request: Option<Handle>,
}

/// What an open read: the project, its copy, and a file project's save
/// kept on this device (its drawing, and the revision it is based on).
pub struct Read {
    pub opened: Opened,
    pub replica: Option<Replica>,
    pub kept: Option<(kentos_domain::Document, u64)>,
    /// Something went wrong with the copy (said, the open goes on).
    pub note: Option<String>,
}

impl std::fmt::Debug for Read {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Read(…)")
    }
}

/// A file project's save kept in the copy, as a drawing.
fn kept_drawing(replica: &Replica) -> Option<(kentos_domain::Document, u64)> {
    let (bytes, based_on) = replica.kept_save().ok().flatten()?;
    let snapshot = kentos_kcad::decode(&bytes).ok()?;
    let doc = kentos_domain::Document::from_snapshot_v2(snapshot).ok()?;
    Some((doc, based_on))
}

/// The copy rewritten from what the server gave (off the UI thread).
fn reset(opened: Opened, replica: Option<Replica>) -> Read {
    let mut replica = replica;
    let note = replica
        .as_mut()
        .and_then(|r| r.reset(&opened).err())
        .map(|e| format!("{e} Proje bağlantısız açılamayacak."));
    let kept = match (&replica, opened.info.storage) {
        (Some(r), ProjectStorage::File) => kept_drawing(r),
        _ => None,
    };
    Read {
        opened,
        replica,
        kept,
        note,
    }
}

/// The project from the copy (off the UI thread).
fn load(replica: Replica) -> Result<Read, (String, Replica)> {
    match replica.load() {
        Ok(Some(opened)) => {
            let kept = match opened.info.storage {
                ProjectStorage::File => kept_drawing(&replica),
                ProjectStorage::Database => None,
            };
            Ok(Read {
                opened,
                replica: Some(replica),
                kept,
                note: None,
            })
        }
        Ok(None) => Err((
            "bu cihazda kopyası yok; açmak için sunucuya bağlanmak gerekiyor".to_owned(),
            replica,
        )),
        Err(e) => Err((e.to_string(), replica)),
    }
}

impl App {
    /// Opens a cloud project into the drawing (the question about the
    /// drawing on screen was asked before). An open under way gives way.
    pub(crate) fn start_cloud_open(&mut self, tenant: Uuid, project: Uuid) -> Task<Message> {
        let (name, storage) = self
            .cloud
            .open_hint
            .take()
            .map_or_else(|| (String::new(), None), |(n, s)| (n, Some(s)));
        let name = if name.is_empty() {
            "Bulut projesi".to_owned()
        } else {
            name
        };
        let Some((server, user)) = self.cloud_identity() else {
            self.warn("Bulut oturumu açık değil; önce giriş yapın (Buluta giriş).");
            return Task::none();
        };
        // An earlier open gives way; its copy is let go.
        self.cloud.opening = None;
        let replica = match self.lock_copy(&server, &user, tenant, project) {
            Ok(replica) => replica,
            Err(e) => {
                self.open_failed(&name, e.to_string());
                return Task::none();
            }
        };
        let id = self.cloud.next_id();
        let started = Instant::now();
        let was = self
            .document
            .as_ref()
            .map(|d| (d.session, d.model.revision()));
        // A project archived from the catalog was the one before this.
        self.cloud.archived_by_me = false;
        let mut opening = Opening {
            id,
            name,
            storage,
            stage: "Proje bilgileri okunuyor".to_owned(),
            fraction: None,
            started,
            offline: false,
            was,
            replica,
            keep_catalog: std::mem::take(&mut self.cloud.reopen_keeps_catalog),
            newest: std::mem::take(&mut self.cloud.open_newest),
            _request: None,
        };
        let task = match self.cloud.signed_in().cloned() {
            Some(client) => {
                let (tell, told) = mpsc::unbounded();
                let last = Arc::new(AtomicU64::new(0));
                let progress: kentos_cloud::Progress = Arc::new(move |done, total| {
                    // A few times a second at most, and the last one always.
                    let ms = started.elapsed().as_millis() as u64;
                    if done < total && ms.saturating_sub(last.load(Ordering::Relaxed)) < 50 {
                        return;
                    }
                    last.store(ms, Ordering::Relaxed);
                    let _ = tell.unbounded_send(crate::cloud::msg(Event::OpenProgress {
                        id,
                        done,
                        total,
                    }));
                });
                let open = kentos_cloud::open(&client, tenant, project, Some(progress));
                let done = stream::once(async move {
                    crate::cloud::msg(Event::Opened {
                        id,
                        result: open.await.map(Once::new),
                    })
                });
                let (task, handle) = Task::stream(stream::select(told, done)).abortable();
                opening._request = Some(handle.abort_on_drop());
                task
            }
            // The newest revision is the server's: this device's copy is not it.
            None if opening.newest => {
                self.give_back(opening.replica.take());
                self.newest_failed(
                    &opening.name,
                    "bulut oturumu açık değil; önce giriş yapın (Buluta giriş).",
                );
                return Task::none();
            }
            None => match opening.replica.take() {
                Some(replica) => {
                    opening.offline = true;
                    opening.stage = "Bu cihazdaki kopya okunuyor".to_owned();
                    load_task(id, replica)
                }
                None => {
                    self.open_failed(
                        &opening.name,
                        "bulut oturumu açık değil ve bu cihazda kopyası tutulmuyor; önce giriş yapın".to_owned(),
                    );
                    return Task::none();
                }
            },
        };
        self.cloud.opening = Some(opening);
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.status = None;
        }
        task
    }

    /// The open file project's newest revision, chosen over the drawing on
    /// screen (file_follow.rs): read from the server, never from this
    /// device's copy; a failure leaves the drawing as it is.
    pub(crate) fn reopen_newest(&mut self) -> Task<Message> {
        self.cloud.open_newest = true;
        let task = self.reopen();
        // Nothing opened (no project, or no session): the flag waits for no one.
        self.cloud.open_newest = false;
        task
    }

    /// The drawing's own cloud project again (a file project's newest
    /// revision; a database project whose missed events are gone).
    pub(crate) fn reopen(&mut self) -> Task<Message> {
        let Some(source) = self.document.as_ref().and_then(|d| d.cloud_source()) else {
            return Task::none();
        };
        let (tenant, project) = (source.tenant, source.project);
        self.cloud.open_hint = Some((source.info.name.clone(), source.storage()));
        // A conflict stands until the project is read again (take_opened):
        // an open stopped or refused leaves everything as it was.
        self.start_cloud_open(tenant, project)
    }

    /// The newest revision could not be opened: the drawing stays as it is.
    fn newest_failed(&mut self, name: &str, why: &str) {
        self.say(
            kentos_interaction::Level::Error,
            crate::cloud::revisions::texts::open_failed(name, why),
        );
    }

    fn open_failed(&mut self, name: &str, why: String) {
        let text = format!("“{name}” açılamadı: {why}");
        match self.cloud.catalog.as_mut() {
            Some(c) if self.dialog == Some(Dialog::Catalog) => {
                c.status = Some(crate::cloud::catalog::Said::error(text));
            }
            _ => self.say(kentos_interaction::Level::Error, text),
        }
    }

    pub(crate) fn opening_cloud_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::OpenProgress { id, done, total } => {
                if let Some(o) = self.cloud.opening.as_mut().filter(|o| o.id == id) {
                    o.fraction = (total > 0).then(|| (done as f32 / total as f32).min(1.0));
                    o.stage = match o.storage {
                        Some(ProjectStorage::File) => format!(
                            "Son revizyon indiriliyor: {:.1} / {:.1} MB",
                            done as f64 / 1e6,
                            total as f64 / 1e6
                        ),
                        _ => format!("Nesneler okunuyor: {done} / {total}"),
                    };
                }
            }
            Event::OpenCancel => {
                if let Some(o) = self.cloud.opening.take() {
                    self.output(format!(
                        "“{}” açılışı durduruldu; ekrandaki çizim olduğu gibi duruyor.",
                        o.name
                    ));
                    self.give_back(o.replica);
                }
            }
            Event::Opened { id, result } => {
                let Some(o) = self.cloud.opening.as_mut().filter(|o| o.id == id) else {
                    return Task::none();
                };
                match result {
                    Ok(opened) => {
                        let Some(opened) = opened.take() else {
                            return Task::none();
                        };
                        o.stage = "Bu cihazdaki kopya yazılıyor".to_owned();
                        let replica = o.replica.take();
                        let online = self.came_online();
                        return Task::batch([online, reset_task(id, opened, replica)]);
                    }
                    // The newest revision: never this device's copy instead.
                    Err(failure) if o.newest => {
                        let o = self.cloud.opening.take();
                        let (name, replica) =
                            o.map_or((String::new(), None), |o| (o.name, o.replica));
                        self.give_back(replica);
                        if failure.status == 0 && failure.transient() {
                            self.went_offline();
                        }
                        self.newest_failed(&name, &failure.message);
                    }
                    // No answer: the project opens from this device's copy, when there is one.
                    Err(failure) if failure.status == 0 && failure.transient() => {
                        let replica = o.replica.take();
                        self.went_offline();
                        return match replica {
                            Some(replica) => {
                                if let Some(o) = self.cloud.opening.as_mut() {
                                    o.offline = true;
                                    o.stage = "Sunucuya ulaşılamadı; bu cihazdaki kopya okunuyor"
                                        .to_owned();
                                }
                                load_task(id, replica)
                            }
                            None => {
                                let o = self.cloud.opening.take();
                                let name = o.map(|o| o.name).unwrap_or_default();
                                self.open_failed(&name, failure.message.clone());
                                Task::none()
                            }
                        };
                    }
                    Err(failure) => {
                        let o = self.cloud.opening.take();
                        let (name, replica) =
                            o.map_or((String::new(), None), |o| (o.name, o.replica));
                        self.give_back(replica);
                        self.open_failed(&name, failure.message.clone());
                    }
                }
            }
            Event::Read { id, result } => {
                if self.cloud.opening.as_ref().is_none_or(|o| o.id != id) {
                    // Overtaken or stopped: the copy goes back (or is let go).
                    if let Ok(read) = &result
                        && let Some(read) = read.take()
                    {
                        self.give_back(read.replica);
                    }
                    return Task::none();
                }
                let Some(o) = self.cloud.opening.take() else {
                    return Task::none();
                };
                return match result {
                    Ok(read) => match read.take() {
                        Some(read) => self.take_opened(read, o),
                        None => Task::none(),
                    },
                    Err(failed) => {
                        let (why, replica) = failed.take().unwrap_or_default();
                        self.give_back(replica);
                        if o.newest {
                            self.newest_failed(&o.name, &why);
                        } else {
                            self.open_failed(&o.name, why);
                        }
                        Task::none()
                    }
                };
            }
            _ => {}
        }
        Task::none()
    }

    /// A copy not used by the open goes back to the project it belongs to, if it is open.
    fn give_back(&mut self, replica: Option<Replica>) {
        let (Some(replica), Some(doc)) = (replica, self.document.as_ref()) else {
            return;
        };
        if self.cloud.held.is_none()
            && let Some(source) = doc.cloud_source()
        {
            self.cloud.held = Some(Held::new(
                doc.session,
                source.tenant,
                source.project,
                replica,
            ));
        }
    }

    /// The project read and checked: it becomes the drawing on screen.
    fn take_opened(&mut self, read: Read, o: Opening) -> Task<Message> {
        let now = self
            .document
            .as_ref()
            .map(|d| (d.session, d.model.revision()));
        if now != o.was {
            self.warn(format!(
                "“{}” açılmadı: açılış sürerken ekrandaki çizim değişti ya da başka bir çizim açıldı; o çizim olduğu gibi duruyor. Projeyi yeniden açın.",
                o.name
            ));
            self.give_back(read.replica);
            return Task::none();
        }
        let Read {
            opened,
            replica,
            kept,
            note,
        } = read;
        let sync = ProjectSync::new(&opened);
        let own = self.cloud.membership(&opened.info.tenant_id).is_some()
            || (self.cloud.me.is_none()
                && opened.info.tenant_kind == kentos_contracts::TenantKind::Personal);
        let workspace = words::workspace(opened.info.tenant_kind, &opened.info.tenant_name, own);
        let revision: Option<Revision> = match &opened.source {
            Kept::File { revision } => revision.clone(),
            Kept::Database { .. } => None,
        };
        let (tenant, project) = (opened.tenant, opened.project);
        let archived = opened.archived();
        let name = opened.info.name.clone();
        let storage = opened.info.storage;
        // A file project's events are followed from the cursor it was read at (file_follow.rs).
        let follows = (storage == ProjectStorage::File).then(|| opened.info.clone());
        // A file project's save kept on this device is the drawing: it is this user's work.
        let kept_based_on = kept.as_ref().map(|(_, b)| *b);
        let model = match kept {
            Some((doc, _)) => doc,
            None => opened.document,
        };
        let doc = Document::cloud(
            model,
            CloudSource {
                tenant,
                project,
                info: opened.info,
                workspace,
                revision,
            },
        );
        let session = doc.session;
        let task = self.update(Message::Opened(Some(Ok(Box::new(doc)))));
        self.cloud.file_conflict = None;
        self.cloud.question = None;
        if let Some(info) = follows {
            self.follow_file(session, &info);
        }
        if let Some(note) = note {
            self.warn(note);
        }
        if let Some(replica) = replica {
            self.cloud.held = Some(Held::new(session, tenant, project, replica));
        }
        if let Some(sync) = sync {
            let key = match (&self.cloud.drafts, self.cloud_identity()) {
                (Some(store), Some((server, user))) => {
                    Some(store.key(&server, &user, tenant, project))
                }
                _ => None,
            };
            let seen = self
                .document
                .as_ref()
                .map_or((0, 0), |d| (d.model.generation(), d.model.revision()));
            self.cloud.live = Some(Live::new(session, tenant, project, sync, key, seen));
            self.restore_draft();
        }
        if o.offline {
            self.cloud.link = Link::Offline;
            self.cloud.probe_at = Some(Instant::now());
            self.warn(format!(
                "“{name}” bu cihazdaki kopyasından açıldı: Çevrimdışı — değişiklikler bu cihazda saklanıyor; bağlantı gelince gönderilir."
            ));
        }
        if let Some(based_on) = kept_based_on {
            self.output(format!(
                "“{name}” için bu cihazda bekleyen bir kayıt var (revizyon {based_on} üstüne); bağlantı gelince gönderilecek."
            ));
        }
        if archived {
            self.output(format!(
                "“{name}” arşivlenmiş bir proje: salt okunur açıldı; değişiklikler buluta kaydedilmez. Düzenlemek için arşivden çıkarılmalı ya da kopyası oluşturulmalı."
            ));
        }
        if !o.keep_catalog {
            if self.dialog == Some(Dialog::Catalog) {
                self.dialog = None;
            }
            self.cloud.catalog = None;
        }
        // A project without a type is asked it (docs/adr/0165 §1); the answer goes with its settings.
        self.ask_project_type();
        let kept_goes = if storage == ProjectStorage::File && !o.offline {
            self.send_kept_save()
        } else {
            Task::none()
        };
        Task::batch([task, kept_goes])
    }

    /// Puts the project's device draft back, before any edit (docs/adr/0040).
    fn restore_draft(&mut self) {
        let (Some(store), Some(live), Some(doc)) = (
            self.cloud.drafts.clone(),
            self.cloud.live.as_mut(),
            self.document.as_mut(),
        ) else {
            return;
        };
        let Some(key) = live.key.clone() else {
            return;
        };
        let mut said = Vec::new();
        match store.load(&key) {
            Ok(Loaded::None) => {}
            Ok(Loaded::Found(draft)) => match live.sync.restore(&mut doc.model, *draft) {
                Ok(restored) => {
                    if restored.changed > 0 || restored.resends {
                        said.push((
                            false,
                            "Bu cihazda gönderilmemiş değişiklikler vardı; çizime geri kondu."
                                .to_owned(),
                        ));
                    }
                    for held in restored.held {
                        said.push((
                            true,
                            format!("Taslaktaki bir değişiklik çizime konamadı: {held}."),
                        ));
                    }
                    // A layer the draft dropped that others drew on since stays (docs/adr/0081).
                    for name in &restored.given_back {
                        said.push((true, given_back_text(name)));
                    }
                    // A block definition that gave way to a name, a removal left out (docs/adr/0144 §5).
                    for note in restored.notes {
                        said.push((true, note));
                    }
                    if restored.conflicts > 0 {
                        said.push((true, format!(
                            "Taslaktaki {} değişikliğin dayandığı sürümü sunucuda başkası değiştirmiş: kayıt çakışması. Durum çubuğundaki Çakışma'ya tıklayıp seçin.",
                            restored.conflicts
                        )));
                    }
                    if restored.resends {
                        // The command that was on its way goes first, with its key.
                        live.send_soon();
                    }
                }
                Err(why) => said.push((true, why)),
            },
            Ok(Loaded::KeptAside { reason, .. }) => said.push((
                true,
                format!(
                    "Bu projenin cihazdaki taslağı {reason}; taslak olduğu gibi ayrıca saklandı."
                ),
            )),
            Err(e) => said.push((
                true,
                format!("Bu projenin cihazdaki taslağı okunamadı ({e}); dosyaya dokunulmadı."),
            )),
        }
        for (warn, text) in said {
            if warn {
                self.warn(text);
            } else {
                self.output(text);
            }
        }
    }
}

/// Rewrites the copy from the opened project, off the UI thread.
fn reset_task(id: u64, opened: Opened, replica: Option<Replica>) -> Task<Message> {
    iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
        let read = reset(opened, replica);
        let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
            &mut out,
            crate::cloud::msg(Event::Read {
                id,
                result: Ok(Once::new(read)),
            }),
        ));
    })
}

/// Reads the project from the copy, off the UI thread.
fn load_task(id: u64, replica: Replica) -> Task<Message> {
    iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
        let result = match load(replica) {
            Ok(read) => Ok(Once::new(read)),
            Err((why, replica)) => Err(Once::new((why, Some(replica)))),
        };
        let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
            &mut out,
            crate::cloud::msg(Event::Read { id, result }),
        ));
    })
}

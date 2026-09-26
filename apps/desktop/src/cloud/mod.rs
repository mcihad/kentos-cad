//! The desktop's cloud interface (docs/adr/0041) over `kentos-cloud`
//! (docs/adr/0040): signing in to a KentOS server, the catalog of cloud
//! projects, a project opened into the drawing and saved the way it is kept
//! (a file project's next revision on Kaydet; a database project's changes
//! as they happen, with a device draft), other editors' changes followed,
//! the conflicts, and a drawing uploaded as a new project. It follows the
//! web's cloud interface (apps/web/src/ui/cloud/, app/cloud/session.ts) and
//! its words.
//!
//! Every request is a task of its own on Iced's executor; its answer comes
//! back as an [`Event`] carrying the id of the request or the drawing it was
//! for, so a late answer never lands on a newer drawing (CLAUDE.md §21.2).
//! Dropping a request's handle stops it. The session lives in the
//! connection's memory only (kentos-cloud): nothing here writes it, and the
//! password is kept only while the sign-in window is open.
//!
//! | File | What it holds |
//! |---|---|
//! | `account.rs` | the sign-in window, signing out |
//! | `catalog.rs` | “Bulut projesi aç”: the lists, search, pages |
//! | `opening.rs` | a project opened into the drawing, its progress |
//! | `leaving.rs` | leaving a project: its draft first, or the question |
//! | `copy.rs` | the project's local copy, work without a connection |
//! | `live.rs` | a database project's autosave and device draft |
//! | `follow.rs` | others' changes followed, access, conflicts resolved |
//! | `file.rs` | a file project's save and its conflict |
//! | `upload.rs` | “Buluta yükle” |
//! | `view.rs` | the windows and the status bar cells |
//! | `words.rs` | the interface's words: roles, lists, states, times |

mod account;
mod catalog;
pub mod copy;
mod file;
mod follow;
mod leaving;
mod live;
mod opening;
mod upload;
mod view;
pub mod words;

#[cfg(test)]
pub(crate) mod tests;
// Against a real server (apps/desktop/scripts/cloud-live.sh), ignored otherwise.
#[cfg(test)]
mod live_run;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iced::Task;
use iced::futures::channel::mpsc;
use iced::futures::{SinkExt as _, Stream};
use kentos_cloud::{ApiFailure, Cloud as Client, DraftStore};
use kentos_contracts::{Me, MembershipView, ProjectInfo, ProjectPermission, ProjectState};
use kentos_domain::Uuid;

use crate::app::{App, Message};

pub use account::SignIn;
pub use catalog::Catalog;
pub use file::FileConflict;
pub use leaving::Leave;
pub use live::Live;
pub use opening::Opening;
pub use upload::Upload;

/// How often the cloud's timers are looked at while something waits on them.
pub const TICK: Duration = Duration::from_millis(200);

/// A value a message carries once. Messages are cloned by Iced's widgets,
/// so their payloads must be `Clone`; an opened project is not, and a
/// file's bytes should not be copied: the first `take` gets it.
pub struct Once<T>(Arc<Mutex<Option<T>>>);

impl<T> Clone for Once<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Once<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(Some(value))))
    }

    pub fn take(&self) -> Option<T> {
        self.0.lock().ok().and_then(|mut v| v.take())
    }
}

impl<T> std::fmt::Debug for Once<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Once(…)")
    }
}

/// The app's message of a cloud event (boxed: some carry a project's whole info).
pub fn msg(event: Event) -> Message {
    Message::Cloud(Box::new(event))
}

/// What the cloud's windows and requests tell the app.
#[derive(Debug, Clone)]
pub enum Event {
    /// The timers: autosave, the draft, following, the catalog's search.
    Tick,
    // ── Signing in ──────────────────────────────────────────────────────
    SignInServer(String),
    SignInLogin(String),
    SignInPassword(String),
    SignInSubmit,
    SignedIn {
        id: u64,
        result: Result<Me, ApiFailure>,
    },
    SignedOut(Result<(), ApiFailure>),
    // ── The catalog ─────────────────────────────────────────────────────
    CatalogView(catalog::List),
    CatalogSearch(String),
    CatalogPick(String),
    CatalogOpen,
    CatalogMore,
    CatalogRetry,
    /// “Bu cihazdan kaldır” on a copy of the offline list, and its question answered.
    CatalogRemove,
    RemoveConfirmed,
    CatalogPage {
        id: u64,
        more: bool,
        result: Result<kentos_contracts::ProjectPage, ApiFailure>,
    },
    // ── Opening ─────────────────────────────────────────────────────────
    OpenProgress {
        id: u64,
        done: u64,
        total: u64,
    },
    Opened {
        id: u64,
        result: Result<Once<kentos_cloud::Opened>, ApiFailure>,
    },
    OpenCancel,
    /// The project read from the server with its copy rewritten, or read
    /// from the copy; or why it could not be (the copy comes back).
    Read {
        id: u64,
        result: Result<Once<opening::Read>, Once<(String, Option<kentos_cloud::Replica>)>>,
    },
    // ── A database project ──────────────────────────────────────────────
    DraftWritten {
        session: u64,
        /// The command the draft carried, to send now that it is on disk.
        send: bool,
        result: Result<(), ApiFailure>,
    },
    Committed {
        session: u64,
        result: Result<kentos_contracts::CommitResult, ApiFailure>,
    },
    Events {
        session: u64,
        result: Result<kentos_contracts::EventPage, ApiFailure>,
    },
    Fetched {
        session: u64,
        incoming: kentos_cloud::Incoming,
        full: bool,
        result: Result<kentos_cloud::Remote, ApiFailure>,
    },
    Access {
        session: u64,
        result: Result<ProjectInfo, ApiFailure>,
    },
    KeepMine,
    TakeTheirs,
    TheirInfo {
        session: u64,
        result: Result<ProjectInfo, ApiFailure>,
    },
    // ── A file project ──────────────────────────────────────────────────
    FileSaved {
        id: u64,
        result: Result<kentos_contracts::FileCommitted, ApiFailure>,
    },
    OpenLatest,
    /// A save kept on this device went to the server (the connection returned).
    KeptSent {
        session: u64,
        based_on: u64,
        result: Result<kentos_contracts::FileCommitted, ApiFailure>,
    },
    /// While offline: the account asked for, to learn whether the server answers.
    Probed(Result<Me, ApiFailure>),
    SaveCopy,
    // ── Buluta yükle ────────────────────────────────────────────────────
    UploadTenant(usize),
    UploadName(String),
    UploadStorage(kentos_contracts::ProjectStorage),
    UploadSubmit,
    UploadStop,
    UploadEncoded {
        id: u64,
        result: Result<Once<Vec<u8>>, String>,
    },
    Uploaded {
        id: u64,
        result: Result<(ProjectInfo, kentos_cloud::Uploaded), ApiFailure>,
    },
    /// Leaving a project: its draft was written, or could not be.
    Left {
        leave: Leave,
        result: Result<usize, ApiFailure>,
    },
    /// A window's Vazgeç or ×.
    Close,
    /// “Yerel kopya kaydet…” on an ended project's notice: Farklı kaydet.
    SaveLocal,
}

/// The cloud's part of the app.
#[derive(Default)]
pub struct CloudState {
    /// The connection to the server, made when signing in; its session lives
    /// in its memory only (kentos-cloud) and ends with the program.
    pub client: Option<Client>,
    /// The signed-in account and its workspaces.
    pub me: Option<Me>,
    /// Where device drafts go; none in tests, snapshots and the trace player,
    /// which never touch the user's files.
    pub drafts: Option<DraftStore>,
    pub sign_in: Option<SignIn>,
    pub catalog: Option<Catalog>,
    pub upload: Option<Upload>,
    pub opening: Option<Opening>,
    /// The open database project's autosave, draft and following.
    pub live: Option<Live>,
    pub file_conflict: Option<FileConflict>,
    /// Where this device keeps its copies of cloud projects (docs/adr/0043);
    /// none in tests, snapshots and the trace player.
    pub replicas: Option<kentos_cloud::ReplicaStore>,
    /// The open cloud project's copy, locked by this program.
    pub held: Option<copy::Held>,
    /// Whether the server answers (the status bar's dot).
    pub link: copy::Link,
    /// When the server is asked for next while offline, and the question on its way.
    pub probe_at: Option<Instant>,
    pub probing: Option<iced::task::Handle>,
    /// A copy about to be removed from this device, with the unsent work of its draft.
    pub removing: Option<catalog::Removing>,
    /// A leaving whose draft is being written.
    pub leaving: Option<Leave>,
    /// Why the last leaving's draft could not be written (the question says it).
    pub leave_failure: Option<String>,
    /// The name and storage of the project about to be opened (its progress says them).
    pub open_hint: Option<(String, kentos_contracts::ProjectStorage)>,
    next: u64,
}

impl CloudState {
    /// A new request's id.
    pub fn next_id(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    /// The signed-in account's membership of a workspace.
    pub fn membership(&self, tenant: &str) -> Option<&MembershipView> {
        self.me
            .as_ref()?
            .memberships
            .iter()
            .find(|m| m.tenant_id == tenant)
    }

    /// Whether anything waits on the timers.
    pub fn wants_ticks(&self) -> bool {
        self.live.is_some()
            || self.held.is_some()
            || (self.link == copy::Link::Offline && self.me.is_some())
            || self.catalog.as_ref().is_some_and(|c| c.search_at.is_some())
    }

    /// The connection, when signed in.
    pub fn signed_in(&self) -> Option<&Client> {
        self.client.as_ref().filter(|_| self.me.is_some())
    }
}

/// The desktop's place for the copies of cloud projects:
/// `$XDG_DATA_HOME/kentos-cad/bulut-kopya` (else `~/.local/share/…`), next to the drafts.
pub fn default_replicas() -> Option<kentos_cloud::ReplicaStore> {
    let recovery = crate::recovery::default_root()?;
    Some(kentos_cloud::ReplicaStore::new(
        recovery.with_file_name("bulut-kopya"),
    ))
}

/// The desktop's place for device drafts: `$XDG_DATA_HOME/kentos-cad/bulut-taslak`
/// (else `~/.local/share/…`), next to the recovery copies (docs/adr/0030).
pub fn default_drafts() -> Option<DraftStore> {
    let recovery = crate::recovery::default_root()?;
    Some(DraftStore::new(recovery.with_file_name("bulut-taslak")))
}

/// Every [`TICK`], while the subscription lives (something waits on a timer).
pub fn ticks() -> impl Stream<Item = Message> {
    let (mut out, ticks) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(TICK);
            if iced::futures::executor::block_on(out.send(crate::cloud::msg(Event::Tick))).is_err()
            {
                break;
            }
        }
    });
    ticks
}

/// A project's access in this account, from its info: whether it may change
/// the objects, and the name, settings and layers (an archived project may neither).
pub fn access_of(info: &ProjectInfo) -> (bool, bool) {
    let archived = info.state == ProjectState::Archived;
    let may = |p| !archived && info.access.permissions.contains(&p);
    (
        may(ProjectPermission::FeatureWrite),
        may(ProjectPermission::Edit),
    )
}

/// A project id from the server's text.
pub fn uuid(text: &str) -> Option<Uuid> {
    Uuid::parse_str(text).ok()
}

impl App {
    /// The cloud's messages.
    pub(crate) fn cloud_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Tick => self.cloud_tick(Instant::now()),
            Event::SignInServer(_)
            | Event::SignInLogin(_)
            | Event::SignInPassword(_)
            | Event::SignInSubmit
            | Event::SignedIn { .. }
            | Event::SignedOut(_) => self.account_event(event),
            Event::CatalogView(_)
            | Event::CatalogSearch(_)
            | Event::CatalogPick(_)
            | Event::CatalogOpen
            | Event::CatalogMore
            | Event::CatalogRetry
            | Event::CatalogRemove
            | Event::RemoveConfirmed
            | Event::CatalogPage { .. } => self.catalog_event(event),
            Event::OpenProgress { .. }
            | Event::Opened { .. }
            | Event::OpenCancel
            | Event::Read { .. } => self.opening_cloud_event(event),
            Event::DraftWritten { .. } | Event::Committed { .. } => self.live_event(event),
            Event::Events { .. }
            | Event::Fetched { .. }
            | Event::Access { .. }
            | Event::KeepMine
            | Event::TakeTheirs
            | Event::TheirInfo { .. } => self.follow_event(event),
            Event::FileSaved { .. }
            | Event::OpenLatest
            | Event::SaveCopy
            | Event::KeptSent { .. } => self.file_event(event),
            Event::UploadTenant(_)
            | Event::UploadName(_)
            | Event::UploadStorage(_)
            | Event::UploadSubmit
            | Event::UploadStop
            | Event::UploadEncoded { .. }
            | Event::Uploaded { .. } => self.upload_event(event),
            Event::Left { leave, result } => self.left(leave, result),
            Event::Probed(result) => self.probed(result),
            Event::Close => {
                self.close_dialog();
                Task::none()
            }
            Event::SaveLocal => {
                self.close_dialog();
                self.run("file.saveAs")
            }
        }
    }

    /// The cloud's commands (`cloud.*`, docs/adr/0041).
    pub(crate) fn cloud_command(&mut self, id: &str) -> Task<Message> {
        match id {
            "cloud.signIn" => {
                if let Some(me) = &self.cloud.me {
                    let name = me.user.display_name.clone();
                    self.output(format!(
                        "{name} olarak giriş yapılmış. Başka bir hesapla girmek için önce oturumu kapatın."
                    ));
                } else {
                    self.open_sign_in(None);
                }
                Task::none()
            }
            "cloud.signOut" => {
                if self.cloud.me.is_none() {
                    self.output("Bulut oturumu açık değil.");
                    return Task::none();
                }
                self.leave(Leave::SignOut)
            }
            "cloud.open" => {
                // Without a session this device's copies still open (docs/adr/0043).
                let offline = self.cloud.replicas.is_some() && self.cloud_identity().is_some();
                if self.cloud.me.is_none() && !offline {
                    self.open_sign_in(Some(account::Next::Catalog));
                    return Task::none();
                }
                self.open_catalog()
            }
            "cloud.upload" => {
                if self.document.is_none() {
                    self.output("Buluta yüklenecek çizim yok. Önce bir çizim açın (Ctrl+O).");
                    return Task::none();
                }
                if self.cloud.me.is_none() {
                    self.open_sign_in(Some(account::Next::Upload));
                    return Task::none();
                }
                self.leave(Leave::Upload)
            }
            "cloud.conflicts" => {
                self.show_conflicts();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// Whether a cloud command can run now (its button dims otherwise).
    pub(crate) fn cloud_available(&self, id: &str) -> bool {
        match id {
            "cloud.signIn" => self.cloud.me.is_none(),
            "cloud.signOut" => self.cloud.me.is_some(),
            "cloud.conflicts" => {
                self.cloud.file_conflict.is_some()
                    || self
                        .cloud
                        .live
                        .as_ref()
                        .is_some_and(|l| !l.sync.conflicts().is_empty())
            }
            _ => true,
        }
    }

    /// The timers: the catalog's search, then the open project's.
    pub(crate) fn cloud_tick(&mut self, now: Instant) -> Task<Message> {
        let due = match self.cloud.catalog.as_mut() {
            Some(c) if c.search_at.is_some_and(|at| now >= at) => {
                c.search_at = None;
                true
            }
            _ => false,
        };
        let search = if due {
            self.catalog_load(false)
        } else {
            Task::none()
        };
        Task::batch([search, self.live_tick(now), self.probe_tick(now)])
    }

    /// After every message: the open database project takes in the drawing's
    /// edits (its autosave and draft timers start), and a drawing that is no
    /// longer the project it followed leaves its machinery.
    pub(crate) fn cloud_after(&mut self, now: Instant) {
        let session = self.document.as_ref().map(|d| d.session);
        let cloud = self
            .document
            .as_ref()
            .is_some_and(|d| d.cloud_source().is_some());
        // Another drawing, or this one saved as a local file: its requests stop, its copy is let go.
        if self
            .cloud
            .held
            .as_ref()
            .is_some_and(|h| Some(h.session) != session || !cloud)
        {
            self.cloud.held = None;
        }
        let follows = match (&self.cloud.live, &self.document) {
            (Some(live), Some(doc)) => live.session == doc.session && doc.is_database(),
            (Some(_), None) => false,
            (None, _) => return,
        };
        if !follows {
            self.cloud.live = None;
            return;
        }
        self.live_observe(now);
    }

    /// Closes the window on top and whatever it holds (the password, a request).
    pub(crate) fn close_dialog(&mut self) {
        use crate::app::Dialog;
        match self.dialog.take() {
            Some(Dialog::SignIn) => self.cloud.sign_in = None,
            Some(Dialog::Catalog) => {
                // An open under way is stopped by its own Vazgeç; the window stays until then.
                if self.cloud.opening.is_some() {
                    self.dialog = Some(Dialog::Catalog);
                    return;
                }
                self.cloud.catalog = None;
            }
            Some(Dialog::Upload) => {
                if self.cloud.upload.as_ref().is_some_and(Upload::working) {
                    self.upload_stop();
                }
                self.cloud.upload = None;
            }
            Some(Dialog::Settings) => self.settings_draft = None,
            Some(Dialog::RemoveCopy) => {
                self.cloud.removing = None;
                if self.cloud.catalog.is_some() {
                    self.dialog = Some(Dialog::Catalog);
                }
            }
            Some(Dialog::Unsaved(then)) => self.unsaved_declined(then),
            Some(Dialog::Exchange) => self.exchange = None,
            Some(Dialog::Project) => self.project = None,
            _ => {}
        }
    }
}

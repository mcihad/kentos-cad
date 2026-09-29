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
//! | `catalog.rs` | “Bulut projeleri”: the lists, search, filters, pages, the selection |
//! | `catalog_view.rs` | the catalog window: its lists, the chosen list's rows, the questions |
//! | `catalog_pane.rs` | the catalog's selected project: facts, tabs, actions |
//! | `catalog_actions.rs` | the selected project's actions: favourite, archive, trash, restore, purge, download |
//! | `catalog_forms.rs` | the selected project's forms: Proje bilgileri, Kopyasını oluştur, the other storage mode |
//! | `catalog_forms_view.rs` | those forms drawn over the catalog |
//! | `forms_plan.rs` | what the forms say and decide (fixtures/cloud/v1/forms.json) |
//! | `catalog_history.rs` | the Geçmiş tab: revisions and checkpoints, followed; its forms and removal |
//! | `catalog_history_view.rs` | the Geçmiş tab's rows, its forms and its question |
//! | `opening.rs` | a project opened into the drawing, its progress |
//! | `leaving.rs` | leaving a project: its draft first, or the question |
//! | `copy.rs` | the project's local copy, work without a connection |
//! | `live.rs` | a database project's autosave and device draft |
//! | `follow.rs` | others' changes followed, access, conflicts resolved |
//! | `file.rs` | a file project's save and its conflict |
//! | `file_follow.rs` | a file project's events followed: another's revision, its end, a resync; the questions and answers |
//! | `revisions.rs` | what a file project knows of its revisions and what that means (fixtures/cloud/v1/file-revisions.json) |
//! | `upload.rs` | “Buluta yükle” |
//! | `view.rs` | the windows |
//! | `cells.rs` | the status bar's save and server cells, the account menu |
//! | `cells_plan.rs` | what those cells say and do (fixtures/cloud/v1/cells.json) |
//! | `plan.rs` | what the catalog shows and offers: rows, the selected project, questions, lines |
//! | `history.rs` | the history's rules: who may name, remove, download and restore |
//! | `share.rs` | “Projeyi paylaş”: the people, their roles, the invitations |
//! | `share_view.rs` | the share window: its tabs, forms, lists, the new link, its questions |
//! | `share_plan.rs` | what the share window says and offers (fixtures/cloud/v1/share.json) |
//! | `local_time.rs` | the device's local time for the lists' dates |
//! | `words.rs` | the interface's words: roles, lists, states, times |

mod account;
mod actions;
pub mod catalog;
mod catalog_actions;
mod catalog_forms;
mod catalog_forms_view;
mod catalog_history;
mod catalog_history_view;
mod catalog_pane;
mod catalog_view;
pub mod cells;
pub mod cells_plan;
#[cfg(test)]
mod cells_plan_tests;
#[cfg(test)]
mod cells_tests;
pub mod copy;
mod file;
mod file_follow;
#[cfg(test)]
mod file_follow_tests;
mod follow;
pub mod forms_plan;
#[cfg(test)]
mod forms_plan_tests;
pub mod history;
mod leaving;
mod live;
pub mod local_time;
mod opening;
pub mod plan;
pub mod revisions;
#[cfg(test)]
mod revisions_tests;
pub mod share;
pub mod share_plan;
#[cfg(test)]
mod share_plan_tests;
#[cfg(test)]
mod share_tests;
mod share_view;
mod upload;
mod view;
pub mod words;

#[cfg(test)]
mod catalog_forms_tests;
#[cfg(test)]
mod catalog_history_tests;
#[cfg(test)]
mod catalog_tests;
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
use kentos_contracts::{
    EventPage, Me, MembershipView, ProjectInfo, ProjectPermission, ProjectState, ProjectStorage,
};
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
    /// The type filter (none: “Tüm türler”), the order, “Kurum projeleri”'s organisation.
    CatalogKind(Option<kentos_contracts::ProjectType>),
    CatalogSort(kentos_contracts::CatalogSort),
    CatalogOrg(String),
    /// ↑ ↓ Home End in the list.
    CatalogStep(catalog::Step),
    CatalogTab(catalog::Tab),
    /// The row under the pointer.
    CatalogHover(Option<String>),
    CatalogDetails {
        id: u64,
        result: Result<catalog::Details, ApiFailure>,
    },
    /// An action on the selected project (catalog_actions.rs), its question answered, its answer.
    CatalogAct(catalog_actions::Act),
    CatalogAnswer(bool),
    CatalogActed {
        id: u64,
        result: Result<catalog_actions::Acted, ApiFailure>,
    },
    /// Where a download goes (none: the save window was closed), and how far it is.
    CatalogDownloadTo {
        id: u64,
        path: Option<std::path::PathBuf>,
    },
    CatalogDownloadProgress {
        id: u64,
        done: u64,
        total: u64,
    },
    // ── The Geçmiş tab (catalog_history.rs) ─────────────────────────────
    HistoryLoaded {
        id: u64,
        result: Result<history::HistoryData, ApiFailure>,
    },
    /// The project's events followed while the tab shows it.
    HistoryEvents {
        id: u64,
        result: Result<kentos_contracts::EventPage, ApiFailure>,
    },
    HistoryRetry,
    HistoryDownloadRevision(kentos_contracts::FileRevision),
    HistoryDownloadCheckpoint(kentos_contracts::Checkpoint),
    HistoryCreate,
    HistoryRestore(catalog_history::Point),
    HistoryRemove(kentos_contracts::Checkpoint),
    HistoryRemoveAnswer(bool),
    /// The forms' fields: a name, a note, the revision named, the new project's workspace.
    HistoryName(String),
    HistoryNote(iced::widget::text_editor::Action),
    HistoryRevision(usize),
    HistoryPlace(usize),
    HistorySubmit,
    HistoryFormClose,
    HistoryActed {
        id: u64,
        result: Result<catalog_history::HistoryActed, ApiFailure>,
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
    /// The server's tree after it refused ours (docs/adr/0081).
    ServerTree {
        session: u64,
        result: Result<ProjectInfo, ApiFailure>,
    },
    // ── A file project ──────────────────────────────────────────────────
    FileSaved {
        id: u64,
        result: Result<kentos_contracts::FileCommitted, ApiFailure>,
    },
    /// The open file project's events (file_follow.rs), the newest revision
    /// asked for, its access, and the project asked in a resync.
    FileEvents {
        session: u64,
        result: Result<EventPage, ApiFailure>,
    },
    FileNewest {
        session: u64,
        result: Result<kentos_contracts::FileRevisions, ApiFailure>,
    },
    FileAccess {
        session: u64,
        result: Result<ProjectInfo, ApiFailure>,
    },
    FileResync {
        session: u64,
        result: Result<ProjectInfo, ApiFailure>,
    },
    /// An answer to a question about the file project's revisions (revisions.rs).
    RevisionAnswer(revisions::Answer),
    /// A save kept on this device went to the server (the connection returned).
    KeptSent {
        session: u64,
        based_on: u64,
        result: Result<kentos_contracts::FileCommitted, ApiFailure>,
    },
    /// While offline: the account asked for, to learn whether the server answers.
    Probed(Result<Me, ApiFailure>),
    // ── Buluta yükle ────────────────────────────────────────────────────
    UploadTenant(usize),
    UploadName(String),
    UploadStorage(kentos_contracts::ProjectStorage),
    /// The catalog's fields of the new project: its type, description and tags.
    UploadType(kentos_contracts::ProjectType),
    UploadDescription(iced::widget::text_editor::Action),
    UploadTags(String),
    /// How much of the file the server has, of all of it.
    UploadProgress {
        id: u64,
        done: u64,
        total: u64,
    },
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
    // ── The open project's actions (actions.rs) ─────────────────────────
    RenameInput(String),
    RenameSubmit,
    Renamed {
        id: u64,
        result: Result<kentos_contracts::ProjectCatalogChange, ApiFailure>,
    },
    TrashConfirm,
    Trashed {
        id: u64,
        result: Result<kentos_contracts::ProjectCatalogChange, ApiFailure>,
    },
    /// A window's Vazgeç or ×.
    Close,
    /// “Yerel kopya kaydet…” on an ended project's notice: Farklı kaydet.
    SaveLocal,
    /// Projeyi paylaş (share.rs).
    Share(share::Event),
    /// The catalog's project forms (catalog_forms.rs).
    Form(catalog_forms::Event),
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
    /// The open file project followed: its events, a newer revision (file_follow.rs).
    pub file: Option<file_follow::FileFollow>,
    /// The question about the file project's revisions on screen (revisions.rs).
    pub question: Option<revisions::RevisionQuestion>,
    /// “Yerel dosyaya kaydet” chose Farklı kaydet for this drawing: once
    /// written, the log says it left the project (its name).
    pub detaching: Option<(u64, String)>,
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
    /// Yeniden adlandır's window (actions.rs).
    pub rename: Option<actions::Rename>,
    /// What waits for the open database project's autosave to send everything first.
    pub settling: Option<actions::Settle>,
    /// The trash request on its way.
    pub trashing: Option<u64>,
    /// The next open is the open project again after it was unarchived in the
    /// catalog, which stays on screen (catalog_actions.rs).
    pub reopen_keeps_catalog: bool,
    /// The next open is the open file project's newest revision (opening.rs `reopen_newest`).
    pub open_newest: bool,
    /// The open project was archived from this window: its end is not announced again.
    pub archived_by_me: bool,
    /// Projeyi paylaş, over the catalog or alone (share.rs).
    pub share: Option<share::Share>,
    /// The open file project's last save here: its drawing, revision and when (cells.rs).
    pub file_saved: Option<(u64, String, i64)>,
    /// Why the open file project's last save failed: its drawing, what the
    /// cell says (an error, the project gone, the access taken) and the reason.
    pub file_failed: Option<(u64, cells_plan::FileState, String)>,
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
            || self.file.is_some()
            || self.held.is_some()
            || (self.link == copy::Link::Offline && self.me.is_some())
            || self.catalog.as_ref().is_some_and(|c| {
                c.search_at.is_some() || c.details_at.is_some() || c.history.waits()
            })
            || self.share.as_ref().is_some_and(|s| s.search_at.is_some())
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

/// Now, in milliseconds since 1970 (the cells' “… önce”).
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
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
            | Event::CatalogPage { .. }
            | Event::CatalogKind(_)
            | Event::CatalogSort(_)
            | Event::CatalogOrg(_)
            | Event::CatalogStep(_)
            | Event::CatalogTab(_)
            | Event::CatalogHover(_)
            | Event::CatalogDetails { .. } => self.catalog_event(event),
            Event::CatalogAct(act) => self.catalog_act(act),
            Event::CatalogAnswer(yes) => self.catalog_answer(yes),
            Event::CatalogActed { id, result } => self.catalog_acted(id, result),
            Event::CatalogDownloadTo { id, path } => self.catalog_download_to(id, path),
            Event::CatalogDownloadProgress { id, done, total } => {
                self.catalog_download_progress(id, done, total);
                Task::none()
            }
            Event::HistoryDownloadRevision(r) => self.history_download_revision(r),
            Event::HistoryDownloadCheckpoint(c) => self.history_download_checkpoint(c),
            Event::HistoryLoaded { .. }
            | Event::HistoryEvents { .. }
            | Event::HistoryRetry
            | Event::HistoryCreate
            | Event::HistoryRestore(_)
            | Event::HistoryRemove(_)
            | Event::HistoryRemoveAnswer(_)
            | Event::HistoryName(_)
            | Event::HistoryNote(_)
            | Event::HistoryRevision(_)
            | Event::HistoryPlace(_)
            | Event::HistorySubmit
            | Event::HistoryFormClose
            | Event::HistoryActed { .. } => self.history_event(event),
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
            | Event::TheirInfo { .. }
            | Event::ServerTree { .. } => self.follow_event(event),
            Event::FileSaved { .. } | Event::KeptSent { .. } => self.file_event(event),
            Event::FileEvents { .. }
            | Event::FileNewest { .. }
            | Event::FileAccess { .. }
            | Event::FileResync { .. }
            | Event::RevisionAnswer(_) => self.file_follow_event(event),
            Event::UploadTenant(_)
            | Event::UploadName(_)
            | Event::UploadStorage(_)
            | Event::UploadType(_)
            | Event::UploadDescription(_)
            | Event::UploadTags(_)
            | Event::UploadProgress { .. }
            | Event::UploadSubmit
            | Event::UploadStop
            | Event::UploadEncoded { .. }
            | Event::Uploaded { .. } => self.upload_event(event),
            Event::RenameInput(_)
            | Event::RenameSubmit
            | Event::Renamed { .. }
            | Event::TrashConfirm
            | Event::Trashed { .. } => self.actions_event(event),
            Event::Share(event) => self.share_event(event),
            Event::Form(event) => self.project_form_event(event),
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
            "cloud.upload" | "cloud.uploadFile" => {
                // Buluta dosya olarak kaydet: the same window, on file storage (the web's).
                let storage = if id == "cloud.uploadFile" {
                    ProjectStorage::File
                } else {
                    ProjectStorage::Database
                };
                if self.document.is_none() {
                    self.output("Buluta yüklenecek çizim yok. Önce bir çizim açın (Ctrl+O).");
                    return Task::none();
                }
                if self.cloud.me.is_none() {
                    self.open_sign_in(Some(account::Next::Upload(storage)));
                    return Task::none();
                }
                self.leave(Leave::Upload(storage))
            }
            "cloud.rename" => self.open_rename(),
            "cloud.share" => self.share_open_project(),
            "cloud.delete" => {
                self.ask_trash();
                Task::none()
            }
            "cloud.openNewest" => self.offer_newest(),
            // Proje geçmişi…: the catalog on the open project's Geçmiş tab (the web's).
            "cloud.history" => {
                let Some(id) = self
                    .document
                    .as_ref()
                    .and_then(|d| d.cloud_source())
                    .map(|s| s.info.id.clone())
                else {
                    return Task::none();
                };
                self.open_catalog_at(Some((id, catalog::Tab::History)))
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
            // The open project's own actions (the web's `openMay`).
            "cloud.rename" => self.open_may(ProjectPermission::Edit, true),
            "cloud.delete" => self.open_may(ProjectPermission::Delete, false),
            "cloud.share" => self.open_may(ProjectPermission::Share, false),
            "cloud.history" => self.open_may(ProjectPermission::History, false),
            "cloud.openNewest" => {
                self.document
                    .as_ref()
                    .and_then(|d| d.cloud_source())
                    .is_some_and(|c| c.storage() == ProjectStorage::File)
                    && self.open_may(ProjectPermission::Read, false)
            }
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
        let details = self.catalog_details_tick(now);
        let history = self.history_tick(now);
        Task::batch([
            search,
            details,
            history,
            self.share_tick(now),
            self.live_tick(now),
            self.file_tick(now),
            self.probe_tick(now),
        ])
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
        // A file project followed: its drawing left, or saved as a local file.
        let file = self.document.as_ref().is_some_and(|d| {
            d.cloud_source()
                .is_some_and(|s| s.storage() == ProjectStorage::File)
        });
        if self
            .cloud
            .file
            .as_ref()
            .is_some_and(|f| Some(f.session) != session || !file)
        {
            self.cloud.file = None;
            self.cloud.question = None;
        }
        let follows = match (&self.cloud.live, &self.document) {
            (Some(live), Some(doc)) => live.session == doc.session && doc.is_database(),
            (Some(_), None) => false,
            (None, _) => return,
        };
        if !follows {
            self.say_waiting();
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
            Some(Dialog::Catalog) if self.cloud.share.is_some() => {
                // The share window over the catalog goes first, its question before it.
                if let Some(s) = self.cloud.share.as_mut().filter(|s| s.asking.is_some()) {
                    s.asking = None;
                } else {
                    self.cloud.share = None;
                }
                self.dialog = Some(Dialog::Catalog);
            }
            Some(Dialog::Share) => {
                if let Some(s) = self.cloud.share.as_mut().filter(|s| s.asking.is_some()) {
                    s.asking = None;
                    self.dialog = Some(Dialog::Share);
                } else {
                    self.cloud.share = None;
                }
            }
            Some(Dialog::Catalog)
                if self
                    .cloud
                    .catalog
                    .as_ref()
                    .is_some_and(|c| c.form.is_some()) =>
            {
                // A project form goes first, unless its request is on its way.
                self.project_form_close();
                self.dialog = Some(Dialog::Catalog);
            }
            Some(Dialog::Catalog) => {
                // A question over the window goes first (its Vazgeç); an open
                // under way is stopped by its own Vazgeç, the window stays until then.
                let asking = self.cloud.catalog.as_mut().is_some_and(|c| {
                    let h = &mut c.history;
                    if h.removing.take().is_some() {
                        return true;
                    }
                    // A form closes unless its request is on its way.
                    if let Some(f) = &h.form {
                        if !matches!(f, catalog_history::Form::Checkpoint(f) if f.busy)
                            && !matches!(f, catalog_history::Form::Restore(f) if f.busy)
                        {
                            h.form = None;
                        }
                        return true;
                    }
                    c.asking.take().is_some()
                });
                if asking || self.cloud.opening.is_some() {
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
            // The answer that changes nothing (revisions.rs `cancel`).
            Some(Dialog::Revision) => self.cloud.question = None,
            Some(Dialog::Exchange) => self.exchange = None,
            Some(Dialog::Project) => self.project = None,
            Some(Dialog::Processing) => self.processing.dialog = None,
            // With changes not applied, Katman stili asks first (style/layer_style/).
            Some(Dialog::LayerStyle) if !self.layer_style_may_close() => {
                self.dialog = Some(Dialog::LayerStyle);
            }
            // A question or a rename closes first; picking for Katman stili goes back to it.
            Some(Dialog::StyleManager) => self.style_manager_close_request(),
            Some(Dialog::Legend) => self.styles.legend = None,
            // Changes are asked about first; the window under it comes back (style/designer/).
            Some(Dialog::SymbolDesigner) => self.designer_close_request(),
            // A menu, a carried tool and a question go first; changes are asked about (processing/designer/).
            Some(Dialog::ModelDesigner) => {
                self.dialog = Some(Dialog::ModelDesigner);
                self.designer_close();
            }
            // A window over it, a question, then unsaved changes are asked about (style/svgedit/).
            Some(Dialog::SvgEditor) => self.svgedit_close_request(),
            Some(Dialog::BlockDefine) => self.block_define_closed(),
            _ => {}
        }
    }
}

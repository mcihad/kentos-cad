//! The catalog's Geçmiş tab behind the scenes (docs/adr/0087; the web's
//! historyPanel.ts and HistoryForms.ts): the selected project's revisions (a
//! file project) and checkpoints (with `project.history`) asked from the
//! server while the tab shows it, and asked again when its events say they
//! changed (the project's events followed with a long wait, docs/adr/0044);
//! what the rows offer: download, name a checkpoint, remove one after a
//! question, restore a point of the history as a new project, which the
//! catalog then shows in “Projelerim” and opens. The server decides every
//! request; a refusal is said in its own words.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use iced::Task;
use iced::task::Handle;
use iced::widget::text_editor;
use kentos_cloud::saving::envelope;
use kentos_cloud::{ApiFailure, follow};
use kentos_contracts::{
    CHECKPOINT_NAME_MAX, CatalogView, Checkpoint, CheckpointChange, CheckpointCreate,
    CheckpointDelete, CheckpointKind, CheckpointRestore, FileRevision, PROJECT_CHECKPOINT_CREATE,
    PROJECT_CHECKPOINT_CREATE_VERSION, PROJECT_CHECKPOINT_DELETE,
    PROJECT_CHECKPOINT_DELETE_VERSION, PROJECT_CHECKPOINT_RESTORE,
    PROJECT_CHECKPOINT_RESTORE_VERSION, ProjectDuplicated, ProjectPermission, ProjectState,
    ProjectStorage, ProjectSummary,
};
use kentos_domain::Uuid;
use kentos_interaction::{Level, js_trim};
use serde_json::json;

use crate::app::{App, Message};
use crate::cloud::actions::{Settle, reason};
use crate::cloud::catalog::{List, Said, Tab};
use crate::cloud::catalog_actions::{Download, Fetch};
use crate::cloud::history::{self, HistoryData, point_text};
use crate::cloud::{Event, uuid, words};

/// How long a burst of events rests before the history is asked again (the web's `SETTLE_MS`).
const SETTLE: Duration = Duration::from_millis(250);
/// After a failed wait, the next one.
const WATCH_AGAIN: Duration = Duration::from_secs(5);
/// The longest name a new project takes, in UTF-16 units (the web's field).
const PROJECT_NAME_MAX: usize = 200;

/// The history while it is on its way, or why it is not there.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum HistoryState {
    #[default]
    None,
    Loading,
    Loaded(HistoryData),
    Failed(String),
}

/// A point of the history restored as a new project.
#[derive(Debug, Clone, PartialEq)]
pub enum Point {
    Checkpoint(Checkpoint),
    Revision(FileRevision),
}

/// “Kontrol noktası oluştur”.
#[derive(Debug)]
pub struct CheckpointForm {
    pub name: String,
    pub note: text_editor::Content,
    /// A file project's revisions, newest first, and the one it names.
    pub revisions: Vec<FileRevision>,
    pub revision: usize,
    pub busy: bool,
    pub status: Option<Said>,
}

/// “Yeni proje olarak geri yükle”.
#[derive(Debug)]
pub struct RestoreForm {
    pub point: Point,
    pub name: String,
    /// The workspaces the account may open projects in (id, name), the project's first.
    pub places: Vec<(String, String)>,
    pub place: usize,
    pub busy: bool,
    pub status: Option<Said>,
}

/// A form over the window.
#[derive(Debug)]
pub enum Form {
    Checkpoint(CheckpointForm),
    Restore(Box<RestoreForm>),
}

/// What a form's or a removal's request answered.
#[derive(Debug, Clone)]
pub enum HistoryActed {
    Made(Box<CheckpointChange>),
    Removed(Box<CheckpointChange>),
    Restored(Box<ProjectDuplicated>),
}

/// The tab's state.
#[derive(Default)]
pub struct History {
    pub state: HistoryState,
    /// The project it shows (its id); none while the tab is hidden.
    shown: Option<String>,
    asked: Option<(u64, Handle)>,
    /// Following the project's events: the request on its way.
    watch: Option<(u64, Handle)>,
    /// The cursor to wait from, and when to wait again after a failure.
    cursor: Option<String>,
    watch_at: Option<Instant>,
    /// When the history is asked again after a burst of events.
    pub again_at: Option<Instant>,
    pub form: Option<Form>,
    /// A checkpoint about to be removed: the question over the window.
    pub removing: Option<Checkpoint>,
    /// A form's or a removal's request on its way, and the name of the
    /// checkpoint a removal is about (for its lines).
    acting: Option<(u64, Handle)>,
    removal: Option<String>,
}

impl History {
    /// Whether a form or a question holds the window.
    pub fn busy(&self) -> bool {
        self.form.is_some() || self.removing.is_some()
    }

    /// Whether a timer waits.
    pub fn waits(&self) -> bool {
        self.again_at.is_some() || self.watch_at.is_some()
    }

    /// The history asked, when its request is on its way (tests answer it).
    #[cfg(test)]
    pub(super) fn request(&self) -> Option<u64> {
        self.asked.as_ref().map(|(id, _)| *id)
    }

    /// The events followed, when a wait is on its way (tests answer it).
    #[cfg(test)]
    pub(super) fn watching(&self) -> Option<u64> {
        self.watch.as_ref().map(|(id, _)| *id)
    }

    /// The form's or the removal's request on its way (tests answer it).
    #[cfg(test)]
    pub(super) fn acting(&self) -> Option<u64> {
        self.acting.as_ref().map(|(id, _)| *id)
    }

    fn hide(&mut self) {
        *self = Self::default();
    }
}

/// The workspaces the account may open projects in, `first` first (the web's `creatableWorkspaces`).
pub(crate) fn creatable(app: &App, first: &str) -> Vec<(String, String)> {
    let Some(me) = &app.cloud.me else {
        return Vec::new();
    };
    let mut places: Vec<_> = me
        .memberships
        .iter()
        .filter(|m| m.active && m.seat && m.capabilities.iter().any(|c| c == "project.create"))
        .collect();
    places.sort_by_key(|m| m.tenant_id != first);
    places
        .into_iter()
        .map(|m| {
            (
                m.tenant_id.clone(),
                words::workspace(m.tenant_kind, &m.tenant_name, true),
            )
        })
        .collect()
}

impl App {
    /// The selected project for the history, when the tab shows one.
    fn history_project(&self) -> Option<ProjectSummary> {
        let c = self.cloud.catalog.as_ref()?;
        (c.tab == Tab::History && matches!(c.list, List::View(_)))
            .then(|| c.picked().cloned())
            .flatten()
            .filter(|p| p.state != ProjectState::Trashed)
    }

    /// After the tab or the selection changed: the history of the project
    /// shown, asked now and followed; or nothing (the web's `show` and `hide`).
    pub(crate) fn history_follow(&mut self) -> Task<Message> {
        let project = self.history_project();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        let Some(p) = project else {
            c.history.hide();
            return Task::none();
        };
        if c.history.shown.as_deref() == Some(p.id.as_str()) {
            return Task::none();
        }
        c.history.hide();
        c.history.shown = Some(p.id.clone());
        Task::batch([self.history_ask(false), self.history_watch()])
    }

    /// Asks the server; `quiet`: the list stays as it is until the answer (an event's refresh).
    pub(crate) fn history_ask(&mut self, quiet: bool) -> Task<Message> {
        let id = self.cloud.next_id();
        let client = self.cloud.signed_in().cloned();
        let project = self.history_project();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        let h = &mut c.history;
        let (Some(client), Some(p)) = (client, project) else {
            h.state = HistoryState::None;
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&p.tenant_id), uuid(&p.id)) else {
            h.state = HistoryState::Failed("sunucunun verdiği kimlik okunamadı".to_owned());
            return Task::none();
        };
        if !quiet || !matches!(h.state, HistoryState::Loaded(_)) {
            h.state = HistoryState::Loading;
        }
        let storage = p.storage;
        let revisions =
            (storage == ProjectStorage::File).then(|| client.file_revisions(tenant, project));
        let checkpoints = p
            .access
            .permissions
            .contains(&ProjectPermission::History)
            .then(|| client.checkpoints(tenant, project));
        let load = async move {
            let (revisions, checkpoints) = iced::futures::join!(
                async {
                    match revisions {
                        Some(f) => f.await.map(Some),
                        None => Ok(None),
                    }
                },
                async {
                    match checkpoints {
                        Some(f) => f.await.map(|c| Some(c.checkpoints)),
                        None => Ok(None),
                    }
                }
            );
            Ok(HistoryData {
                storage,
                revisions: revisions?,
                checkpoints: checkpoints?,
            })
        };
        let (task, handle) = Task::perform(load, move |result| {
            crate::cloud::msg(Event::HistoryLoaded { id, result })
        })
        .abortable();
        h.asked = Some((id, handle.abort_on_drop()));
        task
    }

    /// Follows the shown project's events: its cursor first, then waits
    /// from it; a checkpoint or a file revision asks the history again.
    fn history_watch(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let client = self.cloud.signed_in().cloned();
        let project = self.history_project();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        let h = &mut c.history;
        let (Some(client), Some(p)) = (client, project) else {
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&p.tenant_id), uuid(&p.id)) else {
            return Task::none();
        };
        h.watch_at = None;
        let (task, handle) = match h.cursor.clone() {
            Some(after) => Task::perform(
                follow::wait(&client, tenant, project, &after),
                move |result| crate::cloud::msg(Event::HistoryEvents { id, result }),
            )
            .abortable(),
            None => Task::perform(client.project(tenant, project), move |result| {
                crate::cloud::msg(Event::HistoryEvents {
                    id,
                    result: result.map(|info| kentos_contracts::EventPage {
                        events: Vec::new(),
                        next: info.event_cursor,
                    }),
                })
            })
            .abortable(),
        };
        h.watch = Some((id, handle.abort_on_drop()));
        task
    }

    /// The timers: the history asked again after events, the next wait after a failure.
    pub(crate) fn history_tick(&mut self, now: Instant) -> Task<Message> {
        let Some(h) = self.cloud.catalog.as_mut().map(|c| &mut c.history) else {
            return Task::none();
        };
        let again = h.again_at.is_some_and(|at| now >= at);
        let watch = h.watch_at.is_some_and(|at| now >= at);
        if again {
            h.again_at = None;
        }
        Task::batch([
            if again {
                self.history_ask(true)
            } else {
                Task::none()
            },
            if watch {
                self.history_watch()
            } else {
                Task::none()
            },
        ])
    }

    pub(crate) fn history_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::HistoryLoaded { id, result } => {
                let Some(h) = self.cloud.catalog.as_mut().map(|c| &mut c.history) else {
                    return Task::none();
                };
                if h.asked.as_ref().is_none_or(|(a, _)| *a != id) {
                    return Task::none();
                }
                h.asked = None;
                h.state = match result {
                    Ok(data) => HistoryState::Loaded(data),
                    Err(failure) => HistoryState::Failed(failure.message.clone()),
                };
                Task::none()
            }
            Event::HistoryEvents { id, result } => {
                let Some(h) = self.cloud.catalog.as_mut().map(|c| &mut c.history) else {
                    return Task::none();
                };
                if h.watch.as_ref().is_none_or(|(w, _)| *w != id) {
                    return Task::none();
                }
                h.watch = None;
                match result {
                    Ok(page) => {
                        if history::changes_history(&page.events) {
                            h.again_at = Some(Instant::now() + SETTLE);
                        }
                        h.cursor = Some(page.next);
                        self.history_watch()
                    }
                    Err(_) => {
                        h.watch_at = Some(Instant::now() + WATCH_AGAIN);
                        Task::none()
                    }
                }
            }
            Event::HistoryRetry => self.history_ask(false),
            Event::HistoryCreate => self.checkpoint_form(),
            Event::HistoryRestore(point) => self.restore_form(point),
            Event::HistoryRemove(c) => {
                if let Some(catalog) = self.cloud.catalog.as_mut() {
                    catalog.history.removing = Some(c);
                }
                Task::none()
            }
            Event::HistoryRemoveAnswer(yes) => self.checkpoint_remove(yes),
            Event::HistoryName(text) => {
                match self.form_mut() {
                    Some(Form::Checkpoint(f)) if !f.busy => f.name = text,
                    Some(Form::Restore(f)) if !f.busy => f.name = text,
                    _ => {}
                }
                Task::none()
            }
            Event::HistoryNote(action) => {
                if let Some(Form::Checkpoint(f)) = self.form_mut()
                    && !f.busy
                {
                    f.note.perform(action);
                }
                Task::none()
            }
            Event::HistoryRevision(i) => {
                if let Some(Form::Checkpoint(f)) = self.form_mut()
                    && !f.busy
                    && i < f.revisions.len()
                {
                    f.revision = i;
                }
                Task::none()
            }
            Event::HistoryPlace(i) => {
                if let Some(Form::Restore(f)) = self.form_mut()
                    && !f.busy
                    && i < f.places.len()
                {
                    f.place = i;
                }
                Task::none()
            }
            Event::HistorySubmit => self.form_submit(),
            Event::HistoryFormClose => {
                if let Some(c) = self.cloud.catalog.as_mut()
                    && c.history.form.as_ref().is_none_or(|f| !form_busy(f))
                {
                    c.history.form = None;
                }
                Task::none()
            }
            Event::HistoryActed { id, result } => self.history_acted(id, result),
            _ => Task::none(),
        }
    }

    /// A file project's revision as a `.kcad` (`project.download`).
    pub(crate) fn history_download_revision(&mut self, r: FileRevision) -> Task<Message> {
        let Some(p) = self.history_project() else {
            return Task::none();
        };
        if history::why_not_download(&p.access.permissions).is_some() {
            return Task::none();
        }
        let name = format!("{} (revizyon {})", p.name, r.revision);
        self.catalog_download(
            p,
            Download {
                fetch: Fetch::Revision(r.revision),
                name,
                listed: Some(r.sha256),
            },
        )
    }

    /// A checkpoint's file (`project.history` and `project.download`).
    pub(crate) fn history_download_checkpoint(&mut self, c: Checkpoint) -> Task<Message> {
        let Some(p) = self.history_project() else {
            return Task::none();
        };
        if history::why_not_take(&p.access.permissions, "indirme").is_some() {
            return Task::none();
        }
        let name = format!("{} - {}", p.name, c.name);
        self.catalog_download(
            p,
            Download {
                fetch: Fetch::Checkpoint(c.id),
                name,
                listed: Some(c.sha256),
            },
        )
    }

    fn form_mut(&mut self) -> Option<&mut Form> {
        self.cloud.catalog.as_mut()?.history.form.as_mut()
    }

    /// “Kontrol noktası oluştur…”: the form, when the account may name one.
    fn checkpoint_form(&mut self) -> Task<Message> {
        let Some(p) = self.history_project() else {
            return Task::none();
        };
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        let HistoryState::Loaded(data) = &c.history.state else {
            return Task::none();
        };
        if history::why_not_create(
            &p.access.permissions,
            p.state == ProjectState::Archived,
            data.storage,
            data.revisions.as_ref(),
        )
        .is_some()
        {
            return Task::none();
        }
        let revisions = data
            .revisions
            .as_ref()
            .map(|r| r.revisions.clone())
            .unwrap_or_default();
        c.history.form = Some(Form::Checkpoint(CheckpointForm {
            name: String::new(),
            note: text_editor::Content::new(),
            revisions,
            revision: 0,
            busy: false,
            status: None,
        }));
        iced::widget::operation::focus(iced::widget::Id::new(FORM_NAME))
    }

    /// “Yeni proje olarak geri yükle…”: the form, when the account may take the point.
    fn restore_form(&mut self, point: Point) -> Task<Message> {
        let Some(p) = self.history_project() else {
            return Task::none();
        };
        if history::why_not_take(&p.access.permissions, "geri yükleme").is_some() {
            return Task::none();
        }
        let places = creatable(self, &p.tenant_id);
        let status = places.is_empty().then(|| {
            Said::error(
                "Proje açabileceğiniz bir çalışma alanınız yok (project.create); kurum yöneticinize başvurun.",
            )
        });
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.history.form = Some(Form::Restore(Box::new(RestoreForm {
                point,
                name: String::new(),
                places,
                place: 0,
                busy: false,
                status,
            })));
        }
        iced::widget::operation::focus(iced::widget::Id::new(FORM_NAME))
    }

    fn form_submit(&mut self) -> Task<Message> {
        let Some(p) = self.history_project() else {
            return Task::none();
        };
        let open = self.is_open_project(&p.id) && self.cloud.live.is_some();
        let Some(form) = self.form_mut() else {
            return Task::none();
        };
        match form {
            Form::Checkpoint(f) => {
                let name = js_trim(&f.name).to_owned();
                if f.busy || name.is_empty() || name.chars().count() > CHECKPOINT_NAME_MAX {
                    return Task::none();
                }
                let note = js_trim(&f.note.text()).to_owned();
                let input = CheckpointCreate {
                    name,
                    note: (!note.is_empty()).then_some(note),
                    file_revision: (p.storage == ProjectStorage::File)
                        .then(|| f.revisions.get(f.revision).map(|r| r.revision.clone()))
                        .flatten(),
                };
                f.busy = true;
                f.status = Some(Said::info(if p.storage == ProjectStorage::File {
                    "Oluşturuluyor…"
                } else {
                    "Projenin görüntüsü alınıyor…"
                }));
                // The open project's unsent edits go first, so the checkpoint holds them.
                if open {
                    self.cloud.settling = Some(Settle::Checkpoint(Box::new((p, input))));
                    return self.flush();
                }
                self.checkpoint_send(&p, input)
            }
            Form::Restore(f) => {
                if f.busy || f.places.is_empty() {
                    return Task::none();
                }
                let name = js_trim(&f.name).to_owned();
                if name.encode_utf16().count() > PROJECT_NAME_MAX {
                    f.status = Some(Said::error("Proje adı en çok 200 karakter olabilir."));
                    return Task::none();
                }
                let (checkpoint_id, file_revision) = match &f.point {
                    Point::Checkpoint(c) => (Some(c.id.clone()), None),
                    Point::Revision(r) => (None, Some(r.revision.clone())),
                };
                let input = CheckpointRestore {
                    checkpoint_id,
                    file_revision,
                    name: (!name.is_empty()).then_some(name),
                    tenant_id: f.places.get(f.place).map(|(id, _)| id.clone()),
                };
                f.busy = true;
                f.status = Some(Said::info("Geri yükleniyor…"));
                self.history_command(
                    &p,
                    PROJECT_CHECKPOINT_RESTORE,
                    PROJECT_CHECKPOINT_RESTORE_VERSION,
                    serde_json::to_value(input).unwrap_or(json!({})),
                    |value: ProjectDuplicated| HistoryActed::Restored(Box::new(value)),
                )
            }
        }
    }

    /// Names the checkpoint (after the open project's unsent edits went).
    pub(crate) fn checkpoint_send(
        &mut self,
        p: &ProjectSummary,
        input: CheckpointCreate,
    ) -> Task<Message> {
        self.history_command(
            p,
            PROJECT_CHECKPOINT_CREATE,
            PROJECT_CHECKPOINT_CREATE_VERSION,
            serde_json::to_value(input).unwrap_or(json!({})),
            |value: CheckpointChange| HistoryActed::Made(Box::new(value)),
        )
    }

    fn checkpoint_remove(&mut self, yes: bool) -> Task<Message> {
        let Some(c) = self
            .cloud
            .catalog
            .as_mut()
            .and_then(|c| c.history.removing.take())
        else {
            return Task::none();
        };
        let Some(p) = self.history_project().filter(|_| yes) else {
            return Task::none();
        };
        let task = self.history_command(
            &p,
            PROJECT_CHECKPOINT_DELETE,
            PROJECT_CHECKPOINT_DELETE_VERSION,
            serde_json::to_value(CheckpointDelete {
                checkpoint_id: c.id.clone(),
            })
            .unwrap_or(json!({})),
            |value: CheckpointChange| HistoryActed::Removed(Box::new(value)),
        );
        if let Some(catalog) = self.cloud.catalog.as_mut() {
            catalog.history.removal = Some(c.name);
        }
        task
    }

    /// One history command on `p`, its answer as a [`HistoryActed`].
    fn history_command<T: serde::de::DeserializeOwned + Send + 'static>(
        &mut self,
        p: &ProjectSummary,
        name: &str,
        version: u32,
        input: serde_json::Value,
        wrap: fn(T) -> HistoryActed,
    ) -> Task<Message> {
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&p.tenant_id), uuid(&p.id)) else {
            return Task::none();
        };
        let id = self.cloud.next_id();
        let request = envelope(
            tenant,
            project,
            name,
            version,
            Uuid::new_v4(),
            BTreeMap::new(),
            input,
        );
        let (task, handle) = Task::perform(client.command::<T>(request), move |result| {
            crate::cloud::msg(Event::HistoryActed {
                id,
                result: result.map(wrap),
            })
        })
        .abortable();
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.history.acting = Some((id, handle.abort_on_drop()));
        }
        task
    }

    fn history_acted(
        &mut self,
        id: u64,
        result: Result<HistoryActed, ApiFailure>,
    ) -> Task<Message> {
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        if c.history.acting.as_ref().is_none_or(|(a, _)| *a != id) {
            return Task::none();
        }
        c.history.acting = None;
        let removal = c.history.removal.take();
        match result {
            Ok(HistoryActed::Made(change)) => {
                c.history.form = None;
                self.say(
                    Level::Success,
                    format!(
                        "“{}” kontrol noktası oluşturuldu: {}.",
                        change.checkpoint.name,
                        point_text(&change.checkpoint)
                    ),
                );
                self.history_ask(false)
            }
            Ok(HistoryActed::Removed(change)) => {
                self.say(
                    Level::Success,
                    format!("“{}” kontrol noktası silindi.", change.checkpoint.name),
                );
                self.history_ask(false)
            }
            Ok(HistoryActed::Restored(made)) => {
                let what = match c.history.form.take() {
                    Some(Form::Restore(f)) => point_what(&f.point),
                    _ => String::new(),
                };
                self.say(
                    Level::Success,
                    format!(
                        "“{}” oluşturuldu: {what} geri yüklendi ({} nesne).",
                        made.project.name,
                        words::grouped(&made.objects)
                    ),
                );
                self.catalog_show_made(&made.project.id)
            }
            Err(failure) => {
                let why = reason(&failure);
                match c.history.form.as_mut() {
                    Some(Form::Checkpoint(f)) => {
                        f.busy = false;
                        f.status = Some(Said::error(why));
                    }
                    Some(Form::Restore(f)) => {
                        f.busy = false;
                        f.status = Some(Said::error(why));
                    }
                    // A removal's refusal is said in the log (the web's `remove`).
                    None => self.error(format!(
                        "“{}” kontrol noktası silinemedi: {why}",
                        removal.unwrap_or_default()
                    )),
                }
                Task::none()
            }
        }
    }

    /// A project made from the history: shown selected in “Projelerim” and
    /// opened as soon as the list shows it (the web's `openMade`).
    fn catalog_show_made(&mut self, id: &str) -> Task<Message> {
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        c.wanted = Some((id.to_owned(), true));
        c.tab = Tab::Info;
        // Whatever was being searched: the new project is the one to show.
        c.search.clear();
        c.search_at = None;
        c.kind = None;
        self.catalog_event(Event::CatalogView(List::View(CatalogView::Mine)))
    }
}

/// Whether a form waits for its answer.
fn form_busy(f: &Form) -> bool {
    match f {
        Form::Checkpoint(f) => f.busy,
        Form::Restore(f) => f.busy,
    }
}

/// What a point is, in the lines: “Belediyeye teslim” kontrol noktası
/// (veri revizyonu 4), or revizyon 3.
pub(crate) fn point_what(point: &Point) -> String {
    match point {
        Point::Checkpoint(c) => format!("“{}” kontrol noktası ({})", c.name, point_text(c)),
        Point::Revision(r) => format!("revizyon {}", r.revision),
    }
}

/// What the new project restored from `point` is kept as: a file project's
/// point is a file project, a database checkpoint a database project.
pub(crate) fn restored_storage(point: &Point) -> ProjectStorage {
    match point {
        Point::Checkpoint(c) if c.kind == CheckpointKind::Snapshot => ProjectStorage::Database,
        _ => ProjectStorage::File,
    }
}

/// The forms' first field, focused when they open.
pub(crate) const FORM_NAME: &str = "cloud-history-name";

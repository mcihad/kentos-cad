//! The catalog's Sunucu işleri tab behind the scenes (docs/adr/0547; the server's side is docs/adr/0542): the
//! selected project's server jobs asked from the server while the tab shows it, and asked again every
//! `server_jobs::POLL_ACTIVE` while one waits or runs (the desktop polls and has no WebSocket, docs/adr/0044);
//! what the tab offers: ask for the project as one `.kcad` file (“Projeyi dışa aktar”), stop a job, download a
//! succeeded export's file (the save window and the safe write of every other download, with `Range` resuming).
//!
//! The server keeps the jobs: the window or the app closed, the job goes on and the next look finds it. The server
//! decides every request; a refusal (the organisation's limit too) is said in its own words. What is not a pixel
//! (the request, who may do what, the words) is `kentos_cloud::server_jobs`.

use std::time::{Duration, Instant};

use iced::Task;
use iced::task::Handle;
use kentos_cloud::{ApiFailure, server_jobs};
use kentos_commands::{ProjectState, ProjectSummary, ServerJob, ServerJobList, ServerJobState};
use kentos_interaction::Level;

use crate::app::{App, Message};
use crate::cloud::actions::{Settle, reason};
use crate::cloud::catalog::{List, Said, Tab};
use crate::cloud::catalog_actions::{Download, Fetch};
use crate::cloud::{Event, uuid};

/// After a failed look, the next one.
const RETRY_AFTER: Duration = Duration::from_secs(5);

/// The jobs while they are on their way, or why they are not there.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum JobsState {
    #[default]
    None,
    Loading,
    Loaded(Vec<ServerJob>),
    Failed(String),
}

/// What a request of the tab on its way is.
#[derive(Debug, Clone, PartialEq)]
pub enum Working {
    /// A job being asked for.
    Asking,
    /// A job being stopped (its id).
    Stopping(String),
}

/// The tab's state.
#[derive(Default)]
pub struct Jobs {
    pub state: JobsState,
    /// The project it shows (its id); none while the tab is hidden.
    shown: Option<String>,
    /// The list request on its way (an older answer is dropped).
    asked: Option<(u64, Handle)>,
    /// When the list is looked at again.
    pub again_at: Option<Instant>,
    /// The request to ask for a job or to stop one, on its way.
    acting: Option<(u64, Handle)>,
    pub working: Option<Working>,
    /// The request id of an export asked and not answered yet: sent again after a failure with the same id, so a
    /// request whose answer was lost never makes a second job (the server gives the first one back).
    request: Option<String>,
    /// What the last ask, stop or look said.
    pub status: Option<Said>,
}

impl Jobs {
    /// Whether a timer waits.
    pub fn waits(&self) -> bool {
        self.again_at.is_some()
    }

    /// Whether a request of the tab is on its way.
    pub fn busy(&self) -> bool {
        self.working.is_some()
    }

    /// The jobs it shows, newest first.
    pub fn list(&self) -> &[ServerJob] {
        match &self.state {
            JobsState::Loaded(jobs) => jobs,
            _ => &[],
        }
    }

    /// The list request on its way (tests answer it).
    #[cfg(test)]
    pub(super) fn request(&self) -> Option<u64> {
        self.asked.as_ref().map(|(id, _)| *id)
    }

    /// The ask or stop request on its way (tests answer it).
    #[cfg(test)]
    pub(super) fn acting(&self) -> Option<u64> {
        self.acting.as_ref().map(|(id, _)| *id)
    }

    /// The request id kept for an export asked and not answered (tests look at it).
    #[cfg(test)]
    pub(super) fn pending_request(&self) -> Option<&str> {
        self.request.as_deref()
    }

    fn hide(&mut self) {
        *self = Self::default();
    }

    /// A job the server answered with, in its place (or first, when it is new).
    fn put(&mut self, job: ServerJob) {
        if let JobsState::Loaded(list) = &mut self.state {
            match list.iter_mut().find(|j| j.id == job.id) {
                Some(slot) => *slot = job,
                None => list.insert(0, job),
            }
        } else {
            self.state = JobsState::Loaded(vec![job]);
        }
    }
}

impl App {
    /// The selected project for the jobs, when the tab shows one.
    pub(crate) fn jobs_project(&self) -> Option<ProjectSummary> {
        let c = self.cloud.catalog.as_ref()?;
        (c.tab == Tab::Jobs && matches!(c.list, List::View(_)))
            .then(|| c.picked().cloned())
            .flatten()
            .filter(|p| p.state != ProjectState::Trashed)
    }

    /// After the tab or the selection changed: the jobs of the project shown, asked now; or nothing.
    pub(crate) fn jobs_follow(&mut self) -> Task<Message> {
        let project = self.jobs_project();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        let Some(p) = project else {
            c.jobs.hide();
            return Task::none();
        };
        if c.jobs.shown.as_deref() == Some(p.id.as_str()) {
            return Task::none();
        }
        c.jobs.hide();
        c.jobs.shown = Some(p.id.clone());
        self.jobs_ask(false)
    }

    /// Asks the server for the list; `quiet`: the list stays as it is until the answer (a poll).
    pub(crate) fn jobs_ask(&mut self, quiet: bool) -> Task<Message> {
        let id = self.cloud.next_id();
        let client = self.cloud.signed_in().cloned();
        let project = self.jobs_project();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        let j = &mut c.jobs;
        let (Some(client), Some(p)) = (client, project) else {
            j.state = JobsState::None;
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&p.tenant_id), uuid(&p.id)) else {
            j.state = JobsState::Failed("sunucunun verdiği kimlik okunamadı".to_owned());
            return Task::none();
        };
        if !quiet || !matches!(j.state, JobsState::Loaded(_)) {
            j.state = JobsState::Loading;
        }
        j.again_at = None;
        let (task, handle) = Task::perform(client.server_jobs(tenant, project), move |result| {
            crate::cloud::msg(Event::JobsLoaded { id, result })
        })
        .abortable();
        j.asked = Some((id, handle.abort_on_drop()));
        task
    }

    /// The timer: the list looked at again while a job is under way.
    pub(crate) fn jobs_tick(&mut self, now: Instant) -> Task<Message> {
        let due = self
            .cloud
            .catalog
            .as_mut()
            .map(|c| &mut c.jobs)
            .is_some_and(|j| j.again_at.is_some_and(|at| now >= at));
        if due {
            self.jobs_ask(true)
        } else {
            Task::none()
        }
    }

    pub(crate) fn jobs_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::JobsLoaded { id, result } => {
                let Some(j) = self.cloud.catalog.as_mut().map(|c| &mut c.jobs) else {
                    return Task::none();
                };
                if j.asked.as_ref().is_none_or(|(a, _)| *a != id) {
                    return Task::none();
                }
                j.asked = None;
                let mut ended: Vec<ServerJob> = Vec::new();
                match result {
                    Ok(ServerJobList { jobs }) => {
                        j.status = None;
                        // Jobs seen under way that are over now: the log says so once.
                        ended = jobs
                            .iter()
                            .filter(|new| {
                                new.state.is_over()
                                    && j.list()
                                        .iter()
                                        .any(|old| old.id == new.id && !old.state.is_over())
                            })
                            .cloned()
                            .collect();
                        j.again_at = server_jobs::poll_after(&jobs).map(|d| Instant::now() + d);
                        j.state = JobsState::Loaded(jobs);
                    }
                    // A poll that fails keeps the list and looks again; the first look says why it failed.
                    Err(failure) if matches!(j.state, JobsState::Loaded(_)) => {
                        j.status = Some(Said::error(format!(
                            "İşler yenilenemedi: {}; yeniden denenecek.",
                            failure.message
                        )));
                        j.again_at = Some(Instant::now() + RETRY_AFTER);
                    }
                    Err(failure) => j.state = JobsState::Failed(failure.message.clone()),
                }
                for job in &ended {
                    self.jobs_said_done(job);
                }
                Task::none()
            }
            Event::JobsRetry => self.jobs_ask(true),
            Event::JobExport => self.jobs_export(),
            Event::JobAsked { id, result } => self.jobs_asked(id, result),
            Event::JobStop(job) => self.jobs_stop(job),
            Event::JobStopped { id, result } => self.jobs_stopped(id, result),
            _ => Task::none(),
        }
    }

    /// “Projeyi dışa aktar (.kcad)”: the open project's unsent edits go first (so the file holds them), then the
    /// server is asked.
    fn jobs_export(&mut self) -> Task<Message> {
        let Some(p) = self.jobs_project() else {
            return Task::none();
        };
        if self.cloud.catalog.as_ref().is_none_or(|c| c.jobs.busy()) {
            return Task::none();
        }
        if server_jobs::why_not_export(&p.access.permissions, p.storage, p.state).is_some() {
            return Task::none();
        }
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.jobs.working = Some(Working::Asking);
            c.jobs.status = Some(Said::info("İş isteniyor…"));
        }
        if self.is_open_project(&p.id) && self.cloud.live.is_some() {
            self.cloud.settling = Some(Settle::Job(Box::new(p)));
            return self.flush();
        }
        self.jobs_send_export(&p)
    }

    /// Asks for the export (after the open project's unsent edits went).
    pub(crate) fn jobs_send_export(&mut self, p: &ProjectSummary) -> Task<Message> {
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let id = self.cloud.next_id();
        let (Some(tenant), Some(project)) = (uuid(&p.tenant_id), uuid(&p.id)) else {
            return Task::none();
        };
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        // A failed ask is sent again with the same request id, so it makes one job whatever the retries.
        let request = c
            .jobs
            .request
            .get_or_insert_with(server_jobs::request_id)
            .clone();
        let (task, handle) = Task::perform(
            client.create_server_job(tenant, project, server_jobs::export_request(&request, None)),
            move |result| crate::cloud::msg(Event::JobAsked { id, result }),
        )
        .abortable();
        c.jobs.working = Some(Working::Asking);
        c.jobs.acting = Some((id, handle.abort_on_drop()));
        task
    }

    fn jobs_asked(&mut self, id: u64, result: Result<ServerJob, ApiFailure>) -> Task<Message> {
        let Some(j) = self.cloud.catalog.as_mut().map(|c| &mut c.jobs) else {
            return Task::none();
        };
        if j.acting.as_ref().is_none_or(|(a, _)| *a != id) {
            return Task::none();
        }
        j.acting = None;
        j.working = None;
        match result {
            Ok(job) => {
                j.request = None;
                j.status = Some(Said::info(
                    "İş istendi; sunucuda çalışır. Pencereyi kapatsanız da sürer, bitince buradan indirin.",
                ));
                j.again_at = Some(Instant::now() + server_jobs::POLL_ACTIVE);
                j.put(job);
            }
            Err(failure) => {
                // The request id stays: asking again gives the same job if the server did make it.
                j.status = Some(Said::error(format!("İş istenemedi: {}", reason(&failure))));
            }
        }
        Task::none()
    }

    fn jobs_stop(&mut self, job: String) -> Task<Message> {
        let Some(p) = self.jobs_project() else {
            return Task::none();
        };
        let me = self.cloud.me.as_ref().map(|m| m.user.id.clone());
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let id = self.cloud.next_id();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        if c.jobs.busy() {
            return Task::none();
        }
        let Some(row) = c.jobs.list().iter().find(|j| j.id == job).cloned() else {
            return Task::none();
        };
        if server_jobs::why_not_stop(&row, me.as_deref(), &p.access.permissions).is_some() {
            return Task::none();
        }
        let (Some(tenant), Some(project), Some(job_id)) =
            (uuid(&p.tenant_id), uuid(&p.id), uuid(&job))
        else {
            return Task::none();
        };
        let (task, handle) = Task::perform(
            client.cancel_server_job(tenant, project, job_id),
            move |result| crate::cloud::msg(Event::JobStopped { id, result }),
        )
        .abortable();
        c.jobs.working = Some(Working::Stopping(job));
        c.jobs.status = Some(Said::info("İş durduruluyor…"));
        c.jobs.acting = Some((id, handle.abort_on_drop()));
        task
    }

    fn jobs_stopped(&mut self, id: u64, result: Result<ServerJob, ApiFailure>) -> Task<Message> {
        let Some(j) = self.cloud.catalog.as_mut().map(|c| &mut c.jobs) else {
            return Task::none();
        };
        if j.acting.as_ref().is_none_or(|(a, _)| *a != id) {
            return Task::none();
        }
        j.acting = None;
        j.working = None;
        match result {
            Ok(job) => {
                j.status = Some(Said::info(if job.state.is_over() {
                    "İş durduruldu."
                } else {
                    "Durdurma istendi; işi çalıştıran bir sonraki yoklamada durdurur."
                }));
                // A running job is looked at soon, until the worker stops it.
                j.again_at = Some(Instant::now() + server_jobs::POLL_ACTIVE);
                j.put(job);
            }
            Err(failure) => {
                j.status = Some(Said::error(format!(
                    "İş durdurulamadı: {}",
                    reason(&failure)
                )));
            }
        }
        Task::none()
    }

    /// “İndir”: a succeeded export's file, by the catalog's download (the save window, the checked bytes, the
    /// whole-or-nothing write).
    pub(crate) fn jobs_download(&mut self, job: ServerJob) -> Task<Message> {
        let Some(p) = self.jobs_project() else {
            return Task::none();
        };
        if server_jobs::why_not_download(&job, &p.access.permissions).is_some() {
            return Task::none();
        }
        let Some(artifact) = job.artifact.clone() else {
            return Task::none();
        };
        let name = server_jobs::download_name(&job);
        self.catalog_download(
            p,
            Download {
                fetch: Fetch::Job(job.id.clone()),
                name,
                listed: Some(artifact.sha256),
                compare: None,
                extension: server_jobs::download_extension(&job),
            },
        )
    }

    /// A line the log keeps when a job ends while it is watched (the first look at a finished job is silent).
    fn jobs_said_done(&mut self, job: &ServerJob) {
        let (level, what) = match job.state {
            ServerJobState::Succeeded => (Level::Success, "bitti"),
            ServerJobState::Cancelled => (Level::Info, "durduruldu"),
            _ => (Level::Error, "başarısız oldu"),
        };
        self.say(
            level,
            format!("Sunucu işi {what}: {}.", server_jobs::kind_label(job)),
        );
    }
}

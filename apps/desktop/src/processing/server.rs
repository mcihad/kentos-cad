//! Sunucuda çalıştır (docs/adr/0559 §5): an İşlemler tool run as a durable job on the KentOS server, from the tool's own
//! window. “Nerede çalışır” lists “KentOS sunucusunda” for the open cloud database project (off, with why, when the
//! account may not run jobs there); chosen, Çalıştır asks the server for a `project.processing` job with the window's
//! values, the objects chosen and the box on screen, after the open project's unsent edits went, and the window says
//! the job is asked and where to watch it: the catalog's Sunucu işleri tab, where the job's progress and words, its
//! result and its file are. The job goes on in the server whatever happens to the window or the app.
//!
//! The tool does not run here at all: nothing is written to the open drawing by the ask. A result committed to the
//! project comes back as the project's own change (the autosave's events, docs/adr/0044).

use iced::Task;
use kentos_cloud::{ApiFailure, server_jobs};
use kentos_commands::{
    ProcessingJobInput, ProcessingJobMode, ProjectPermission, ProjectState, ProjectStorage,
    ServerJob,
};
use kentos_interaction::Level;
use kentos_processing::command::command_values;
use kentos_processing::model_runner::MODEL_PREFIX;

use super::RunStatus;
use super::plan::{Choice, ServerPlace, effective_choice};
use crate::app::{App, Message};
use crate::cloud::actions::{Settle, reason};
use crate::cloud::catalog::Tab;
use crate::cloud::{Event, uuid};

impl App {
    /// Where the server stands for the open drawing: not a cloud database project (not listed), a project the account
    /// may run jobs in, or one it may not (listed off, a few words why).
    pub(crate) fn server_place(&self) -> ServerPlace {
        let Some(source) = self.document.as_ref().and_then(|d| d.cloud_source()) else {
            return ServerPlace::Absent;
        };
        if source.storage() != ProjectStorage::Database {
            return ServerPlace::Absent;
        }
        if self.cloud.signed_in().is_none() {
            return ServerPlace::Off("oturum açık değil".to_owned());
        }
        if source.info.state == ProjectState::Archived {
            return ServerPlace::Off("proje arşivde".to_owned());
        }
        if !source
            .info
            .access
            .permissions
            .contains(&ProjectPermission::JobsRun)
        {
            return ServerPlace::Off("yetkiniz yok (project.jobs.run)".to_owned());
        }
        ServerPlace::Ready
    }

    /// Whether the window's run goes to the server: the place chosen and in reach.
    pub(crate) fn processing_at_server(&self) -> bool {
        let Some(window) = &self.processing.dialog else {
            return false;
        };
        let targets = window.targets(&self.processing.registry, &self.server_place());
        let choice = effective_choice(
            Choice::read(self.processing.memory.target(&window.tool.id)),
            &targets.available,
        );
        choice == Some(Choice::At(kentos_processing::Target::Server))
    }

    /// The job the window's values ask for, as `cad.processing.run` takes them: the tool and its values, the objects
    /// chosen (persistent ids), the box on screen, and where the result goes. None for a model (it runs here).
    pub(crate) fn processing_server_input(&self) -> Option<ProcessingJobInput> {
        let (doc, window) = (self.document.as_ref()?, self.processing.dialog.as_ref()?);
        if window.tool.id.starts_with(MODEL_PREFIX) {
            return None;
        }
        let selection: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        let v = self.viewport.camera.visible_bounds();
        Some(ProcessingJobInput {
            tool: window.tool.id.clone(),
            label: Some(window.tool.label.clone()),
            parameters: command_values(&doc.model, &window.tool, &window.values),
            selection,
            view: Some([v.min_x, v.min_y, v.max_x, v.max_y]),
            mode: if window.server_sink {
                ProcessingJobMode::Sink
            } else {
                ProcessingJobMode::Commit
            },
            ..ProcessingJobInput::default()
        })
    }

    /// Çalıştır with the server chosen: the open project's unsent edits go first, then the job is asked.
    pub(super) fn processing_server_run(&mut self) -> Task<Message> {
        let Some(input) = self.processing_server_input() else {
            if let Some(window) = &mut self.processing.dialog {
                window.status = RunStatus::Error(
                    "Modeller sunucuda çalışmaz; adımlarını tek tek çalıştırın ya da modeli bu bilgisayarda çalıştırın."
                        .to_owned(),
                );
            }
            return Task::none();
        };
        if let Some(window) = &mut self.processing.dialog {
            window.status = RunStatus::Asked("Sunucu işi isteniyor…".to_owned());
        }
        if self.cloud.live.is_some() {
            self.cloud.settling = Some(Settle::ProcessingJob(Box::new(input)));
            return self.flush();
        }
        self.processing_job_send(input)
    }

    /// Asks the server for the job (after the open project's unsent edits went).
    pub(crate) fn processing_job_send(&mut self, input: ProcessingJobInput) -> Task<Message> {
        let (Some(client), Some(source)) = (
            self.cloud.signed_in().cloned(),
            self.document.as_ref().and_then(|d| d.cloud_source()),
        ) else {
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&source.info.tenant_id), uuid(&source.info.id))
        else {
            return Task::none();
        };
        let id = self.cloud.next_id();
        // A failed ask is sent again with the same request id, so it makes one job whatever the retries.
        let request = self
            .processing
            .server_request
            .get_or_insert_with(server_jobs::request_id)
            .clone();
        self.processing.server_asked = Some(id);
        Task::perform(
            client.create_server_job(
                tenant,
                project,
                server_jobs::processing_request(&request, &input),
            ),
            move |result| crate::cloud::msg(Event::ProcessingJobAsked { id, result }),
        )
    }

    /// The server's answer to the ask.
    pub(crate) fn processing_job_asked(
        &mut self,
        id: u64,
        result: Result<ServerJob, ApiFailure>,
    ) -> Task<Message> {
        if self.processing.server_asked != Some(id) {
            return Task::none();
        }
        self.processing.server_asked = None;
        match result {
            Ok(job) => {
                self.processing.server_request = None;
                let what = server_jobs::kind_label(&job);
                if let Some(window) = &mut self.processing.dialog {
                    window.status = RunStatus::Asked(format!(
                        "Sunucu işi istendi: {what}. Sunucuda çalışır; pencereyi kapatsanız da sürer."
                    ));
                }
                self.say(Level::Info, format!("Sunucu işi istendi: {what}."));
            }
            Err(failure) => {
                // The request id stays: asking again gives the same job if the server did make it.
                let why = format!("Sunucu işi istenemedi: {}", reason(&failure));
                if let Some(window) = &mut self.processing.dialog {
                    window.status = RunStatus::Error(why);
                }
            }
        }
        Task::none()
    }

    /// “Sunucu işleri” on the window's line: the catalog on the open project's jobs.
    pub(super) fn processing_open_jobs(&mut self) -> Task<Message> {
        let Some(project) = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .map(|s| s.info.id.clone())
        else {
            return Task::none();
        };
        self.processing.dialog = None;
        self.processing.batch = None;
        self.dialog = None;
        self.open_catalog_at(Some((project, Tab::Jobs)))
    }
}

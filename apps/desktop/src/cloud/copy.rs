//! The open cloud project's local copy and the connection (docs/adr/0043,
//! 0041): the desktop works without a connection. Every project opened is
//! kept on this device (kentos-cloud's `Replica`, locked while it is open);
//! it opens from there when the server cannot be reached, and its work
//! waits in the device draft (a file project's Kaydet in the copy) until the
//! connection returns.
//!
//! - After every step that changes what this device knows the server has —
//!   a command answered, others' changes taken in, a conflict's choice — the
//!   step is appended to the copy first and the draft written after it, so
//!   copy and draft agree after a crash. The copy is compacted when its log
//!   grows long and when the project closes.
//! - The connection: a request that gets no answer puts the cloud offline;
//!   while signed in, the account is asked for every 15 s, and the first
//!   answer brings it back: waiting work goes, others' changes are caught up.
//!   The status bar shows çevrimiçi, çevrimdışı or eşitleniyor.

use std::time::{Duration, Instant};

use iced::Task;
use iced::task::Handle;
use kentos_cloud::{ApiFailure, Cloud as Client, Replica, ReplicaError};
use kentos_contracts::{Me, ProjectStorage};
use kentos_domain::Uuid;
use serde_json::Value;

use crate::app::{App, Message};
use crate::cloud::Event;

/// Steps the copy's log holds before it is compacted.
pub const COMPACT_AFTER: usize = 200;
/// How often the server is asked for while the cloud is offline.
pub const PROBE: Duration = Duration::from_secs(15);

/// Whether the cloud can be reached (the status bar's dot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Link {
    #[default]
    Online,
    Offline,
}

/// The open cloud project's local copy, held by this program.
pub struct Held {
    /// The drawing it belongs to.
    pub session: u64,
    pub tenant: Uuid,
    pub project: Uuid,
    pub replica: Replica,
    /// Steps in its log since it was last written whole.
    pub steps: usize,
    /// A file project's Kaydet made without a connection waits in it.
    pub kept_save: bool,
    /// A kept save on its way to the server.
    pub(super) sending: Option<Handle>,
    /// What went wrong writing it, said once.
    told: Option<String>,
}

impl Held {
    pub fn new(session: u64, tenant: Uuid, project: Uuid, replica: Replica) -> Self {
        let steps = replica.steps().unwrap_or(0);
        let kept_save = replica.kept_save().ok().flatten().is_some();
        Self {
            session,
            tenant,
            project,
            replica,
            steps,
            kept_save,
            sending: None,
            told: None,
        }
    }
}

impl App {
    /// The account the cloud works for: the signed-in one, or the last one
    /// kept for work without a connection (`cloud.account`; its id is not a
    /// secret, the session is never kept). The server's address as the
    /// connection writes it, and the account's id.
    pub(crate) fn cloud_identity(&self) -> Option<(String, String)> {
        if let (Some(client), Some(me)) = (&self.cloud.client, &self.cloud.me) {
            return Some((client.server().to_owned(), me.user.id.clone()));
        }
        let user = self.settings.text("cloud.account");
        if user.is_empty() {
            return None;
        }
        let server = Client::new(&self.settings.text("cloud.server")).ok()?;
        Some((server.server().to_owned(), user))
    }

    /// Keeps the signed-in account for work without a connection.
    pub(crate) fn keep_account(&mut self, me: &Me) {
        if self.settings.text("cloud.account") != me.user.id {
            let _ = self
                .settings
                .choose(&[("cloud.account", Value::from(me.user.id.as_str()))]);
        }
    }

    /// The copy of a project for an open: the open project's own, or locked now
    /// (another KentOS window may have it). None without a place for copies.
    pub(crate) fn lock_copy(
        &mut self,
        server: &str,
        user: &str,
        tenant: Uuid,
        project: Uuid,
    ) -> Result<Option<Replica>, ReplicaError> {
        if self
            .cloud
            .held
            .as_ref()
            .is_some_and(|h| h.tenant == tenant && h.project == project)
        {
            return Ok(self.cloud.held.take().map(|h| h.replica));
        }
        match &self.cloud.replicas {
            Some(store) => store.open(server, user, tenant, project).map(Some),
            None => Ok(None),
        }
    }

    /// After a step that changed what this device knows the server has: the
    /// step goes to the copy first, then the draft is written (docs/adr/0043).
    pub(crate) fn after_server_step(&mut self) -> Task<Message> {
        let mut said = None;
        if let (Some(live), Some(held), Some(doc)) = (
            self.cloud.live.as_mut(),
            self.cloud.held.as_mut(),
            self.document.as_ref(),
        ) && held.session == live.session
            && let Some(step) = live.sync.take_base_step()
        {
            match held.replica.append(&step) {
                Ok(()) => held.steps += 1,
                Err(e) => said = Some(e.to_string()),
            }
            if held.steps > COMPACT_AFTER {
                match held.replica.compact(&live.sync.base(&doc.model)) {
                    Ok(()) => held.steps = 0,
                    Err(e) => said = Some(e.to_string()),
                }
            }
            if let Some(text) = &said
                && held.told.as_ref() != Some(text)
            {
                held.told = Some(text.clone());
            } else {
                said = None;
            }
        }
        if let Some(text) = said {
            self.warn(format!(
                "{text} Değişiklikleriniz cihaz taslağında duruyor; proje sunucudan yeniden açılınca kopya yeniden kurulur."
            ));
        }
        self.write_draft(false)
    }

    /// The open cloud project is left: its copy is written whole and let go.
    pub(crate) fn close_cloud_project(&mut self) {
        if let (Some(live), Some(held), Some(doc)) = (
            self.cloud.live.as_mut(),
            self.cloud.held.as_mut(),
            self.document.as_ref(),
        ) && held.session == live.session
        {
            if let Some(step) = live.sync.take_base_step()
                && held.replica.append(&step).is_ok()
            {
                held.steps += 1;
            }
            if held.steps > 0 {
                let _ = held.replica.compact(&live.sync.base(&doc.model));
            }
        }
        self.cloud.live = None;
        self.cloud.held = None;
    }

    /// A request got no answer: the cloud is offline until one does.
    pub(crate) fn went_offline(&mut self) {
        if self.cloud.link != Link::Offline {
            self.cloud.link = Link::Offline;
            self.cloud.probe_at = Some(Instant::now() + PROBE);
        }
    }

    /// A request was answered: back online, what waited goes and others'
    /// changes are caught up.
    pub(crate) fn came_online(&mut self) -> Task<Message> {
        if self.cloud.link == Link::Online {
            return Task::none();
        }
        self.cloud.link = Link::Online;
        self.output("Sunucuya yeniden ulaşıldı; bu cihazda bekleyen değişiklikler gönderiliyor.");
        self.resume()
    }

    /// The server can be reached with a session: what waited goes, following starts again.
    pub(crate) fn resume(&mut self) -> Task<Message> {
        self.cloud.probe_at = None;
        self.cloud.probing = None;
        if let Some(live) = self.cloud.live.as_mut() {
            live.kick();
        }
        self.send_kept_save()
    }

    /// While offline and signed in: the account is asked for now and then.
    pub(crate) fn probe_tick(&mut self, now: Instant) -> Task<Message> {
        let due = self.cloud.link == Link::Offline
            && self.cloud.probing.is_none()
            && self.cloud.probe_at.is_some_and(|at| now >= at);
        let Some(client) = self.cloud.signed_in().cloned().filter(|_| due) else {
            return Task::none();
        };
        let (task, handle) = Task::perform(client.me(), |result| {
            crate::cloud::msg(Event::Probed(result))
        })
        .abortable();
        self.cloud.probing = Some(handle.abort_on_drop());
        task
    }

    pub(crate) fn probed(&mut self, result: Result<Me, ApiFailure>) -> Task<Message> {
        self.cloud.probing = None;
        match result {
            Ok(me) => {
                self.cloud.me = Some(me);
                self.came_online()
            }
            Err(failure) if failure.signed_out() => {
                self.cloud.me = None;
                self.warn("Bulut oturumunuz sona erdi; değişiklikleriniz bu cihazda bekliyor. Yeniden giriş yapınca gönderilir.");
                Task::none()
            }
            Err(_) => {
                self.cloud.probe_at = Some(Instant::now() + PROBE);
                Task::none()
            }
        }
    }

    /// A request's failure: no answer puts the cloud offline.
    pub(crate) fn heard(&mut self, failure: Option<&ApiFailure>) -> Task<Message> {
        match failure {
            Some(f) if f.status == 0 && f.transient() => {
                self.went_offline();
                Task::none()
            }
            _ => self.came_online(),
        }
    }

    /// Whether the open cloud project's work is on its way (the dot says eşitleniyor).
    pub(crate) fn syncing(&self) -> bool {
        self.cloud
            .live
            .as_ref()
            .is_some_and(|l| l.sending() || !l.sync.all_sent())
            || self.saving.is_some()
            || self
                .cloud
                .held
                .as_ref()
                .is_some_and(|h| h.sending.is_some())
    }

    /// A file project's save that waits in the copy goes now (the connection returned).
    pub(crate) fn send_kept_save(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let (Some(client), Some(held), Some(doc)) =
            (client, self.cloud.held.as_mut(), self.document.as_ref())
        else {
            return Task::none();
        };
        if !held.kept_save || held.sending.is_some() || doc.session != held.session {
            return Task::none();
        }
        let Some(source) = doc
            .cloud_source()
            .filter(|s| s.storage() == ProjectStorage::File)
        else {
            return Task::none();
        };
        let Ok(Some((bytes, based_on))) = held.replica.kept_save() else {
            return Task::none();
        };
        let session = held.session;
        let (tenant, project) = (source.tenant, source.project);
        let (task, handle) = Task::perform(
            kentos_cloud::save_revision(&client, tenant, project, bytes, based_on),
            move |result| {
                crate::cloud::msg(Event::KeptSent {
                    session,
                    based_on,
                    result,
                })
            },
        )
        .abortable();
        held.sending = Some(handle.abort_on_drop());
        task
    }
}

//! Following an open file project (docs/specs/file-revisions.md; the web's
//! app/cloud/fileProject.ts `receive`, `resync`, ui/cloud/FileConflict.ts):
//! while it is open and has not ended, its events are waited for on the
//! server (the long poll of database projects, docs/adr/0044). Another's
//! `project.file` asks the server which revision is its newest, who saved it
//! and when: it is said once and offered, never loaded by itself. A
//! deletion, an archive or a taken access ends saving there; a changed grant
//! asks the access again. When the missed events cannot be replayed the
//! project is asked what became of it and followed from its cursor now; the
//! drawing is never replaced. Without an answer the next try waits 1 s,
//! doubling up to 30 s.
//!
//! The questions a click on the save cell, Son revizyonu aç…, Kayıt
//! çakışmalarını çöz… or a refused Kaydet bring, and what each answer does,
//! are here too. What is known and what it means are revisions.rs's.

use std::collections::HashSet;
use std::time::Instant;

use iced::Task;
use iced::task::Handle;
use kentos_cloud::ApiFailure;
use kentos_cloud::follow;
use kentos_contracts::{EventPage, FileRevisions, ProjectInfo, ProjectState, ProjectStorage};
use kentos_domain::Uuid;

use crate::app::{App, Dialog, Message};
use crate::cloud::cells_plan::FileState;
use crate::cloud::file::FileConflict;
use crate::cloud::local_time::Zone;
use crate::cloud::revisions::{
    self, AnswerDoes, AnswerWork, Ended, Offer, ProjectAnswer, ResyncStep, RevisionConflict,
    RevisionInput, RevisionState, SaveStage, Tone, Via,
};
use crate::cloud::{Event, access_of};
use crate::saving::{Stage, Target};

/// An open file project followed.
pub struct FileFollow {
    /// The drawing it belongs to.
    pub session: u64,
    pub tenant: Uuid,
    pub project: Uuid,
    /// The newest event cursor heard.
    pub cursor: String,
    /// A newer revision than the drawing's, as far as known.
    pub newer: Option<revisions::NewerRevision>,
    /// Nothing more is saved there: heard from the events or a resync.
    pub ended: Option<Ended>,
    /// Request ids of this window's commits: their events are its own.
    own: HashSet<String>,
    /// The wait on its way.
    polling: Option<Handle>,
    /// When the next wait may start, and the failed tries in a row.
    poll_at: Instant,
    poll_tries: u32,
    /// The newest revision asked for (`GET …/files`).
    asking: Option<Handle>,
    /// The project asked for: after a grant changed, or in a resync.
    access: Option<Handle>,
    resyncing: Option<Handle>,
    /// A resync got no answer: said once, until one comes.
    unanswered: bool,
}

impl FileFollow {
    pub fn new(session: u64, tenant: Uuid, project: Uuid, cursor: String) -> Self {
        Self {
            session,
            tenant,
            project,
            cursor,
            newer: None,
            ended: None,
            own: HashSet::new(),
            polling: None,
            poll_at: Instant::now(),
            poll_tries: 0,
            asking: None,
            access: None,
            resyncing: None,
            unanswered: false,
        }
    }

    /// A commit of this window: its event is its own, never another's revision.
    pub fn expect(&mut self, request: String) {
        self.own.insert(request);
    }

    /// The request ids of this window's commits (tests).
    #[cfg(test)]
    pub fn own_requests(&self) -> impl Iterator<Item = &str> {
        self.own.iter().map(String::as_str)
    }

    /// Whether the newest revision is being asked for (tests).
    #[cfg(test)]
    pub fn asking(&self) -> bool {
        self.asking.is_some()
    }

    /// Whether a resync is on its way (tests).
    #[cfg(test)]
    pub fn resyncing(&self) -> bool {
        self.resyncing.is_some()
    }

    /// Whether the next wait may start now.
    fn due(&self, now: Instant) -> bool {
        self.ended.is_none()
            && self.polling.is_none()
            && self.resyncing.is_none()
            && now >= self.poll_at
    }
}

/// The project's answer as a resync reads it.
fn answer_of(result: &Result<ProjectInfo, ApiFailure>) -> ProjectAnswer {
    match result {
        Ok(info) => ProjectAnswer::Project {
            state: info.state,
            event_cursor: info.event_cursor.clone(),
        },
        Err(f) if f.deleted() => ProjectAnswer::Deleted,
        Err(f) if f.not_found() => ProjectAnswer::NotFound,
        Err(f) if f.code == "forbidden" => ProjectAnswer::Forbidden(f.message.clone()),
        Err(f) if f.transient() => ProjectAnswer::Unreachable,
        Err(f) => ProjectAnswer::Failed(f.message.clone()),
    }
}

impl App {
    /// What the open file project knows now, gathered from the drawing, its
    /// source, a Kaydet on its way, its last failure, a conflict and what
    /// its events said. A save kept on this device counts as unsaved work
    /// (docs/adr/0043): it is on no revision yet. None without one.
    pub(crate) fn file_revisions(&self) -> Option<RevisionState> {
        let doc = self.document.as_ref()?;
        let source = doc
            .cloud_source()
            .filter(|s| s.storage() == ProjectStorage::File)?;
        let session = doc.session;
        let follow = self.cloud.file.as_ref().filter(|f| f.session == session);
        let saving = self
            .saving
            .as_ref()
            .filter(|s| s.session == session && matches!(s.target, Target::Cloud { .. }));
        let stage = match saving.map(|s| s.step.as_ref()) {
            None => SaveStage::Idle,
            Some(Some(Stage::Uploading { done, total })) if done >= total => SaveStage::Verifying,
            Some(Some(Stage::Uploading { .. })) => SaveStage::Uploading,
            Some(_) => SaveStage::Encoding,
        };
        let failed = self
            .cloud
            .file_failed
            .as_ref()
            .filter(|(s, ..)| *s == session)
            .map(|(_, state, _)| *state);
        let ended = follow
            .and_then(|f| f.ended)
            .or(match failed {
                Some(FileState::Deleted) => Some(Ended::Deleted),
                Some(FileState::Revoked) => Some(Ended::Revoked),
                _ => None,
            })
            .or(source.archived().then_some(Ended::Archived));
        let kept = self
            .cloud
            .held
            .as_ref()
            .is_some_and(|h| h.kept_save && h.session == session);
        Some(RevisionState {
            base: source
                .revision
                .as_ref()
                .map_or_else(|| "0".to_owned(), |r| r.number.to_string()),
            newer: follow.and_then(|f| f.newer.clone()),
            conflict: self
                .cloud
                .file_conflict
                .as_ref()
                .filter(|c| c.session == session)
                .map(|c| RevisionConflict {
                    expected: c.based_on.to_string(),
                    actual: c.server.to_string(),
                }),
            dirty: doc.dirty() || kept,
            stage,
            failed: failed == Some(FileState::Error) && saving.is_none(),
            writable: source.can_write(),
            ended,
        })
    }

    /// One input to what the open file project knows: the newer revision it
    /// leaves is kept, a standing conflict follows it, and a newer revision
    /// heard for the first time is said in the log.
    pub(crate) fn file_step(&mut self, input: &RevisionInput) {
        let Some(before) = self.file_revisions() else {
            return;
        };
        let (after, say) = revisions::step(&before, input);
        let session = self.document.as_ref().map_or(0, |d| d.session);
        if let Some(f) = self.cloud.file.as_mut().filter(|f| f.session == session) {
            f.newer.clone_from(&after.newer);
        }
        if let (Some(c), Some(after)) = (
            self.cloud
                .file_conflict
                .as_mut()
                .filter(|c| c.session == session),
            &after.conflict,
        ) {
            c.server = after.actual.parse().unwrap_or(c.server);
        }
        if say && let Some(newer) = &after.newer {
            let name = self
                .document
                .as_ref()
                .map_or(String::new(), |d| d.name().to_owned());
            self.warn(revisions::newer_line(
                &name,
                newer,
                &before.base,
                before.dirty,
                Zone::system(),
            ));
        }
    }

    /// Starts following the file project just opened (an archived one is
    /// not followed: nothing is saved there).
    pub(crate) fn follow_file(&mut self, session: u64, info: &ProjectInfo) {
        let (Some(tenant), Some(project)) = (
            crate::cloud::uuid(&info.tenant_id),
            crate::cloud::uuid(&info.id),
        ) else {
            return;
        };
        self.cloud.file = (info.state != ProjectState::Archived)
            .then(|| FileFollow::new(session, tenant, project, info.event_cursor.clone()));
    }

    /// The timers: the next wait when it is due.
    pub(crate) fn file_tick(&mut self, now: Instant) -> Task<Message> {
        let due = self.cloud.file.as_ref().is_some_and(|f| f.due(now));
        if due && self.cloud.signed_in().is_some() {
            self.file_poll()
        } else {
            Task::none()
        }
    }

    /// Waits on the server for the events after the project's cursor.
    fn file_poll(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let (Some(client), Some(f)) = (client, self.cloud.file.as_mut()) else {
            return Task::none();
        };
        if f.ended.is_some() || f.polling.is_some() || f.resyncing.is_some() {
            return Task::none();
        }
        let session = f.session;
        let (task, handle) = Task::perform(
            follow::wait(&client, f.tenant, f.project, &f.cursor),
            move |result| crate::cloud::msg(Event::FileEvents { session, result }),
        )
        .abortable();
        f.polling = Some(handle.abort_on_drop());
        task
    }

    pub(crate) fn file_follow_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::FileEvents { session, result } => self.file_events(session, result),
            Event::FileNewest { session, result } => self.file_newest(session, result),
            Event::FileAccess { session, result } => self.file_access(session, result),
            Event::FileResync { session, result } => self.file_resynced(session, result),
            Event::RevisionAnswer(answer) => self.file_answer(answer),
            _ => Task::none(),
        }
    }

    /// A batch of the project's events: its end, a grant changed, another's
    /// revision; then the next wait at once.
    fn file_events(
        &mut self,
        session: u64,
        result: Result<EventPage, ApiFailure>,
    ) -> Task<Message> {
        let Some(f) = self.cloud.file.as_mut().filter(|f| f.session == session) else {
            return Task::none();
        };
        f.polling = None;
        let page = match result {
            Ok(page) => page,
            Err(failure) => return self.file_poll_failed(&failure),
        };
        f.poll_tries = 0;
        let read = revisions::read_events(&page.events, |id| f.own.contains(id));
        if let Some(cursor) = read.cursor {
            f.cursor = cursor;
        }
        let mut tasks = vec![self.heard(None)];
        if let Some((why, quiet)) = read.end {
            self.file_end(why, "", quiet);
            return Task::batch(tasks);
        }
        if read.access {
            tasks.push(self.file_ask_access());
        }
        if read.newest {
            tasks.push(self.file_ask_newest());
        }
        tasks.push(self.file_poll());
        Task::batch(tasks)
    }

    /// Following failed: the reason decides what follows.
    fn file_poll_failed(&mut self, failure: &ApiFailure) -> Task<Message> {
        if failure.resync() {
            return self.file_resync();
        }
        if failure.deleted() {
            self.file_end(Ended::Deleted, "", false);
            return Task::none();
        }
        if failure.archived() {
            self.file_end(Ended::Archived, "", false);
            return Task::none();
        }
        if failure.not_found() {
            self.file_end(Ended::Revoked, "", false);
            return Task::none();
        }
        if failure.signed_out() {
            self.cloud.me = None;
            self.warn(format!(
                "Bulut oturumunuz sona erdi ({}); başkasının kaydettiği revizyonlar duyulmuyor. Yeniden giriş yapın.",
                failure.message
            ));
            return Task::none();
        }
        let Some(f) = self.cloud.file.as_mut() else {
            return Task::none();
        };
        // No answer: 1 s, doubling up to 30 s, then again from the cursor.
        f.poll_tries += 1;
        f.poll_at = Instant::now() + failure.backoff(f.poll_tries);
        // The organisation may not be used now: what is left is asked.
        let access = if failure.code == "forbidden" {
            self.file_ask_access()
        } else {
            Task::none()
        };
        Task::batch([access, self.heard(Some(failure))])
    }

    /// Asks the server which revision is its newest, who saved it and when.
    /// A question already on its way gives way to this one.
    pub(crate) fn file_ask_newest(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let (Some(client), Some(f)) = (client, self.cloud.file.as_mut()) else {
            return Task::none();
        };
        let session = f.session;
        let (task, handle) = Task::perform(
            follow::file_revisions_now(&client, f.tenant, f.project),
            move |result| crate::cloud::msg(Event::FileNewest { session, result }),
        )
        .abortable();
        f.asking = Some(handle.abort_on_drop());
        task
    }

    fn file_newest(
        &mut self,
        session: u64,
        result: Result<FileRevisions, ApiFailure>,
    ) -> Task<Message> {
        let Some(f) = self.cloud.file.as_mut().filter(|f| f.session == session) else {
            return Task::none();
        };
        f.asking = None;
        match result {
            Ok(list) => {
                self.file_step(&RevisionInput::Newest(revisions::newest_of(&list)));
                Task::none()
            }
            // Asked again with the next event; a newer revision is never guessed.
            Err(failure) => self.heard(Some(&failure)),
        }
    }

    /// Asks what this account may do in the project now.
    fn file_ask_access(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let (Some(client), Some(f)) = (client, self.cloud.file.as_mut()) else {
            return Task::none();
        };
        let session = f.session;
        let (task, handle) = Task::perform(client.project(f.tenant, f.project), move |result| {
            crate::cloud::msg(Event::FileAccess { session, result })
        })
        .abortable();
        f.access = Some(handle.abort_on_drop());
        task
    }

    fn file_access(
        &mut self,
        session: u64,
        result: Result<ProjectInfo, ApiFailure>,
    ) -> Task<Message> {
        let Some(f) = self.cloud.file.as_mut().filter(|f| f.session == session) else {
            return Task::none();
        };
        f.access = None;
        match result {
            Ok(info) => self.file_take_access(&info),
            Err(failure) if failure.deleted() => self.file_end(Ended::Deleted, "", false),
            Err(failure) if failure.not_found() => self.file_end(Ended::Revoked, "", false),
            Err(failure) if failure.code == "forbidden" => {
                self.file_end(Ended::Revoked, &failure.message, false);
            }
            Err(_) => {}
        }
        Task::none()
    }

    /// The account's access as the server gave it: the role said when it changed.
    fn file_take_access(&mut self, info: &ProjectInfo) {
        match info.state {
            ProjectState::Trashed => return self.file_end(Ended::Deleted, "", false),
            ProjectState::Archived => return self.file_end(Ended::Archived, "", false),
            ProjectState::Active => {}
        }
        let (write, _) = access_of(info);
        let Some(source) = self.document.as_mut().and_then(|d| d.cloud_source_mut()) else {
            return;
        };
        let changed = source.info.access.role != info.access.role;
        source.info.access = info.access.clone();
        if changed {
            let name = source.info.name.clone();
            let role = crate::cloud::words::role(info.access.role);
            let saves = if write {
                String::new()
            } else {
                " Kaydet artık buluta yazmaz; değişiklikleri saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin.".to_owned()
            };
            self.output(format!("“{name}” projesindeki rolünüz: {role}.{saves}"));
        }
    }

    /// The missed events cannot be replayed: the project is asked what became of it.
    fn file_resync(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let (Some(client), Some(f)) = (client, self.cloud.file.as_mut()) else {
            return Task::none();
        };
        let session = f.session;
        let (task, handle) = Task::perform(
            follow::project_now(&client, f.tenant, f.project),
            move |result| crate::cloud::msg(Event::FileResync { session, result }),
        )
        .abortable();
        f.resyncing = Some(handle.abort_on_drop());
        task
    }

    fn file_resynced(
        &mut self,
        session: u64,
        result: Result<ProjectInfo, ApiFailure>,
    ) -> Task<Message> {
        let Some(f) = self.cloud.file.as_mut().filter(|f| f.session == session) else {
            return Task::none();
        };
        f.resyncing = None;
        match revisions::resync_step(&answer_of(&result)) {
            ResyncStep::Follow { cursor } => {
                f.cursor = cursor;
                f.unanswered = false;
                f.poll_at = Instant::now();
                // The events missed may have changed the access, or brought a
                // revision: taken and asked as those events would.
                if let Ok(info) = &result {
                    self.file_take_access(info);
                }
                let newest = self.file_ask_newest();
                Task::batch([self.heard(None), newest, self.file_poll()])
            }
            ResyncStep::End { why, reason } => {
                self.file_end(why, &reason, false);
                Task::none()
            }
            ResyncStep::Retry => {
                f.poll_at = Instant::now() + revisions::RESYNC_RETRY;
                if !std::mem::replace(&mut f.unanswered, true) {
                    let name = self
                        .document
                        .as_ref()
                        .map_or(String::new(), |d| d.name().to_owned());
                    self.warn(revisions::texts::resync_failed(&name));
                }
                match &result {
                    Err(failure) => self.heard(Some(failure)),
                    Ok(_) => Task::none(),
                }
            }
        }
    }

    /// Nothing more is saved to the open file project: said once, the
    /// drawing stays on screen, and its copy says so too (docs/adr/0043).
    /// `quiet`: this window archived it (the catalog said so).
    pub(crate) fn file_end(&mut self, why: Ended, reason: &str, quiet: bool) {
        let Some(f) = self.cloud.file.as_mut() else {
            return;
        };
        if f.ended.is_some() {
            return;
        }
        f.ended = Some(why);
        f.polling = None;
        f.asking = None;
        f.access = None;
        f.resyncing = None;
        let session = f.session;
        if let Some(held) = self.cloud.held.as_ref().filter(|h| h.session == session) {
            let _ = held.replica.mark_ended(match why {
                Ended::Deleted => kentos_cloud::replica::Ended::Deleted,
                Ended::Revoked => kentos_cloud::replica::Ended::Revoked,
                Ended::Archived => kentos_cloud::replica::Ended::Archived,
            });
        }
        let Some(doc) = self.document.as_mut().filter(|d| d.session == session) else {
            return;
        };
        let name = doc.name().to_owned();
        if let Some(source) = doc.cloud_source_mut() {
            match why {
                Ended::Archived => source.info.state = ProjectState::Archived,
                Ended::Revoked => source.info.access.permissions.clear(),
                Ended::Deleted => {}
            }
        }
        // Archived from this window: already said, without the notice (catalog_actions.rs).
        let by_me =
            why == Ended::Archived && (quiet || std::mem::take(&mut self.cloud.archived_by_me));
        if by_me {
            return;
        }
        let text = match why {
            Ended::Deleted => format!(
                "“{name}” bulut projesi silindi; çizim artık buluta kaydedilemez. Çizim ekranda kalıyor; saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
            ),
            Ended::Archived => format!(
                "“{name}” bulut projesi arşivlendi; çizim artık buluta kaydedilemez. Çizim ekranda kalıyor; kaydedilmemiş değişiklikleri saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
            ),
            Ended::Revoked => {
                let why = if reason.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", reason.trim_end_matches('.'))
                };
                format!(
                    "“{name}” projesine erişiminiz kaldırıldı{why}. Değişiklikleriniz artık buluta kaydedilmiyor. Çizimi saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
                )
            }
        };
        self.warn(text);
        // The web's notice (AccessLostNotice.ts), unless another window is up.
        if self.dialog.is_none() {
            self.dialog = Some(Dialog::Ended);
        }
    }

    /// Asks what the plan offers (revisions.rs `offer`): the question, or a
    /// line instead of one.
    pub(crate) fn ask_file(&mut self, via: Via) {
        let Some(state) = self.file_revisions() else {
            return;
        };
        let name = self
            .document
            .as_ref()
            .map_or(String::new(), |d| d.name().to_owned());
        let busy = state.stage != SaveStage::Idle;
        match revisions::offer(&name, &state, busy, via, Zone::system()) {
            Offer::Ask(question) => {
                self.cloud.question = Some(question);
                self.dialog = Some(Dialog::Revision);
            }
            Offer::Say(Tone::Info, line) => self.output(line),
            Offer::Say(Tone::Warn, line) => self.warn(line),
            Offer::None => {}
        }
    }

    /// What an answer does (revisions.rs `AnswerDoes`, `AnswerWork`).
    fn file_answer(&mut self, answer: revisions::Answer) -> Task<Message> {
        let Some(question) = self.cloud.question.take() else {
            return Task::none();
        };
        if self.dialog == Some(Dialog::Revision) {
            self.dialog = None;
        }
        let Some(a) = question.answers.iter().find(|a| a.value == answer) else {
            return Task::none();
        };
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let (session, name) = (doc.session, doc.name().to_owned());
        match a.does {
            AnswerDoes::Nothing => Task::none(),
            AnswerDoes::Copy => self.save_copy(),
            AnswerDoes::Local => {
                // Farklı kaydet: once written, the drawing leaves the project (app.rs).
                self.cloud.detaching = Some((session, name));
                self.run("file.saveAs")
            }
            AnswerDoes::Latest => {
                if a.work == AnswerWork::Dropped {
                    // Dropped on purpose: their recovery copy and a save kept here go too.
                    self.recovery.discard();
                    if let Some(held) = self
                        .cloud
                        .held
                        .as_mut()
                        .filter(|h| h.session == session && h.kept_save)
                        && held.replica.clear_save().is_ok()
                    {
                        held.kept_save = false;
                    }
                }
                self.reopen_newest()
            }
        }
    }

    /// Kaydet met a newer revision, known or refused by the server: nothing
    /// was written; what is known follows the refusal and the question comes.
    pub(crate) fn file_refused(&mut self, session: u64, server: u64, based_on: u64) {
        self.cloud.file_conflict = Some(FileConflict {
            session,
            server,
            based_on,
        });
        self.file_step(&RevisionInput::Refused {
            actual: server.to_string(),
        });
    }
}

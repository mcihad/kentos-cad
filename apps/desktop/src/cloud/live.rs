//! A database project's autosave and device draft (docs/adr/0041, 0040),
//! with the web's timing (apps/web/src/app/cloud/sync.ts):
//!
//! - an edit is sent 1 s after the last one, and at most 5 s after the
//!   first one waiting; the next batch, a resent command and Ctrl+S go at once;
//! - one command at a time; a passing failure waits as long as kentos-cloud
//!   says (`After::Retry`), then the same command goes again with its key;
//! - the device draft is written 300 ms after an edit, right before every
//!   send (the command on its way is on the disk before it leaves), and
//!   after every answer or failure; one write at a time. When nothing is
//!   left to keep, the draft is removed. A failed write is said and work
//!   goes on.
//!
//! The timers are looked at on the app's cloud tick; everything here is a
//! state of the app, so the tests drive it with their own clock.

use std::time::{Duration, Instant};

use iced::Task;
use iced::task::Handle;
use kentos_cloud::{After, ApiFailure, DraftKey, ProjectSync, SaveState};
use kentos_contracts::{CommandEnvelope, CommitResult};
use kentos_domain::Uuid;

use crate::app::{App, Message};
use crate::cloud::Event;

/// An edit waits this long for the next one (the web's `debounceMs`).
pub const DEBOUNCE: Duration = Duration::from_millis(1000);
/// And at most this long after the first one waiting (the web's `maxDelayMs`).
pub const MAX_WAIT: Duration = Duration::from_millis(5000);
/// The device draft is written this long after an edit (the web's `DRAFT_MS`).
pub const DRAFT_DELAY: Duration = Duration::from_millis(300);

/// What was said once while this project is open.
#[derive(Debug, Default)]
pub struct Told {
    pub read_only: bool,
    pub meta_here: bool,
    pub draft_failure: Option<String>,
    pub ended: bool,
}

/// The machinery of an open database project.
pub struct Live {
    /// The drawing it follows (`Document::session`).
    pub session: u64,
    pub tenant: Uuid,
    pub project: Uuid,
    pub sync: ProjectSync,
    /// Where its device draft goes; none without a draft folder.
    pub key: Option<DraftKey>,
    /// The drawing's generation and revision last taken in.
    seen: (u64, u64),
    /// The first and the last edit waiting since the last send.
    first: Option<Instant>,
    last: Option<Instant>,
    /// Not before this, after a passing failure.
    retry_at: Option<Instant>,
    /// Send at once: Ctrl+S, a resent command, the next batch, a new session.
    now: bool,
    /// The command about to go or on its way (the server's request).
    pub sent: Option<CommandEnvelope>,
    sending: Option<Handle>,
    /// The draft: when it is due, a write on its way, another one wanted.
    draft_at: Option<Instant>,
    draft_busy: bool,
    draft_again: bool,
    /// The command waits for the draft that carries it.
    send_waits: bool,
    // Following (follow.rs).
    pub(super) poll_at: Instant,
    pub(super) polling: Option<Handle>,
    pub(super) poll_tries: u32,
    /// Others' changes fetched while an edit was open: taken once it ends.
    pub(super) taking: Option<(kentos_cloud::Incoming, kentos_cloud::Remote, bool)>,
    /// The server's copies chosen while an edit was open: taken once it ends.
    pub(super) theirs: Option<Option<kentos_contracts::ProjectInfo>>,
    pub told: Told,
}

impl Live {
    pub fn new(
        session: u64,
        tenant: Uuid,
        project: Uuid,
        sync: ProjectSync,
        key: Option<DraftKey>,
        seen: (u64, u64),
    ) -> Self {
        Self {
            session,
            tenant,
            project,
            sync,
            key,
            seen,
            first: None,
            last: None,
            retry_at: None,
            now: false,
            sent: None,
            sending: None,
            draft_at: None,
            draft_busy: false,
            draft_again: false,
            send_waits: false,
            // Others' changes are waited for at once (a long poll, follow.rs).
            poll_at: Instant::now(),
            polling: None,
            poll_tries: 0,
            taking: None,
            theirs: None,
            told: Told::default(),
        }
    }

    /// Sends what waits at the next look (Ctrl+S, a resent command).
    pub fn send_soon(&mut self) {
        self.now = true;
        self.retry_at = None;
    }

    /// After a new sign-in: what waited for a session goes, and following resumes.
    pub fn kick(&mut self) {
        self.send_soon();
        self.poll_at = Instant::now();
    }

    /// Whether a command is on its way (or waits for its draft).
    pub fn sending(&self) -> bool {
        self.sending.is_some() || self.send_waits
    }

    /// When the next command is due, if one may go.
    fn send_due(&self, now: Instant) -> bool {
        if self.sending() || !self.sync.wants_to_send() {
            return false;
        }
        if let Some(at) = self.retry_at {
            return now >= at;
        }
        if self.now {
            return true;
        }
        match (self.first, self.last) {
            (Some(first), Some(last)) => now >= (last + DEBOUNCE).min(first + MAX_WAIT),
            // Nothing timed: a restored command, the next batch after an answer.
            _ => true,
        }
    }
}

impl App {
    /// The drawing changed: the autosave takes the edits in, the timers start.
    pub(crate) fn live_observe(&mut self, now: Instant) {
        let (Some(live), Some(doc)) = (self.cloud.live.as_mut(), self.document.as_mut()) else {
            return;
        };
        let seen = (doc.model.generation(), doc.model.revision());
        if seen == live.seen {
            return;
        }
        let edited = seen.1 != live.seen.1;
        live.seen = seen;
        live.sync.observe(&doc.model);
        let mut say = Vec::new();
        if edited {
            if live.sync.pending() > 0 {
                live.last = Some(now);
                live.first.get_or_insert(now);
                if live.sync.state() == SaveState::ReadOnly && !live.told.read_only {
                    live.told.read_only = true;
                    say.push("Bu projeyi yalnız görüntüleyebilirsiniz; değişiklikleriniz buluta kaydedilmez ve bu cihazda saklanmaz.");
                }
                if live.sync.keeps_meta_here() && !live.told.meta_here {
                    live.told.meta_here = true;
                    say.push("Katman ve proje bilgisi değişiklikleriniz yalnız bu cihazda kalıyor: projede bunları değiştirme yetkiniz yok.");
                }
            }
            live.draft_at = Some(now + DRAFT_DELAY);
        }
        // Nothing differs from the server (every edit sent, or undone): the drawing is saved.
        if live.sync.all_sent() && doc.dirty() {
            let revision = doc.model.revision();
            doc.model.mark_saved(revision);
        }
        for text in say {
            self.warn(text);
        }
    }

    /// The open project's timers (the app's cloud tick).
    pub(crate) fn live_tick(&mut self, now: Instant) -> Task<Message> {
        let Some(live) = self.cloud.live.as_mut() else {
            return Task::none();
        };
        let draft = live.draft_at.is_some_and(|at| now >= at);
        if draft {
            live.draft_at = None;
        }
        let send = self.cloud.me.is_some() && live.send_due(now);
        let poll = self.cloud.me.is_some()
            && live.polling.is_none()
            && live.taking.is_none()
            && !live.sync.state().ended()
            && now >= live.poll_at;
        let mut tasks = vec![self.retry_waiting()];
        if draft {
            tasks.push(self.write_draft(false));
        }
        if send {
            tasks.push(self.send_next());
        }
        if poll {
            tasks.push(self.poll());
        }
        Task::batch(tasks)
    }

    /// Ctrl+S on a database project: what waits goes now.
    pub(crate) fn send_now(&mut self) -> Task<Message> {
        let Some(live) = self.cloud.live.as_mut() else {
            return Task::none();
        };
        if live.sync.all_sent() {
            self.output("Her şey buluta kaydedildi.");
            return Task::none();
        }
        if !live.sync.wants_to_send() {
            let text = match live.sync.state() {
                SaveState::Conflict => {
                    "Kayıt çakışması seçim bekliyor: durum çubuğundaki Çakışma'ya tıklayın."
                }
                SaveState::ReadOnly => {
                    "Bu projede değişiklik yetkiniz yok; değişiklikleriniz buluta gönderilmez. Saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
                }
                _ => {
                    "Bu projeye artık kaydedilemiyor; çizimi saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
                }
            };
            self.warn(text);
            return Task::none();
        }
        live.send_soon();
        self.output("Değişiklikler buluta gönderiliyor.");
        self.live_tick(Instant::now())
    }

    /// The next command: planned now, written into the draft, then sent.
    fn send_next(&mut self) -> Task<Message> {
        let (Some(live), Some(doc)) = (self.cloud.live.as_mut(), self.document.as_ref()) else {
            return Task::none();
        };
        let Some(envelope) = live.sync.next(&doc.model) else {
            // Nothing differs, or an edit is open (looked at again on the next tick).
            live.now = false;
            return Task::none();
        };
        live.sent = Some(envelope);
        live.first = None;
        live.last = None;
        live.now = false;
        live.retry_at = None;
        if live.key.is_some() && self.cloud.drafts.is_some() {
            // The command on its way is on the disk before it leaves.
            live.send_waits = true;
            return self.write_draft(true);
        }
        self.send_command()
    }

    /// Sends the command about to go.
    fn send_command(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(live) = self.cloud.live.as_mut() else {
            return Task::none();
        };
        live.send_waits = false;
        let (Some(client), Some(envelope)) = (client, live.sent.clone()) else {
            // Signed out meanwhile: it waits, with its key, for the next session.
            return Task::none();
        };
        let session = live.session;
        let (task, handle) =
            Task::perform(client.command::<CommitResult>(envelope), move |result| {
                crate::cloud::msg(Event::Committed { session, result })
            })
            .abortable();
        live.sending = Some(handle.abort_on_drop());
        task
    }

    /// Writes the device draft (`send`: the command waiting for it goes after).
    /// One write at a time: one asked for meanwhile follows the first.
    pub(crate) fn write_draft(&mut self, send: bool) -> Task<Message> {
        // The signed-in account, or the one kept for work without a connection.
        let user = self.cloud_identity().map(|(_, user)| user);
        let store = self.cloud.drafts.clone();
        let (Some(live), Some(doc)) = (self.cloud.live.as_mut(), self.document.as_ref()) else {
            return Task::none();
        };
        let (Some(store), Some(user), Some(key)) = (store, user, live.key.clone()) else {
            return if send {
                self.send_command()
            } else {
                Task::none()
            };
        };
        if live.draft_busy {
            live.draft_again = true;
            return Task::none();
        }
        match live.sync.draft(&doc.model, &user) {
            Some(draft) => {
                live.draft_busy = true;
                let session = live.session;
                Task::perform(store.save_later(key, draft), move |result| {
                    crate::cloud::msg(Event::DraftWritten {
                        session,
                        send,
                        result,
                    })
                })
            }
            None => {
                if let Err(e) = store.remove(&key) {
                    let text = format!(
                        "Cihazdaki eski taslak silinemedi ({e}); proje yeniden açılınca geri konabilir."
                    );
                    self.warn(text);
                }
                if send {
                    self.send_command()
                } else {
                    Task::none()
                }
            }
        }
    }

    pub(crate) fn live_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::DraftWritten {
                session,
                send,
                result,
            } => {
                let Some(live) = self.cloud.live.as_mut().filter(|l| l.session == session) else {
                    return Task::none();
                };
                live.draft_busy = false;
                let failed = result.err().map(|f| f.message.clone());
                let say = failed.filter(|m| live.told.draft_failure.as_ref() != Some(m));
                live.told.draft_failure.clone_from(&say);
                let again = std::mem::take(&mut live.draft_again);
                let waits = live.send_waits;
                if let Some(text) = say {
                    self.warn(text);
                }
                if again {
                    // A newer state was asked for meanwhile; the waiting command rides with it.
                    return self.write_draft(waits);
                }
                if send || waits {
                    return self.send_command();
                }
            }
            Event::Committed { session, result } => return self.committed(session, result),
            _ => {}
        }
        Task::none()
    }

    fn committed(
        &mut self,
        session: u64,
        result: Result<CommitResult, ApiFailure>,
    ) -> Task<Message> {
        let (Some(live), Some(doc)) = (
            self.cloud.live.as_mut().filter(|l| l.session == session),
            self.document.as_mut(),
        ) else {
            return Task::none();
        };
        live.sending = None;
        let mut say: Vec<(bool, String)> = Vec::new();
        let mut ended = false;
        let answered = result.is_ok();
        let heard = result.as_ref().err().cloned();
        match result {
            Ok(answer) => {
                live.sync.answered(&doc.model, &answer);
                live.sent = None;
                live.retry_at = None;
                // The next batch, or edits made while it was on its way, go at once.
                live.now = live.sync.pending() > 0;
                if live.sync.all_sent() {
                    let revision = doc.model.revision();
                    doc.model.mark_saved(revision);
                }
            }
            Err(failure) => match live.sync.failed(&failure) {
                After::Retry(wait) => live.retry_at = Some(Instant::now() + wait),
                After::Stop => {
                    if live.sync.state() != SaveState::Revoked {
                        live.sent = None;
                    }
                    ended = live.sync.state().ended();
                    if !ended {
                        say.push(stopped(&live.sync, &failure));
                    }
                    if failure.signed_out() {
                        self.cloud.me = None;
                    }
                }
            },
        }
        for (warn, text) in say {
            if warn {
                self.warn(text);
            } else {
                self.output(text);
            }
        }
        let link = self.heard(heard.as_ref());
        if ended {
            // Said once; the draft keeps what is left (follow.rs).
            return Task::batch([link, self.ended()]);
        }
        // After an answer the server's side goes to the copy first; then, after
        // every answer or failure, the draft says what is left (docs/adr/0043).
        let next = if answered {
            self.after_server_step()
        } else {
            self.write_draft(false)
        };
        Task::batch([link, next])
    }
}

/// What the command line says when sending stops (the web's words); an
/// ended project is said by `ended` (follow.rs).
fn stopped(sync: &ProjectSync, failure: &ApiFailure) -> (bool, String) {
    let text = match sync.state() {
        SaveState::Conflict => format!(
            "Kayıt çakışması: {} nesneyi başka biri daha önce kaydetti. Hiçbir şeyin üzerine yazılmadı; seçene kadar değişiklikleriniz yalnız bu cihazda. Durum çubuğundaki Çakışma'ya tıklayın.",
            sync.conflicts().len()
        ),
        _ if failure.signed_out() => format!(
            "Bulut oturumunuz sona erdi ({}); değişiklikleriniz bu cihazda bekliyor. Yeniden giriş yapınca gönderilir.",
            failure.message
        ),
        _ => format!("Bulut kaydı yapılamadı: {}", failure.message),
    };
    (true, text)
}

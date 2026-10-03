//! The sheet template library in the desktop (docs/sheet/design.md §13; the
//! web's `app/sheet/cloudLibrary.ts` and `templateSync.ts`): the templates
//! kept in the sheet store are synced with the signed-in account's cloud
//! library by `kentos-cloud`'s `sheet_library` (the core plans, the client
//! carries out), here run as the app's tasks:
//!
//! - when the account signs in, its newest event cursor is read first (so
//!   nothing between it and the sync is missed), then a sync, then the
//!   account's template events are waited for on the server (a long poll of
//!   25 s); an event runs the sync 300 ms later (they come in bursts);
//! - a sync asked while one runs makes another follow it; one that cannot
//!   reach the server leaves the copies as they are (they are used offline)
//!   and runs again after 5, 15, 30, then every 60 s, and when the
//!   connection comes back;
//! - “Buluta eşitle”, Eşitle, a change saved here and a deletion run it; the
//!   share window's questions (who has it, whom it may go to, share, unshare)
//!   are asked here and answered to the sheet mode.
//!
//! The lists show the signed-in account's copies; without a session (the
//! program started offline) the last account's (`cloud.account`), so they
//! can be used; after a sign-out in this session, none (the web's).

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::Task;
use iced::futures::Stream;
use iced::futures::channel::mpsc;
use iced::task::Handle;
use kentos_cloud::ApiFailure;
use kentos_cloud::sheet_library::{
    self as lib, Account, DeviceLibrary, DeviceRecord, SyncEnd, SyncReport, Tone,
};
use kentos_cloud::sheet_templates as api;
use kentos_domain::Uuid;
use kentos_interaction::Level;
use kentos_sheet::cloud::{
    DeviceOrganization, SheetTemplateAccess, SheetTemplateCandidates, SheetTemplateChanged,
    SheetTemplateEventPage, TemplateGrantRole,
};
use kentos_sheet_ui::library::texts;
use kentos_sheet_ui::share_template::texts as share_texts;
use kentos_sheet_ui::{Library, LibraryAccount, LibraryEvent, LibraryRequest, Store, SyncStatus};

use crate::app::{App, Message};
use crate::cloud::copy::Link;

/// The waits before a sync that could not reach the server runs again (the web's `RETRY_MS`).
const RETRY: [Duration; 4] = [
    Duration::from_secs(5),
    Duration::from_secs(15),
    Duration::from_secs(30),
    Duration::from_secs(60),
];
/// Events come in bursts (one change tells the owner and everyone it is shared with).
const SETTLE: Duration = Duration::from_millis(300);
/// How often the library's timers are looked at.
const TICK: Duration = Duration::from_millis(200);

/// The device's templates as the sync sees them: the sheet store's template files.
pub struct StoreLibrary(pub Store);

impl DeviceLibrary for StoreLibrary {
    fn records(&self) -> Vec<DeviceRecord> {
        self.0
            .templates()
            .into_iter()
            .map(|r| DeviceRecord {
                id: r.id,
                template: r.template,
                cloud: r.cloud,
            })
            .collect()
    }

    fn save(&self, r: &DeviceRecord) -> Result<(), String> {
        self.0
            .save_template(&r.id, &r.template, r.cloud.as_ref())
            .map_err(|e| e.to_string())
    }

    fn remove(&self, id: &str) -> Result<(), String> {
        self.0.remove_template(id).map_err(|e| e.to_string())
    }
}

/// What the library's tasks answer.
#[derive(Debug, Clone)]
pub enum LibraryMsg {
    Cursor {
        generation: u64,
        result: Result<String, ApiFailure>,
    },
    Busy {
        generation: u64,
        id: String,
        on: bool,
    },
    Synced {
        generation: u64,
        report: Box<SyncReport>,
    },
    Heard {
        generation: u64,
        result: Result<SheetTemplateEventPage, ApiFailure>,
    },
    Access {
        id: String,
        result: Result<SheetTemplateAccess, ApiFailure>,
    },
    Candidates {
        id: String,
        query: String,
        result: Result<SheetTemplateCandidates, ApiFailure>,
    },
    Shared {
        id: String,
        person: String,
        /// None: an unshare.
        role: Option<TemplateGrantRole>,
        /// A role changed in its row (not someone added).
        change: bool,
        result: Result<SheetTemplateChanged, ApiFailure>,
    },
    /// “Kuruma yayımla” answered.
    Published {
        id: String,
        name: String,
        organization_name: String,
        result: Result<SheetTemplateChanged, ApiFailure>,
    },
    Tick,
}

fn msg(m: LibraryMsg) -> Message {
    Message::SheetLibrary(m)
}

/// The library's state in the app.
#[derive(Default)]
pub struct SheetLibrary {
    /// The account it works for (signed in), and its personal space.
    account: Option<(Account, Option<Uuid>)>,
    /// A new one for every account: answers of an earlier one are dropped.
    generation: u64,
    running: Option<Handle>,
    again: bool,
    pub status: SyncStatus,
    pub busy: BTreeSet<String>,
    cursor: Option<String>,
    reading: Option<Handle>,
    listening: Option<Handle>,
    listen_at: Option<Instant>,
    retry_at: Option<Instant>,
    failures: usize,
    settle_at: Option<Instant>,
    /// “Buluta eşitle” waiting for the run that takes it up: its id and name.
    uploads: Vec<(String, String)>,
    /// The account signed out in this session: the lists hide its copies.
    signed_out: bool,
    /// The connection as last seen: its return runs the library again.
    online: bool,
    /// What the sheet mode was last told.
    told: Option<Library>,
    /// The organisations whose libraries the account sees, from the last list read.
    organizations: Vec<DeviceOrganization>,
}

impl SheetLibrary {
    /// Whether a timer waits (the subscription ticks only then).
    pub fn wants_ticks(&self) -> bool {
        self.listen_at.is_some() || self.retry_at.is_some() || self.settle_at.is_some()
    }

    fn stop(&mut self) {
        self.generation += 1;
        self.running = None;
        self.reading = None;
        self.listening = None;
        self.listen_at = None;
        self.retry_at = None;
        self.settle_at = None;
        self.cursor = None;
        self.again = false;
        self.failures = 0;
        self.busy.clear();
        self.uploads.clear();
        self.organizations.clear();
        self.account = None;
        self.status = SyncStatus::SignedOut;
    }

    /// The next wait before a retry, longer each time.
    fn retry_later(&mut self, now: Instant) {
        if self.retry_at.is_none() {
            let wait = RETRY[self.failures.min(RETRY.len() - 1)];
            self.failures += 1;
            self.retry_at = Some(now + wait);
        }
    }
}

/// Every [`TICK`], while the subscription lives (a timer waits).
pub fn ticks() -> impl Stream<Item = Message> {
    let (mut out, ticks) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(TICK);
            let sent = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                &mut out,
                msg(LibraryMsg::Tick),
            ));
            if sent.is_err() {
                break;
            }
        }
    });
    ticks
}

/// A refusal as the share window says it (the web's `failureText`): what failed, then why.
fn failure_text(e: &ApiFailure, failed: &str) -> String {
    format!("{failed}: {}", failure_why(e))
}

/// Why a request failed, in the user's words.
fn failure_why(e: &ApiFailure) -> String {
    if e.code == "network" {
        "sunucuya ulaşılamadı. Bağlantınızı denetleyip yeniden deneyin.".to_owned()
    } else if e.status == 401 {
        "oturumunuz sona erdi. Yeniden giriş yapıp tekrar deneyin.".to_owned()
    } else if e.transient() {
        "sunucu şu an yanıt vermiyor. Birazdan yeniden deneyin.".to_owned()
    } else {
        e.message.clone()
    }
}

fn level(t: Tone) -> Level {
    match t {
        Tone::Info => Level::Info,
        Tone::Warn => Level::Warn,
        Tone::Error => Level::Error,
        Tone::Success => Level::Success,
    }
}

impl App {
    /// What the sheet mode is told of the library now.
    fn library_view(&self) -> Library {
        let l = &self.sheet_library;
        let me = self.cloud.signed_in().and(self.cloud.me.as_ref());
        let viewer = match me {
            Some(me) => Some(me.user.id.clone()),
            None if l.signed_out => None,
            None => Some(self.settings.text("cloud.account")).filter(|a| !a.is_empty()),
        };
        Library {
            viewer,
            account: me.map(|me| LibraryAccount {
                id: me.user.id.clone(),
                name: me.user.display_name.clone(),
                email: me.user.email.clone(),
            }),
            status: if me.is_some() {
                l.status.clone()
            } else {
                SyncStatus::SignedOut
            },
            busy: l.busy.clone(),
            online: self.cloud.link == Link::Online,
            organizations: if me.is_some() {
                l.organizations.clone()
            } else {
                Vec::new()
            },
        }
    }

    /// After every message: the library follows the sign-in and the
    /// connection, and the sheet mode hears what changed.
    pub(crate) fn follow_sheet_library(&mut self, now: Instant) -> Task<Message> {
        // Without a place for the sheets (the tests, the snapshots) there is no library to sync.
        let signed = self
            .cloud
            .signed_in()
            .and(self.cloud.me.as_ref())
            .filter(|_| self.sheet_store.is_some())
            .map(|me| (Account::of(me), lib::personal_of(me)));
        let online = self.cloud.link == Link::Online;
        let mut tasks = Vec::new();
        let current = self
            .sheet_library
            .account
            .as_ref()
            .map(|(a, _)| a.id.clone());
        match (current, signed) {
            (None, Some(a)) => {
                self.sheet_library.signed_out = false;
                tasks.push(self.library_start(a));
            }
            (Some(old), Some(a)) if old != a.0.id => {
                self.sheet_library.stop();
                tasks.push(self.library_start(a));
            }
            (Some(_), None) => {
                self.sheet_library.stop();
                self.sheet_library.signed_out = true;
                // The lists hide that account's copies.
                tasks.push(self.sheet_event(LibraryEvent::Changed));
            }
            _ => {}
        }
        // The connection came back: run now, and hear events again.
        if online && !self.sheet_library.online && self.sheet_library.account.is_some() {
            self.sheet_library.failures = 0;
            self.sheet_library.retry_at = None;
            tasks.push(self.library_run());
            if self.sheet_library.cursor.is_none() {
                tasks.push(self.library_read_cursor());
            } else if self.sheet_library.listening.is_none() {
                self.sheet_library.listen_at = Some(now);
            }
        }
        self.sheet_library.online = online;
        let view = self.library_view();
        if self.sheet_library.told.as_ref() != Some(&view) {
            self.sheet_library.told = Some(view.clone());
            tasks.push(self.sheet_event(LibraryEvent::State(view)));
        }
        Task::batch(tasks)
    }

    /// The sheet mode hears something of the library; what it asks back is done.
    fn sheet_event(&mut self, e: LibraryEvent) -> Task<Message> {
        let effects = self.sheets.library_event(e);
        self.sheet_effects(effects)
    }

    fn library_start(&mut self, account: (Account, Option<Uuid>)) -> Task<Message> {
        let l = &mut self.sheet_library;
        l.generation += 1;
        l.account = Some(account);
        l.status = SyncStatus::Syncing;
        l.failures = 0;
        l.cursor = None;
        self.library_read_cursor()
    }

    /// The account's newest event cursor, then a sync and the long poll.
    fn library_read_cursor(&mut self) -> Task<Message> {
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        if self.sheet_library.reading.is_some() {
            return Task::none();
        }
        let generation = self.sheet_library.generation;
        let (task, handle) = Task::perform(
            async move { lib::newest_cursor(&client).await },
            move |result| msg(LibraryMsg::Cursor { generation, result }),
        )
        .abortable();
        self.sheet_library.reading = Some(handle.abort_on_drop());
        task
    }

    /// One sync now; asked while one runs, another follows it.
    pub(crate) fn library_run(&mut self) -> Task<Message> {
        let (Some(client), Some(store)) =
            (self.cloud.signed_in().cloned(), self.sheet_store.clone())
        else {
            return Task::none();
        };
        let Some((account, personal)) = self.sheet_library.account.clone() else {
            return Task::none();
        };
        if self.sheet_library.running.is_some() {
            self.sheet_library.again = true;
            return Task::none();
        }
        let generation = self.sheet_library.generation;
        let device: Arc<dyn DeviceLibrary> = Arc::new(StoreLibrary(store));
        let (tell, told) = mpsc::unbounded::<(String, bool)>();
        let run = async move {
            let busy = move |id: &str, on: bool| {
                let _ = tell.unbounded_send((id.to_owned(), on));
            };
            lib::sync_once(&client, personal, &account, &*device, &busy).await
        };
        let (task, handle) = Task::perform(run, move |report| {
            msg(LibraryMsg::Synced {
                generation,
                report: Box::new(report),
            })
        })
        .abortable();
        self.sheet_library.running = Some(handle.abort_on_drop());
        self.sheet_library.status = SyncStatus::Syncing;
        let busy = Task::run(told, move |(id, on)| {
            msg(LibraryMsg::Busy { generation, id, on })
        });
        Task::batch([task, busy])
    }

    /// The library's messages.
    pub(crate) fn sheet_library_message(&mut self, m: LibraryMsg) -> Task<Message> {
        let now = Instant::now();
        let current = self.sheet_library.generation;
        match m {
            LibraryMsg::Cursor { generation, result } if generation == current => {
                self.sheet_library.reading = None;
                match result {
                    Ok(cursor) => {
                        self.sheet_library.cursor = Some(cursor);
                        self.sheet_library.listen_at = Some(now);
                        self.library_run()
                    }
                    Err(e) => self.library_failed(&e, now),
                }
            }
            LibraryMsg::Busy { generation, id, on } if generation == current => {
                if on {
                    self.sheet_library.busy.insert(id);
                } else {
                    self.sheet_library.busy.remove(&id);
                }
                Task::none()
            }
            LibraryMsg::Synced { generation, report } if generation == current => {
                self.library_synced(*report, now)
            }
            LibraryMsg::Heard { generation, result } if generation == current => {
                self.sheet_library.listening = None;
                match result {
                    Ok(page) => {
                        self.sheet_library.cursor = Some(page.next);
                        if !page.events.is_empty() && self.sheet_library.settle_at.is_none() {
                            self.sheet_library.settle_at = Some(now + SETTLE);
                        }
                        self.sheet_library.listen_at = Some(now);
                        Task::none()
                    }
                    Err(e) => {
                        // A cursor the server no longer knows: the newest one is read again at the retry.
                        if !e.transient() && e.status != 401 {
                            self.sheet_library.cursor = None;
                        }
                        self.library_failed(&e, now)
                    }
                }
            }
            LibraryMsg::Access { id, result } => self.sheet_event(LibraryEvent::Access {
                id,
                result: result.map_err(|e| failure_text(&e, share_texts::READ_FAILED)),
            }),
            LibraryMsg::Candidates { id, query, result } => {
                self.sheet_event(LibraryEvent::Candidates {
                    id,
                    query,
                    result: result.map_err(|e| failure_text(&e, share_texts::FIND_FAILED)),
                })
            }
            LibraryMsg::Shared {
                id,
                person,
                role,
                change,
                result,
            } => {
                let failed = match (role, change) {
                    (None, _) => share_texts::REVOKE_FAILED,
                    (Some(_), true) => share_texts::CHANGE_FAILED,
                    (Some(_), false) => share_texts::SHARE_FAILED,
                };
                let said = result
                    .as_ref()
                    .map(|c| match role {
                        Some(r) => share_texts::shared(&person, r, c.changed || change),
                        None => share_texts::revoked(&person),
                    })
                    .map_err(|e| failure_text(e, failed));
                // What changed is said in the log too, with the template's name.
                if let (Ok(text), Ok(c)) = (&said, &result)
                    && (c.changed || change || role.is_none())
                {
                    let name = self
                        .sheet_store
                        .as_ref()
                        .and_then(|s| s.template(&id))
                        .map(|r| r.template.meta.name)
                        .unwrap_or_default();
                    self.say(Level::Success, format!("“{name}”: {text}"));
                }
                // The cloud tells the owner too: the list follows at the next sync.
                let run = if result.is_ok() {
                    self.library_run()
                } else {
                    Task::none()
                };
                Task::batch([
                    self.sheet_event(LibraryEvent::Shared { id, result: said }),
                    run,
                ])
            }
            LibraryMsg::Published {
                id,
                name,
                organization_name,
                result,
            } => {
                let said = match &result {
                    Ok(c) => Ok((
                        texts::published(&name, &organization_name),
                        Some(c.template_id.clone()),
                    )),
                    Err(e) => Err(failure_why(e)),
                };
                // The organisation's copy comes with the next run; its members hear of it.
                let run = if result.is_ok() {
                    self.library_run()
                } else {
                    Task::none()
                };
                Task::batch([
                    self.sheet_event(LibraryEvent::Published { id, result: said }),
                    run,
                ])
            }
            LibraryMsg::Tick => self.library_tick(now),
            // An answer of an earlier account's library.
            _ => Task::none(),
        }
    }

    /// The timers: the next wait for events, a retry, a run after events.
    fn library_tick(&mut self, now: Instant) -> Task<Message> {
        let mut tasks = Vec::new();
        let l = &mut self.sheet_library;
        if l.retry_at.is_some_and(|at| now >= at) {
            l.retry_at = None;
            if l.cursor.is_none() {
                tasks.push(self.library_read_cursor());
            } else {
                tasks.push(self.library_run());
                self.sheet_library.listen_at.get_or_insert(now);
            }
        }
        let l = &mut self.sheet_library;
        if l.settle_at.is_some_and(|at| now >= at) {
            l.settle_at = None;
            tasks.push(self.library_run());
        }
        let l = &mut self.sheet_library;
        if l.listen_at.is_some_and(|at| now >= at) && l.listening.is_none() {
            l.listen_at = None;
            tasks.push(self.library_listen());
        }
        Task::batch(tasks)
    }

    /// Waits on the server for the account's template events after the cursor.
    fn library_listen(&mut self) -> Task<Message> {
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let (Some(cursor), true) = (
            self.sheet_library.cursor.clone(),
            self.sheet_library.account.is_some(),
        ) else {
            return Task::none();
        };
        let generation = self.sheet_library.generation;
        let (task, handle) = Task::perform(api::events(&client, &cursor, true), move |result| {
            msg(LibraryMsg::Heard { generation, result })
        })
        .abortable();
        self.sheet_library.listening = Some(handle.abort_on_drop());
        task
    }

    /// The library could not reach the server, or was refused.
    fn library_failed(&mut self, e: &ApiFailure, now: Instant) -> Task<Message> {
        if e.status == 401 {
            return self.library_signed_out(&e.message);
        }
        // The library's own state only: the connection's is the rest of the cloud's to judge.
        if e.transient() {
            self.sheet_library.status = SyncStatus::Offline;
        } else {
            self.sheet_library.status = SyncStatus::Failed(e.message.clone());
        }
        self.sheet_library.retry_later(now);
        Task::none()
    }

    /// The session ended (401): nothing more is synced until the account signs in again.
    fn library_signed_out(&mut self, why: &str) -> Task<Message> {
        self.cloud.me = None;
        self.warn(format!(
            "Bulut oturumunuz sona erdi ({why}); pafta şablonları eşitlenmiyor. Yeniden giriş yapın."
        ));
        Task::none()
    }

    /// A sync's end: what it said, what changed, how it ended, whether another follows.
    fn library_synced(&mut self, report: SyncReport, now: Instant) -> Task<Message> {
        self.sheet_library.running = None;
        self.sheet_library.busy.clear();
        if let Some(o) = &report.organizations {
            self.sheet_library.organizations = o.clone();
        }
        for (tone, text) in &report.said {
            self.say(level(*tone), text.clone());
        }
        let mut tasks = Vec::new();
        if !report.created.is_empty() {
            tasks.push(self.sheet_event(LibraryEvent::Created(report.created.clone())));
        } else if report.changed {
            tasks.push(self.sheet_event(LibraryEvent::Changed));
        }
        // “Buluta eşitle”: went up, or goes when the connection comes back.
        for (id, name) in std::mem::take(&mut self.sheet_library.uploads) {
            let made = report.created.iter().any(|(old, _)| *old == id);
            if made {
                self.say(Level::Success, texts::uploaded(&name));
            } else {
                self.say(Level::Warn, texts::not_uploaded(&name));
                tasks.push(self.sheet_event(LibraryEvent::NotUploaded(id)));
            }
        }
        match &report.end {
            SyncEnd::Synced => {
                self.sheet_library.failures = 0;
                let at = crate::cloud::local_time::clock(
                    u64::try_from(crate::cloud::now_ms()).unwrap_or(0),
                    crate::cloud::local_time::Zone::system(),
                );
                self.sheet_library.status =
                    SyncStatus::Synced(at.get(..5).unwrap_or(&at).to_owned());
            }
            SyncEnd::Offline => {
                self.sheet_library.status = SyncStatus::Offline;
                self.sheet_library.retry_later(now);
            }
            SyncEnd::SignedOut => {
                tasks.push(self.library_signed_out("401"));
                return Task::batch(tasks);
            }
            SyncEnd::Failed(reason) => {
                self.sheet_library.status = SyncStatus::Failed(reason.clone());
                self.sheet_library.retry_later(now);
            }
        }
        if std::mem::take(&mut self.sheet_library.again) || report.again {
            tasks.push(self.library_run());
        }
        Task::batch(tasks)
    }

    /// What the gallery and the share window ask of the cloud.
    pub(crate) fn library_request(&mut self, r: LibraryRequest) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let personal = self.sheet_library.account.as_ref().and_then(|(_, p)| *p);
        match r {
            LibraryRequest::Sync => self.library_run(),
            LibraryRequest::Upload { id, name } => {
                self.sheet_library.uploads.push((id, name));
                self.library_run()
            }
            LibraryRequest::Access { id } => {
                let (Some(client), Ok(uuid)) = (client, Uuid::parse_str(&id)) else {
                    return self.sheet_event(LibraryEvent::Access {
                        id,
                        result: Err(texts::SIGN_IN.to_owned()),
                    });
                };
                Task::perform(api::access(&client, uuid), move |result| {
                    msg(LibraryMsg::Access { id, result })
                })
            }
            LibraryRequest::Candidates { id, query } => {
                let (Some(client), Ok(uuid)) = (client, Uuid::parse_str(&id)) else {
                    return self.sheet_event(LibraryEvent::Candidates {
                        id,
                        query,
                        result: Err(texts::SIGN_IN.to_owned()),
                    });
                };
                let q = query.clone();
                Task::perform(api::candidates(&client, uuid, &q), move |result| {
                    msg(LibraryMsg::Candidates { id, query, result })
                })
            }
            LibraryRequest::Share {
                id,
                user,
                person,
                role,
                change,
            } => {
                let ids = (Uuid::parse_str(&id), Uuid::parse_str(&user));
                let (Some(client), Some(personal), (Ok(uuid), Ok(user))) = (client, personal, ids)
                else {
                    return self.sheet_event(LibraryEvent::Shared {
                        id,
                        result: Err(texts::SIGN_IN.to_owned()),
                    });
                };
                Task::perform(
                    api::share(&client, personal, uuid, user, role, Uuid::now_v7()),
                    move |result| {
                        msg(LibraryMsg::Shared {
                            id,
                            person,
                            role: Some(role),
                            change,
                            result,
                        })
                    },
                )
            }
            LibraryRequest::Publish {
                id,
                name,
                organization,
                organization_name,
            } => {
                let ids = (Uuid::parse_str(&id), Uuid::parse_str(&organization));
                let (Some(client), (Ok(uuid), Ok(org))) = (client, ids) else {
                    return self.sheet_event(LibraryEvent::Published {
                        id,
                        result: Err(texts::SIGN_IN.to_owned()),
                    });
                };
                Task::perform(
                    api::publish(&client, org, uuid, Uuid::now_v7()),
                    move |result| {
                        msg(LibraryMsg::Published {
                            id,
                            name,
                            organization_name,
                            result,
                        })
                    },
                )
            }
            LibraryRequest::Unshare { id, user, person } => {
                let ids = (Uuid::parse_str(&id), Uuid::parse_str(&user));
                let (Some(client), Some(personal), (Ok(uuid), Ok(user))) = (client, personal, ids)
                else {
                    return self.sheet_event(LibraryEvent::Shared {
                        id,
                        result: Err(texts::SIGN_IN.to_owned()),
                    });
                };
                Task::perform(
                    api::unshare(&client, personal, uuid, user, Uuid::now_v7()),
                    move |result| {
                        msg(LibraryMsg::Shared {
                            id,
                            person,
                            role: None,
                            change: false,
                            result,
                        })
                    },
                )
            }
        }
    }
}

#[cfg(test)]
#[path = "sheet_library_tests.rs"]
mod tests;

//! Portal dosyaları (docs/adr/9001): the cloud portal's dossiers (docs/adr/9000 §5–8) in the desktop. A professional
//! sees the applications they sent and the transfers sent to them, a municipality member their organisation's; one
//! dossier opens with its packages by round, its messages, its timeline and the steps the server allows now. A KentOS
//! project file of it opens as a cloud project in the drawing; the open cloud project goes into the package being
//! prepared and is sent or delivered from here, without the web portal.
//!
//! Every request is a task with an id; a late answer for another list or dossier is dropped. The server decides every
//! step (`actions`, `canAddFiles`, `canMessage`, `canImport`) and words every name; the window shows what it answers.
//!
//! | File | What it holds |
//! |---|---|
//! | `mod.rs` | the window's state and its messages, opened and closed |
//! | `reading.rs` | the places, the filters, the account, the lists and one dossier asked and answered |
//! | `plan.rs` | what the window decides and offers, from the server's answers (no Iced) |
//! | `words.rs` | what it calls things: numbers, places, days, packages, files, messages (no Iced) |
//! | `facts.rs` | a dossier's facts, the other side's and the document list's line (no Iced) |
//! | `steps.rs` | a step's question and the step taken; a project file opened as a project |
//! | `package.rs` | the open project into the package, a pending file taken out, a file saved to disk |
//! | `messages.rs` | a message written and sent to the other side |
//! | `view.rs` | the window: places, filters, the list, the chosen dossier's head, steps and notices |
//! | `view_tabs.rs` | the chosen dossier's tabs: package, messages, timeline, facts |
//! | `view_ask.rs` | a step's or a project file's question over the window |

pub mod facts;
pub mod plan;
pub mod words;

mod messages;
mod package;
mod reading;
mod steps;
mod view;
mod view_ask;
mod view_tabs;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod plan_tests;
#[cfg(test)]
mod screens;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::time::Instant;

use iced::Task;
use iced::widget::text_editor;
use kentos_cloud::ApiFailure;
use kentos_commands::{
    DossierAction, DossierDetail, DossierFileImported, DossierFileView, DossierList,
    DossierMessageView, PortalCatalog, PortalMe,
};

use crate::app::{App, Dialog, Message};

use reading::SEARCH_WAIT;
pub use reading::{Direction, Place, places};
pub use steps::{Asking, Made, Question};

/// The most dossiers a list asks for; a longer one is narrowed by searching.
pub const LIMIT: u32 = 100;

/// The chosen dossier's tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Package,
    Messages,
    History,
    Facts,
}

/// What the window keeps once it is closed, so that it opens where it was.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    pub place: Option<Place>,
    pub direction: Direction,
    pub chosen: Option<String>,
}

/// A line at the window's foot: what was done, or what went wrong and how to go on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    Info(String),
    Done(String),
    Error(String),
}

/// The window.
pub struct Portal {
    /// The account on the portal and the names the portal speaks in; none until the server answered.
    pub me: Option<PortalMe>,
    pub catalog: Option<PortalCatalog>,
    pub place: Option<Place>,
    pub direction: Direction,
    pub search: String,
    /// When the typed search goes to the server.
    pub search_at: Option<Instant>,
    pub list: Option<DossierList>,
    pub chosen: Option<String>,
    /// The chosen dossier, as the server answered it last.
    pub detail: Option<DossierDetail>,
    pub tab: Tab,
    /// The message being written to the other side.
    pub draft: text_editor::Content,
    /// A step's question, or a project file's, over the window (steps.rs).
    pub asking: Option<Asking>,
    pub said: Option<Said>,
    /// Why the account or the list could not be read: the window says it and offers to try again.
    pub failed: Option<String>,
    /// A change on its way (a step, a message, a file): its id; the buttons wait for its answer.
    pub busy: Option<u64>,
    /// A project file opened as a project that did not open in the drawing (steps.rs): offered again.
    pub made: Option<Made>,
    /// The open database project's add, waiting for its edits to reach the server first (package.rs).
    pub adding: bool,
    /// The file being saved to disk, while its place is asked.
    pub download: Option<String>,
    requests: Requests,
}

/// The ids of the reads on their way: an answer with another id is a late one.
#[derive(Debug, Default)]
struct Requests {
    me: u64,
    list: u64,
    detail: u64,
}

impl Portal {
    fn new(memory: &Memory) -> Self {
        Self {
            me: None,
            catalog: None,
            place: memory.place.clone(),
            direction: memory.direction,
            search: String::new(),
            search_at: None,
            list: None,
            chosen: memory.chosen.clone(),
            detail: None,
            tab: Tab::Package,
            draft: text_editor::Content::new(),
            asking: None,
            said: None,
            failed: None,
            busy: None,
            made: None,
            adding: false,
            download: None,
            requests: Requests::default(),
        }
    }

    /// What it keeps once closed.
    pub fn memory(&self) -> Memory {
        Memory {
            place: self.place.clone(),
            direction: self.direction,
            chosen: self.chosen.clone(),
        }
    }

    /// The chosen dossier, when its page has come.
    pub fn chosen_detail(&self) -> Option<&DossierDetail> {
        self.detail
            .as_ref()
            .filter(|d| Some(&d.summary.id) == self.chosen.as_ref())
    }

    /// Whether a change is on its way.
    pub fn working(&self) -> bool {
        self.busy.is_some() || self.adding
    }

    /// The list's request on its way (tests answer it).
    #[cfg(test)]
    pub fn list_request(&self) -> u64 {
        self.requests.list
    }

    /// The dossier's request on its way (tests answer it).
    #[cfg(test)]
    pub fn detail_request(&self) -> u64 {
        self.requests.detail
    }

    /// The account's request on its way (tests answer it).
    #[cfg(test)]
    pub fn me_request(&self) -> u64 {
        self.requests.me
    }
}

/// What the window and its requests tell the app.
#[derive(Debug, Clone)]
pub enum Event {
    Loaded {
        id: u64,
        result: Result<Box<(PortalMe, PortalCatalog)>, ApiFailure>,
    },
    Place(usize),
    Direction(Direction),
    Search(String),
    Listed {
        id: u64,
        result: Result<DossierList, ApiFailure>,
    },
    Choose(String),
    Detail {
        id: u64,
        result: Result<Box<DossierDetail>, ApiFailure>,
    },
    Tab(Tab),
    /// Yenile: the list and the chosen dossier asked again.
    Refresh,
    // ── A step and its question (steps.rs) ──────────────────────────────
    Step(DossierAction),
    AskNote(text_editor::Action),
    AskAssignee(usize),
    AskConfirm,
    AskClose,
    Acted {
        id: u64,
        result: Result<Box<DossierDetail>, ApiFailure>,
    },
    /// Proje olarak aç on a file (its id), and the server's answer.
    Import(String),
    Imported {
        id: u64,
        result: Result<DossierFileImported, ApiFailure>,
    },
    /// The project a file became, opened again after it did not open.
    OpenMade,
    // ── The package and the messages (package.rs) ───────────────────────
    /// The open cloud project's present state into the package being prepared.
    AddOpen,
    /// Kaydet on the open file project, so that its newest revision is what goes in.
    SaveOpen,
    Added {
        id: u64,
        result: Result<DossierFileView, ApiFailure>,
    },
    /// Paketten çıkar on a pending file (its id).
    Remove(String),
    Removed {
        id: u64,
        name: String,
        result: Result<(), ApiFailure>,
    },
    /// İndir… on a file (its id), where it goes, and how it went.
    Download(String),
    DownloadTo {
        id: u64,
        path: Option<PathBuf>,
    },
    Downloaded {
        id: u64,
        result: Result<(PathBuf, usize), ApiFailure>,
    },
    Draft(text_editor::Action),
    Send,
    Sent {
        id: u64,
        result: Result<DossierMessageView, ApiFailure>,
    },
}

/// The app's message of a portal event.
pub fn msg(event: Event) -> Message {
    crate::cloud::msg(crate::cloud::Event::Portal(event))
}

impl App {
    /// Portal dosyaları (`cloud.portal`): the window where it was left, the account and the list asked again.
    pub(crate) fn open_portal(&mut self) -> Task<Message> {
        self.dialog = Some(Dialog::Portal);
        if let Some(p) = self.cloud.portal.as_mut() {
            p.said = None;
            if p.me.is_some() {
                return Task::batch([self.portal_list(), self.portal_detail()]);
            }
        } else {
            self.cloud.portal = Some(Portal::new(&self.cloud.portal_memory));
        }
        self.portal_load()
    }

    /// Esc, ×, Kapat: a question over the window goes first; then the window, keeping where it was.
    pub(crate) fn close_portal(&mut self) {
        let Some(p) = self.cloud.portal.as_mut() else {
            return;
        };
        if p.asking.is_some() {
            if !p.asking.as_ref().is_some_and(|a| a.busy) {
                p.asking = None;
            }
            self.dialog = Some(Dialog::Portal);
            return;
        }
        self.cloud.portal_memory = p.memory();
        self.cloud.portal = None;
    }

    pub(crate) fn portal_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Loaded { id, result } => self.portal_loaded(id, result),
            Event::Place(i) => {
                let Some(p) = self.cloud.portal.as_mut() else {
                    return Task::none();
                };
                let chosen = p.me.as_ref().and_then(|me| places(me).into_iter().nth(i));
                if chosen.is_none() || chosen == p.place {
                    return Task::none();
                }
                p.place = chosen;
                p.list = None;
                p.chosen = None;
                p.detail = None;
                self.portal_list()
            }
            Event::Direction(d) => {
                let Some(p) = self.cloud.portal.as_mut() else {
                    return Task::none();
                };
                if p.direction == d {
                    return Task::none();
                }
                p.direction = d;
                self.portal_list()
            }
            Event::Search(text) => {
                if let Some(p) = self.cloud.portal.as_mut() {
                    p.search = text;
                    p.search_at = Some(Instant::now() + SEARCH_WAIT);
                }
                Task::none()
            }
            Event::Listed { id, result } => self.portal_listed(id, result),
            Event::Choose(dossier) => {
                let Some(p) = self.cloud.portal.as_mut() else {
                    return Task::none();
                };
                if p.chosen.as_deref() == Some(dossier.as_str()) {
                    return Task::none();
                }
                p.chosen = Some(dossier);
                // The message was being written to the dossier left.
                p.draft = text_editor::Content::new();
                p.said = None;
                self.portal_detail()
            }
            Event::Detail { id, result } => {
                let Some(p) = self
                    .cloud
                    .portal
                    .as_mut()
                    .filter(|p| p.requests.detail == id)
                else {
                    return Task::none();
                };
                match result {
                    Ok(d) => p.detail = Some(*d),
                    Err(f) => {
                        p.detail = None;
                        p.said = Some(Said::Error(format!(
                            "Dosya okunamadı: {} Listeyi yenileyip yeniden seçin.",
                            f.message
                        )));
                    }
                }
                Task::none()
            }
            Event::Tab(tab) => {
                if let Some(p) = self.cloud.portal.as_mut() {
                    p.tab = tab;
                }
                Task::none()
            }
            Event::Refresh => {
                let load = self
                    .cloud
                    .portal
                    .as_ref()
                    .is_some_and(|p| p.me.is_none() || p.failed.is_some());
                if load {
                    return self.portal_load();
                }
                Task::batch([self.portal_list(), self.portal_detail()])
            }
            Event::Step(_)
            | Event::AskNote(_)
            | Event::AskAssignee(_)
            | Event::AskConfirm
            | Event::AskClose
            | Event::Acted { .. }
            | Event::Import(_)
            | Event::Imported { .. }
            | Event::OpenMade => self.portal_step_event(event),
            Event::AddOpen
            | Event::SaveOpen
            | Event::Added { .. }
            | Event::Remove(_)
            | Event::Removed { .. }
            | Event::Download(_)
            | Event::DownloadTo { .. }
            | Event::Downloaded { .. }
            | Event::Draft(_)
            | Event::Send
            | Event::Sent { .. } => self.portal_package_event(event),
        }
    }
}

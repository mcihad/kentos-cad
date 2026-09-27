//! Projeyi paylaş on the desktop (docs/adr/0111; the web's ShareDialog.ts,
//! shareFind.ts and shareInvites.ts): who can use a cloud project, with
//! the role each works with now and where it comes from, and where the
//! project keeps its content. Kişiler adds a person found by name or e-mail
//! among those the account may share with, with a role and an optional end;
//! a grant's role changes in its row and is taken away after a question.
//! Davetler invites by e-mail (an address, a role up to Düzenleyici, how
//! long it waits); a new invitation's link is shown once, with Kopyala, and
//! kept nowhere else, not in the log. The server decides every request
//! (`project.share`): the controls are on only once it has listed the
//! people, and a refusal is said in its words with what to do. The words
//! and rules are share_plan.rs's (fixtures/cloud/v1/share.json).
//!
//! The window opens over the catalog, from the selected project's Paylaş…,
//! or alone for the open project (`cloud.share`). Every answer carries the
//! window's id: one that comes after the window closed changes nothing, and
//! a search's answer counts only while its words are still typed.

use std::collections::BTreeMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use iced::Task;
use iced::task::Handle;
use kentos_cloud::ApiFailure;
use kentos_cloud::saving::envelope;
use kentos_contracts::{
    AuthConfig, CommandEnvelope, GrantRole, InvitationChange, PROJECT_ACCESS_REVOKE,
    PROJECT_ACCESS_REVOKE_VERSION, PROJECT_INVITATION_REVOKE, PROJECT_INVITATION_REVOKE_VERSION,
    PROJECT_INVITE, PROJECT_INVITE_VERSION, PROJECT_SHARE, PROJECT_SHARE_VERSION,
    ProjectAccessChange, ProjectAccessList, ProjectInvitation, ProjectInvitations,
    ProjectPermission, ShareCandidate, ShareCandidates, TenantKind,
};
use kentos_domain::Uuid;
use kentos_expression::js::text::trim;
use kentos_interaction::Level;
use kentos_ui::attribute::Date;

use crate::app::{App, Dialog, Message};
use crate::cloud::catalog::{Said, place_of};
use crate::cloud::local_time::Zone;
use crate::cloud::plan::{self as catalog_plan, DetailAction};
use crate::cloud::share_plan::{self as plan, PersonRow, Question, find, invites, people};
use crate::cloud::uuid;

/// How long typing rests before the finder asks the server (the web's 250 ms).
const SEARCH: Duration = Duration::from_millis(250);

/// The fields the window focuses: Kişi ekle's finder and the invitation's address.
pub const FIND_FIELD: &str = "share-find";
pub const EMAIL_FIELD: &str = "share-email";

/// The project the window shares (the web's `ProjectTarget`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub tenant_id: String,
    /// The workspace as the catalog names it.
    pub tenant_name: String,
    pub project_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    People,
    Invites,
}

/// A list the server gives: on its way, shown, or why it could not be read.
#[derive(Debug, Clone, PartialEq)]
pub enum Listed<T> {
    Loading,
    Ready(T),
    Failed(String),
}

/// A question over the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Asking {
    /// Erişimi kaldır, for this row.
    Revoke(PersonRow),
    /// A new invitation that replaces a waiting one, or goes to someone who
    /// can use the project already.
    Invite { address: String, question: Question },
    /// Daveti geri al.
    InvitationRevoke(ProjectInvitation),
}

impl Asking {
    /// What the question says and offers.
    pub fn question(&self, project: &str) -> Question {
        match self {
            Self::Revoke(row) => plan::revoke_question(&row.name, project, row.guest),
            Self::Invite { question, .. } => question.clone(),
            Self::InvitationRevoke(i) => plan::invitation_revoke_question(&i.email),
        }
    }

    /// Whether its action takes something away (the red button).
    pub fn destructive(&self) -> bool {
        !matches!(self, Self::Invite { .. })
    }
}

/// The window.
pub struct Share {
    /// This window: every answer carries it.
    pub id: u64,
    pub target: Target,
    /// Opened from the catalog, which stays under it.
    pub over_catalog: bool,
    pub tab: Tab,
    /// The people as the server last listed them, and why the last read failed.
    pub access: Option<ProjectAccessList>,
    pub access_failed: Option<String>,
    /// The server listed the people: the account may share (`project.share`).
    pub may_share: bool,
    /// The people's first answer came; the invitations are asked for after it.
    pub answered: bool,
    // ── Kişi ekle ───────────────────────────────────────────────────────
    pub query: String,
    /// What the server found for the words typed; none while the list is closed.
    pub found: Option<(String, Vec<ShareCandidate>)>,
    /// The person picked from the list.
    pub chosen: Option<ShareCandidate>,
    /// When the typed words go to the server.
    pub search_at: Option<Instant>,
    pub role: GrantRole,
    pub until: Option<Date>,
    /// A share or an invitation on its way: the forms wait for it.
    pub busy: bool,
    /// The person whose role is being changed in their row.
    pub changing: Option<String>,
    /// The line under the tabs: what the last action did, or why it failed.
    pub status: Option<Said>,
    pub asking: Option<Asking>,
    // ── Davetler ────────────────────────────────────────────────────────
    pub invitations: Listed<Vec<ProjectInvitation>>,
    pub email: String,
    pub invite_role: GrantRole,
    pub days: u32,
    /// The new invitation: what the panel says and its link, shown this once.
    pub invited: Option<(plan::Invited, Option<String>)>,
    pub copied: bool,
    /// Where the web app is (the server's `publicUrl`): the base of a link.
    pub base: Option<String>,
    /// The search on its way; dropping it stops it.
    searching: Option<Handle>,
}

impl Share {
    fn new(id: u64, target: Target, over_catalog: bool) -> Self {
        Self {
            id,
            target,
            over_catalog,
            tab: Tab::People,
            access: None,
            access_failed: None,
            may_share: false,
            answered: false,
            query: String::new(),
            found: None,
            chosen: None,
            search_at: None,
            // The web's defaults: a share gives Düzenleyici, an invitation
            // Görüntüleyici for 14 days.
            role: GrantRole::Editor,
            until: None,
            busy: false,
            changing: None,
            status: None,
            asking: None,
            invitations: Listed::Loading,
            email: String::new(),
            invite_role: GrantRole::Viewer,
            days: plan::INVITE_DAYS,
            invited: None,
            copied: false,
            base: None,
            searching: None,
        }
    }

    /// The controls that add someone take input.
    pub fn open_to_input(&self) -> bool {
        self.may_share && !self.busy
    }

    /// Whether the workspace is a personal one (what the finder and the rules say).
    pub fn personal(&self) -> bool {
        self.access
            .as_ref()
            .is_some_and(|a| a.tenant_kind == TenantKind::Personal)
    }

    /// The invitations shown, waiting ones first.
    pub fn invitation_list(&self) -> &[ProjectInvitation] {
        match &self.invitations {
            Listed::Ready(list) => list,
            _ => &[],
        }
    }
}

/// The window's messages.
#[derive(Debug, Clone)]
pub enum Event {
    /// Paylaş… on the catalog's selected project.
    Open,
    Tab(Tab),
    Access {
        id: u64,
        result: Result<ProjectAccessList, ApiFailure>,
    },
    Invitations {
        id: u64,
        result: Result<ProjectInvitations, ApiFailure>,
    },
    /// The server's addresses: where the web app is.
    Base {
        id: u64,
        result: Result<AuthConfig, ApiFailure>,
    },
    // ── Kişi ekle ───────────────────────────────────────────────────────
    Query(String),
    Found {
        id: u64,
        query: String,
        result: Result<ShareCandidates, ApiFailure>,
    },
    /// A found person, or the offer to invite the address typed.
    Pick(usize),
    Role(GrantRole),
    Until(Option<Date>),
    Submit,
    Shared {
        id: u64,
        who: ShareCandidate,
        role: GrantRole,
        had: bool,
        result: Result<ProjectAccessChange, ApiFailure>,
    },
    /// A row's role changed, and the answer.
    Change {
        user: String,
        role: GrantRole,
    },
    Changed {
        id: u64,
        name: String,
        role: GrantRole,
        result: Result<ProjectAccessChange, ApiFailure>,
    },
    /// A row's Kaldır, the question answered, and the answer.
    Revoke(String),
    Answer(bool),
    Revoked {
        id: u64,
        name: String,
        result: Result<ProjectAccessChange, ApiFailure>,
    },
    // ── Davetler ────────────────────────────────────────────────────────
    Email(String),
    InviteRole(GrantRole),
    Days(u32),
    Invite,
    Invited {
        id: u64,
        days: u32,
        result: Result<InvitationChange, ApiFailure>,
    },
    /// A waiting invitation's Geri al, by its id, and the answer.
    InvitationRevoke(String),
    InvitationRevoked {
        id: u64,
        email: String,
        result: Result<InvitationChange, ApiFailure>,
    },
    /// Kopyala on the new invitation's link.
    Copy,
    Close,
}

/// The app's message of a window event.
pub fn msg(event: Event) -> Message {
    crate::cloud::msg(crate::cloud::Event::Share(event))
}

fn focus(field: &'static str) -> Task<Message> {
    iced::widget::operation::focus(iced::widget::Id::new(field))
}

/// A picked day as the server's function takes it: `YYYY-MM-DD`.
fn iso_date(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

impl App {
    /// The window's messages.
    pub(crate) fn share_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => self.share_from_catalog(),
            Event::Close => self.share_close(),
            Event::Access { id, result } => self.share_access(id, result),
            Event::Invitations { id, result } => {
                if let Some(s) = self.share_of(id) {
                    s.invitations = match result {
                        Ok(r) => Listed::Ready(r.invitations),
                        Err(e) => Listed::Failed(plan::failure_text(&e, invites::READ_FAILED)),
                    };
                }
                Task::none()
            }
            Event::Base { id, result } => {
                if let (Some(s), Ok(config)) = (self.share_of(id), result) {
                    s.base = config.public_url;
                }
                Task::none()
            }
            Event::Tab(tab) => {
                let Some(s) = self.cloud.share.as_mut() else {
                    return Task::none();
                };
                s.tab = tab;
                s.found = None;
                if !s.may_share {
                    return Task::none();
                }
                focus(match tab {
                    Tab::People => FIND_FIELD,
                    Tab::Invites => EMAIL_FIELD,
                })
            }
            Event::Query(text) => {
                self.share_query(text);
                Task::none()
            }
            Event::Found { id, query, result } => {
                let Some(s) = self.share_of(id) else {
                    return Task::none();
                };
                // An older search's answer, or the words changed since.
                if trim(&s.query) != query || s.chosen.is_some() {
                    return Task::none();
                }
                s.searching = None;
                match result {
                    Ok(r) => s.found = Some((query, r.candidates)),
                    Err(e) => {
                        s.found = None;
                        s.status = Some(Said::error(plan::failure_text(&e, find::FAILED)));
                    }
                }
                Task::none()
            }
            Event::Pick(index) => self.share_pick(index),
            Event::Role(role) => {
                if let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) {
                    s.role = role;
                }
                Task::none()
            }
            Event::Until(day) => {
                if let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) {
                    s.until = day;
                }
                Task::none()
            }
            Event::Submit => self.share_submit(),
            Event::Shared {
                id,
                who,
                role,
                had,
                result,
            } => self.share_shared(id, who, role, had, result),
            Event::Change { user, role } => self.share_change(&user, role),
            Event::Changed {
                id,
                name,
                role,
                result,
            } => {
                let Some(s) = self.share_of(id) else {
                    return Task::none();
                };
                s.changing = None;
                match result {
                    Ok(_) => {
                        let text = plan::role_changed_text(&name, role);
                        s.status = Some(Said::info(text.clone()));
                        let line = plan::share_log(&s.target.name, &text);
                        self.say(Level::Success, line);
                        self.share_reload()
                    }
                    Err(e) => {
                        s.status = Some(Said::error(plan::failure_text(&e, people::CHANGE_FAILED)));
                        Task::none()
                    }
                }
            }
            Event::Revoke(user) => {
                let row = self.share_rows().into_iter().find(|r| r.user_id == user);
                if let (Some(s), Some(row)) = (self.cloud.share.as_mut(), row)
                    && row.can_revoke
                {
                    s.asking = Some(Asking::Revoke(row));
                }
                Task::none()
            }
            Event::Answer(yes) => self.share_answer(yes),
            Event::Revoked { id, name, result } => {
                let Some(s) = self.share_of(id) else {
                    return Task::none();
                };
                match result {
                    Ok(_) => {
                        let text = plan::revoked_text(&name);
                        s.status = Some(Said::info(text.clone()));
                        let line = plan::share_log(&s.target.name, &text);
                        self.say(Level::Success, line);
                        self.share_reload()
                    }
                    Err(e) => {
                        s.status = Some(Said::error(plan::failure_text(&e, people::REVOKE_FAILED)));
                        Task::none()
                    }
                }
            }
            Event::Email(text) => {
                if let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) {
                    s.email = text;
                }
                Task::none()
            }
            Event::InviteRole(role) => {
                if let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) {
                    s.invite_role = role;
                }
                Task::none()
            }
            Event::Days(days) => {
                if let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) {
                    s.days = days;
                }
                Task::none()
            }
            Event::Invite => self.share_invite(),
            Event::Invited { id, days, result } => self.share_invited(id, days, result),
            Event::InvitationRevoke(invitation) => {
                if let Some(s) = self.cloud.share.as_mut().filter(|s| s.may_share)
                    && let Some(i) = s
                        .invitation_list()
                        .iter()
                        .find(|i| i.id == invitation)
                        .cloned()
                {
                    s.asking = Some(Asking::InvitationRevoke(i));
                }
                Task::none()
            }
            Event::InvitationRevoked { id, email, result } => {
                let Some(s) = self.share_of(id) else {
                    return Task::none();
                };
                let project = s.target.name.clone();
                match result {
                    Ok(_) => {
                        s.status = Some(Said::info(plan::invitation_revoked_say(&email)));
                        self.say(
                            Level::Success,
                            plan::invitation_revoked_log(&project, &email),
                        );
                    }
                    Err(e) => {
                        s.status =
                            Some(Said::error(plan::failure_text(&e, invites::REVOKE_FAILED)));
                    }
                }
                self.share_load_invitations()
            }
            Event::Copy => {
                let Some(s) = self.cloud.share.as_mut() else {
                    return Task::none();
                };
                let Some((_, Some(link))) = &s.invited else {
                    return Task::none();
                };
                let link = link.clone();
                s.copied = true;
                s.status = Some(Said::info(invites::COPIED_SAY));
                iced::clipboard::write(link)
            }
        }
    }

    /// The open window, when an answer is for it.
    fn share_of(&mut self, id: u64) -> Option<&mut Share> {
        self.cloud.share.as_mut().filter(|s| s.id == id)
    }

    /// Kişiler's rows as the plan writes them.
    pub(crate) fn share_rows(&self) -> Vec<PersonRow> {
        let (Some(s), Some(me)) = (&self.cloud.share, &self.cloud.me) else {
            return Vec::new();
        };
        s.access.as_ref().map_or_else(Vec::new, |list| {
            plan::people_view(list, &me.user.id, s.may_share, Zone::system()).rows
        })
    }

    /// Paylaş… on the catalog's selected project, when its plan offers it.
    fn share_from_catalog(&mut self) -> Task<Message> {
        let Some(p) = self
            .cloud
            .catalog
            .as_ref()
            .filter(|c| !c.busy())
            .and_then(|c| c.picked().cloned())
        else {
            return Task::none();
        };
        let open = self.is_open_project(&p.id);
        let offered = catalog_plan::detail_plan(&p, open)
            .actions
            .iter()
            .any(|a| a.id == DetailAction::Share && a.why.is_none());
        if !offered || self.cloud.opening.is_some() {
            return Task::none();
        }
        let target = Target {
            tenant_id: p.tenant_id.clone(),
            tenant_name: place_of(self, &p),
            project_id: p.id.clone(),
            name: p.name.clone(),
        };
        self.open_share(target, true)
    }

    /// `cloud.share`: the open project's window.
    pub(crate) fn share_open_project(&mut self) -> Task<Message> {
        if !self.open_may(ProjectPermission::Share, false) {
            return Task::none();
        }
        let Some((doc, c)) = self
            .document
            .as_ref()
            .and_then(|d| Some((d, d.cloud_source()?)))
        else {
            return Task::none();
        };
        let target = Target {
            tenant_id: c.info.tenant_id.clone(),
            tenant_name: c.info.tenant_name.clone(),
            project_id: c.info.id.clone(),
            name: doc.name().to_owned(),
        };
        let task = self.open_share(target, false);
        if self.cloud.share.is_some() {
            self.dialog = Some(Dialog::Share);
        }
        task
    }

    /// Opens the window and asks who can use the project, and where the web app is.
    fn open_share(&mut self, target: Target, over_catalog: bool) -> Task<Message> {
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&target.tenant_id), uuid(&target.project_id))
        else {
            return Task::none();
        };
        let id = self.cloud.next_id();
        self.cloud.share = Some(Share::new(id, target, over_catalog));
        Task::batch([
            Task::perform(client.access(tenant, project), move |result| {
                msg(Event::Access { id, result })
            }),
            Task::perform(client.auth_config(), move |result| {
                msg(Event::Base { id, result })
            }),
        ])
    }

    /// Asks for the people again (after a share, a role changed, an access taken away).
    fn share_reload(&mut self) -> Task<Message> {
        let Some((client, s)) = self
            .cloud
            .signed_in()
            .cloned()
            .zip(self.cloud.share.as_ref())
        else {
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&s.target.tenant_id), uuid(&s.target.project_id))
        else {
            return Task::none();
        };
        let id = s.id;
        Task::perform(client.access(tenant, project), move |result| {
            msg(Event::Access { id, result })
        })
    }

    /// Asks for the invitations, or says the account may not see them.
    fn share_load_invitations(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(s) = self.cloud.share.as_mut() else {
            return Task::none();
        };
        let (Some(client), Some(tenant), Some(project)) = (
            client,
            uuid(&s.target.tenant_id),
            uuid(&s.target.project_id),
        ) else {
            return Task::none();
        };
        if !s.may_share {
            return Task::none();
        }
        let id = s.id;
        Task::perform(client.invitations(tenant, project), move |result| {
            msg(Event::Invitations { id, result })
        })
    }

    fn share_access(
        &mut self,
        id: u64,
        result: Result<ProjectAccessList, ApiFailure>,
    ) -> Task<Message> {
        let Some(s) = self.share_of(id) else {
            return Task::none();
        };
        let first = !s.answered;
        s.answered = true;
        match result {
            Ok(list) => {
                s.access = Some(list);
                s.access_failed = None;
                s.may_share = true;
            }
            Err(e) => {
                if e.code == "forbidden" {
                    s.may_share = false;
                }
                s.access_failed = Some(plan::failure_text(&e, people::READ_FAILED));
            }
        }
        if !first {
            return Task::none();
        }
        // Ready for the first person, or the first address (the web's `load().then(…)`).
        let field = (s.may_share).then_some(match s.tab {
            Tab::People => FIND_FIELD,
            Tab::Invites => EMAIL_FIELD,
        });
        Task::batch([
            field.map_or_else(Task::none, focus),
            self.share_load_invitations(),
        ])
    }

    /// The finder's words changed: a picked person no longer named goes,
    /// and the server is asked once the typing rests.
    fn share_query(&mut self, text: String) {
        let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) else {
            return;
        };
        s.query = text;
        let query = trim(&s.query).to_owned();
        if s.chosen.as_ref().is_some_and(|c| c.display_name != query) {
            s.chosen = None;
        }
        s.searching = None;
        if s.chosen.is_some() || !plan::searches_for(&query) {
            s.found = None;
            s.search_at = None;
            return;
        }
        s.search_at = Some(Instant::now() + SEARCH);
    }

    /// The timer: the typed words go to the server.
    pub(crate) fn share_tick(&mut self, now: Instant) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(s) = self.cloud.share.as_mut() else {
            return Task::none();
        };
        if s.search_at.is_none_or(|at| now < at) {
            return Task::none();
        }
        s.search_at = None;
        let (Some(client), Some(tenant), Some(project)) = (
            client,
            uuid(&s.target.tenant_id),
            uuid(&s.target.project_id),
        ) else {
            return Task::none();
        };
        let id = s.id;
        let query = trim(&s.query).to_owned();
        let asked = query.clone();
        let (task, handle) =
            Task::perform(client.candidates(tenant, project, &asked), move |result| {
                msg(Event::Found { id, query, result })
            })
            .abortable();
        s.searching = Some(handle.abort_on_drop());
        task
    }

    /// A row of the finder's list: a person, or the offer to invite the address.
    fn share_pick(&mut self, index: usize) -> Task<Message> {
        let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) else {
            return Task::none();
        };
        let Some((query, found)) = s.found.take() else {
            return Task::none();
        };
        if let Some(c) = found.get(index) {
            s.query = c.display_name.clone();
            s.chosen = Some(c.clone());
            s.status = None;
            return Task::none();
        }
        // Nobody found for a whole address: an invitation instead (docs/adr/0042).
        if found.is_empty() && plan::email_problem(&query).is_none() {
            s.tab = Tab::Invites;
            s.email = trim(&query).to_owned();
            return focus(EMAIL_FIELD);
        }
        s.found = Some((query, found));
        Task::none()
    }

    /// Paylaş: the picked person gets the role, until the end of the day picked.
    fn share_submit(&mut self) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) else {
            return Task::none();
        };
        let Some(who) = s.chosen.clone() else {
            return Task::none();
        };
        let expires = match s.until {
            Some(day) => match plan::end_of_day(&iso_date(day), Zone::system()) {
                Some(end) => Some(end),
                None => {
                    s.status = Some(Said::error(people::BAD_DATE));
                    return Task::none();
                }
            },
            None => None,
        };
        let Some((client, request)) = client.zip(command(
            s,
            PROJECT_SHARE,
            PROJECT_SHARE_VERSION,
            plan::share_input(&who.user_id, s.role, expires.as_deref()),
        )) else {
            return Task::none();
        };
        let role = s.role;
        // Shared already: sharing again changes the role (and the end).
        let had = s.access.as_ref().is_some_and(|l| {
            l.people
                .iter()
                .any(|p| p.user_id == who.user_id && p.grant.is_some())
        });
        s.busy = true;
        s.status = Some(Said::info(people::adding(&who.display_name)));
        let id = s.id;
        Task::perform(
            client.command::<ProjectAccessChange>(request),
            move |result| {
                msg(Event::Shared {
                    id,
                    who,
                    role,
                    had,
                    result,
                })
            },
        )
    }

    fn share_shared(
        &mut self,
        id: u64,
        who: ShareCandidate,
        role: GrantRole,
        had: bool,
        result: Result<ProjectAccessChange, ApiFailure>,
    ) -> Task<Message> {
        let Some(s) = self.share_of(id) else {
            return Task::none();
        };
        s.busy = false;
        match result {
            Ok(r) => {
                let text = plan::shared_text(&who.display_name, role, r.changed, had);
                // Ready for the next person.
                s.chosen = None;
                s.query.clear();
                s.found = None;
                s.until = None;
                s.status = Some(Said::info(text.clone()));
                let line = plan::share_log(&s.target.name, &text);
                if r.changed {
                    self.say(Level::Success, line);
                }
                Task::batch([self.share_reload(), focus(FIND_FIELD)])
            }
            Err(e) => {
                s.status = Some(Said::error(plan::failure_text(&e, people::SHARE_FAILED)));
                focus(FIND_FIELD)
            }
        }
    }

    /// A row's role changed: the same grant with the new role and its end.
    fn share_change(&mut self, user: &str, role: GrantRole) -> Task<Message> {
        let row = self.share_rows().into_iter().find(|r| r.user_id == user);
        let client = self.cloud.signed_in().cloned();
        let Some(s) = self.cloud.share.as_mut() else {
            return Task::none();
        };
        let Some(row) = row.filter(|r| r.can_change && r.grant.is_some_and(|g| g != role)) else {
            return Task::none();
        };
        if s.changing.is_some() {
            return Task::none();
        }
        let Some((client, request)) = client.zip(command(
            s,
            PROJECT_SHARE,
            PROJECT_SHARE_VERSION,
            plan::share_input(&row.user_id, role, row.expires_at.as_deref()),
        )) else {
            return Task::none();
        };
        s.changing = Some(row.user_id.clone());
        s.status = Some(Said::info(people::changing(&row.name)));
        let (id, name) = (s.id, row.name);
        Task::perform(
            client.command::<ProjectAccessChange>(request),
            move |result| {
                msg(Event::Changed {
                    id,
                    name,
                    role,
                    result,
                })
            },
        )
    }

    /// The question over the window answered.
    fn share_answer(&mut self, yes: bool) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(s) = self.cloud.share.as_mut() else {
            return Task::none();
        };
        let Some(asking) = s.asking.take() else {
            return Task::none();
        };
        if !yes {
            return Task::none();
        }
        let Some(client) = client else {
            return Task::none();
        };
        let id = s.id;
        match asking {
            Asking::Revoke(row) => {
                let Some(request) = command(
                    s,
                    PROJECT_ACCESS_REVOKE,
                    PROJECT_ACCESS_REVOKE_VERSION,
                    plan::revoke_input(&row.user_id),
                ) else {
                    return Task::none();
                };
                s.status = Some(Said::info(people::revoking(&row.name)));
                let name = row.name;
                Task::perform(
                    client.command::<ProjectAccessChange>(request),
                    move |result| msg(Event::Revoked { id, name, result }),
                )
            }
            Asking::Invite { address, .. } => self.share_send_invite(address),
            Asking::InvitationRevoke(i) => {
                let Some(request) = command(
                    s,
                    PROJECT_INVITATION_REVOKE,
                    PROJECT_INVITATION_REVOKE_VERSION,
                    plan::invitation_revoke_input(&i.id),
                ) else {
                    return Task::none();
                };
                s.status = Some(Said::info(invites::revoking(&i.email)));
                let email = i.email;
                Task::perform(client.command::<InvitationChange>(request), move |result| {
                    msg(Event::InvitationRevoked { id, email, result })
                })
            }
        }
    }

    /// Davet et: the address checked, a question when something it has would change.
    fn share_invite(&mut self) -> Task<Message> {
        let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) else {
            return Task::none();
        };
        let address = trim(&s.email).to_owned();
        if let Some(bad) = plan::email_problem(&address) {
            s.status = Some(Said::error(bad));
            return focus(EMAIL_FIELD);
        }
        if let Some(q) = plan::invite_question(&address, s.invitation_list(), s.access.as_ref()) {
            let question = plan::invite_ask(&address, &q, Zone::system());
            s.asking = Some(Asking::Invite { address, question });
            return Task::none();
        }
        self.share_send_invite(address)
    }

    fn share_send_invite(&mut self, address: String) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let Some(s) = self.cloud.share.as_mut().filter(|s| s.open_to_input()) else {
            return Task::none();
        };
        let days = s.days;
        let expires = plan::invite_expiry(days, now_ms());
        let Some((client, request)) = client.zip(command(
            s,
            PROJECT_INVITE,
            PROJECT_INVITE_VERSION,
            plan::invite_input(&address, s.invite_role, expires.as_deref()),
        )) else {
            return Task::none();
        };
        s.busy = true;
        s.status = Some(Said::info(invites::inviting(&address)));
        let id = s.id;
        Task::perform(client.command::<InvitationChange>(request), move |result| {
            msg(Event::Invited { id, days, result })
        })
    }

    fn share_invited(
        &mut self,
        id: u64,
        days: u32,
        result: Result<InvitationChange, ApiFailure>,
    ) -> Task<Message> {
        let server = self.cloud.client.as_ref().map(|c| c.server().to_owned());
        let Some(s) = self.share_of(id) else {
            return Task::none();
        };
        s.busy = false;
        match result {
            Ok(change) => {
                // The web app's address; an older server that does not say it
                // serves the web app at its own address.
                let base = s
                    .base
                    .clone()
                    .or_else(|| server.map(|server| format!("{}/", server.trim_end_matches('/'))));
                let link = change
                    .token
                    .as_deref()
                    .zip(base)
                    .map(|(token, base)| plan::invitation_link(token, &base));
                s.invited = Some((plan::invited_link(&change, days, Zone::system()), link));
                s.copied = false;
                s.email.clear();
                s.status = Some(Said::info(invites::CREATED));
                // The log says who was invited, never the link.
                let line = plan::invited_log(
                    &s.target.name,
                    &change.invitation.email,
                    change.invitation.role,
                );
                self.say(Level::Success, line);
                self.share_load_invitations()
            }
            Err(e) => {
                s.status = Some(Said::error(plan::failure_text(&e, invites::SEND_FAILED)));
                if e.path.as_deref() == Some("email") {
                    return focus(EMAIL_FIELD);
                }
                Task::none()
            }
        }
    }

    /// Kapat: the window goes; over the catalog its list is read again (the
    /// web's `done`), for the roles and the shares changed.
    pub(crate) fn share_close(&mut self) -> Task<Message> {
        let Some(s) = self.cloud.share.take() else {
            return Task::none();
        };
        if self.dialog == Some(Dialog::Share) {
            self.dialog = None;
        }
        if s.over_catalog && self.cloud.catalog.is_some() {
            return self.catalog_load(false);
        }
        Task::none()
    }

    /// Esc: the question goes first, then the window.
    pub(crate) fn share_escape(&mut self) -> Task<Message> {
        match self.cloud.share.as_mut() {
            Some(s) if s.asking.is_some() => {
                s.asking = None;
                Task::none()
            }
            Some(_) => self.share_close(),
            None => Task::none(),
        }
    }
}

/// A product command on the window's project, with its own idempotency key.
fn command(
    s: &Share,
    name: &str,
    version: u32,
    input: serde_json::Value,
) -> Option<CommandEnvelope> {
    let (tenant, project) = (uuid(&s.target.tenant_id)?, uuid(&s.target.project_id)?);
    Some(envelope(
        tenant,
        project,
        name,
        version,
        Uuid::new_v4(),
        BTreeMap::new(),
        input,
    ))
}

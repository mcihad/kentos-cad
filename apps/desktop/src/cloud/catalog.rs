//! “Bulut projeleri” (docs/adr/0041, 0086; the web's CatalogDialog.ts,
//! docs/adr/0028): the account's lists — recently opened, favourites, its
//! own, an organisation's, shared with it, archived, the trash — searched,
//! filtered by type, sorted and paged by the server after its access check;
//! the selected project on the right with its facts and every action on it
//! (catalog_actions.rs); and “Bu cihazdaki projeler”: the projects this
//! device keeps a copy of, which open without a connection (docs/adr/0043).
//! Without a session, or when the server does not answer, the window shows
//! that list. What a row, the pane and the main button say comes from the
//! plan (plan.rs), pinned with the web by fixtures/cloud/v1/catalog.json.
//! Opening asks about the drawing on screen first (leaving.rs), then shows
//! its progress here and can be stopped (opening.rs).

use std::time::{Duration, Instant};

use iced::Task;
use iced::task::Handle;
use kentos_cloud::{ApiFailure, CatalogQuery, Kept};
use kentos_contracts::{
    CatalogSort, CatalogView, ProjectDetails, ProjectPage, ProjectState, ProjectStorage,
    ProjectSummary, ProjectType, TenantKind,
};

use crate::app::{App, Dialog, Message, Then};
use crate::cloud::catalog_actions::{Act, Acting};
use crate::cloud::catalog_history::History;
use crate::cloud::{Event, plan, uuid, words};

/// Projects asked for at once (the web's page).
pub const PAGE: u32 = 50;
/// How long typing rests before the server is asked (the web's).
pub const SEARCH_DELAY: Duration = Duration::from_millis(250);
/// How long a selection rests before its counts are asked (the web's `DETAILS_MS`).
pub const DETAILS_DELAY: Duration = Duration::from_millis(120);

/// A list of the catalog: one of the server's, or this device's copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum List {
    View(CatalogView),
    /// “Bu cihazdaki projeler”: the copies kept on this device.
    Device,
}

/// What the server worked out on asking about the selected project (the
/// web's `DetailsState`).
#[derive(Debug, Clone, PartialEq)]
pub enum Details {
    /// Nothing to ask: no project, or one in the trash (it is not counted).
    None,
    Loading,
    /// A database project's objects, layers and extent.
    Database(Box<ProjectDetails>),
    /// A file project's content as its newest revision holds it: the server
    /// counts rows of a database project only.
    File {
        project: String,
        /// None before the first Kaydet.
        revision: Option<String>,
        objects: Option<String>,
    },
    Failed(String),
}

impl Details {
    /// The project these are about, once they are there.
    fn about(&self) -> Option<&str> {
        match self {
            Self::Database(d) => Some(&d.project.id),
            Self::File { project, .. } => Some(project),
            _ => None,
        }
    }
}

/// A move of the selection by the list's keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    Home,
    End,
}

/// The selected project's two tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Info,
    History,
}

/// The line under the list: what an action did, or why it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub text: String,
    pub error: bool,
}

impl Said {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            error: true,
        }
    }
}

/// The catalog window.
pub struct Catalog {
    pub list: List,
    pub search: String,
    /// When the typed search goes to the server.
    pub search_at: Option<Instant>,
    /// The type filter (“Tüm türler” when none).
    pub kind: Option<ProjectType>,
    /// The list's order: one of its view's (the first when a list is chosen).
    pub sort: CatalogSort,
    /// “Kurum projeleri”'s organisation (its tenant id).
    pub org: Option<String>,
    pub projects: Vec<ProjectSummary>,
    /// The copies on this device (“Bu cihazdaki projeler”).
    pub device: Vec<Kept>,
    pub total: u32,
    pub next: Option<String>,
    /// How long the trash keeps a project, in days (the server's answer).
    pub retention: u32,
    /// The page on its way: its id (an older answer is dropped) and request.
    loading: Option<(u64, Handle)>,
    /// Why the list could not be read.
    pub error: Option<String>,
    /// The selected project's id, and the row under the pointer.
    pub picked: Option<String>,
    pub hovered: Option<String>,
    /// The selected project's counts, and when they are asked.
    pub details: Details,
    pub details_at: Option<Instant>,
    details_asked: Option<(u64, Handle)>,
    pub tab: Tab,
    /// What the last action or open said.
    pub status: Option<Said>,
    /// A question over the window, about this project (catalog_actions.rs).
    pub asking: Option<(Act, ProjectSummary)>,
    /// The action on its way.
    pub(super) acting: Option<Acting>,
    /// The Geçmiş tab (catalog_history.rs).
    pub history: History,
    /// A project made here to select once the list shows it, and whether to
    /// open it then (the web's `wanted` and `openAfter`).
    pub(super) wanted: Option<(String, bool)>,
}

impl Catalog {
    pub fn loading(&self) -> bool {
        self.loading.is_some()
    }

    /// The page on its way (tests answer it).
    #[cfg(test)]
    pub(super) fn request(&self) -> Option<u64> {
        self.loading.as_ref().map(|(id, _)| *id)
    }

    /// The details request on its way (tests answer it).
    #[cfg(test)]
    pub(super) fn details_request(&self) -> Option<u64> {
        self.details_asked.as_ref().map(|(id, _)| *id)
    }

    pub fn picked(&self) -> Option<&ProjectSummary> {
        let id = self.picked.as_deref()?;
        self.projects.iter().find(|p| p.id == id)
    }

    pub fn picked_kept(&self) -> Option<&Kept> {
        let id = self.picked.as_deref()?;
        self.device.iter().find(|k| k.info.id == id)
    }

    /// Whether an action, a question or a form holds the window.
    pub fn busy(&self) -> bool {
        self.acting.is_some() || self.asking.is_some() || self.history.busy()
    }

    /// The selection moved (or the list changed under it): its counts are
    /// asked a moment after it settles, unless they are already here.
    fn select(&mut self, id: Option<String>) {
        let changed = self.picked != id;
        self.picked = id;
        let picked = self.picked().cloned();
        match picked {
            None => self.forget_details(),
            Some(p) if p.state == ProjectState::Trashed => self.forget_details(),
            Some(p) => {
                if self.details.about() == Some(p.id.as_str()) {
                    // Already here: the list's newer summary goes with them.
                    if let Details::Database(d) = &mut self.details {
                        d.project = p;
                    }
                } else if changed || !matches!(self.details, Details::Loading) {
                    self.details_asked = None;
                    self.details = Details::Loading;
                    self.details_at = Some(Instant::now() + DETAILS_DELAY);
                }
            }
        }
    }

    fn forget_details(&mut self) {
        self.details = Details::None;
        self.details_at = None;
        self.details_asked = None;
    }
}

/// The list the window opens on: the one shown last while the app runs.
static LAST: std::sync::Mutex<Option<List>> = std::sync::Mutex::new(None);

impl App {
    /// The account's organisations whose projects it may see (active, with a seat).
    pub(crate) fn organizations(&self) -> Vec<(String, String)> {
        self.cloud.me.as_ref().map_or_else(Vec::new, |me| {
            me.memberships
                .iter()
                .filter(|m| m.active && m.seat && m.tenant_kind == TenantKind::Organization)
                .map(|m| (m.tenant_id.clone(), m.tenant_name.clone()))
                .collect()
        })
    }

    /// Whether `id` is the project open here.
    pub(crate) fn is_open_project(&self, id: &str) -> bool {
        self.document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .is_some_and(|s| s.info.id == id)
    }

    /// Opens the window on the last list (on this device's copies without a
    /// session) and asks for its first page.
    pub(crate) fn open_catalog(&mut self) -> Task<Message> {
        let list = if self.cloud.me.is_none() {
            List::Device
        } else {
            LAST.lock()
                .ok()
                .and_then(|l| l.clone())
                .unwrap_or(List::View(CatalogView::Mine))
        };
        // “Kurum projeleri” shows the open project's organisation, else the first.
        let orgs = self.organizations();
        let open_tenant = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .map(|s| s.info.tenant_id.clone());
        let org = orgs
            .iter()
            .find(|(id, _)| Some(id) == open_tenant.as_ref())
            .or(orgs.first())
            .map(|(id, _)| id.clone());
        let sort = match &list {
            List::View(v) => words::view(*v).sorts[0],
            List::Device => CatalogSort::Updated,
        };
        self.cloud.catalog = Some(Catalog {
            list,
            search: String::new(),
            search_at: None,
            kind: None,
            sort,
            org,
            projects: Vec::new(),
            device: Vec::new(),
            total: 0,
            next: None,
            retention: 0,
            loading: None,
            error: None,
            picked: None,
            hovered: None,
            details: Details::None,
            details_at: None,
            details_asked: None,
            tab: Tab::Info,
            status: None,
            asking: None,
            acting: None,
            history: History::default(),
            wanted: None,
        });
        self.dialog = Some(Dialog::Catalog);
        self.catalog_load(false)
    }

    /// Asks for the first page of the list as it is now, or the next one.
    pub(crate) fn catalog_load(&mut self, more: bool) -> Task<Message> {
        let id = self.cloud.next_id();
        let kept = self.kept_projects();
        let client = self.cloud.signed_in().cloned();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        if c.list == List::Device {
            let text = c.search.trim().to_lowercase();
            c.device = kept
                .into_iter()
                .filter(|k| text.is_empty() || k.info.name.to_lowercase().contains(&text))
                .collect();
            c.loading = None;
            c.error = None;
            return Task::none();
        }
        let Some(client) = client else {
            c.list = List::Device;
            return self.catalog_load(false);
        };
        let List::View(view) = c.list else {
            return Task::none();
        };
        let tenant = if view == CatalogView::Organization {
            match c.org.as_deref().and_then(uuid) {
                Some(t) => Some(t),
                None => {
                    // No organisation to list: the window says so.
                    c.loading = None;
                    c.projects.clear();
                    c.total = 0;
                    c.next = None;
                    c.select(None);
                    return Task::none();
                }
            }
        } else {
            None
        };
        let mut query = CatalogQuery::new(view);
        query.tenant = tenant;
        query.text = Some(c.search.trim().to_owned()).filter(|t| !t.is_empty());
        query.project_type = c.kind;
        query.sort = Some(c.sort);
        query.limit = Some(PAGE);
        query.after = if more { c.next.clone() } else { None };
        if !more {
            c.projects.clear();
            c.total = 0;
            c.next = None;
        }
        c.error = None;
        let (task, handle) = Task::perform(client.catalog(&query), move |result| {
            crate::cloud::msg(Event::CatalogPage { id, more, result })
        })
        .abortable();
        c.loading = Some((id, handle.abort_on_drop()));
        task
    }

    /// The projects this device keeps for the account (the signed-in one, or the last).
    fn kept_projects(&self) -> Vec<Kept> {
        match (&self.cloud.replicas, self.cloud_identity()) {
            (Some(store), Some((server, user))) => store.list(&server, &user),
            _ => Vec::new(),
        }
    }

    /// The selected project's counts, once the selection rested (the web's `loadDetails`).
    pub(crate) fn catalog_details_tick(&mut self, now: Instant) -> Task<Message> {
        let id = self.cloud.next_id();
        let client = self.cloud.signed_in().cloned();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        if c.details_at.is_none_or(|at| now < at) {
            return Task::none();
        }
        c.details_at = None;
        let (Some(client), Some(p)) = (client, c.picked().cloned()) else {
            c.details = Details::None;
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (uuid(&p.tenant_id), uuid(&p.id)) else {
            c.details = Details::Failed("Sunucunun verdiği kimlik okunamadı.".to_owned());
            return Task::none();
        };
        let (task, handle) = if p.storage == ProjectStorage::File {
            // A file project's content is its newest revision (the server counts no rows of it).
            let asked = client.file_revisions(tenant, project);
            Task::perform(
                async move {
                    asked.await.map(|r| {
                        let objects = r
                            .revisions
                            .iter()
                            .find(|x| Some(&x.revision) == r.current.as_ref())
                            .and_then(|x| x.objects.clone());
                        Details::File {
                            project: p.id,
                            revision: r.current,
                            objects,
                        }
                    })
                },
                move |result| crate::cloud::msg(Event::CatalogDetails { id, result }),
            )
            .abortable()
        } else {
            let asked = client.details(tenant, project);
            Task::perform(
                async move { asked.await.map(|d| Details::Database(Box::new(d))) },
                move |result| crate::cloud::msg(Event::CatalogDetails { id, result }),
            )
            .abortable()
        };
        c.details_asked = Some((id, handle.abort_on_drop()));
        task
    }

    pub(crate) fn catalog_event(&mut self, event: Event) -> Task<Message> {
        let opening = self.cloud.opening.is_some();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        // While a project opens, or an action or a question holds the window, the list stays as it is.
        let held = opening || c.busy();
        match event {
            Event::CatalogView(_)
            | Event::CatalogSearch(_)
            | Event::CatalogPick(_)
            | Event::CatalogKind(_)
            | Event::CatalogSort(_)
            | Event::CatalogOrg(_)
            | Event::CatalogStep(_)
                if held => {}
            Event::CatalogView(list) => {
                if list != List::Device
                    && let Ok(mut last) = LAST.lock()
                {
                    *last = Some(list.clone());
                }
                if let List::View(v) = &list {
                    c.sort = words::view(*v).sorts[0];
                }
                c.list = list;
                c.select(None);
                c.status = None;
                let load = self.catalog_load(false);
                return Task::batch([load, self.history_follow()]);
            }
            Event::CatalogSearch(text) => {
                c.search = text;
                c.search_at = Some(Instant::now() + SEARCH_DELAY);
            }
            Event::CatalogKind(kind) => {
                c.kind = kind;
                return self.catalog_load(false);
            }
            Event::CatalogSort(sort) => {
                c.sort = sort;
                return self.catalog_load(false);
            }
            Event::CatalogOrg(org) => {
                c.org = Some(org);
                return self.catalog_load(false);
            }
            Event::CatalogPick(id) => {
                c.select(Some(id));
                return self.history_follow();
            }
            // ↑ ↓ Home End in the list (the web's list keys).
            Event::CatalogStep(step) => {
                let ids: Vec<String> = match c.list {
                    List::Device => c.device.iter().map(|k| k.info.id.clone()).collect(),
                    List::View(_) => c.projects.iter().map(|p| p.id.clone()).collect(),
                };
                if ids.is_empty() {
                    return Task::none();
                }
                let at = c
                    .picked
                    .as_ref()
                    .and_then(|id| ids.iter().position(|x| x == id));
                let last = ids.len() - 1;
                let next = match (step, at) {
                    (Step::Home, _) | (_, None) => 0,
                    (Step::End, _) => last,
                    (Step::Down, Some(i)) => (i + 1).min(last),
                    (Step::Up, Some(i)) => i.saturating_sub(1),
                };
                c.select(Some(ids[next].clone()));
                return self.history_follow();
            }
            Event::CatalogTab(tab) => {
                c.tab = tab;
                return self.history_follow();
            }
            Event::CatalogHover(id) => c.hovered = id,
            Event::CatalogMore => {
                if c.next.is_some() && c.loading.is_none() {
                    return self.catalog_load(true);
                }
            }
            Event::CatalogRetry => return self.catalog_load(false),
            Event::CatalogDetails { id, result } => {
                if c.details_asked.as_ref().is_none_or(|(d, _)| *d != id) {
                    return Task::none();
                }
                c.details_asked = None;
                c.details = match result {
                    Ok(d) => d,
                    Err(failure) => Details::Failed(failure.message.clone()),
                };
            }
            Event::CatalogOpen => {
                if opening || c.busy() {
                    return Task::none();
                }
                // In the trash the main button restores (catalog_actions.rs).
                if c.list == List::View(CatalogView::Trash) {
                    return self.catalog_act(Act::Restore);
                }
                let picked = match &c.list {
                    List::Device => c.picked_kept().map(|k| {
                        (
                            k.info.tenant_id.clone(),
                            k.info.id.clone(),
                            k.info.name.clone(),
                            k.info.storage,
                        )
                    }),
                    List::View(_) => c
                        .picked()
                        .map(|p| (p.tenant_id.clone(), p.id.clone(), p.name.clone(), p.storage)),
                };
                let Some((tenant, project, name, storage)) = picked else {
                    c.status = Some(Said::info("Önce listeden bir proje seçin."));
                    return Task::none();
                };
                let (Some(tenant), Some(project)) = (uuid(&tenant), uuid(&project)) else {
                    c.status = Some(Said::error(format!(
                        "“{name}”: sunucunun verdiği kimlik okunamadı."
                    )));
                    return Task::none();
                };
                self.cloud.open_hint = Some((name, storage));
                return self.leave(Then::OpenCloud { tenant, project });
            }
            Event::CatalogRemove => {
                let Some(k) = c.picked_kept() else {
                    c.status = Some(Said::info("Önce listeden bu cihazdaki bir projeyi seçin."));
                    return Task::none();
                };
                let (Some(tenant), Some(project)) = (uuid(&k.info.tenant_id), uuid(&k.info.id))
                else {
                    return Task::none();
                };
                let name = k.info.name.clone();
                // Unsent work in its draft is asked about first; without it the copy just goes.
                let unsent = self.draft_work(tenant, project);
                if unsent > 0 {
                    self.cloud.removing = Some(Removing {
                        tenant,
                        project,
                        name,
                        unsent,
                    });
                    self.dialog = Some(Dialog::RemoveCopy);
                    return Task::none();
                }
                self.remove_copy(tenant, project, &name, false);
                return self.catalog_load(false);
            }
            Event::RemoveConfirmed => {
                if let Some(r) = self.cloud.removing.take() {
                    self.dialog = Some(Dialog::Catalog);
                    self.remove_copy(r.tenant, r.project, &r.name, true);
                    return self.catalog_load(false);
                }
            }
            Event::CatalogPage { id, more, result } => {
                if c.loading.as_ref().is_none_or(|(l, _)| *l != id) {
                    return Task::none();
                }
                c.loading = None;
                // No answer: this device's copies are what can be opened now.
                if let Err(failure) = &result
                    && failure.status == 0
                    && failure.transient()
                {
                    c.list = List::Device;
                    c.status = Some(Said::error(format!(
                        "{} Bu cihazdaki projeler gösteriliyor; bağlantısız açılırlar.",
                        failure.message
                    )));
                    self.went_offline();
                    return self.catalog_load(false);
                }
                page(c, more, result);
                let made = self.catalog_listed();
                return Task::batch([made, self.history_follow(), self.came_online()]);
            }
            _ => {}
        }
        Task::none()
    }
}

impl App {
    /// After a page: a project made here (a restored point) is opened as
    /// soon as the list shows it; a later list opens nothing by itself (the
    /// web's `paintList`).
    fn catalog_listed(&mut self) -> Task<Message> {
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        let Some((id, open)) = c.wanted.take() else {
            return Task::none();
        };
        if !open {
            return Task::none();
        }
        if c.picked.as_deref() == Some(id.as_str()) {
            return self.catalog_event(Event::CatalogOpen);
        }
        c.status = Some(Said::info(
            "Yeni proje oluşturuldu ama bu listede görünmüyor; “Projelerim”de arayıp açın.",
        ));
        Task::none()
    }
}

/// A copy to remove from this device whose draft still holds unsent work.
#[derive(Debug, Clone, PartialEq)]
pub struct Removing {
    pub tenant: kentos_domain::Uuid,
    pub project: kentos_domain::Uuid,
    pub name: String,
    pub unsent: usize,
}

impl App {
    /// How much unsent work a project's device draft holds (0 without one).
    fn draft_work(&self, tenant: kentos_domain::Uuid, project: kentos_domain::Uuid) -> usize {
        let (Some(store), Some((server, user))) = (&self.cloud.drafts, self.cloud_identity())
        else {
            return 0;
        };
        match store.load(&store.key(&server, &user, tenant, project)) {
            Ok(kentos_cloud::Loaded::Found(d)) => {
                d.changes.len() + usize::from(d.meta.is_some()) + usize::from(d.inflight.is_some())
            }
            _ => 0,
        }
    }

    /// “Bu cihazdan kaldır”: the copy goes (refused while a window holds it);
    /// its draft too when `with_draft` (the user said so).
    fn remove_copy(
        &mut self,
        tenant: kentos_domain::Uuid,
        project: kentos_domain::Uuid,
        name: &str,
        with_draft: bool,
    ) {
        let (Some(store), Some((server, user))) =
            (self.cloud.replicas.clone(), self.cloud_identity())
        else {
            return;
        };
        let text = match store.remove(&server, &user, tenant, project) {
            Ok(()) => {
                if with_draft && let Some(drafts) = &self.cloud.drafts {
                    let _ = drafts.remove(&drafts.key(&server, &user, tenant, project));
                }
                Said::info(format!(
                    "“{name}” bu cihazdan kaldırıldı; proje sunucuda olduğu gibi duruyor."
                ))
            }
            Err(e) => Said::error(e.to_string()),
        };
        match self.cloud.catalog.as_mut() {
            Some(c) => c.status = Some(text),
            None => self.output(text.text),
        }
    }
}

/// A page into the list: the first replaces it, a next one is added below.
/// The selection stays while its project is listed.
fn page(c: &mut Catalog, more: bool, result: Result<ProjectPage, ApiFailure>) {
    match result {
        Ok(page) => {
            if more {
                c.projects.extend(page.projects);
            } else {
                c.projects = page.projects;
            }
            c.total = page.total;
            c.next = page.next;
            c.retention = page.trash_retention_days;
            let listed = |id: &String| c.projects.iter().any(|p| &p.id == id);
            let keep = c
                .wanted
                .as_ref()
                .map(|(id, _)| id.clone())
                .filter(listed)
                .or_else(|| c.picked.clone().filter(listed));
            c.select(keep);
        }
        Err(failure) => {
            c.error = Some(failure.message.clone());
            c.select(None);
        }
    }
}

/// The row's `place`: where a project is, as the account names it (the web's `placeOf`).
pub(crate) fn place_of(app: &App, p: &ProjectSummary) -> String {
    let own = app.cloud.membership(&p.tenant_id).is_some();
    words::workspace(p.tenant_kind, &p.tenant_name, own)
}

/// What the list says with no rows.
pub(crate) fn empty_text(c: &Catalog, orgs: usize) -> String {
    if c.loading() {
        return "Projeler yükleniyor…".to_owned();
    }
    let filtered = !c.search.trim().is_empty() || c.kind.is_some();
    match &c.list {
        List::Device => {
            if filtered {
                plan::EMPTY_SEARCH.to_owned()
            } else {
                "Bu bilgisayarda kopyası tutulan bir proje yok. Bir bulut projesini bir kez açınca kopyası burada tutulur.".to_owned()
            }
        }
        _ if filtered => plan::EMPTY_SEARCH.to_owned(),
        List::View(CatalogView::Organization) if orgs == 0 => plan::NO_ORGANIZATION.to_owned(),
        List::View(v) => words::view(*v).empty.to_owned(),
    }
}

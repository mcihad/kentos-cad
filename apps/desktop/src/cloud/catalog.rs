//! “Bulut projesi aç” (docs/adr/0041; the web's CatalogDialog.ts, docs/adr/0028):
//! the account's lists — recently opened, favourites, its own, each
//! organisation's, shared with it, archived — searched and paged by the
//! server after its access check, and “Bu cihazdaki projeler”: the projects
//! this device keeps a copy of, which open without a connection
//! (docs/adr/0043). Without a session, or when the server does not answer,
//! the window shows that list. A row says the project's name, where it is
//! and whose, when it last changed, how it is kept and the account's role.
//! Opening asks about the drawing on screen first (leaving.rs), then shows
//! its progress here and can be stopped (opening.rs).

use std::time::{Duration, Instant};

use iced::Task;
use iced::task::Handle;
use kentos_cloud::{ApiFailure, CatalogQuery, Kept};
use kentos_contracts::{CatalogView, ProjectPage, ProjectSummary, TenantKind};

use crate::app::{App, Dialog, Message, Then};
use crate::cloud::{Event, uuid, words};

/// Projects asked for at once (the web's page).
pub const PAGE: u32 = 50;
/// How long typing rests before the server is asked (the web's).
pub const SEARCH_DELAY: Duration = Duration::from_millis(250);

/// A list of the catalog: a view, one organisation's projects, or this device's copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum List {
    View(CatalogView),
    /// “Kurum projeleri” of one organisation (its tenant id).
    Organization(String),
    /// “Bu cihazdaki projeler”: the copies kept on this device.
    Device,
}

/// The catalog window.
pub struct Catalog {
    pub list: List,
    pub search: String,
    /// When the typed search goes to the server.
    pub search_at: Option<Instant>,
    pub projects: Vec<ProjectSummary>,
    /// The copies on this device (“Bu cihazdaki projeler”).
    pub device: Vec<Kept>,
    pub total: u32,
    pub next: Option<String>,
    /// The page on its way: its id (an older answer is dropped) and request.
    loading: Option<(u64, Handle)>,
    /// Why the list could not be read.
    pub error: Option<String>,
    /// The selected project's id.
    pub picked: Option<String>,
    /// What the last open said (its failure), or why the list changed.
    pub status: Option<String>,
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

    pub fn picked(&self) -> Option<&ProjectSummary> {
        let id = self.picked.as_deref()?;
        self.projects.iter().find(|p| p.id == id)
    }

    pub fn picked_kept(&self) -> Option<&Kept> {
        let id = self.picked.as_deref()?;
        self.device.iter().find(|k| k.info.id == id)
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
        self.cloud.catalog = Some(Catalog {
            list,
            search: String::new(),
            search_at: None,
            projects: Vec::new(),
            device: Vec::new(),
            total: 0,
            next: None,
            loading: None,
            error: None,
            picked: None,
            status: None,
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
        let (view, tenant) = match &c.list {
            List::View(view) => (*view, None),
            List::Organization(tenant) => (CatalogView::Organization, uuid(tenant)),
            List::Device => return Task::none(),
        };
        if view == CatalogView::Organization && tenant.is_none() {
            // No organisation to list: the window says so.
            c.loading = None;
            c.projects.clear();
            c.total = 0;
            c.next = None;
            return Task::none();
        }
        let mut query = CatalogQuery::new(view);
        query.tenant = tenant;
        query.text = Some(c.search.trim().to_owned()).filter(|t| !t.is_empty());
        query.sort = Some(words::view(view).sort);
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

    pub(crate) fn catalog_event(&mut self, event: Event) -> Task<Message> {
        let opening = self.cloud.opening.is_some();
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        match event {
            // While a project opens, the list stays as it is.
            Event::CatalogView(_) | Event::CatalogSearch(_) | Event::CatalogPick(_) if opening => {}
            Event::CatalogView(list) => {
                if list != List::Device
                    && let Ok(mut last) = LAST.lock()
                {
                    *last = Some(list.clone());
                }
                c.list = list;
                c.picked = None;
                c.status = None;
                return self.catalog_load(false);
            }
            Event::CatalogSearch(text) => {
                c.search = text;
                c.search_at = Some(Instant::now() + SEARCH_DELAY);
            }
            Event::CatalogPick(id) => {
                c.picked = Some(id);
                c.status = None;
            }
            Event::CatalogMore => {
                if c.next.is_some() && c.loading.is_none() {
                    return self.catalog_load(true);
                }
            }
            Event::CatalogRetry => return self.catalog_load(false),
            Event::CatalogOpen => {
                if opening {
                    return Task::none();
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
                    _ => c
                        .picked()
                        .map(|p| (p.tenant_id.clone(), p.id.clone(), p.name.clone(), p.storage)),
                };
                let Some((tenant, project, name, storage)) = picked else {
                    c.status = Some("Önce listeden bir proje seçin.".to_owned());
                    return Task::none();
                };
                let (Some(tenant), Some(project)) = (uuid(&tenant), uuid(&project)) else {
                    c.status = Some(format!("“{name}”: sunucunun verdiği kimlik okunamadı."));
                    return Task::none();
                };
                self.cloud.open_hint = Some((name, storage));
                return self.leave(Then::OpenCloud { tenant, project });
            }
            Event::CatalogRemove => {
                let Some(k) = c.picked_kept() else {
                    c.status = Some("Önce listeden bu cihazdaki bir projeyi seçin.".to_owned());
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
                    c.status = Some(format!(
                        "{} Bu cihazdaki projeler gösteriliyor; bağlantısız açılırlar.",
                        failure.message
                    ));
                    self.went_offline();
                    return self.catalog_load(false);
                }
                page(c, more, result);
                return self.came_online();
            }
            _ => {}
        }
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
                format!("“{name}” bu cihazdan kaldırıldı; proje sunucuda olduğu gibi duruyor.")
            }
            Err(e) => e.to_string(),
        };
        match self.cloud.catalog.as_mut() {
            Some(c) => c.status = Some(text),
            None => self.output(text),
        }
    }
}

/// A page into the list: the first replaces it, a next one is added below.
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
            if c.picked
                .as_ref()
                .is_some_and(|id| !c.projects.iter().any(|p| &p.id == id))
            {
                c.picked = None;
            }
        }
        Err(failure) => c.error = Some(failure.message.clone()),
    }
}

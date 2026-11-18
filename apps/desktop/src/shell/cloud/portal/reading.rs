//! Portal dosyaları' reads (docs/adr/9001 §3): the places an account works in on the portal, the list's filters, the
//! account and the catalog asked together, a place's list with its filters (the typed search once the keys stop),
//! and the chosen dossier's page. Every read carries an id; an answer with another id is a late one and is dropped.

use std::time::{Duration, Instant};

use iced::Task;
use iced::widget::text_editor;
use kentos_cloud::{ApiFailure, DossierQuery};
use kentos_commands::{
    DossierDirection, DossierList, OrganizationCategory, PortalCatalog, PortalMe,
};

use super::{Event, LIMIT, msg};
use crate::app::{App, Message};

/// The search goes to the server this long after the last key.
pub(super) const SEARCH_WAIT: Duration = Duration::from_millis(300);

/// Where the dossiers are listed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// The professional's own work: the applications they sent and the transfers sent to them.
    Own,
    /// A municipality's dossiers, as its member sees them.
    Organization { tenant: String, name: String },
}

impl Place {
    /// The place's name in the window's chooser.
    pub fn label(&self) -> String {
        match self {
            Place::Own => "Meslek mensubu olarak".to_owned(),
            Place::Organization { name, .. } => name.clone(),
        }
    }
}

/// The places an account works in on the portal: its own work as a professional, then each municipality it may use.
pub fn places(me: &PortalMe) -> Vec<Place> {
    let mut out = Vec::new();
    if me.professional.is_some() {
        out.push(Place::Own);
    }
    out.extend(
        me.organizations
            .iter()
            .filter(|o| o.usable && o.category == OrganizationCategory::Municipality)
            .map(|o| Place::Organization {
                tenant: o.tenant_id.clone(),
                name: o.tenant_name.clone(),
            }),
    );
    out
}

/// The list's direction filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    All,
    Applications,
    Transfers,
}

impl Direction {
    pub const ALL: [Direction; 3] = [
        Direction::All,
        Direction::Applications,
        Direction::Transfers,
    ];

    fn asked(self) -> Option<DossierDirection> {
        match self {
            Direction::All => None,
            Direction::Applications => Some(DossierDirection::Application),
            Direction::Transfers => Some(DossierDirection::Transfer),
        }
    }

    /// Its name for the place's side: a professional's “Başvurularım”, a municipality's “Gelen başvurular”.
    pub fn label(self, place: &Place) -> &'static str {
        match (self, place) {
            (Direction::All, _) => "Tümü",
            (Direction::Applications, Place::Own) => "Başvurularım",
            (Direction::Applications, Place::Organization { .. }) => "Gelen başvurular",
            (Direction::Transfers, Place::Own) => "Aktarılan işler",
            (Direction::Transfers, Place::Organization { .. }) => "Proje aktarımları",
        }
    }
}

impl App {
    /// The typed search goes once the keys stop.
    pub(crate) fn portal_tick(&mut self, now: Instant) -> Task<Message> {
        let due = self
            .cloud
            .portal
            .as_mut()
            .is_some_and(|p| p.search_at.take_if(|at| now >= *at).is_some());
        if due {
            self.portal_list()
        } else {
            Task::none()
        }
    }

    /// The account on the portal and the catalog's names, asked together.
    pub(super) fn portal_load(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let Some(client) = self.cloud.signed_in().cloned() else {
            if let Some(p) = self.cloud.portal.as_mut() {
                p.failed = Some(
                    "Bulut oturumu açık değil; portal dosyaları için önce Buluta giriş ile giriş yapın.".to_owned(),
                );
            }
            return Task::none();
        };
        let Some(p) = self.cloud.portal.as_mut() else {
            return Task::none();
        };
        p.requests.me = id;
        p.failed = None;
        let asked = async move {
            let me = client.portal_me().await?;
            let catalog = client.portal_catalog().await?;
            Ok(Box::new((me, catalog)))
        };
        Task::perform(asked, move |result| msg(Event::Loaded { id, result }))
    }

    pub(super) fn portal_loaded(
        &mut self,
        id: u64,
        result: Result<Box<(PortalMe, PortalCatalog)>, ApiFailure>,
    ) -> Task<Message> {
        let Some(p) = self.cloud.portal.as_mut().filter(|p| p.requests.me == id) else {
            return Task::none();
        };
        match result {
            Ok(both) => {
                let (me, catalog) = *both;
                let all = places(&me);
                // The place it was left in, while the account still has it; else the first.
                if p.place.as_ref().is_none_or(|place| !all.contains(place)) {
                    p.place = all.first().cloned();
                }
                p.me = Some(me);
                p.catalog = Some(catalog);
                if p.place.is_none() {
                    return Task::none();
                }
                self.portal_list()
            }
            Err(f) => {
                p.failed = Some(format!(
                    "Portal bilgileri okunamadı: {} Bağlantıyı denetleyip Yenile'ye basın.",
                    f.message
                ));
                Task::none()
            }
        }
    }

    /// The place's list with the filters, asked again.
    pub(super) fn portal_list(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let Some(p) = self.cloud.portal.as_mut() else {
            return Task::none();
        };
        let Some(place) = p.place.clone() else {
            return Task::none();
        };
        p.requests.list = id;
        p.search_at = None;
        let query = DossierQuery {
            direction: p.direction.asked(),
            text: Some(p.search.clone()),
            limit: Some(LIMIT),
            ..DossierQuery::default()
        };
        match place {
            Place::Own => Task::perform(client.my_dossiers(&query), move |result| {
                msg(Event::Listed { id, result })
            }),
            Place::Organization { tenant, .. } => {
                let Some(tenant) = crate::cloud::uuid(&tenant) else {
                    return Task::none();
                };
                Task::perform(
                    client.organization_dossiers(tenant, &query),
                    move |result| msg(Event::Listed { id, result }),
                )
            }
        }
    }

    pub(super) fn portal_listed(
        &mut self,
        id: u64,
        result: Result<DossierList, ApiFailure>,
    ) -> Task<Message> {
        let Some(p) = self.cloud.portal.as_mut().filter(|p| p.requests.list == id) else {
            return Task::none();
        };
        let list = match result {
            Ok(list) => list,
            Err(f) => {
                p.failed = Some(format!(
                    "Dosyalar okunamadı: {} Bağlantıyı denetleyip Yenile'ye basın.",
                    f.message
                ));
                return Task::none();
            }
        };
        p.failed = None;
        // The chosen one stays while it is listed; else the first one waiting for this side, else none.
        let listed = |id: &str| list.items.iter().any(|d| d.id == id);
        let keep = p.chosen.as_deref().is_some_and(listed);
        if !keep {
            p.chosen = list
                .items
                .iter()
                .find(|d| d.awaiting_me)
                .map(|d| d.id.clone());
            p.draft = text_editor::Content::new();
        }
        let ask = p.chosen.is_some() && (!keep || p.chosen_detail().is_none());
        p.list = Some(list);
        if ask {
            self.portal_detail()
        } else {
            Task::none()
        }
    }

    /// The chosen dossier's page, asked again.
    pub(super) fn portal_detail(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let Some(p) = self.cloud.portal.as_mut() else {
            return Task::none();
        };
        let Some(dossier) = p.chosen.as_deref().and_then(crate::cloud::uuid) else {
            return Task::none();
        };
        p.requests.detail = id;
        Task::perform(client.dossier(dossier), move |result| {
            msg(Event::Detail {
                id,
                result: result.map(Box::new),
            })
        })
    }
}

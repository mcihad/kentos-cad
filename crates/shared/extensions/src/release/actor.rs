//! Who moves a release, and in which catalog (plan/eklenti.md ADIM 22A; PY-CLOUD-02, 07, 09, 18): permissions are data,
//! roles name the usual sets of them, and an [`Actor`] is what the server knows of the account behind a call.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// What an account may do in a catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    /// Upload a version under one's publisher; renew its content while it is a draft or sent back.
    Upload,
    /// Send one's publisher's version for review.
    Submit,
    /// Report a gate's outcome: the catalog's scanners, in their isolated workers (PY-CLOUD-05, 06).
    Scan,
    /// Take a version into review, ask for changes, lift a blocking finding with a reason.
    Review,
    /// Approve a version in review (PY-CLOUD-07).
    Approve,
    /// Publish one's publisher's approved version, withdraw or archive it; not approving it (PY-CLOUD-09).
    Publish,
    /// Suspend, reinstate, withdraw and archive any version (PY-CLOUD-15).
    Moderate,
}

impl Permission {
    pub const ALL: [Permission; 7] = [
        Permission::Upload,
        Permission::Submit,
        Permission::Scan,
        Permission::Review,
        Permission::Approve,
        Permission::Publish,
        Permission::Moderate,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Permission::Upload => "Sürüm yükleme",
            Permission::Submit => "İncelemeye gönderme",
            Permission::Scan => "Denetim sonucu bildirme",
            Permission::Review => "İnceleme",
            Permission::Approve => "Onaylama",
            Permission::Publish => "Yayımlama",
            Permission::Moderate => "Katalog yönetimi",
        }
    }
}

/// The catalog's roles: each the usual set of permissions for a part. The server may give an account several roles, or
/// sets of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    /// An extension's author: a member of its publisher.
    Publisher,
    Reviewer,
    CatalogAdmin,
    /// The catalog's automatic checks.
    Scanner,
}

impl Role {
    pub const ALL: [Role; 4] = [
        Role::Publisher,
        Role::Reviewer,
        Role::CatalogAdmin,
        Role::Scanner,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Role::Publisher => "Yayımcı",
            Role::Reviewer => "İnceleyici",
            Role::CatalogAdmin => "Katalog yöneticisi",
            Role::Scanner => "Denetim hizmeti",
        }
    }

    /// Its permissions. Approving and publishing are never in the same role (PY-CLOUD-09).
    pub fn permissions(self) -> &'static [Permission] {
        match self {
            Role::Publisher => &[Permission::Upload, Permission::Submit, Permission::Publish],
            Role::Reviewer => &[Permission::Review, Permission::Approve],
            Role::CatalogAdmin => &[Permission::Moderate],
            Role::Scanner => &[Permission::Scan],
        }
    }

    /// The roles a permission is in, named for a refusal ("İnceleyici").
    pub fn holding(permission: Permission) -> String {
        Role::ALL
            .into_iter()
            .filter(|r| r.permissions().contains(&permission))
            .map(Role::title)
            .collect::<Vec<_>>()
            .join(" ya da ")
    }
}

/// How a call came: a person in the catalog's window, or a program with an account's token (Python, MCP, an agent, a
/// build server). The server says which from how the call was authenticated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    Window,
    Program,
}

/// What the server knows of the account behind a call. Built by the server from how the call was authenticated, never
/// read from a request's body: for that it has no `Deserialize`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    /// The account's stable id.
    pub id: String,
    pub permissions: BTreeSet<Permission>,
    /// The publishers it is a member of, by id.
    pub publishers: BTreeSet<String>,
    pub via: Channel,
}

impl Actor {
    /// An account with these roles' permissions, a member of no publisher.
    pub fn new(id: &str, roles: &[Role], via: Channel) -> Actor {
        Actor {
            id: id.to_owned(),
            permissions: roles
                .iter()
                .flat_map(|r| r.permissions().iter().copied())
                .collect(),
            publishers: BTreeSet::new(),
            via,
        }
    }

    /// The same account, a member of this publisher too.
    pub fn member_of(mut self, publisher: &str) -> Actor {
        self.publishers.insert(publisher.to_owned());
        self
    }

    pub fn can(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }
}

/// Which catalog a release is in (PY-CLOUD-02). Fixed at upload: a release reviewed in one's own catalog is never
/// moved to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogScope {
    /// One's own: seen by its owner only, who may review their own release there (nobody else installs it).
    Private,
    /// An organisation's: its members see it once published; the author never reviews their own.
    Tenant,
    /// The public catalog: everyone sees it once published; neither its author nor its publisher's members review it.
    Public,
}

impl CatalogScope {
    pub fn title(self) -> &'static str {
        match self {
            CatalogScope::Private => "Kişisel katalog",
            CatalogScope::Tenant => "Kurum kataloğu",
            CatalogScope::Public => "Genel katalog",
        }
    }
}

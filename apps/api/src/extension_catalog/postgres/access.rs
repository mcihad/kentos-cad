//! What the persistent store says an account may do in a catalog (docs/adr/8059 §10, §13): the administrator's grants
//! in that catalog and in every one, the publishers it is a member of, an organisation's membership; and whether a
//! catalog trusts a publisher.

use std::collections::BTreeSet;

use kentos_extensions::release::Role;
use kentos_postgres::Scope;

use super::{PgStore, account_id, failed, tenant_id};
use crate::extension_catalog::CatalogId;
use crate::extension_catalog::store::{Access, StoreError};

/// A role as the grants table names it: the flow's own word.
pub(crate) fn role_id(role: Role) -> &'static str {
    match role {
        Role::Publisher => "publisher",
        Role::Reviewer => "reviewer",
        Role::CatalogAdmin => "catalogAdmin",
        Role::Scanner => "scanner",
    }
}

pub(crate) fn role_of(text: &str) -> Option<Role> {
    Role::ALL.into_iter().find(|r| role_id(*r) == text)
}

impl PgStore {
    /// What `account` may do in `catalog`: an account id that is not one is nobody (the public catalog's reader); an
    /// organisation that does not exist has no catalog for anyone, its services' included.
    pub(super) async fn access_of(
        &self,
        catalog: &CatalogId,
        account: &str,
    ) -> Result<Access, StoreError> {
        let Some(user) = account_id(account) else {
            return Ok(Access {
                member: *catalog == CatalogId::Public,
                ..Access::default()
            });
        };
        let mut tx = self
            .db
            .scoped(Scope {
                user: Some(user),
                ..Scope::default()
            })
            .await
            .map_err(failed)?;
        let (kind, reference, tenant) = match catalog {
            CatalogId::Public => ("public", String::new(), None),
            CatalogId::Tenant(slug) => match tenant_id(&mut tx, slug).await? {
                Some(id) => ("tenant", id.to_string(), Some(id)),
                // No such organisation: no such catalog, for anyone.
                None => return Ok(Access::default()),
            },
            CatalogId::Personal(_) => ("personal", String::new(), None),
        };
        let granted: Vec<(String, String)> = sqlx::query_as(
            "select catalog_kind, role from kentos.extension_catalog_grant
              where account = $1 and (catalog_kind = 'every' or (catalog_kind = $2 and catalog_ref = $3))",
        )
        .bind(user)
        .bind(kind)
        .bind(&reference)
        .fetch_all(&mut *tx)
        .await
        .map_err(failed)?;
        let publishers: Vec<String> = sqlx::query_scalar(
            "select publisher_id from kentos.extension_publisher_member where account = $1",
        )
        .bind(user)
        .fetch_all(&mut *tx)
        .await
        .map_err(failed)?;
        let member_of_tenant = match tenant {
            Some(id) => sqlx::query_scalar(
                "select exists (select 1 from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
                                 where m.user_id = $1 and t.id = $2 and m.status = 'active' and t.status = 'active')",
            )
            .bind(user)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(failed)?,
            None => false,
        };
        tx.commit().await.map_err(failed)?;
        let service = granted.iter().any(|(kind, _)| kind == "every");
        let roles: BTreeSet<Role> = granted.iter().filter_map(|(_, r)| role_of(r)).collect();
        let member = match catalog {
            CatalogId::Public => true,
            CatalogId::Tenant(_) => service || !roles.is_empty() || member_of_tenant,
            CatalogId::Personal(_) => service,
        };
        Ok(Access {
            member,
            roles,
            publishers: publishers.into_iter().collect(),
        })
    }

    /// Whether the administrator let `catalog` trust `publisher`; one's own catalog trusts nobody.
    pub(super) async fn trusts(
        &self,
        catalog: &CatalogId,
        publisher: &str,
    ) -> Result<bool, StoreError> {
        let mut tx = self.db.pool.begin().await.map_err(failed)?;
        let (kind, reference) = match catalog {
            CatalogId::Public => ("public", String::new()),
            CatalogId::Tenant(slug) => match tenant_id(&mut tx, slug).await? {
                Some(id) => ("tenant", id.to_string()),
                None => return Ok(false),
            },
            CatalogId::Personal(_) => return Ok(false),
        };
        let trusted = sqlx::query_scalar(
            "select exists (select 1 from kentos.extension_catalog_trust
                             where catalog_kind = $1 and catalog_ref = $2 and publisher_id = $3)",
        )
        .bind(kind)
        .bind(&reference)
        .bind(publisher)
        .fetch_one(&mut *tx)
        .await
        .map_err(failed)?;
        tx.commit().await.map_err(failed)?;
        Ok(trusted)
    }
}

//! The catalog's revocation list (plan/eklenti.md ADIM 22A; docs/adr/8059 §12): the release flow makes it from every
//! release of the catalog, and every member gets the same one.

use kentos_extensions::release::revocation_list;
use kentos_extensions::revocations::Revocations;

use super::{Account, Catalog, CatalogError, CatalogId};

impl Catalog {
    /// A catalog's revocation list, for one of its members; in a personal catalog for its owner alone. It is made from
    /// every release, whoever may see which: only published versions taken out are in it, which every member sees
    /// anyway, with the reason anyone may read, so the list is the same for an insider and an outsider.
    pub async fn revocations(
        &self,
        catalog: &CatalogId,
        account: &Account,
    ) -> Result<Revocations, CatalogError> {
        if matches!(catalog, CatalogId::Personal(owner) if *owner != account.id) {
            return Err(CatalogError::NotFound);
        }
        self.actor(catalog, account).await?;
        Ok(revocation_list(&self.store.releases(catalog).await?))
    }
}

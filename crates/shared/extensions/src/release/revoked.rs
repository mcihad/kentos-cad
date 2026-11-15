//! The catalog's revocation list, made from its releases (plan/eklenti.md ADIM 22A; D15; docs/adr/8059 §12). Every
//! published release a moderator suspended or withdrew is an entry: its version as `=X.Y.Z`, with the reason its notice
//! gives. A release put back drops out with its state; an archived one keeps running where it is installed, so it is
//! not listed; one never published never ran anywhere and is never listed.
//!
//! An entry's reason is a clause without a closing full stop: the host says it inside its own sentence
//! (`1.0.0 sürümü geri çekildi: {reason}.`). A suspension adds, in its notice's own words, that the version may come
//! back. The list is ordered by extension id, then by version, so the same releases always make the same bytes.

use super::{Release, ReleaseState, UNDER_REVIEW};
use crate::revocations::{REVOCATIONS_FORMAT, Revocations, Revoked, SCHEMA_VERSION};
use crate::version::Version;

#[cfg(test)]
mod tests;

impl Release {
    /// This release's entry in its catalog's revocation list: none unless it was published and is now suspended or
    /// withdrawn.
    pub fn revoked(&self) -> Option<Revoked> {
        let suspended = match self.state {
            ReleaseState::Suspended => true,
            ReleaseState::Withdrawn => false,
            _ => return None,
        };
        if !self.was_published() {
            return None;
        }
        let version = Version::parse(&self.version)?;
        let given = self.notice_reason().unwrap_or("katalog bir neden yazmadı");
        let reason = if suspended {
            format!("{given} (askıya alındı; {UNDER_REVIEW})")
        } else {
            given.to_owned()
        };
        Some(Revoked {
            id: self.extension_id.clone(),
            versions: format!("={version}"),
            reason,
        })
    }
}

/// The revocation list of one catalog's releases (`kentos.extension-revocations` v1), in the same order every time.
pub fn revocation_list(releases: &[Release]) -> Revocations {
    let mut entries: Vec<(Version, Revoked)> = releases
        .iter()
        .filter_map(|r| Some((Version::parse(&r.version)?, r.revoked()?)))
        .collect();
    entries.sort_by(|a, b| {
        a.1.id
            .cmp(&b.1.id)
            .then_with(|| a.0.cmp(&b.0))
            .then_with(|| a.1.versions.cmp(&b.1.versions))
            .then_with(|| a.1.reason.cmp(&b.1.reason))
    });
    Revocations {
        format: REVOCATIONS_FORMAT.to_owned(),
        schema_version: SCHEMA_VERSION,
        entries: entries.into_iter().map(|(_, entry)| entry).collect(),
    }
}

//! A release read back from storage (plan/eklenti.md ADIM 22A; PY-CLOUD-16): through the same rules as its moves, so
//! a record that skipped a step (published without an approval, approved by its author, a gate open after approval, a
//! move missing) does not load.

use serde::Deserialize;

use super::{
    Action, Approval, AuditRecord, CatalogScope, ChangeRequest, Gate, GateKind, Listing, Notice,
    Refusal, Release, ReleaseContent, ReleaseState, ReviewFocus,
};

/// A release as stored, read through [`Release::inconsistency`].
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Stored {
    extension_id: String,
    version: String,
    scope: CatalogScope,
    publisher: String,
    author: String,
    content: ReleaseContent,
    #[serde(default)]
    listing: Option<Listing>,
    state: ReleaseState,
    gates: Vec<Gate>,
    #[serde(default)]
    focus: Option<ReviewFocus>,
    #[serde(default)]
    approval: Option<Approval>,
    #[serde(default)]
    change_request: Option<ChangeRequest>,
    #[serde(default)]
    notice: Option<Notice>,
    audit: Vec<AuditRecord>,
}

impl TryFrom<Stored> for Release {
    type Error = Refusal;

    fn try_from(s: Stored) -> Result<Release, Refusal> {
        let release = Release {
            extension_id: s.extension_id,
            version: s.version,
            scope: s.scope,
            publisher: s.publisher,
            author: s.author,
            content: s.content,
            listing: s.listing,
            state: s.state,
            gates: s.gates,
            focus: s.focus,
            approval: s.approval,
            change_request: s.change_request,
            notice: s.notice,
            audit: s.audit,
        };
        match release.inconsistency() {
            Some(why) => Err(Refusal::new(
                "record_inconsistent",
                format!(
                    "{} {} kaydı akıştan geçmiş bir sürümün kaydı olamaz: {why}.",
                    release.extension_id, release.version
                ),
            )),
            None => Ok(release),
        }
    }
}

impl Release {
    /// What in a stored record could not have come from the flow's moves; none when it could.
    fn inconsistency(&self) -> Option<&'static str> {
        use ReleaseState::*;
        if !self.gates.iter().map(|g| g.kind).eq(GateKind::ALL) {
            return Some("kapılar eksik ya da sırası bozuk");
        }
        let (Some(first), Some(last)) = (self.audit.first(), self.audit.last()) else {
            return Some("hareket kaydı boş");
        };
        if first.action != Action::Upload || first.from.is_some() {
            return Some("ilk hareket yükleme değil");
        }
        if self.audit.windows(2).any(|w| w[1].from != Some(w[0].to)) {
            return Some("hareketler birbirini izlemiyor");
        }
        if last.to != self.state {
            return Some("son hareket sürümün durumunu söylemiyor");
        }
        let approved = matches!(self.state, Approved | Published | Suspended | Withdrawn)
            || (self.state == Archived && self.was_published());
        match &self.approval {
            None if approved => return Some("onaysız sürüm onaylı durumda"),
            Some(_) if matches!(self.state, Draft | Submitted | InReview | ChangesRequested) => {
                return Some("onaylanmamış durumda onay kaydı var");
            }
            Some(a) if a.content != self.content => return Some("onay başka bir içeriğin"),
            Some(a) if self.scope != CatalogScope::Private && a.by == self.author => {
                return Some("yazar kendi sürümünü onaylamış");
            }
            Some(_) if !self.gates_verified() => {
                return Some("onaylı sürümde doğrulanmamış denetim var");
            }
            _ => {}
        }
        if self.state == ChangesRequested && self.change_request.is_none() {
            return Some("geri gönderilen sürümde gerekçe yok");
        }
        if self.state == InReview
            && self
                .focus
                .as_ref()
                .is_none_or(|f| f.content != self.content.package)
        {
            return Some("incelenen sürümün odağı yok");
        }
        let closed = matches!(self.state, Suspended | Withdrawn | Archived);
        match &self.notice {
            Some(n) if closed && n.state == self.state => {}
            None if !closed => {}
            _ => return Some("duyuru sürümün durumunu söylemiyor"),
        }
        None
    }
}

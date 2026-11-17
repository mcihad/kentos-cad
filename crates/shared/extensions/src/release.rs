//! The cloud catalog's release flow (plan/eklenti.md ADIM 22A; PY-CLOUD-01…18, D11–D13; docs/adr/8059): the one state
//! machine every version of an extension goes through, from its upload to its archive. The catalog's server, its window
//! and every program (Python, MCP, an agent) move a release with the same calls, so no call skips review (PY-CLOUD-18).
//!
//! - **States** ([`ReleaseState`]): draft → submitted → in review → changes requested / approved → published →
//!   suspended / withdrawn / archived. Each move is a method of [`Release`]; [`Action::sources`] is the table.
//! - **Upload is not publication:** a draft is seen by its author, its publisher and the catalog only.
//! - **A release is its content** ([`ReleaseContent`]: the package's digest, the lock's, the SBOM's and the capabilities
//!   asked for), fixed from submission (PY-CLOUD-04). An approval is of that content; other content is another review
//!   (PY-CLOUD-08), and a download is installed only when it is the approved content (PY-CLOUD-10).
//! - **Gates do not stay open** ([`gate`], D11): only a scan that ran to its end on this content and found nothing
//!   verifies a gate. A blocking gate sends the release back with its findings.
//! - **Who** ([`actor`]): permissions are data, roles name the usual sets of them. Approving and publishing are separate
//!   permissions (PY-CLOUD-09); approving, asking for changes, lifting a block and reinstating are a person's decisions
//!   in the catalog's window; outside one's personal catalog an author never reviews their own release (PY-CLOUD-07).
//! - **Records:** every move appends an [`AuditRecord`] (who, how, when, from, to, why; PY-CLOUD-16); a refused move
//!   changes nothing and says why in a [`Refusal`]. A stored release is checked when it is read, so a record that
//!   skipped a step does not load.
//! - [`facts`]: what a package says, which the release's identity is computed from; [`review`]: how deep a person
//!   reads a version, from its difference with the version approved before it (D12).
//! - [`revocation_list`]: the catalog's revocation list, made from its suspended and withdrawn releases (D15).
//!
//! Pure: no clock (every move is given its time), no storage, no network. The server keeps the records and resolves
//! who is calling; the scanners run the gates in their isolated workers.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::manifest::Capability;
use crate::version::Version;
use admit::need;

pub mod actor;
mod admit;
pub mod facts;
pub mod gate;
mod record;
pub mod review;
mod revoked;
mod state;
mod stored;

pub use actor::{Actor, CatalogScope, Channel, Permission, Role};
pub use facts::{LockedPackage, PythonFacts, ReleaseFacts, locked_packages, seconds_of};
pub use gate::{Gate, GateKind, GateStatus, ScanOutcome, Waiver};
pub use record::{
    Approval, AuditRecord, ChangeCode, ChangeRequest, Notice, Reason, RequestedChange,
};
pub use review::{Difference, DifferenceKind, ReviewDepth, ReviewFocus, classify};
pub use revoked::revocation_list;
pub use state::{Action, ReleaseState};

#[cfg(test)]
mod tests;

/// What may come of a suspension, as its notice and its revocation entry both say it.
const UNDER_REVIEW: &str = "inceleme bitince askıdan alınabilir ya da geri çekilir";

/// A moment, given by the caller: milliseconds since 1970-01-01 00:00 UTC. The flow reads no clock: the server's time
/// is the record's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub u64);

/// What a release is (PY-CLOUD-04): fixed from submission; an approval binds to all of it. A byte of the package, the
/// lock or a capability changed is other content, so another review (PY-CLOUD-08), and the publisher cannot put other
/// content in place of the approved one (PY-CLOUD-10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseContent {
    /// The package's content digest (`sha256:…`): every file's path, size and SHA-256.
    pub package: String,
    /// The Python lock's SHA-256, for a package with Python.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock: Option<String>,
    /// The SBOM's SHA-256, once the catalog made one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sbom: Option<String>,
    /// The capabilities it asks for.
    pub capabilities: BTreeSet<Capability>,
}

/// A move refused: a stable code (a test's, an agent's repair loop's, the API's error) and why, with what would make it
/// possible, in Turkish. The release is left as it was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
}

impl Refusal {
    pub fn new(code: &'static str, message: impl Into<String>) -> Refusal {
        Refusal {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for Refusal {}

/// What a person sees of a release in a catalog (docs/adr/8059 §14): its name and its icon's SHA-256, read from its
/// package when it is uploaded or its content renewed; what the publisher gate's imitation rule compares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Listing {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// A version of an extension in a catalog: its content, where it is, its gates, its review and every move it made.
/// Moved only by its methods; read back from storage through the same checks ([`Release::read`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "stored::Stored")]
pub struct Release {
    extension_id: String,
    version: String,
    scope: CatalogScope,
    /// The publisher it was uploaded under; whether it owns the id's namespace is the publisher gate's (D13).
    publisher: String,
    /// The account that uploaded it.
    author: String,
    content: ReleaseContent,
    /// Its name and icon as its content says them; none in a record kept before they were (docs/adr/8059 §14).
    #[serde(skip_serializing_if = "Option::is_none")]
    listing: Option<Listing>,
    state: ReleaseState,
    /// One per [`GateKind::ALL`], in that order.
    gates: Vec<Gate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<ReviewFocus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval: Option<Approval>,
    /// The last time it was sent back; kept after it goes again, so the reviewer sees what was asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    change_request: Option<ChangeRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notice: Option<Notice>,
    audit: Vec<AuditRecord>,
}

impl Release {
    /// Uploads a version under `publisher` into a catalog: a draft, not a publication. `existing` is the same catalog's
    /// releases of the extension: a version number, once uploaded, never carries other content (P14).
    pub fn upload(
        actor: &Actor,
        scope: CatalogScope,
        publisher: &str,
        facts: &ReleaseFacts,
        existing: &[Release],
        at: Timestamp,
    ) -> Result<Release, Refusal> {
        need(actor, Permission::Upload, Action::Upload)?;
        if !actor.publishers.contains(publisher) {
            return Err(Refusal::new(
                "not_owner",
                format!(
                    "“{publisher}” yayımcısının üyesi değilsiniz; onun adına sürüm yükleyemezsiniz."
                ),
            ));
        }
        publisher_claim(facts, publisher)?;
        let m = &facts.manifest;
        let Some(version) = Version::parse(&m.version) else {
            return Err(Refusal::new(
                "package_invalid",
                format!(
                    "Sürüm numarası “{}” okunamadı; manifestte anlamsal sürüm yazın (ör. 1.2.0).",
                    m.version
                ),
            ));
        };
        if let Some(taken) = existing.iter().find(|r| {
            r.extension_id == m.id && Version::parse(&r.version).as_ref() == Some(&version)
        }) {
            return Err(Refusal::new(
                "version_taken",
                format!(
                    "{} {} bu katalogda zaten var (“{}”). Bir sürüm numarası tek bir içerik taşır: içerik değiştiyse \
                     sürüm numarasını artırın; taslağın içeriğini değiştirmek için “İçeriği yenile”yi kullanın.",
                    m.id,
                    taken.version,
                    taken.state.title()
                ),
            ));
        }
        let mut release = Release {
            extension_id: m.id.clone(),
            version: m.version.clone(),
            scope,
            publisher: publisher.to_owned(),
            author: actor.id.clone(),
            content: facts.content(),
            listing: Some(facts.listing()),
            state: ReleaseState::Draft,
            gates: fresh_gates(),
            focus: None,
            approval: None,
            change_request: None,
            notice: None,
            audit: Vec::new(),
        };
        release.audit.push(AuditRecord {
            at,
            by: actor.id.clone(),
            via: actor.via,
            action: Action::Upload,
            from: None,
            to: ReleaseState::Draft,
            content: release.content.package.clone(),
            gate: None,
            reason: None,
        });
        Ok(release)
    }

    /// Puts new content in a draft or a release sent back: every gate starts again, any review focus goes.
    pub fn replace(
        &mut self,
        actor: &Actor,
        facts: &ReleaseFacts,
        at: Timestamp,
    ) -> Result<(), Refusal> {
        self.admit(actor, Action::Replace)?;
        let m = &facts.manifest;
        if m.id != self.extension_id || m.version != self.version {
            return Err(Refusal::new(
                "identity_mismatch",
                format!(
                    "Yüklenen paket {} {}; bu sürüm {} {}. Başka bir sürüm numarası yeni bir sürümdür.",
                    m.id, m.version, self.extension_id, self.version
                ),
            ));
        }
        publisher_claim(facts, &self.publisher)?;
        let content = facts.content();
        if content == self.content {
            return Err(Refusal::new(
                "content_unchanged",
                "Yüklenen içerik bu sürümdekiyle aynı; yenilenecek bir şey yok.",
            ));
        }
        let from = self.state;
        self.content = content;
        self.listing = Some(facts.listing());
        self.gates = fresh_gates();
        self.focus = None;
        self.log(actor, Action::Replace, from, at, None, None);
        Ok(())
    }

    /// Sends a draft, or a release sent back, for review. With a gate blocking, or the content the same as when changes
    /// were asked, the author's note says why it goes as it is.
    pub fn submit(
        &mut self,
        actor: &Actor,
        note: Option<&str>,
        at: Timestamp,
    ) -> Result<(), Refusal> {
        self.admit(actor, Action::Submit)?;
        let note = note.map(str::trim).filter(|n| !n.is_empty());
        let blocked: Vec<&str> = self
            .gates
            .iter()
            .filter(|g| g.status == GateStatus::Blocked)
            .map(|g| g.kind.title())
            .collect();
        if !blocked.is_empty() && note.is_none() {
            return Err(Refusal::new(
                "gate_blocked",
                format!(
                    "Engelleyen denetim var: {}. Bulguları düzeltip paketi yeniden yükleyin; bulgu yanlışsa nedenini \
                     not olarak yazıp öyle gönderin.",
                    blocked.join(", ")
                ),
            ));
        }
        let unchanged = self.state == ReleaseState::ChangesRequested
            && self
                .change_request
                .as_ref()
                .is_some_and(|c| c.content == self.content.package);
        if unchanged && note.is_none() {
            return Err(Refusal::new(
                "unchanged_resubmit",
                "İçerik, değişiklik istendiğinden beri aynı. İstenenleri yapıp paketi yeniden yükleyin; \
                 değiştirmeyecekseniz nedenini not olarak yazıp gönderin.",
            ));
        }
        let from = self.state;
        self.state = ReleaseState::Submitted;
        self.log(
            actor,
            Action::Submit,
            from,
            at,
            None,
            note.map(str::to_owned),
        );
        Ok(())
    }

    /// A scanner's report on one gate, for the content it scanned. A report for other content is refused: a late
    /// answer about the old content never verifies the new. A gate that now blocks sends a release in review back to
    /// its author with the findings.
    pub fn record_scan(
        &mut self,
        actor: &Actor,
        gate: GateKind,
        content: &str,
        outcome: ScanOutcome,
        at: Timestamp,
    ) -> Result<(), Refusal> {
        self.admit(actor, Action::Scan)?;
        if content != self.content.package {
            return Err(Refusal::new(
                "scan_stale",
                format!(
                    "Bu sonuç başka bir içeriğin ({content}); sürümün içeriği {}. Güncel içerik yeniden taranır.",
                    self.content.package
                ),
            ));
        }
        if matches!(&outcome, ScanOutcome::Blocked(findings) if findings.is_empty()) {
            return Err(Refusal::new(
                "scan_unexplained",
                format!(
                    "{} engelledi ama nedenini söylemedi; bulgusuz engel kaydedilmez. Denetim bulgularıyla yeniden \
                     bildirmeli.",
                    gate.title()
                ),
            ));
        }
        let from = self.state;
        let words = outcome.words();
        let index = self.gate_index(gate);
        let was_blocked = self.gates[index].status == GateStatus::Blocked;
        self.gates[index].report(&outcome, at);
        let blocks = self.gates[index].status == GateStatus::Blocked && !was_blocked;
        if blocks && matches!(self.state, ReleaseState::Submitted | ReleaseState::InReview) {
            self.state = ReleaseState::ChangesRequested;
            self.change_request = Some(ChangeRequest {
                by: actor.id.clone(),
                at,
                content: self.content.package.clone(),
                gate: Some(gate),
                reasons: self.gates[index].findings.clone(),
            });
        }
        self.log(actor, Action::Scan, from, at, Some(gate), Some(words));
        Ok(())
    }

    /// A reviewer takes a submitted release. Its focus ([`classify`], D12) is computed here, never handed in: from the
    /// release's own package (`current`) and the version approved before it (`baseline`: that release, which
    /// [`Release::baseline`] finds, and its package; none for the first). `trusted`: the catalog trusts the publisher.
    pub fn start_review(
        &mut self,
        actor: &Actor,
        current: &ReleaseFacts,
        baseline: Option<(&Release, &ReleaseFacts)>,
        trusted: bool,
        at: Timestamp,
    ) -> Result<(), Refusal> {
        self.admit(actor, Action::StartReview)?;
        if current.content() != self.content {
            return Err(Refusal::new(
                "package_mismatch",
                "Verilen paket bu sürümün içeriği değil; inceleme sürümün kendi paketiyle başlar.",
            ));
        }
        if let Some((release, facts)) = baseline {
            let approved = release
                .approval
                .as_ref()
                .is_some_and(|a| a.content == facts.content());
            if !approved || self.baseline(std::slice::from_ref(release)).is_none() {
                return Err(Refusal::new(
                    "baseline_invalid",
                    format!(
                        "{} {} karşılaştırma için uygun değil: bu eklentinin bu sürümden önce onaylanmış, askıya \
                         alınmamış ve geri çekilmemiş bir sürümü ve onun onaylanan paketi olmalı.",
                        release.extension_id, release.version
                    ),
                ));
            }
        }
        let focus = classify(baseline.map(|(_, facts)| facts), current, trusted);
        let from = self.state;
        let summary = focus.summary.clone();
        self.focus = Some(focus);
        self.state = ReleaseState::InReview;
        self.log(actor, Action::StartReview, from, at, None, Some(summary));
        Ok(())
    }

    /// Sends a release back with ready reasons; an approval not yet published is taken back too.
    pub fn request_changes(
        &mut self,
        actor: &Actor,
        changes: &[RequestedChange],
        at: Timestamp,
    ) -> Result<(), Refusal> {
        self.admit(actor, Action::RequestChanges)?;
        if changes.is_empty() {
            return Err(Refusal::new(
                "changes_missing",
                "Ne değişmesi gerektiği yazılmadı: en az bir hazır gerekçe seçin, dosyası ya da bağımlılığıyla.",
            ));
        }
        if changes.iter().any(|c| {
            c.code == ChangeCode::Other && c.note.as_deref().is_none_or(|n| n.trim().is_empty())
        }) {
            return Err(Refusal::new(
                "reason_missing",
                "“Diğer” gerekçesi bir not ister: neyin değişmesi gerektiğini yazın.",
            ));
        }
        let from = self.state;
        let codes: Vec<&str> = changes.iter().map(|c| c.code.id()).collect();
        self.state = ReleaseState::ChangesRequested;
        self.approval = None;
        self.change_request = Some(ChangeRequest {
            by: actor.id.clone(),
            at,
            content: self.content.package.clone(),
            gate: None,
            reasons: changes.iter().map(RequestedChange::reason).collect(),
        });
        self.log(
            actor,
            Action::RequestChanges,
            from,
            at,
            None,
            Some(codes.join(", ")),
        );
        Ok(())
    }

    /// Lifts a blocking finding with a reason (D11): only a gate whose rules allow judgement, only a scan that finished
    /// and blocked; a gate not scanned or errored is scanned again, never lifted.
    pub fn waive(
        &mut self,
        actor: &Actor,
        gate: GateKind,
        reason: &str,
        at: Timestamp,
    ) -> Result<(), Refusal> {
        self.admit(actor, Action::Waive)?;
        let reason = required(
            reason,
            "Engeli kaldırmanın gerekçesi yazılmalı: bulgu neden yanlış ya da neden kabul edilebilir?",
        )?;
        if !gate.waivable() {
            return Err(Refusal::new(
                "waiver_not_allowed",
                format!(
                    "{} gerekçeyle kaldırılamaz: kuralları kesindir, paket düzeltilmeli.",
                    gate.title()
                ),
            ));
        }
        let index = self.gate_index(gate);
        if self.gates[index].status != GateStatus::Blocked {
            return Err(Refusal::new(
                "waiver_not_blocked",
                format!(
                    "{}: {}. Yalnız engelleyen bir bulgu gerekçeyle kaldırılır; taranmamış ya da hata veren denetim \
                     yeniden çalıştırılır, atlanmaz.",
                    gate.title(),
                    self.gates[index].detail()
                ),
            ));
        }
        let from = self.state;
        let g = &mut self.gates[index];
        g.status = GateStatus::Verified;
        g.waiver = Some(Waiver {
            by: actor.id.clone(),
            at,
            reason: reason.clone(),
        });
        self.log(actor, Action::Waive, from, at, Some(gate), Some(reason));
        Ok(())
    }

    /// Approves this content: a person's decision, every gate verified (PY-CLOUD-07; D11).
    pub fn approve(
        &mut self,
        actor: &Actor,
        note: Option<&str>,
        at: Timestamp,
    ) -> Result<(), Refusal> {
        self.admit(actor, Action::Approve)?;
        if let Some(open) = self.open_gates() {
            return Err(open);
        }
        let from = self.state;
        // A release in review always has its focus; were it lost, the deepest reading is the one asked.
        let depth = self.focus.as_ref().map_or(ReviewDepth::Deep, |f| f.depth);
        self.approval = Some(Approval {
            by: actor.id.clone(),
            at,
            content: self.content.clone(),
            depth,
        });
        self.state = ReleaseState::Approved;
        let note = note
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_owned);
        self.log(actor, Action::Approve, from, at, None, note);
        Ok(())
    }

    /// Publishes an approved release: its publisher's explicit act (PY-CLOUD-01), from a window or a program alike.
    pub fn publish(&mut self, actor: &Actor, at: Timestamp) -> Result<(), Refusal> {
        self.admit(actor, Action::Publish)?;
        self.approval_holds()?;
        let from = self.state;
        self.state = ReleaseState::Published;
        self.log(actor, Action::Publish, from, at, None, None);
        Ok(())
    }

    /// Suspends a published release under suspicion: no new install, no run, the reason shown (PY-CLOUD-15).
    pub fn suspend(&mut self, actor: &Actor, reason: &str, at: Timestamp) -> Result<(), Refusal> {
        self.admit(actor, Action::Suspend)?;
        let reason = required(
            reason,
            "Askıya alma nedeni yazılmalı; kuranlar ve yayımcı onu görür.",
        )?;
        self.close(
            actor,
            Action::Suspend,
            ReleaseState::Suspended,
            Some(reason),
            at,
        );
        Ok(())
    }

    /// Puts a suspended release back: a person's decision, its approval still of its content.
    pub fn reinstate(&mut self, actor: &Actor, reason: &str, at: Timestamp) -> Result<(), Refusal> {
        self.admit(actor, Action::Reinstate)?;
        let reason = required(
            reason,
            "Askıdan almanın gerekçesi yazılmalı: inceleme ne buldu?",
        )?;
        self.approval_holds()?;
        let from = self.state;
        self.state = ReleaseState::Published;
        self.notice = None;
        self.log(actor, Action::Reinstate, from, at, None, Some(reason));
        Ok(())
    }

    /// Withdraws a published or suspended release for good: its publisher or a moderator, with the reason shown.
    pub fn withdraw(&mut self, actor: &Actor, reason: &str, at: Timestamp) -> Result<(), Refusal> {
        self.admit(actor, Action::Withdraw)?;
        let reason = required(
            reason,
            "Geri çekme nedeni yazılmalı; kuranlar ve yayımcı onu görür.",
        )?;
        self.close(
            actor,
            Action::Withdraw,
            ReleaseState::Withdrawn,
            Some(reason),
            at,
        );
        Ok(())
    }

    /// Archives a release: a published one retires (installed copies keep running), an unpublished one is given up.
    pub fn archive(
        &mut self,
        actor: &Actor,
        reason: Option<&str>,
        at: Timestamp,
    ) -> Result<(), Refusal> {
        self.admit(actor, Action::Archive)?;
        let reason = reason
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .map(str::to_owned);
        self.close(actor, Action::Archive, ReleaseState::Archived, reason, at);
        Ok(())
    }

    /// Whether a downloaded package may be installed as this release (PY-CLOUD-10, 15): the release is published and
    /// what was downloaded is what was approved, byte for byte.
    pub fn allows_install(&self, downloaded: &ReleaseContent) -> Result<(), Refusal> {
        if self.state != ReleaseState::Published {
            return Err(Refusal::new("not_installable", self.unavailable()));
        }
        match &self.approval {
            Some(a) if a.content == *downloaded && self.content == *downloaded => Ok(()),
            _ => Err(Refusal::new(
                "content_mismatch",
                format!(
                    "İndirilen paket {} {} için onaylanan içerik değil; kurulmadı. Paketi katalogdan yeniden indirin.",
                    self.extension_id, self.version
                ),
            )),
        }
    }

    /// Whether this release may run: in the catalog's cloud jobs, and on a KentOS that syncs the catalog (PY-CLOUD-15).
    /// A retired version that was published keeps running; a suspended or withdrawn one does not.
    pub fn allows_run(&self) -> Result<(), Refusal> {
        match self.state {
            ReleaseState::Published => Ok(()),
            ReleaseState::Archived if self.was_published() => Ok(()),
            _ => Err(Refusal::new("not_runnable", self.unavailable())),
        }
    }

    /// Whether this account may see the release, inside its catalog (whose membership the server checks first): a
    /// published release and the notice of one taken back are seen by all, anything else by its author, its publisher
    /// and the catalog's reviewers, moderators and scanners; a personal catalog's by its owner and the catalog's
    /// scanners only (its gates are scanned there too, by a service that decides nothing). Counts, searches and results
    /// are computed after this (PY-CLOUD-17).
    pub fn visible_to(&self, actor: &Actor) -> bool {
        if self.scope == CatalogScope::Private {
            return self.owned_by(actor) || actor.can(Permission::Scan);
        }
        let public = matches!(
            self.state,
            ReleaseState::Published | ReleaseState::Suspended | ReleaseState::Withdrawn
        ) || (self.state == ReleaseState::Archived && self.was_published());
        public
            || self.owned_by(actor)
            || [
                Permission::Scan,
                Permission::Review,
                Permission::Approve,
                Permission::Moderate,
            ]
            .into_iter()
            .any(|p| actor.can(p))
    }

    /// What the catalog says of a release taken out, in Turkish: why, what it stops, and, honestly, that bytes already
    /// downloaded cannot be taken back (PY-CLOUD-15).
    pub fn notice_text(&self) -> Option<String> {
        let notice = self.notice.as_ref()?;
        let why = self
            .notice_reason()
            .map(|r| format!(": {r}"))
            .unwrap_or_default();
        const DOWNLOADED: &str = "İndirilmiş kopyalar silinemez ve geri alınamaz; KentOS geri çekme listesini \
                                  eşitlediğinde onları durdurur (verileri kalır), eşitlemeyen bilgisayar bunu bilmez.";
        Some(match notice.state {
            ReleaseState::Suspended => format!(
                "{} {} askıya alındı{why}. Yeni kurulum ve çalıştırma durdu; {UNDER_REVIEW}. {DOWNLOADED}",
                self.extension_id, self.version
            ),
            ReleaseState::Withdrawn => format!(
                "{} {} geri çekildi{why}. Kurulmaz ve çalıştırılmaz. {DOWNLOADED}",
                self.extension_id, self.version
            ),
            _ if self.was_published() => format!(
                "{} {} arşivlendi{why}. Yeni kurulum yok; kurulu kopyalar çalışmaya devam eder.",
                self.extension_id, self.version
            ),
            _ => format!(
                "{} {} yayımlanmadan arşivlendi{why}.",
                self.extension_id, self.version
            ),
        })
    }

    /// The reason the notice gives, without its closing full stop: every sentence that says it (the notice, the
    /// revocation list's entry, the host's message) closes it itself.
    fn notice_reason(&self) -> Option<&str> {
        self.notice
            .as_ref()?
            .reason
            .as_deref()
            .map(|r| r.trim().trim_end_matches('.'))
            .filter(|r| !r.is_empty())
    }

    /// The release a new version's review is compared with (D12): of `releases` (the same catalog's), the highest
    /// version of this extension below this one that a reviewer approved and that was neither suspended nor withdrawn
    /// since: a withdrawn version is no baseline, its code would hide in the difference.
    pub fn baseline<'a>(&self, releases: &'a [Release]) -> Option<&'a Release> {
        let below = Version::parse(&self.version)?;
        releases
            .iter()
            .filter(|r| r.extension_id == self.extension_id && r.approval.is_some())
            .filter(|r| !matches!(r.state, ReleaseState::Suspended | ReleaseState::Withdrawn))
            .filter_map(|r| Some((Version::parse(&r.version)?, r)))
            .filter(|(v, _)| *v < below)
            .max_by(|a, b| a.0.cmp(&b.0))
            .map(|(_, r)| r)
    }

    /// A stored release, read through the flow's checks: a record that is not one is `record_invalid`, one that could
    /// not have come from the flow's moves `record_inconsistent`. Deserializing a [`Release`] runs the same checks.
    pub fn read(text: &str) -> Result<Release, Refusal> {
        let stored: stored::Stored = serde_json::from_str(text).map_err(|e| {
            Refusal::new(
                "record_invalid",
                format!(
                    "Sürüm kaydı okunamadı: {}",
                    e.to_string().split(" at line ").next().unwrap_or_default()
                ),
            )
        })?;
        Release::try_from(stored)
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn scope(&self) -> CatalogScope {
        self.scope
    }

    pub fn publisher(&self) -> &str {
        &self.publisher
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn content(&self) -> &ReleaseContent {
        &self.content
    }

    pub fn state(&self) -> ReleaseState {
        self.state
    }

    pub fn gates(&self) -> &[Gate] {
        &self.gates
    }

    pub fn focus(&self) -> Option<&ReviewFocus> {
        self.focus.as_ref()
    }

    pub fn approval(&self) -> Option<&Approval> {
        self.approval.as_ref()
    }

    pub fn change_request(&self) -> Option<&ChangeRequest> {
        self.change_request.as_ref()
    }

    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    pub fn audit(&self) -> &[AuditRecord] {
        &self.audit
    }

    /// In the catalog's list and search: published.
    pub fn listed(&self) -> bool {
        self.state == ReleaseState::Published
    }

    /// Its name and icon as its content says them; none in a record kept before they were.
    pub fn listing(&self) -> Option<&Listing> {
        self.listing.as_ref()
    }

    /// What a person sees of it, as the imitation rule reads it, under its publisher.
    pub fn known(&self) -> Option<crate::resemblance::Known> {
        self.listing.as_ref().map(|l| crate::resemblance::Known {
            id: self.extension_id.clone(),
            name: l.name.clone(),
            icon: l.icon.clone(),
            publisher: Some(self.publisher.clone()),
        })
    }

    /// The publisher gate's imitation rule (docs/adr/8059 §14, D13): the listed extensions of other publishers among
    /// `releases` (the same catalog's) whose name, id or icon this release reads as, each extension once; the findings
    /// that keep it out. A record kept before its name and icon were is not compared.
    pub fn imitations(&self, releases: &[Release]) -> Vec<Reason> {
        let Some(me) = self.known() else {
            return Vec::new();
        };
        let mut others: std::collections::BTreeMap<&str, crate::resemblance::Known> =
            std::collections::BTreeMap::new();
        for release in releases.iter().filter(|r| r.listed()) {
            if let Some(known) = release.known() {
                others.insert(release.extension_id.as_str(), known);
            }
        }
        let others: Vec<crate::resemblance::Known> = others.into_values().collect();
        crate::resemblance::imitations(&me, &others)
    }

    /// Every gate verified for this content.
    pub fn gates_verified(&self) -> bool {
        self.gates.iter().all(|g| g.status == GateStatus::Verified)
    }

    fn owned_by(&self, actor: &Actor) -> bool {
        actor.id == self.author || actor.publishers.contains(&self.publisher)
    }

    fn was_published(&self) -> bool {
        self.audit.iter().any(|r| r.action == Action::Publish)
    }

    fn gate_index(&self, kind: GateKind) -> usize {
        // The gates are one per kind in `GateKind::ALL`'s order (checked when a record is read).
        GateKind::ALL
            .iter()
            .position(|k| *k == kind)
            .unwrap_or_default()
    }

    /// The gates that keep an approval back, said with why; none when every gate is verified.
    fn open_gates(&self) -> Option<Refusal> {
        let open: Vec<String> = self
            .gates
            .iter()
            .filter(|g| g.status != GateStatus::Verified)
            .map(|g| format!("{} ({})", g.kind.title(), g.detail()))
            .collect();
        (!open.is_empty()).then(|| {
            Refusal::new(
                "gates_open",
                format!(
                    "Kendiliğinden denetimler doğrulanmadan onay verilmez: {}. Taranmamış ya da hata veren denetim \
                     geçmiş sayılmaz.",
                    open.join(", ")
                ),
            )
        })
    }

    fn approval_holds(&self) -> Result<(), Refusal> {
        match &self.approval {
            Some(a) if a.content == self.content && self.gates_verified() => Ok(()),
            _ => Err(Refusal::new(
                "approval_stale",
                "Onay bu içeriğin değil ya da denetimleri doğrulanmamış; sürüm yeniden incelenmeli.",
            )),
        }
    }

    fn unavailable(&self) -> String {
        self.notice_text().unwrap_or_else(|| {
            format!(
                "{} {} yayımlanmadı (sürüm “{}”); yalnız yayımlanan sürüm kurulur ve çalışır.",
                self.extension_id,
                self.version,
                self.state.title()
            )
        })
    }

    fn close(
        &mut self,
        actor: &Actor,
        action: Action,
        to: ReleaseState,
        reason: Option<String>,
        at: Timestamp,
    ) {
        let from = self.state;
        self.state = to;
        self.notice = Some(Notice {
            state: to,
            by: actor.id.clone(),
            at,
            reason: reason.clone(),
        });
        self.log(actor, action, from, at, None, reason);
    }

    fn log(
        &mut self,
        actor: &Actor,
        action: Action,
        from: ReleaseState,
        at: Timestamp,
        gate: Option<GateKind>,
        reason: Option<String>,
    ) {
        self.audit.push(AuditRecord {
            at,
            by: actor.id.clone(),
            via: actor.via,
            action,
            from: Some(from),
            to: self.state,
            content: self.content.package.clone(),
            gate,
            reason,
        });
    }
}

fn fresh_gates() -> Vec<Gate> {
    GateKind::ALL.into_iter().map(Gate::new).collect()
}

/// The manifest's `publisherId` is a claim; when it names one, it is the publisher the version is uploaded under.
fn publisher_claim(facts: &ReleaseFacts, publisher: &str) -> Result<(), Refusal> {
    match &facts.manifest.publisher_id {
        Some(claimed) if claimed != publisher => Err(Refusal::new(
            "publisher_mismatch",
            format!(
                "Manifest yayımcıyı “{claimed}” diyor, sürüm “{publisher}” adına yükleniyor; manifestteki publisherId'yi \
                 düzeltin ya da doğru yayımcı adına yükleyin."
            ),
        )),
        _ => Ok(()),
    }
}

/// The text, trimmed, or the refusal that says it is wanted.
fn required(text: &str, wanted: &str) -> Result<String, Refusal> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Refusal::new("reason_missing", wanted));
    }
    Ok(text.to_owned())
}

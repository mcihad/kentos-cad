//! What the extension catalog's routes answer (docs/adr/8059 §10): a release as the caller may see it (its whole record
//! for its insiders, what anyone may know for the rest, PY-CLOUD-17), a page of releases, an install ticket.

use kentos_extensions::release::{
    Action, Actor, GateKind, GateStatus, Permission, Release, ReleaseContent, ReleaseState,
    ReviewDepth, Timestamp,
};
use serde::Serialize;

use crate::extension_catalog::Listing;

/// A release as the caller may see it.
#[derive(Serialize)]
#[serde(tag = "view", rename_all = "camelCase")]
pub enum ReleaseView {
    /// Its author, its publisher, the catalog's reviewers, administrators and scanners: the whole record.
    Insider(Box<InsiderView>),
    /// Anyone else who may see it: what it is and where it stands, without accounts, notes or findings (PY-CLOUD-17).
    Public(Box<PublicView>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsiderView {
    /// The flow's record: gates, review, approval, notice and every move.
    pub release: Release,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    /// The moves its state allows; whether this caller may make one is that move's own answer.
    pub moves: Vec<Action>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicView {
    pub extension_id: String,
    pub version: String,
    pub state: ReleaseState,
    pub publisher: String,
    pub content: ReleaseContent,
    pub gates: Vec<PublicGate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<PublicApproval>,
    /// Why it is suspended, withdrawn or archived, and what that stops (PY-CLOUD-15).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
    /// The moves that changed its state: when, from which to which; not who or why.
    pub history: Vec<PublicMove>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicGate {
    pub kind: GateKind,
    pub status: GateStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicApproval {
    pub at: Timestamp,
    pub depth: ReviewDepth,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicMove {
    pub at: Timestamp,
    pub action: Action,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ReleaseState>,
    pub to: ReleaseState,
}

/// The author, its publisher's members and the catalog's reviewers, administrators and scanners.
fn insider(release: &Release, actor: &Actor) -> bool {
    actor.id == release.author()
        || actor.publishers.contains(release.publisher())
        || [
            Permission::Scan,
            Permission::Review,
            Permission::Approve,
            Permission::Moderate,
        ]
        .into_iter()
        .any(|p| actor.can(p))
}

pub fn view(release: Release, actor: &Actor) -> ReleaseView {
    if insider(&release, actor) {
        return ReleaseView::Insider(Box::new(InsiderView {
            notice: release.notice_text(),
            moves: release.state().actions(),
            release,
        }));
    }
    ReleaseView::Public(Box::new(PublicView {
        extension_id: release.extension_id().to_owned(),
        version: release.version().to_owned(),
        state: release.state(),
        publisher: release.publisher().to_owned(),
        content: release.content().clone(),
        gates: release
            .gates()
            .iter()
            .map(|g| PublicGate {
                kind: g.kind,
                status: g.status,
            })
            .collect(),
        approval: release.approval().map(|a| PublicApproval {
            at: a.at,
            depth: a.depth,
        }),
        notice: release.notice_text(),
        history: release
            .audit()
            .iter()
            .filter(|m| m.from != Some(m.to))
            .map(|m| PublicMove {
                at: m.at,
                action: m.action,
                from: m.from,
                to: m.to,
            })
            .collect(),
    }))
}

/// A page of the releases the caller may see, and how many there are in all.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseList {
    pub total: usize,
    pub releases: Vec<ReleaseSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSummary {
    pub extension_id: String,
    pub version: String,
    pub state: ReleaseState,
    pub publisher: String,
    /// The content's digest (`sha256:…`).
    pub package: String,
    pub listed: bool,
}

impl ReleaseList {
    pub fn of(listing: Listing) -> ReleaseList {
        ReleaseList {
            total: listing.total,
            releases: listing
                .releases
                .iter()
                .map(|r| ReleaseSummary {
                    extension_id: r.extension_id().to_owned(),
                    version: r.version().to_owned(),
                    state: r.state(),
                    publisher: r.publisher().to_owned(),
                    package: r.content().package.clone(),
                    listed: r.listed(),
                })
                .collect(),
        }
    }
}

/// What may be installed (PY-CLOUD-10): a download is installed only when it is this content, byte for byte.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTicket {
    pub extension_id: String,
    pub version: String,
    pub publisher: String,
    pub content: ReleaseContent,
    pub approved_at: Timestamp,
    pub depth: ReviewDepth,
}

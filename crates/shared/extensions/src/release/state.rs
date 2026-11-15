//! Where a release is and the moves between states (plan/eklenti.md ADIM 22A): [`Action::sources`] is the flow's
//! table, which the moves read, the window shows (what a state offers) and docs/adr/8059 writes out.

use serde::{Deserialize, Serialize};

use super::Permission;

/// Where a release is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseState {
    /// Uploaded; seen by its author, its publisher and the catalog only. Its content may be renewed.
    Draft,
    /// Sent for review; its content is fixed from here on.
    Submitted,
    /// A reviewer reads it.
    InReview,
    /// Sent back by a reviewer or a blocking gate, with the reasons: new content, or a word why not, and it goes again.
    ChangesRequested,
    /// A reviewer approved this content; it is not out until it is published.
    Approved,
    /// Listed, installed and run.
    Published,
    /// Under suspicion: not installed nor run until it is reinstated or withdrawn.
    Suspended,
    /// Taken back for good: not installed nor run.
    Withdrawn,
    /// Retired, or given up before publication: not installed anew; copies installed before keep running.
    Archived,
}

impl ReleaseState {
    pub const ALL: [ReleaseState; 9] = [
        ReleaseState::Draft,
        ReleaseState::Submitted,
        ReleaseState::InReview,
        ReleaseState::ChangesRequested,
        ReleaseState::Approved,
        ReleaseState::Published,
        ReleaseState::Suspended,
        ReleaseState::Withdrawn,
        ReleaseState::Archived,
    ];

    /// Its id in a record.
    pub fn id(self) -> &'static str {
        match self {
            ReleaseState::Draft => "draft",
            ReleaseState::Submitted => "submitted",
            ReleaseState::InReview => "inReview",
            ReleaseState::ChangesRequested => "changesRequested",
            ReleaseState::Approved => "approved",
            ReleaseState::Published => "published",
            ReleaseState::Suspended => "suspended",
            ReleaseState::Withdrawn => "withdrawn",
            ReleaseState::Archived => "archived",
        }
    }

    /// Its name in the catalog's window.
    pub fn title(self) -> &'static str {
        match self {
            ReleaseState::Draft => "Taslak",
            ReleaseState::Submitted => "Gönderildi",
            ReleaseState::InReview => "Denetleniyor",
            ReleaseState::ChangesRequested => "Değişiklik isteniyor",
            ReleaseState::Approved => "Onaylandı",
            ReleaseState::Published => "Yayımlandı",
            ReleaseState::Suspended => "Askıda",
            ReleaseState::Withdrawn => "Geri çekildi",
            ReleaseState::Archived => "Arşivlendi",
        }
    }

    /// What may be done to a release in it.
    pub fn actions(self) -> Vec<Action> {
        Action::ALL
            .into_iter()
            .filter(|a| a.sources().contains(&self))
            .collect()
    }

    /// Nothing moves it any more: withdrawn or archived.
    pub fn is_final(self) -> bool {
        self.actions().is_empty()
    }
}

/// A move: each is a method of [`Release`](super::Release) and leaves an [`AuditRecord`](super::AuditRecord).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Upload,
    Replace,
    Submit,
    Scan,
    StartReview,
    RequestChanges,
    Waive,
    Approve,
    Publish,
    Suspend,
    Reinstate,
    Withdraw,
    Archive,
}

impl Action {
    pub const ALL: [Action; 13] = [
        Action::Upload,
        Action::Replace,
        Action::Submit,
        Action::Scan,
        Action::StartReview,
        Action::RequestChanges,
        Action::Waive,
        Action::Approve,
        Action::Publish,
        Action::Suspend,
        Action::Reinstate,
        Action::Withdraw,
        Action::Archive,
    ];

    /// Its name in the catalog's window.
    pub fn title(self) -> &'static str {
        match self {
            Action::Upload => "Yükle",
            Action::Replace => "İçeriği yenile",
            Action::Submit => "İncelemeye gönder",
            Action::Scan => "Denetim sonucunu bildir",
            Action::StartReview => "İncelemeye al",
            Action::RequestChanges => "Değişiklik iste",
            Action::Waive => "Engeli gerekçeyle kaldır",
            Action::Approve => "Onayla",
            Action::Publish => "Yayımla",
            Action::Suspend => "Askıya al",
            Action::Reinstate => "Askıdan al",
            Action::Withdraw => "Geri çek",
            Action::Archive => "Arşivle",
        }
    }

    /// The states it may be done in: the flow's table. An upload makes a release and has none.
    pub fn sources(self) -> &'static [ReleaseState] {
        use ReleaseState::*;
        match self {
            Action::Upload => &[],
            Action::Replace | Action::Submit => &[Draft, ChangesRequested],
            Action::Scan => &[Draft, Submitted, InReview, ChangesRequested],
            Action::StartReview => &[Submitted],
            Action::RequestChanges => &[InReview, Approved],
            Action::Waive | Action::Approve => &[InReview],
            Action::Publish => &[Approved],
            Action::Suspend => &[Published],
            Action::Reinstate => &[Suspended],
            Action::Withdraw => &[Published, Suspended],
            Action::Archive => &[
                Draft,
                Submitted,
                InReview,
                ChangesRequested,
                Approved,
                Published,
            ],
        }
    }

    /// The permission it takes. Withdrawing and archiving take [`Permission::Moderate`], or [`Permission::Publish`] on
    /// one's own publisher's release.
    pub fn permission(self) -> Permission {
        match self {
            Action::Upload | Action::Replace => Permission::Upload,
            Action::Submit => Permission::Submit,
            Action::Scan => Permission::Scan,
            Action::StartReview | Action::RequestChanges | Action::Waive => Permission::Review,
            Action::Approve => Permission::Approve,
            Action::Publish => Permission::Publish,
            Action::Suspend | Action::Reinstate | Action::Withdraw | Action::Archive => {
                Permission::Moderate
            }
        }
    }

    /// A person's decision, made in the catalog's window: a program's call is refused, whoever's token it carries
    /// (PY-CLOUD-18).
    pub fn needs_person(self) -> bool {
        matches!(
            self,
            Action::RequestChanges | Action::Waive | Action::Approve | Action::Reinstate
        )
    }
}

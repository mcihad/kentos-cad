//! What the registry shows: an extension's state and trust in words, the review before installing, a project's
//! extensions (plan/eklenti.md ADIM 17). The same meaning on the window, in Python and over MCP (PY-RUN-03).

use kentos_extensions::Inspection;
use kentos_extensions::manifest::{Capability, Manifest};
use serde::Serialize;

use crate::journal::Record;
use crate::store::{Channel, Metrics, Source};

/// An installed extension's state, worked out from the registry and what runs. A package being looked at, inspected
/// or prepared is a [`Review`] or a call in progress, never a state on disk: a crash in the middle leaves the previous
/// state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum State {
    /// Running: its contributions are in the window.
    Active,
    /// Enabled and ready, not running (a start without extensions, or waiting for its turn).
    Ready,
    /// Stopped by the user.
    Disabled,
    /// An extension it depends on is missing, stopped or of another version.
    Waiting { reason: String },
    /// The organisation's policy refuses it, or its version was withdrawn.
    Blocked { reason: String },
    /// Not for this KentOS.
    Incompatible { reason: String },
    /// It crashed again and again, or its files changed: not tried until the user retries.
    Failed { reason: String },
}

impl State {
    pub fn label(&self) -> &'static str {
        match self {
            State::Active => "Etkin",
            State::Ready => "Hazır",
            State::Disabled => "Devre dışı",
            State::Waiting { .. } => "Bağımlılık bekliyor",
            State::Blocked { .. } => "Engellendi",
            State::Incompatible { .. } => "Uyumsuz",
            State::Failed { .. } => "Hata",
        }
    }

    /// What it means and what to do.
    pub fn help(&self) -> String {
        match self {
            State::Active => "Katkıları şeritte, menülerde ve Kaynaklar'da.".to_owned(),
            State::Ready => {
                "Etkinleşmeye hazır; eklentisiz başlatıldıysa bir sonraki açılışta etkinleşir."
                    .to_owned()
            }
            State::Disabled => "Siz kapattınız; Etkinleştir ile açılır. Verisi duruyor.".to_owned(),
            State::Waiting { reason } => format!("{reason} Önce onu kurun ya da etkinleştirin."),
            State::Blocked { reason } => format!("{reason} Verisi silinmedi."),
            State::Incompatible { reason } => reason.clone(),
            State::Failed { reason } => format!(
                "{reason} Bir sonraki açılışta denenmez; Yeniden dene ile bir kez daha başlatın, sorun sürerse \
                 yayımcısına bildirin."
            ),
        }
    }
}

/// How far an installed package is trusted, always shown (Ö·5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Trust {
    /// A file or a working folder of this computer.
    Local,
    /// The organisation's catalog.
    Organization,
    /// The public catalog's reviewed release (its signature is the catalog's, ADIM 22A).
    Approved,
    /// Its files are not what was installed.
    IntegrityBroken,
    /// The policy refuses it or its version was withdrawn.
    Blocked,
}

impl Trust {
    pub fn label(self) -> &'static str {
        match self {
            Trust::Local => "Yerel",
            Trust::Organization => "Kurum",
            Trust::Approved => "Onaylı (genel katalog)",
            Trust::IntegrityBroken => "Bütünlüğü bozuk",
            Trust::Blocked => "Engelli",
        }
    }
}

/// An installed extension as the manager lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionView {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: State,
    pub state_label: &'static str,
    pub help: String,
    pub trust: Trust,
    pub trust_label: &'static str,
    pub source: Source,
    pub enabled: bool,
    /// Every version on disk, the one in use first.
    pub versions: Vec<String>,
    pub channel: Channel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Metrics>,
    /// The error still standing: the newest in its log not followed by new files or a retry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<Record>,
    /// The newest records of its log, oldest first.
    pub log: Vec<Record>,
}

/// A dependency as the review shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub id: String,
    pub versions: String,
    /// Installed and enabled in a version of the range.
    pub met: bool,
    pub note: String,
}

/// A capability as the review shows it, against the version installed now (D5: the difference at every update).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityLine {
    pub capability: Capability,
    pub title: &'static str,
    pub reason: String,
    pub hosts: Vec<String>,
    pub sends_data_to_ai: bool,
    pub change: Change,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Change {
    /// First install: nothing to compare with.
    Asked,
    /// The installed version did not ask for it.
    Added,
    /// Asked before too; its reason, hosts or AI use changed.
    Changed,
    /// Asked before too, the same.
    Same,
    /// The installed version asked for it; this one does not.
    Removed,
}

/// Everything to show before the user decides (PY-KEK-10): nothing is installed by reviewing.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub inspection: Inspection,
    /// Why this KentOS cannot run it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incompatible: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked: Option<String>,
    pub dependencies: Vec<DependencyStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle: Option<Vec<String>>,
    /// The version installed now, and from where; an update or a reinstall.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed: Option<(String, Source)>,
    /// Installed from another source: installing this one is the user's explicit choice.
    pub source_conflict: bool,
    pub capabilities: Vec<CapabilityLine>,
    /// It can be installed (it may still wait for a dependency).
    pub installable: bool,
}

/// The capabilities of `new` against `old`'s.
pub fn capability_lines(new: &Manifest, old: Option<&Manifest>) -> Vec<CapabilityLine> {
    let mut lines: Vec<CapabilityLine> = new
        .requested_capabilities
        .iter()
        .map(|r| {
            let before = old.and_then(|o| o.requested_capabilities.iter().find(|p| p.id == r.id));
            let change = match (old, before) {
                (None, _) => Change::Asked,
                (Some(_), None) => Change::Added,
                (Some(_), Some(b))
                    if b.reason != r.reason
                        || b.hosts != r.hosts
                        || b.sends_data_to_ai != r.sends_data_to_ai =>
                {
                    Change::Changed
                }
                (Some(_), Some(_)) => Change::Same,
            };
            CapabilityLine {
                capability: r.id,
                title: r.id.title(),
                reason: r.reason.clone(),
                hosts: r.hosts.clone(),
                sends_data_to_ai: r.sends_data_to_ai == Some(true),
                change,
            }
        })
        .collect();
    if let Some(old) = old {
        for r in &old.requested_capabilities {
            if !new.requested_capabilities.iter().any(|n| n.id == r.id) {
                lines.push(CapabilityLine {
                    capability: r.id,
                    title: r.id.title(),
                    reason: r.reason.clone(),
                    hosts: r.hosts.clone(),
                    sends_data_to_ai: r.sends_data_to_ai == Some(true),
                    change: Change::Removed,
                });
            }
        }
    }
    lines
}

/// An extension a project names (its data in `.kcad`'s `modules.ext`, D16).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectNeed {
    pub id: String,
    pub versions: String,
    pub status: NeedStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum NeedStatus {
    /// Running in a version of the range.
    Ready,
    /// Installed but not running, or in another version.
    NotRunning { reason: String },
    /// Not installed: the project opens restricted, nothing is installed for it unasked.
    Missing,
}

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
    /// Enabled, held off for this round of "Sorunlu eklentiyi bul": it comes back at the end; the user's choice did
    /// not change.
    Held,
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
            State::Held => "Sorun bulmada kapalı",
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
            State::Held => "Sorunlu eklentiyi bulma sürüyor: bu turda kapalı. Bitince ya da bırakılınca kendiliğinden \
                            açılır; seçiminiz değişmedi."
                .to_owned(),
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
    /// Another installed extension it reads as, by name or id (docs/adr/8059 §14): said beside it while both are
    /// installed, so the one installed in spite of the review's warning stays marked. Icons were compared at Kur.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resemblances: Vec<kentos_extensions::resemblance::Resemblance>,
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
    /// What it reads as among the installed extensions and the catalog's (docs/adr/8059 §14): its name, its id or its
    /// icon, the imitations first. Said before Kur; an imitation installs only when the person says so.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resemblances: Vec<kentos_extensions::resemblance::Resemblance>,
    /// What an update changes in where it runs (PY-CLOUD-14): the systems and the runtimes it says, a sentence each;
    /// none for a first install or when they are the same.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
    /// The Python packages its lock installs, an update's against the installed version's (docs/adr/8057 §81).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub python_packages: Vec<crate::packages::PackageLine>,
    /// It can be installed (it may still wait for a dependency).
    pub installable: bool,
}

impl Review {
    /// Whether it imitates another extension: the person must say to install it anyway.
    pub fn imitates(&self) -> bool {
        self.resemblances.iter().any(|r| r.exact)
    }
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

/// What `new` changes against the installed `old` in where it runs (PY-CLOUD-14): the operating systems (none written:
/// every one) and the runtimes, what it no longer runs on first; none for a first install.
pub fn platform_lines(new: &Manifest, old: Option<&Manifest>) -> Vec<String> {
    use kentos_extensions::compat::Os;
    use kentos_extensions::manifest::Runtime;
    let Some(old) = old else {
        return Vec::new();
    };
    let systems = |m: &Manifest| -> Vec<Os> {
        let written = &m.compatibility.operating_systems;
        if written.is_empty() {
            Os::ALL.to_vec()
        } else {
            Os::ALL
                .into_iter()
                .filter(|o| written.contains(o))
                .collect()
        }
    };
    let (before, after) = (systems(old), systems(new));
    let names = |list: Vec<Os>| {
        list.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let dropped: Vec<Os> = before
        .iter()
        .copied()
        .filter(|o| !after.contains(o))
        .collect();
    let added: Vec<Os> = after
        .iter()
        .copied()
        .filter(|o| !before.contains(o))
        .collect();
    let mut out = Vec::new();
    if !dropped.is_empty() {
        out.push(format!(
            "Artık şu sistemlerde çalışmıyor: {}.",
            names(dropped)
        ));
    }
    let runtime = |r: Runtime| match r {
        Runtime::Desktop => "KentOS masaüstünde",
        Runtime::Headless => "başsız (MCP ajanları ve KentOS'un Python kitaplığı)",
        Runtime::Cloud => "bulutta (sunucunun işlerinde)",
    };
    for r in [Runtime::Desktop, Runtime::Headless, Runtime::Cloud] {
        if old.runtimes.contains(&r) && !new.runtimes.contains(&r) {
            out.push(format!("Artık {} çalışmıyor.", runtime(r)));
        }
    }
    if !added.is_empty() {
        out.push(format!(
            "Artık şu sistemlerde de çalışıyor: {}.",
            names(added)
        ));
    }
    for r in [Runtime::Desktop, Runtime::Headless, Runtime::Cloud] {
        if new.runtimes.contains(&r) && !old.runtimes.contains(&r) {
            out.push(format!("Artık {} da çalışıyor.", runtime(r)));
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The example extension's manifest with `systems` and `runtimes` written.
    fn manifest(systems: &[&str], runtimes: &[&str]) -> Manifest {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/extensions/v1/ornek-altlik/manifest.json");
        let mut m: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root).unwrap()).unwrap();
        m["compatibility"]["operatingSystems"] = serde_json::json!(systems);
        m["runtimes"] = serde_json::json!(runtimes);
        Manifest::read(&m.to_string()).expect("a manifest")
    }

    /// An update says the systems and runtimes it drops, then those it adds; a first install and the same places say
    /// nothing; no system written is every system.
    #[test]
    fn an_update_says_where_it_no_longer_runs() {
        let every = manifest(&[], &["desktop", "headless"]);
        let two = manifest(&["linux", "macos"], &["desktop", "cloud"]);
        assert_eq!(
            platform_lines(&two, Some(&every)),
            [
                "Artık şu sistemlerde çalışmıyor: Windows.",
                "Artık başsız (MCP ajanları ve KentOS'un Python kitaplığı) çalışmıyor.",
                "Artık bulutta (sunucunun işlerinde) da çalışıyor."
            ]
        );
        assert_eq!(
            platform_lines(&every, Some(&two)),
            [
                "Artık bulutta (sunucunun işlerinde) çalışmıyor.",
                "Artık şu sistemlerde de çalışıyor: Windows.",
                "Artık başsız (MCP ajanları ve KentOS'un Python kitaplığı) da çalışıyor."
            ]
        );
        assert!(platform_lines(&two, None).is_empty(), "a first install");
        let all_written = manifest(&["windows", "macos", "linux"], &["desktop", "headless"]);
        assert!(
            platform_lines(&all_written, Some(&every)).is_empty(),
            "the same places"
        );
    }
}

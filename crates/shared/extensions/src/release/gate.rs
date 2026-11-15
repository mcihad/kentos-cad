//! The automatic gates every version passes before a person approves it (plan/eklenti.md ADIM 22A; PY-CLOUD-05, 06;
//! D11). A gate is verified only by a scan that ran to its end on this content and found nothing that blocks: not
//! scanned, running and errored are all "verifying", never a pass, so a scanner that could not run never lets a
//! release through. A finding blocks; a reviewer may lift the finding of a gate whose rules allow judgement, with a
//! reason, never a scan that did not finish.

use serde::{Deserialize, Serialize};

use super::{Reason, Timestamp};

/// What a gate checks. Every version passes them all, in every catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GateKind {
    /// The archive, the manifest and what it names, the integrity list, invisible characters: `kentos kek verify`'s
    /// gates, the same code as on the author's computer.
    Package,
    /// The lock: known vulnerabilities, names that do not exist or imitate known ones, the package indexes.
    Dependencies,
    /// The package's licence and its dependencies'.
    Licenses,
    /// The SDK protocol and the KentOS versions it says it runs on.
    Compatibility,
    /// Keys, tokens and passwords in its files, AI tools' configuration files too.
    Secrets,
    /// Its tests and a smoke run in the isolated review worker, with no real secret or production data (PY-CLOUD-06).
    Tests,
    /// The publisher owns the id's namespace; the name, icon and id imitate no other's (D13).
    Publisher,
}

impl GateKind {
    pub const ALL: [GateKind; 7] = [
        GateKind::Package,
        GateKind::Dependencies,
        GateKind::Licenses,
        GateKind::Compatibility,
        GateKind::Secrets,
        GateKind::Tests,
        GateKind::Publisher,
    ];

    pub fn title(self) -> &'static str {
        match self {
            GateKind::Package => "Paket denetimi",
            GateKind::Dependencies => "Bağımlılık denetimi",
            GateKind::Licenses => "Lisans denetimi",
            GateKind::Compatibility => "Uyumluluk denetimi",
            GateKind::Secrets => "Sır taraması",
            GateKind::Tests => "Test ve duman testi",
            GateKind::Publisher => "Yayımcı denetimi",
        }
    }

    /// Whether a reviewer may lift its finding with a reason. The package's, the compatibility's and the publisher's
    /// rules are exact: a package that breaks them would not install, would not run, or is not its publisher's to put
    /// out, so it is mended, never let through.
    pub fn waivable(self) -> bool {
        matches!(
            self,
            GateKind::Dependencies | GateKind::Licenses | GateKind::Secrets | GateKind::Tests
        )
    }
}

/// Where a gate is. There is no fourth "open" state: what has not finished is verifying.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GateStatus {
    /// Not scanned yet for this content, scanning, or its scan errored and runs again. Never a pass.
    Verifying,
    /// The scan finished and found what keeps the release out; its findings say what to mend.
    Blocked,
    /// The scan finished and found nothing that blocks, or a reviewer lifted its finding with a reason.
    Verified,
}

impl GateStatus {
    pub fn title(self) -> &'static str {
        match self {
            GateStatus::Verifying => "Doğrulanıyor",
            GateStatus::Blocked => "Engellendi",
            GateStatus::Verified => "Doğrulandı",
        }
    }
}

/// One gate of one release's content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gate {
    pub kind: GateKind,
    pub status: GateStatus,
    /// The last report's time; none before the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Timestamp>,
    /// What the last finished scan found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<Reason>,
    /// Why the last scan could not finish; it runs again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver: Option<Waiver>,
}

impl Gate {
    pub(super) fn new(kind: GateKind) -> Gate {
        Gate {
            kind,
            status: GateStatus::Verifying,
            at: None,
            findings: Vec::new(),
            error: None,
            waiver: None,
        }
    }

    /// Where it is, as the author and the reviewer read it.
    pub fn detail(&self) -> &'static str {
        match self.status {
            GateStatus::Verifying if self.at.is_none() => "taranmadı",
            GateStatus::Verifying if self.error.is_some() => "tarama hata verdi; yeniden taranacak",
            GateStatus::Verifying => "taranıyor",
            GateStatus::Blocked => "engellendi",
            GateStatus::Verified if self.waiver.is_some() => {
                "doğrulandı (engel gerekçeyle kaldırıldı)"
            }
            GateStatus::Verified => "doğrulandı",
        }
    }

    /// Takes a scan's report. Only a finished, clean scan verifies; a lifted finding stays lifted while the scans
    /// find the same again, and goes when they find something else or nothing.
    pub(super) fn report(&mut self, outcome: &ScanOutcome, at: Timestamp) {
        self.at = Some(at);
        match outcome {
            ScanOutcome::Started => {
                self.status = GateStatus::Verifying;
                self.error = None;
            }
            ScanOutcome::Passed => {
                self.status = GateStatus::Verified;
                self.findings.clear();
                self.error = None;
                self.waiver = None;
            }
            ScanOutcome::Blocked(findings) => {
                self.error = None;
                if self.waiver.is_some() && self.findings == *findings {
                    self.status = GateStatus::Verified;
                } else {
                    self.status = GateStatus::Blocked;
                    self.findings = findings.clone();
                    self.waiver = None;
                }
            }
            ScanOutcome::Errored(error) => {
                self.status = GateStatus::Verifying;
                self.error = Some(if error.trim().is_empty() {
                    "neden bildirilmedi".to_owned()
                } else {
                    error.trim().to_owned()
                });
            }
        }
    }
}

/// A reviewer's reasoned lift of a gate's finding (D11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Waiver {
    pub by: String,
    pub at: Timestamp,
    pub reason: String,
}

/// What a scanner reports for one gate. "Could not run" is its own answer and never reads as a pass: a scanner that is
/// missing, a queue that is full and a time-out are all [`ScanOutcome::Errored`] (D11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanOutcome {
    /// Queued or running.
    Started,
    /// Ran to its end and found nothing that blocks.
    Passed,
    /// Ran to its end and found these, each with its code and its file or dependency.
    Blocked(Vec<Reason>),
    /// Could not run or finish, and why.
    Errored(String),
}

impl ScanOutcome {
    /// The report in a record.
    pub(super) fn words(&self) -> String {
        match self {
            ScanOutcome::Started => "başladı".to_owned(),
            ScanOutcome::Passed => "temiz".to_owned(),
            ScanOutcome::Blocked(findings) => format!(
                "engelledi: {}",
                findings
                    .iter()
                    .map(|f| f.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ScanOutcome::Errored(error) => format!("hata verdi: {}", error.trim()),
        }
    }
}

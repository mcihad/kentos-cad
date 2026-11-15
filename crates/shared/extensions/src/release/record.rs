//! What a release keeps of its review and its moves (plan/eklenti.md ADIM 22A; PY-CLOUD-11, 15, 16): the reasons it
//! was sent back with, its approval, the notice of its taking out and the record of every move.

use serde::{Deserialize, Serialize};

use super::{Action, Channel, GateKind, ReleaseContent, ReleaseState, ReviewDepth, Timestamp};
use crate::problem::Problem;

/// Why: a stable code an agent mends by, what is wrong and what to do (in Turkish), and the file or dependency it is
/// about. A gate's findings and a reviewer's requested changes are both said so (PY-CLOUD-11, D12).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reason {
    /// Stable: a gate's (`secret_found`, `path_unsafe`) or a reviewer's ready code (`capability_unused`).
    pub code: String,
    pub message: String,
    /// The file in the package it is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Where in the file: a JSON pointer or `satır 12`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// The dependency it is about: an extension's id or a Python package's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency: Option<String>,
}

impl Reason {
    pub fn new(code: &str, message: impl Into<String>) -> Reason {
        Reason {
            code: code.to_owned(),
            message: message.into(),
            file: None,
            at: None,
            dependency: None,
        }
    }
}

/// The inspector's problem as a gate's finding: the package gate runs the same check as `kentos kek verify`.
impl From<&Problem> for Reason {
    fn from(p: &Problem) -> Reason {
        Reason {
            code: p.code.to_owned(),
            message: p.message.clone(),
            file: p.file.clone(),
            at: p.at.clone(),
            dependency: None,
        }
    }
}

/// A reviewer's ready reason for sending a version back (D12): each says what to do, so an agent mends it in one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
// Snake case: the code is a stable code, written as the inspector's are.
#[serde(rename_all = "snake_case")]
pub enum ChangeCode {
    CapabilityUnused,
    CapabilityTooWide,
    UndeclaredBehaviour,
    DependencyUnneeded,
    DependencyUnknown,
    CodeUnreadable,
    ListingMisleading,
    Broken,
    /// Said in the reviewer's note, which it then needs.
    Other,
}

impl ChangeCode {
    pub const ALL: [ChangeCode; 9] = [
        ChangeCode::CapabilityUnused,
        ChangeCode::CapabilityTooWide,
        ChangeCode::UndeclaredBehaviour,
        ChangeCode::DependencyUnneeded,
        ChangeCode::DependencyUnknown,
        ChangeCode::CodeUnreadable,
        ChangeCode::ListingMisleading,
        ChangeCode::Broken,
        ChangeCode::Other,
    ];

    /// The code in a record.
    pub fn id(self) -> &'static str {
        match self {
            ChangeCode::CapabilityUnused => "capability_unused",
            ChangeCode::CapabilityTooWide => "capability_too_wide",
            ChangeCode::UndeclaredBehaviour => "undeclared_behaviour",
            ChangeCode::DependencyUnneeded => "dependency_unneeded",
            ChangeCode::DependencyUnknown => "dependency_unknown",
            ChangeCode::CodeUnreadable => "code_unreadable",
            ChangeCode::ListingMisleading => "listing_misleading",
            ChangeCode::Broken => "broken",
            ChangeCode::Other => "other",
        }
    }

    /// Its name in the reviewer's list.
    pub fn title(self) -> &'static str {
        match self {
            ChangeCode::CapabilityUnused => "Kullanılmayan yetenek",
            ChangeCode::CapabilityTooWide => "Gereğinden geniş yetenek",
            ChangeCode::UndeclaredBehaviour => "Beyan edilmeyen davranış",
            ChangeCode::DependencyUnneeded => "Gereksiz bağımlılık",
            ChangeCode::DependencyUnknown => "Tanınmayan bağımlılık",
            ChangeCode::CodeUnreadable => "Okunamayan kod",
            ChangeCode::ListingMisleading => "Yanıltıcı tanıtım",
            ChangeCode::Broken => "Çalışmayan işlev",
            ChangeCode::Other => "Diğer",
        }
    }

    /// What to do.
    pub fn fix(self) -> &'static str {
        match self {
            ChangeCode::CapabilityUnused => {
                "Kodun kullanmadığı yeteneği requestedCapabilities'ten çıkarın."
            }
            ChangeCode::CapabilityTooWide => {
                "Yeteneğin kapsamını ya da ağ adreslerini daraltın; gerekçe neden gerektiğini söylesin."
            }
            ChangeCode::UndeclaredBehaviour => {
                "Kod beyan etmediği bir şey yapıyor (ağ, dosya, alt süreç, dış yapay zekâ): yeteneğini isteyin ya da \
                 kaldırın."
            }
            ChangeCode::DependencyUnneeded => {
                "Kullanılmayan bağımlılığı pyproject.toml'dan çıkarıp “uv lock” çalıştırın."
            }
            ChangeCode::DependencyUnknown => {
                "Bağımlılık var olmayan ya da tanınmış bir paketin adını andıran bir paket: doğru paketi yazıp kilidi \
                 yenileyin."
            }
            ChangeCode::CodeUnreadable => {
                "Karartılmış, sıkıştırılmış ya da kaynağı olmadan üretilmiş kodun okunur kaynağını pakete koyun."
            }
            ChangeCode::ListingMisleading => {
                "Ad, özet, açıklama ya da simge eklentinin yaptığını söylemiyor ya da başka bir eklentiyi andırıyor; \
                 düzeltin."
            }
            ChangeCode::Broken => "Anlatılan işlev çalışmıyor; düzeltip testini ekleyin.",
            ChangeCode::Other => "İnceleyicinin notundakini yapın.",
        }
    }
}

/// A change a reviewer asks for: a ready code, the file or dependency, and the reviewer's own words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestedChange {
    pub code: ChangeCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl RequestedChange {
    pub fn new(code: ChangeCode) -> RequestedChange {
        RequestedChange {
            code,
            file: None,
            dependency: None,
            note: None,
        }
    }

    pub(super) fn reason(&self) -> Reason {
        let note = self
            .note
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty());
        let mut message = format!("{}: {}", self.code.title(), self.code.fix());
        if let Some(note) = note {
            message.push_str(" İnceleyicinin notu: ");
            message.push_str(note);
        }
        Reason {
            file: self.file.clone(),
            dependency: self.dependency.clone(),
            ..Reason::new(self.code.id(), message)
        }
    }
}

/// Why a release was sent back, and of which content (PY-CLOUD-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeRequest {
    /// The reviewer, or the scanner whose gate blocked.
    pub by: String,
    pub at: Timestamp,
    /// The content it was asked of (`sha256:…`): sending the same back takes a word why.
    pub content: String,
    /// The gate that blocked, when it was a gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateKind>,
    pub reasons: Vec<Reason>,
}

impl ChangeRequest {
    /// The way back, in Turkish: what the author does next.
    pub fn resubmission(&self) -> String {
        match self.gate {
            Some(gate) => format!(
                "{} bu içeriği engelledi. Bulguları düzeltip paketi aynı sürüm numarasıyla yeniden yükleyin ve \
                 gönderin; bulgu yanlışsa içeriği değiştirmeden, nedenini not olarak yazıp gönderin, bir inceleyici \
                 bakar.",
                gate.title()
            ),
            None => "İstenen değişiklikleri yapıp paketi aynı sürüm numarasıyla yeniden yükleyin ve gönderin; bir \
                     değişikliği yapmayacaksanız nedenini not olarak yazıp gönderin."
                .to_owned(),
        }
    }
}

/// A reviewer's approval: of this content, byte for byte.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub by: String,
    pub at: Timestamp,
    pub content: ReleaseContent,
    /// How deep the review was asked to read.
    pub depth: ReviewDepth,
}

/// Why a release is suspended, withdrawn or archived: shown to whoever would install or run it (PY-CLOUD-15).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Notice {
    pub state: ReleaseState,
    pub by: String,
    pub at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One move, as it happened (PY-CLOUD-16).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditRecord {
    pub at: Timestamp,
    /// The account.
    pub by: String,
    pub via: Channel,
    pub action: Action,
    /// None for the upload, which made the release.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<ReleaseState>,
    pub to: ReleaseState,
    /// The content it was done to (`sha256:…`).
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateKind>,
    /// What was said: the reason, the note, a gate's outcome, the review's focus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

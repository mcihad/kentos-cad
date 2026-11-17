//! What Portal dosyaları decides and offers, from the server's answers (docs/adr/9001 §3; the web portal's dossier
//! page, apps/portal/src/pages/dossier/): a step's button and its question, which step is the window's one filled
//! button, whose turn it is, what the reader must know first and a status's tone. No Iced here: the window draws
//! these and the tests read them. The names and sentences of the rest are `words.rs` and `facts.rs`.
//!
//! The server decides which steps a person may take now (`DossierDetail::actions`); nothing here adds or hides one.

use kentos_commands::{
    DossierAction, DossierDetail, DossierDirection, DossierSide, DossierStatus, PortalCatalog,
};

pub use super::facts::*;
pub use super::words::*;

/// A status's tone, as the server's catalog gives it (`neutral`, `info`, `warning`, `success`, `danger`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Info,
    Warning,
    Success,
    Danger,
}

impl Tone {
    fn of(code: &str) -> Self {
        match code {
            "info" => Tone::Info,
            "warning" => Tone::Warning,
            "success" => Tone::Success,
            "danger" => Tone::Danger,
            _ => Tone::Neutral,
        }
    }
}

/// The tone of `status` in the server's catalog; neutral when the catalog does not name it.
pub fn status_tone(catalog: Option<&PortalCatalog>, status: DossierStatus) -> Tone {
    let code = serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    catalog
        .and_then(|c| c.statuses.iter().find(|s| s.code == code))
        .map_or(Tone::Neutral, |s| Tone::of(&s.tone))
}

/// Whether the dossier is closed: decided, withdrawn, completed, declined or cancelled.
pub fn closed(status: DossierStatus) -> bool {
    matches!(
        status,
        DossierStatus::Approved
            | DossierStatus::Rejected
            | DossierStatus::Withdrawn
            | DossierStatus::Completed
            | DossierStatus::Declined
            | DossierStatus::Cancelled
    )
}

/// A step's button, in the web portal's words.
pub fn step_label(action: DossierAction, d: &DossierDetail) -> String {
    let application = d.summary.direction == DossierDirection::Application;
    match action {
        DossierAction::Send if application && d.summary.round > 0 => {
            "Düzeltilmiş başvuruyu gönder".to_owned()
        }
        DossierAction::Send if application => "Başvuruyu gönder".to_owned(),
        DossierAction::Send => "İşi gönder".to_owned(),
        DossierAction::Withdraw => "Geri çek".to_owned(),
        DossierAction::StartReview => "İncelemeye al".to_owned(),
        DossierAction::Assign => "Sorumlu ata".to_owned(),
        DossierAction::RequestChanges | DossierAction::RequestRevision => {
            "Düzeltme iste".to_owned()
        }
        DossierAction::Approve => "Onayla".to_owned(),
        DossierAction::Reject => "Reddet".to_owned(),
        DossierAction::Accept => "İşi üstlen".to_owned(),
        DossierAction::Decline => "Üstlenme".to_owned(),
        DossierAction::Deliver => "Teslim et".to_owned(),
        DossierAction::Complete => "Teslimi onayla".to_owned(),
        DossierAction::Cancel => "İptal et".to_owned(),
    }
}

/// How a step's button looks: the first step forward is the window's one filled button, a refusal is the red
/// outlined one, the rest plain (DESIGN.md §7.9: one primary button on a surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Primary,
    Danger,
    Plain,
}

const FORWARD: [DossierAction; 6] = [
    DossierAction::Send,
    DossierAction::StartReview,
    DossierAction::Accept,
    DossierAction::Deliver,
    DossierAction::Approve,
    DossierAction::Complete,
];

const STOPPING: [DossierAction; 4] = [
    DossierAction::Reject,
    DossierAction::Decline,
    DossierAction::Cancel,
    DossierAction::Withdraw,
];

/// The steps the server allows now with their looks, the filled one last, at the right edge where the eye ends.
pub fn steps(d: &DossierDetail) -> Vec<(DossierAction, Look)> {
    let primary = d.actions.iter().copied().find(|a| FORWARD.contains(a));
    let look = |a: DossierAction| {
        if Some(a) == primary {
            Look::Primary
        } else if STOPPING.contains(&a) {
            Look::Danger
        } else {
            Look::Plain
        }
    };
    let mut out: Vec<(DossierAction, Look)> = d
        .actions
        .iter()
        .copied()
        .filter(|a| Some(*a) != primary)
        .map(|a| (a, look(a)))
        .collect();
    out.extend(primary.map(|a| (a, Look::Primary)));
    out
}

/// Whether a step's note is asked, and whether it must be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Note {
    Required,
    Optional,
}

/// A step's question before it is taken: its title, what it does, the note it takes, the decision's caution, the
/// confirming button and what is said once it is taken.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub title: String,
    pub lead: Option<String>,
    pub note: Option<(Note, &'static str)>,
    pub note_hint: Option<&'static str>,
    pub caution: Option<&'static str>,
    pub confirm: &'static str,
    pub danger: bool,
    pub done: &'static str,
}

/// What the portal's decisions are and are not (docs/adr/9000 §5).
pub const DECISION_CAUTION: &str = "Bu karar portalda dosyanın kaydına geçer; ıslak ya da elektronik imzalı resmî \
    yazının yerine geçmez.";

/// A step's question; none for a step taken at once (İncelemeye al) and for Sorumlu ata, which asks its own.
pub fn ask_of(action: DossierAction, d: &DossierDetail) -> Option<Ask> {
    let application = d.summary.direction == DossierDirection::Application;
    let org = &d.organization.name;
    let plain = |title: &str, note: Option<(Note, &'static str)>, confirm, done| Ask {
        title: title.to_owned(),
        lead: None,
        note,
        note_hint: None,
        caution: None,
        confirm,
        danger: false,
        done,
    };
    Some(match action {
        DossierAction::StartReview | DossierAction::Assign => return None,
        DossierAction::Send if application => Ask {
            lead: Some(format!(
                "{org} başvuruyu ve hazırladığınız paketi görecek. Gönderilen dosyalar değiştirilmez."
            )),
            ..plain(
                &step_label(action, d),
                Some((Note::Optional, "Belediyeye not")),
                "Gönder",
                "Başvuru gönderildi.",
            )
        },
        DossierAction::Send => Ask {
            lead: Some(
                "Meslek mensubu işi ve paketi görecek; üstlenip üstlenmeyeceğini bildirecek."
                    .to_owned(),
            ),
            ..plain(
                "İşi gönder",
                Some((Note::Optional, "Meslek mensubuna not")),
                "Gönder",
                "İş gönderildi.",
            )
        },
        DossierAction::Withdraw => Ask {
            lead: Some(
                "Geri çekilen başvuru kapanır; yeniden başvurmak için yeni bir başvuru açarsınız."
                    .to_owned(),
            ),
            danger: true,
            ..plain(
                "Başvuruyu geri çek",
                Some((Note::Optional, "Belediyeye not")),
                "Geri çek",
                "Başvuru geri çekildi.",
            )
        },
        DossierAction::RequestChanges => Ask {
            lead: Some(
                "Başvuru sahibi düzeltilmiş başvuruyu yeni bir sürüm olarak gönderir.".to_owned(),
            ),
            note_hint: Some(
                "İmar Kanunu m.22: eksik ve yanlışlar bütün olarak, bir defada yazılır.",
            ),
            ..plain(
                "Düzeltme iste",
                Some((Note::Required, "Eksik ve yanlışlar")),
                "Düzeltme iste",
                "Düzeltme istendi.",
            )
        },
        DossierAction::Approve => Ask {
            caution: Some(DECISION_CAUTION),
            ..plain(
                "Başvuruyu onayla",
                Some((Note::Optional, "Karar notu")),
                "Onayla",
                "Başvuru onaylandı.",
            )
        },
        DossierAction::Reject => Ask {
            caution: Some(DECISION_CAUTION),
            danger: true,
            ..plain(
                "Başvuruyu reddet",
                Some((Note::Required, "Ret gerekçesi")),
                "Reddet",
                "Başvuru reddedildi.",
            )
        },
        DossierAction::Accept => Ask {
            lead: Some(format!(
                "{org} işi üstlendiğinizi görecek. Teslim etmek için hazırladığınız projeyi ve belgeleri pakete \
                 ekleyip Teslim et'e basarsınız."
            )),
            ..plain(
                "İşi üstlen",
                Some((Note::Optional, "Belediyeye not")),
                "Üstlen",
                "İşi üstlendiniz.",
            )
        },
        DossierAction::Decline => Ask {
            danger: true,
            ..plain(
                "İşi üstlenme",
                Some((Note::Required, "Neden")),
                "Üstlenme",
                "İşi üstlenmediğiniz bildirildi.",
            )
        },
        DossierAction::Deliver => Ask {
            lead: Some(
                "Hazırladığınız paket teslim olarak gider; gönderilen dosyalar değiştirilmez."
                    .to_owned(),
            ),
            ..plain(
                "İşi teslim et",
                Some((Note::Optional, "Teslim notu")),
                "Teslim et",
                "İş teslim edildi.",
            )
        },
        DossierAction::RequestRevision => plain(
            "Teslimde düzeltme iste",
            Some((Note::Required, "Düzeltilecekler")),
            "Düzeltme iste",
            "Düzeltme istendi.",
        ),
        DossierAction::Complete => Ask {
            lead: Some("İş tamamlanır ve dosya kapanır.".to_owned()),
            caution: Some(DECISION_CAUTION),
            ..plain(
                "Teslimi onayla",
                Some((Note::Optional, "Karar notu")),
                "Onayla",
                "Teslim onaylandı; iş tamamlandı.",
            )
        },
        DossierAction::Cancel => Ask {
            danger: true,
            ..plain(
                "Aktarımı iptal et",
                Some((Note::Required, "İptal nedeni")),
                "İptal et",
                "Aktarım iptal edildi.",
            )
        },
    })
}

/// Why a required note stops a step: what is missing and how to go on.
pub fn note_missing(label: &str) -> String {
    format!("“{label}” yazılmadan bu adım atılamaz; kısaca yazıp yeniden deneyin.")
}

/// Whose turn it is, in a sentence.
pub fn waiting(d: &DossierDetail) -> &'static str {
    let municipality = d.side == DossierSide::Municipality;
    if closed(d.summary.status) {
        "Dosya kapandı."
    } else if d.summary.awaiting_me {
        if municipality {
            "Sıra belediyenizde."
        } else {
            "Sıra sizde."
        }
    } else if municipality {
        "Sıra meslek mensubunda."
    } else {
        "Sıra belediyede."
    }
}

/// A notice the reader should see before anything else: its tone, its sentence and the words behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    pub tone: Tone,
    pub title: String,
    pub body: Option<String>,
}

/// What the reader must know first: a draft's, a correction asked for, a decision.
pub fn notices(d: &DossierDetail) -> Vec<Notice> {
    let s = &d.summary;
    let last = |kind: &str| {
        d.messages
            .iter()
            .rev()
            .find(|m| m.kind == kind)
            .map(|m| m.body.clone())
    };
    let professional = d.side == DossierSide::Professional;
    let mut out = Vec::new();
    match s.status {
        DossierStatus::Draft => out.push(Notice {
            tone: Tone::Info,
            title: if professional {
                "Bu bir taslak; belediye henüz görmüyor.".to_owned()
            } else {
                "Bu bir taslak; meslek mensubu henüz görmüyor.".to_owned()
            },
            body: Some(if professional {
                "Paketi hazırlayın, belge listesine bakın ve başvuruyu gönderin.".to_owned()
            } else {
                "Aktaracağınız projeyi ve belgeleri pakete ekleyip işi gönderin.".to_owned()
            }),
        }),
        DossierStatus::ChangesRequested | DossierStatus::RevisionRequested => out.push(Notice {
            tone: Tone::Warning,
            title: if professional {
                "Belediye düzeltme istedi.".to_owned()
            } else {
                "Düzeltme istendi; meslek mensubunun yeni sürümü bekleniyor.".to_owned()
            },
            body: last("deficiency"),
        }),
        DossierStatus::Approved | DossierStatus::Completed => out.push(Notice {
            tone: Tone::Success,
            title: if s.status == DossierStatus::Approved {
                "Başvuru onaylandı.".to_owned()
            } else {
                "İş tamamlandı.".to_owned()
            },
            body: last("decision"),
        }),
        DossierStatus::Rejected | DossierStatus::Declined | DossierStatus::Cancelled => {
            out.push(Notice {
                tone: Tone::Danger,
                title: format!("{}.", s.status_label),
                body: last("decision"),
            });
        }
        _ => {}
    }
    out
}

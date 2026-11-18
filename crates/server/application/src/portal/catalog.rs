//! The portal's catalog (docs/adr/9000 §4): the kinds of dossiers with the documents they are usually sent with and
//! the legal deadline some of them run under, the documents, professions and statuses, each with its Turkish name. The
//! server words them in one place; the clients show what it sends (`GET /v1/portal/catalog`).
//!
//! Only two kinds name a law: a building permit's documents and deadlines are İmar Kanunu (3194) m.22's, an occupancy
//! permit's deadline m.30's. Every other document list is the portal's suggestion and says so. The municipality decides
//! what it needs; a missing document is pointed out, never refused here.

use kentos_contracts::{
    DocTypeOption, DossierDirection, DossierStatus, KindOption, PortalCatalog, Profession,
    ProfessionOption, ProfessionalStatus, StatusOption,
};

/// A legal deadline a kind runs under while the municipality has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deadline {
    /// 3194 m.22: deficiencies within 15 days, all at once in writing; a complete one decided within 30; after the
    /// deficiencies are remedied, within 15 of the new application.
    BuildingPermit,
    /// 3194 m.30: decided within 30 days, else the permit counts as given.
    OccupancyPermit,
}

pub struct Kind {
    pub code: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub documents: &'static [&'static str],
    pub documents_basis: Option<&'static str>,
    pub deadline: Option<Deadline>,
}

const SUGGESTED: Option<&str> = Some("Önerilen belgeler: belediye gerekeni ayrıca isteyebilir.");

pub const APPLICATION_KINDS: &[Kind] = &[
    Kind {
        code: "building_permit",
        label: "Yapı ruhsatı",
        description: "Yapı ruhsatı başvurusu, projeleri ve belgeleriyle.",
        documents: &[
            "title_deed",
            "architectural",
            "structural",
            "electrical",
            "mechanical",
            "calculations",
            "site_sketch",
        ],
        documents_basis: Some("İmar Kanunu m.22'de sayılan belgeler."),
        deadline: Some(Deadline::BuildingPermit),
    },
    Kind {
        code: "architectural_project",
        label: "Mimari proje",
        description: "Mimari projenin incelenmesi.",
        documents: &["architectural", "kentos_project"],
        documents_basis: SUGGESTED,
        deadline: None,
    },
    Kind {
        code: "structural_project",
        label: "Statik proje",
        description: "Taşıyıcı sistem projesinin incelenmesi.",
        documents: &["structural", "calculations", "soil_report"],
        documents_basis: SUGGESTED,
        deadline: None,
    },
    Kind {
        code: "mechanical_project",
        label: "Mekanik tesisat projesi",
        description: "Mekanik tesisat projesinin incelenmesi.",
        documents: &["mechanical", "calculations"],
        documents_basis: SUGGESTED,
        deadline: None,
    },
    Kind {
        code: "electrical_project",
        label: "Elektrik projesi",
        description: "Elektrik tesisatı projesinin incelenmesi.",
        documents: &["electrical", "calculations"],
        documents_basis: SUGGESTED,
        deadline: None,
    },
    Kind {
        code: "occupancy_permit",
        label: "Yapı kullanma izni",
        description: "Biten yapının ya da bölümünün kullanılması için izin.",
        documents: &[],
        documents_basis: None,
        deadline: Some(Deadline::OccupancyPermit),
    },
    Kind {
        code: "application_sketch",
        label: "Aplikasyon krokisi",
        description: "Parsel sınırlarının zemine aplikasyonunun krokisi.",
        documents: &["application_sketch", "kentos_project"],
        documents_basis: SUGGESTED,
        deadline: None,
    },
    Kind {
        code: "site_sketch",
        label: "Röperli kroki",
        description: "Yapının parseldeki yerini röperleriyle gösteren kroki.",
        documents: &["site_sketch"],
        documents_basis: SUGGESTED,
        deadline: None,
    },
    Kind {
        code: "subdivision",
        label: "İfraz / tevhit",
        description: "Parselin bölünmesi ya da parsellerin birleştirilmesi (İmar Kanunu m.15).",
        documents: &["subdivision_map", "title_deed", "kentos_project"],
        documents_basis: SUGGESTED,
        deadline: None,
    },
    Kind {
        code: "plan_amendment",
        label: "Plan değişikliği teklifi",
        description: "İmar planında değişiklik teklifi (İmar Kanunu m.8).",
        documents: &["plan_report", "kentos_project"],
        documents_basis: SUGGESTED,
        deadline: None,
    },
    Kind {
        code: "other",
        label: "Diğer",
        description: "Başka bir başvuru.",
        documents: &[],
        documents_basis: None,
        deadline: None,
    },
];

pub const TRANSFER_KINDS: &[Kind] = &[
    Kind {
        code: "survey",
        label: "Ölçü ve aplikasyon işi",
        description: "Arazide ölçü, aplikasyon ya da röperleme.",
        documents: &[],
        documents_basis: None,
        deadline: None,
    },
    Kind {
        code: "project_preparation",
        label: "Proje hazırlama",
        description: "Verilen altlık üzerinde projenin hazırlanması.",
        documents: &[],
        documents_basis: None,
        deadline: None,
    },
    Kind {
        code: "project_revision",
        label: "Proje düzeltme",
        description: "Projenin istenen noktalarda düzeltilmesi.",
        documents: &[],
        documents_basis: None,
        deadline: None,
    },
    Kind {
        code: "technical_review",
        label: "Teknik görüş",
        description: "Projeye ya da çalışmaya teknik görüş.",
        documents: &[],
        documents_basis: None,
        deadline: None,
    },
    Kind {
        code: "other",
        label: "Diğer",
        description: "Başka bir iş.",
        documents: &[],
        documents_basis: None,
        deadline: None,
    },
];

pub const DOC_TYPES: &[(&str, &str)] = &[
    ("kentos_project", "KentOS projesi (KCAD)"),
    ("title_deed", "Tapu ya da tapu yerine geçen belge"),
    ("architectural", "Mimari proje"),
    ("structural", "Statik proje"),
    ("electrical", "Elektrik projesi"),
    ("mechanical", "Mekanik tesisat projesi"),
    ("calculations", "Resim ve hesaplar"),
    ("site_sketch", "Röperli kroki (yoksa ebatlı kroki)"),
    ("application_sketch", "Aplikasyon krokisi"),
    ("soil_report", "Zemin etüdü raporu"),
    ("subdivision_map", "İfraz / tevhit haritası"),
    ("plan_report", "Plan açıklama raporu"),
    ("zoning_status", "İmar durumu belgesi"),
    ("decision_letter", "Karar yazısı"),
    ("other", "Diğer belge"),
];

pub fn kinds(direction: DossierDirection) -> &'static [Kind] {
    match direction {
        DossierDirection::Application => APPLICATION_KINDS,
        DossierDirection::Transfer => TRANSFER_KINDS,
    }
}

pub fn kind(direction: DossierDirection, code: &str) -> Option<&'static Kind> {
    kinds(direction).iter().find(|k| k.code == code)
}

pub fn kind_label(direction: DossierDirection, code: &str) -> String {
    kind(direction, code).map_or_else(|| code.to_owned(), |k| k.label.to_owned())
}

pub fn is_doc_type(code: &str) -> bool {
    DOC_TYPES.iter().any(|(c, _)| *c == code)
}

pub fn doc_type_label(code: &str) -> String {
    DOC_TYPES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or_else(|| code.to_owned(), |(_, l)| (*l).to_owned())
}

pub fn status_code(status: DossierStatus) -> &'static str {
    match status {
        DossierStatus::Draft => "draft",
        DossierStatus::Submitted => "submitted",
        DossierStatus::InReview => "in_review",
        DossierStatus::ChangesRequested => "changes_requested",
        DossierStatus::Approved => "approved",
        DossierStatus::Rejected => "rejected",
        DossierStatus::Withdrawn => "withdrawn",
        DossierStatus::Sent => "sent",
        DossierStatus::Accepted => "accepted",
        DossierStatus::Declined => "declined",
        DossierStatus::Delivered => "delivered",
        DossierStatus::RevisionRequested => "revision_requested",
        DossierStatus::Completed => "completed",
        DossierStatus::Cancelled => "cancelled",
    }
}

pub const STATUSES: [DossierStatus; 14] = [
    DossierStatus::Draft,
    DossierStatus::Submitted,
    DossierStatus::InReview,
    DossierStatus::ChangesRequested,
    DossierStatus::Approved,
    DossierStatus::Rejected,
    DossierStatus::Withdrawn,
    DossierStatus::Sent,
    DossierStatus::Accepted,
    DossierStatus::Declined,
    DossierStatus::Delivered,
    DossierStatus::RevisionRequested,
    DossierStatus::Completed,
    DossierStatus::Cancelled,
];

pub fn status_of(code: &str) -> Option<DossierStatus> {
    STATUSES.iter().copied().find(|s| status_code(*s) == code)
}

pub fn status_label(status: DossierStatus) -> &'static str {
    match status {
        DossierStatus::Draft => "Taslak",
        DossierStatus::Submitted => "Gönderildi",
        DossierStatus::InReview => "İncelemede",
        DossierStatus::ChangesRequested => "Düzeltme istendi",
        DossierStatus::Approved => "Onaylandı",
        DossierStatus::Rejected => "Reddedildi",
        DossierStatus::Withdrawn => "Geri çekildi",
        DossierStatus::Sent => "Yanıt bekleniyor",
        DossierStatus::Accepted => "Üstlenildi",
        DossierStatus::Declined => "Üstlenilmedi",
        DossierStatus::Delivered => "Teslim edildi",
        DossierStatus::RevisionRequested => "Düzeltme istendi",
        DossierStatus::Completed => "Tamamlandı",
        DossierStatus::Cancelled => "İptal edildi",
    }
}

fn status_tone(status: DossierStatus) -> &'static str {
    match status {
        DossierStatus::Draft | DossierStatus::Withdrawn | DossierStatus::Cancelled => "neutral",
        DossierStatus::Submitted
        | DossierStatus::InReview
        | DossierStatus::Sent
        | DossierStatus::Accepted
        | DossierStatus::Delivered => "info",
        DossierStatus::ChangesRequested | DossierStatus::RevisionRequested => "warning",
        DossierStatus::Approved | DossierStatus::Completed => "success",
        DossierStatus::Rejected | DossierStatus::Declined => "danger",
    }
}

pub fn direction_code(direction: DossierDirection) -> &'static str {
    match direction {
        DossierDirection::Application => "application",
        DossierDirection::Transfer => "transfer",
    }
}

pub fn direction_of(code: &str) -> Option<DossierDirection> {
    match code {
        "application" => Some(DossierDirection::Application),
        "transfer" => Some(DossierDirection::Transfer),
        _ => None,
    }
}

pub const PROFESSIONS: [Profession; 8] = [
    Profession::Architect,
    Profession::Surveyor,
    Profession::CivilEngineer,
    Profession::MechanicalEngineer,
    Profession::ElectricalEngineer,
    Profession::UrbanPlanner,
    Profession::LandscapeArchitect,
    Profession::GeologicalEngineer,
];

pub fn profession_code(p: Profession) -> &'static str {
    match p {
        Profession::Architect => "architect",
        Profession::Surveyor => "surveyor",
        Profession::CivilEngineer => "civil_engineer",
        Profession::MechanicalEngineer => "mechanical_engineer",
        Profession::ElectricalEngineer => "electrical_engineer",
        Profession::UrbanPlanner => "urban_planner",
        Profession::LandscapeArchitect => "landscape_architect",
        Profession::GeologicalEngineer => "geological_engineer",
    }
}

pub fn profession_of(code: &str) -> Option<Profession> {
    PROFESSIONS
        .iter()
        .copied()
        .find(|p| profession_code(*p) == code)
}

pub fn profession_label(p: Profession) -> &'static str {
    match p {
        Profession::Architect => "Mimar",
        Profession::Surveyor => "Harita mühendisi",
        Profession::CivilEngineer => "İnşaat mühendisi",
        Profession::MechanicalEngineer => "Makine mühendisi",
        Profession::ElectricalEngineer => "Elektrik mühendisi",
        Profession::UrbanPlanner => "Şehir plancısı",
        Profession::LandscapeArchitect => "Peyzaj mimarı",
        Profession::GeologicalEngineer => "Jeoloji mühendisi",
    }
}

/// The chamber (TMMOB) a profession registers with.
pub fn chamber(p: Profession) -> &'static str {
    match p {
        Profession::Architect => "Mimarlar Odası",
        Profession::Surveyor => "Harita ve Kadastro Mühendisleri Odası",
        Profession::CivilEngineer => "İnşaat Mühendisleri Odası",
        Profession::MechanicalEngineer => "Makina Mühendisleri Odası",
        Profession::ElectricalEngineer => "Elektrik Mühendisleri Odası",
        Profession::UrbanPlanner => "Şehir Plancıları Odası",
        Profession::LandscapeArchitect => "Peyzaj Mimarları Odası",
        Profession::GeologicalEngineer => "Jeoloji Mühendisleri Odası",
    }
}

pub fn professional_status_code(s: ProfessionalStatus) -> &'static str {
    match s {
        ProfessionalStatus::Pending => "pending",
        ProfessionalStatus::Verified => "verified",
        ProfessionalStatus::Rejected => "rejected",
        ProfessionalStatus::Suspended => "suspended",
    }
}

pub fn professional_status_of(code: &str) -> Option<ProfessionalStatus> {
    [
        ProfessionalStatus::Pending,
        ProfessionalStatus::Verified,
        ProfessionalStatus::Rejected,
        ProfessionalStatus::Suspended,
    ]
    .into_iter()
    .find(|s| professional_status_code(*s) == code)
}

fn kind_option(k: &Kind) -> KindOption {
    KindOption {
        code: k.code.into(),
        label: k.label.into(),
        description: k.description.into(),
        documents: k
            .documents
            .iter()
            .map(|d| DocTypeOption {
                code: (*d).into(),
                label: doc_type_label(d),
            })
            .collect(),
        documents_basis: k.documents_basis.map(Into::into),
        deadline: k.deadline.map(|d| deadline_sentence(d).into()),
    }
}

pub(crate) fn deadline_sentence(d: Deadline) -> &'static str {
    match d {
        Deadline::BuildingPermit => {
            "Eksiksiz başvuru en geç 30 günde sonuçlanır; eksik ve yanlışlar 15 gün içinde bütün olarak yazıyla \
             bildirilir, giderildikten sonra 15 günde karar verilir (İmar Kanunu m.22)."
        }
        Deadline::OccupancyPermit => {
            "Başvuru en geç 30 günde sonuçlandırılır; aksi hâlde izin verilmiş sayılır (İmar Kanunu m.30)."
        }
    }
}

pub fn catalog() -> PortalCatalog {
    PortalCatalog {
        application_kinds: APPLICATION_KINDS.iter().map(kind_option).collect(),
        transfer_kinds: TRANSFER_KINDS.iter().map(kind_option).collect(),
        doc_types: DOC_TYPES
            .iter()
            .map(|(code, label)| DocTypeOption {
                code: (*code).into(),
                label: (*label).into(),
            })
            .collect(),
        professions: PROFESSIONS
            .iter()
            .map(|p| ProfessionOption {
                code: *p,
                label: profession_label(*p).into(),
                chamber: chamber(*p).into(),
            })
            .collect(),
        statuses: STATUSES
            .iter()
            .map(|s| StatusOption {
                code: status_code(*s).into(),
                label: status_label(*s).into(),
                tone: status_tone(*s).into(),
            })
            .collect(),
        professional_statuses: [
            (ProfessionalStatus::Pending, "Doğrulama bekliyor", "warning"),
            (ProfessionalStatus::Verified, "Doğrulandı", "success"),
            (ProfessionalStatus::Rejected, "Onaylanmadı", "danger"),
            (ProfessionalStatus::Suspended, "Askıya alındı", "neutral"),
        ]
        .into_iter()
        .map(|(s, label, tone)| StatusOption {
            code: professional_status_code(s).into(),
            label: label.into(),
            tone: tone.into(),
        })
        .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_document_a_kind_names_is_in_the_list() {
        for k in APPLICATION_KINDS.iter().chain(TRANSFER_KINDS) {
            for d in k.documents {
                assert!(is_doc_type(d), "{}: {d}", k.code);
            }
        }
        for s in STATUSES {
            assert_eq!(status_of(status_code(s)), Some(s));
        }
        for p in PROFESSIONS {
            assert_eq!(profession_of(profession_code(p)), Some(p));
        }
    }
}

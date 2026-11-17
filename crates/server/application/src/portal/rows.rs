//! The portal's rows as the contracts show them: a dossier's summary, a file, a professional. Shared by the lists and
//! the pages so both read the same columns the same way.

use kentos_contracts::{
    DossierDirection, DossierFileView, DossierSide, DossierSite, DossierStatus, DossierSummary,
    FileSource, ProfessionalStatus, ProfessionalView,
};
use sqlx::types::Json;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::{catalog, flow, rfc3339, turkey};

/// Every summary query selects these columns, in this shape, from `kentos.dossier d`.
pub(crate) const SUMMARY_SELECT: &str = "select d.id, d.direction, d.year, d.number, d.kind, d.title, d.status,
        d.tenant_id, t.slug as tenant_slug, t.name as tenant_name, d.professional_id,
        u.display_name as professional_name, p.profession, d.assignee_id, d.assignee_name, d.round, d.site,
        d.created_at, d.updated_at, d.sent_at, d.round_sent_at, d.due_on
   from kentos.dossier d
   left join kentos.tenant t on t.id = d.tenant_id
   left join kentos.app_user u on u.id = d.professional_id
   left join kentos.professional p on p.user_id = d.professional_id";

#[derive(sqlx::FromRow)]
pub(crate) struct SummaryRow {
    pub id: Uuid,
    pub direction: String,
    pub year: Option<i32>,
    pub number: Option<i32>,
    pub kind: String,
    pub title: String,
    pub status: String,
    pub tenant_id: Uuid,
    pub tenant_slug: Option<String>,
    pub tenant_name: Option<String>,
    pub professional_id: Uuid,
    pub professional_name: Option<String>,
    pub profession: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub assignee_name: Option<String>,
    pub round: i32,
    pub site: Json<DossierSite>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub sent_at: Option<OffsetDateTime>,
    pub round_sent_at: Option<OffsetDateTime>,
    pub due_on: Option<Date>,
}

impl SummaryRow {
    pub fn direction(&self) -> DossierDirection {
        catalog::direction_of(&self.direction).unwrap_or(DossierDirection::Application)
    }

    pub fn status(&self) -> DossierStatus {
        catalog::status_of(&self.status).unwrap_or(DossierStatus::Draft)
    }

    pub fn round(&self) -> u32 {
        u32::try_from(self.round).unwrap_or(0)
    }

    /// "2026/0007" once it was sent.
    pub fn number(&self) -> Option<String> {
        Some(format!("{}/{:04}", self.year?, self.number?))
    }

    /// As the caller on `side` sees it, on `today`.
    pub fn view(&self, side: Option<DossierSide>, today: Date) -> DossierSummary {
        let direction = self.direction();
        let status = self.status();
        DossierSummary {
            id: self.id.to_string(),
            direction,
            number: self.number(),
            kind: self.kind.clone(),
            kind_label: catalog::kind_label(direction, &self.kind),
            title: self.title.clone(),
            status,
            status_label: catalog::status_label(status).into(),
            tenant_id: self.tenant_id.to_string(),
            tenant_slug: self.tenant_slug.clone().unwrap_or_default(),
            tenant_name: self.tenant_name.clone().unwrap_or_else(|| "—".into()),
            professional_id: self.professional_id.to_string(),
            professional_name: self.professional_name.clone().unwrap_or_else(|| "—".into()),
            profession: self.profession.as_deref().and_then(catalog::profession_of),
            assignee_name: self.assignee_name.clone(),
            round: self.round(),
            site: self.site.0.clone(),
            created_at: rfc3339(self.created_at),
            updated_at: rfc3339(self.updated_at),
            sent_at: self.sent_at.map(rfc3339),
            due_on: self.due_on.map(|d| d.to_string()),
            clock: super::clock::clock(
                direction,
                &self.kind,
                status,
                self.round(),
                self.round_sent_at.map(|t| t.to_offset(turkey()).date()),
                today,
            ),
            awaiting_me: side.is_some() && flow::waiting_for(direction, status) == side,
        }
    }
}

pub(crate) const FILE_SELECT: &str =
    "select f.id, f.side, f.purpose, f.round, f.message_id, f.state, f.doc_type,
        f.name, f.media_type, f.size, f.sha256, f.added_by_name, f.added_at, f.source, f.blob_key, f.scan, f.scan_note
   from kentos.dossier_file f";

#[derive(sqlx::FromRow)]
pub(crate) struct FileRow {
    pub id: Uuid,
    pub side: String,
    pub purpose: String,
    pub round: Option<i32>,
    pub message_id: Option<Uuid>,
    pub state: String,
    pub doc_type: String,
    pub name: String,
    pub media_type: String,
    pub size: i64,
    pub sha256: String,
    pub added_by_name: String,
    pub added_at: OffsetDateTime,
    pub source: Option<Json<FileSource>>,
    pub blob_key: String,
    pub scan: String,
    pub scan_note: Option<String>,
}

pub(crate) fn side_of(text: &str) -> DossierSide {
    if text == "municipality" {
        DossierSide::Municipality
    } else {
        DossierSide::Professional
    }
}

pub(crate) fn side_code(side: DossierSide) -> &'static str {
    match side {
        DossierSide::Municipality => "municipality",
        DossierSide::Professional => "professional",
    }
}

/// A KentOS project file: by its kind of document or its name.
pub(crate) fn is_project_file(name: &str, doc_type: &str) -> bool {
    doc_type == "kentos_project" || name.to_lowercase().ends_with(".kcad")
}

impl FileRow {
    pub fn view(&self) -> DossierFileView {
        DossierFileView {
            id: self.id.to_string(),
            side: side_of(&self.side),
            purpose: self.purpose.clone(),
            round: self.round.and_then(|r| u32::try_from(r).ok()),
            state: self.state.clone(),
            doc_type: self.doc_type.clone(),
            doc_type_label: catalog::doc_type_label(&self.doc_type),
            name: self.name.clone(),
            media_type: self.media_type.clone(),
            size: u64::try_from(self.size).unwrap_or(0),
            sha256: self.sha256.clone(),
            added_by_name: self.added_by_name.clone(),
            added_at: rfc3339(self.added_at),
            source: self.source.as_ref().map(|s| s.0.clone()),
            is_project: is_project_file(&self.name, &self.doc_type),
            scan: kentos_contracts::ScanState::of(&self.scan),
            scan_note: self.scan_note.clone(),
        }
    }
}

pub(crate) const PROFESSIONAL_SELECT: &str = "select p.user_id, u.display_name, u.email, p.profession, p.chamber_no,
        p.office_name, p.phone, p.province, p.listed, p.status, p.status_note, p.created_at, p.reviewed_at
   from kentos.professional p join kentos.app_user u on u.id = p.user_id";

#[derive(sqlx::FromRow)]
pub(crate) struct ProfessionalRow {
    pub user_id: Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub profession: String,
    pub chamber_no: String,
    pub office_name: Option<String>,
    pub phone: Option<String>,
    pub province: Option<String>,
    pub listed: bool,
    pub status: String,
    pub status_note: String,
    pub created_at: OffsetDateTime,
    pub reviewed_at: Option<OffsetDateTime>,
}

impl ProfessionalRow {
    pub fn status(&self) -> ProfessionalStatus {
        catalog::professional_status_of(&self.status).unwrap_or(ProfessionalStatus::Pending)
    }

    pub fn view(&self) -> ProfessionalView {
        ProfessionalView {
            user_id: self.user_id.to_string(),
            display_name: self.display_name.clone(),
            email: self.email.clone(),
            profession: catalog::profession_of(&self.profession)
                .unwrap_or(kentos_contracts::Profession::Architect),
            chamber_no: self.chamber_no.clone(),
            office_name: self.office_name.clone(),
            phone: self.phone.clone(),
            province: self.province.clone(),
            listed: self.listed,
            status: self.status(),
            status_note: self.status_note.clone(),
            created_at: rfc3339(self.created_at),
            reviewed_at: self.reviewed_at.map(rfc3339),
        }
    }
}

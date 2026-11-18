//! One dossier's page (docs/adr/9000 §5): its summary and legal deadline, the packages sent round by round, the one
//! being prepared, messages with their attachments, the timeline in sentences, the documents its kind is usually sent
//! with, both sides' particulars and the steps the caller may take now.

use std::collections::BTreeMap;

use kentos_contracts::{
    AssigneeOption, DocCheckItem, DocChecklist, DossierDetail, DossierDirection, DossierEventView,
    DossierMessageView, DossierOrganization, DossierRound, DossierSide,
};
use kentos_postgres::Db;
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use super::accounts::user_scope;
use super::dossiers::{Held, hold};
use super::flow::{self, Party};
use super::rows::{FILE_SELECT, FileRow, PROFESSIONAL_SELECT, ProfessionalRow, side_of};
use super::{catalog, rfc3339, today};
use crate::error::AppResult;
use crate::identity::Actor;

/// `GET /v1/dossiers/{id}`.
pub async fn detail(db: &Db, actor: &Actor, id: Uuid) -> AppResult<DossierDetail> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, false).await?;
    let detail = assemble(&mut tx, &held).await?;
    tx.commit().await?;
    Ok(detail)
}

/// What a step of the timeline says.
pub(crate) fn event_text(
    direction: DossierDirection,
    action: &str,
    actor: &str,
    round: u32,
    detail: &serde_json::Value,
) -> String {
    let files = detail
        .get("files")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let text = |k: &str| {
        detail
            .get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let application = direction == DossierDirection::Application;
    match action {
        "office_shared" => format!("{actor} dosyayı {} ile paylaştı; büronun meslektaşları da çalışır.", text("office")),
        "office_unshared" => format!("{actor} dosyanın {} ile paylaşımını kaldırdı.", text("office")),
        "created" if application => format!("{actor} başvuruyu taslak olarak açtı."),
        "created" => format!("{actor} aktarımı taslak olarak hazırlamaya başladı."),
        "send" if application && round <= 1 => {
            format!("{actor} başvuruyu gönderdi ({files} dosya).")
        }
        "send" if application => {
            format!("{actor} düzeltilmiş başvuruyu gönderdi ({round}. sürüm, {files} dosya).")
        }
        "send" if files > 0 => format!("{actor} projeyi aktardı ({files} dosya)."),
        "send" => format!("{actor} işi gönderdi."),
        "withdraw" => format!("{actor} başvuruyu geri çekti."),
        "start_review" => format!("{actor} başvuruyu incelemeye aldı."),
        "assign" => format!("Sorumlu: {} ({actor} belirledi).", text("assignee")),
        "request_changes" => format!("{actor} düzeltme istedi."),
        "approve" => format!("{actor} başvuruyu onayladı."),
        "reject" => format!("{actor} başvuruyu reddetti."),
        "accept" => format!("{actor} işi üstlendi."),
        "decline" => format!("{actor} işi üstlenmedi."),
        "deliver" => format!(
            "{actor} işi teslim etti ({}. teslim, {files} dosya).",
            round.saturating_sub(1).max(1)
        ),
        "request_revision" => format!("{actor} teslimde düzeltme istedi."),
        "complete" => format!("{actor} teslimi onayladı; iş tamamlandı."),
        "cancel" => format!("{actor} aktarımı iptal etti."),
        "edited" => format!("{actor} dosyanın bilgilerini değiştirdi."),
        "imported" => format!("{actor}, “{}” dosyasını proje olarak açtı.", text("file")),
        "fee_issued" => format!("{actor} harç ve ücret tahakkuk ettirdi: {}.", text("total")),
        "fee_paid" => format!("{actor} {} tutarındaki ödemeyi kaydetti (makbuz {}).", text("total"), text("receipt")),
        "fee_cancelled" if detail.get("wasPaid").and_then(serde_json::Value::as_bool) == Some(true) => format!(
            "{actor} ödenmiş {} tutarındaki tahakkuku iptal etti: {} (iadesi veznenin işi).",
            text("total"),
            text("reason")
        ),
        "fee_cancelled" => format!("{actor} {} tutarındaki tahakkuku iptal etti: {}.", text("total"), text("reason")),
        "live_project_linked" => format!("{actor} işi kurumun canlı projesine bağladı; üstlenen düzenleyici olarak çalışır."),
        "live_project_unlinked" => format!("{actor} canlı proje bağlantısını kaldırdı."),
        "signed" => format!(
            "{actor} {} dosyayı e-imzaladı ({}).",
            detail.get("files").and_then(serde_json::Value::as_u64).unwrap_or(0),
            text("subject")
        ),
        other => format!("{actor}: {other}"),
    }
}

type MessageRow = (Uuid, i32, String, String, String, String, OffsetDateTime);
type EventRow = (
    i64,
    OffsetDateTime,
    String,
    String,
    String,
    i32,
    Json<serde_json::Value>,
);
type OrganizationRow = (
    Uuid,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn round_of(n: i32) -> u32 {
    u32::try_from(n).unwrap_or(0)
}

/// The page of the dossier the caller holds, in the open transaction.
pub(crate) async fn assemble(
    tx: &mut Transaction<'static, Postgres>,
    held: &Held,
) -> AppResult<DossierDetail> {
    let id = held.id();
    let (direction, status, party) = (held.direction(), held.status(), held.party);
    let side = party.side();
    let acts = held.may_act();
    let files: Vec<FileRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{FILE_SELECT} where f.dossier_id = $1 order by f.round nulls last, f.added_at"
    )))
    .bind(id)
    .fetch_all(&mut **tx)
    .await?;
    let messages: Vec<MessageRow> = sqlx::query_as(
        "select id, round, side, kind, author_name, body, at from kentos.dossier_message
          where dossier_id = $1 order by at, id",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await?;
    let events: Vec<EventRow> = sqlx::query_as(
        "select id, at, actor_name, side, action, round, detail from kentos.dossier_event
          where dossier_id = $1 order by id",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await?;
    let live: Option<(Uuid, Uuid, String, bool)> =
        sqlx::query_as("select tenant_id, project_id, name, granted from kentos.dossier_live_project($1)")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    let fee_count: i64 =
        sqlx::query_scalar("select count(*) from kentos.dossier_fee where dossier_id = $1 and state <> 'cancelled'")
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;
    // Each round's sending: the step that sent it.
    let sent_at: BTreeMap<u32, String> = events
        .iter()
        .filter(|e| matches!(e.4.as_str(), "send" | "deliver"))
        .map(|e| (round_of(e.5), rfc3339(e.1)))
        .collect();
    let mut rounds: BTreeMap<u32, DossierRound> = BTreeMap::new();
    let mut pending = Vec::new();
    let mut attachments: BTreeMap<Uuid, Vec<_>> = BTreeMap::new();
    for f in &files {
        match (f.purpose.as_str(), f.state.as_str(), f.round, f.message_id) {
            ("package", "final", Some(r), _) => {
                let r = round_of(r);
                rounds
                    .entry(r)
                    .or_insert_with(|| DossierRound {
                        round: r,
                        side: side_of(&f.side),
                        sent_at: sent_at.get(&r).cloned(),
                        files: Vec::new(),
                    })
                    .files
                    .push(f.view());
            }
            ("package", _, _, _) => pending.push(f.view()),
            (_, "final", _, Some(m)) => attachments.entry(m).or_default().push(f.view()),
            _ => {}
        }
    }
    let professional: Option<ProfessionalRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{PROFESSIONAL_SELECT} where p.user_id = $1"
    )))
    .bind(held.row.professional_id)
    .fetch_optional(&mut **tx)
    .await?;
    let (tenant_id, slug, name, province, district, contact_email, contact_phone): OrganizationRow = sqlx::query_as(
        "select t.id, t.slug, t.name, t.province, t.district, tp.contact_email, tp.contact_phone
           from kentos.tenant t left join kentos.tenant_portal tp on tp.tenant_id = t.id where t.id = $1",
    )
    .bind(held.row.tenant_id)
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or((
        held.row.tenant_id,
        String::new(),
        held.row.tenant_name.clone().unwrap_or_default(),
        None,
        None,
        None,
        None,
    ));
    let assignees: Vec<(Uuid, String)> = if let Party::Member(_) = party {
        sqlx::query_as(
            "select m.user_id, u.display_name from kentos.membership m join kentos.app_user u on u.id = m.user_id
              where m.tenant_id = $1 and m.status = 'active' and kentos.role_rank(m.role) >= 2
                and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
              order by u.display_name",
        )
        .bind(held.row.tenant_id)
        .fetch_all(&mut **tx)
        .await?
    } else {
        Vec::new()
    };
    let can_add_files = acts && flow::can_add_files(direction, status, party);
    let checklist = catalog::kind(direction, &held.row.kind)
        .filter(|k| !k.documents.is_empty())
        .map(|k| {
            // The dossier's documents as they stand: a corrected round sends only what changed, so every package sent
            // counts, and the one being prepared too while the caller prepares it.
            let mut shown: Vec<&str> = rounds
                .values()
                .flat_map(|r| r.files.iter().map(|f| f.doc_type.as_str()))
                .collect();
            if can_add_files {
                shown.extend(
                    pending
                        .iter()
                        .map(|f: &kentos_contracts::DossierFileView| f.doc_type.as_str()),
                );
            }
            let round = if can_add_files {
                held.row.round() + 1
            } else {
                held.row.round()
            };
            DocChecklist {
                basis: k.documents_basis.unwrap_or_default().to_owned(),
                round,
                items: k
                    .documents
                    .iter()
                    .map(|d| DocCheckItem {
                        doc_type: (*d).into(),
                        label: catalog::doc_type_label(d),
                        present: shown.contains(d),
                    })
                    .collect(),
            }
        });
    Ok(DossierDetail {
        summary: held.row.view(Some(side), today()),
        description: held.description.clone(),
        owner_name: held.owner_name.clone(),
        side,
        actions: if acts {
            flow::actions(direction, status, party)
        } else {
            Vec::new()
        },
        can_edit: acts && flow::can_edit(direction, status, party),
        can_add_files,
        can_message: acts && flow::can_message(status, party),
        can_delete: flow::can_delete(direction, status, party),
        can_import: match party {
            Party::Professional => acts,
            Party::Colleague(role) => acts && role >= kentos_contracts::TenantRole::ProjectManager,
            Party::Member(role) => role >= kentos_contracts::TenantRole::ProjectManager,
        },
        can_share_with_office: party == Party::Professional && !flow::closed(status),
        can_assess_fees: matches!(party, Party::Member(r) if r >= kentos_contracts::TenantRole::Editor)
            && direction == DossierDirection::Application
            && !flow::closed(status),
        can_cancel_fees: matches!(party, Party::Member(r) if r >= kentos_contracts::TenantRole::ProjectManager),
        fee_count: u32::try_from(fee_count).unwrap_or(0),
        can_sign: acts
            && match party {
                Party::Member(r) => r >= kentos_contracts::TenantRole::Editor,
                p => p.works_for_professional(),
            },
        live_project: live.map(|(tenant, project, name, granted)| kentos_contracts::LiveProjectView {
            tenant_id: tenant.to_string(),
            project_id: project.to_string(),
            name,
            granted,
        }),
        can_link_live_project: direction == DossierDirection::Transfer
            && matches!(party, Party::Member(r) if r >= kentos_contracts::TenantRole::ProjectManager)
            && !flow::closed(status),
        pending_round: held.row.round() + 1,
        rounds: rounds.into_values().collect(),
        pending,
        messages: messages
            .into_iter()
            .map(
                |(mid, round, side, kind, author_name, body, at)| DossierMessageView {
                    id: mid.to_string(),
                    round: round_of(round),
                    side: side_of(&side),
                    kind,
                    author_name,
                    body,
                    at: rfc3339(at),
                    attachments: attachments.remove(&mid).unwrap_or_default(),
                },
            )
            .collect(),
        events: events
            .into_iter()
            .map(
                |(eid, at, actor_name, side, action, round, Json(detail))| DossierEventView {
                    id: eid.to_string(),
                    at: rfc3339(at),
                    text: event_text(direction, &action, &actor_name, round_of(round), &detail),
                    side: side_of(&side),
                    actor_name,
                    action,
                    round: round_of(round),
                },
            )
            .collect(),
        checklist,
        professional: professional.map(|p| p.view()),
        organization: DossierOrganization {
            tenant_id: tenant_id.to_string(),
            slug,
            name,
            province,
            district,
            contact_email,
            contact_phone,
        },
        assignees: assignees
            .into_iter()
            .map(|(user, name)| AssigneeOption {
                user_id: user.to_string(),
                display_name: name,
            })
            .collect(),
    })
}

/// The side a step's notification goes to: the other one.
pub(crate) fn other(side: DossierSide) -> DossierSide {
    match side {
        DossierSide::Municipality => DossierSide::Professional,
        DossierSide::Professional => DossierSide::Municipality,
    }
}

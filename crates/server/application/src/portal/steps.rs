//! A dossier's steps and messages (docs/adr/9000 §5–6). A step is taken under the dossier's lock, only from the status
//! the caller saw (another's step in between is a conflict, not an overwrite) and only as [`flow`] allows. A step that
//! sends a package seals its pending files as the next round and gives the dossier its number the first time. A note
//! goes with the step as a message (a deficiency list, a decision's reason). The other side is told.

use kentos_contracts::{
    DossierAction, DossierActionRequest, DossierDetail, DossierDirection, DossierMessageView,
    DossierSide, MessagePost,
};
use kentos_postgres::Db;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::accounts::user_scope;
use super::detail::{detail, other};
use super::dossiers::{Held, hold, record};
use super::flow::{self, Note, Party};
use super::notifications::tell;
use super::rows::{FILE_SELECT, FileRow, side_code};
use super::{catalog, excerpt, id_at, optional_at, rfc3339, text_at};
use crate::error::{AppError, AppResult};
use crate::identity::Actor;

fn unverified() -> AppError {
    AppError::forbidden(
        "Meslek mensubu kaydınız doğrulanmadığı için bu dosyada adım atamazsınız; kaydınızın durumu Profil sayfasında.",
    )
}

/// What the person must write when a step needs a note.
fn note_needed(action: DossierAction) -> &'static str {
    match action {
        DossierAction::RequestChanges => {
            "Eksik ve yanlışları bütün olarak yazın: başvuru sahibi düzeltmeyi buna göre yapar (İmar Kanunu m.22 \
             eksiklerin bir defada bildirilmesini ister)."
        }
        DossierAction::Reject => "Ret gerekçesini yazın: başvuru sahibi bildirimde bunu okur.",
        DossierAction::Decline => "İşi neden üstlenmediğinizi yazın.",
        DossierAction::Cancel => "İptalin nedenini yazın: meslek mensubu bildirimde bunu okur.",
        DossierAction::RequestRevision => "Teslimde neyin düzeltileceğini yazın.",
        _ => "Bir not yazın.",
    }
}

/// The member `user` of the organisation in scope, if they may be given dossiers (editor and above, with a seat).
async fn assignee(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    user: Uuid,
) -> AppResult<String> {
    let name: Option<String> = sqlx::query_scalar(
        "select u.display_name from kentos.membership m join kentos.app_user u on u.id = m.user_id
          where m.tenant_id = $1 and m.user_id = $2 and m.status = 'active' and kentos.role_rank(m.role) >= 2
            and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)",
    )
    .bind(tenant)
    .bind(user)
    .fetch_optional(&mut **tx)
    .await?;
    name.ok_or_else(|| {
        AppError::invalid_at(
            "assigneeId",
            "Seçilen kişi bu kurumda dosyalara bakamaz (en az düzenleyici olmalı ve koltuğu bulunmalı).",
        )
    })
}

/// Seals the caller's pending attachments `ids` onto `message`; all of them or none.
async fn attach(
    tx: &mut Transaction<'static, Postgres>,
    dossier: Uuid,
    message: Uuid,
    side: DossierSide,
    ids: &[String],
) -> AppResult<Vec<FileRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| id_at(&format!("attachments[{i}]"), id))
        .collect::<AppResult<_>>()?;
    let rows: Vec<FileRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with sealed as (
           update kentos.dossier_file set message_id = $2, state = 'final'
            where dossier_id = $1 and id = any($3) and purpose = 'attachment' and state = 'pending' and side = $4
           returning *)
         {} order by f.added_at",
        FILE_SELECT.replace("from kentos.dossier_file f", "from sealed f")
    )))
    .bind(dossier)
    .bind(message)
    .bind(&ids)
    .bind(side_code(side))
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() != ids.len() {
        return Err(AppError::invalid_at(
            "attachments",
            "Eklerden biri bulunamadı ya da başka bir iletiye eklenmiş; dosyayı yeniden ekleyin.",
        ));
    }
    Ok(rows)
}

/// Writes a message of `kind` from the caller's side, with its attachments.
async fn write_message(
    tx: &mut Transaction<'static, Postgres>,
    (dossier, side, round): (Uuid, DossierSide, u32),
    actor: &Actor,
    kind: &str,
    body: &str,
    attachments: &[String],
) -> AppResult<DossierMessageView> {
    let id = Uuid::now_v7();
    let at: time::OffsetDateTime = sqlx::query_scalar(
        "insert into kentos.dossier_message (id, dossier_id, round, side, kind, author_id, author_name, body)
         values ($1, $2, $3, $4, $5, $6, $7, $8) returning at",
    )
    .bind(id)
    .bind(dossier)
    .bind(i32::try_from(round).unwrap_or(i32::MAX))
    .bind(side_code(side))
    .bind(kind)
    .bind(actor.user_id)
    .bind(&actor.display_name)
    .bind(body)
    .fetch_one(&mut **tx)
    .await?;
    let files = attach(tx, dossier, id, side, attachments).await?;
    Ok(DossierMessageView {
        id: id.to_string(),
        round,
        side,
        kind: kind.into(),
        author_name: actor.display_name.clone(),
        body: body.into(),
        at: rfc3339(at),
        attachments: files.iter().map(FileRow::view).collect(),
    })
}

/// Who is told of a step, and what they read.
fn notice(
    held: &Held,
    action: DossierAction,
    actor: &Actor,
    round: u32,
    files: usize,
    note: Option<&str>,
) -> (DossierSide, String, String, String) {
    let title = &held.row.title;
    let note = note.map(|n| excerpt(n, 180)).unwrap_or_default();
    let application = held.direction() == DossierDirection::Application;
    let (to, kind, head, body) = match action {
        DossierAction::Send if application && round <= 1 => (
            DossierSide::Municipality,
            "dossier.submitted",
            format!("Yeni başvuru: {title}"),
            format!(
                "{} gönderdi: {}.",
                actor.display_name,
                catalog::kind_label(held.direction(), &held.row.kind)
            ),
        ),
        DossierAction::Send if application => (
            DossierSide::Municipality,
            "dossier.resubmitted",
            format!("Düzeltilmiş başvuru: {title}"),
            format!("{round}. sürüm geldi ({files} dosya)."),
        ),
        DossierAction::Send => (
            DossierSide::Professional,
            "dossier.transferred",
            format!("Size iş aktarıldı: {title}"),
            format!(
                "{} gönderdi: {}.",
                held.row.tenant_name.as_deref().unwrap_or("Belediye"),
                catalog::kind_label(held.direction(), &held.row.kind)
            ),
        ),
        DossierAction::Withdraw => (
            DossierSide::Municipality,
            "dossier.withdrawn",
            format!("Başvuru geri çekildi: {title}"),
            note,
        ),
        DossierAction::StartReview => (
            DossierSide::Professional,
            "dossier.in_review",
            format!("Başvurunuz incelemede: {title}"),
            format!("{} inceliyor.", actor.display_name),
        ),
        DossierAction::Assign => (
            DossierSide::Municipality,
            "dossier.assigned",
            format!("Size dosya verildi: {title}"),
            format!("{} sizi sorumlu yaptı.", actor.display_name),
        ),
        DossierAction::RequestChanges => (
            DossierSide::Professional,
            "dossier.changes_requested",
            format!("Düzeltme istendi: {title}"),
            note,
        ),
        DossierAction::Approve => (
            DossierSide::Professional,
            "dossier.approved",
            format!("Başvurunuz onaylandı: {title}"),
            note,
        ),
        DossierAction::Reject => (
            DossierSide::Professional,
            "dossier.rejected",
            format!("Başvurunuz reddedildi: {title}"),
            note,
        ),
        DossierAction::Accept => (
            DossierSide::Municipality,
            "dossier.accepted",
            format!("İş üstlenildi: {title}"),
            format!("{} işi üstlendi.", actor.display_name),
        ),
        DossierAction::Decline => (
            DossierSide::Municipality,
            "dossier.declined",
            format!("İş üstlenilmedi: {title}"),
            note,
        ),
        DossierAction::Deliver => (
            DossierSide::Municipality,
            "dossier.delivered",
            format!("Teslim geldi: {title}"),
            format!("{}. teslim, {files} dosya.", round.saturating_sub(1).max(1)),
        ),
        DossierAction::RequestRevision => (
            DossierSide::Professional,
            "dossier.revision_requested",
            format!("Teslimde düzeltme istendi: {title}"),
            note,
        ),
        DossierAction::Complete => (
            DossierSide::Professional,
            "dossier.completed",
            format!("Teslim onaylandı: {title}"),
            if note.is_empty() {
                "İş tamamlandı.".into()
            } else {
                note
            },
        ),
        DossierAction::Cancel => (
            DossierSide::Professional,
            "dossier.cancelled",
            format!("Aktarım iptal edildi: {title}"),
            note,
        ),
    };
    (to, kind.into(), head, body)
}

/// `POST /v1/dossiers/{id}/actions`: takes the step and answers with the dossier's page as it is now.
pub async fn act(
    db: &Db,
    actor: &Actor,
    id: Uuid,
    input: &DossierActionRequest,
) -> AppResult<DossierDetail> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, true).await?;
    let (direction, status) = (held.direction(), held.status());
    if status != input.expected_status {
        return Err(AppError::Conflict {
            message: format!(
                "Dosya siz bakarken değişti; şimdi “{}”. Sayfayı yenileyip yeniden deneyin.",
                catalog::status_label(status)
            ),
            conflicts: Vec::new(),
            revision: None,
        });
    }
    if !held.may_act() {
        return Err(unverified());
    }
    let step = flow::step_of(direction, status, input.action, held.party).map_err(|r| {
        if r.role {
            AppError::forbidden(r.message)
        } else {
            AppError::invalid(r.message)
        }
    })?;
    let note = optional_at("note", "Not", input.note.as_deref(), 8000)?;
    if step.note == Note::Required && note.is_none() {
        return Err(AppError::invalid_at("note", note_needed(input.action)));
    }
    let side = held.party.side();
    let mut round = held.row.round();
    let mut files = 0;
    if step.sends {
        let pending: Vec<Uuid> = sqlx::query_scalar(
            "select id from kentos.dossier_file
              where dossier_id = $1 and purpose = 'package' and state = 'pending' and side = $2",
        )
        .bind(id)
        .bind(side_code(side))
        .fetch_all(&mut *tx)
        .await?;
        if step.needs_files && pending.is_empty() {
            return Err(AppError::invalid(
                "Gönderilecek dosya yok: önce pakete dosya ekleyin (bir KentOS projesi ya da belgeler).",
            ));
        }
        if held.row.number.is_none() {
            sqlx::query("select kentos.dossier_assign_number($1)")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        round += 1;
        files = pending.len();
        sqlx::query(
            "update kentos.dossier_file set state = 'final', round = $2 where id = any($1)",
        )
        .bind(&pending)
        .bind(i32::try_from(round).unwrap_or(i32::MAX))
        .execute(&mut *tx)
        .await?;
    }
    let given: Option<(Uuid, String)> = match (input.action, held.party) {
        (DossierAction::Assign, _) => {
            let user = id_at(
                "assigneeId",
                input.assignee_id.as_deref().unwrap_or_default(),
            )?;
            Some((user, assignee(&mut tx, held.row.tenant_id, user).await?))
        }
        (DossierAction::StartReview, Party::Member(_)) if held.row.assignee_id.is_none() => {
            Some((actor.user_id, actor.display_name.clone()))
        }
        _ => None,
    };
    sqlx::query(
        "update kentos.dossier
            set status = $2, round = $3, updated_at = now(),
                round_sent_at = case when $4 then now() else round_sent_at end,
                sent_at = case when $4 then coalesce(sent_at, now()) else sent_at end,
                closed_at = case when $5 then now() else null end,
                assignee_id = coalesce($6, assignee_id),
                assignee_name = case when $6 is null then assignee_name else $7 end
          where id = $1",
    )
    .bind(id)
    .bind(catalog::status_code(step.to))
    .bind(i32::try_from(round).unwrap_or(i32::MAX))
    .bind(step.sends)
    .bind(flow::closed(step.to))
    .bind(given.as_ref().map(|g| g.0))
    .bind(given.as_ref().map(|g| g.1.clone()))
    .execute(&mut *tx)
    .await?;
    if let (Some(note), true) = (&note, step.note != Note::None) {
        write_message(
            &mut tx,
            (id, side, round),
            actor,
            step.note_kind,
            note,
            &input.attachments,
        )
        .await?;
    }
    record(
        &mut tx,
        id,
        actor,
        side,
        flow::action_code(input.action),
        round,
        serde_json::json!({
            "files": files,
            "assignee": given.as_ref().map(|g| g.1.clone()),
            "note": note.is_some(),
        }),
    )
    .await?;
    let (to, kind, head, body) = notice(&held, input.action, actor, round, files, note.as_deref());
    debug_assert!(to == other(side) || input.action == DossierAction::Assign);
    tell(&mut tx, id, to, &kind, &head, &body).await?;
    tx.commit().await?;
    detail(db, actor, id).await
}

/// `POST /v1/dossiers/{id}/messages`: once the dossier was sent and until it closes.
pub async fn message(
    db: &Db,
    actor: &Actor,
    id: Uuid,
    input: &MessagePost,
) -> AppResult<DossierMessageView> {
    let body = text_at("body", "İleti", &input.body, 1, 8000)?;
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, true).await?;
    if !held.may_act() {
        return Err(unverified());
    }
    if !flow::can_message(held.status(), held.party) {
        return Err(AppError::forbidden(if flow::closed(held.status()) {
            "Dosya kapandı; yazışma da kapandı."
        } else {
            "Bu dosyaya şimdi ileti yazılmaz."
        }));
    }
    let view = write_message(
        &mut tx,
        (id, held.party.side(), held.row.round()),
        actor,
        "message",
        &body,
        &input.attachments,
    )
    .await?;
    tell(
        &mut tx,
        id,
        other(held.party.side()),
        "dossier.message",
        &format!("Yeni ileti: {}", held.row.title),
        &format!("{}: {}", actor.display_name, excerpt(&body, 160)),
    )
    .await?;
    tx.commit().await?;
    Ok(view)
}

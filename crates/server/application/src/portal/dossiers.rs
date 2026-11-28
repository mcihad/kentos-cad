//! Opening, listing, editing and removing dossiers (docs/adr/9000 §5). A professional opens an application to a
//! municipality that takes its kind; a municipality's project manager opens a transfer to a verified professional.
//! Both start as drafts their own side alone sees. The page of one dossier is `detail.rs`; its steps `steps.rs`; the
//! lists `lists.rs` (re-exported here).

use kentos_contracts::{
    ApplicationCreate, DossierDirection, DossierEdit, DossierOfficeSet, DossierSide, DossierSite,
    DossierStatus, TenantRole, TransferCreate,
};
use kentos_postgres::{Db, Scope, rescope};
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::accounts::user_scope;
use super::catalog;
use super::flow::{self, Party};
use super::organization::require_role;
use super::rows::{SUMMARY_SELECT, SummaryRow};
use super::{date_at, id_at, optional_at, text_at, today};
use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::tenancy::{Access, role_from_db};

pub use super::lists::{
    ListFilter, dashboard_for_professional, dashboard_for_tenant, list_for_professional,
    list_for_tenant,
};

/// A dossier as its caller holds it: its row, the caller's part in it and the parts only its page shows.
pub(crate) struct Held {
    pub row: SummaryRow,
    pub party: Party,
    pub description: String,
    pub owner_name: Option<String>,
    /// The dossier professional's own check (`verified`, …): only a verified professional takes steps.
    pub professional_status: Option<String>,
}

impl Held {
    pub fn id(&self) -> Uuid {
        self.row.id
    }

    pub fn direction(&self) -> DossierDirection {
        self.row.direction()
    }

    pub fn status(&self) -> DossierStatus {
        self.row.status()
    }

    /// Whether the caller takes steps at all: a member always (their role decides which), the professional once
    /// verified.
    pub fn may_act(&self) -> bool {
        matches!(self.party, Party::Member(_))
            || self.professional_status.as_deref() == Some("verified")
    }
}

fn missing() -> AppError {
    AppError::not_found("Dosya bulunamadı ya da göremiyorsunuz.")
}

/// The dossier `id` as `actor` may see it, locked against other steps when `lock`. For a member the transaction moves
/// to the organisation's scope; a professional's never carries one.
pub(crate) async fn hold(
    tx: &mut Transaction<'static, Postgres>,
    actor: &Actor,
    id: Uuid,
    lock: bool,
) -> AppResult<Held> {
    let sql = if lock {
        "select kentos.dossier_side(d.id), kentos.member_role(d.tenant_id), d.tenant_id,
                d.professional_id = kentos.current_user_id(), kentos.office_role(d.office_id)
           from kentos.dossier d where d.id = $1 for update of d"
    } else {
        "select kentos.dossier_side(d.id), kentos.member_role(d.tenant_id), d.tenant_id,
                d.professional_id = kentos.current_user_id(), kentos.office_role(d.office_id)
           from kentos.dossier d where d.id = $1"
    };
    type Found = (
        Option<String>,
        Option<String>,
        Uuid,
        Option<bool>,
        Option<String>,
    );
    let found: Option<Found> = sqlx::query_as(sql)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    let (side, role, tenant, own, office_role) = found.ok_or_else(missing)?;
    let party = match (side.as_deref(), role.as_deref().and_then(role_from_db)) {
        // The responsible professional before a colleague: a professional of their own office is both.
        (Some("professional"), _) if own == Some(true) => Party::Professional,
        (Some("professional"), _) => match office_role.as_deref().and_then(role_from_db) {
            Some(r) => Party::Colleague(r),
            None => return Err(missing()),
        },
        (Some("municipality"), Some(role)) => Party::Member(role),
        _ => return Err(missing()),
    };
    if let Party::Member(_) = party {
        rescope(
            tx,
            Scope {
                tenant: Some(tenant),
                user: Some(actor.user_id),
                project: None,
            },
        )
        .await?;
    }
    let row: SummaryRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SUMMARY_SELECT} where d.id = $1"
    )))
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    let (description, owner_name, professional_status): (String, Option<String>, Option<String>) =
        sqlx::query_as(
            "select d.description, d.owner_name, p.status
           from kentos.dossier d left join kentos.professional p on p.user_id = d.professional_id
          where d.id = $1",
        )
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(Held {
        row,
        party,
        description,
        owner_name,
        professional_status,
    })
}

/// Records a step of the dossier's timeline in the open transaction.
pub(crate) async fn record(
    tx: &mut Transaction<'static, Postgres>,
    dossier: Uuid,
    actor: &Actor,
    side: DossierSide,
    action: &str,
    round: u32,
    detail: serde_json::Value,
) -> AppResult<()> {
    sqlx::query(
        "insert into kentos.dossier_event (dossier_id, actor_id, actor_name, side, action, round, detail)
         values ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(dossier)
    .bind(actor.user_id)
    .bind(&actor.display_name)
    .bind(super::rows::side_code(side))
    .bind(action)
    .bind(i32::try_from(round).unwrap_or(i32::MAX))
    .bind(Json(detail))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn site(input: &DossierSite) -> AppResult<DossierSite> {
    Ok(DossierSite {
        province: optional_at("site.province", "İl", input.province.as_deref(), 60)?,
        district: optional_at("site.district", "İlçe", input.district.as_deref(), 60)?,
        neighbourhood: optional_at(
            "site.neighbourhood",
            "Mahalle",
            input.neighbourhood.as_deref(),
            120,
        )?,
        block: optional_at("site.block", "Ada", input.block.as_deref(), 20)?,
        parcel: optional_at("site.parcel", "Parsel", input.parcel.as_deref(), 20)?,
        address: optional_at("site.address", "Adres", input.address.as_deref(), 400)?,
    })
}

fn kind_at(direction: DossierDirection, kind: &str) -> AppResult<String> {
    catalog::kind(direction, kind)
        .map(|k| k.code.to_owned())
        .ok_or_else(|| {
            AppError::invalid_at("kind", format!("“{kind}” bu yönde bir dosya türü değil."))
        })
}

/// The professional's check, as the caller sees it.
async fn professional_status(
    tx: &mut Transaction<'static, Postgres>,
    user: Uuid,
) -> AppResult<Option<String>> {
    Ok(
        sqlx::query_scalar("select status from kentos.professional where user_id = $1")
            .bind(user)
            .fetch_optional(&mut **tx)
            .await?,
    )
}

/// `POST /v1/me/dossiers`: a professional's application to a municipality that takes its kind, as a draft.
pub async fn create_application(
    db: &Db,
    actor: &Actor,
    input: &ApplicationCreate,
) -> AppResult<Uuid> {
    let tenant = id_at("tenantId", &input.tenant_id)?;
    let kind = kind_at(DossierDirection::Application, &input.kind)?;
    let title = text_at("title", "Başlık", &input.title, 3, 200)?;
    let description = text_at("description", "Açıklama", &input.description, 0, 8000)?;
    let site = site(&input.site)?;
    let owner_name = optional_at("ownerName", "Yapı sahibi", input.owner_name.as_deref(), 200)?;
    let mut tx = db.scoped(user_scope(actor)).await?;
    match professional_status(&mut tx, actor.user_id)
        .await?
        .as_deref()
    {
        Some("verified") => {}
        Some(_) => {
            return Err(AppError::forbidden(
                "Başvuru yapmak için meslek mensubu kaydınızın doğrulanması gerekir.",
            ));
        }
        None => {
            return Err(AppError::forbidden(
                "Başvuruyu meslek mensupları yapar; hesabınız bir meslek mensubunun değil.",
            ));
        }
    }
    let target: Option<(String, Option<bool>, Option<Vec<String>>)> = sqlx::query_as(
        "select t.name, tp.accepts_applications, tp.application_kinds
           from kentos.tenant t left join kentos.tenant_portal tp on tp.tenant_id = t.id
          where t.id = $1 and t.kind = 'organization' and t.category = 'municipality' and t.status = 'active'",
    )
    .bind(tenant)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((name, accepts, kinds)) = target else {
        return Err(AppError::invalid_at("tenantId", "Belediye bulunamadı."));
    };
    if accepts != Some(true) {
        return Err(AppError::invalid_at(
            "tenantId",
            format!("{name} şu anda portaldan başvuru almıyor."),
        ));
    }
    let kinds = kinds.unwrap_or_default();
    if !kinds.is_empty() && !kinds.contains(&kind) {
        return Err(AppError::invalid_at(
            "kind",
            format!(
                "{name} portaldan “{}” başvurusu almıyor.",
                catalog::kind_label(DossierDirection::Application, &kind)
            ),
        ));
    }
    let office = match input
        .office_id
        .as_deref()
        .map(str::trim)
        .filter(|o| !o.is_empty())
    {
        Some(o) => Some(office_for(&mut tx, id_at("officeId", o)?).await?),
        None => None,
    };
    let id = Uuid::now_v7();
    sqlx::query(
        "insert into kentos.dossier (id, tenant_id, professional_id, direction, kind, title, description, site,
                                     owner_name, status, created_by, office_id)
         values ($1, $2, $3, 'application', $4, $5, $6, $7, $8, 'draft', $3, $9)",
    )
    .bind(id)
    .bind(tenant)
    .bind(actor.user_id)
    .bind(&kind)
    .bind(&title)
    .bind(&description)
    .bind(Json(&site))
    .bind(&owner_name)
    .bind(office)
    .execute(&mut *tx)
    .await?;
    record(
        &mut tx,
        id,
        actor,
        DossierSide::Professional,
        "created",
        0,
        serde_json::json!({}),
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// An office the caller may share a dossier with: one they are an editor of, or above (docs/adr/9000 §18).
async fn office_for(tx: &mut Transaction<'static, Postgres>, office: Uuid) -> AppResult<Uuid> {
    let rank: Option<i32> = sqlx::query_scalar("select kentos.role_rank(kentos.office_role($1))")
        .bind(office)
        .fetch_one(&mut **tx)
        .await?;
    if rank.unwrap_or(0) < 2 {
        return Err(AppError::invalid_at(
            "officeId",
            "Dosyayı yalnız düzenleyici ya da daha üst rolle üyesi olduğunuz bir büroyla paylaşabilirsiniz.",
        ));
    }
    Ok(office)
}

/// `PUT /v1/dossiers/{id}/office`: its responsible professional shares the dossier with an office of theirs, or takes it
/// What the live project's definer functions said (docs/adr/9000 §24), as the person reads it.
pub(crate) fn live_refused(code: Option<String>) -> AppResult<()> {
    match code.as_deref() {
        None => Ok(()),
        Some("forbidden") => Err(AppError::forbidden(
            "Aktarıma canlı projeyi belediyenin proje yöneticisi bağlar.",
        )),
        Some("not_found") => Err(AppError::not_found("Proje bulunamadı ya da kurumun değil.")),
        Some("invalid") => Err(AppError::invalid("Kapanmış aktarıma proje bağlanmaz.")),
        Some("guests_off") => Err(AppError::invalid(
            "Belediye kurum dışından misafir kabul etmiyor; canlı projede çalışmak için belediyeyle görüşün.",
        )),
        Some("inactive") => Err(AppError::invalid(
            "Meslek mensubunun bu kurumdaki üyeliği ya da koltuğu etkin değil; canlı proje paylaşılamaz.",
        )),
        Some(other) => Err(AppError::invalid(format!(
            "Canlı proje bağlanamadı ({other})."
        ))),
    }
}

/// `PUT /v1/dossiers/{id}/live-project`: links one of the municipality's cloud projects to its transfer (none
/// unlinks); a job already taken on gets its grant at once. The municipality's project managers, while it is open.
pub async fn set_live_project(
    db: &Db,
    actor: &Actor,
    id: Uuid,
    input: &kentos_contracts::DossierLiveProjectSet,
) -> AppResult<()> {
    let project = match input
        .project_id
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        Some(p) => Some(id_at("projectId", p)?),
        None => None,
    };
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, true).await?;
    let code: Option<String> = sqlx::query_scalar("select kentos.dossier_set_live_project($1, $2)")
        .bind(id)
        .bind(project)
        .fetch_one(&mut *tx)
        .await?;
    live_refused(code)?;
    record(
        &mut tx,
        id,
        actor,
        held.party.side(),
        if project.is_some() {
            "live_project_linked"
        } else {
            "live_project_unlinked"
        },
        held.row.round(),
        serde_json::json!({}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// back; the timeline says so.
pub async fn set_office(
    db: &Db,
    actor: &Actor,
    id: Uuid,
    input: &DossierOfficeSet,
) -> AppResult<()> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, true).await?;
    if held.party != Party::Professional {
        return Err(AppError::forbidden(
            "Dosyayı bir büroyla yalnız sorumlu meslek mensubu paylaşır ya da paylaşımı kaldırır.",
        ));
    }
    if flow::closed(held.status()) {
        return Err(AppError::invalid("Kapanmış dosyanın paylaşımı değişmez."));
    }
    let office = match input
        .office_id
        .as_deref()
        .map(str::trim)
        .filter(|o| !o.is_empty())
    {
        Some(o) => Some(office_for(&mut tx, id_at("officeId", o)?).await?),
        None => None,
    };
    if office == held.row.office_id {
        tx.commit().await?;
        return Ok(());
    }
    sqlx::query("update kentos.dossier set office_id = $2, updated_at = now() where id = $1")
        .bind(id)
        .bind(office)
        .execute(&mut *tx)
        .await?;
    let name: Option<String> = match office {
        Some(o) => {
            sqlx::query_scalar("select name from kentos.tenant where id = $1")
                .bind(o)
                .fetch_optional(&mut *tx)
                .await?
        }
        None => None,
    };
    record(
        &mut tx,
        id,
        actor,
        DossierSide::Professional,
        if office.is_some() {
            "office_shared"
        } else {
            "office_unshared"
        },
        held.row.round(),
        serde_json::json!({ "office": name.or(held.row.office_name.clone()) }),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// `POST /v1/tenants/{tenant}/dossiers`: a municipality's transfer to a verified professional, as a draft.
pub async fn create_transfer(db: &Db, access: &Access, input: &TransferCreate) -> AppResult<Uuid> {
    require_role(access, TenantRole::ProjectManager, "Proje aktarmak").map_err(|_| {
        AppError::forbidden(
            "Proje aktarımını kurumunuzun proje yöneticileri ve yöneticileri yapar.",
        )
    })?;
    let professional = id_at("professionalId", &input.professional_id)?;
    let kind = kind_at(DossierDirection::Transfer, &input.kind)?;
    let title = text_at("title", "Başlık", &input.title, 3, 200)?;
    let description = text_at("description", "Açıklama", &input.description, 0, 8000)?;
    let site = site(&input.site)?;
    let due_on = date_at("dueOn", input.due_on.as_deref())?;
    if due_on.is_some_and(|d| d < today()) {
        return Err(AppError::invalid_at(
            "dueOn",
            "Teslim günü geçmiş bir gün olamaz.",
        ));
    }
    let mut tx = db.scoped(access.scope()).await?;
    let category: String = sqlx::query_scalar("select category from kentos.tenant where id = $1")
        .bind(access.tenant)
        .fetch_one(&mut *tx)
        .await?;
    if category != "municipality" {
        return Err(AppError::forbidden(
            "Proje aktarımı belediyelerin portalında yapılır.",
        ));
    }
    match professional_status(&mut tx, professional).await?.as_deref() {
        Some("verified") => {}
        Some(_) => {
            return Err(AppError::invalid_at(
                "professionalId",
                "Bu meslek mensubunun kaydı doğrulanmamış; ona iş aktarılamaz.",
            ));
        }
        None => {
            return Err(AppError::invalid_at(
                "professionalId",
                "Meslek mensubu bulunamadı.",
            ));
        }
    }
    let id = Uuid::now_v7();
    sqlx::query(
        "insert into kentos.dossier (id, tenant_id, professional_id, direction, kind, title, description, site,
                                     status, due_on, created_by)
         values ($1, $2, $3, 'transfer', $4, $5, $6, $7, 'draft', $8, $9)",
    )
    .bind(id)
    .bind(access.tenant)
    .bind(professional)
    .bind(&kind)
    .bind(&title)
    .bind(&description)
    .bind(Json(&site))
    .bind(due_on)
    .bind(access.actor.user_id)
    .execute(&mut *tx)
    .await?;
    record(
        &mut tx,
        id,
        &access.actor,
        DossierSide::Municipality,
        "created",
        0,
        serde_json::json!({}),
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// `PATCH /v1/dossiers/{id}`: its title, kind, place, building owner (an application's) or delivery day (a
/// transfer's), by its sending side while it prepares the dossier.
pub async fn edit(db: &Db, actor: &Actor, id: Uuid, input: &DossierEdit) -> AppResult<()> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, true).await?;
    let (direction, status) = (held.direction(), held.status());
    if !held.may_act() || !flow::can_edit(direction, status, held.party) {
        return Err(AppError::forbidden(
            "Dosyanın bilgileri şimdi değiştirilemez.",
        ));
    }
    let kind = kind_at(direction, &input.kind)?;
    let title = text_at("title", "Başlık", &input.title, 3, 200)?;
    let description = text_at("description", "Açıklama", &input.description, 0, 8000)?;
    let site = site(&input.site)?;
    let owner_name = match direction {
        DossierDirection::Application => {
            optional_at("ownerName", "Yapı sahibi", input.owner_name.as_deref(), 200)?
        }
        DossierDirection::Transfer => None,
    };
    let due_on = match direction {
        DossierDirection::Transfer => date_at("dueOn", input.due_on.as_deref())?,
        DossierDirection::Application => None,
    };
    sqlx::query(
        "update kentos.dossier set kind = $2, title = $3, description = $4, site = $5, owner_name = $6, due_on = $7,
                updated_at = now()
          where id = $1",
    )
    .bind(id)
    .bind(&kind)
    .bind(&title)
    .bind(&description)
    .bind(Json(&site))
    .bind(&owner_name)
    .bind(due_on)
    .execute(&mut *tx)
    .await?;
    // A draft's edits are its own side's business; a corrected application's are part of its record.
    if status != DossierStatus::Draft {
        record(
            &mut tx,
            id,
            actor,
            held.party.side(),
            "edited",
            held.row.round(),
            serde_json::json!({}),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// `DELETE /v1/dossiers/{id}`: a draft, by its side, with its files.
pub async fn delete(db: &Db, blobs: &Blobs, actor: &Actor, id: Uuid) -> AppResult<()> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let held = hold(&mut tx, actor, id, true).await?;
    if !flow::can_delete(held.direction(), held.status(), held.party) {
        return Err(AppError::forbidden(
            "Yalnız taslak, onu hazırlayan tarafça silinir.",
        ));
    }
    let keys: Vec<String> =
        sqlx::query_scalar("select blob_key from kentos.dossier_file where dossier_id = $1")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
    sqlx::query("delete from kentos.dossier where id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    super::files::remove_all(blobs, &keys).await;
    Ok(())
}

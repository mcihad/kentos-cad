//! An organisation on the portal (docs/adr/9000 §3–4): its settings (whether it takes applications and of which kinds,
//! how to reach it), its members as its owners and admins manage them, and the directory of professionals its members
//! transfer projects to. Also the municipalities a professional chooses from.

use kentos_contracts::{
    MemberCreate, MemberEdit, MemberView, MunicipalityCard, OrganizationPortal,
    OrganizationPortalEdit, Profession, ProfessionalCard, TenantRole,
};
use kentos_postgres::Db;
use uuid::Uuid;

use super::accounts::{category_of, user_scope};
use super::{answer, catalog, email_at, optional_at, text_at};
use crate::admin::{check_login, check_password};
use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::tenancy::{Access, role_to_db};

/// Refuses a member below `role`, naming what it takes.
pub(crate) fn require_role(access: &Access, role: TenantRole, what: &str) -> AppResult<()> {
    if access.role >= role {
        Ok(())
    } else {
        Err(AppError::forbidden(format!(
            "{what} kurumunuzun yöneticilerine açık."
        )))
    }
}

type PortalRow = (
    Uuid,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    i32,
    i64,
    Option<bool>,
    Option<Vec<String>>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// `GET /v1/tenants/{tenant}/portal`.
pub async fn portal(db: &Db, access: &Access) -> AppResult<OrganizationPortal> {
    let mut tx = db.scoped(access.scope()).await?;
    let row: PortalRow = sqlx::query_as(
        "select t.id, t.slug, t.name, t.category, t.province, t.district, t.seat_limit,
                (select count(*) from kentos.seat_allocation s where s.tenant_id = t.id),
                tp.accepts_applications, tp.application_kinds, tp.contact_email, tp.contact_phone, tp.address
           from kentos.tenant t left join kentos.tenant_portal tp on tp.tenant_id = t.id
          where t.id = $1",
    )
    .bind(access.tenant)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let (
        id,
        slug,
        name,
        category,
        province,
        district,
        seat_limit,
        seats_used,
        accepts,
        kinds,
        email,
        phone,
        address,
    ) = row;
    Ok(OrganizationPortal {
        tenant_id: id.to_string(),
        slug,
        name,
        category: category_of(&category),
        province,
        district,
        accepts_applications: accepts.unwrap_or(false),
        application_kinds: kinds.unwrap_or_default(),
        contact_email: email,
        contact_phone: phone,
        address,
        seat_limit: u32::try_from(seat_limit).unwrap_or(0),
        seats_used: u32::try_from(seats_used).unwrap_or(0),
        role: access.role,
    })
}

/// `PUT /v1/tenants/{tenant}/portal`: by its owners and admins.
pub async fn set_portal(db: &Db, access: &Access, input: &OrganizationPortalEdit) -> AppResult<()> {
    require_role(access, TenantRole::Admin, "Kurumun portal ayarları")?;
    for (i, kind) in input.application_kinds.iter().enumerate() {
        if catalog::kind(kentos_contracts::DossierDirection::Application, kind).is_none() {
            return Err(AppError::invalid_at(
                format!("applicationKinds[{i}]"),
                format!("“{kind}” bir başvuru türü değil."),
            ));
        }
    }
    let email = email_at("contactEmail", input.contact_email.as_deref(), false)?;
    let phone = optional_at(
        "contactPhone",
        "Telefon",
        input.contact_phone.as_deref(),
        40,
    )?;
    let address = optional_at("address", "Adres", input.address.as_deref(), 400)?;
    let mut tx = db.scoped(access.scope()).await?;
    sqlx::query(
        "insert into kentos.tenant_portal (tenant_id, accepts_applications, application_kinds, contact_email,
                                           contact_phone, address, updated_at, updated_by)
         values ($1, $2, $3, $4, $5, $6, now(), $7)
         on conflict (tenant_id) do update
           set accepts_applications = excluded.accepts_applications, application_kinds = excluded.application_kinds,
               contact_email = excluded.contact_email, contact_phone = excluded.contact_phone,
               address = excluded.address, updated_at = now(), updated_by = excluded.updated_by",
    )
    .bind(access.tenant)
    .bind(input.accepts_applications)
    .bind(&input.application_kinds)
    .bind(&email)
    .bind(&phone)
    .bind(&address)
    .bind(access.actor.user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// `GET /v1/tenants/{tenant}/members`: by its owners and admins.
pub async fn members(db: &Db, access: &Access) -> AppResult<Vec<MemberView>> {
    require_role(access, TenantRole::Admin, "Üyelerin yönetimi")?;
    let mut tx = db.scoped(access.scope()).await?;
    let members = super::platform::members_of(&mut tx, access.tenant).await?;
    tx.commit().await?;
    Ok(members)
}

/// `POST /v1/tenants/{tenant}/members`: a new account as a member with a seat.
pub async fn add_member(db: &Db, access: &Access, input: &MemberCreate) -> AppResult<Uuid> {
    require_role(access, TenantRole::Admin, "Üye eklemek")?;
    let login = input.login.trim().to_lowercase();
    check_login(&login).map_err(|e| e.at("login"))?;
    check_password(&input.password).map_err(|e| e.at("password"))?;
    let name = text_at("displayName", "Ad soyad", &input.display_name, 3, 120)?;
    let email = email_at("email", input.email.as_deref(), false)?;
    let id = Uuid::now_v7();
    let mut tx = db.scoped(access.scope()).await?;
    let code: Option<String> =
        sqlx::query_scalar("select kentos.tenant_create_member($1, $2, $3, $4, $5, $6, $7)")
            .bind(access.tenant)
            .bind(id)
            .bind(&login)
            .bind(&input.password)
            .bind(&name)
            .bind(&email)
            .bind(role_to_db(input.role))
            .fetch_one(&mut *tx)
            .await?;
    answer(code)?;
    tx.commit().await?;
    Ok(id)
}

/// `PATCH /v1/tenants/{tenant}/members/{user}`: role, status, seat.
pub async fn update_member(
    db: &Db,
    access: &Access,
    user: Uuid,
    input: &MemberEdit,
) -> AppResult<()> {
    require_role(access, TenantRole::Admin, "Üyelerin yönetimi")?;
    if !matches!(input.status.as_str(), "active" | "disabled") {
        return Err(AppError::invalid_at(
            "status",
            "Durum “active” ya da “disabled” olmalı.",
        ));
    }
    let mut tx = db.scoped(access.scope()).await?;
    let code: Option<String> =
        sqlx::query_scalar("select kentos.tenant_update_member($1, $2, $3, $4, $5)")
            .bind(access.tenant)
            .bind(user)
            .bind(role_to_db(input.role))
            .bind(&input.status)
            .bind(input.seat)
            .fetch_one(&mut *tx)
            .await?;
    answer(code)?;
    tx.commit().await?;
    Ok(())
}

type CardRow = (
    Uuid,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    i64,
);

/// `GET /v1/tenants/{tenant}/professionals?q=&profession=`: verified professionals who are listed, and anyone the
/// organisation has a dossier with; by name.
pub async fn directory(
    db: &Db,
    access: &Access,
    query: Option<&str>,
    profession: Option<Profession>,
) -> AppResult<Vec<ProfessionalCard>> {
    let mut tx = db.scoped(access.scope()).await?;
    let rows: Vec<CardRow> = sqlx::query_as(
        "select p.user_id, u.display_name, p.profession, p.chamber_no, p.office_name, p.province, p.status,
                (select count(*) from kentos.dossier d
                  where d.professional_id = p.user_id and d.tenant_id = $1
                    and not (d.direction = 'application' and d.status = 'draft')) as dossiers
           from kentos.professional p join kentos.app_user u on u.id = p.user_id
          where ((p.status = 'verified' and p.listed)
                 or exists (select 1 from kentos.dossier d where d.professional_id = p.user_id and d.tenant_id = $1
                              and not (d.direction = 'application' and d.status = 'draft')))
            and ($2::text = '' or u.display_name ilike '%' || $2 || '%' or p.chamber_no ilike '%' || $2 || '%'
                 or coalesce(p.office_name, '') ilike '%' || $2 || '%')
            and ($3::text is null or p.profession = $3)
          order by u.display_name
          limit 200",
    )
    .bind(access.tenant)
    .bind(query.map(str::trim).unwrap_or_default())
    .bind(profession.map(catalog::profession_code))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, name, prof, chamber_no, office, province, status, dossiers)| ProfessionalCard {
                user_id: id.to_string(),
                display_name: name,
                profession: catalog::profession_of(&prof).unwrap_or(Profession::Architect),
                chamber_no,
                office_name: office,
                province,
                status: catalog::professional_status_of(&status)
                    .unwrap_or(kentos_contracts::ProfessionalStatus::Pending),
                dossiers: u32::try_from(dossiers).unwrap_or(0),
            },
        )
        .collect())
}

type MunicipalityRow = (
    Uuid,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<bool>,
    Option<Vec<String>>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// `GET /v1/portal/municipalities`: the active municipalities, those taking applications first.
pub async fn municipalities(db: &Db, actor: &Actor) -> AppResult<Vec<MunicipalityCard>> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let rows: Vec<MunicipalityRow> = sqlx::query_as(
        "select t.id, t.slug, t.name, t.province, t.district, tp.accepts_applications, tp.application_kinds,
                tp.contact_email, tp.contact_phone, tp.address
           from kentos.tenant t left join kentos.tenant_portal tp on tp.tenant_id = t.id
          where t.kind = 'organization' and t.category = 'municipality' and t.status = 'active'
          order by coalesce(tp.accepts_applications, false) desc, t.province, t.name",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, slug, name, province, district, accepts, kinds, email, phone, address)| {
                MunicipalityCard {
                    tenant_id: id.to_string(),
                    slug,
                    name,
                    province,
                    district,
                    accepts_applications: accepts.unwrap_or(false),
                    application_kinds: kinds.unwrap_or_default(),
                    contact_email: email,
                    contact_phone: phone,
                    address,
                }
            },
        )
        .collect())
}

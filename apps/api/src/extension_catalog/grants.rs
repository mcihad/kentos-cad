//! The persistent catalog's grants, the server administrator's (docs/adr/8059 §11, §13; `kentosd extension-catalog`):
//! roles in a catalog or in every one, publishers and their members, the publishers a catalog trusts. Written as the
//! owner role; the server's role only reads them. An organisation's members are in its catalog without a grant; its
//! own roles (owner, editor …) grant nothing there.

use kentos_application::admin;
use kentos_extensions::release::Role;
use sqlx::PgPool;
use uuid::Uuid;

use super::CatalogId;
use super::postgres::{role_id, role_of};
use super::store::Where;

/// One line of `kentosd extension-catalog list`.
pub struct Line {
    pub what: &'static str,
    pub text: String,
}

fn database(e: sqlx::Error) -> String {
    format!("Veritabanı: {e}")
}

/// A role by the flow's word.
pub fn role(text: &str) -> Result<Role, String> {
    role_of(text).ok_or_else(|| {
        format!("Rol “{text}” olmaz: publisher, reviewer, catalogAdmin ya da scanner.")
    })
}

/// An organisation's id by its short name.
async fn tenant(owner: &PgPool, slug: &str) -> Result<Uuid, String> {
    sqlx::query_scalar("select id from kentos.tenant where slug = $1")
        .bind(slug)
        .fetch_optional(owner)
        .await
        .map_err(database)?
        .ok_or_else(|| format!("“{slug}” kısa adlı bir kurum yok."))
}

/// How the tables name where a role is granted or a publisher trusted.
async fn columns(owner: &PgPool, place: &Where) -> Result<(&'static str, String), String> {
    match place {
        Where::Every => Ok(("every", String::new())),
        Where::In(CatalogId::Public) => Ok(("public", String::new())),
        Where::In(CatalogId::Tenant(slug)) => {
            Ok(("tenant", tenant(owner, slug).await?.to_string()))
        }
        Where::In(CatalogId::Personal(_)) => {
            Err("Bir hesabın kendi kataloğunda rol verilmez ve yayımcıya güvenilmez.".to_owned())
        }
    }
}

/// An account by its login or id, which must exist.
async fn account(owner: &PgPool, who: &str) -> Result<Uuid, String> {
    let id = admin::user_id(owner, who)
        .await
        .map_err(|e| e.to_string())?;
    let exists: bool =
        sqlx::query_scalar("select exists (select 1 from kentos.app_user where id = $1)")
            .bind(id)
            .fetch_one(owner)
            .await
            .map_err(database)?;
    if exists {
        Ok(id)
    } else {
        Err(format!("“{who}” kimlikli bir hesap yok."))
    }
}

/// Grants an account a role; granted again is the same grant.
pub async fn grant(owner: &PgPool, place: &Where, who: &str, role: Role) -> Result<(), String> {
    let (kind, reference) = columns(owner, place).await?;
    let account = account(owner, who).await?;
    sqlx::query(
        "insert into kentos.extension_catalog_grant (catalog_kind, catalog_ref, account, role)
         values ($1, $2, $3, $4) on conflict do nothing",
    )
    .bind(kind)
    .bind(&reference)
    .bind(account)
    .bind(role_id(role))
    .execute(owner)
    .await
    .map_err(database)?;
    Ok(())
}

/// Takes a role back; whether the account held it.
pub async fn revoke(owner: &PgPool, place: &Where, who: &str, role: Role) -> Result<bool, String> {
    let (kind, reference) = columns(owner, place).await?;
    let account = account(owner, who).await?;
    let done = sqlx::query(
        "delete from kentos.extension_catalog_grant
          where catalog_kind = $1 and catalog_ref = $2 and account = $3 and role = $4",
    )
    .bind(kind)
    .bind(&reference)
    .bind(account)
    .bind(role_id(role))
    .execute(owner)
    .await
    .map_err(database)?;
    Ok(done.rows_affected() > 0)
}

/// Opens a publisher, or renames it; a new one needs its name.
pub async fn publisher(owner: &PgPool, id: &str, name: Option<&str>) -> Result<(), String> {
    let valid = (2..=100).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !valid {
        return Err(format!(
            "Yayımcı “{id}” olmaz: küçük İngilizce harf, rakam ve tire (ör. ornek-belediye)."
        ));
    }
    let name = name.map(str::trim).filter(|n| !n.is_empty());
    let exists: bool = sqlx::query_scalar(
        "select exists (select 1 from kentos.extension_publisher where id = $1)",
    )
    .bind(id)
    .fetch_one(owner)
    .await
    .map_err(database)?;
    match (exists, name) {
        (false, None) => Err(format!(
            "“{id}” yayımcısı yok; açmak için adını da verin (--name)."
        )),
        (_, Some(name)) => {
            sqlx::query(
                "insert into kentos.extension_publisher (id, name) values ($1, $2)
                 on conflict (id) do update set name = excluded.name",
            )
            .bind(id)
            .bind(name)
            .execute(owner)
            .await
            .map_err(database)?;
            Ok(())
        }
        (true, None) => Ok(()),
    }
}

/// Makes an account a member of a publisher, or no longer; whether that changed anything.
pub async fn member(owner: &PgPool, publisher: &str, who: &str, on: bool) -> Result<bool, String> {
    let account = account(owner, who).await?;
    let done = if on {
        sqlx::query(
            "insert into kentos.extension_publisher_member (publisher_id, account) values ($1, $2)
             on conflict do nothing",
        )
    } else {
        sqlx::query(
            "delete from kentos.extension_publisher_member where publisher_id = $1 and account = $2",
        )
    }
    .bind(publisher)
    .bind(account)
    .execute(owner)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.code().as_deref() == Some("23503") => format!(
            "“{publisher}” yayımcısı yok; önce açın: kentosd extension-catalog publisher --publisher {publisher} \
             --name AD"
        ),
        _ => database(e),
    })?;
    Ok(done.rows_affected() > 0)
}

/// Lets a catalog trust a publisher, or no longer.
pub async fn trust(owner: &PgPool, place: &Where, publisher: &str, on: bool) -> Result<(), String> {
    let (kind, reference) = match place {
        Where::Every => {
            return Err(
                "Güven bir kataloğun kararıdır: public ya da tenant:KISA-AD verin.".to_owned(),
            );
        }
        place => columns(owner, place).await?,
    };
    let done = if on {
        sqlx::query(
            "insert into kentos.extension_catalog_trust (catalog_kind, catalog_ref, publisher_id)
             values ($1, $2, $3) on conflict do nothing",
        )
    } else {
        sqlx::query(
            "delete from kentos.extension_catalog_trust
              where catalog_kind = $1 and catalog_ref = $2 and publisher_id = $3",
        )
    }
    .bind(kind)
    .bind(&reference)
    .bind(publisher)
    .execute(owner)
    .await;
    match done {
        Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23503") => Err(format!(
            "“{publisher}” yayımcısı yok; önce açın: kentosd extension-catalog publisher --publisher {publisher} \
             --name AD"
        )),
        Err(e) => Err(database(e)),
        Ok(_) => Ok(()),
    }
}

/// The grants, the publishers with their members and the trusts, one per line: what the administrator gave.
pub async fn listing(owner: &PgPool) -> Result<Vec<Line>, String> {
    let catalog =
        "case g.catalog_kind when 'tenant' then 'tenant:' || coalesce(t.slug, g.catalog_ref)
                   else g.catalog_kind end";
    let who = "coalesce(c.login, u.id::text) || ' (' || u.display_name || ')'";
    let grants: Vec<(String, String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {catalog}, {who}, g.role from kentos.extension_catalog_grant g
           join kentos.app_user u on u.id = g.account
           left join kentos.local_credential c on c.user_id = u.id
           left join kentos.tenant t on g.catalog_kind = 'tenant' and t.id::text = g.catalog_ref
          order by 1, 2, 3"
    )))
    .fetch_all(owner)
    .await
    .map_err(database)?;
    let members: Vec<(String, String, Option<String>)> =
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "select p.id, p.name, case when u.id is null then null else {who} end
           from kentos.extension_publisher p
           left join kentos.extension_publisher_member m on m.publisher_id = p.id
           left join kentos.app_user u on u.id = m.account
           left join kentos.local_credential c on c.user_id = u.id
          order by 1, 3"
        )))
        .fetch_all(owner)
        .await
        .map_err(database)?;
    let trusts: Vec<(String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {}, g.publisher_id from kentos.extension_catalog_trust g
           left join kentos.tenant t on g.catalog_kind = 'tenant' and t.id::text = g.catalog_ref
          order by 1, 2",
        catalog
    )))
    .fetch_all(owner)
    .await
    .map_err(database)?;
    let mut lines: Vec<Line> = grants
        .into_iter()
        .map(|(catalog, who, role)| Line {
            what: "rol",
            text: format!("{catalog}\t{who}\t{role}"),
        })
        .collect();
    lines.extend(members.into_iter().map(|(id, name, who)| Line {
        what: "yayımcı",
        text: format!("{id}\t{name}\t{}", who.as_deref().unwrap_or("(üyesi yok)")),
    }));
    lines.extend(trusts.into_iter().map(|(catalog, publisher)| Line {
        what: "güvenilen",
        text: format!("{catalog}\t{publisher}"),
    }));
    Ok(lines)
}

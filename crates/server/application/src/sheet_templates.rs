//! The sheet template library in the cloud (docs/sheet/design.md §13,
//! migrations 0013 and 0014): a person's templates and the ones shared with
//! them, and the libraries of the organisations they work in, with every
//! revision kept.
//!
//! - Changing: the product commands `sheet.template.create`, `.update`,
//!   `.delete`, `.share`, `.unshare`, `.publish` v1 ([`COMMANDS`], the list
//!   `kentos_sheet::cloud::catalog_entries` describes; `tests/sheet_templates.rs`
//!   keeps the two equal). They arrive at the command route of the template's
//!   space ([`run`]): the caller's personal space for a person's template
//!   (their own or one shared with them), the organisation for one of its
//!   library; each is one transaction under the template's lock, with its
//!   audit record, the events of the people who should hear it and its
//!   idempotent answer.
//! - Reading: the caller's list ([`mine`]), one template with its newest
//!   content ([`detail`]), its sharing ([`access_list`], the owner only),
//!   the people it may be shared with ([`candidates`], the owner only), the
//!   caller's events after a cursor ([`events_after`]).
//! - Who may do what: the owner everything; an editor saves new revisions; a
//!   viewer reads. A template the caller has no role in answers 404 word for
//!   word as one that does not exist. Sharing goes only to the active members
//!   of an organisation the owner works in (docs/adr/0024): no open search,
//!   no counting of accounts. Nobody changes their own role.
//! - An organisation's library (design §13 “Kurum şablonları”): its owner,
//!   administrators and the members who may create projects publish into it
//!   (create there, or copy their own template there with `.publish`); the
//!   one who published a template and the administrators edit and delete it;
//!   every other active member with a seat uses it. Guests and people who
//!   left see nothing. It is not shared one by one.
//! - The content is the template file, read and checked by `kentos-sheet`
//!   exactly as a device reads it (an older schema is migrated, an unknown
//!   field refused), with its id and revision the server's. Row-level
//!   security (`kentos.sheet_template_role`) holds every row to the same rules.

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{CommandEnvelope, TenantKind};
use kentos_postgres::{Db, Scope};
use kentos_sheet::cloud::{
    COMMAND_CREATE, COMMAND_DELETE, COMMAND_PUBLISH, COMMAND_SHARE, COMMAND_UNSHARE,
    COMMAND_UPDATE, COMMAND_VERSION, CloudOrganization, OrganizationTemplates, SheetTemplateAccess,
    SheetTemplateCandidate, SheetTemplateCandidates, SheetTemplateChanged, SheetTemplateDelete,
    SheetTemplateDetail, SheetTemplateEvent, SheetTemplateEventKind, SheetTemplateEventPage,
    SheetTemplateGrant, SheetTemplateList, SheetTemplatePublish, SheetTemplateShare,
    SheetTemplateSummary, SheetTemplateUnshare, TEMPLATE_CONTENT_MAX, TEMPLATES_PER_PERSON_MAX,
    TemplateGrantRole, TemplateRole,
};
use kentos_sheet::template::{PaperChoice, Template};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::people::{FOLD_FROM, FOLD_TO, fold, search_patterns};
use crate::projects::rfc3339;
use crate::tenancy::{self, Access};

/// The commands this module runs, with their versions.
pub const COMMANDS: &[(&str, u32)] = &[
    (COMMAND_CREATE, COMMAND_VERSION),
    (COMMAND_UPDATE, COMMAND_VERSION),
    (COMMAND_DELETE, COMMAND_VERSION),
    (COMMAND_SHARE, COMMAND_VERSION),
    (COMMAND_UNSHARE, COMMAND_VERSION),
    (COMMAND_PUBLISH, COMMAND_VERSION),
];

/// Most events one page gives.
pub const EVENTS_PAGE_MAX: i64 = 500;

/// Whether the command is one of this module's.
pub fn handles(name: &str) -> bool {
    COMMANDS.iter().any(|(n, _)| *n == name)
}

/// The refusal for a template the caller may not see, the same as for one that does not exist.
pub fn not_found() -> AppError {
    AppError::not_found("Pafta şablonu bulunamadı.")
}

fn template_id(text: &str, path: &str) -> AppResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| {
        AppError::invalid_at(
            path,
            format!("{path} bir şablon kimliği (UUID) olmalı: {text}"),
        )
    })
}

fn person(text: &str) -> AppResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| {
        AppError::invalid_at(
            "userId",
            format!("userId bir hesap kimliği (UUID) olmalı: {text}"),
        )
    })
}

fn role_of(text: &str) -> Option<TemplateRole> {
    match text {
        "owner" => Some(TemplateRole::Owner),
        "admin" => Some(TemplateRole::Admin),
        "editor" => Some(TemplateRole::Editor),
        "viewer" => Some(TemplateRole::Viewer),
        _ => None,
    }
}

fn grant_role_of(text: &str) -> Option<TemplateGrantRole> {
    match text {
        "editor" => Some(TemplateGrantRole::Editor),
        "viewer" => Some(TemplateGrantRole::Viewer),
        _ => None,
    }
}

fn grant_role_name(role: TemplateGrantRole) -> &'static str {
    match role {
        TemplateGrantRole::Editor => "editor",
        TemplateGrantRole::Viewer => "viewer",
    }
}

fn kind_name(kind: SheetTemplateEventKind) -> &'static str {
    match kind {
        SheetTemplateEventKind::Created => "created",
        SheetTemplateEventKind::Updated => "updated",
        SheetTemplateEventKind::Deleted => "deleted",
        SheetTemplateEventKind::Shared => "shared",
        SheetTemplateEventKind::Unshared => "unshared",
    }
}

fn kind_of(text: &str) -> Option<SheetTemplateEventKind> {
    Some(match text {
        "created" => SheetTemplateEventKind::Created,
        "updated" => SheetTemplateEventKind::Updated,
        "deleted" => SheetTemplateEventKind::Deleted,
        "shared" => SheetTemplateEventKind::Shared,
        "unshared" => SheetTemplateEventKind::Unshared,
        _ => return None,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A paper as the list keeps it: `a3-landscape`.
fn paper_text(p: &PaperChoice) -> String {
    let o = serde_json::to_value(p.orientation)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default();
    format!("{}-{o}", p.paper.id())
}

fn paper_of(text: &str) -> Option<PaperChoice> {
    let (paper, orientation) = text.split_once('-')?;
    Some(PaperChoice {
        paper: serde_json::from_value(Value::String(paper.into())).ok()?,
        orientation: serde_json::from_value(Value::String(orientation.into())).ok()?,
    })
}

/// A contract enum's name, as the lists keep it.
fn name_of<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn named<T: serde::de::DeserializeOwned>(text: &str) -> Option<T> {
    serde_json::from_value(Value::String(text.into())).ok()
}

// ── Commands ─────────────────────────────────────────────────────────────

/// The input of a command, after its name and version are checked.
fn input<T: serde::de::DeserializeOwned>(envelope: &CommandEnvelope) -> AppResult<T> {
    match COMMANDS.iter().find(|(n, _)| *n == envelope.command_name) {
        None => {
            return Err(AppError::invalid(format!(
                "Bilinmeyen komut: {}",
                envelope.command_name
            )));
        }
        Some((name, version)) if envelope.version != *version => {
            return Err(AppError::invalid(format!(
                "{name} komutunun {} sürümü desteklenmiyor (desteklenen: {version}).",
                envelope.version
            )));
        }
        Some(_) => {}
    }
    serde_json::from_value(envelope.input.clone())
        .map_err(|e| AppError::invalid(format!("Komut girdisi okunamadı: {e}")))
}

/// A template file in a command, read and checked as a device reads one.
fn content_of(v: &Value, path: &str) -> AppResult<Template> {
    kentos_sheet::template::read_template(&v.to_string()).map_err(|e| {
        let at = match &e.path {
            Some(p) => format!("{path}.{p}"),
            None => path.to_owned(),
        };
        AppError::invalid_at(at, format!("Şablon geçersiz ({}): {}", e.code, e.message))
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateInput {
    content: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateInput {
    template_id: String,
    expected_revision: u32,
    content: Value,
}

/// The canonical text of a request, whose hash tells a retry from another request with the same key.
fn request_text(envelope: &CommandEnvelope) -> String {
    serde_json::json!({ "command": envelope.command_name, "version": envelope.version, "input": envelope.input }).to_string()
}

fn reused() -> AppError {
    AppError::invalid(
        "Bu idempotency anahtarı başka bir istek için kullanılmış; her komuta yeni bir anahtar verin.",
    )
}

/// The template's row, locked for the command, when the caller has a role in it and it is not deleted.
struct Locked {
    id: Uuid,
    owner: Uuid,
    role: TemplateRole,
    revision: i32,
    name: String,
    created_at: OffsetDateTime,
    /// Its space: a personal space, or an organisation's library.
    tenant: Uuid,
    organization: bool,
}

type LockRow = (
    Uuid,
    Option<String>,
    i32,
    String,
    OffsetDateTime,
    Option<OffsetDateTime>,
    Uuid,
    bool,
);

/// The template's row with the caller's role and whether its space is an organisation (the
/// caller sees an organisation's row only as its member, which the role asks anyway).
const LOCK_SELECT: &str = "select t.owner_user_id, kentos.sheet_template_role(t.id, t.owner_user_id, t.tenant_id), t.revision, t.name,
        t.created_at, t.deleted_at, t.tenant_id,
        exists (select 1 from kentos.tenant k where k.id = t.tenant_id and k.kind = 'organization')
   from kentos.sheet_template t where t.id = $1";

fn locked(id: Uuid, row: Option<LockRow>) -> AppResult<Locked> {
    match row {
        Some((owner, Some(role), revision, name, created_at, None, tenant, organization)) => {
            Ok(Locked {
                id,
                owner,
                role: role_of(&role).ok_or_else(not_found)?,
                revision,
                name,
                created_at,
                tenant,
                organization,
            })
        }
        _ => Err(not_found()),
    }
}

/// An organisation's template is not shared one by one (design §13).
fn not_shared_one_by_one() -> AppError {
    AppError::invalid(
        "Kurum şablonu tek tek paylaşılmaz: kurumun etkin, koltuklu bütün üyeleri onu görür; yayımlayan ve kurum yöneticileri düzenler.",
    )
}

/// A command about a template goes to its space's command route: an organisation's template to
/// that organisation, a person's (one's own, or one shared with one) to one's personal space.
fn on_its_route(access: &Access, t: &Locked) -> AppResult<()> {
    match (t.organization, access.kind) {
        (true, TenantKind::Organization) if access.tenant == t.tenant => Ok(()),
        (true, _) => Err(AppError::invalid(
            "Kurum şablonunun komutu kurumun komut adresine gönderilir: POST /v1/tenants/{kurum}/commands.",
        )),
        (false, TenantKind::Personal) => Ok(()),
        (false, _) => Err(AppError::invalid(
            "Kişisel bir şablonun komutu kişisel alanınızın komut adresine gönderilir; kurumun adresine yalnız kurum şablonlarının komutları gider.",
        )),
    }
}

/// Whether the caller may publish into the organisation now (its owner, administrators and the
/// members who may create projects, with an active membership and a seat).
async fn can_publish(tx: &mut Transaction<'static, Postgres>, tenant: Uuid) -> AppResult<bool> {
    Ok(
        sqlx::query_scalar("select kentos.sheet_template_can_publish($1)")
            .bind(tenant)
            .fetch_one(&mut **tx)
            .await?,
    )
}

fn may_not_publish() -> AppError {
    AppError::forbidden(
        "Bu kurumun şablon kitaplığına yalnız kurum sahibi, yöneticiler ve proje açabilen üyeler şablon yayımlar; kurum yöneticinize başvurun.",
    )
}

/// The template as the caller sees it; locked for the command (and read again under the lock) when they may write it.
/// A viewer's lock would be refused by row-level security (`for update` asks the update policy), so it is not asked.
async fn lock(tx: &mut Transaction<'static, Postgres>, id: Uuid) -> AppResult<Locked> {
    let seen: Option<LockRow> = sqlx::query_as(LOCK_SELECT)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    let seen = locked(id, seen)?;
    if seen.role == TemplateRole::Viewer {
        return Ok(seen);
    }
    let row: Option<LockRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{LOCK_SELECT} for update of t"
    )))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    locked(id, row)
}

/// Everyone who should hear of a change to the template: a person's, its owner and its grantees
/// (an editor does not see the other grants; the database answers for them); an organisation's,
/// its active members with a seat.
async fn audience(tx: &mut Transaction<'static, Postgres>, t: &Locked) -> AppResult<Vec<Uuid>> {
    let mut people = audience_of(tx, t.id).await?;
    if !t.organization && !people.contains(&t.owner) {
        people.push(t.owner);
    }
    people.sort_unstable();
    Ok(people)
}

async fn audience_of(tx: &mut Transaction<'static, Postgres>, id: Uuid) -> AppResult<Vec<Uuid>> {
    let mut people: Vec<Uuid> = sqlx::query_scalar("select kentos.sheet_template_audience($1)")
        .bind(id)
        .fetch_all(&mut **tx)
        .await?;
    people.sort_unstable();
    people.dedup();
    Ok(people)
}

async fn announce(
    tx: &mut Transaction<'static, Postgres>,
    people: &[Uuid],
    template: Uuid,
    revision: i32,
    kind: SheetTemplateEventKind,
    actor: Uuid,
    request: &str,
) -> AppResult<()> {
    for p in people.iter().collect::<BTreeSet<_>>() {
        sqlx::query(
            "insert into kentos.sheet_template_event (user_id, template_id, revision, kind, actor, request_id) values ($1, $2, $3, $4, $5, $6)",
        )
        .bind(p)
        .bind(template)
        .bind(revision)
        .bind(kind_name(kind))
        .bind(actor)
        .bind(request)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn audit(
    tx: &mut Transaction<'static, Postgres>,
    template: Uuid,
    action: &str,
    request: &str,
    revision: i32,
    detail: Value,
) -> AppResult<()> {
    sqlx::query("select kentos.sheet_template_audit($1, $2, $3, $4, $5)")
        .bind(template)
        .bind(action)
        .bind(request)
        .bind(revision)
        .bind(detail)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// The content as it is kept: the template with its id, revision and dates the server's.
struct Stored {
    bytes: Vec<u8>,
    sha: Vec<u8>,
    template: Template,
}

fn stored(
    mut t: Template,
    id: Uuid,
    revision: i32,
    created: OffsetDateTime,
    now: OffsetDateTime,
) -> AppResult<Stored> {
    t.meta.id = id.to_string();
    t.meta.revision = u32::try_from(revision).unwrap_or(u32::MAX);
    t.meta.created = rfc3339(created);
    t.meta.updated = rfc3339(now);
    // The content again, as the server keeps it (an id it may not take is refused like any other).
    kentos_sheet::template::validate_template(&t, false)
        .map_err(|e| AppError::invalid_at("content", e.message))?;
    let bytes =
        serde_json::to_vec(&t).map_err(|e| AppError::invalid(format!("Şablon yazılamadı: {e}")))?;
    if bytes.len() > TEMPLATE_CONTENT_MAX as usize {
        return Err(AppError::invalid_at(
            "content",
            format!(
                "Şablon resimleriyle birlikte en çok 8 MB olabilir; bu şablon {:.1} MB. Resimleri küçültün ya da azaltın.",
                bytes.len() as f64 / 1_048_576.0
            ),
        ));
    }
    let sha = Sha256::digest(&bytes).to_vec();
    Ok(Stored {
        bytes,
        sha,
        template: t,
    })
}

async fn write_revision(
    tx: &mut Transaction<'static, Postgres>,
    id: Uuid,
    revision: i32,
    s: &Stored,
    author: Uuid,
) -> AppResult<()> {
    sqlx::query(
        "insert into kentos.sheet_template_revision (template_id, revision, content, sha256, size, author_user_id) values ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(revision)
    .bind(&s.bytes)
    .bind(&s.sha)
    .bind(s.bytes.len() as i32)
    .bind(author)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn meta_columns(t: &Template) -> (Vec<String>, Vec<String>, Vec<String>) {
    (
        t.meta.papers.iter().map(paper_text).collect(),
        t.meta.workspaces.iter().map(name_of).collect(),
        t.meta.project_types.iter().map(name_of).collect(),
    )
}

/// Runs a template command sent to a space's command route (`access`): the caller's personal
/// space, or an organisation they are a member of (the route has checked the membership).
pub async fn run(
    db: &Db,
    access: &Access,
    envelope: CommandEnvelope,
) -> AppResult<SheetTemplateChanged> {
    if Uuid::parse_str(&envelope.tenant_id).ok() != Some(access.tenant) {
        return Err(AppError::invalid(
            "Komutun çalışma alanı adresteki çalışma alanıyla aynı değil.",
        ));
    }
    if !envelope.project_id.is_empty() {
        return Err(AppError::invalid(
            "Pafta şablonu komutları bir projeye değil, kişisel alana ya da kuruma gönderilir: projectId boş olmalı.",
        ));
    }
    if !(8..=200).contains(&envelope.idempotency_key.len()) || envelope.request_id.len() > 200 {
        return Err(AppError::invalid(
            "idempotencyKey 8–200 karakter, requestId en çok 200 karakter olmalı.",
        ));
    }
    let text = request_text(&envelope);
    let actor = access.actor.user_id;
    let mut tx = db.scoped(access.scope()).await?;
    let earlier: Option<(bool, Value)> = sqlx::query_as(
        "select request_hash = public.digest($3, 'sha256'), response from kentos.sheet_template_command where user_id = $1 and idempotency_key = $2",
    )
    .bind(actor)
    .bind(&envelope.idempotency_key)
    .bind(&text)
    .fetch_optional(&mut *tx)
    .await?;
    match earlier {
        Some((false, _)) => return Err(reused()),
        Some((true, response)) => {
            tx.commit().await?;
            let mut r: SheetTemplateChanged = serde_json::from_value(response)
                .map_err(|e| AppError::invalid(format!("Saklı yanıt okunamadı: {e}")))?;
            r.replayed = true;
            return Ok(r);
        }
        None => {}
    }
    let request = envelope.request_id.as_str();
    let result = match envelope.command_name.as_str() {
        COMMAND_CREATE => create(&mut tx, access, input(&envelope)?, request).await?,
        COMMAND_UPDATE => update(&mut tx, access, input(&envelope)?, request).await?,
        COMMAND_DELETE => delete(&mut tx, access, input(&envelope)?, request).await?,
        COMMAND_SHARE => share(db, &mut tx, access, input(&envelope)?, request).await?,
        COMMAND_UNSHARE => unshare(&mut tx, access, input(&envelope)?, request).await?,
        COMMAND_PUBLISH => publish(&mut tx, access, input(&envelope)?, request).await?,
        other => return Err(AppError::invalid(format!("Bilinmeyen komut: {other}"))),
    };
    let response = serde_json::to_value(&result)
        .map_err(|e| AppError::invalid(format!("Yanıt saklanamadı: {e}")))?;
    let done = sqlx::query(
        "insert into kentos.sheet_template_command (user_id, idempotency_key, command_name, request_hash, response) values ($1, $2, $3, public.digest($4, 'sha256'), $5)",
    )
    .bind(actor)
    .bind(&envelope.idempotency_key)
    .bind(&envelope.command_name)
    .bind(&text)
    .bind(response)
    .execute(&mut *tx)
    .await;
    match done {
        Ok(_) => {}
        // The same key at the same moment from another request.
        Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23505") => {
            return Err(reused());
        }
        Err(e) => return Err(e.into()),
    }
    tx.commit().await?;
    Ok(result)
}

async fn create(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    i: CreateInput,
    request: &str,
) -> AppResult<SheetTemplateChanged> {
    // Into an organisation's library only by those who may publish there.
    if access.kind == TenantKind::Organization && !can_publish(tx, access.tenant).await? {
        return Err(may_not_publish());
    }
    let t = content_of(&i.content, "content")?;
    let name = t.meta.name.clone();
    insert_new(
        tx,
        access,
        t,
        None,
        COMMAND_CREATE,
        request,
        |size| serde_json::json!({ "name": name, "size": size }),
    )
    .await
}

/// A new template of the caller's in the space `access` names, with its first revision: the
/// person's limit, the row (`published_from`: the template an organisation's copy was published
/// from), the revision, the audit record (`detail` of the content's size) and the events of the
/// people who should hear of it.
async fn insert_new(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    t: Template,
    published_from: Option<Uuid>,
    command: &str,
    request: &str,
    detail: impl FnOnce(usize) -> Value,
) -> AppResult<SheetTemplateChanged> {
    let actor = access.actor.user_id;
    // One person's creations one at a time, so two at once cannot both pass the limit.
    sqlx::query("select pg_advisory_xact_lock(hashtextextended('kentos.sheet_template.create:' || $1::text, 0))")
        .bind(actor)
        .execute(&mut **tx)
        .await?;
    let owned: i64 = sqlx::query_scalar("select count(*) from kentos.sheet_template where owner_user_id = $1 and deleted_at is null")
        .bind(actor)
        .fetch_one(&mut **tx)
        .await?;
    if owned >= i64::from(TEMPLATES_PER_PERSON_MAX) {
        return Err(AppError::invalid(format!(
            "En çok {TEMPLATES_PER_PERSON_MAX} pafta şablonunuz olabilir; yenisini kaydetmek için kullanmadıklarınızı silin."
        )));
    }
    let id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();
    let s = stored(t, id, 1, now, now)?;
    let (papers, workspaces, project_types) = meta_columns(&s.template);
    let m = &s.template.meta;
    sqlx::query(
        "insert into kentos.sheet_template (id, tenant_id, owner_user_id, name, description, category, tags, papers, workspaces, project_types,
                                            revision, sha256, size, created_at, updated_at, published_from)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, $11, $12, $13, $13, $14)",
    )
    .bind(id)
    .bind(access.tenant)
    .bind(actor)
    .bind(&m.name)
    .bind(&m.description)
    .bind(&m.category)
    .bind(&m.tags)
    .bind(&papers)
    .bind(&workspaces)
    .bind(&project_types)
    .bind(&s.sha)
    .bind(s.bytes.len() as i32)
    .bind(now)
    .bind(published_from)
    .execute(&mut **tx)
    .await?;
    write_revision(tx, id, 1, &s, actor).await?;
    audit(tx, id, command, request, 1, detail(s.bytes.len())).await?;
    // A person's: its owner; an organisation's: its members.
    let mut people = audience_of(tx, id).await?;
    if !people.contains(&actor) {
        people.push(actor);
    }
    announce(
        tx,
        &people,
        id,
        1,
        SheetTemplateEventKind::Created,
        actor,
        request,
    )
    .await?;
    Ok(SheetTemplateChanged {
        template_id: id.to_string(),
        revision: 1,
        deleted: false,
        changed: true,
        replayed: false,
    })
}

async fn update(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    i: UpdateInput,
    request: &str,
) -> AppResult<SheetTemplateChanged> {
    let actor = access.actor.user_id;
    let id = template_id(&i.template_id, "templateId")?;
    let t = content_of(&i.content, "content")?;
    let now_row = lock(tx, id).await?;
    on_its_route(access, &now_row)?;
    if now_row.role == TemplateRole::Viewer {
        return Err(AppError::forbidden(if now_row.organization {
            "Bu kurum şablonunu yalnız kullanabilirsiniz: yayımlayan ve kurum yöneticileri düzenler. Değiştirmek için “Şablonlarıma kopyala” ile kendinize bir kopyasını alın."
        } else {
            "Bu şablonu yalnız görüntüleyebilirsiniz; yeni revizyon kaydetmek için sahibinden düzenleme yetkisi isteyin ya da şablonu çoğaltın."
        }));
    }
    if i64::from(i.expected_revision) != i64::from(now_row.revision) {
        return Err(AppError::Conflict {
            message: format!(
                "“{}” şablonu siz düzenlerken değişti (şimdi {}. revizyon, siz {}. revizyona dayandınız); hiçbir şey kaydedilmedi. Yeni revizyonu indirip değişikliğinizi onun üstüne yapın ya da kopya olarak kaydedin.",
                now_row.name, now_row.revision, i.expected_revision
            ),
            conflicts: Vec::new(),
            revision: Some(i64::from(now_row.revision)),
        });
    }
    let revision = now_row.revision + 1;
    let now = OffsetDateTime::now_utc();
    let s = stored(t, id, revision, now_row.created_at, now)?;
    let (papers, workspaces, project_types) = meta_columns(&s.template);
    let m = &s.template.meta;
    write_revision(tx, id, revision, &s, actor).await?;
    sqlx::query(
        "update kentos.sheet_template set name = $2, description = $3, category = $4, tags = $5, papers = $6, workspaces = $7, project_types = $8,
                                          revision = $9, sha256 = $10, size = $11, updated_at = $12
          where id = $1",
    )
    .bind(id)
    .bind(&m.name)
    .bind(&m.description)
    .bind(&m.category)
    .bind(&m.tags)
    .bind(&papers)
    .bind(&workspaces)
    .bind(&project_types)
    .bind(revision)
    .bind(&s.sha)
    .bind(s.bytes.len() as i32)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    audit(
        tx,
        id,
        COMMAND_UPDATE,
        request,
        revision,
        serde_json::json!({ "name": m.name, "size": s.bytes.len() }),
    )
    .await?;
    let people = audience(tx, &now_row).await?;
    announce(
        tx,
        &people,
        id,
        revision,
        SheetTemplateEventKind::Updated,
        actor,
        request,
    )
    .await?;
    Ok(SheetTemplateChanged {
        template_id: id.to_string(),
        revision: u32::try_from(revision).unwrap_or(u32::MAX),
        deleted: false,
        changed: true,
        replayed: false,
    })
}

async fn delete(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    i: SheetTemplateDelete,
    request: &str,
) -> AppResult<SheetTemplateChanged> {
    let actor = access.actor.user_id;
    let id = template_id(&i.template_id, "templateId")?;
    let now_row = lock(tx, id).await?;
    on_its_route(access, &now_row)?;
    if !matches!(now_row.role, TemplateRole::Owner | TemplateRole::Admin) {
        return Err(AppError::forbidden(if now_row.organization {
            "Kurum şablonunu yalnız yayımlayan ve kurum yöneticileri siler."
        } else {
            "Pafta şablonunu yalnız sahibi siler; sizinle paylaşılanı listenizden kaldırması için sahibine başvurun."
        }));
    }
    if let Some(expected) = i.expected_revision
        && i64::from(expected) != i64::from(now_row.revision)
    {
        return Err(AppError::Conflict {
            message: format!(
                "“{}” şablonu siz sildikten sonra başka bir yerde değişti ({}. revizyon); silinmedi.",
                now_row.name, now_row.revision
            ),
            conflicts: Vec::new(),
            revision: Some(i64::from(now_row.revision)),
        });
    }
    let people = audience(tx, &now_row).await?;
    sqlx::query(
        "update kentos.sheet_template set deleted_at = now(), updated_at = now() where id = $1",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    audit(
        tx,
        id,
        COMMAND_DELETE,
        request,
        now_row.revision,
        serde_json::json!({ "name": now_row.name }),
    )
    .await?;
    announce(
        tx,
        &people,
        id,
        now_row.revision,
        SheetTemplateEventKind::Deleted,
        actor,
        request,
    )
    .await?;
    Ok(SheetTemplateChanged {
        template_id: id.to_string(),
        revision: u32::try_from(now_row.revision).unwrap_or(u32::MAX),
        deleted: true,
        changed: true,
        replayed: false,
    })
}

/// `sheet.template.publish`: one's own template copied into the organisation's library the
/// command was sent to, as a template of its own (a new id, revision 1, the newest revision's
/// content), its source kept. Those who may not publish there are refused before the source is
/// looked at, so nothing is told of a template they cannot see.
async fn publish(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    i: SheetTemplatePublish,
    request: &str,
) -> AppResult<SheetTemplateChanged> {
    if access.kind != TenantKind::Organization {
        return Err(AppError::invalid(
            "sheet.template.publish bir kuruma gönderilir: POST /v1/tenants/{kurum}/commands; şablon o kurumun kitaplığına kopyalanır.",
        ));
    }
    if Uuid::parse_str(&i.tenant_id).ok() != Some(access.tenant) {
        return Err(AppError::invalid_at(
            "tenantId",
            "tenantId komutun gönderildiği kurum olmalı.",
        ));
    }
    if !can_publish(tx, access.tenant).await? {
        return Err(may_not_publish());
    }
    let source = template_id(&i.template_id, "templateId")?;
    let seen: Option<LockRow> = sqlx::query_as(LOCK_SELECT)
        .bind(source)
        .fetch_optional(&mut **tx)
        .await?;
    let src = locked(source, seen)?;
    if src.role != TemplateRole::Owner || src.organization {
        return Err(AppError::forbidden(
            "Kuruma yalnız kendi şablonunuzu yayımlarsınız; başkasınınkini ya da bir kurumunkini önce “Şablonlarıma kopyala” ile kendinize alın.",
        ));
    }
    let content: Option<Vec<u8>> = sqlx::query_scalar(
        "select content from kentos.sheet_template_revision where template_id = $1 and revision = $2",
    )
    .bind(source)
    .bind(src.revision)
    .fetch_optional(&mut **tx)
    .await?;
    let text = content
        .and_then(|c| String::from_utf8(c).ok())
        .ok_or_else(|| AppError::invalid("Şablonun saklı içeriği okunamadı."))?;
    let t = kentos_sheet::template::read_template(&text).map_err(|e| {
        AppError::invalid(format!(
            "Şablonun saklı içeriği okunamadı ({}): {}",
            e.code, e.message
        ))
    })?;
    let name = t.meta.name.clone();
    let from = src.revision;
    insert_new(
        tx,
        access,
        t,
        Some(source),
        COMMAND_PUBLISH,
        request,
        |size| {
            serde_json::json!({ "name": name, "size": size, "publishedFrom": source, "sourceRevision": from })
        },
    )
    .await
}

/// Whether `user` is an active member of one of the organisations `owner` works in now (docs/adr/0024).
async fn in_common_organisation(db: &Db, owner: &Actor, user: Uuid) -> AppResult<bool> {
    for m in tenancy::memberships(db, owner).await? {
        if m.tenant_kind != TenantKind::Organization || !m.active || !m.seat {
            continue;
        }
        let Ok(tenant) = Uuid::parse_str(&m.tenant_id) else {
            continue;
        };
        let mut tx = db
            .scoped(Scope {
                tenant: Some(tenant),
                user: Some(owner.user_id),
                project: None,
            })
            .await?;
        let found: bool = sqlx::query_scalar(
            "select exists (select 1 from kentos.membership m join kentos.app_user u on u.id = m.user_id
                             where m.tenant_id = $1 and m.user_id = $2 and m.status = 'active' and u.status = 'active')",
        )
        .bind(tenant)
        .bind(user)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        if found {
            return Ok(true);
        }
    }
    Ok(false)
}

fn owner_only(action: &str) -> AppError {
    AppError::forbidden(format!("Pafta şablonunu yalnız sahibi {action}."))
}

async fn share(
    db: &Db,
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    i: SheetTemplateShare,
    request: &str,
) -> AppResult<SheetTemplateChanged> {
    let actor = &access.actor;
    let id = template_id(&i.template_id, "templateId")?;
    let user = person(&i.user_id)?;
    let now_row = lock(tx, id).await?;
    if now_row.organization {
        return Err(not_shared_one_by_one());
    }
    on_its_route(access, &now_row)?;
    if now_row.role != TemplateRole::Owner {
        return Err(owner_only("paylaşır"));
    }
    if user == actor.user_id {
        return Err(AppError::invalid_at(
            "userId",
            "Şablon sizin; kendi rolünüz paylaşımla değişmez.",
        ));
    }
    // Only people of an organisation the owner works in; anyone else (or no such account) reads the same.
    if !in_common_organisation(db, actor, user).await? {
        return Err(AppError::invalid_at(
            "userId",
            "Bu kişi kurumlarınızdan birinin etkin üyesi değil. Pafta şablonları yalnız ortak kurumlardaki kişilerle paylaşılır; kurum dışına paylaşım yok.",
        ));
    }
    let before: Option<String> = sqlx::query_scalar(
        "select role from kentos.sheet_template_grant where template_id = $1 and user_id = $2",
    )
    .bind(id)
    .bind(user)
    .fetch_optional(&mut **tx)
    .await?;
    let role = grant_role_name(i.role);
    let changed = before.as_deref() != Some(role);
    if changed {
        sqlx::query(
            "insert into kentos.sheet_template_grant (template_id, user_id, role, granted_by) values ($1, $2, $3, $4)
             on conflict (template_id, user_id) do update set role = excluded.role, granted_by = excluded.granted_by, updated_at = now()",
        )
        .bind(id)
        .bind(user)
        .bind(role)
        .bind(actor.user_id)
        .execute(&mut **tx)
        .await?;
        audit(
            tx,
            id,
            COMMAND_SHARE,
            request,
            now_row.revision,
            serde_json::json!({ "userId": user, "role": role, "previousRole": before }),
        )
        .await?;
        let people = audience(tx, &now_row).await?;
        announce(
            tx,
            &people,
            id,
            now_row.revision,
            SheetTemplateEventKind::Shared,
            actor.user_id,
            request,
        )
        .await?;
    }
    Ok(SheetTemplateChanged {
        template_id: id.to_string(),
        revision: u32::try_from(now_row.revision).unwrap_or(u32::MAX),
        deleted: false,
        changed,
        replayed: false,
    })
}

async fn unshare(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    i: SheetTemplateUnshare,
    request: &str,
) -> AppResult<SheetTemplateChanged> {
    let actor = access.actor.user_id;
    let id = template_id(&i.template_id, "templateId")?;
    let user = person(&i.user_id)?;
    let now_row = lock(tx, id).await?;
    if now_row.organization {
        return Err(not_shared_one_by_one());
    }
    on_its_route(access, &now_row)?;
    if now_row.role != TemplateRole::Owner {
        return Err(owner_only("paylaşımdan çıkarır"));
    }
    let removed: Option<String> = sqlx::query_scalar("delete from kentos.sheet_template_grant where template_id = $1 and user_id = $2 returning role")
        .bind(id)
        .bind(user)
        .fetch_optional(&mut **tx)
        .await?;
    if let Some(role) = &removed {
        audit(
            tx,
            id,
            COMMAND_UNSHARE,
            request,
            now_row.revision,
            serde_json::json!({ "userId": user, "previousRole": role }),
        )
        .await?;
        let mut people = audience(tx, &now_row).await?;
        people.push(user);
        announce(
            tx,
            &people,
            id,
            now_row.revision,
            SheetTemplateEventKind::Unshared,
            actor,
            request,
        )
        .await?;
    }
    Ok(SheetTemplateChanged {
        template_id: id.to_string(),
        revision: u32::try_from(now_row.revision).unwrap_or(u32::MAX),
        deleted: false,
        changed: removed.is_some(),
        replayed: false,
    })
}

// ── Reading ──────────────────────────────────────────────────────────────

fn user_scope(actor: &Actor) -> Scope {
    Scope {
        tenant: None,
        user: Some(actor.user_id),
        project: None,
    }
}

type SummaryRow = (
    Uuid,
    String,
    String,
    String,
    // Tags, papers, work modes and project types together (a row has at most 16 columns here).
    Value,
    i32,
    Vec<u8>,
    i32,
    Option<String>,
    Uuid,
    Option<String>,
    bool,
    OffsetDateTime,
    OffsetDateTime,
    // Its space and source: `{ tenant, name, kind, from }` (the organisation's name and kind are
    // seen by its members only, who are the only ones with a role in its templates).
    Value,
);

/// Every template the caller may see that is not deleted, with the caller's role, the owner's
/// name, whether it is shared, and the organisation whose library it is in.
const SUMMARY_SELECT: &str = "select t.id, t.name, t.description, t.category, jsonb_build_array(t.tags, t.papers, t.workspaces, t.project_types),
        t.revision, t.sha256, t.size, kentos.sheet_template_role(t.id, t.owner_user_id, t.tenant_id), t.owner_user_id, u.display_name,
        exists (select 1 from kentos.sheet_template_grant g where g.template_id = t.id), t.created_at, t.updated_at,
        jsonb_build_object('tenant', t.tenant_id, 'name', k.name, 'kind', k.kind, 'from', t.published_from)
   from kentos.sheet_template t left join kentos.app_user u on u.id = t.owner_user_id
        left join kentos.tenant k on k.id = t.tenant_id
  where t.deleted_at is null";

fn summary(row: SummaryRow) -> Option<SheetTemplateSummary> {
    let (
        id,
        name,
        description,
        category,
        lists,
        revision,
        sha,
        size,
        role,
        owner,
        owner_name,
        shared,
        created,
        updated,
        place,
    ) = row;
    let text = |k: &str| place.get(k).and_then(Value::as_str).map(str::to_owned);
    let organization = (text("kind").as_deref() == Some("organization"))
        .then(|| {
            Some(CloudOrganization {
                tenant_id: text("tenant")?,
                name: text("name")?,
            })
        })
        .flatten();
    let list = |i: usize| -> Vec<String> {
        serde_json::from_value(lists.get(i).cloned().unwrap_or(Value::Null)).unwrap_or_default()
    };
    let (tags, papers, workspaces, project_types) = (list(0), list(1), list(2), list(3));
    Some(SheetTemplateSummary {
        id: id.to_string(),
        name,
        description,
        category,
        tags,
        papers: papers.iter().filter_map(|p| paper_of(p)).collect(),
        workspaces: workspaces.iter().filter_map(|w| named(w)).collect(),
        project_types: project_types.iter().filter_map(|p| named(p)).collect(),
        revision: u32::try_from(revision).ok()?,
        sha256: hex(&sha),
        size: u32::try_from(size).ok()?,
        role: role_of(role.as_deref()?)?,
        owner_id: owner.to_string(),
        owner_name: owner_name.unwrap_or_default(),
        shared,
        created_at: rfc3339(created),
        updated_at: rfc3339(updated),
        organization,
        published_from: text("from"),
    })
}

/// `GET /v1/me/sheet-templates`: the caller's own templates and the ones shared with them, by
/// name; and the library of every organisation the caller is an active member of with a seat
/// (by the organisation's name), with whether they may publish into it.
pub async fn mine(db: &Db, actor: &Actor) -> AppResult<SheetTemplateList> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let rows: Vec<SummaryRow> = sqlx::query_as(SUMMARY_SELECT).fetch_all(&mut *tx).await?;
    let spaces: Vec<(Uuid, String, bool)> = sqlx::query_as(
        "select t.id, t.name, kentos.sheet_template_can_publish(t.id)
           from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
          where m.user_id = $1 and m.status = 'active' and t.kind = 'organization' and t.status = 'active'
            and exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)",
    )
    .bind(actor.user_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut templates = Vec::new();
    let mut theirs: BTreeMap<String, Vec<SheetTemplateSummary>> = BTreeMap::new();
    for s in rows.into_iter().filter_map(summary) {
        match &s.organization {
            Some(o) => theirs.entry(o.tenant_id.clone()).or_default().push(s),
            None => templates.push(s),
        }
    }
    let by_name = |t: &SheetTemplateSummary| (fold(&t.name), t.id.clone());
    templates.sort_by_cached_key(by_name);
    let mut organizations: Vec<OrganizationTemplates> = spaces
        .into_iter()
        .map(|(id, name, can_publish)| {
            let tenant_id = id.to_string();
            let mut list = theirs.remove(&tenant_id).unwrap_or_default();
            list.sort_by_cached_key(by_name);
            OrganizationTemplates {
                tenant_id,
                name,
                can_publish,
                templates: list,
            }
        })
        .collect();
    organizations.sort_by_cached_key(|o| (fold(&o.name), o.tenant_id.clone()));
    Ok(SheetTemplateList {
        templates,
        organizations,
    })
}

async fn one(tx: &mut Transaction<'static, Postgres>, id: Uuid) -> AppResult<SheetTemplateSummary> {
    let row: Option<SummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SUMMARY_SELECT} and t.id = $1"
    )))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    row.and_then(summary).ok_or_else(not_found)
}

/// `GET /v1/sheet-templates/{id}`: the metadata and the newest content.
pub async fn detail(db: &Db, actor: &Actor, id: Uuid) -> AppResult<SheetTemplateDetail> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let summary = one(&mut tx, id).await?;
    let content: Option<Vec<u8>> =
        sqlx::query_scalar("select content from kentos.sheet_template_revision where template_id = $1 and revision = $2")
            .bind(id)
            .bind(summary.revision as i32)
            .fetch_optional(&mut *tx)
            .await?;
    tx.commit().await?;
    let content = content.ok_or_else(not_found)?;
    let text = String::from_utf8(content)
        .map_err(|_| AppError::invalid("Şablonun saklı içeriği okunamadı."))?;
    let content = kentos_sheet::template::read_template(&text).map_err(|e| {
        AppError::invalid(format!(
            "Şablonun saklı içeriği okunamadı ({}): {}",
            e.code, e.message
        ))
    })?;
    Ok(SheetTemplateDetail { summary, content })
}

/// The template, when the caller is its owner: others who see it are refused, anyone else gets
/// 404; an organisation's template has no sharing of its own.
async fn owned(
    tx: &mut Transaction<'static, Postgres>,
    id: Uuid,
    what: &str,
) -> AppResult<SheetTemplateSummary> {
    let s = one(tx, id).await?;
    if s.organization.is_some() {
        return Err(not_shared_one_by_one());
    }
    if s.role != TemplateRole::Owner {
        return Err(AppError::forbidden(format!(
            "Pafta şablonunun {what} yalnız sahibi görür."
        )));
    }
    Ok(s)
}

/// A grant as the owner reads it: the person, their name and e-mail, the role, who gave it and when.
type GrantRow = (
    Uuid,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    OffsetDateTime,
);

/// `GET /v1/sheet-templates/{id}/access`: whom the owner has shared it with.
pub async fn access_list(db: &Db, actor: &Actor, id: Uuid) -> AppResult<SheetTemplateAccess> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let s = owned(&mut tx, id, "paylaşımını").await?;
    let rows: Vec<GrantRow> = sqlx::query_as(
        "select g.user_id, u.display_name, u.email, g.role, b.display_name, g.created_at
           from kentos.sheet_template_grant g
           left join kentos.app_user u on u.id = g.user_id
           left join kentos.app_user b on b.id = g.granted_by
          where g.template_id = $1",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut grants: Vec<SheetTemplateGrant> = rows
        .into_iter()
        .filter_map(|(user, name, email, role, by, created)| {
            Some(SheetTemplateGrant {
                user_id: user.to_string(),
                display_name: name.unwrap_or_default(),
                email,
                role: grant_role_of(&role)?,
                granted_by_name: by.unwrap_or_default(),
                created_at: rfc3339(created),
            })
        })
        .collect();
    grants.sort_by_cached_key(|g| (fold(&g.display_name), g.user_id.clone()));
    Ok(SheetTemplateAccess {
        owner_id: s.owner_id,
        owner_name: s.owner_name,
        grants,
    })
}

/// Most people one search returns.
pub const CANDIDATES_MAX: i64 = 20;

/// `GET /v1/sheet-templates/{id}/access/candidates?q=`: active members of the
/// owner's organisations whose name or e-mail holds every word, the owner
/// left out; people it is already shared with are not (sharing again changes
/// the role).
pub async fn candidates(
    db: &Db,
    actor: &Actor,
    id: Uuid,
    query: &str,
) -> AppResult<SheetTemplateCandidates> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    owned(&mut tx, id, "paylaşılacak kişilerini").await?;
    tx.commit().await?;
    let patterns = search_patterns(query)?;
    let mut found: BTreeMap<Uuid, SheetTemplateCandidate> = BTreeMap::new();
    for m in tenancy::memberships(db, actor).await? {
        if m.tenant_kind != TenantKind::Organization || !m.active || !m.seat {
            continue;
        }
        let Ok(tenant) = Uuid::parse_str(&m.tenant_id) else {
            continue;
        };
        let mut tx = db
            .scoped(Scope {
                tenant: Some(tenant),
                user: Some(actor.user_id),
                project: None,
            })
            .await?;
        let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            "select u.id, u.display_name, u.email
               from kentos.membership m join kentos.app_user u on u.id = m.user_id
              where m.tenant_id = $1 and m.status = 'active' and u.status = 'active' and u.id <> $2
                and lower(translate(u.display_name || ' ' || coalesce(u.email, ''), $3, $4)) like all ($5::text[])
              order by lower(translate(u.display_name, $3, $4)), u.id
              limit $6",
        )
        .bind(tenant)
        .bind(actor.user_id)
        .bind(FOLD_FROM)
        .bind(FOLD_TO)
        .bind(&patterns)
        .bind(CANDIDATES_MAX)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        for (user, name, email) in rows {
            found.entry(user).or_insert(SheetTemplateCandidate {
                user_id: user.to_string(),
                display_name: name,
                email,
            });
        }
    }
    let mut candidates: Vec<SheetTemplateCandidate> = found.into_values().collect();
    candidates.sort_by_cached_key(|c| (fold(&c.display_name), c.user_id.clone()));
    candidates.truncate(CANDIDATES_MAX as usize);
    Ok(SheetTemplateCandidates { candidates })
}

/// `GET /v1/me/sheet-templates/events?after=&limit=`: the caller's events after the cursor, oldest first.
pub async fn events_after(
    db: &Db,
    actor: &Actor,
    after: i64,
    limit: i64,
) -> AppResult<SheetTemplateEventPage> {
    let limit = limit.clamp(1, EVENTS_PAGE_MAX);
    let mut tx = db.scoped(user_scope(actor)).await?;
    let rows: Vec<(i64, Uuid, i32, String, Option<Uuid>)> = sqlx::query_as(
        "select seq, template_id, revision, kind, actor from kentos.sheet_template_event where user_id = $1 and seq > $2 order by seq limit $3",
    )
    .bind(actor.user_id)
    .bind(after)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let next = rows.last().map_or(after, |r| r.0);
    Ok(SheetTemplateEventPage {
        events: rows
            .into_iter()
            .filter_map(|(seq, template, revision, kind, by)| {
                Some(SheetTemplateEvent {
                    seq: seq.to_string(),
                    template_id: template.to_string(),
                    revision: u32::try_from(revision).ok()?,
                    kind: kind_of(&kind)?,
                    actor: by.map(|a| a.to_string()),
                })
            })
            .collect(),
        next: next.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_sheet::{Orientation, Paper};

    #[test]
    fn papers_and_names_are_kept_as_text_and_read_back() {
        let p = PaperChoice {
            paper: Paper::A3,
            orientation: Orientation::Landscape,
        };
        assert_eq!(paper_text(&p), "a3-landscape");
        assert_eq!(paper_of("a3-landscape"), Some(p));
        assert_eq!(paper_of("a3"), None);
        assert_eq!(name_of(&kentos_contracts::Workspace::Gis), "gis");
        assert_eq!(
            named::<kentos_contracts::ProjectType>("zoningPlan"),
            Some(kentos_contracts::ProjectType::ZoningPlan)
        );
        for k in [
            SheetTemplateEventKind::Created,
            SheetTemplateEventKind::Updated,
            SheetTemplateEventKind::Deleted,
            SheetTemplateEventKind::Shared,
            SheetTemplateEventKind::Unshared,
        ] {
            assert_eq!(kind_of(kind_name(k)), Some(k));
        }
        assert!(handles("sheet.template.share") && !handles("project.share"));
    }
}

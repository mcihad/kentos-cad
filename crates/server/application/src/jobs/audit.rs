//! `project.audit` (docs/adr/8057 §42, the server's part): an extension's audit rule, a published release of the
//! catalog, run on a database project through the host's processor, as the desktop runs it; its report kept as the
//! job's result.
//!
//! - **The release** comes from the host's audit source (`Processor::audits`): the organisation's catalog, else (when
//!   it does not know the extension) the public one; the version asked when it may run, else the newest published one.
//!   Its package goes beside the request (`extension.kek`) and the job runner checks it against the digest the catalog
//!   gave.
//! - **The interpreter** is the one the server prepared for that version (its setting); without one the job ends with
//!   `tool_unavailable` before anything runs.
//! - **The cache:** a job whose release, rule, project state, reach and software are an earlier succeeded job's takes
//!   that job's report ([`crate::job_cache::audit_key`]); one asked `fresh` runs again.
//! - **The result** is the job runner's report with the catalog, the revision read and what the run took; a report of
//!   another release or rule than the one asked is refused.

use kentos_contracts::audit_job::{AuditReport, AuditRun};
use kentos_contracts::project_audit::{
    ProjectAuditInput, ProjectAuditResult, check_project_audit_input,
};
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Cached, Claim, External, Outcome};
use crate::access::ProjectAccess;
use crate::error::{AppError, AppResult};
use crate::processing_job::{Processed, Processor, RunRequest};
use crate::snapshot::ProjectSnapshot;

/// What the worker decided for an audit before it reads the project: the catalog its release came from and what the
/// cache key is made of (the release and the rule).
pub(super) struct Planned {
    pub catalog: String,
    pub key_input: Value,
}

/// A `project.audit` input checked when it is asked for.
pub(super) fn checked(input: &Value) -> AppResult<Value> {
    let parsed: ProjectAuditInput = serde_json::from_value(input.clone()).map_err(|e| {
        AppError::invalid_at("input", format!("project.audit'in girdisi okunamadı: {e}"))
    })?;
    if let Some((path, message)) = check_project_audit_input(&parsed) {
        return Err(AppError::invalid_at(path, message));
    }
    Ok(input.clone())
}

/// A job's end before anything ran, boxed: an outcome is large and made once a job.
fn failed(code: &'static str, message: impl Into<String>) -> Box<Outcome> {
    Box::new(Outcome::Failed {
        code,
        message: message.into(),
    })
}

/// The audit's request: its release taken from the host's catalog for the organisation the job is in, its package and
/// interpreter (none when the server prepared none; said before the run, after the cache is asked).
pub(super) async fn external(
    db: &kentos_postgres::Db,
    claim: &Claim,
    access: &ProjectAccess,
    processor: &std::sync::Arc<dyn Processor>,
) -> Result<External, Box<Outcome>> {
    let input: ProjectAuditInput = serde_json::from_value(claim.input.clone())
        .map_err(|e| failed("invalid", format!("İşin girdisi okunamadı: {e}")))?;
    let Some(source) = processor.audits() else {
        return Err(failed(
            "tool_unavailable",
            "Bu sunucuda eklenti kataloğu açık değil; denetim işi çalıştırılamaz.",
        ));
    };
    let slug = tenant_slug(db, access).await.ok_or_else(|| {
        failed(
            "failed",
            "İşin kurumunun kısa adı okunamadı; iş yeniden denenecek.",
        )
    })?;
    let package = source
        .release(&slug, &input.extension, input.version.as_deref())
        .await
        .map_err(|why| failed("invalid", why))?;
    let python = source.python(&input.extension, &package.version);
    let key_input = json!({
        "extension": input.extension,
        "version": package.version,
        "digest": package.digest,
        "rule": input.rule,
    });
    Ok(External {
        writes: false,
        downloads: false,
        expected_revision: input.expected_revision.clone(),
        cpu_seconds: None,
        memory_mb: None,
        request: RunRequest::Audit(AuditRun {
            extension: input.extension,
            version: package.version,
            digest: package.digest,
            rule: input.rule,
            python,
            timeout_seconds: None,
        }),
        staged: None,
        package: Some(package.bytes),
        audit: Some(Planned {
            catalog: package.catalog,
            key_input,
        }),
    })
}

/// The organisation's short name, as the catalog names its catalog (`tenant:{slug}`).
async fn tenant_slug(db: &kentos_postgres::Db, access: &ProjectAccess) -> Option<String> {
    let mut tx = db.scoped(access.scope()).await.ok()?;
    let slug: Option<String> = sqlx::query_scalar("select slug from kentos.tenant where id = $1")
        .bind(access.tenant)
        .fetch_optional(&mut *tx)
        .await
        .ok()?;
    let _ = tx.rollback().await;
    slug
}

/// Before an audit runs, once the project is read: the job is done with an earlier succeeded job's report when its key
/// is this one's (the same release, rule, project state, reach and software), else it must have the interpreter the
/// server prepared for its release; the key it keeps when it runs. None for any other work.
pub(super) async fn before_run(
    db: &kentos_postgres::Db,
    claim: &Claim,
    access: &ProjectAccess,
    work: &External,
    snap: &ProjectSnapshot,
    software: &str,
) -> Result<Option<String>, Box<Outcome>> {
    let (Some(planned), RunRequest::Audit(run)) = (&work.audit, &work.request) else {
        return Ok(None);
    };
    let key = crate::job_cache::audit_key(
        &planned.key_input,
        &crate::job_cache::State {
            revision: snap.revision,
            cursor: snap.event_cursor,
            reach: snap.reach.as_deref(),
            user: "",
            day: "",
            software,
        },
    );
    if let Some((from, result)) = earlier(db, claim, access, &key).await {
        return Err(Box::new(Outcome::Done {
            result,
            artifact: None,
            reach: snap.reach.clone(),
            commit: None,
            cache: Some(Cached {
                key,
                from: Some(from),
            }),
        }));
    }
    if run.python.is_none() {
        return Err(failed(
            "tool_unavailable",
            format!(
                "Bu sunucuda {} {} için hazırlanmış bir Python ortamı yok; denetim çalıştırılamadı. Sunucu yöneticisi sürümün ortamını hazırlayıp denetim ortamlarının listesine eklemeli.",
                run.extension, run.version
            ),
        ));
    }
    Ok(Some(key))
}

/// An earlier succeeded audit of the project with `key`, the job and its result; none when this job was asked `fresh`
/// or there is none.
async fn earlier(
    db: &kentos_postgres::Db,
    claim: &Claim,
    access: &ProjectAccess,
    key: &str,
) -> Option<(Uuid, Value)> {
    let mut tx = db.snapshot(access.scope()).await.ok()?;
    let fresh: bool = sqlx::query_scalar(
        "select fresh from kentos.server_job where tenant_id = $1 and project_id = $2 and id = $3",
    )
    .bind(claim.tenant)
    .bind(claim.project)
    .bind(claim.id)
    .fetch_optional(&mut *tx)
    .await
    .ok()??;
    if fresh {
        return None;
    }
    let found: Option<(Uuid, Option<Value>)> = sqlx::query_as(
        "select id, result from kentos.server_job
          where tenant_id = $1 and project_id = $2 and kind = 'project.audit' and cache_key = $3
            and state = 'succeeded' and result is not null and id <> $4
          order by finished_at desc limit 1",
    )
    .bind(claim.tenant)
    .bind(claim.project)
    .bind(key)
    .bind(claim.id)
    .fetch_optional(&mut *tx)
    .await
    .ok()?;
    let _ = tx.rollback().await;
    let (id, result) = found?;
    Some((id, result?))
}

/// The job's result from what the job runner answered: its report, of the release and rule asked, with the catalog,
/// the revision read and what the run took.
pub(super) fn result(
    processed: &Processed,
    run: &AuditRun,
    planned: &Planned,
    base_revision: i64,
) -> Result<Value, String> {
    let report: AuditReport = serde_json::from_value(processed.details.clone())
        .map_err(|e| format!("İşçi denetimin raporunu vermedi: {e}"))?;
    if report.rule.digest != run.digest || report.rule.rule != run.rule {
        return Err(format!(
            "İşçinin raporu istenen kuralın değil ({} {}); sonuç saklanmadı.",
            report.rule.rule, report.rule.digest
        ));
    }
    let result = ProjectAuditResult {
        catalog: planned.catalog.clone(),
        base_revision: base_revision.to_string(),
        summary: processed.summary.clone(),
        report,
        usage: processed.usage.clone(),
    };
    serde_json::to_value(&result).map_err(|e| format!("Sonuç yazılamadı: {e}"))
}

#[cfg(test)]
mod tests;

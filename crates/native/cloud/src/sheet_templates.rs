//! The sheet template library from the desktop (docs/sheet/design.md §13,
//! docs/adr/0040): the routes and commands the web uses, through this
//! crate's HTTP. A person's templates are kept in the account's personal
//! space, an organisation's in that organisation's library: each command goes
//! to its template's space's command route (`space`), with an idempotency
//! key the caller keeps for a retry; `sheet.template.publish` copies one of
//! the account's own into an organisation. What changed elsewhere comes as the account's
//! template events after a cursor, waiting on the server for the next one
//! (a long poll, as a project's events, docs/adr/0044). Which way a
//! template goes on a sync is `kentos_sheet::sync::plan_sync`'s; the
//! network is this module's.

use std::collections::BTreeMap;
use std::future::Future;
use std::time::Duration;

use kentos_contracts::TenantKind;
use kentos_sheet::cloud::{
    COMMAND_CREATE, COMMAND_DELETE, COMMAND_PUBLISH, COMMAND_SHARE, COMMAND_UNSHARE,
    COMMAND_UPDATE, COMMAND_VERSION, SheetTemplateAccess, SheetTemplateCandidates,
    SheetTemplateChanged, SheetTemplateDelete, SheetTemplateDetail, SheetTemplateEventPage,
    SheetTemplateList, SheetTemplatePublish, SheetTemplateShare, SheetTemplateUnshare,
    TemplateGrantRole,
};
use kentos_sheet::sync::RemoteTemplate;
use kentos_sheet::template::Template;
use uuid::Uuid;

use crate::api::Cloud;
use crate::failure::ApiFailure;
use crate::runtime::run;

/// How long one request for events waits on the server for a change (the server's most).
pub const WAIT: Duration = Duration::from_secs(25);

/// The account's templates, the ones shared with it, and the libraries of its organisations
/// (`GET /v1/me/sheet-templates`).
pub fn list(
    cloud: &Cloud,
) -> impl Future<Output = Result<SheetTemplateList, ApiFailure>> + Send + 'static {
    cloud.get_at("/v1/me/sheet-templates", Vec::new(), None)
}

/// One template with its newest content (`GET /v1/sheet-templates/{id}`).
pub fn detail(
    cloud: &Cloud,
    id: Uuid,
) -> impl Future<Output = Result<SheetTemplateDetail, ApiFailure>> + Send + 'static {
    cloud.get_at(&format!("/v1/sheet-templates/{id}"), Vec::new(), None)
}

/// Whom the account has shared its template with (the owner only).
pub fn access(
    cloud: &Cloud,
    id: Uuid,
) -> impl Future<Output = Result<SheetTemplateAccess, ApiFailure>> + Send + 'static {
    cloud.get_at(
        &format!("/v1/sheet-templates/{id}/access"),
        Vec::new(),
        None,
    )
}

/// People of the account's organisations to share its template with, by name or e-mail (docs/adr/0024).
pub fn candidates(
    cloud: &Cloud,
    id: Uuid,
    query: &str,
) -> impl Future<Output = Result<SheetTemplateCandidates, ApiFailure>> + Send + 'static {
    cloud.get_at(
        &format!("/v1/sheet-templates/{id}/access/candidates"),
        vec![("q", query.to_owned())],
        None,
    )
}

/// The account's template events after `after`, waiting up to [`WAIT`] for one when none is new.
pub fn events(
    cloud: &Cloud,
    after: &str,
    wait: bool,
) -> impl Future<Output = Result<SheetTemplateEventPage, ApiFailure>> + Send + 'static {
    let mut query = vec![("after", after.to_owned())];
    let mut timeout = None;
    if wait {
        query.push(("wait", WAIT.as_secs().to_string()));
        // The request may take the whole wait, and a little more.
        timeout = Some(WAIT + Duration::from_secs(10));
    }
    cloud.get_at("/v1/me/sheet-templates/events", query, timeout)
}

/// The account's personal space: where its templates are kept and its template commands go.
pub fn personal_space(
    cloud: &Cloud,
) -> impl Future<Output = Result<Uuid, ApiFailure>> + Send + 'static {
    let me = cloud.me();
    run(async move {
        let me = me.await?;
        me.memberships
            .iter()
            .find(|m| m.tenant_kind == TenantKind::Personal)
            .and_then(|m| Uuid::parse_str(&m.tenant_id).ok())
            .ok_or_else(|| {
                ApiFailure::local(
                    "Hesabın kişisel alanı bulunamadı; sunucu bu sürümle uyuşmuyor olabilir.",
                )
            })
    })
}

/// A template command to a space's command route: the personal space, or an organisation.
fn command(
    cloud: &Cloud,
    space: Uuid,
    name: &str,
    key: Uuid,
    input: serde_json::Value,
) -> impl Future<Output = Result<SheetTemplateChanged, ApiFailure>> + Send + 'static {
    let envelope = crate::saving::envelope(
        space,
        Uuid::nil(),
        name,
        COMMAND_VERSION,
        key,
        BTreeMap::new(),
        input,
    );
    // A template command names no project.
    let envelope = kentos_contracts::CommandEnvelope {
        project_id: String::new(),
        ..envelope
    };
    cloud.tenant_command(envelope)
}

fn value<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

/// `sheet.template.create`: the template as a new one of the account's, in `space` (its personal
/// space, or an organisation it may publish into); the answer names its id.
pub fn create(
    cloud: &Cloud,
    space: Uuid,
    content: &Template,
    key: Uuid,
) -> impl Future<Output = Result<SheetTemplateChanged, ApiFailure>> + Send + 'static {
    command(
        cloud,
        space,
        COMMAND_CREATE,
        key,
        serde_json::json!({ "content": value(content) }),
    )
}

/// `sheet.template.update`: a new revision based on `expected` (a later one answers 409).
pub fn update(
    cloud: &Cloud,
    space: Uuid,
    id: Uuid,
    expected: u32,
    content: &Template,
    key: Uuid,
) -> impl Future<Output = Result<SheetTemplateChanged, ApiFailure>> + Send + 'static {
    command(
        cloud,
        space,
        COMMAND_UPDATE,
        key,
        serde_json::json!({ "templateId": id.to_string(), "expectedRevision": expected, "content": value(content) }),
    )
}

/// `sheet.template.delete` (the owner only); with `expected`, a later revision answers 409.
pub fn delete(
    cloud: &Cloud,
    space: Uuid,
    id: Uuid,
    expected: Option<u32>,
    key: Uuid,
) -> impl Future<Output = Result<SheetTemplateChanged, ApiFailure>> + Send + 'static {
    let input = SheetTemplateDelete {
        template_id: id.to_string(),
        expected_revision: expected,
    };
    command(cloud, space, COMMAND_DELETE, key, value(&input))
}

/// `sheet.template.share` (the owner only, with people of a common organisation).
pub fn share(
    cloud: &Cloud,
    space: Uuid,
    id: Uuid,
    user: Uuid,
    role: TemplateGrantRole,
    key: Uuid,
) -> impl Future<Output = Result<SheetTemplateChanged, ApiFailure>> + Send + 'static {
    let input = SheetTemplateShare {
        template_id: id.to_string(),
        user_id: user.to_string(),
        role,
    };
    command(cloud, space, COMMAND_SHARE, key, value(&input))
}

/// `sheet.template.unshare` (the owner only).
pub fn unshare(
    cloud: &Cloud,
    space: Uuid,
    id: Uuid,
    user: Uuid,
    key: Uuid,
) -> impl Future<Output = Result<SheetTemplateChanged, ApiFailure>> + Send + 'static {
    let input = SheetTemplateUnshare {
        template_id: id.to_string(),
        user_id: user.to_string(),
    };
    command(cloud, space, COMMAND_UNSHARE, key, value(&input))
}

/// `sheet.template.publish`: one of the account's own templates copied into the organisation
/// `org` (its command route), as a template of the organisation's; the answer names the new one.
pub fn publish(
    cloud: &Cloud,
    org: Uuid,
    id: Uuid,
    key: Uuid,
) -> impl Future<Output = Result<SheetTemplateChanged, ApiFailure>> + Send + 'static {
    let input = SheetTemplatePublish {
        template_id: id.to_string(),
        tenant_id: org.to_string(),
    };
    command(cloud, org, COMMAND_PUBLISH, key, value(&input))
}

/// The cloud's side of a sync plan: every template of the list (the organisations' too) as
/// `plan_sync` takes it.
pub fn remote(list: &SheetTemplateList) -> Vec<RemoteTemplate> {
    list.every()
        .map(|t| RemoteTemplate {
            id: t.id.clone(),
            name: t.name.clone(),
            revision: t.revision,
            deleted: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_is_the_remote_side_of_a_sync() {
        let list: SheetTemplateList = serde_json::from_value(serde_json::json!({ "templates": [{
            "id": "t1", "name": "Ada", "description": "", "category": "", "tags": [], "papers": [], "workspaces": [], "projectTypes": [],
            "revision": 3, "sha256": "00", "size": 1, "role": "owner", "ownerId": "u", "ownerName": "", "shared": false,
            "createdAt": "2026-10-02T00:00:00Z", "updatedAt": "2026-10-02T00:00:00Z" }] }))
        .unwrap();
        let r = remote(&list);
        assert_eq!(
            (r[0].id.as_str(), r[0].revision, r[0].deleted),
            ("t1", 3, false)
        );
        // A device that never uploaded it creates it; one whose copy is older downloads.
        let plan = kentos_sheet::sync::plan_sync(&[], &r);
        assert_eq!(
            serde_json::to_value(&plan.actions).unwrap()[0]["type"],
            "download"
        );
    }

    #[test]
    fn an_organisation_s_library_is_on_the_remote_side_too() {
        let summary = |id: &str, revision: u32| {
            serde_json::json!({ "id": id, "name": id, "description": "", "category": "", "tags": [], "papers": [], "workspaces": [], "projectTypes": [],
                "revision": revision, "sha256": "00", "size": 1, "role": "viewer", "ownerId": "u", "ownerName": "Ayşe", "shared": false,
                "createdAt": "2026-10-03T00:00:00Z", "updatedAt": "2026-10-03T00:00:00Z",
                "organization": { "tenantId": "buro", "name": "Harita Bürosu" } })
        };
        let list: SheetTemplateList = serde_json::from_value(serde_json::json!({
            "templates": [],
            "organizations": [{ "tenantId": "buro", "name": "Harita Bürosu", "canPublish": false, "templates": [summary("k1", 2)] }] }))
        .unwrap();
        let r = remote(&list);
        assert_eq!(
            r.iter()
                .map(|t| (t.id.as_str(), t.revision))
                .collect::<Vec<_>>(),
            [("k1", 2)]
        );
        let orgs = kentos_sheet::cloud::DeviceOrganization::of_list(&list);
        assert_eq!(
            orgs.iter()
                .map(|o| (o.name.as_str(), o.can_publish))
                .collect::<Vec<_>>(),
            [("Harita Bürosu", false)]
        );
    }
}

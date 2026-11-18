//! Portal dosyaları' test data (docs/adr/9001), written as the server sends it: JSON read into the contracts' types,
//! so a field the server adds later with its default (a file's scan state) does not stop the tests. The names and
//! the dossiers are the demo's (apps/portal/scripts/seed.mjs): Ayşe Yılmaz, architect; Örnek Belediyesi.

use kentos_commands::{
    DossierAction, DossierDetail, DossierDirection, DossierFileView, DossierStatus, DossierSummary,
    OrganizationCategory, PortalCatalog, PortalMe, PortalMembership,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub(super) const MUNICIPALITY: &str = "01a1ea91-fa5d-7693-b241-c3e515a0d28b";
pub(super) const TRANSFER: &str = "01a1ea92-0b91-777f-8c49-0a95082ec501";
pub(super) const APPLICATION: &str = "01a1ea92-0a33-743e-b4fe-8cd177ad6d91";
pub(super) const KCAD: &str = "01a1ea92-0b91-777f-8c49-0a95082ec5f1";
pub(super) const PDF: &str = "01a1ea92-0b91-777f-8c49-0a95082ec5f2";
const USER: &str = "01a1ea92-0275-771c-8fce-f9e1503b012f";

fn read<T: DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).expect("the server's shape")
}

fn code<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).expect("a code")
}

pub(super) fn membership(
    tenant: &str,
    name: &str,
    category: OrganizationCategory,
    usable: bool,
) -> PortalMembership {
    read(json!({
        "tenantId": tenant, "tenantSlug": name.to_lowercase(), "tenantName": name, "category": code(category),
        "role": "project_manager", "usable": usable, "province": "Örnek", "district": "Merkez"
    }))
}

pub(super) fn professional_me() -> PortalMe {
    read(json!({
        "user": { "id": USER, "displayName": "Ayşe Yılmaz", "email": null, "method": "local" },
        "login": "ayse.yilmaz",
        "platformAdmin": false,
        "professional": {
            "userId": USER, "displayName": "Ayşe Yılmaz", "email": "ayse.yilmaz@ornek-posta.test",
            "profession": "architect", "chamberNo": "41234", "officeName": "Yılmaz Mimarlık", "phone": null,
            "province": "Örnek", "listed": true, "status": "verified", "statusNote": "",
            "createdAt": "2026-11-17T17:33:19Z", "reviewedAt": null
        },
        "organizations": [],
        "unread": 0
    }))
}

pub(super) fn catalog() -> PortalCatalog {
    read(json!({
        "applicationKinds": [], "transferKinds": [], "docTypes": [], "professionalStatuses": [],
        "professions": [{ "code": "architect", "label": "Mimar", "chamber": "Mimarlar Odası" }],
        "statuses": [
            { "code": "sent", "label": "Yanıt bekleniyor", "tone": "info" },
            { "code": "accepted", "label": "Üstlenildi", "tone": "info" },
            { "code": "changes_requested", "label": "Düzeltme istendi", "tone": "warning" },
            { "code": "declined", "label": "Üstlenilmedi", "tone": "danger" }
        ]
    }))
}

pub(super) fn summary(
    id: &str,
    direction: DossierDirection,
    status: DossierStatus,
) -> DossierSummary {
    let transfer = direction == DossierDirection::Transfer;
    let (number, title, kind, label) = if transfer {
        (
            "2026/0007",
            "Belediye hizmet binası cephe düzenlemesi",
            "Proje düzeltme",
            "Yanıt bekleniyor",
        )
    } else {
        (
            "2026/0003",
            "105 ada 2 parsel yapı kullanma izni",
            "Yapı kullanma izni",
            "Düzeltme istendi",
        )
    };
    read(json!({
        "id": id, "direction": code(direction), "number": number, "kind": "kind", "kindLabel": kind,
        "title": title, "status": code(status), "statusLabel": label, "tenantId": MUNICIPALITY,
        "tenantSlug": "ornek-belediyesi", "tenantName": "Örnek Belediyesi", "professionalId": USER,
        "professionalName": "Ayşe Yılmaz", "profession": "architect", "assigneeName": null, "round": 1, "site": {},
        "createdAt": "2026-11-17T17:33:21Z", "updatedAt": "2026-11-17T17:33:22Z",
        "sentAt": "2026-11-17T17:33:22Z", "dueOn": if transfer { json!("2026-11-27") } else { Value::Null },
        "clock": null, "awaitingMe": true
    }))
}

pub(super) fn file(id: &str, name: &str, project: bool, state: &str) -> DossierFileView {
    read(json!({
        "id": id, "side": "municipality", "purpose": "package", "round": 1, "state": state,
        "docType": if project { "kentos_project" } else { "other" },
        "docTypeLabel": if project { "KentOS projesi (KCAD)" } else { "Diğer belge" },
        "name": name, "mediaType": "application/octet-stream", "size": 2048, "sha256": "ab".repeat(32),
        "addedByName": "Ali Çelik", "addedAt": "2026-11-17T17:33:22Z", "source": null, "isProject": project
    }))
}

/// The open file project's revision 3 (the cloud tests' project), as the package lists it once it went in.
pub(super) fn added_file() -> DossierFileView {
    let mut added = file(
        "01a1ea92-0b91-777f-8c49-0a95082ec5f9",
        "Ada 101.kcad",
        true,
        "pending",
    );
    added.source = Some(read(json!({
        "tenantId": crate::cloud::tests::TENANT, "projectId": crate::cloud::tests::PROJECT,
        "projectName": "Ada 101", "revision": "3"
    })));
    added
}

pub(super) fn detail(s: DossierSummary, actions: Vec<DossierAction>) -> DossierDetail {
    let pending_round = s.round + 1;
    read(json!({
        "summary": s,
        "description": "Ana cephede pencere oranlarının ve giriş saçağının düzeltilmesi.",
        "ownerName": null, "side": "professional", "actions": actions, "canEdit": false, "canAddFiles": false,
        "canMessage": true, "canDelete": false, "canImport": true, "pendingRound": pending_round,
        "rounds": [{
            "round": 1, "side": "municipality", "sentAt": "2026-11-17T17:33:22Z",
            "files": [file(KCAD, "Cephe.kcad", true, "final"), file(PDF, "cephe-notlari.pdf", false, "final")]
        }],
        "pending": [], "messages": [], "events": [], "checklist": null, "professional": null,
        "organization": {
            "tenantId": MUNICIPALITY, "slug": "ornek-belediyesi", "name": "Örnek Belediyesi", "province": "Örnek",
            "district": "Merkez", "contactEmail": null, "contactPhone": null
        },
        "assignees": []
    }))
}

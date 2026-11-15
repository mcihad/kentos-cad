//! The revocation list from a catalog's releases: suspended and withdrawn published versions only, a suspension's
//! words, the order, and not one field more than the v1 reader knows.

use std::collections::BTreeSet;

use super::*;
use crate::integrity::{FileDigest, sha256_hex};
use crate::manifest::Manifest;
use crate::release::{
    Actor, CatalogScope, Channel, GateKind, ReleaseFacts, Role, ScanOutcome, Timestamp,
};

const MANIFEST: &str =
    include_str!("../../../../../../fixtures/extensions/v1/ornek-altlik/manifest.json");
const PUBLISHER: &str = "ornek-belediye";

fn facts(id: &str, version: &str) -> ReleaseFacts {
    let mut manifest: Manifest = serde_json::from_str(MANIFEST).expect("the example's manifest");
    manifest.id = id.to_owned();
    manifest.version = version.to_owned();
    let file = FileDigest {
        path: "manifest.json".into(),
        size: 1,
        sha256: sha256_hex(format!("{id} {version}").as_bytes()),
    };
    ReleaseFacts {
        manifest,
        files: vec![file],
        python: None,
        sbom: None,
    }
}

fn author() -> Actor {
    Actor::new("ayse", &[Role::Publisher], Channel::Window).member_of(PUBLISHER)
}

fn reviewer() -> Actor {
    Actor::new("selin", &[Role::Reviewer], Channel::Window)
}

fn admin() -> Actor {
    Actor::new("yonetici", &[Role::CatalogAdmin], Channel::Window)
}

fn t(minute: u64) -> Timestamp {
    Timestamp(1_763_190_000_000 + minute * 60_000)
}

fn draft(id: &str, version: &str) -> Release {
    Release::upload(
        &author(),
        CatalogScope::Public,
        PUBLISHER,
        &facts(id, version),
        &[],
        t(0),
    )
    .unwrap()
}

/// Every gate clean, sent, reviewed and approved; not yet published.
fn approved(id: &str, version: &str) -> Release {
    let mut release = draft(id, version);
    let scanner = Actor::new("denetim", &[Role::Scanner], Channel::Program);
    let content = release.content().package.clone();
    for gate in GateKind::ALL {
        release
            .record_scan(&scanner, gate, &content, ScanOutcome::Passed, t(1))
            .unwrap();
    }
    release.submit(&author(), None, t(2)).unwrap();
    release
        .start_review(&reviewer(), &facts(id, version), None, false, t(3))
        .unwrap();
    release.approve(&reviewer(), None, t(4)).unwrap();
    release
}

fn published(id: &str, version: &str) -> Release {
    let mut release = approved(id, version);
    release.publish(&author(), t(5)).unwrap();
    release
}

fn withdrawn(id: &str, version: &str, reason: &str) -> Release {
    let mut release = published(id, version);
    release.withdraw(&admin(), reason, t(6)).unwrap();
    release
}

fn suspended(id: &str, version: &str, reason: &str) -> Release {
    let mut release = published(id, version);
    release.suspend(&admin(), reason, t(6)).unwrap();
    release
}

fn entry(id: &str, versions: &str, reason: &str) -> Revoked {
    Revoked {
        id: id.to_owned(),
        versions: versions.to_owned(),
        reason: reason.to_owned(),
    }
}

#[test]
fn a_suspended_release_is_listed_until_it_is_put_back() {
    let mut release = suspended("tr.ornek.altlik", "1.0.0", "Şüpheli ağ trafiği bildirildi.");
    let list = revocation_list(std::slice::from_ref(&release));
    let reason = "Şüpheli ağ trafiği bildirildi (askıya alındı; inceleme bitince askıdan alınabilir ya da geri çekilir)";
    assert_eq!(list.entries, [entry("tr.ornek.altlik", "=1.0.0", reason)]);
    // The notice and the entry say it in the same words.
    let notice = release.notice_text().unwrap();
    assert!(
        notice.contains("Şüpheli ağ trafiği bildirildi") && notice.contains(UNDER_REVIEW),
        "{notice}"
    );
    release
        .reinstate(&admin(), "İnceleme temiz çıktı.", t(7))
        .unwrap();
    assert!(revocation_list(&[release]).entries.is_empty());
}

#[test]
fn a_withdrawn_release_is_listed_with_its_reason() {
    let straight = withdrawn("tr.ornek.a", "1.0.0", "Kötü amaçlı kod bulundu.");
    let mut after_suspension = suspended("tr.ornek.b", "2.0.0", "İnceleniyor.");
    after_suspension
        .withdraw(&admin(), "Kullanıcı verisini dışarı gönderiyor", t(8))
        .unwrap();
    let list = revocation_list(&[straight, after_suspension]);
    assert_eq!(
        list.entries,
        [
            entry("tr.ornek.a", "=1.0.0", "Kötü amaçlı kod bulundu"),
            entry(
                "tr.ornek.b",
                "=2.0.0",
                "Kullanıcı verisini dışarı gönderiyor"
            ),
        ]
    );
}

#[test]
fn an_archived_or_never_published_release_is_not_listed() {
    let mut retired = published("tr.ornek.a", "1.0.0");
    retired
        .archive(&author(), Some("2.0 yerine geçti."), t(7))
        .unwrap();
    let mut given_up = approved("tr.ornek.b", "1.0.0");
    given_up.archive(&author(), None, t(7)).unwrap();
    let mut sent = draft("tr.ornek.c", "1.0.0");
    sent.submit(&author(), None, t(1)).unwrap();
    let releases = [
        retired,
        given_up,
        sent,
        draft("tr.ornek.d", "1.0.0"),
        approved("tr.ornek.e", "1.0.0"),
        published("tr.ornek.f", "1.0.0"),
    ];
    assert!(releases.iter().all(|r| r.revoked().is_none()));
    assert!(revocation_list(&releases).entries.is_empty());
}

#[test]
fn the_list_is_ordered_by_id_then_version_whatever_order_the_releases_come_in() {
    let releases = vec![
        withdrawn("tr.ornek.b", "1.0.0", "b"),
        withdrawn("tr.ornek.a", "1.10.0", "a 1.10"),
        suspended("tr.ornek.a", "1.2.0", "a 1.2"),
        withdrawn("tr.ornek.a", "1.2.0-beta.1", "a beta"),
        withdrawn("tr.ornek.a", "1.3.0+20261115", "a 1.3"),
    ];
    let list = revocation_list(&releases);
    let order: Vec<(&str, &str)> = list
        .entries
        .iter()
        .map(|e| (e.id.as_str(), e.versions.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            ("tr.ornek.a", "=1.2.0-beta.1"),
            ("tr.ornek.a", "=1.2.0"),
            ("tr.ornek.a", "=1.3.0"),
            ("tr.ornek.a", "=1.10.0"),
            ("tr.ornek.b", "=1.0.0"),
        ]
    );
    let mut reversed = releases.clone();
    reversed.reverse();
    let bytes = |list: &Revocations| serde_json::to_string(list).unwrap();
    assert_eq!(bytes(&revocation_list(&reversed)), bytes(&list));
}

#[test]
fn the_list_writes_no_field_the_v1_reader_does_not_know() {
    let list = revocation_list(&[withdrawn("tr.ornek.a", "1.0.0", "Kötü amaçlı kod.")]);
    let value = serde_json::to_value(&list).unwrap();
    let keys = |v: &serde_json::Value| -> BTreeSet<String> {
        v.as_object().unwrap().keys().cloned().collect()
    };
    assert_eq!(
        keys(&value),
        BTreeSet::from(["entries".into(), "format".into(), "schemaVersion".into()])
    );
    assert_eq!(
        keys(&value["entries"][0]),
        BTreeSet::from(["id".into(), "reason".into(), "versions".into()])
    );
    assert_eq!(
        (value["format"].as_str(), value["schemaVersion"].as_u64()),
        (Some(REVOCATIONS_FORMAT), Some(1))
    );
    // What the reader takes back is what was written; one key more and every installed reader refuses the list.
    let text = value.to_string();
    assert_eq!(Revocations::read(&text).unwrap(), list);
    let mut stamped = value.clone();
    stamped["generatedAt"] = serde_json::json!("2026-11-15T10:00:00Z");
    assert_eq!(
        Revocations::read(&stamped.to_string()).unwrap_err().code,
        "revocations_invalid"
    );
    let mut signed = value;
    signed["entries"][0]["signature"] = serde_json::json!("…");
    assert_eq!(
        Revocations::read(&signed.to_string()).unwrap_err().code,
        "revocations_invalid"
    );
}

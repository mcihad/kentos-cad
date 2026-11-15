//! The revocation list the catalog writes is the one the host reads (plan/eklenti.md ADIM 22A; docs/adr/8059 §12): the
//! release flow's list goes through the host's own reader, and the host finds each version's reason in it.

use kentos_extension_host::Revocations;
use kentos_extension_host::policy::REVOCATIONS_FORMAT;
use kentos_extensions::integrity::{FileDigest, sha256_hex};
use kentos_extensions::manifest::Manifest;
use kentos_extensions::release::{
    Actor, CatalogScope, Channel, GateKind, Release, ReleaseFacts, Role, ScanOutcome, Timestamp,
    revocation_list,
};

const MANIFEST: &str =
    include_str!("../../../../fixtures/extensions/v1/ornek-altlik/manifest.json");
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

/// A version through the whole flow: uploaded, every gate clean, reviewed, approved and published.
fn published(id: &str, version: &str) -> Release {
    let at = |minute: u64| Timestamp(1_763_190_000_000 + minute * 60_000);
    let author = Actor::new("ayse", &[Role::Publisher], Channel::Window).member_of(PUBLISHER);
    let reviewer = Actor::new("selin", &[Role::Reviewer], Channel::Window);
    let scanner = Actor::new("denetim", &[Role::Scanner], Channel::Program);
    let facts = facts(id, version);
    let mut release =
        Release::upload(&author, CatalogScope::Public, PUBLISHER, &facts, &[], at(0)).unwrap();
    let content = release.content().package.clone();
    for gate in GateKind::ALL {
        release
            .record_scan(&scanner, gate, &content, ScanOutcome::Passed, at(1))
            .unwrap();
    }
    release.submit(&author, None, at(2)).unwrap();
    release
        .start_review(&reviewer, &facts, None, false, at(3))
        .unwrap();
    release.approve(&reviewer, None, at(4)).unwrap();
    release.publish(&author, at(5)).unwrap();
    release
}

#[test]
fn the_host_reads_the_list_the_catalog_writes() {
    let moderator = Actor::new("yonetici", &[Role::CatalogAdmin], Channel::Window);
    let at = Timestamp(1_763_190_600_000);
    let mut withdrawn = published("tr.ornek.altlik", "1.0.0");
    withdrawn
        .withdraw(&moderator, "Kullanıcı verisini dışarı gönderiyor.", at)
        .unwrap();
    let mut suspended = published("tr.ornek.mebis", "2.1.0");
    suspended
        .suspend(&moderator, "Şüpheli ağ trafiği bildirildi", at)
        .unwrap();
    let still_out = published("tr.ornek.altlik", "1.1.0");
    let text =
        serde_json::to_string_pretty(&revocation_list(&[withdrawn, suspended, still_out])).unwrap();

    let list = Revocations::read(&text).expect("the host reads the catalog's list");
    assert_eq!(
        (list.format.as_str(), list.schema_version),
        (REVOCATIONS_FORMAT, 1)
    );
    assert_eq!(
        list.reason("tr.ornek.altlik", "1.0.0"),
        Some("Kullanıcı verisini dışarı gönderiyor")
    );
    assert_eq!(
        list.reason("tr.ornek.mebis", "2.1.0"),
        Some(
            "Şüpheli ağ trafiği bildirildi (askıya alındı; inceleme bitince askıdan alınabilir ya da geri çekilir)"
        )
    );
    // A version not taken back, and the same version under another id, stay as they are.
    assert_eq!(list.reason("tr.ornek.altlik", "1.1.0"), None);
    assert_eq!(list.reason("tr.ornek.altlik", "2.1.0"), None);
    // As the host says it: "1.0.0 sürümü geri çekildi: {reason}." reads as one sentence.
    let said = format!(
        "1.0.0 sürümü geri çekildi: {}.",
        list.reason("tr.ornek.altlik", "1.0.0").unwrap_or_default()
    );
    assert_eq!(
        said,
        "1.0.0 sürümü geri çekildi: Kullanıcı verisini dışarı gönderiyor."
    );
}

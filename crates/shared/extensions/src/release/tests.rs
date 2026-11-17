//! The release flow: the way to publication and every refusal on it, gates that never pass unfinished, the records,
//! a stored release read back through the same rules, and the review's depth from the difference with the version
//! approved before.

use std::collections::BTreeMap;
use std::path::Path;

use super::*;
use crate::archive::{Limits, open, pack};
use crate::integrity::{FileDigest, sha256_hex};
use crate::manifest::{ExtensionDependency, Manifest, RequestedCapability, Scope};

const MANIFEST: &str =
    include_str!("../../../../../fixtures/extensions/v1/ornek-imar/manifest.json");
const PUBLISHER: &str = "ornek-belediye";

fn file(path: &str, text: &str) -> FileDigest {
    FileDigest {
        path: path.to_owned(),
        size: text.len() as u64,
        sha256: sha256_hex(text.as_bytes()),
    }
}

fn package(name: &str, version: &str) -> (String, LockedPackage) {
    let package = LockedPackage {
        version: version.to_owned(),
        source: "registry https://pypi.org/simple".to_owned(),
        files: sha256_hex(format!("{name}-{version}").as_bytes()),
        uploaded: None,
    };
    (name.to_owned(), package)
}

/// The İmar example as a version: its manifest, its files (the manifest's, the change notes', the project's and the
/// lock's bytes say the version) and its lock.
fn facts(version: &str) -> ReleaseFacts {
    let mut manifest: Manifest = serde_json::from_str(MANIFEST).expect("the example's manifest");
    manifest.version = version.to_owned();
    ReleaseFacts {
        manifest,
        files: vec![
            file("CHANGELOG.md", &format!("## {version}")),
            file("LICENSE", "CC-BY-4.0"),
            file("assets/imar.svg", "<svg/>"),
            file("assets/taks.svg", "<svg/>"),
            file("manifest.json", version),
            file("pyproject.toml", version),
            file("src/imar/logic.py", "def check_setbacks(ctx): ..."),
            file("tools/taks.tool.json", "{}"),
            file("uv.lock", version),
        ],
        python: Some(PythonFacts {
            lock: sha256_hex(format!("lock {version}").as_bytes()),
            lock_compared: sha256_hex(b"lock"),
            project_compared: sha256_hex(b"project"),
            packages: BTreeMap::from([package("numpy", "2.5.4"), package("shapely", "2.0.7")]),
        }),
        sbom: None,
    }
}

/// The facts with one file's bytes changed or added.
fn with_file(mut facts: ReleaseFacts, path: &str, text: &str) -> ReleaseFacts {
    facts.files.retain(|f| f.path != path);
    facts.files.push(file(path, text));
    facts
}

fn author() -> Actor {
    Actor::new("ayse", &[Role::Publisher], Channel::Window).member_of(PUBLISHER)
}

/// A colleague at the same publisher who also reviews.
fn colleague() -> Actor {
    Actor::new(
        "mehmet",
        &[Role::Publisher, Role::Reviewer],
        Channel::Window,
    )
    .member_of(PUBLISHER)
}

fn reviewer() -> Actor {
    Actor::new("selin", &[Role::Reviewer], Channel::Window)
}

fn admin() -> Actor {
    Actor::new("yonetici", &[Role::CatalogAdmin], Channel::Window)
}

fn scanner() -> Actor {
    Actor::new("denetim", &[Role::Scanner], Channel::Program)
}

/// The author's token in a program: Python, MCP, an agent.
fn agent() -> Actor {
    Actor {
        via: Channel::Program,
        ..author()
    }
}

fn outsider() -> Actor {
    Actor::new("can", &[Role::Publisher], Channel::Window).member_of("baska-belediye")
}

fn t(minute: u64) -> Timestamp {
    Timestamp(1_763_190_000_000 + minute * 60_000)
}

fn upload(scope: CatalogScope, version: &str) -> Release {
    Release::upload(&author(), scope, PUBLISHER, &facts(version), &[], t(0)).expect("a draft")
}

fn draft(scope: CatalogScope) -> Release {
    upload(scope, "1.1.0")
}

fn scan_all(release: &mut Release, outcome: &ScanOutcome, at: Timestamp) {
    let content = release.content().package.clone();
    for gate in GateKind::ALL {
        release
            .record_scan(&scanner(), gate, &content, outcome.clone(), at)
            .expect("the scan's report");
    }
}

/// Takes the release into review as a first version: its own package, no baseline.
fn review(release: &mut Release, actor: &Actor, at: Timestamp) -> Result<(), Refusal> {
    let current = facts(release.version());
    release.start_review(actor, &current, None, false, at)
}

/// Uploaded, every gate clean, submitted and taken into review by the reviewer.
fn in_review(scope: CatalogScope, version: &str) -> Release {
    let mut release = upload(scope, version);
    scan_all(&mut release, &ScanOutcome::Passed, t(1));
    release.submit(&author(), None, t(2)).unwrap();
    review(&mut release, &reviewer(), t(3)).unwrap();
    release
}

fn published(scope: CatalogScope, version: &str) -> Release {
    let mut release = in_review(scope, version);
    release.approve(&reviewer(), None, t(4)).unwrap();
    release.publish(&author(), t(5)).unwrap();
    release
}

fn gate(release: &Release, kind: GateKind) -> &Gate {
    release.gates().iter().find(|g| g.kind == kind).unwrap()
}

fn refused(result: Result<(), Refusal>) -> &'static str {
    result.expect_err("a refusal").code
}

fn kinds(focus: &ReviewFocus) -> Vec<(DifferenceKind, &str)> {
    focus
        .differences
        .iter()
        .map(|d| (d.kind, d.subject.as_str()))
        .collect()
}

fn capability(id: Capability, scope: Scope, reason: &str) -> RequestedCapability {
    RequestedCapability {
        id,
        scope,
        reason: reason.to_owned(),
        hosts: Vec::new(),
        sends_data_to_ai: None,
    }
}

fn fixture(name: &str) -> BTreeMap<String, Vec<u8>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/extensions/v1")
        .join(name);
    let mut files = BTreeMap::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/");
                files.insert(relative, std::fs::read(&path).unwrap());
            }
        }
    }
    files
}

#[test]
fn the_table_of_moves() {
    use Action::*;
    use ReleaseState::*;
    let table: Vec<(ReleaseState, Vec<Action>)> = ReleaseState::ALL
        .into_iter()
        .map(|s| (s, s.actions()))
        .collect();
    assert_eq!(
        table,
        [
            (Draft, vec![Replace, Submit, Scan, Archive]),
            (Submitted, vec![Scan, StartReview, Archive]),
            (
                InReview,
                vec![Scan, RequestChanges, Waive, Approve, Archive]
            ),
            (ChangesRequested, vec![Replace, Submit, Scan, Archive]),
            (Approved, vec![RequestChanges, Publish, Archive]),
            (Published, vec![Suspend, Withdraw, Archive]),
            (Suspended, vec![Reinstate, Withdraw]),
            (Withdrawn, vec![]),
            (Archived, vec![]),
        ]
    );
    assert!(Withdrawn.is_final() && Archived.is_final() && !Suspended.is_final());
    for state in ReleaseState::ALL {
        assert_eq!(serde_json::to_value(state).unwrap(), state.id());
    }
    // No role both approves and publishes (PY-CLOUD-09).
    assert!(Role::ALL.iter().all(|r| {
        let p = r.permissions();
        !(p.contains(&Permission::Approve) && p.contains(&Permission::Publish))
    }));
}

#[test]
fn a_version_goes_from_upload_to_publication_one_record_a_move() {
    let mut release = draft(CatalogScope::Public);
    assert_eq!(release.state(), ReleaseState::Draft);
    assert!(!release.listed());
    assert!(
        release
            .gates()
            .iter()
            .all(|g| g.status == GateStatus::Verifying && g.detail() == "taranmadı")
    );
    scan_all(&mut release, &ScanOutcome::Started, t(1));
    scan_all(&mut release, &ScanOutcome::Passed, t(2));
    assert!(release.gates_verified());
    release.submit(&author(), None, t(3)).unwrap();
    review(&mut release, &reviewer(), t(4)).unwrap();
    assert_eq!(release.focus().unwrap().depth, ReviewDepth::Deep);
    release
        .approve(
            &reviewer(),
            Some("Yetenekler gerekçeleriyle tutuyor."),
            t(5),
        )
        .unwrap();
    let approval = release.approval().unwrap();
    assert_eq!(
        (approval.by.as_str(), approval.depth, &approval.content),
        ("selin", ReviewDepth::Deep, &facts("1.1.0").content())
    );
    release.publish(&author(), t(6)).unwrap();
    assert_eq!(release.state(), ReleaseState::Published);
    assert!(release.listed());
    release.allows_install(&facts("1.1.0").content()).unwrap();
    release.allows_run().unwrap();
    use ReleaseState::*;
    let moves: Vec<(Action, Option<ReleaseState>, ReleaseState)> = release
        .audit()
        .iter()
        .filter(|r| r.action != Action::Scan)
        .map(|r| (r.action, r.from, r.to))
        .collect();
    assert_eq!(
        moves,
        [
            (Action::Upload, None, Draft),
            (Action::Submit, Some(Draft), Submitted),
            (Action::StartReview, Some(Submitted), InReview),
            (Action::Approve, Some(InReview), Approved),
            (Action::Publish, Some(Approved), Published),
        ]
    );
    // The upload, two reports for each gate, four moves.
    assert_eq!(release.audit().len(), 1 + 2 * GateKind::ALL.len() + 4);
    let last = release.audit().last().unwrap();
    assert_eq!(
        (last.by.as_str(), last.via, last.at, last.content.as_str()),
        (
            "ayse",
            Channel::Window,
            t(6),
            release.content().package.as_str()
        )
    );
}

#[test]
fn an_author_never_reviews_their_own_release_outside_their_own_catalog() {
    // The author who also holds the reviewer's role.
    let author_reviewer = Actor::new("ayse", &[Role::Publisher, Role::Reviewer], Channel::Window)
        .member_of(PUBLISHER);
    for scope in [CatalogScope::Public, CatalogScope::Tenant] {
        let mut release = draft(scope);
        scan_all(&mut release, &ScanOutcome::Passed, t(1));
        release.submit(&author(), None, t(2)).unwrap();
        assert_eq!(
            refused(review(&mut release, &author_reviewer, t(3))),
            "self_review"
        );
        review(&mut release, &reviewer(), t(3)).unwrap();
        assert_eq!(
            refused(release.approve(&author_reviewer, None, t(4))),
            "self_review"
        );
        assert_eq!(
            refused(release.request_changes(
                &author_reviewer,
                &[RequestedChange::new(ChangeCode::Broken)],
                t(4)
            )),
            "self_review"
        );
    }
    // In the public catalog no member of its publisher approves either; in an organisation's a colleague may.
    let mut public = in_review(CatalogScope::Public, "1.1.0");
    assert_eq!(
        refused(public.approve(&colleague(), None, t(4))),
        "self_review"
    );
    let mut tenant = in_review(CatalogScope::Tenant, "1.1.0");
    tenant.approve(&colleague(), None, t(4)).unwrap();
    // In one's personal catalog nobody else installs it: its owner reviews it, the gates still have to pass.
    let mut private = Release::upload(
        &author_reviewer,
        CatalogScope::Private,
        PUBLISHER,
        &facts("1.1.0"),
        &[],
        t(0),
    )
    .unwrap();
    private.submit(&author_reviewer, None, t(1)).unwrap();
    review(&mut private, &author_reviewer, t(2)).unwrap();
    assert_eq!(
        refused(private.approve(&author_reviewer, None, t(3))),
        "gates_open"
    );
    scan_all(&mut private, &ScanOutcome::Passed, t(3));
    private.approve(&author_reviewer, None, t(4)).unwrap();
}

#[test]
fn approving_and_publishing_are_separate_permissions() {
    let mut release = in_review(CatalogScope::Public, "1.1.0");
    assert_eq!(
        refused(release.approve(&author(), None, t(4))),
        "permission_missing"
    );
    release.approve(&reviewer(), None, t(4)).unwrap();
    let r = release.publish(&reviewer(), t(5)).unwrap_err();
    assert_eq!(r.code, "permission_missing");
    assert!(r.message.contains("Yayımcı rolündedir"), "{r}");
    // A publisher publishes its own versions only, and uploads under its own name only.
    assert_eq!(refused(release.publish(&outsider(), t(5))), "not_owner");
    let upload = Release::upload(
        &outsider(),
        CatalogScope::Public,
        PUBLISHER,
        &facts("1.1.0"),
        &[],
        t(0),
    );
    assert_eq!(upload.unwrap_err().code, "not_owner");
    // The manifest's publisherId is the publisher it is uploaded under.
    let mut claimed = facts("1.1.0");
    claimed.manifest.publisher_id = Some("baska-belediye".into());
    let upload = Release::upload(
        &author(),
        CatalogScope::Public,
        PUBLISHER,
        &claimed,
        &[],
        t(0),
    );
    assert_eq!(upload.unwrap_err().code, "publisher_mismatch");
}

#[test]
fn a_program_moves_a_release_only_as_far_as_review_allows() {
    // An agent with the author's token uploads and submits, and tries to publish at every step before approval.
    let mut release = Release::upload(
        &agent(),
        CatalogScope::Public,
        PUBLISHER,
        &facts("1.1.0"),
        &[],
        t(0),
    )
    .unwrap();
    let not_approved = release.clone().publish(&agent(), t(1)).unwrap_err();
    assert_eq!(not_approved.code, "not_approved");
    assert!(not_approved.message.contains("“Taslak”"), "{not_approved}");
    scan_all(&mut release, &ScanOutcome::Passed, t(1));
    release.submit(&agent(), None, t(2)).unwrap();
    assert_eq!(refused(release.publish(&agent(), t(2))), "not_approved");
    review(&mut release, &reviewer(), t(3)).unwrap();
    assert_eq!(refused(release.publish(&agent(), t(3))), "not_approved");
    // A reviewer's token in a program decides nothing.
    let bot = Actor {
        via: Channel::Program,
        ..reviewer()
    };
    assert_eq!(
        refused(release.approve(&bot, None, t(4))),
        "decision_by_program"
    );
    let change = [RequestedChange::new(ChangeCode::Broken)];
    assert_eq!(
        refused(release.request_changes(&bot, &change, t(4))),
        "decision_by_program"
    );
    // A person approves; then the program may publish, as its publisher's explicit act.
    release.approve(&reviewer(), None, t(5)).unwrap();
    release.publish(&agent(), t(6)).unwrap();
    assert_eq!(release.audit().last().unwrap().via, Channel::Program);
}

#[test]
fn content_is_fixed_from_submission_and_other_content_is_another_review() {
    let mut release = in_review(CatalogScope::Public, "1.1.0");
    let patched = with_file(
        facts("1.1.0"),
        "src/imar/logic.py",
        "def check_setbacks(ctx): steal()",
    );
    assert_eq!(
        refused(release.replace(&author(), &patched, t(4))),
        "content_frozen"
    );
    release.approve(&reviewer(), None, t(4)).unwrap();
    assert_eq!(
        refused(release.replace(&author(), &patched, t(5))),
        "content_frozen"
    );
    // The approval is taken back before publication; new content starts every gate again and needs its own approval.
    let broken = RequestedChange {
        file: Some("src/imar/logic.py".into()),
        ..RequestedChange::new(ChangeCode::Broken)
    };
    release
        .request_changes(&reviewer(), &[broken], t(5))
        .unwrap();
    assert_eq!(release.approval(), None);
    release.replace(&author(), &patched, t(6)).unwrap();
    assert_eq!(release.content(), &patched.content());
    assert!(release.gates().iter().all(|g| g.detail() == "taranmadı"));
    assert_eq!(refused(release.publish(&author(), t(7))), "not_approved");
    assert_eq!(
        refused(release.replace(&author(), &patched, t(7))),
        "content_unchanged"
    );
    // Another version's package is another release.
    assert_eq!(
        refused(release.replace(&author(), &facts("1.2.0"), t(7))),
        "identity_mismatch"
    );
    // A published release installs only the approved bytes.
    let published = published(CatalogScope::Public, "1.1.0");
    assert_eq!(
        refused(published.allows_install(&patched.content())),
        "content_mismatch"
    );
    // A version number, once uploaded, never carries other content (P14); build metadata does not make it another.
    let mut again = patched.clone();
    again.manifest.version = "1.1.0+yeniden".into();
    let taken = Release::upload(
        &author(),
        CatalogScope::Public,
        PUBLISHER,
        &again,
        std::slice::from_ref(&published),
        t(8),
    );
    assert_eq!(taken.unwrap_err().code, "version_taken");
}

#[test]
fn a_gate_passes_only_when_its_scan_finished_clean_on_this_content() {
    let mut release = draft(CatalogScope::Public);
    let content = release.content().package.clone();
    release.submit(&author(), None, t(1)).unwrap();
    review(&mut release, &reviewer(), t(2)).unwrap();
    let open = release.approve(&reviewer(), None, t(3)).unwrap_err();
    assert_eq!(open.code, "gates_open");
    assert!(open.message.contains("Sır taraması (taranmadı)"), "{open}");
    // Running, then errored: still verifying, never a pass.
    let secrets = GateKind::Secrets;
    release
        .record_scan(&scanner(), secrets, &content, ScanOutcome::Started, t(3))
        .unwrap();
    assert_eq!(gate(&release, secrets).detail(), "taranıyor");
    let full = ScanOutcome::Errored("kuyruk dolu".into());
    release
        .record_scan(&scanner(), secrets, &content, full, t(4))
        .unwrap();
    let g = gate(&release, secrets);
    assert_eq!(
        (g.status, g.detail(), g.error.as_deref()),
        (
            GateStatus::Verifying,
            "tarama hata verdi; yeniden taranacak",
            Some("kuyruk dolu")
        )
    );
    let open = release.approve(&reviewer(), None, t(4)).unwrap_err();
    assert!(
        open.message
            .contains("Sır taraması (tarama hata verdi; yeniden taranacak)"),
        "{open}"
    );
    // An errored gate is scanned again, never lifted.
    assert_eq!(
        refused(release.waive(&reviewer(), secrets, "Yanlış alarm.", t(5))),
        "waiver_not_blocked"
    );
    // A block that says no finding, a late report about other content, the author's own report: refused.
    let silent = ScanOutcome::Blocked(vec![]);
    assert_eq!(
        refused(release.record_scan(&scanner(), secrets, &content, silent, t(5))),
        "scan_unexplained"
    );
    let late = ScanOutcome::Passed;
    assert_eq!(
        refused(release.record_scan(&scanner(), secrets, "sha256:eski", late, t(5))),
        "scan_stale"
    );
    let own = Actor::new("ayse", &[Role::Scanner], Channel::Program);
    assert_eq!(
        refused(release.record_scan(&own, secrets, &content, ScanOutcome::Passed, t(5))),
        "self_scan"
    );
    scan_all(&mut release, &ScanOutcome::Passed, t(6));
    release.approve(&reviewer(), None, t(7)).unwrap();
    // The approval fixed the gates.
    assert_eq!(
        refused(release.record_scan(&scanner(), secrets, &content, ScanOutcome::Passed, t(8))),
        "gates_closed"
    );
    // The scanner's outcomes as a server sends them.
    let sent: ScanOutcome = serde_json::from_str(r#"{"errored": "zaman aşımı"}"#).unwrap();
    assert_eq!(sent, ScanOutcome::Errored("zaman aşımı".into()));
}

#[test]
fn a_blocking_gate_sends_the_release_back_with_its_findings() {
    let mut release = draft(CatalogScope::Public);
    let content = release.content().package.clone();
    release.submit(&author(), None, t(1)).unwrap();
    let finding = Reason {
        file: Some(".kentos/mcp.json".into()),
        ..Reason::new(
            "secret_found",
            "Dosyada bir erişim jetonu var; dosyayı paketten çıkarıp jetonu yenileyin.",
        )
    };
    let blocked = ScanOutcome::Blocked(vec![finding.clone()]);
    release
        .record_scan(
            &scanner(),
            GateKind::Secrets,
            &content,
            blocked.clone(),
            t(2),
        )
        .unwrap();
    assert_eq!(release.state(), ReleaseState::ChangesRequested);
    let asked = release.change_request().unwrap();
    assert_eq!(
        (asked.gate, asked.reasons.as_slice(), asked.by.as_str()),
        (
            Some(GateKind::Secrets),
            std::slice::from_ref(&finding),
            "denetim"
        )
    );
    assert!(
        asked
            .resubmission()
            .starts_with("Sır taraması bu içeriği engelledi.")
    );
    // The same content goes again only with a word why.
    assert_eq!(
        refused(release.submit(&author(), None, t(3))),
        "gate_blocked"
    );
    // The mended package starts every gate again and goes as it is.
    let mut mended = release.clone();
    let clean = with_file(
        facts("1.1.0"),
        "src/imar/logic.py",
        "def check_setbacks(ctx): pass",
    );
    mended.replace(&author(), &clean, t(3)).unwrap();
    assert!(mended.gates().iter().all(|g| g.detail() == "taranmadı"));
    mended.submit(&author(), None, t(4)).unwrap();
    // A false positive: the author's word, then a reviewer lifts the finding with a reason.
    release
        .submit(
            &author(),
            Some("Jeton örnek değer; hiçbir hizmette geçerli değil."),
            t(3),
        )
        .unwrap();
    review(&mut release, &reviewer(), t(4)).unwrap();
    assert_eq!(
        refused(release.waive(&reviewer(), GateKind::Package, "Kural fazla katı.", t(5))),
        "waiver_not_allowed"
    );
    assert_eq!(
        refused(release.waive(&reviewer(), GateKind::Secrets, "  ", t(5))),
        "reason_missing"
    );
    let author_reviewer = Actor::new("ayse", &[Role::Publisher, Role::Reviewer], Channel::Window)
        .member_of(PUBLISHER);
    assert_eq!(
        refused(release.waive(&author_reviewer, GateKind::Secrets, "Örnek.", t(5))),
        "self_review"
    );
    release
        .waive(
            &reviewer(),
            GateKind::Secrets,
            "Örnek jeton; hiçbir hizmette geçerli değil.",
            t(5),
        )
        .unwrap();
    assert_eq!(
        gate(&release, GateKind::Secrets).detail(),
        "doğrulandı (engel gerekçeyle kaldırıldı)"
    );
    // The same finding found again keeps the lift; another finding does not, and sends it back.
    release
        .record_scan(&scanner(), GateKind::Secrets, &content, blocked, t(6))
        .unwrap();
    assert_eq!(release.state(), ReleaseState::InReview);
    let mut other = release.clone();
    let another = ScanOutcome::Blocked(vec![Reason::new("secret_found", "Başka bir jeton.")]);
    other
        .record_scan(&scanner(), GateKind::Secrets, &content, another, t(7))
        .unwrap();
    assert_eq!(
        (
            other.state(),
            gate(&other, GateKind::Secrets).waiver.is_none()
        ),
        (ReleaseState::ChangesRequested, true)
    );
    for kind in GateKind::ALL
        .into_iter()
        .filter(|k| *k != GateKind::Secrets)
    {
        release
            .record_scan(&scanner(), kind, &content, ScanOutcome::Passed, t(7))
            .unwrap();
    }
    release.approve(&reviewer(), None, t(8)).unwrap();
    assert!(
        release
            .audit()
            .iter()
            .any(|r| r.action == Action::Waive && r.gate == Some(GateKind::Secrets))
    );
}

#[test]
fn a_reviewer_sends_back_with_ready_codes_and_the_author_has_a_way_back() {
    let mut release = in_review(CatalogScope::Public, "1.1.0");
    assert_eq!(
        refused(release.request_changes(&reviewer(), &[], t(4))),
        "changes_missing"
    );
    let other = [RequestedChange::new(ChangeCode::Other)];
    assert_eq!(
        refused(release.request_changes(&reviewer(), &other, t(4))),
        "reason_missing"
    );
    let changes = [
        RequestedChange {
            file: Some("manifest.json".into()),
            note: Some("network hiç kullanılmıyor.".into()),
            ..RequestedChange::new(ChangeCode::CapabilityUnused)
        },
        RequestedChange {
            dependency: Some("requests".into()),
            ..RequestedChange::new(ChangeCode::DependencyUnneeded)
        },
    ];
    release
        .request_changes(&reviewer(), &changes, t(4))
        .unwrap();
    assert_eq!(release.state(), ReleaseState::ChangesRequested);
    let asked = release.change_request().unwrap();
    let first = &asked.reasons[0];
    assert_eq!(
        (first.code.as_str(), first.file.as_deref()),
        ("capability_unused", Some("manifest.json"))
    );
    assert!(
        first
            .message
            .starts_with("Kullanılmayan yetenek: Kodun kullanmadığı yeteneği")
            && first
                .message
                .ends_with("İnceleyicinin notu: network hiç kullanılmıyor."),
        "{}",
        first.message
    );
    let second = &asked.reasons[1];
    assert_eq!(
        (second.code.as_str(), second.dependency.as_deref()),
        ("dependency_unneeded", Some("requests"))
    );
    assert!(
        asked
            .resubmission()
            .contains("aynı sürüm numarasıyla yeniden yükleyin")
    );
    assert_eq!(
        release.audit().last().unwrap().reason.as_deref(),
        Some("capability_unused, dependency_unneeded")
    );
    // The same content goes back only with a word why.
    assert_eq!(
        refused(release.submit(&author(), None, t(5))),
        "unchanged_resubmit"
    );
    let mut replied = release.clone();
    let why = "requests panelin bağlantı testinde kullanılıyor.";
    replied.submit(&author(), Some(why), t(5)).unwrap();
    assert_eq!(replied.audit().last().unwrap().reason.as_deref(), Some(why));
    // The mended package goes again; the request stays on record for the next review.
    let mended = with_file(facts("1.1.0"), "src/imar/logic.py", "düzeltildi");
    release.replace(&author(), &mended, t(6)).unwrap();
    release.submit(&author(), None, t(7)).unwrap();
    assert_eq!(release.state(), ReleaseState::Submitted);
    assert!(release.change_request().is_some());
    // The ready codes are the stable codes a record and an agent read.
    for code in ChangeCode::ALL {
        assert_eq!(serde_json::to_value(code).unwrap(), code.id());
    }
}

#[test]
fn suspension_and_withdrawal_stop_installs_and_runs_and_say_why_honestly() {
    let mut release = published(CatalogScope::Public, "1.1.0");
    let content = release.content().clone();
    assert_eq!(
        refused(release.suspend(&author(), "Şüpheli trafik.", t(6))),
        "permission_missing"
    );
    assert_eq!(
        refused(release.suspend(&admin(), " ", t(6))),
        "reason_missing"
    );
    release
        .suspend(&admin(), "Şüpheli ağ trafiği bildirildi.", t(6))
        .unwrap();
    assert!(!release.listed());
    for refusal in [
        release.allows_install(&content).unwrap_err(),
        release.allows_run().unwrap_err(),
    ] {
        assert!(
            refusal
                .message
                .contains("askıya alındı: Şüpheli ağ trafiği bildirildi.")
                && refusal.message.contains("silinemez ve geri alınamaz"),
            "{refusal}"
        );
    }
    // Putting it back is a person's decision, and never the author's own.
    let admin_bot = Actor {
        via: Channel::Program,
        ..admin()
    };
    assert_eq!(
        refused(release.reinstate(&admin_bot, "Temiz.", t(7))),
        "decision_by_program"
    );
    let author_admin = Actor::new(
        "ayse",
        &[Role::Publisher, Role::CatalogAdmin],
        Channel::Window,
    )
    .member_of(PUBLISHER);
    assert_eq!(
        refused(release.reinstate(&author_admin, "Temiz.", t(7))),
        "self_review"
    );
    release
        .reinstate(&admin(), "Trafik güncelleme denetimiymiş.", t(7))
        .unwrap();
    release.allows_install(&content).unwrap();
    assert_eq!(release.notice(), None);
    // Its publisher withdraws it for good; another publisher cannot.
    assert_eq!(
        refused(release.withdraw(&outsider(), "Bozuk.", t(8))),
        "not_owner"
    );
    release
        .withdraw(
            &author(),
            "Veri kaybına yol açan hata; 1.1.1'e geçin.",
            t(8),
        )
        .unwrap();
    let text = release.notice_text().unwrap();
    assert!(
        text.contains("geri çekildi: Veri kaybına") && text.contains("silinemez ve geri alınamaz"),
        "{text}"
    );
    assert_eq!(refused(release.allows_run()), "not_runnable");
    assert!(release.state().is_final());
    for attempt in [
        release.clone().reinstate(&admin(), "Temiz.", t(9)),
        release.clone().publish(&author(), t(9)),
        release.clone().archive(&admin(), None, t(9)),
    ] {
        assert_eq!(refused(attempt), "wrong_state");
    }
    // A retired version keeps running where it is installed, and installs no more.
    let mut retired = published(CatalogScope::Public, "1.1.0");
    retired
        .archive(&author(), Some("2.0 yerine geçti."), t(6))
        .unwrap();
    retired.allows_run().unwrap();
    assert_eq!(refused(retired.allows_install(&content)), "not_installable");
    assert!(
        retired
            .notice_text()
            .unwrap()
            .contains("kurulu kopyalar çalışmaya devam eder")
    );
    // A draft given up was never out: it does not run.
    let mut given_up = draft(CatalogScope::Public);
    given_up.archive(&author(), None, t(1)).unwrap();
    assert_eq!(refused(given_up.allows_run()), "not_runnable");
    assert!(
        given_up
            .notice_text()
            .unwrap()
            .ends_with("yayımlanmadan arşivlendi.")
    );
}

#[test]
fn a_refused_move_changes_nothing_and_every_move_is_on_record() {
    let mut release = in_review(CatalogScope::Public, "1.1.0");
    let before = release.clone();
    assert!(release.publish(&agent(), t(4)).is_err());
    assert!(release.approve(&author(), None, t(4)).is_err());
    assert!(release.suspend(&admin(), "Şüpheli.", t(4)).is_err());
    assert!(release.withdraw(&author(), "Bozuk.", t(4)).is_err());
    assert!(release.replace(&author(), &facts("1.1.0"), t(4)).is_err());
    let stale = ScanOutcome::Passed;
    assert!(
        release
            .record_scan(&scanner(), GateKind::Tests, "sha256:eski", stale, t(4))
            .is_err()
    );
    assert_eq!(release, before);
    release.approve(&reviewer(), None, t(5)).unwrap();
    release.publish(&agent(), t(6)).unwrap();
    let audit = release.audit();
    assert!(
        audit
            .windows(2)
            .all(|w| w[1].from == Some(w[0].to) && w[0].at <= w[1].at)
    );
    assert!(
        audit
            .iter()
            .all(|r| !r.by.is_empty() && r.content == release.content().package)
    );
    assert!(
        audit
            .iter()
            .filter(|r| r.action == Action::Scan)
            .all(|r| r.gate.is_some() && r.reason.as_deref() == Some("temiz"))
    );
    assert_eq!(
        serde_json::to_value(audit.last().unwrap()).unwrap(),
        serde_json::json!({
            "at": t(6).0,
            "by": "ayse",
            "via": "program",
            "action": "publish",
            "from": "approved",
            "to": "published",
            "content": release.content().package,
        })
    );
}

#[test]
fn a_stored_release_is_read_through_the_same_rules() {
    let release = published(CatalogScope::Public, "1.1.0");
    let json = serde_json::to_string(&release).unwrap();
    assert_eq!(Release::read(&json).unwrap(), release);
    type Tamper = fn(&mut serde_json::Value);
    let tampered = |edit: Tamper| -> Refusal {
        let mut value = serde_json::to_value(&release).unwrap();
        edit(&mut value);
        Release::read(&value.to_string()).unwrap_err()
    };
    // An approval of other content, by its author, a gate not verified after approval, a move missing.
    let cases: [(Tamper, &str); 4] = [
        (
            |v| v["approval"]["content"]["package"] = "sha256:baska".into(),
            "onay başka bir içeriğin",
        ),
        (
            |v| v["approval"]["by"] = "ayse".into(),
            "yazar kendi sürümünü onaylamış",
        ),
        (
            |v| v["gates"][4]["status"] = "verifying".into(),
            "onaylı sürümde doğrulanmamış denetim var",
        ),
        (
            |v| {
                v["audit"].as_array_mut().unwrap().pop();
            },
            "son hareket sürümün durumunu söylemiyor",
        ),
    ];
    for (edit, why) in cases {
        let refusal = tampered(edit);
        assert_eq!(refusal.code, "record_inconsistent", "{refusal}");
        assert!(refusal.message.ends_with(&format!("{why}.")), "{refusal}");
    }
    // A draft written as published, without its moves.
    let mut forged = serde_json::to_value(draft(CatalogScope::Public)).unwrap();
    forged["state"] = "published".into();
    assert_eq!(
        Release::read(&forged.to_string()).unwrap_err().code,
        "record_inconsistent"
    );
    assert!(serde_json::from_value::<Release>(forged).is_err());
    assert_eq!(tampered(|v| v["kind"] = true.into()).code, "record_invalid");
}

#[test]
fn who_sees_a_release() {
    let draft = draft(CatalogScope::Public);
    for actor in [author(), colleague(), reviewer(), admin(), scanner()] {
        assert!(draft.visible_to(&actor), "{}", actor.id);
    }
    let stranger = Actor::new("vatandas", &[], Channel::Window);
    assert!(!draft.visible_to(&stranger) && !draft.visible_to(&outsider()));
    assert!(published(CatalogScope::Public, "1.1.0").visible_to(&stranger));
    let private = upload(CatalogScope::Private, "1.1.0");
    assert!(private.visible_to(&author()));
    assert!(!private.visible_to(&reviewer()) && !private.visible_to(&admin()));
    // The catalog's scanner, or a personal catalog's release would never pass its gates.
    assert!(private.visible_to(&scanner()));
}

#[test]
fn the_baseline_is_the_last_approved_version_below_that_was_not_taken_back() {
    let next = upload(CatalogScope::Public, "1.2.0");
    let v100 = published(CatalogScope::Public, "1.0.0");
    let mut v103 = published(CatalogScope::Public, "1.0.3");
    v103.suspend(&admin(), "İnceleniyor.", t(9)).unwrap();
    let mut v105 = published(CatalogScope::Public, "1.0.5");
    v105.withdraw(&admin(), "Kötü amaçlı kod.", t(9)).unwrap();
    let all = [
        v100.clone(),
        v103,
        v105,
        published(CatalogScope::Public, "2.0.0"),
        upload(CatalogScope::Public, "1.1.0"),
    ];
    assert_eq!(next.baseline(&all).map(Release::version), Some("1.0.0"));
    // A retired version that was approved still is one.
    let mut v110 = published(CatalogScope::Public, "1.1.0");
    v110.archive(&author(), None, t(9)).unwrap();
    assert_eq!(
        next.baseline(&[v100, v110]).map(Release::version),
        Some("1.1.0")
    );
    assert_eq!(upload(CatalogScope::Public, "0.9.0").baseline(&all), None);
}

#[test]
fn a_review_starts_with_the_focus_the_flow_computes_from_the_packages() {
    let v100 = published(CatalogScope::Public, "1.0.0");
    let mut next = upload(CatalogScope::Public, "1.1.0");
    next.submit(&author(), None, t(1)).unwrap();
    let current = facts("1.1.0");
    // Not its own package; a baseline not approved, not before it, or not with its approved package: refused.
    assert_eq!(
        refused(next.start_review(&reviewer(), &facts("1.2.0"), None, false, t(2))),
        "package_mismatch"
    );
    let unapproved = upload(CatalogScope::Public, "1.0.5");
    let later = published(CatalogScope::Public, "1.2.0");
    for (baseline, its) in [
        (&unapproved, facts("1.0.5")),
        (&later, facts("1.2.0")),
        (&v100, facts("1.0.5")),
    ] {
        let refusal = next
            .start_review(&reviewer(), &current, Some((baseline, &its)), false, t(2))
            .unwrap_err();
        assert_eq!(refusal.code, "baseline_invalid", "{refusal}");
    }
    // Against 1.0.0 only the change notes differ: the fast path, kept in the review and its approval.
    next.start_review(
        &reviewer(),
        &current,
        Some((&v100, &facts("1.0.0"))),
        false,
        t(2),
    )
    .unwrap();
    let focus = next.focus().unwrap().clone();
    assert_eq!(
        (focus.depth, focus.previous.as_deref()),
        (ReviewDepth::Fast, Some("1.0.0"))
    );
    assert_eq!(next.audit().last().unwrap().reason, Some(focus.summary));
    scan_all(&mut next, &ScanOutcome::Passed, t(3));
    next.approve(&reviewer(), None, t(4)).unwrap();
    assert_eq!(next.approval().unwrap().depth, ReviewDepth::Fast);
}

#[test]
fn the_first_version_is_read_whole() {
    use DifferenceKind::*;
    let focus = classify(None, &facts("1.0.0"), true);
    assert_eq!(
        (
            focus.depth,
            focus.previous.as_deref(),
            focus.trusted_fast_path
        ),
        (ReviewDepth::Deep, None, false)
    );
    assert_eq!(
        kinds(&focus),
        [
            (NewCapability, "document.read"),
            (NewCapability, "expression.provider"),
            (NewCapability, "processing.provider"),
            (NewCapability, "ui.ribbonTab"),
            (NewDependency, "numpy"),
            (NewDependency, "shapely"),
        ]
    );
    assert_eq!(
        focus.differences[0].detail,
        "Açık projeyi okur · current-project"
    );
    assert_eq!(focus.differences[4].detail, "Python, 2.5.4");
    assert_eq!(
        focus.summary,
        "Derin inceleme — ilk onaylı sürüm, paketin tamamı okunur"
    );
    assert_eq!(focus.content, facts("1.0.0").content().package);
}

#[test]
fn text_and_icons_alone_take_the_fast_path() {
    use DifferenceKind::*;
    let before = facts("1.0.0");
    // The version and its change notes only: the version in the manifest, the project and the lock is left out.
    let focus = classify(Some(&before), &facts("1.1.0"), false);
    assert_eq!(
        (focus.depth, kinds(&focus)),
        (ReviewDepth::Fast, vec![(Text, "CHANGELOG.md")])
    );
    assert_eq!(
        focus.summary,
        "Hızlı yol — Metin ve simge: CHANGELOG.md (önceki onaylı sürüm 1.0.0)"
    );
    // An icon, a new document, the listing's words, a capability's reason: still text.
    let mut next = with_file(facts("1.1.0"), "assets/taks.svg", "<svg>yeni</svg>");
    next = with_file(next, "docs/KULLANIM.md", "# Kullanım");
    next.manifest.summary = "Yapıların yaklaşma mesafesini ve TAKS'ı denetler.".into();
    next.manifest.requested_capabilities[0].reason =
        "Yapı ve parsel katmanlarını okur; başka katmana dokunmaz.".into();
    let focus = classify(Some(&before), &next, false);
    assert_eq!(
        (focus.depth, kinds(&focus)),
        (
            ReviewDepth::Fast,
            vec![
                (Text, "CHANGELOG.md"),
                (Text, "assets/taks.svg"),
                (Text, "docs/KULLANIM.md"),
                (Text, "document.read"),
                (Text, "manifest.json"),
            ]
        )
    );
    assert_eq!(focus.differences[4].detail, "değişen alanlar: summary");
    // Nothing but the version number.
    let mut same = facts("1.0.0");
    same.manifest.version = "1.0.1".into();
    let focus = classify(Some(&before), &same, false);
    assert_eq!(
        (focus.depth, focus.differences.len()),
        (ReviewDepth::Fast, 0)
    );
    assert_eq!(
        focus.summary,
        "Hızlı yol — önceki onaylı sürümden (1.0.0) yalnız sürüm numarası farklı"
    );
}

#[test]
fn code_is_read_as_a_difference_and_a_trusted_publishers_code_takes_the_fast_path() {
    use DifferenceKind::*;
    let before = facts("1.0.0");
    let next = with_file(
        facts("1.1.0"),
        "src/imar/logic.py",
        "def check_setbacks(ctx): return 2",
    );
    let focus = classify(Some(&before), &next, false);
    assert_eq!(focus.depth, ReviewDepth::Standard);
    assert_eq!(
        focus.differences[0],
        Difference {
            kind: Code,
            subject: "src/imar/logic.py".into(),
            detail: "değişti".into(),
        }
    );
    let trusted = classify(Some(&before), &next, true);
    assert_eq!(
        (trusted.depth, trusted.trusted_fast_path),
        (ReviewDepth::Fast, true)
    );
    assert_eq!(
        trusted.summary,
        "Hızlı yol (güvenilir yayımcı, yetenek ve bağımlılık farkı yok) — Kod: src/imar/logic.py · Metin ve simge: \
         CHANGELOG.md (önceki onaylı sürüm 1.0.0)"
    );
    // The manifest's behaviour is code: a command's title (what the ribbon says it does), a tool's descriptor.
    let mut wired = facts("1.1.0");
    wired.manifest.entrypoints.commands[0].title = "Mesafeleri sil".into();
    let focus = classify(Some(&before), &wired, true);
    assert_eq!(
        (focus.depth, &focus.differences[0]),
        (
            ReviewDepth::Fast,
            &Difference {
                kind: Code,
                subject: "manifest.json".into(),
                detail: "değişen alanlar: entrypoints".into(),
            }
        )
    );
    assert_eq!(
        classify(Some(&before), &wired, false).depth,
        ReviewDepth::Standard
    );
    let tool = with_file(facts("1.1.0"), "tools/taks.tool.json", "{\"x\": 1}");
    assert_eq!(
        classify(Some(&before), &tool, false).depth,
        ReviewDepth::Standard
    );
}

#[test]
fn a_new_or_wider_capability_or_a_new_dependency_is_read_whole_even_from_a_trusted_publisher() {
    use DifferenceKind::*;
    use ReviewDepth::*;
    let before = facts("1.0.0");
    let mut network = facts("1.1.0");
    network
        .manifest
        .requested_capabilities
        .push(RequestedCapability {
            hosts: vec!["api.ornek.gov.tr".into()],
            ..capability(
                Capability::Network,
                Scope::PluginOwned,
                "Plan notlarını belediyenin servisinden alır.",
            )
        });
    let focus = classify(Some(&before), &network, true);
    assert_eq!((focus.depth, focus.trusted_fast_path), (Deep, false));
    assert_eq!(
        focus.differences[0],
        Difference {
            kind: NewCapability,
            subject: "network".into(),
            detail: "İnternete bağlanır · plugin-owned · api.ornek.gov.tr".into(),
        }
    );
    assert!(
        focus
            .summary
            .starts_with("Derin inceleme — Yeni yetenek: network"),
        "{}",
        focus.summary
    );
    // Wider: another host and data to an outside AI service; a wider scope.
    let mut wider = network.clone();
    if let Some(net) = wider.manifest.requested_capabilities.last_mut() {
        net.hosts.push("yz.example.com".into());
        net.sends_data_to_ai = Some(true);
    }
    let focus = classify(Some(&network), &wider, true);
    assert_eq!(
        (focus.depth, focus.differences[0].detail.as_str()),
        (
            Deep,
            "yeni adres: yz.example.com; dış yapay zekâya veri göndermeye başlıyor"
        )
    );
    let mut narrow = facts("1.0.0");
    narrow.manifest.requested_capabilities[0].scope = Scope::UserSelected;
    let focus = classify(Some(&narrow), &facts("1.1.0"), true);
    assert_eq!(
        (
            focus.depth,
            kinds(&focus)[0],
            focus.differences[0].detail.as_str()
        ),
        (
            Deep,
            (WiderCapability, "document.read"),
            "kapsam user-selected → current-project"
        )
    );
    // Narrower is a difference too: not deep, but it closes the trusted fast path.
    let narrowed = with_file(
        narrow,
        "src/imar/logic.py",
        "def check_setbacks(ctx): return 3",
    );
    let focus = classify(Some(&before), &narrowed, true);
    assert_eq!(
        (focus.depth, focus.trusted_fast_path, kinds(&focus)[0]),
        (Standard, false, (NarrowerCapability, "document.read"))
    );
    // A new Python package, a new extension, a package now from another index.
    let mut requests = facts("1.1.0");
    if let Some(python) = requests.python.as_mut() {
        python.packages.extend([package("requests", "2.32.3")]);
    }
    let focus = classify(Some(&before), &requests, true);
    assert_eq!(
        (focus.depth, kinds(&focus)[0]),
        (Deep, (NewDependency, "requests"))
    );
    let mut extension = facts("1.1.0");
    extension
        .manifest
        .dependencies
        .extensions
        .push(ExtensionDependency {
            id: "tr.ornek.altlik".into(),
            version: ">=1.0, <2".into(),
        });
    let focus = classify(Some(&before), &extension, true);
    assert_eq!(
        (focus.depth, &focus.differences[0]),
        (
            Deep,
            &Difference {
                kind: NewDependency,
                subject: "tr.ornek.altlik".into(),
                detail: "eklenti, >=1.0, <2".into(),
            }
        )
    );
    let mut mirror = facts("1.1.0");
    if let Some(numpy) = mirror
        .python
        .as_mut()
        .and_then(|p| p.packages.get_mut("numpy"))
    {
        numpy.source = "registry https://paketler.example.com/simple".into();
    }
    let focus = classify(Some(&before), &mirror, true);
    assert_eq!(
        (focus.depth, kinds(&focus)[0]),
        (Deep, (DependencySource, "numpy"))
    );
}

#[test]
fn a_dependency_or_lock_change_closes_the_trusted_fast_path() {
    use DifferenceKind::*;
    let before = facts("1.0.0");
    let mut bumped = with_file(
        facts("1.1.0"),
        "src/imar/logic.py",
        "def check_setbacks(ctx): return 4",
    );
    if let Some(python) = bumped.python.as_mut() {
        python.packages.extend([package("numpy", "2.6.0")]);
    }
    let focus = classify(Some(&before), &bumped, true);
    assert_eq!(
        (focus.depth, focus.trusted_fast_path),
        (ReviewDepth::Standard, false)
    );
    assert_eq!(
        focus.differences[0],
        Difference {
            kind: ChangedDependency,
            subject: "numpy".into(),
            detail: "Python, 2.5.4 → 2.6.0".into(),
        }
    );
    // The same version's files changed.
    let mut rebuilt = facts("1.1.0");
    if let Some(shapely) = rebuilt
        .python
        .as_mut()
        .and_then(|p| p.packages.get_mut("shapely"))
    {
        shapely.files = sha256_hex(b"other wheels");
    }
    assert_eq!(
        classify(Some(&before), &rebuilt, true).differences[0].detail,
        "Python, 2.0.7: aynı sürümün dosyaları değişti"
    );
    // The lock alone.
    let mut relocked = facts("1.1.0");
    if let Some(python) = relocked.python.as_mut() {
        python.lock_compared = sha256_hex(b"another lock");
    }
    let focus = classify(Some(&before), &relocked, true);
    assert_eq!(
        (focus.depth, kinds(&focus)[0]),
        (ReviewDepth::Standard, (Lock, "uv.lock"))
    );
}

#[test]
fn facts_come_from_the_package_and_a_version_bump_is_not_code() {
    let limits = Limits::default();
    let folder = fixture("ornek-imar");
    let opened = open(&pack(&folder, &limits).unwrap().bytes, &limits);
    let v110 = ReleaseFacts::of(&opened).unwrap();
    let python = v110.python.as_ref().unwrap();
    assert!(python.packages.contains_key("numpy") && python.packages.contains_key("shapely"));
    assert!(
        !python.packages.contains_key("imar-denetim"),
        "the project itself is not its own dependency"
    );
    let content = v110.content();
    assert_eq!(Some(&content.package), opened.inspection.digest.as_ref());
    assert_eq!(content.lock, Some(sha256_hex(&folder["uv.lock"])));
    assert_eq!(
        content.capabilities,
        [
            Capability::DocumentRead,
            Capability::ProcessingProvider,
            Capability::ExpressionProvider,
            Capability::UiRibbonTab,
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        serde_json::from_value::<ReleaseFacts>(serde_json::to_value(&v110).unwrap()).unwrap(),
        v110
    );
    // 1.1.1: the version in the manifest, the project and the lock, and a line of change notes.
    let mut bumped = folder.clone();
    let mut edit = |path: &str, from: &str, to: &str| {
        let text = String::from_utf8(bumped[path].clone()).unwrap();
        assert!(text.contains(from), "{path}");
        bumped.insert(path.to_owned(), text.replacen(from, to, 1).into_bytes());
    };
    edit(
        "manifest.json",
        "\"version\": \"1.1.0\"",
        "\"version\": \"1.1.1\"",
    );
    edit(
        "pyproject.toml",
        "version = \"1.1.0\"",
        "version = \"1.1.1\"",
    );
    edit(
        "uv.lock",
        "name = \"imar-denetim\"\nversion = \"1.1.0\"",
        "name = \"imar-denetim\"\nversion = \"1.1.1\"",
    );
    edit(
        "CHANGELOG.md",
        "# Değişiklikler\n",
        "# Değişiklikler\n\n## 1.1.1 — 15 Kasım 2026\n\n- Yazım düzeltmeleri.\n",
    );
    let v111 = ReleaseFacts::of(&open(&pack(&bumped, &limits).unwrap().bytes, &limits)).unwrap();
    assert_ne!(v111.content(), v110.content());
    assert_ne!(v111.python.as_ref().unwrap().lock, python.lock);
    let focus = classify(Some(&v110), &v111, false);
    assert_eq!(
        (focus.depth, kinds(&focus)),
        (
            ReviewDepth::Fast,
            vec![(DifferenceKind::Text, "CHANGELOG.md")]
        )
    );
    // Its approved release installs 1.1.0's bytes, not 1.1.1's.
    let mut release =
        Release::upload(&author(), CatalogScope::Public, PUBLISHER, &v110, &[], t(0)).unwrap();
    scan_all(&mut release, &ScanOutcome::Passed, t(1));
    release.submit(&author(), None, t(2)).unwrap();
    release
        .start_review(&reviewer(), &v110, None, false, t(3))
        .unwrap();
    release.approve(&reviewer(), None, t(4)).unwrap();
    release.publish(&author(), t(5)).unwrap();
    release.allows_install(&v110.content()).unwrap();
    assert_eq!(
        refused(release.allows_install(&v111.content())),
        "content_mismatch"
    );
    // A package that does not pass has no facts.
    let mut broken = folder.clone();
    broken.insert("manifest.json".into(), b"{}".to_vec());
    let entries: Vec<(&str, &[u8])> = broken
        .iter()
        .map(|(p, d)| (p.as_str(), d.as_slice()))
        .collect();
    let bytes = kentos_zip::write(&entries).unwrap();
    assert_eq!(
        ReleaseFacts::of(&open(&bytes, &limits)).unwrap_err().code,
        "package_invalid"
    );
}

/// An instant read as the lock writes it: the newest of a package's files, fractions of any length (docs/adr/8057 §85).
#[test]
fn an_upload_time_is_read_as_an_instant() {
    assert_eq!(super::seconds_of("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(
        super::seconds_of("2026-11-16T00:00:00Z"),
        Some(1_794_787_200)
    );
    assert_eq!(
        super::seconds_of("2000-03-01T12:30:15.5Z"),
        Some(951_913_815)
    );
    assert_eq!(super::seconds_of("2026-11-16"), None);
    assert_eq!(super::seconds_of("2026-13-01T00:00:00Z"), None);
    let lock = "version = 1\nrevision = 3\nrequires-python = \">=3.12\"\n\n[[package]]\nname = \"proje\"\nversion = \"0.1.0\"\n\
                source = { virtual = \".\" }\ndependencies = [{ name = \"ornek\" }]\n\n[[package]]\nname = \"ornek\"\n\
                version = \"1.0\"\nsource = { registry = \"https://pypi.org/simple\" }\nsdist = { url = \"https://x/o.tar.gz\", \
                hash = \"sha256:aa\", size = 1, upload-time = \"2026-10-10T20:02:43.45Z\" }\nwheels = [\n    { url = \
                \"https://x/o.whl\", hash = \"sha256:bb\", size = 1, upload-time = \"2026-10-10T20:02:43.4Z\" },\n    { url = \
                \"https://x/o2.whl\", hash = \"sha256:cc\", size = 1, upload-time = \"2026-10-10T20:05:31.422Z\" },\n]\n";
    let project = crate::manifest::PythonProject {
        project_file: "pyproject.toml".into(),
        lock_file: "uv.lock".into(),
        optional_features: Default::default(),
    };
    let files = std::collections::BTreeMap::from([
        (
            "pyproject.toml".to_owned(),
            b"[project]\nname = \"proje\"\nversion = \"0.1.0\"\n".to_vec(),
        ),
        ("uv.lock".to_owned(), lock.as_bytes().to_vec()),
    ]);
    let packages = super::locked_packages(&project, &files).expect("read");
    assert_eq!(
        packages["ornek"].uploaded.as_deref(),
        Some("2026-10-10T20:05:31.422Z")
    );
}

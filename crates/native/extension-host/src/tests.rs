//! ADIM 17's acceptance (plan/eklenti.md): a Python-free package installed, enabled, disabled, enabled again,
//! updated, taken back and removed, with nothing of a lease left after each step; a half update that does not touch
//! the running version; a crashing extension that fails for good and is not tried at the next start; a withdrawn
//! version stopped with its data kept; a dependency cycle refused.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kentos_extensions::manifest::Capability;
use kentos_extensions::version::Version;
use kentos_extensions::{Host, Limits, Os};
use serde_json::{Value, json};

use super::*;

/// A folder of its own under the system's temporary folder, removed afterwards.
struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Temp {
        let dir =
            std::env::temp_dir().join(format!("kentos-eklenti-{name}-{}", std::process::id()));
        let _ = store::remove_tree(&dir);
        fs::create_dir_all(&dir).unwrap();
        Temp(dir)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = store::remove_tree(&self.0);
    }
}

fn host() -> Host {
    Host {
        sdk_protocol: 1,
        kentos: Version::parse("0.2.0").unwrap(),
        os: Some(Os::Linux),
    }
}

fn open(root: &Path) -> Registry {
    Registry::open(root, host(), Options::default()).unwrap()
}

fn open_with(root: &Path, options: Options) -> Registry {
    Registry::open(root, host(), options).unwrap()
}

/// A package's files: a Python-free utility with a tile source, its manifest changed by `edit`.
fn files(
    id: &str,
    version: &str,
    deps: &[(&str, &str)],
    edit: impl Fn(&mut Value),
) -> BTreeMap<String, Vec<u8>> {
    let mut manifest = json!({
        "format": "kentos.extension",
        "schemaVersion": 1,
        "id": id,
        "version": version,
        "name": format!("Sınama {id}"),
        "summary": "Sınama paketi.",
        "authors": ["Sınama"],
        "license": "MIT",
        "licenseFile": "LICENSE",
        "changelogFile": "CHANGELOG.md",
        "kind": "utility",
        "modules": ["harita"],
        "runtimes": ["desktop"],
        "compatibility": { "sdkProtocol": { "min": 1, "max": 1 } },
        "contributions": { "dataSources": [{
            "id": "altlik", "title": format!("Altlık {version}"), "kind": "xyz",
            "url": "https://tiles.ornek.bel.tr/{z}/{x}/{y}.png"
        }] },
        "requestedCapabilities": [
            { "id": "dataSource", "scope": "plugin-owned", "reason": "Altlık ekler." },
            { "id": "network", "scope": "plugin-owned", "reason": "Karo sunucusunu okur.",
              "hosts": ["tiles.ornek.bel.tr"], "sendsDataToAi": false }
        ]
    });
    if !deps.is_empty() {
        let list: Vec<Value> = deps
            .iter()
            .map(|(i, v)| json!({"id": i, "version": v}))
            .collect();
        manifest["dependencies"] = json!({ "extensions": list });
    }
    edit(&mut manifest);
    BTreeMap::from([
        (
            "manifest.json".to_owned(),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        ),
        ("LICENSE".to_owned(), b"MIT".to_vec()),
        (
            "CHANGELOG.md".to_owned(),
            format!("# {version}\n").into_bytes(),
        ),
    ])
}

fn package(id: &str, version: &str, deps: &[(&str, &str)]) -> Vec<u8> {
    package_with(id, version, deps, |_| {})
}

fn package_with(
    id: &str,
    version: &str,
    deps: &[(&str, &str)],
    edit: impl Fn(&mut Value),
) -> Vec<u8> {
    kentos_extensions::pack(&files(id, version, deps, edit), &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

fn file_source() -> Source {
    Source {
        kind: SourceKind::File,
        location: "/indirilenler/paket.kek".to_owned(),
    }
}

fn all_granted() -> Choice {
    Choice {
        grant: Grant {
            capabilities: vec![Capability::DataSource, Capability::Network],
            python: true,
        },
        enable: true,
        replace_source: false,
    }
}

fn install(r: &mut Registry, bytes: &[u8]) -> Installed {
    r.install(bytes, &file_source(), &all_granted())
        .unwrap_or_else(|p| panic!("{p:#?}"))
}

/// The contributions under any lease but `current`: must be none.
fn leftovers(r: &Registry, current: Option<&ExtensionLease>) -> Vec<String> {
    r.contributions()
        .iter()
        .filter(|c| Some(&c.lease) != current)
        .map(|c| c.id.clone())
        .collect()
}

const ID: &str = "tr.ornek.altlik";

#[test]
fn the_whole_lifecycle_leaves_nothing_of_a_lease_behind() {
    let temp = Temp::new("yasam");
    let mut r = open(&temp.0);
    let first = install(&mut r, &package(ID, "1.0.0", &[]));
    let l1 = first.activation.unwrap().unwrap().lease;
    assert_eq!(r.state(ID), Some(State::Active));
    assert_eq!(r.contributions().len(), 1);
    assert_eq!(r.contributions()[0].id, "ext.tr.ornek.altlik.altlik");
    assert_eq!(r.contributions()[0].kind, ContributionKind::DataSource);

    assert_eq!(r.disable(ID).unwrap(), Vec::<String>::new());
    assert_eq!(r.state(ID), Some(State::Disabled));
    assert!(r.contributions().is_empty() && !r.is_current(&l1));

    let l2 = r.enable(ID).unwrap().unwrap().lease;
    assert!(l2.generation > l1.generation);
    assert_eq!(leftovers(&r, Some(&l2)), Vec::<String>::new());
    // How the activation went is kept for the manager; a run's measures come from its process.
    r.record_metrics(&l2, 48 << 20, 1).unwrap();
    r.record_metrics(&l1, 1, 9).unwrap();
    let metrics = r.extensions()[0].metrics.unwrap();
    assert_eq!(
        (metrics.memory_bytes, metrics.processes),
        (48 << 20, 1),
        "a stopped lease's measures are not kept"
    );

    let update = install(&mut r, &package(ID, "1.1.0", &[]));
    assert_eq!(update.previous.as_deref(), Some("1.0.0"));
    let l3 = update.activation.unwrap().unwrap().lease;
    assert_eq!(l3.version, "1.1.0");
    assert_eq!(leftovers(&r, Some(&l3)), Vec::<String>::new());
    assert_eq!(r.contributions()[0].title, "Altlık 1.1.0");

    let back = r.switch_version(ID, "1.0.0").unwrap().unwrap();
    assert_eq!(
        (
            back.lease.version.as_str(),
            r.contributions()[0].title.as_str()
        ),
        ("1.0.0", "Altlık 1.0.0")
    );
    assert_eq!(leftovers(&r, Some(&back.lease)), Vec::<String>::new());
    let view = &r.extensions()[0];
    assert_eq!(
        (view.version.as_str(), view.versions.clone()),
        ("1.0.0", vec!["1.0.0".to_owned(), "1.1.0".to_owned()])
    );

    // A restart reads it all back from the disk.
    drop(r);
    let mut r = open(&temp.0);
    assert_eq!(r.state(ID), Some(State::Ready));
    let started = r.startup();
    assert_eq!(started.len(), 1);
    assert!(started[0].1.is_ok());

    assert_eq!(r.package_file(ID, "LICENSE").as_deref(), Some(&b"MIT"[..]));
    assert_eq!(r.package_file(ID, "../registry.json"), None);
    let data = r.data_dir(ID).unwrap();
    fs::write(data.join("ayar.json"), "{}").unwrap();
    assert_eq!(r.remove(ID, true).unwrap(), Vec::<String>::new());
    assert!(r.contributions().is_empty() && r.extensions().is_empty() && r.state(ID).is_none());
    assert!(
        !temp.0.join("packages").join(ID).exists(),
        "its files are gone"
    );
    assert!(
        data.join("ayar.json").exists(),
        "its data stays unless asked"
    );
    drop(r);
    let r = open(&temp.0);
    assert!(r.extensions().is_empty());
}

#[test]
fn a_failed_update_leaves_the_running_version_untouched() {
    let temp = Temp::new("yarim");
    let mut r = open(&temp.0);
    let lease = install(&mut r, &package(ID, "1.0.0", &[]))
        .activation
        .unwrap()
        .unwrap()
        .lease;
    let before = r.contributions().to_vec();
    let mut broken = package(ID, "1.1.0", &[]);
    let middle = broken.len() / 2;
    broken.truncate(middle);
    assert!(r.install(&broken, &file_source(), &all_granted()).is_err());
    assert!(r.is_current(&lease));
    assert_eq!(r.contributions(), before.as_slice());
    assert_eq!(r.extensions()[0].version, "1.0.0");
    // A crash in the middle of preparing leaves a staging folder: the next start removes it, the old version runs.
    let leftover = temp.0.join("staging").join(format!("{ID}@1.1.0.1.1"));
    fs::create_dir_all(&leftover).unwrap();
    fs::write(leftover.join("manifest.json"), "{yarım").unwrap();
    // Another process's install in progress is left alone; a crashed one's (an hour old) is removed.
    let fresh = temp.0.join("staging").join(format!("{ID}@1.2.0.2.1"));
    fs::create_dir_all(&fresh).unwrap();
    let hours_ago = std::time::SystemTime::now() - Duration::from_secs(7200);
    fs::File::open(&leftover)
        .unwrap()
        .set_modified(hours_ago)
        .unwrap();
    drop(r);
    let mut r = open(&temp.0);
    assert!(!leftover.exists() && fresh.exists());
    assert!(r.startup()[0].1.is_ok());
    assert_eq!(r.lease(ID).unwrap().version, "1.0.0");
}

#[test]
fn the_same_version_with_other_content_is_refused() {
    let temp = Temp::new("ayni");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    let again = install(&mut r, &package(ID, "1.0.0", &[]));
    assert!(again.already, "the same bytes: nothing to do");
    let other = package_with(ID, "1.0.0", &[], |m| m["summary"] = json!("Başka içerik."));
    let p = r
        .install(&other, &file_source(), &all_granted())
        .unwrap_err();
    assert_eq!(p[0].code, "registry_same_version");
}

#[test]
fn a_crashing_extension_fails_for_good_and_is_not_tried_at_the_next_start() {
    let temp = Temp::new("bekci");
    let mut r = open(&temp.0);
    let lease = install(&mut r, &package(ID, "1.0.0", &[]))
        .activation
        .unwrap()
        .unwrap()
        .lease;
    assert_eq!(
        r.report_crash(&lease, "bellek hatası").unwrap(),
        Crash::Counted(1)
    );
    r.report_healthy(&lease).unwrap();
    assert_eq!(
        r.report_crash(&lease, "bellek hatası").unwrap(),
        Crash::Counted(1),
        "a good run starts the count again"
    );
    assert_eq!(
        r.report_crash(&lease, "bellek hatası").unwrap(),
        Crash::Counted(2)
    );
    assert_eq!(
        r.report_crash(&lease, "bellek hatası").unwrap(),
        Crash::Failed
    );
    assert!(r.contributions().is_empty());
    assert_eq!(
        r.report_crash(&lease, "geç gelen").unwrap(),
        Crash::Ignored,
        "a stopped lease's report"
    );
    let Some(State::Failed { reason }) = r.state(ID) else {
        panic!("{:?}", r.state(ID))
    };
    assert!(
        reason.contains("Art arda 3 kez çöktü") && reason.contains("bellek hatası"),
        "{reason}"
    );
    drop(r);
    let mut r = open(&temp.0);
    assert!(r.startup().is_empty(), "not tried at start-up");
    assert!(matches!(r.state(ID), Some(State::Failed { .. })));
    assert!(r.activate(ID).is_err());
    let retried = r.retry(ID).unwrap().unwrap();
    assert_eq!(r.state(ID), Some(State::Active));
    assert!(r.is_current(&retried.lease));
    let log: Vec<String> = r.extensions()[0]
        .log
        .iter()
        .map(|e| e.message.clone())
        .collect();
    assert!(
        log.iter().any(|l| l.starts_with("Çöktü (3. kez art arda)")),
        "{log:#?}"
    );
}

#[test]
fn a_withdrawn_version_stops_and_its_data_stays() {
    let temp = Temp::new("geri-cekme");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    let data = r.data_dir(ID).unwrap();
    fs::write(data.join("kayit.txt"), "kullanıcının verisi").unwrap();
    let list = Revocations::read(
        &json!({
            "format": "kentos.extension-revocations", "schemaVersion": 1,
            "entries": [{ "id": ID, "versions": "<1.1", "reason": "kullanıcı verisini dışarı gönderiyor" }]
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(r.apply_revocations(list).unwrap(), vec![ID.to_owned()]);
    assert!(r.contributions().is_empty());
    let view = &r.extensions()[0];
    assert!(
        matches!(&view.state, State::Blocked { reason } if reason.contains("dışarı gönderiyor")),
        "{:?}",
        view.state
    );
    assert_eq!(view.trust, Trust::Blocked);
    assert!(data.join("kayit.txt").exists());
    let p = r
        .install(&package(ID, "1.0.1", &[]), &file_source(), &all_granted())
        .unwrap_err();
    assert_eq!(p[0].code, "registry_revoked");
    drop(r);
    let mut r = open(&temp.0);
    assert!(r.startup().is_empty(), "the list holds offline");
    // A fixed version is allowed and runs.
    let fixed = install(&mut r, &package(ID, "1.1.0", &[]));
    assert!(fixed.activation.unwrap().is_ok());
}

#[test]
fn dependencies_order_the_start_and_a_cycle_is_refused() {
    let temp = Temp::new("bagimlilik");
    let mut r = open(&temp.0);
    let base = "tr.ornek.kent-temel";
    let app = "tr.ornek.mebis";
    let first = install(&mut r, &package(app, "1.0.0", &[(base, "^1.2")]));
    assert!(
        first.activation.unwrap().is_err(),
        "its dependency is not there"
    );
    assert!(
        matches!(r.state(app), Some(State::Waiting { reason }) if reason.contains("kurulu değil"))
    );
    install(&mut r, &package(base, "1.2.0", &[]));
    assert_eq!(
        r.state(app),
        Some(State::Active),
        "it ran when its dependency came"
    );
    assert_eq!(r.disable(base).unwrap(), vec![app.to_owned()]);
    assert!(
        matches!(r.state(app), Some(State::Waiting { reason }) if reason.contains("etkin değil"))
    );
    r.enable(base).unwrap();
    assert_eq!(r.state(app), Some(State::Active));
    // base 1.3.0 depending on the app closes a circle.
    let circle = r
        .install(
            &package(base, "1.3.0", &[(app, "^1")]),
            &file_source(),
            &all_granted(),
        )
        .unwrap_err();
    assert_eq!(circle[0].code, "registry_dependency_cycle");
    assert!(circle[0].message.contains("→"), "{}", circle[0].message);
    // An update outside the range: the app waits.
    install(&mut r, &package(base, "2.0.0", &[]));
    assert!(
        matches!(r.state(app), Some(State::Waiting { reason }) if reason.contains("^1.2 gerekli"))
    );
    r.switch_version(base, "1.2.0").unwrap();
    assert_eq!(r.state(app), Some(State::Active));
    drop(r);
    let mut r = open(&temp.0);
    let order: Vec<String> = r.startup().into_iter().map(|(id, _)| id).collect();
    assert_eq!(order, [base, app], "the dependency first");
}

#[test]
fn another_source_of_the_same_id_is_the_users_choice() {
    let temp = Temp::new("kaynak");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    let catalog = Source {
        kind: SourceKind::Organization,
        location: "https://katalog.ornek.bel.tr".to_owned(),
    };
    let bytes = package(ID, "1.1.0", &[]);
    let review = r.review(&bytes, &catalog);
    assert!(review.source_conflict && review.installable);
    assert_eq!(
        review.installed.as_ref().map(|(v, _)| v.as_str()),
        Some("1.0.0")
    );
    let p = r.install(&bytes, &catalog, &all_granted()).unwrap_err();
    assert_eq!(p[0].code, "registry_source_conflict");
    let chosen = Choice {
        replace_source: true,
        ..all_granted()
    };
    r.install(&bytes, &catalog, &chosen).unwrap();
    let view = &r.extensions()[0];
    assert_eq!(
        (view.trust, view.source.kind),
        (Trust::Organization, SourceKind::Organization)
    );
}

#[test]
fn the_organisations_policy_cannot_be_loosened() {
    let temp = Temp::new("ilke");
    let policy = |extra: Value| {
        let mut p = json!({"format": "kentos.extension-policy", "schemaVersion": 1});
        for (k, v) in extra.as_object().unwrap() {
            p[k] = v.clone();
        }
        Policy::read(&p.to_string()).unwrap()
    };
    let only_catalog = Options {
        policy: Some(policy(json!({"allowedSources": ["organization"]}))),
        ..Options::default()
    };
    let mut r = open_with(&temp.0, only_catalog);
    let p = r
        .install(&package(ID, "1.0.0", &[]), &file_source(), &all_granted())
        .unwrap_err();
    assert_eq!(p[0].code, "registry_policy");
    assert!(
        p[0].message.contains("dosya kaynağından"),
        "{}",
        p[0].message
    );
    drop(r);
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    drop(r);
    // IT blocks the network capability afterwards: the installed extension stops, it is not deleted.
    let no_network = Options {
        policy: Some(policy(json!({"blockedCapabilities": ["network"]}))),
        ..Options::default()
    };
    let mut r = open_with(&temp.0, no_network);
    assert!(r.startup().is_empty());
    assert!(
        matches!(r.state(ID), Some(State::Blocked { reason }) if reason.contains("İnternete bağlanır (network)"))
    );
    assert_eq!(r.extensions()[0].trust, Trust::Blocked);
    let strict = Options {
        policy: Some(policy(
            json!({"allowDataToAi": false, "minimumVersions": { "tr.ornek.altlik": "1.2.0" }}),
        )),
        ..Options::default()
    };
    let r = open_with(&temp.0, strict);
    assert!(
        matches!(r.state(ID), Some(State::Blocked { reason }) if reason.contains("en az 1.2.0"))
    );
}

#[test]
fn what_the_user_did_not_grant_is_left_out() {
    let temp = Temp::new("izin");
    let mut r = open(&temp.0);
    let only_sources = Choice {
        grant: Grant {
            capabilities: vec![Capability::DataSource],
            python: false,
        },
        enable: true,
        replace_source: false,
    };
    let installed = r
        .install(&package(ID, "1.0.0", &[]), &file_source(), &only_sources)
        .unwrap();
    let activation = installed.activation.unwrap().unwrap();
    assert!(r.contributions().is_empty());
    assert_eq!(activation.skipped.len(), 1);
    assert!(
        activation.skipped[0].reason.contains("network"),
        "{:?}",
        activation.skipped
    );
    let foreign = Choice {
        grant: Grant {
            capabilities: vec![Capability::FilesUserSelected],
            python: false,
        },
        ..only_sources
    };
    let p = r
        .install(&package(ID, "1.0.1", &[]), &file_source(), &foreign)
        .unwrap_err();
    assert_eq!(
        p[0].code, "registry_grant",
        "a capability the package does not ask for"
    );
}

/// The MEBİS golden folder without its dependency on another extension.
fn mebis_folder(dir: &Path) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/extensions/v1/mebis");
    for (path, data) in store::read_files(&root).unwrap() {
        let target = dir.join(&path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        let data = if path == "manifest.json" {
            let mut m: Value = serde_json::from_slice(&data).unwrap();
            m.as_object_mut().unwrap().remove("dependencies");
            serde_json::to_vec_pretty(&m).unwrap()
        } else {
            data
        };
        fs::write(target, data).unwrap();
    }
}

#[test]
fn python_contributions_wait_for_the_choice_and_a_working_folder_reloads_cleanly() {
    let temp = Temp::new("gelistirici");
    let folder = temp.0.join("mebis-kaynak");
    mebis_folder(&folder);
    let mut r = open(&temp.0.join("kayit"));
    let text = fs::read_to_string(folder.join("manifest.json")).unwrap();
    let granted: Vec<Capability> = kentos_extensions::Manifest::read(&text)
        .unwrap()
        .capabilities()
        .into_iter()
        .collect();
    let no_python = Choice {
        grant: Grant {
            capabilities: granted.clone(),
            python: false,
        },
        enable: true,
        replace_source: false,
    };
    let first = r.install_folder(&folder, &no_python).unwrap();
    let a1 = first.activation.unwrap().unwrap();
    let kinds: Vec<ContributionKind> = r.contributions().iter().map(|c| c.kind).collect();
    assert!(
        kinds.contains(&ContributionKind::Form) && kinds.contains(&ContributionKind::DataSource)
    );
    assert!(
        kinds.contains(&ContributionKind::RibbonTab) && !kinds.contains(&ContributionKind::Command)
    );
    assert!(
        a1.skipped
            .iter()
            .any(|s| s.id == "ext.tr.ornek.mebis.init-schema" && s.reason.contains("Python"))
    );
    assert_eq!(r.extensions()[0].trust, Trust::Local);
    // The author changes a title in the working folder and reloads: same version, a new lease, nothing old left.
    let path = folder.join("manifest.json");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("Mezarlık BS", "Mezarlık Bilgi Sistemi");
    fs::write(&path, text).unwrap();
    let reloaded = r.reload("tr.ornek.mebis").unwrap();
    assert_eq!(reloaded.version, "1.0.0");
    let a2 = reloaded.activation.unwrap().unwrap();
    assert!(a2.lease.generation > a1.lease.generation);
    assert_eq!(leftovers(&r, Some(&a2.lease)), Vec::<String>::new());
    let tab = r
        .contributions()
        .iter()
        .find(|c| c.kind == ContributionKind::RibbonTab)
        .unwrap();
    assert_eq!(tab.title, "Mezarlık Bilgi Sistemi");
    // With Python chosen, the commands are registered (they start their process when first called).
    let with_python = Choice {
        grant: Grant {
            capabilities: granted,
            python: true,
        },
        enable: true,
        replace_source: false,
    };
    r.install_folder(&folder, &with_python).unwrap();
    let command = r
        .contributions()
        .iter()
        .find(|c| c.id == "ext.tr.ornek.mebis.init-schema")
        .unwrap();
    assert!(command.needs_python);
    assert!(r.reload(ID).is_err(), "not installed");
}

#[test]
fn files_changed_on_disk_are_caught_before_they_run() {
    let temp = Temp::new("butunluk");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    r.disable(ID).unwrap();
    let file = temp
        .0
        .join("packages")
        .join(ID)
        .join("1.0.0")
        .join("manifest.json");
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    assert!(permissions.readonly(), "installed files are read-only");
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&file, permissions).unwrap();
    let text = fs::read_to_string(&file)
        .unwrap()
        .replace("Sınama paketi.", "Değiştirilmiş.");
    fs::write(&file, text).unwrap();
    let p = r.enable(ID).unwrap_err();
    assert_eq!(p.code, "registry_integrity");
    let view = &r.extensions()[0];
    assert!(matches!(view.state, State::Failed { .. }));
    assert_eq!(view.trust, Trust::IntegrityBroken);
    drop(r);
    let mut r = open(&temp.0);
    assert!(r.startup().is_empty());
    // Installing the package again mends it.
    let mended = install(&mut r, &package(ID, "1.0.0", &[]));
    assert!(!mended.already && mended.activation.unwrap().is_ok());
}

#[test]
fn a_newer_protocol_keeps_the_extension_installed_but_stopped() {
    let temp = Temp::new("protokol");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    drop(r);
    let newer = Host {
        sdk_protocol: 2,
        ..host()
    };
    let mut r = Registry::open(&temp.0, newer, Options::default()).unwrap();
    assert!(r.startup().is_empty());
    assert!(
        matches!(r.state(ID), Some(State::Incompatible { reason }) if reason.contains("yeni sürümünü"))
    );
    assert_eq!(r.extensions().len(), 1);
}

#[test]
fn the_review_shows_what_changes_before_anything_is_installed() {
    let temp = Temp::new("inceleme");
    let mut r = open(&temp.0);
    let first = r.review(&package(ID, "1.0.0", &[]), &file_source());
    assert!(first.installable && first.installed.is_none());
    assert!(first.capabilities.iter().all(|c| c.change == Change::Asked));
    assert!(r.extensions().is_empty(), "reviewing installs nothing");
    install(&mut r, &package(ID, "1.0.0", &[]));
    let update = package_with(ID, "1.1.0", &[], |m| {
        m["requestedCapabilities"][1]["reason"] = json!("Karo ve arama sunucusunu okur.");
        m["requestedCapabilities"].as_array_mut().unwrap().push(
            json!({"id": "files.userSelected", "scope": "user-selected", "reason": "Seçtiğiniz dosyayı okur."}),
        );
    });
    let review = r.review(&update, &file_source());
    let change = |c: Capability| {
        review
            .capabilities
            .iter()
            .find(|l| l.capability == c)
            .map(|l| l.change)
    };
    assert_eq!(change(Capability::DataSource), Some(Change::Same));
    assert_eq!(change(Capability::Network), Some(Change::Changed));
    assert_eq!(change(Capability::FilesUserSelected), Some(Change::Added));
    install(&mut r, &update);
    let back = r.review(&package(ID, "1.2.0", &[]), &file_source());
    assert_eq!(
        back.capabilities
            .iter()
            .find(|l| l.capability == Capability::FilesUserSelected)
            .map(|l| l.change),
        Some(Change::Removed)
    );
    let bad = r.review(b"paket degil", &file_source());
    assert!(!bad.installable && !bad.inspection.is_valid());
}

#[test]
fn a_projects_extensions_open_restricted_until_ready() {
    let temp = Temp::new("proje");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    let needs = r.project_needs(&[
        (ID.to_owned(), "^1".to_owned()),
        (ID.to_owned(), "^2".to_owned()),
        ("tr.ornek.yok".to_owned(), "^1".to_owned()),
    ]);
    assert_eq!(needs[0].status, NeedStatus::Ready);
    assert!(matches!(&needs[1].status, NeedStatus::NotRunning { reason } if reason.contains("^2")));
    assert_eq!(needs[2].status, NeedStatus::Missing);
    r.disable(ID).unwrap();
    let needs = r.project_needs(&[(ID.to_owned(), "^1".to_owned())]);
    assert!(
        matches!(&needs[0].status, NeedStatus::NotRunning { reason } if reason.contains("kapattınız"))
    );
}

#[test]
fn the_dump_shows_what_would_run_without_running_it() {
    let temp = Temp::new("dokum");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    drop(r);
    let r = open(&temp.0);
    let before = fs::read(temp.0.join("registry.json")).unwrap();
    let (planned, skipped) = r.planned_contributions();
    assert_eq!(planned.len(), 1);
    assert_eq!(
        (planned[0].id.as_str(), planned[0].lease.generation),
        ("ext.tr.ornek.altlik.altlik", 0)
    );
    assert!(
        skipped.is_empty() && r.contributions().is_empty(),
        "nothing activated"
    );
    assert_eq!(
        fs::read(temp.0.join("registry.json")).unwrap(),
        before,
        "nothing written"
    );
}

#[test]
fn safe_mode_starts_nothing() {
    let temp = Temp::new("guvenli");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    drop(r);
    let safe = Options {
        safe_mode: true,
        ..Options::default()
    };
    let mut r = open_with(&temp.0, safe);
    assert!(r.startup().is_empty());
    assert_eq!(r.state(ID), Some(State::Ready));
    assert_eq!(
        r.enable(ID).unwrap(),
        None,
        "enabling records the wish, runs nothing"
    );
    assert!(r.contributions().is_empty());
}

#[test]
fn two_processes_share_one_registry() {
    let temp = Temp::new("iki-surec");
    let mut desktop = open(&temp.0);
    let mut headless = open(&temp.0);
    install(&mut desktop, &package(ID, "1.0.0", &[]));
    // The other process reads the registry again under the lock before it changes it.
    headless.set_channel(ID, Channel::Preview).unwrap();
    headless.pin(ID, Some("1.0.0".to_owned())).unwrap();
    let again = open(&temp.0);
    let view = &again.extensions()[0];
    assert_eq!(
        (view.channel, view.pinned.as_deref()),
        (Channel::Preview, Some("1.0.0"))
    );
    // A lock another process holds: the change waits, then says so.
    fs::write(temp.0.join(".lock"), "123").unwrap();
    let mut busy = open_with(
        &temp.0,
        Options {
            lock_wait: Duration::from_millis(60),
            ..Options::default()
        },
    );
    assert_eq!(
        busy.set_channel(ID, Channel::Stable).unwrap_err().code,
        "registry_busy"
    );
    // A crashed process's lock (older than two minutes) is taken over.
    let old = std::time::SystemTime::now() - Duration::from_secs(600);
    fs::File::options()
        .write(true)
        .open(temp.0.join(".lock"))
        .unwrap()
        .set_modified(old)
        .unwrap();
    busy.set_channel(ID, Channel::Stable).unwrap();
    assert!(!temp.0.join(".lock").exists(), "released after the change");
}

#[test]
fn every_state_says_what_to_do_in_turkish() {
    let states = [
        State::Active,
        State::Ready,
        State::Disabled,
        State::Waiting {
            reason: "“x” kurulu değil.".into(),
        },
        State::Blocked {
            reason: "Kurum ilkesi engelliyor.".into(),
        },
        State::Incompatible {
            reason: "Bu KentOS için değil.".into(),
        },
        State::Failed {
            reason: "Art arda 3 kez çöktü.".into(),
        },
    ];
    for state in states {
        assert!(
            !state.label().is_empty() && state.help().len() > 10,
            "{state:?}"
        );
    }
    for trust in [
        Trust::Local,
        Trust::Organization,
        Trust::Approved,
        Trust::IntegrityBroken,
        Trust::Blocked,
    ] {
        assert!(!trust.label().is_empty());
    }
}

/// The owner's wish (13 November): when an extension fails we know clearly what, where and why. Its log says it with
/// a level, a code, the version, the part and the error as it came (which file changed); the manager's problem is the
/// newest error until new files or a retry; a state that keeps it from starting is said once, not at every start.
#[test]
fn a_failing_extension_says_what_where_and_why() {
    let temp = Temp::new("hata");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    drop(r);
    let file = temp
        .0
        .join("packages")
        .join(ID)
        .join("1.0.0")
        .join("CHANGELOG.md");
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&file, permissions).unwrap();
    fs::write(&file, "# değişti\n").unwrap();

    let mut r = open(&temp.0);
    let started = r.startup();
    assert!(
        matches!(&started[..], [(id, Err(p))] if id == ID && p.code == "registry_integrity"),
        "{started:#?}"
    );
    let problem = r.extensions()[0].problem.clone().expect("the problem");
    assert_eq!(
        (problem.level, problem.code.as_str()),
        (Level::Error, "integrity_mismatch")
    );
    assert_eq!(problem.version.as_deref(), Some("1.0.0"));
    assert_eq!(problem.part.as_deref(), Some("etkinleştirme"));
    assert!(
        problem
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("CHANGELOG.md değişmiş")),
        "which file: {problem:#?}"
    );
    // The same in the file, for the CLI, the MCP server and the publisher.
    assert_eq!(r.journal().tail(Some(ID), 1)[0], problem);
    drop(r);

    // Failed: not tried at the next starts, and that is said once.
    for _ in 0..2 {
        let mut r = open(&temp.0);
        assert!(r.startup().is_empty());
    }
    let mut r = open(&temp.0);
    let said = r
        .log_of(ID)
        .iter()
        .filter(|x| x.code == "not_started_failed")
        .count();
    assert_eq!(said, 1, "{:#?}", r.log_of(ID));

    // A retry is a fresh start: failing again is the problem again.
    assert!(r.retry(ID).is_err());
    let codes: Vec<&str> = r.log_of(ID).iter().map(|x| x.code.as_str()).collect();
    assert!(
        codes.ends_with(&["retried", "integrity_mismatch"]),
        "{codes:?}"
    );
    assert!(r.extensions()[0].problem.is_some());
    // Installing the files again mends it.
    install(&mut r, &package(ID, "1.0.0", &[]));
    assert_eq!(r.extensions()[0].problem, None);
    assert_eq!(r.state(ID), Some(State::Active));

    // An update this KentOS cannot run is refused and said in the extension's log, naming the version refused; the
    // version in use stays well.
    let newer = package_with(ID, "2.0.0", &[], |m| {
        m["compatibility"]["sdkProtocol"] = json!({ "min": 2, "max": 2 });
    });
    assert!(r.install(&newer, &file_source(), &all_granted()).is_err());
    let last = r.log_of(ID).last().unwrap().clone();
    assert_eq!(
        (last.level, last.code.as_str()),
        (Level::Warning, "install_refused")
    );
    assert!(
        last.message.contains("2.0.0 kurulmadı (dosya)"),
        "{}",
        last.message
    );
    assert_eq!(last.version.as_deref(), Some("1.0.0"), "the version in use");
    assert_eq!(r.extensions()[0].problem, None);
    // What is no one extension's goes to the registry's log.
    assert!(r.install(b"bozuk", &file_source(), &all_granted()).is_err());
    assert_eq!(r.journal().tail(None, 1)[0].code, "install_refused");
}

#[test]
fn an_unreadable_registry_is_said_once_in_the_registrys_log() {
    let temp = Temp::new("bozuk-kayit");
    fs::write(temp.0.join("registry.json"), "{ bozuk").unwrap();
    for _ in 0..2 {
        assert!(Registry::open(&temp.0, host(), Options::default()).is_err());
    }
    let log = Journal::new(&temp.0).tail(None, 10);
    assert_eq!(log.len(), 1, "{log:#?}");
    assert_eq!(
        (log[0].level, log[0].code.as_str()),
        (Level::Error, "registry_corrupt")
    );
}

/// `registry.json` kept a short history before the log (ADIM 17): it is read into the log once and no longer written.
#[test]
fn the_registrys_earlier_history_moves_into_the_log() {
    let temp = Temp::new("gecmis");
    let mut r = open(&temp.0);
    install(&mut r, &package(ID, "1.0.0", &[]));
    drop(r);
    let path = temp.0.join("registry.json");
    let mut file: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    file["extensions"][ID]["log"] =
        json!([{ "at": 1_763_000_000u64, "message": "1.0.0 kuruldu (dosya)." }]);
    fs::write(&path, serde_json::to_string(&file).unwrap()).unwrap();
    fs::remove_dir_all(temp.0.join("logs")).unwrap();

    let mut r = open(&temp.0);
    let earlier = r.log_of(ID)[0].clone();
    assert_eq!(
        (earlier.code.as_str(), earlier.at, earlier.message.as_str()),
        ("earlier", 1_763_000_000_000, "1.0.0 kuruldu (dosya).")
    );
    r.disable(ID).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(!text.contains("\"log\""), "no longer written: {text}");
    let r = open(&temp.0);
    assert_eq!(
        r.log_of(ID).iter().filter(|x| x.code == "earlier").count(),
        1
    );
}

/// "Sorunlu eklentiyi bul" (D6): the running extensions are halved round by round; the user's choices never change
/// (a round only holds extensions off); a restart in the middle goes on where it was; the culprit is named in its log;
/// at the end, or when the user leaves, every extension runs again.
#[test]
fn finding_the_culprit_holds_extensions_off_without_changing_the_users_choices() {
    use crate::watchdog::Finding;

    let temp = Temp::new("bulma");
    let mut r = open(&temp.0);
    let ids = ["tr.ornek.birinci", "tr.ornek.ikinci", "tr.ornek.ucuncu"];
    for id in ids {
        install(&mut r, &package(id, "1.0.0", &[]));
    }
    let enabled_in_file = |root: &Path| -> Vec<bool> {
        let file: Value =
            serde_json::from_str(&fs::read_to_string(root.join("registry.json")).unwrap()).unwrap();
        ids.iter()
            .map(|id| file["extensions"][id]["enabled"].as_bool().unwrap())
            .collect()
    };
    // The first round runs none.
    assert_eq!(r.start_bisection().unwrap(), Finding::Testing(Vec::new()));
    for id in ids {
        assert_eq!(r.state(id), Some(State::Held), "{id}");
    }
    assert!(r.contributions().is_empty());
    assert_eq!(
        enabled_in_file(&temp.0),
        [true, true, true],
        "the user's choices"
    );
    // Not seen with none: the first half alone.
    assert_eq!(
        r.answer_bisection(false).unwrap(),
        Finding::Testing(vec![ids[0].to_owned()])
    );
    assert_eq!(r.state(ids[0]), Some(State::Active));
    assert_eq!(r.state(ids[1]), Some(State::Held));
    // A restart in the middle goes on where it was.
    drop(r);
    let mut r = open(&temp.0);
    let started: Vec<String> = r.startup().into_iter().map(|(id, _)| id).collect();
    assert_eq!(started, [ids[0]]);
    assert_eq!(r.bisection().map(|b| b.rounds()), Some(1));
    // The culprit is the third: not seen with the first, not seen with the second.
    assert_eq!(
        r.answer_bisection(false).unwrap(),
        Finding::Testing(vec![ids[1].to_owned()])
    );
    assert_eq!(
        r.answer_bisection(false).unwrap(),
        Finding::Found(ids[2].to_owned())
    );
    for id in ids {
        assert_eq!(r.state(id), Some(State::Active), "back: {id}");
    }
    assert!(!temp.0.join("sorun-bulma.json").exists());
    let named = r
        .extensions()
        .into_iter()
        .find(|e| e.id == ids[2])
        .and_then(|e| e.problem)
        .expect("named in its log");
    assert_eq!(
        (named.code.as_str(), named.part.as_deref()),
        ("bisect_found", Some("sorun bulma"))
    );
    assert!(named.message.contains("3 turda"), "{}", named.message);
    // Seen with none: not an extension's; and leaving: back as before.
    r.start_bisection().unwrap();
    assert_eq!(r.answer_bisection(true).unwrap(), Finding::NotAnExtension);
    r.start_bisection().unwrap();
    r.stop_bisection().unwrap();
    for id in ids {
        assert_eq!(r.state(id), Some(State::Active), "{id}");
    }
    assert_eq!(enabled_in_file(&temp.0), [true, true, true]);
    let codes: Vec<String> = r
        .journal()
        .tail(None, 10)
        .into_iter()
        .map(|x| x.code)
        .collect();
    assert_eq!(
        codes,
        [
            "bisect_started",
            "bisect_started",
            "bisect_not_extension",
            "bisect_started",
            "bisect_stopped"
        ]
    );
    // Nothing running: nothing to find.
    for id in ids {
        r.disable(id).unwrap();
    }
    assert_eq!(r.start_bisection().unwrap_err().code, "bisect_nothing");
}

//! A release's Python environment prepared as the server prepares it (docs/adr/8057 §66), then a real `project.audit`
//! job run in it in the whole box. A tiny pure-Python wheel is made here and a lock names it with its hash, as an index
//! would; the package carries both. Preparing is offline, from a folder of wheels; the map the audit source reads names
//! the environment's Python; the rule imports the wheel's module in the box. A dependency with only a source
//! distribution is refused with its name and nothing is mapped; a package of another content is refused; preparing
//! again runs nothing; removing the release takes its environment and its entry. No network: uv is offline, the index
//! in the lock is never asked. Needs uv, a Python in KentOS's range (`KENTOS_TEST_PYTHON`, else `.run/py`) and KentOS's
//! library's wheel (`tools/` beside the built programs); without one it says so and passes.

use std::path::{Path, PathBuf};

use kentos_application::processing_job::Processor;
use kentos_contracts::audit_job::AuditReport;
use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
use kentos_extensions::manifest::Module;

use super::audit::{audit_task_with, python};
use super::{budget, hooks, processor};
use crate::audit_env::{self, EnvSettings};
use crate::support::audit_package::{Gone, built_tools, package, project_and_lock, wheel};

#[test]
fn a_release_s_environment_is_prepared_offline_from_wheels_and_a_real_audit_runs_in_it() {
    let Some(base) = python() else {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        return;
    };
    let found = kentos_python_env::uv::detect(None);
    let Some(uv) = found.path().filter(|_| found.ok()).map(Path::to_path_buf) else {
        eprintln!("uv yok ({}): sınama atlandı", found.words());
        return;
    };
    let Some((tools, work)) = built_tools() else {
        eprintln!(
            "KentOS'un Python kitaplığının tekerleği yok (tools/kentos-sdk.json): sınama atlandı"
        );
        return;
    };
    let _ = std::fs::remove_dir_all(&work);
    let _gone = Gone(work.clone());
    let wheels = work.join("tekerlekler");
    wheel(&base, &wheels, "kucuk", "1.0");
    let blueprint_project = blueprint(&Ask {
        id: "tr.ornek.denetim".into(),
        name: "Denetim örneği".into(),
        summary: "Bir bulut denetim işinin kuralı.".into(),
        vertical: false,
        module: Module::Cbs,
        contributions: vec![Contribution::Audit],
    })
    .unwrap()["pyproject.toml"]
        .clone();
    let (pyproject, lock) = project_and_lock(
        &uv,
        &work,
        &blueprint_project,
        &wheels,
        &[("kucuk", "1.0")],
        None,
    );
    let (bytes, digest, manifest) = package("tr.ornek.denetim", None, &pyproject, &lock);
    let version = manifest.version.to_string();
    let settings = EnvSettings {
        dir: work.join("ortamlar"),
        base_python: Some(PathBuf::from(&base)),
        wheels: Some(wheels.clone()),
        index_url: None,
        uv: Some(uv.clone()),
        sdk_dirs: vec![tools.clone()],
        pythons: Default::default(),
    };
    // Prepared: offline, from the folder, the wheel against its hash; mapped.
    let prepared =
        audit_env::prepare_release(&settings, &manifest.id, &version, Some(&digest), &bytes)
            .unwrap_or_else(|e| panic!("{e}"));
    eprintln!(
        "ortam hazırlandı: {:.2} sn ({} paket, KentOS'un kitaplığı dahil değil)",
        prepared.took.as_secs_f64(),
        prepared.packages.len()
    );
    assert!(prepared.made);
    assert_eq!(prepared.digest, digest);
    assert!(
        prepared
            .packages
            .iter()
            .any(|p| p.name == "kucuk" && p.version == "1.0"),
        "{prepared:?}"
    );
    let mapped = settings.map().get(&manifest.id, &version).expect("mapped");
    assert_eq!(PathBuf::from(&mapped), prepared.python);
    // Again: ready, nothing run, mapped the same.
    let again =
        audit_env::prepare_release(&settings, &manifest.id, &version, Some(&digest), &bytes)
            .unwrap();
    eprintln!(
        "yeniden hazırlama (hazır): {:.3} sn",
        again.took.as_secs_f64()
    );
    assert!(!again.made);
    assert_eq!(again.python, prepared.python);
    // The real audit job in the whole box, in the environment's Python: the rule imports the wheel's module.
    let done = processor()
        .process(
            &audit_task_with(
                &digest,
                &manifest,
                Some(mapped),
                Some(bytes.clone()),
                budget(),
            ),
            &hooks(),
        )
        .unwrap_or_else(|f| panic!("{f:?}"));
    let report: AuditReport = serde_json::from_value(done.details).unwrap();
    assert!(!report.findings.is_empty());
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.message.starts_with("kucuk 1.0: ")),
        "{:?}",
        report.findings
    );
    // A package of another content: refused, the map as it was.
    let other = format!("sha256:{}", "0".repeat(64));
    let refused =
        audit_env::prepare_release(&settings, &manifest.id, &version, Some(&other), &bytes)
            .unwrap_err();
    assert!(refused.contains("içerik özeti"), "{refused}");
    // A dependency with only a source distribution: refused with its name, nothing mapped, nothing half-made.
    wheel(&base, &wheels, "tasli", "1.0");
    let (pyproject, lock) = project_and_lock(
        &uv,
        &work,
        &blueprint_project,
        &wheels,
        &[("kucuk", "1.0"), ("tasli", "1.0")],
        Some(("tasli", "1.0")),
    );
    let (source_bytes, source_digest, source_manifest) =
        package("tr.ornek.tasli", None, &pyproject, &lock);
    let refused = audit_env::prepare_release(
        &settings,
        &source_manifest.id,
        &version,
        Some(&source_digest),
        &source_bytes,
    )
    .unwrap_err();
    eprintln!("kaynak paketi yalnız: {refused}");
    assert!(
        refused.contains("“tasli” 1.0") && refused.contains("tekerleği yok"),
        "{refused}"
    );
    assert_eq!(
        settings.map().get(&source_manifest.id, &version),
        None,
        "nothing mapped"
    );
    let listed = audit_env::listing(&settings).unwrap();
    assert_eq!(listed.len(), 1, "{listed:?}");
    let folders = std::fs::read_dir(kentos_python_env::extension::folder(&settings.dir))
        .unwrap()
        .flatten()
        .count();
    assert_eq!(folders, 1, "no half-made environment");
    // Removed: the environment and its entry.
    assert_eq!(
        audit_env::remove_release(&settings, &manifest.id, &version).unwrap(),
        1
    );
    assert_eq!(settings.map().get(&manifest.id, &version), None);
    assert!(!prepared.python.exists());
    let _ = std::fs::remove_dir_all(&work);
}

//! `project.audit`'s processor side (docs/adr/8057 §42) against the real child: the release's package goes into the
//! job's folder beside the request, `kentos-jobrun` measures the drawing and runs the rule in the prepared Python, and
//! its report comes back as the work's details, the same twice. A package that is not the release asked, no package
//! and no prepared interpreter are refused before any rule runs. The real rule needs a Python (`KENTOS_TEST_PYTHON`,
//! else the repository's `.run/py`); without one that test says so and passes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kentos_application::processing_job::{ProcessingTask, Processor, RunRequest};
use kentos_contracts::audit_job::{AuditReport, AuditRun};
use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
use kentos_extensions::manifest::Module;
use uuid::Uuid;

use super::{budget, hooks, processor, support};

/// Every object without an island number (`Ada`), from the facts KentOS measured.
const RULE: &str = r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Ada numarası olmayan nesneler."""
    return [ext.AuditFinding(message=f"{n.layer}: ada numarası yok", severity="info", feature_id=n.id)
            for n in facts.all_features() if not n.attrs.get("Ada")]
"#;

/// A published release as the catalog hands it over: its package's bytes, its content digest and its manifest.
fn release() -> (Vec<u8>, String, kentos_extensions::Manifest) {
    let mut files = blueprint(&Ask {
        id: "tr.ornek.denetim".into(),
        name: "Denetim örneği".into(),
        summary: "Bir bulut denetim işinin kuralı.".into(),
        vertical: false,
        module: Module::Cbs,
        contributions: vec![Contribution::Audit],
    })
    .unwrap();
    let logic = files
        .iter()
        .find(|(_, text)| text.contains(r#"@ext.audit("adsiz")"#))
        .map(|(path, _)| path.clone())
        .expect("the rule's module");
    let text = files[&logic].clone();
    let at = text.find(r#"@ext.audit("adsiz")"#).unwrap();
    files.insert(logic, format!("{}{RULE}", &text[..at]));
    let bytes: BTreeMap<String, Vec<u8>> = files
        .into_iter()
        .map(|(k, v)| (k, v.into_bytes()))
        .collect();
    let packed = kentos_extensions::pack(&bytes, &kentos_extensions::Limits::default()).unwrap();
    let opened = kentos_extensions::open(&packed.bytes, &kentos_extensions::Limits::default());
    let manifest = opened.inspection.manifest.expect("the manifest");
    (packed.bytes, packed.digest, manifest)
}

/// The job's work: the parcels as the server reads them, the rule of `manifest` at `digest`, the package beside it.
fn audit_task(
    digest: &str,
    manifest: &kentos_extensions::Manifest,
    python: Option<String>,
    package: Option<Vec<u8>>,
) -> ProcessingTask {
    ProcessingTask {
        job: Uuid::now_v7(),
        snapshot: support::PARCELS.to_vec(),
        request: RunRequest::Audit(AuditRun {
            extension: manifest.id.clone(),
            version: manifest.version.to_string(),
            digest: digest.to_owned(),
            rule: "adsiz".into(),
            python,
            timeout_seconds: None,
        }),
        file: None,
        package,
        budget: budget(),
    }
}

/// The interpreter the tests run real rules with: `KENTOS_TEST_PYTHON`, else the repository's (`.run/py`).
fn python() -> Option<String> {
    let path = std::env::var_os("KENTOS_TEST_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/py/bin/python"));
    // Absolute, not resolved: an environment's interpreter is a link whose own place makes it the environment's.
    let path = std::path::absolute(path).ok().filter(|p| p.exists())?;
    Some(path.display().to_string())
}

/// What is refused before any rule runs, by the child, in its codes: a package of another content than the digest
/// asked, no package beside the request, a version the server prepared no interpreter for.
#[test]
fn an_audit_without_its_release_or_its_interpreter_is_refused_before_a_rule_runs() {
    let (bytes, digest, manifest) = release();
    let interpreter = Some("/srv/kentos/python/denetim-1.0.0/bin/python".to_owned());
    let other = format!("sha256:{}", "0".repeat(64));
    let failure = processor()
        .process(
            &audit_task(&other, &manifest, interpreter.clone(), Some(bytes.clone())),
            &hooks(),
        )
        .expect_err("not the release asked");
    assert_eq!(failure.code, "invalid", "{failure:?}");
    assert!(failure.message.contains("içerik özeti"), "{failure:?}");
    let failure = processor()
        .process(&audit_task(&digest, &manifest, interpreter, None), &hooks())
        .expect_err("no package");
    assert_eq!(failure.code, "invalid", "{failure:?}");
    assert!(failure.message.contains("paketi"), "{failure:?}");
    let failure = processor()
        .process(&audit_task(&digest, &manifest, None, Some(bytes)), &hooks())
        .expect_err("no interpreter");
    assert_eq!(failure.code, "tool_unavailable", "{failure:?}");
}

/// The rule runs in the child on the server's file, from the package beside the request; its report is of the release
/// and rule asked, stamps the drawing and its facts, and is the same twice.
#[test]
fn an_audit_runs_the_release_beside_the_request_and_reports_the_same_twice() {
    let Some(python) = python() else {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        return;
    };
    let (bytes, digest, manifest) = release();
    let entry = &manifest.entrypoints.audits[0];
    let mut reports = Vec::new();
    for _ in 0..2 {
        let done = processor()
            .process(
                &audit_task(
                    &digest,
                    &manifest,
                    Some(python.clone()),
                    Some(bytes.clone()),
                ),
                &hooks(),
            )
            .expect("the rule ran");
        assert!(
            done.summary.starts_with(&format!("{}: ", entry.title)),
            "{}",
            done.summary
        );
        assert!(
            done.file.is_none() && done.delta.is_none(),
            "nothing written"
        );
        reports.push(serde_json::to_string(&done.details).unwrap());
    }
    assert_eq!(reports[0], reports[1], "the same report twice");
    let report: AuditReport = serde_json::from_str(&reports[0]).unwrap();
    assert_eq!(
        (
            report.rule.extension.as_str(),
            report.rule.digest.as_str(),
            report.rule.rule.as_str()
        ),
        (manifest.id.as_str(), digest.as_str(), "adsiz")
    );
    assert!(
        report.drawing.len() == 16 && report.drawing.bytes().all(|b| b.is_ascii_hexdigit()),
        "{}",
        report.drawing
    );
    assert!(report.facts.starts_with("sha256:"), "{}", report.facts);
    assert!(
        !report.findings.is_empty(),
        "the parcels have no island numbers"
    );
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.message.ends_with("ada numarası yok"))
    );
}

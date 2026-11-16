use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kentos_contracts::DocumentSnapshotV2;
use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
use kentos_extensions::manifest::Module;

use super::*;

/// The rule of these tests, in place of the blueprint's: the parcels' overlaps and gaps KentOS measured
/// (`facts.project`), and every object without an island number (`Ada`) with a fix that writes one.
const RULE: &str = r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Parsellerin çakışması ve boşluğu, ada numarası olmayan nesneler."""
    bulgular = []
    parsel = facts.project.layer("parsel")
    for o in parsel.overlaps:
        bulgular.append(ext.AuditFinding(message=f"{o.area:.2f} m² çakışma", severity="error", feature_id=o.a))
    for g in parsel.gaps:
        bulgular.append(ext.AuditFinding(message=f"{g.area:.2f} m² boşluk", severity="warning", feature_id=g.around[0]))
    for nesne in facts.all_features():
        if not nesne.attrs.get("Ada"):
            bulgular.append(ext.AuditFinding(
                message=f"{nesne.layer}: ada numarası yok", severity="info", feature_id=nesne.id,
                fix=ext.SemanticFix(title="Ada yaz", command="cad.entities.set",
                                    args={"uids": [nesne.id], "operation": "attributes", "attrs": {"Ada": "0"}})))
    return bulgular
"#;

/// A packed audit extension: its bytes, content digest and manifest; `network` asks for the capability.
pub(super) fn package(network: bool) -> (Vec<u8>, String, kentos_extensions::Manifest) {
    package_with(RULE, network)
}

/// … whose rule `adsiz` is `rule` (its Python, from the decorator on).
pub(super) fn package_with(
    rule: &str,
    network: bool,
) -> (Vec<u8>, String, kentos_extensions::Manifest) {
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
    files.insert(logic, format!("{}{rule}", &text[..at]));
    if network {
        let mut manifest: Value = serde_json::from_str(&files["manifest.json"]).unwrap();
        manifest["requestedCapabilities"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "id": "network", "scope": "plugin-owned", "reason": "Harita servisine bağlanır.",
                          "hosts": ["harita.ornek.bel.tr"], "sendsDataToAi": false }));
        files.insert(
            "manifest.json".into(),
            serde_json::to_string_pretty(&manifest).unwrap(),
        );
    }
    let bytes: BTreeMap<String, Vec<u8>> = files
        .into_iter()
        .map(|(k, v)| (k, v.into_bytes()))
        .collect();
    let packed = kentos_extensions::pack(&bytes, &kentos_extensions::Limits::default()).unwrap();
    let opened = kentos_extensions::open(&packed.bytes, &kentos_extensions::Limits::default());
    let manifest = opened.inspection.manifest.expect("the manifest");
    (packed.bytes, packed.digest, manifest)
}

/// The facts' first fixture (fixtures/audit-facts/v1/denetim.kcad), a v2 file's snapshot.
fn snapshot() -> DocumentSnapshotV2 {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/audit-facts/v1/denetim.kcad");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// A job's folder: the drawing as the server writes it (`snapshot.kcad`), the package beside it.
pub(super) fn folder(name: &str, package: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-audit-job-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("snapshot.kcad"),
        kentos_kcad::encode(&snapshot()).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.join(PACKAGE), package).unwrap();
    dir
}

pub(super) fn request(
    digest: &str,
    manifest: &kentos_extensions::Manifest,
    python: &str,
) -> AuditRun {
    AuditRun {
        extension: manifest.id.clone(),
        version: manifest.version.to_string(),
        digest: digest.to_owned(),
        rule: "adsiz".into(),
        python: Some(python.into()),
        timeout_seconds: None,
    }
}

/// A rule in Rust that answers as the Python one would, and keeps what it was given.
#[derive(Default)]
pub(super) struct StandIn {
    pub(super) given: RefCell<Option<(Start, Value)>>,
}

impl RuleRunner for StandIn {
    fn run(
        &self,
        start: Start,
        invoke: Value,
        _: Duration,
        _: &dyn Fn() -> bool,
    ) -> Result<Value, String> {
        let overlap = invoke["facts"]["project"]["layers"][0]["overlaps"][0].clone();
        *self.given.borrow_mut() = Some((start, invoke));
        Ok(
            json!({ "ok": true, "result": { "kind": "findings", "findings": [
            { "severity": "error", "message": "çakışma", "featureId": overlap["a"] },
            { "severity": "bilinmeyen", "message": "okunmaz" }
        ] } }),
        )
    }
}

/// The job reads the package beside the request, measures the drawing with the desktop's code and sends the rule what
/// the desktop sends it, through the desktop's protocol; the report names the rule and the drawing. The rule's process
/// starts in the box (docs/adr/8057 §61): the package's folder readable, the job's scratch writable, the folder itself
/// (the drawing, the request) neither.
#[test]
fn a_job_runs_the_package_s_rule_on_the_measured_facts() {
    let (bytes, digest, manifest) = package(false);
    let dir = folder("stand-in", &bytes);
    let runner = StandIn::default();
    let boxed = Ok(crate::kutu::Plan::Whole("sınamanın kutusu".into()));
    let done = run_audit_in(
        &dir,
        request(&digest, &manifest, "/opt/kentos/python/bin/python"),
        &runner,
        boxed,
    )
    .unwrap();
    let (start, invoke) = runner.given.borrow_mut().take().unwrap();
    assert_eq!(start.python, crate::kutu::program().unwrap());
    assert_eq!(start.args[0], crate::kutu::ARG);
    assert!(
        !start.args.iter().any(|a| a == crate::kutu::NO_FILTER),
        "the whole box: Landlock and the filter"
    );
    let split = start.args.iter().position(|a| a == "--").unwrap();
    let given = |word: &str| -> Vec<PathBuf> {
        start.args[..split]
            .windows(2)
            .filter(|w| w[0] == word)
            .map(|w| PathBuf::from(&w[1]))
            .collect()
    };
    let source = PathBuf::from(start.hello["source"].as_str().unwrap());
    assert!(given("--oku").contains(&source), "{:?}", start.args);
    assert!(
        !given("--oku").iter().any(|p| dir.starts_with(p)),
        "{:?}",
        start.args
    );
    assert_eq!(
        given("--yaz")[..2],
        [dir.join(BYTECODE), dir.join(crate::kutu::SCRATCH)]
    );
    assert_eq!(
        start.args[split + 1..split + 3],
        ["/opt/kentos/python/bin/python", "-I"]
    );
    assert_eq!(start.hello["digest"], digest);
    assert!(
        source.join("manifest.json").is_file(),
        "the package's files in the folder"
    );
    let entry = &manifest.entrypoints.audits[0];
    assert_eq!(
        (invoke["kind"].as_str(), invoke["local"].as_str()),
        (Some("audit"), Some("adsiz"))
    );
    assert_eq!(invoke["handler"], json!(entry.handler));
    let doc = kentos_domain::Document::from_snapshot_v2(snapshot()).unwrap();
    let facts = audit_job::facts_of(&doc, &|| false).unwrap();
    assert_eq!(invoke["facts"], facts, "the desktop's facts");
    let report: AuditReport = serde_json::from_value(done.details).unwrap();
    assert_eq!(report.drawing, audit_job::drawing_stamp(&doc));
    assert_eq!(report.facts, digest_of(&facts));
    assert_eq!(
        (report.rule.digest.as_str(), report.rule.rule.as_str()),
        (digest.as_str(), "adsiz")
    );
    assert_eq!(report.findings.len(), 1, "an unknown severity is left out");
    assert_eq!(done.summary, format!("{}: 1 hata", entry.title));
    let _ = std::fs::remove_dir_all(&dir);
}

/// What keeps a rule from running is said before anything runs: another package, another rule, a rule that wants the
/// network, a server without the extension's Python.
#[test]
fn a_job_refuses_what_it_cannot_run() {
    let (bytes, digest, manifest) = package(false);
    let dir = folder("refusals", &bytes);
    let runner = StandIn::default();
    let code = |run: AuditRun| run_audit(&dir, run, &runner).unwrap_err();
    let mut other = request(
        &format!("sha256:{}", "0".repeat(64)),
        &manifest,
        "/opt/p/bin/python",
    );
    assert!(code(other.clone()).message.contains("içerik özeti"));
    other = request(&digest, &manifest, "/opt/p/bin/python");
    other.rule = "yok".into();
    assert!(code(other).message.contains("bir denetimi değil"));
    let mut bare = request(&digest, &manifest, "/opt/p/bin/python");
    bare.python = None;
    assert_eq!(code(bare).code, "tool_unavailable");
    let (bytes, digest, manifest) = package(true);
    std::fs::write(dir.join(PACKAGE), bytes).unwrap();
    let failure = code(request(&digest, &manifest, "/opt/p/bin/python"));
    assert_eq!(failure.code, "invalid");
    assert!(
        failure.message.contains("ağa çıkmaz"),
        "{}",
        failure.message
    );
    assert!(runner.given.borrow().is_none(), "no rule ran");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The interpreter the tests run real rules with: `KENTOS_TEST_PYTHON`, else the repository's (`.run/py`); none
/// when neither is there (the tests then say so and pass).
pub(super) fn python() -> Option<String> {
    let path = std::env::var_os("KENTOS_TEST_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/py/bin/python"));
    // Absolute, not resolved: an environment's interpreter is a link whose own place makes it the environment's.
    let path = std::path::absolute(path).ok().filter(|p| p.exists())?;
    Some(path.display().to_string())
}

/// The acceptance (plan/eklenti.md ADIM 20): the same drawing and rule give the same findings and provenance on the
/// desktop and in the job, byte for byte. The desktop's way: the drawing opened from its snapshot, the facts, the
/// rule's process through `kentos.ext_host` from its installed package, its answer read; the job's: the server's file
/// and the package beside the request. The real rule runs in Python.
#[test]
fn the_desktop_and_the_job_give_the_same_report() {
    let Some(python) = python() else {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        return;
    };
    let (bytes, digest, manifest) = package(false);
    let entry = manifest.entrypoints.audits[0].clone();
    // The desktop: its installed package's folder, its drawing.
    let desktop = folder("desktop", &bytes);
    let source = desktop.join("installed");
    let opened = kentos_extensions::open(&bytes, &kentos_extensions::Limits::default());
    for (path, data) in &opened.files {
        let to = source.join(path);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::write(to, data).unwrap();
    }
    let doc = kentos_domain::Document::from_snapshot_v2(snapshot()).unwrap();
    let facts = audit_job::facts_of(&doc, &|| false).unwrap();
    let mut invoke = audit_job::message(facts.clone());
    invoke["type"] = json!("invoke");
    invoke["local"] = json!(entry.id);
    invoke["handler"] = json!(entry.handler);
    invoke["document"] = json!(false);
    let start = Start {
        python: python.clone().into(),
        args: interpreter_args(&desktop.join("pyc")),
        hello: json!({ "type": "hello", "protocol": 1, "extension": manifest.id, "version": manifest.version.to_string(),
                       "digest": digest, "source": source.display().to_string(), "paths": ["src"] }),
    };
    let done = ExtHost::default()
        .run(start, invoke, Duration::from_secs(60), &|| false)
        .unwrap();
    let (findings, dropped) = findings_of(&done).unwrap();
    let on_desktop = AuditReport {
        rule: RuleRef {
            extension: manifest.id.clone(),
            version: manifest.version.to_string(),
            digest: digest.clone(),
            rule: entry.id.clone(),
            handler: entry.handler.clone(),
            title: entry.title.clone(),
        },
        drawing: audit_job::drawing_stamp(&doc),
        facts: digest_of(&facts),
        kentos: env!("CARGO_PKG_VERSION").to_owned(),
        findings,
        dropped,
        cut: facts["cut"] == true,
    };
    // The job, twice.
    let job = folder("job", &bytes);
    let mut reports = Vec::new();
    for _ in 0..2 {
        let done = run_audit(
            &job,
            request(&digest, &manifest, &python),
            &ExtHost::default(),
        )
        .unwrap();
        // Where this system's box is not whole, the job runs only under `report` and says what is open (docs/adr/8057
        // §61, §63).
        let open = crate::kutu::here()
            .ok()
            .and_then(|plan| plan.note())
            .unwrap_or_default();
        assert_eq!(
            done.summary,
            format!("{}: 1 hata, 1 uyarı, 9 bilgi{open}", entry.title)
        );
        reports.push(serde_json::to_string(&done.details).unwrap());
    }
    assert_eq!(reports[0], reports[1], "the job twice");
    // The rule's process was measured before it closed (docs/adr/8057 §58): its peak counts in the job's own usage.
    if Path::new("/proc/self/status").exists() {
        assert!(rule_peak_kb() > 0);
        eprintln!(
            "kuralın Python süreci en çok {:.1} MB",
            rule_peak_kb() as f64 / 1024.0
        );
    }
    assert_eq!(
        reports[0],
        serde_json::to_string(&serde_json::to_value(&on_desktop).unwrap()).unwrap(),
        "the desktop and the job"
    );
    let kept: AuditReport = serde_json::from_str(&reports[0]).unwrap();
    assert_eq!(
        kept.findings[0].message, "50.00 m² çakışma",
        "from the measured facts"
    );
    assert_eq!(
        kept.findings[2].fix.as_ref().map(|f| f.command.as_str()),
        Some("cad.entities.set")
    );
    for dir in [desktop, job] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// A rule that fails is the job's failure, in the rule's words with its traceback; the folder is the job's request.
#[test]
fn a_failing_rule_fails_the_job_with_its_traceback() {
    let Some(python) = python() else {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        return;
    };
    let (bytes, digest, manifest) = package(false);
    let dir = folder("request", &bytes);
    let mut run = request(&digest, &manifest, &python);
    std::fs::write(
        dir.join("request.json"),
        serde_json::to_vec(&kentos_contracts::processing_job::RunRequest::Audit(
            run.clone(),
        ))
        .unwrap(),
    )
    .unwrap();
    let done = crate::run_task(&dir).unwrap();
    assert!(done.summary.contains("1 hata"), "{}", done.summary);
    // The same package with a rule that raises.
    let (bytes, broken_digest) = {
        let opened = kentos_extensions::open(&bytes, &kentos_extensions::Limits::default());
        let mut files = opened.files;
        files.remove("integrity.json");
        let logic = files
            .keys()
            .find(|k| k.ends_with("logic.py"))
            .unwrap()
            .clone();
        let text = String::from_utf8(files[&logic].clone()).unwrap();
        files.insert(
            logic,
            text.replace("bulgular = []", "bulgular = {}['yok']")
                .into_bytes(),
        );
        let packed =
            kentos_extensions::pack(&files, &kentos_extensions::Limits::default()).unwrap();
        (packed.bytes, packed.digest)
    };
    std::fs::write(dir.join(PACKAGE), bytes).unwrap();
    run.digest = broken_digest;
    let failure = run_audit(&dir, run, &ExtHost::default()).unwrap_err();
    assert_eq!(failure.code, "tool_failed");
    assert!(
        failure.message.contains("KeyError: 'yok'"),
        "{}",
        failure.message
    );
    assert!(failure.message.contains("Traceback"), "{}", failure.message);
    let _ = std::fs::remove_dir_all(&dir);
}

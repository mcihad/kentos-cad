//! A release's Python environment prepared as the server prepares it (docs/adr/8057 §66), then a real `project.audit`
//! job run in it in the whole box. A tiny pure-Python wheel is made here and a lock names it with its hash, as an index
//! would; the package carries both. Preparing is offline, from a folder of wheels; the map the audit source reads names
//! the environment's Python; the rule imports the wheel's module in the box. A dependency with only a source
//! distribution is refused with its name and nothing is mapped; a package of another content is refused; preparing
//! again runs nothing; removing the release takes its environment and its entry. No network: uv is offline, the index
//! in the lock is never asked. Needs uv, a Python in KentOS's range (`KENTOS_TEST_PYTHON`, else `.run/py`) and KentOS's
//! library's wheel (`tools/` beside the built programs); without one it says so and passes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use kentos_application::processing_job::Processor;
use kentos_contracts::audit_job::AuditReport;
use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
use kentos_extensions::manifest::Module;

use super::audit::{audit_task_with, python};
use super::{budget, hooks, processor};
use crate::audit_env::{self, EnvSettings};

/// The rule: every object without an island number, said with the wheel's own words.
const RULE: &str = r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Ada numarası olmayan nesneler, tekerleğin sözüyle."""
    import kucuk
    return [ext.AuditFinding(message=f"{kucuk.ETIKET}: {n.layer}: ada numarası yok", severity="info", feature_id=n.id)
            for n in facts.all_features() if not n.attrs.get("Ada")]
"#;

/// Writes a pure-Python wheel by hand (`name-version-py3-none-any.whl`, its module saying `ETIKET`) into `dir`, with
/// the Python at hand; its path.
fn wheel(python: &str, dir: &Path, name: &str, version: &str) -> PathBuf {
    let script = r#"
import base64, hashlib, os, sys, zipfile
name, version, out = sys.argv[1], sys.argv[2], sys.argv[3]
dist = f"{name}-{version}.dist-info"
files = {
    f"{name}/__init__.py": f'ETIKET = "{name} {version}"\n',
    f"{dist}/METADATA": f"Metadata-Version: 2.1\nName: {name}\nVersion: {version}\n",
    f"{dist}/WHEEL": "Wheel-Version: 1.0\nGenerator: kentos-sinama\nRoot-Is-Purelib: true\nTag: py3-none-any\n",
}
digest = lambda b: "sha256=" + base64.urlsafe_b64encode(hashlib.sha256(b).digest()).rstrip(b"=").decode()
record = "".join(f"{p},{digest(c.encode())},{len(c.encode())}\n" for p, c in files.items()) + f"{dist}/RECORD,,\n"
path = os.path.join(out, f"{name}-{version}-py3-none-any.whl")
with zipfile.ZipFile(path, "w") as z:
    for p, c in files.items():
        z.writestr(p, c)
    z.writestr(f"{dist}/RECORD", record)
"#;
    std::fs::create_dir_all(dir).unwrap();
    let ran = Command::new(python)
        .args(["-I", "-c", script, name, version])
        .arg(dir)
        .status()
        .unwrap();
    assert!(ran.success(), "the wheel");
    dir.join(format!("{name}-{version}-py3-none-any.whl"))
}

fn sha256(path: &Path) -> String {
    kentos_python_env::sdk::sha256_file(path).unwrap()
}

/// The project (the blueprint's, its dependencies `needs`) and its lock: locked by uv from the folder of wheels, then
/// written as an index writes a lock (the index's address, each wheel's hash); `source_only`'s package keeps a source
/// distribution alone (`<name>-<version>.tar.gz`, its hash), no wheel.
fn project_and_lock(
    uv: &Path,
    work: &Path,
    project: &str,
    wheels: &Path,
    needs: &[(&str, &str)],
    source_only: Option<(&str, &str)>,
) -> (String, String) {
    let deps: Vec<String> = needs.iter().map(|(n, v)| format!("\"{n}=={v}\"")).collect();
    let pyproject = project
        .replace(
            "requires-python = \">=3.12,<3.14\"",
            "requires-python = \">=3.12,<3.15\"",
        )
        .replace(
            "dependencies = []",
            &format!("dependencies = [{}]", deps.join(", ")),
        );
    let dir = work.join(format!("kilit-{}", needs.len()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("pyproject.toml"), &pyproject).unwrap();
    let ran = Command::new(uv)
        .args([
            "lock",
            "--no-config",
            "--offline",
            "--no-index",
            "--find-links",
        ])
        .arg(wheels)
        .env("UV_CACHE_DIR", work.join("kilit-onbellegi"))
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "uv lock: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let mut lock = std::fs::read_to_string(dir.join("uv.lock")).unwrap();
    lock = lock.replace(
        &format!("source = {{ registry = \"{}\" }}", wheels.display()),
        "source = { registry = \"https://pypi.org/simple\" }",
    );
    for (name, version) in needs {
        let file = wheels.join(format!("{name}-{version}-py3-none-any.whl"));
        let local = format!("{{ path = \"{}\" }}", file.display());
        let indexed = match source_only {
            Some((n, _)) if n == *name => continue,
            _ => format!(
                "{{ url = \"https://files.pythonhosted.org/packages/00/00/{name}-{version}-py3-none-any.whl\", hash = \
                 \"sha256:{}\" }}",
                sha256(&file)
            ),
        };
        assert!(lock.contains(&local), "{lock}");
        lock = lock.replace(&local, &indexed);
    }
    if let Some((name, version)) = source_only {
        let file = wheels.join(format!("{name}-{version}-py3-none-any.whl"));
        // Put in after locking, so the lock was uv's from the wheel: the source distribution's bytes do not matter,
        // nothing is built from them.
        let sdist = wheels.join(format!("{name}-{version}.tar.gz"));
        std::fs::write(&sdist, b"bir kaynak paketi").unwrap();
        let wheels_line = format!("wheels = [\n    {{ path = \"{}\" }},\n]", file.display());
        assert!(lock.contains(&wheels_line), "{lock}");
        lock = lock.replace(
            &wheels_line,
            &format!(
                "sdist = {{ url = \"https://files.pythonhosted.org/packages/00/00/{name}-{version}.tar.gz\", hash = \
                 \"sha256:{}\" }}",
                sha256(&sdist)
            ),
        );
        // The wheel is not in the folder: the source distribution alone is.
        std::fs::remove_file(file).unwrap();
    }
    (pyproject, lock)
}

/// The package of `id`: the blueprint's with the rule, the project and its lock; its bytes, digest and manifest.
fn package(
    id: &str,
    pyproject: &str,
    lock: &str,
) -> (Vec<u8>, String, kentos_extensions::Manifest) {
    let mut files = blueprint(&Ask {
        id: id.into(),
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
    files.insert("pyproject.toml".into(), pyproject.to_owned());
    files.insert("uv.lock".into(), lock.to_owned());
    let bytes: BTreeMap<String, Vec<u8>> = files
        .into_iter()
        .map(|(k, v)| (k, v.into_bytes()))
        .collect();
    let packed = kentos_extensions::pack(&bytes, &kentos_extensions::Limits::default()).unwrap();
    let opened = kentos_extensions::open(&packed.bytes, &kentos_extensions::Limits::default());
    (
        packed.bytes,
        packed.digest,
        opened.inspection.manifest.expect("the manifest"),
    )
}

/// A work folder removed when the test ends, however it ends.
struct Gone(PathBuf);

impl Drop for Gone {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The folder beside the built programs that holds KentOS's library's wheel (`tools/`), and the work folder beside it
/// (on disk: an environment holds the library's debug build, hundreds of megabytes).
fn built_tools() -> Option<(PathBuf, PathBuf)> {
    let me = std::env::current_exe().ok()?;
    me.ancestors()
        .skip(1)
        .map(|dir| dir.join("tools"))
        .find(|tools| tools.join(kentos_python_env::sdk::DESCRIPTION).is_file())
        .map(|tools| {
            let work = tools
                .parent()
                .unwrap()
                .join(format!("denetim-ortami-sinama-{}", std::process::id()));
            (tools, work)
        })
}

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
    let (bytes, digest, manifest) = package("tr.ornek.denetim", &pyproject, &lock);
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
        package("tr.ornek.tasli", &pyproject, &lock);
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

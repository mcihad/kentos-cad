//! An audit release as the cloud runs it (docs/adr/8057 §66; docs/adr/8059 §13): a tiny pure-Python wheel made here,
//! a project and its lock that name it with its hash as an index would, and the package that carries both, with a
//! rule that imports the wheel's module. The job tests and the catalog's end-to-end test make their releases here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
use kentos_extensions::manifest::Module;

/// The rule: every object without an island number, said with the wheel's own words.
pub const RULE: &str = r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Ada numarası olmayan nesneler, tekerleğin sözüyle."""
    import kucuk
    return [ext.AuditFinding(message=f"{kucuk.ETIKET}: {n.layer}: ada numarası yok", severity="info", feature_id=n.id)
            for n in facts.all_features() if not n.attrs.get("Ada")]
"#;

/// The interpreter the tests run real rules with: `KENTOS_TEST_PYTHON`, else the repository's (`.run/py`).
pub fn python() -> Option<String> {
    let path = std::env::var_os("KENTOS_TEST_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/py/bin/python"));
    // Absolute, not resolved: an environment's interpreter is a link whose own place makes it the environment's.
    let path = std::path::absolute(path).ok().filter(|p| p.exists())?;
    Some(path.display().to_string())
}

/// Writes a pure-Python wheel by hand (`name-version-py3-none-any.whl`, its module saying `ETIKET`) into `dir`, with
/// the Python at hand; its path.
pub fn wheel(python: &str, dir: &Path, name: &str, version: &str) -> PathBuf {
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

pub fn sha256(path: &Path) -> String {
    kentos_python_env::sdk::sha256_file(path).unwrap()
}

/// The project (the blueprint's, its dependencies `needs`) and its lock: locked by uv from the folder of wheels, then
/// written as an index writes a lock (the index's address, each wheel's hash); `source_only`'s package keeps a source
/// distribution alone (`<name>-<version>.tar.gz`, its hash), no wheel.
pub fn project_and_lock(
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

/// The package of `id`: the blueprint's with the rule, the project and its lock, under `publisher` when one is given
/// (a catalog's publisher); its bytes, digest and manifest.
pub fn package(
    id: &str,
    publisher: Option<&str>,
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
    if let Some(publisher) = publisher {
        let mut manifest: serde_json::Value =
            serde_json::from_str(&files["manifest.json"]).unwrap();
        manifest["publisherId"] = serde_json::json!(publisher);
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
    (
        packed.bytes,
        packed.digest,
        opened.inspection.manifest.expect("the manifest"),
    )
}

/// A work folder removed when the test ends, however it ends.
pub struct Gone(pub PathBuf);

impl Drop for Gone {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The folder beside the built programs that holds KentOS's library's wheel (`tools/`), and the work folder beside it
/// (on disk: an environment holds the library's debug build, hundreds of megabytes).
pub fn built_tools() -> Option<(PathBuf, PathBuf)> {
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

//! The national rules (`catalog/standards/tr-mevzuat`, plan/eklenti.md ADIM 20) as the desktop and the cloud job run
//! them: the package packed from its folder as `kentos kek pack` packs it, each rule through `kentos.ext_host` in real
//! Python. A rule's compliant and violating cases are the facts KentOS measured on the synthetic two-storey building
//! (`fixtures/audit-facts/v1/mimari.json`, of `mimari.kcad`) with a few measures changed ([`paiy`], [`bykhy`]); the
//! rules also run on that drawing as a cloud job runs them (`run_audit`). Without a Python (`KENTOS_TEST_PYTHON` or
//! `.run/py`) the tests say so and pass.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use kentos_contracts::audit_job::{Finding, Severity};
use kentos_extensions::Limits;

use super::tests::python;
use super::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The package: its bytes, its content digest, its manifest and its files where an installed package's are.
struct Package {
    bytes: Vec<u8>,
    digest: String,
    manifest: kentos_extensions::Manifest,
    source: PathBuf,
}

/// A folder's files as `kentos kek pack` reads them (bytecode and environments left out).
fn read(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default();
            if name != "__pycache__" && name != ".venv" {
                read(root, &path, files);
            }
        } else {
            let relative = path.strip_prefix(root).unwrap().to_string_lossy();
            files.insert(relative.replace('\\', "/"), std::fs::read(&path).unwrap());
        }
    }
}

fn package() -> &'static Package {
    static PACKAGE: OnceLock<Package> = OnceLock::new();
    PACKAGE.get_or_init(|| {
        let folder = root().join("catalog/standards/tr-mevzuat");
        let mut files = BTreeMap::new();
        read(&folder, &folder, &mut files);
        let packed =
            kentos_extensions::pack(&files, &Limits::default()).expect("the package packs");
        let opened = kentos_extensions::open(&packed.bytes, &Limits::default());
        assert!(
            opened.inspection.problems.is_empty(),
            "{:?}",
            opened.inspection.problems
        );
        let manifest = opened.inspection.manifest.expect("its manifest");
        let source = std::env::temp_dir().join(format!("kentos-mevzuat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&source);
        for (path, data) in &opened.files {
            let to = source.join(path);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::write(to, data).unwrap();
        }
        Package {
            bytes: packed.bytes,
            digest: packed.digest,
            manifest,
            source,
        }
    })
}

/// The building's measured facts (`facts.project`), `edit` applied to their `architecture`, as a rule is given them.
fn facts(edit: impl FnOnce(&mut Value)) -> Value {
    let path = root().join("fixtures/audit-facts/v1/mimari.json");
    let mut project: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    edit(&mut project["architecture"]);
    json!({ "layers": [], "cut": false, "project": project })
}

/// A rule's findings on `facts`, run as the desktop's Eklenti denetimi runs it: the installed package's folder and
/// the rule's process through `kentos.ext_host`.
fn findings(python: &str, rule: &str, facts: Value) -> Vec<Finding> {
    let p = package();
    let entry = p
        .manifest
        .entrypoints
        .audits
        .iter()
        .find(|a| a.id == rule)
        .expect("the rule is the package's");
    let mut invoke = audit_job::message(facts);
    invoke["type"] = json!("invoke");
    invoke["local"] = json!(entry.id);
    invoke["handler"] = json!(entry.handler);
    invoke["document"] = json!(false);
    let pyc = std::env::temp_dir().join(format!("kentos-mevzuat-pyc-{}", std::process::id()));
    let start = Start {
        python: python.into(),
        args: interpreter_args(&pyc),
        hello: json!({ "type": "hello", "protocol": 1, "extension": p.manifest.id,
                       "version": p.manifest.version.to_string(), "digest": p.digest,
                       "source": p.source.display().to_string(), "paths": ["src"] }),
    };
    let done = ExtHost::default()
        .run(start, invoke, Duration::from_secs(60), &|| false)
        .expect("the rule answered");
    findings_of(&done).expect("its findings").0
}

fn rooms(a: &mut Value) -> &mut Vec<Value> {
    a["rooms"].as_array_mut().unwrap()
}

/// A room's facts by its number.
fn room<'a>(a: &'a mut Value, number: &str) -> &'a mut Value {
    rooms(a)
        .iter_mut()
        .find(|r| r["number"] == number)
        .expect("the room")
}

/// A room's facts by its name and storey (the common rooms have no number).
fn named<'a>(a: &'a mut Value, name: &str, level: &str) -> &'a mut Value {
    rooms(a)
        .iter_mut()
        .find(|r| r["name"] == name && r["level"] == level)
        .expect("the room")
}

/// A dwelling unit's door by its unit.
fn door<'a>(a: &'a mut Value, unit: &str) -> &'a mut Value {
    a["doors"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["unit"] == unit)
        .expect("the door")
}

/// The persistent id of a room or a door, in the unedited facts.
fn uid(of: impl FnOnce(&mut Value) -> &mut Value) -> String {
    let mut project = facts(|_| {});
    of(&mut project["project"]["architecture"])["uid"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Three more storeys over the two: a dwelling building of five, which BYKHY 48/1 no longer spares the distances.
fn five_storeys(a: &mut Value) {
    let levels = a["levels"].as_array_mut().unwrap();
    for (i, elevation) in [6.0, 9.0, 12.0].into_iter().enumerate() {
        levels.push(
            json!({ "id": format!("k{}", i + 2), "name": format!("{}. kat", i + 2),
                            "elevation": elevation, "height": 3.0 }),
        );
    }
}

/// The facts as a KentOS before the room's kind and narrow side, the sprinklers and the door into the stair gave them.
fn older_kentos(a: &mut Value) {
    for r in rooms(a) {
        let r = r.as_object_mut().unwrap();
        r.remove("kind");
        r.remove("narrow");
    }
    for d in a["doors"].as_array_mut().unwrap() {
        d.as_object_mut().unwrap().remove("intoStair");
    }
    a.as_object_mut().unwrap().remove("sprinklers");
}

/// The findings' severities and objects.
fn said(found: &[Finding]) -> Vec<(Severity, Option<&str>)> {
    found
        .iter()
        .map(|f| (f.severity, f.feature_id.as_deref()))
        .collect()
}

/// Whether the one finding's words hold every one of `words`.
fn says(found: &Finding, words: &[&str]) -> bool {
    words.iter().all(|w| found.message.contains(w))
}

macro_rules! need_python {
    () => {
        match python() {
            Some(p) => p,
            None => {
                eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
                return;
            }
        }
    };
}

mod ara_kat;
mod bykhy;
mod cikmaz;
mod paiy;

/// The cloud job's way: the drawing as the server writes it and the package beside the request; each rule gives its
/// report, of this package's release. The synthetic building has living rooms and bedrooms only: its two flats lack a
/// bathroom and a toilet (errors) and may have a kitchen niche (cannot decide); the first storey's corridor ends in a
/// dead end of 21.82 m (an error, as KentOS Mimari's own fire audit finds); every other rule finds nothing.
#[test]
fn the_rules_run_on_the_drawing_as_a_cloud_job_runs_them() {
    let python = need_python!();
    let p = package();
    let dir = std::env::temp_dir().join(format!("kentos-mevzuat-job-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let drawing = std::fs::read(root().join("fixtures/audit-facts/v1/mimari.kcad")).unwrap();
    std::fs::write(dir.join("snapshot.kcad"), drawing).unwrap();
    std::fs::write(dir.join(PACKAGE), &p.bytes).unwrap();
    let mut ran = Vec::new();
    for entry in &p.manifest.entrypoints.audits {
        let run = AuditRun {
            extension: p.manifest.id.clone(),
            version: p.manifest.version.to_string(),
            digest: p.digest.clone(),
            rule: entry.id.clone(),
            python: Some(python.clone()),
            timeout_seconds: None,
        };
        let done = run_audit(&dir, run, &ExtHost::default()).expect("the rule ran");
        let report: AuditReport = serde_json::from_value(done.details).unwrap();
        assert_eq!(
            (report.rule.rule.as_str(), report.rule.digest.as_str()),
            (entry.id.as_str(), p.digest.as_str())
        );
        let severities: Vec<Severity> = report.findings.iter().map(|f| f.severity).collect();
        use Severity::{Error, Info};
        let expected: &[Severity] = match entry.id.as_str() {
            "piyesler" => &[Info, Error, Error, Info, Error, Error],
            "cikmaz-koridor" => &[Error],
            _ => &[],
        };
        assert_eq!(severities, expected, "{}: {:?}", entry.id, report.findings);
        ran.push(entry.id.clone());
    }
    assert_eq!(
        ran,
        [
            "piyesler",
            "koridor",
            "cikmaz-koridor",
            "daire-ici",
            "ara-katli-daire",
            "daire-kapisi",
            "cikis-erisimi"
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

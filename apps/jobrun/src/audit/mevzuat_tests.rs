//! The national rules (`catalog/standards/tr-mevzuat`, plan/eklenti.md ADIM 20) as the desktop and the cloud job run
//! them: the package packed from its folder as `kentos kek pack` packs it, each rule through `kentos.ext_host` in real
//! Python. A rule's compliant and violating cases are the facts KentOS measured on the synthetic two-storey building
//! (`fixtures/audit-facts/v1/mimari.json`, of `mimari.kcad`) with one measure changed; the four rules also run on that
//! drawing as a cloud job runs them (`run_audit`). Without a Python (`KENTOS_TEST_PYTHON` or `.run/py`) the tests say
//! so and pass.

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

/// A room's facts by its number.
fn room<'a>(a: &'a mut Value, number: &str) -> &'a mut Value {
    a["rooms"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["number"] == number)
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

/// The persistent id of a room by its number, or of a unit's door, in the unedited facts.
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

/// The findings' severities and objects.
fn said(found: &[Finding]) -> Vec<(Severity, Option<&str>)> {
    found
        .iter()
        .map(|f| (f.severity, f.feature_id.as_deref()))
        .collect()
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

/// The cloud job's way: the drawing as the server writes it and the package beside the request; each of the four rules
/// gives its report, of this package's release, and the synthetic building complies with all of them.
#[test]
fn the_four_rules_run_on_the_drawing_as_a_cloud_job_runs_them() {
    let python = need_python!();
    let p = package();
    assert_eq!(p.manifest.entrypoints.audits.len(), 4);
    let dir = std::env::temp_dir().join(format!("kentos-mevzuat-job-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let drawing = std::fs::read(root().join("fixtures/audit-facts/v1/mimari.kcad")).unwrap();
    std::fs::write(dir.join("snapshot.kcad"), drawing).unwrap();
    std::fs::write(dir.join(PACKAGE), &p.bytes).unwrap();
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
        assert!(
            report.findings.is_empty(),
            "{}: {:?}",
            entry.id,
            report.findings
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// PAİY 29/1: a dwelling unit without a room of 12.0 m² net cannot have its living room, whichever room it is; the
/// largest room is the finding's object.
#[test]
fn a_dwelling_without_a_room_of_twelve_square_metres_has_no_living_room() {
    let python = need_python!();
    assert!(findings(&python, "oturma-odasi", facts(|_| {})).is_empty());
    let small = facts(|a| {
        room(a, "Z02")["net"] = json!(11.5);
        room(a, "Z03")["net"] = json!(10.0);
    });
    let found = findings(&python, "oturma-odasi", small);
    let salon = uid(|a| room(a, "Z02"));
    assert_eq!(said(&found), [(Severity::Error, Some(salon.as_str()))]);
    assert!(
        found[0].message.contains("PAİY md. 29/1"),
        "{}",
        found[0].message
    );
    assert!(
        found[0].message.contains("11.50 m²"),
        "{}",
        found[0].message
    );
}

/// BYKHY 48/2: inside a dwelling the door is 20 m away at most, 30 m with a sprinkler system; a building of four
/// storeys or fewer is spared (48/1); a room that cannot reach its unit's door is an error.
#[test]
fn inside_a_dwelling_its_door_is_twenty_metres_away_at_most() {
    let python = need_python!();
    let rule = "daire-ici";
    let far = |length: f64| {
        facts(move |a| {
            five_storeys(a);
            room(a, "K03")["escape"]["entrance"]["length"] = json!(length);
        })
    };
    assert!(
        findings(
            &python,
            rule,
            facts(|a| room(a, "K03")["escape"]["entrance"]["length"] = json!(24.0))
        )
        .is_empty(),
        "two storeys: 48/1 asks no distance"
    );
    assert!(findings(&python, rule, facts(five_storeys)).is_empty());
    let bedroom = uid(|a| room(a, "K03"));
    let warned = findings(&python, rule, far(24.0));
    assert_eq!(said(&warned), [(Severity::Warning, Some(bedroom.as_str()))]);
    assert!(
        warned[0].message.contains("BYKHY md. 48/2"),
        "{}",
        warned[0].message
    );
    assert!(warned[0].message.contains("yağmurlama sistemi yoksa"));
    let refused = findings(&python, rule, far(31.0));
    assert_eq!(said(&refused), [(Severity::Error, Some(bedroom.as_str()))]);
    let cut_off = facts(|a| {
        five_storeys(a);
        room(a, "K03")["escape"]["entrance"] = Value::Null;
    });
    let found = findings(&python, rule, cut_off);
    assert_eq!(said(&found), [(Severity::Error, Some(bedroom.as_str()))]);
    assert!(
        found[0].message.contains("yürüyüş yolu yok"),
        "{}",
        found[0].message
    );
}

/// BYKHY 48/4, 32/3, Ek-5/B: from a dwelling's door the nearest storey exit as the apartments' row: one way 15 m
/// (30 m with a sprinkler system), two ways 30 m (75 m).
#[test]
fn from_a_dwelling_s_door_the_exit_is_as_ek_5b_says_for_apartments() {
    let python = need_python!();
    let rule = "daire-kapisi";
    assert!(findings(&python, rule, facts(five_storeys)).is_empty());
    let one_exit = |length: f64| {
        facts(move |a| {
            five_storeys(a);
            let walk = &mut door(a, "d2")["exit"];
            walk["length"] = json!(length);
            walk["oneWay"] = json!(length);
        })
    };
    let entrance = uid(|a| door(a, "d2"));
    let warned = findings(&python, rule, one_exit(18.0));
    assert_eq!(
        said(&warned),
        [(Severity::Warning, Some(entrance.as_str()))]
    );
    assert!(
        warned[0].message.contains("tek yönde 18.00 m"),
        "{}",
        warned[0].message
    );
    assert!(warned[0].message.contains("BYKHY md. 32/3, 48/4 ve Ek-5/B"));
    let refused = findings(&python, rule, one_exit(32.0));
    assert_eq!(said(&refused), [(Severity::Error, Some(entrance.as_str()))]);
    // Two exits: a short shared part, a long whole walk.
    let two_ways = facts(|a| {
        five_storeys(a);
        let walk = &mut door(a, "d1")["exit"];
        walk["length"] = json!(40.0);
        walk["oneWay"] = json!(5.0);
    });
    let found = findings(&python, rule, two_ways);
    let first = uid(|a| door(a, "d1"));
    assert_eq!(said(&found), [(Severity::Warning, Some(first.as_str()))]);
    assert!(
        found[0].message.contains("iki yönde 40.00 m"),
        "{}",
        found[0].message
    );
}

/// BYKHY 31/3 and 33/6-b: every storey with rooms has a storey exit; every room reaches one through its own unit's
/// rooms and the common ones.
#[test]
fn every_storey_has_an_exit_and_every_room_reaches_one() {
    let python = need_python!();
    let rule = "cikis-erisimi";
    assert!(findings(&python, rule, facts(|_| {})).is_empty());
    let archive = uid(|a| room(a, "K04"));
    let cut_off = facts(|a| room(a, "K04")["escape"]["exit"] = Value::Null);
    let found = findings(&python, rule, cut_off);
    assert_eq!(said(&found), [(Severity::Error, Some(archive.as_str()))]);
    assert!(
        found[0].message.contains("BYKHY md. 33/6-b"),
        "{}",
        found[0].message
    );
    let no_exit = facts(|a| {
        for e in a["exits"].as_array_mut().unwrap() {
            if e["level"] == "k1" {
                e["count"] = json!(0);
            }
        }
    });
    let found = findings(&python, rule, no_exit);
    assert_eq!(
        said(&found),
        [(Severity::Error, None)],
        "one for the storey"
    );
    assert!(
        found[0].message.starts_with("1. kat: katta kat çıkışı yok"),
        "{}",
        found[0].message
    );
    assert!(found[0].message.contains("BYKHY md. 31/3"));
}

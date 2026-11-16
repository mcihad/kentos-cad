use std::collections::BTreeMap;

use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
use kentos_extensions::manifest::Module;

use super::*;

struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Temp {
        let dir = std::env::temp_dir().join(format!(
            "kentos-denetim-ortami-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Temp(dir)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn settings(dir: &Path, setting: &str) -> EnvSettings {
    EnvSettings {
        dir: dir.to_path_buf(),
        pythons: AuditPythons::parse(setting).unwrap(),
        ..EnvSettings::default()
    }
}

/// A packed extension with the blueprint's audit (else a command alone), asking for the network when told.
fn package(audit: bool, network: bool) -> (Vec<u8>, String) {
    let mut files = blueprint(&Ask {
        id: "tr.ornek.denetim".into(),
        name: "Denetim örneği".into(),
        summary: "Bir bulut denetim işinin kuralı.".into(),
        vertical: false,
        module: Module::Cbs,
        contributions: if audit {
            vec![Contribution::Audit]
        } else {
            vec![Contribution::Command]
        },
    })
    .unwrap();
    if network {
        let mut manifest: serde_json::Value =
            serde_json::from_str(&files["manifest.json"]).unwrap();
        manifest["requestedCapabilities"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "id": "network", "scope": "plugin-owned", "reason": "Harita servisine bağlanır.",
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
    (packed.bytes, packed.digest)
}

/// The map is the file the server writes over the setting's: the file wins for a version, the setting's others stay;
/// written whole, nothing left beside it; a version taken out of the file falls back to the setting's; a file that
/// cannot be read is said, and a job then reads the setting's alone.
#[test]
fn the_map_is_the_server_s_file_over_the_setting_written_whole() {
    let temp = Temp::new("esleme");
    let settings = settings(
        &temp.0,
        r#"{"a@1": "/eski/bin/python", "b@1": "/b/bin/python"}"#,
    );
    let map = settings.map();
    assert_eq!(map.get("a", "1").as_deref(), Some("/eski/bin/python"));
    map.set("a", "1", Path::new("/yeni/bin/python")).unwrap();
    map.set("c", "2", Path::new("/c/bin/python")).unwrap();
    assert_eq!(
        map.get("a", "1").as_deref(),
        Some("/yeni/bin/python"),
        "the file wins"
    );
    assert_eq!(
        map.get("b", "1").as_deref(),
        Some("/b/bin/python"),
        "the setting's others stay"
    );
    let names: Vec<String> = std::fs::read_dir(&temp.0)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, [MAP], "written whole, nothing beside it");
    let file = AuditPythons::parse(&std::fs::read_to_string(temp.0.join(MAP)).unwrap()).unwrap();
    assert_eq!(
        file.entries().count(),
        2,
        "the file holds the server's own alone"
    );
    assert!(map.unset("a", "1").unwrap());
    assert!(!map.unset("a", "1").unwrap());
    assert_eq!(
        map.get("a", "1").as_deref(),
        Some("/eski/bin/python"),
        "the setting's again"
    );
    std::fs::write(temp.0.join(MAP), "[bozuk").unwrap();
    assert!(map.current().unwrap_err().contains(MAP));
    assert_eq!(map.get("b", "1").as_deref(), Some("/b/bin/python"));
    assert_eq!(map.get("c", "2"), None);
}

/// A release is checked as the audit job checks it, before anything runs: its content digest, its id and version; one
/// asking for the network, one without an audit, are refused. Nothing is mapped, nothing is left taken out.
#[test]
fn a_release_is_checked_as_the_audit_job_checks_it_before_anything_runs() {
    let temp = Temp::new("denetim");
    let settings = settings(&temp.0, "{}");
    let (bytes, digest) = package(true, false);
    let other = format!("sha256:{}", "0".repeat(64));
    let refused = |extension: &str, version: &str, digest: Option<&str>, bytes: &[u8]| {
        prepare_release(&settings, extension, version, digest, bytes).unwrap_err()
    };
    assert!(refused("tr.ornek.denetim", "0.1.0", Some(&other), &bytes).contains("içerik özeti"));
    assert!(
        refused("tr.ornek.denetim", "0.2.0", Some(&digest), &bytes)
            .contains("istenen tr.ornek.denetim 0.2.0")
    );
    assert!(refused("tr.ornek.baska", "0.1.0", None, &bytes).contains("istenen tr.ornek.baska"));
    let (network, _) = package(true, true);
    assert!(refused("tr.ornek.denetim", "0.1.0", None, &network).contains("ağa çıkmaz"));
    let (no_audit, _) = package(false, false);
    assert!(refused("tr.ornek.denetim", "0.1.0", None, &no_audit).contains("denetimi yok"));
    assert!(refused("tr.ornek.denetim", "0.1.0", None, b"bir paket degil").contains("açılamadı"));
    assert!(!temp.0.join(MAP).exists(), "nothing mapped");
    assert!(!temp.0.join(OPENED).exists(), "nothing taken out");
}

/// `extension@version`, both sides; KentOS's Python's range read clause by clause.
#[test]
fn a_release_and_kentos_python_s_range_are_read_as_written() {
    assert_eq!(
        release_of("tr.ornek.denetim@1.0.0").unwrap(),
        ("tr.ornek.denetim".to_owned(), "1.0.0".to_owned())
    );
    for bad in ["tr.ornek.denetim", "@1.0.0", "tr.ornek.denetim@"] {
        assert!(release_of(bad).is_err(), "{bad}");
    }
    let range = ">=3.10,<3.15";
    for (version, holds) in [
        ("3.10.0", true),
        ("3.12.13", true),
        ("3.14.5", true),
        ("3.9.18", false),
        ("3.15.0", false),
        ("üç", false),
    ] {
        assert_eq!(python::in_range(version, range), holds, "{version}");
    }
    assert!(python::in_range("3.12.1", "==3.12.1"));
    assert!(!python::in_range("3.12.1", "!=3.12.1"));
}

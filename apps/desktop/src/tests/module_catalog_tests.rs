//! The modules' shared definition and their loose coupling (docs/adr/9195 §6, CLAUDE.md §25 rules 3–4):
//! `catalog/v1/modules.json`, what the desktop, the cloud portal and later the mobile and web clients read of each
//! module (its id, program, project types and works, storage, layer kinds and ribbon commands), written from
//! kentos-launch and the ribbon's catalog (`KENTOS_WRITE_MODULES=1` rewrites it); and the modules' dependencies kept to
//! their baseline (`scripts/dev/module_deps.py --check`). Test code only.

use std::path::PathBuf;

use kentos_launch::{Client, LayerKind, Module, Storage};
use serde_json::{Value, json};

use crate::catalog::catalog;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The catalog as it is written: every module, the layer kinds and the storage kinds, in their orders.
fn catalog_text() -> String {
    let modules: Vec<Value> = Module::ALL
        .into_iter()
        .map(|m| {
            json!({
                "id": m.id(),
                "title": m.title(),
                "program": m.program(),
                "note": m.note(),
                "projectType": m.project_type(),
                "cadWorks": m.cad_works(),
                "cloudTypes": m.project_types(),
                "storage": m.storage().id(),
                "layerKinds": m.layer_kinds().iter().map(|k| k.id()).collect::<Vec<_>>(),
                "commands": catalog().module_command_list(m),
            })
        })
        .collect();
    // Where each kind's content is kept and how a client writes on it (docs/adr/9197 §2.4).
    let kinds: Vec<Value> = LayerKind::ALL
        .into_iter()
        .map(|k| {
            json!({
                "id": k.id(),
                "title": k.title(),
                "kept": k.kept().id(),
                "writtenAs": k.written_as().map(|w| w.id()),
            })
        })
        .collect();
    // What each client writes, by the project's storage (docs/adr/9197 §2.1).
    let writes = |c: Client, s: Storage| c.writes(s).iter().map(|w| w.id()).collect::<Vec<_>>();
    let clients: Vec<Value> = Client::ALL
        .into_iter()
        .map(|c| {
            json!({
                "id": c.id(),
                "title": c.title(),
                "writes": {
                    "database": writes(c, Storage::Database),
                    "file": writes(c, Storage::File),
                },
            })
        })
        .collect();
    let storages: Vec<Value> = [Storage::Database, Storage::File]
        .into_iter()
        .map(|s| json!({ "id": s.id() }))
        .collect();
    let text = serde_json::to_string_pretty(&json!({
        "format": "kentos.modules",
        "version": 1,
        "note": "KentOS modüllerinin ortak tanımı (ADR 9195 §6, CLAUDE.md §25): kimlik, program, proje türü ve işleri, \
                 bulut türleri, saklama türü (database: CBS verisi veritabanında; file: CAD projesinin tamamı proje \
                 dosyasında, ADR 9196), katman türleri ve şeridin modüle verdiği komutlar. İstemciler saklama türüne \
                 göre ne yazdıklarını, katman türleri tutuldukları yeri ve yazılışlarını söyler (ADR 9197). Hiçbir şeridin tutmadığı \
                 komut her modülündür ve listelenmez. Üretici: KENTOS_WRITE_MODULES=1 cargo test -p kentos-desktop \
                 --lib module_catalog.",
        "storages": storages,
        "layerKinds": kinds,
        "clients": clients,
        "modules": modules,
    }))
    .expect("serializes");
    format!("{text}\n")
}

#[test]
fn the_module_catalog_is_current() {
    let path = root().join("catalog/v1/modules.json");
    let text = catalog_text();
    if std::env::var("KENTOS_WRITE_MODULES").as_deref() == Ok("1") {
        std::fs::write(&path, &text).expect("writes catalog/v1/modules.json");
        return;
    }
    assert!(
        std::fs::read_to_string(&path).unwrap_or_default() == text,
        "catalog/v1/modules.json is out of date: KENTOS_WRITE_MODULES=1 cargo test -p kentos-desktop --lib module_catalog"
    );
}

/// What the catalog says agrees with the program: each module's commands are the ones it offers, its storage is its
/// type's.
#[test]
fn the_module_catalog_says_what_each_module_offers() {
    let written: Value = serde_json::from_str(&catalog_text()).expect("reads");
    for (m, entry) in Module::ALL
        .into_iter()
        .zip(written["modules"].as_array().expect("modules"))
    {
        assert_eq!(entry["id"], m.id());
        let commands = entry["commands"].as_array().expect("commands");
        assert!(!commands.is_empty(), "{m:?}");
        for id in commands {
            assert!(
                catalog().module_has(m, id.as_str().expect("an id")),
                "{m:?} {id}"
            );
        }
        assert_eq!(
            entry["storage"] == "database",
            entry["projectType"] == "gis",
            "{m:?}"
        );
    }
    // A viewer writes nothing; the mobile application a CBS project's data rows alone (docs/adr/9197 §2.1).
    let clients = written["clients"].as_array().expect("clients");
    let of = |id: &str| clients.iter().find(|c| c["id"] == id).expect("a client").clone();
    assert_eq!(of("mobileViewer")["writes"], json!({ "database": [], "file": [] }));
    assert_eq!(of("mobileApp")["writes"], json!({ "database": ["dataRows"], "file": [] }));
    // A building's commands are not a CBS's, nor a CBS's edit session tools a building's.
    let ids = |m: Module| catalog().module_command_list(m);
    assert!(
        ids(Module::Architecture).contains(&"tool.wall")
            && !ids(Module::Gis).contains(&"tool.wall")
    );
}

#[test]
fn the_modules_keep_to_their_dependency_baseline() {
    let out = std::process::Command::new("python3")
        .arg("-I")
        .arg(root().join("scripts/dev/module_deps.py"))
        .arg("--check")
        .output()
        .expect("python3 runs");
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

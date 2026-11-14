//! ADIM 16's acceptance (plan/eklenti.md): the golden packages pack, inspect and repeat byte for byte; the negative
//! corpus is each a structural Turkish refusal, none a panic; the smallest Python-free package passes.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

use super::*;

/// A fixture folder's files, by their paths inside it.
fn folder(name: &str) -> BTreeMap<String, Vec<u8>> {
    files_under(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/extensions/v1")
            .join(name),
    )
}

/// Every file under `root` by its path from there.
fn files_under(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let root = root.to_path_buf();
    let mut files = BTreeMap::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/");
                files.insert(relative, std::fs::read(&path).unwrap());
            }
        }
    }
    files
}

/// The repository's demo extension (samples/eklenti-vitrini/eklenti, the owner's 14 November 2026: every demo with
/// it) packs without a warning and passes the package's check; its contributions are all there. A change that breaks
/// it breaks the demos, so it is held here.
#[test]
fn the_demo_extension_packs_and_passes_its_check() {
    let files = files_under(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../samples/eklenti-vitrini/eklenti"),
    );
    // A Python run of it leaves __pycache__ beside its code; the packer leaves it out.
    let files: BTreeMap<String, Vec<u8>> = files
        .into_iter()
        .filter(|(p, _)| !p.contains("__pycache__"))
        .collect();
    let packed = pack(&files, &limits()).unwrap_or_else(|p| panic!("{p:#?}"));
    assert_eq!(packed.warnings, Vec::new());
    let opened = open(&packed.bytes, &limits());
    assert!(
        opened.inspection.is_valid(),
        "{:#?}",
        opened.inspection.problems
    );
    let manifest = opened.inspection.manifest.expect("its manifest");
    assert_eq!(manifest.id, "tr.ornek.vitrin");
    let e = &manifest.entrypoints;
    assert_eq!(
        (
            e.commands.len(),
            e.processing.len(),
            e.expressions.len(),
            e.map_tools.len()
        ),
        (3, 1, 1, 1),
        "commands, an İşlemler tool, an expression function and a map tool"
    );
    let c = &manifest.contributions;
    assert_eq!(
        (
            c.ribbon_tabs.len(),
            c.context_menus.len(),
            c.data_sources.len()
        ),
        (1, 1, 1)
    );
}

fn limits() -> Limits {
    Limits::default()
}

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    kentos_zip::write(files).unwrap()
}

/// A package of these files with a fresh integrity list: what is left to judge is the content.
fn signed(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let list = Integrity::of(
        files
            .iter()
            .filter(|(p, _)| p.as_str() != integrity::FILE)
            .map(|(p, d)| (p.as_str(), d.as_slice())),
    );
    let json = list.to_json();
    let mut entries: Vec<(&str, &[u8])> = vec![(integrity::FILE, json.as_bytes())];
    entries.extend(
        files
            .iter()
            .filter(|(p, _)| p.as_str() != integrity::FILE)
            .map(|(p, d)| (p.as_str(), d.as_slice())),
    );
    zip_of(&entries)
}

fn codes(bytes: &[u8]) -> Vec<&'static str> {
    inspect(bytes, &limits())
        .problems
        .iter()
        .map(|p| p.code)
        .collect()
}

/// The MEBİS folder with its manifest changed.
fn mebis_with(edit: impl Fn(&mut Value)) -> BTreeMap<String, Vec<u8>> {
    let mut files = folder("mebis");
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    edit(&mut manifest);
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    files
}

#[track_caller]
fn refused_with(bytes: &[u8], code: &str) {
    let found = codes(bytes);
    assert!(found.contains(&code), "{code} not in {found:?}");
}

#[test]
fn the_golden_packages_pack_inspect_and_repeat_byte_for_byte() {
    for name in ["mebis", "ornek-altlik"] {
        let files = folder(name);
        let packed = pack(&files, &limits()).unwrap_or_else(|p| panic!("{name}: {p:#?}"));
        let again = pack(&files, &limits()).unwrap();
        assert_eq!(
            packed.bytes, again.bytes,
            "{name}: the same files make the same bytes"
        );
        assert!(packed.left_out.is_empty(), "{:?}", packed.left_out);
        let opened = open(&packed.bytes, &limits());
        let inspection = &opened.inspection;
        assert!(inspection.is_valid(), "{name}: {:#?}", inspection.problems);
        assert_eq!(inspection.digest.as_deref(), Some(packed.digest.as_str()));
        assert_eq!(inspection.files.len(), files.len());
        assert_eq!(opened.files.len(), files.len() + 1, "and integrity.json");
        assert_eq!(inspection.manifest.as_ref().unwrap().version, "1.0.0");
        let names = kentos_zip::list_every(&packed.bytes, &kentos_zip::Limits::default()).unwrap();
        assert_eq!(
            (names[0].name.as_str(), names[1].name.as_str()),
            ("manifest.json", "integrity.json")
        );
    }
    let mebis = pack(&folder("mebis"), &limits()).unwrap();
    assert_eq!(mebis.warnings, Vec::new());
    // Worked out apart from this crate (Python: the files' SHA-256, sizes and paths in path order, one line each), so
    // the canonical form is the documented one; fixtures/extensions/.gitattributes keeps the bytes on every checkout.
    assert_eq!(
        mebis.digest,
        "sha256:ec9730da3c4841b7ec3b5ff82103e90db52766eb7d3e3c3da47ebec40ac46501"
    );
    let altlik = pack(&folder("ornek-altlik"), &limits()).unwrap();
    let warned: Vec<&str> = altlik.warnings.iter().map(|w| w.code).collect();
    assert_eq!(
        warned,
        ["catalog_changelog"],
        "the smallest package passes with one warning"
    );
    assert_eq!(
        altlik.digest,
        "sha256:4bc65045c06ff1340f7147962d6176f1f71efcef2cc3649c23226ded02878441"
    );
}

#[test]
fn the_content_digest_does_not_depend_on_the_zip() {
    let packed = pack(&folder("mebis"), &limits()).unwrap();
    let opened = open(&packed.bytes, &limits());
    // The same files stored instead of deflated, in another order.
    let mut entries: Vec<(&str, &[u8])> = opened
        .files
        .iter()
        .map(|(p, d)| (p.as_str(), d.as_slice()))
        .collect();
    entries.reverse();
    let stored: Vec<(&str, &[u8], Pack)> = entries
        .iter()
        .map(|(p, d)| (*p, *d, Pack::Stored))
        .collect();
    let other = kentos_zip::write_with(&stored).unwrap();
    assert_ne!(other, packed.bytes);
    let inspection = inspect(&other, &limits());
    assert!(inspection.is_valid(), "{:#?}", inspection.problems);
    assert_eq!(inspection.digest, Some(packed.digest));
}

#[test]
fn the_packer_leaves_a_working_folders_files_out_and_says_which() {
    let clean = pack(&folder("mebis"), &limits()).unwrap();
    let mut files = folder("mebis");
    let working = [
        ".venv/lib/python3.12/site-packages/shapely/__init__.py",
        ".venv/pyvenv.cfg",
        "src/mebis/__pycache__/logic.cpython-312.pyc",
        ".git/config",
        ".env",
        "mcp.json",
        ".cursorrules",
        ".claude/settings.json",
        ".DS_Store",
        ".pytest_cache/v/cache/nodeids",
        "dist/tr.ornek.mebis-0.9.0.kek",
        "build/lib/mebis/logic.py",
        "integrity.json",
        "id_ed25519",
    ];
    for path in working {
        files.insert(path.to_owned(), b"OPENAI_API_KEY=sk-test".to_vec());
    }
    let packed = pack(&files, &limits()).unwrap();
    assert_eq!(
        packed.bytes, clean.bytes,
        "nothing of them is in the package"
    );
    let mut expected: Vec<&str> = working.to_vec();
    expected.sort_unstable();
    assert_eq!(packed.left_out, expected);
}

#[test]
fn the_packer_refuses_what_it_must_not_leave_out_silently() {
    let cases: [(&str, &[u8], &str); 6] = [
        ("lib/hizli.so", b"\x7fELF\x02\x01", "content_binary"),
        (
            "src/mebis/yardimci.py",
            b"\x7fELF\x02\x01\x01",
            "content_binary",
        ),
        ("data/eski.zip", b"PK\x03\x04", "content_archive"),
        ("docs/Readme.md", b"# A", "path_case"),
        ("docs/açıklama.md", b"# A", "path_unsafe"),
        ("scripts/kur.bat", b"@echo off", "content_binary"),
    ];
    for (path, data, code) in cases {
        let mut files = folder("mebis");
        files.insert(path.to_owned(), data.to_vec());
        let problems = pack(&files, &limits()).expect_err(path);
        assert!(
            problems.iter().any(|p| p.code == code),
            "{path}: {problems:#?}"
        );
    }
    let mut files = folder("mebis");
    files.remove("CHANGELOG.md");
    let problems = pack(&files, &limits()).unwrap_err();
    assert_eq!(
        problems.iter().map(|p| p.code).collect::<Vec<_>>(),
        ["reference_missing"]
    );
}

/// Patches the `index`th central directory record.
fn patch_central(bytes: &mut [u8], index: usize, edit: impl Fn(&mut [u8])) {
    let end = bytes.len() - 22;
    let mut at = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
    for _ in 0..index {
        let len = |o: usize| u16::from_le_bytes([bytes[at + o], bytes[at + o + 1]]) as usize;
        at += 46 + len(28) + len(30) + len(32);
    }
    edit(&mut bytes[at..at + 46]);
}

/// A Unix host wrote the `index`th entry with this mode.
fn with_mode(mut bytes: Vec<u8>, index: usize, mode: u32) -> Vec<u8> {
    patch_central(&mut bytes, index, |record| {
        record[4..6].copy_from_slice(&((3u16 << 8) | 20).to_le_bytes());
        record[38..42].copy_from_slice(&(mode << 16).to_le_bytes());
    });
    bytes
}

#[test]
fn the_archive_and_its_paths_are_refused_before_anything_is_unpacked() {
    let ok: &[u8] = b"print(1)\n";
    let deep = format!("{}a.py", "d/".repeat(12));
    let spaces = vec![b' '; 2 * 1024 * 1024];
    let cases: Vec<(Vec<u8>, &str)> = vec![
        (zip_of(&[("../evil.py", ok)]), "path_unsafe"),
        (zip_of(&[("/etc/cron.d/x", ok)]), "path_unsafe"),
        (zip_of(&[("src\\evil.py", ok)]), "path_unsafe"),
        (zip_of(&[("C:/Users/x.py", ok)]), "path_unsafe"),
        (zip_of(&[("src/mezarlık.py", ok)]), "path_unsafe"),
        (zip_of(&[("con.py", ok)]), "path_unsafe"),
        (zip_of(&[("a.py", ok), ("a.py", ok)]), "path_duplicate"),
        (zip_of(&[("Src/A.py", ok), ("src/a.py", ok)]), "path_case"),
        (zip_of(&[(deep.as_str(), ok)]), "path_depth"),
        (
            with_mode(zip_of(&[("manifest.json", b"/etc/passwd")]), 0, 0o120_777),
            "archive_link",
        ),
        (
            with_mode(zip_of(&[("fifo", b"")]), 0, 0o010_644),
            "archive_special",
        ),
        (zip_of(&[("klasor/", b"x")]), "archive_folder"),
        (
            zip_of(&[("bomba.json", spaces.as_slice())]),
            "archive_ratio",
        ),
        (zip_of(&[("__MACOSX/._a.py", ok)]), "content_junk"),
        (zip_of(&[(".venv/bin/python", ok)]), "content_environment"),
        (zip_of(&[("src/a.pyc", ok)]), "content_cache"),
        (zip_of(&[(".git/HEAD", ok)]), "content_vcs"),
        (zip_of(&[(".env", ok)]), "content_secret"),
        (zip_of(&[("keys/sunucu.pem", ok)]), "content_secret"),
        (zip_of(&[("mcp.json", ok)]), "content_ai_config"),
        (zip_of(&[(".cursor/rules.md", ok)]), "content_ai_config"),
        (zip_of(&[("wheels/x.whl", ok)]), "content_archive"),
        (zip_of(&[("lib/x.dll", ok)]), "content_binary"),
        (
            zip_of(&[("src/a.py", b"\x7fELF\x02\x01\x01\x00")]),
            "content_binary",
        ),
        (zip_of(&[("tools/a.json", b"MZ\x90\x00")]), ""),
        (b"bu bir paket degil".to_vec(), "archive_not_zip"),
        (
            [&b"MZ\x90\x00 program"[..], &zip_of(&[("a.py", ok)])].concat(),
            "archive_not_zip",
        ),
    ];
    for (bytes, code) in cases {
        let found = codes(&bytes);
        if code.is_empty() {
            assert!(
                !found.contains(&"content_binary"),
                "MZ alone is not a program: {found:?}"
            );
        } else {
            assert!(found.contains(&code), "{code} not in {found:?}");
        }
    }
    let mut encrypted = zip_of(&[("a.py", ok)]);
    patch_central(&mut encrypted, 0, |record| record[8] |= 1);
    refused_with(&encrypted, "archive_encrypted");
    let valid = pack(&folder("ornek-altlik"), &limits()).unwrap().bytes;
    refused_with(&valid[..valid.len() / 2], "archive_broken");
    let mut corrupt = valid.clone();
    let at = 30 + "manifest.json".len() + 5;
    corrupt[at] ^= 0xff;
    refused_with(&corrupt, "archive_broken");
    let small = Limits {
        archive: 100,
        ..limits()
    };
    assert_eq!(
        inspect(&valid, &small).problems[0].code,
        "archive_too_large"
    );
    let few = Limits {
        entries: 2,
        ..limits()
    };
    let problem = &inspect(&valid, &few).problems[0];
    assert_eq!(problem.code, "archive_broken");
    assert!(
        problem.message.contains("en çok 2 dosya"),
        "{}",
        problem.message
    );
}

#[test]
fn the_integrity_list_must_hold_for_every_file() {
    let files = folder("mebis");
    let packed = pack(&files, &limits()).unwrap();
    let opened = open(&packed.bytes, &limits()).files;
    let rezip = |files: &BTreeMap<String, Vec<u8>>| {
        zip_of(
            &files
                .iter()
                .map(|(p, d)| (p.as_str(), d.as_slice()))
                .collect::<Vec<_>>(),
        )
    };
    let mut extra = opened.clone();
    extra.insert("docs/eklenen.md".into(), b"# Sonradan".to_vec());
    assert_eq!(codes(&rezip(&extra)), ["integrity_unlisted"]);
    let mut changed = opened.clone();
    changed
        .get_mut("src/mebis/logic.py")
        .unwrap()
        .extend(b"\nimport os\nos.system('x')\n");
    assert_eq!(codes(&rezip(&changed)), ["integrity_mismatch"]);
    let mut missing = opened.clone();
    missing.remove("docs/README.md");
    assert_eq!(codes(&rezip(&missing)), ["integrity_missing"]);
    let mut unlisted = opened.clone();
    unlisted.remove("integrity.json");
    assert_eq!(codes(&rezip(&unlisted)), ["integrity_missing_file"]);
    let mut extended = opened.clone();
    let mut list: Value = serde_json::from_slice(&extended["integrity.json"]).unwrap();
    list["signature"] = json!("güvenilir");
    extended.insert("integrity.json".into(), serde_json::to_vec(&list).unwrap());
    assert_eq!(codes(&rezip(&extended)), ["integrity_json"]);
}

#[test]
fn the_manifest_and_everything_it_names_are_judged() {
    let cases: Vec<(BTreeMap<String, Vec<u8>>, &str)> = vec![
        (
            mebis_with(|m| m["format"] = json!("kentos.plugin")),
            "manifest_format",
        ),
        (
            mebis_with(|m| m["autoUpdate"] = json!(true)),
            "manifest_unknown_field",
        ),
        (mebis_with(|m| m["id"] = json!("MEBIS")), "manifest_id"),
        (
            mebis_with(|m| m["version"] = json!("1.0")),
            "manifest_version",
        ),
        (
            mebis_with(|m| m["kind"] = json!("utility")),
            "manifest_utility_tab",
        ),
        (
            mebis_with(|m| m["modules"][0] = json!("kadastro")),
            "manifest_bad_value",
        ),
        (
            mebis_with(|m| m["requestedCapabilities"][0]["id"] = json!("shell.exec")),
            "manifest_bad_value",
        ),
        (
            mebis_with(|m| m["requestedCapabilities"][0]["reason"] = json!("")),
            "manifest_capability_reason",
        ),
        (
            mebis_with(|m| {
                m["contributions"]["dataSources"][0]["url"] = json!("https://evil.example.org/wmts")
            }),
            "manifest_data_source_host",
        ),
        (
            mebis_with(|m| m["dependencies"]["extensions"][0]["id"] = json!("tr.ornek.mebis")),
            "manifest_dependency_cycle",
        ),
        (
            mebis_with(|m| {
                m["entrypoints"]["commands"][0]["handler"] = json!("mebis.kayip:init_schema")
            }),
            "handler_missing",
        ),
        (
            mebis_with(|m| m["changelogFile"] = json!("NOTLAR.md")),
            "reference_missing",
        ),
    ];
    for (files, code) in cases {
        refused_with(&signed(&files), code);
    }
    let mut no_manifest = folder("mebis");
    no_manifest.remove("manifest.json");
    assert_eq!(codes(&signed(&no_manifest)), ["manifest_missing"]);
}

#[test]
fn the_named_files_are_what_their_fields_say() {
    let change = |path: &str, edit: &dyn Fn(&str) -> String| {
        let mut files = folder("mebis");
        let text = String::from_utf8(files[path].clone()).unwrap();
        files.insert(path.to_owned(), edit(&text).into_bytes());
        signed(&files)
    };
    // A form whose body binds a field the package does not carry.
    let form = change("forms/defin_kaydi.kform", &|t| {
        t.replace("\"field\": \"merhum_adi\"", "\"field\": \"yok\"")
    });
    refused_with(&form, "form_invalid");
    let form = change("forms/defin_kaydi.kform", &|t| {
        t.replace("\"version\": 1", "\"version\": 7")
    });
    refused_with(&form, "form_invalid");
    // The form core's preflight: a rule that does not compile.
    let form = change("forms/defin_kaydi.kform", &|t| {
        t.replace(
            "\"field\": \"merhum_adi\", \"required\": true",
            "\"field\": \"merhum_adi\", \"requiredWhen\": \"mezar_id = \"",
        )
    });
    refused_with(&form, "form_preflight");
    let icon = change("assets/mebis.svg", &|t| {
        t.replace("<path d=\"M3 17h14\"/>", "<script>alert(1)</script>")
    });
    refused_with(&icon, "icon_unsafe");
    let icon = change("assets/mebis.svg", &|t| {
        t.replace(
            "<path d=\"M3 17h14\"/>",
            "<image href=\"https://x/y.png\"/>",
        )
    });
    refused_with(&icon, "icon_unsafe");
    let tool = change("tools/occupancy-by-island.tool.json", &|t| {
        t.replace("\"id\": \"occupancy-by-island\"", "\"id\": \"baska\"")
    });
    refused_with(&tool, "tool_descriptor");
    let panel = change("ui/search_dock.panel.json", &|t| t.replace('{', "["));
    refused_with(&panel, "reference_json");
    let invisible = change("src/mebis/logic.py", &|t| {
        t.replace("return findings", "return findings\u{200B}")
    });
    refused_with(&invisible, "text_invisible");
    let home = change("src/mebis/logic.py", &|t| {
        format!("{t}\nVERI = \"/home/ayse/mebis/veri.gpkg\"\n")
    });
    refused_with(&home, "text_user_path");
    let stale = change("pyproject.toml", &|t| {
        t.replace("shapely>=2.0", "shapely>=2.1")
    });
    refused_with(&stale, "python_lock_stale");
    let windows_dropped = change("pyproject.toml", &|t| {
        t.replacen(
            "    \"sys_platform == 'win32' and platform_machine == 'AMD64'\",\n",
            "",
            2,
        )
    });
    refused_with(&windows_dropped, "python_environments");
    let mut gpkg = mebis_with(|m| {
        m["contributions"]["dataSources"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "eski-kayitlar", "title": "Eski kayıtlar", "kind": "geopackage", "file": "data/eski.gpkg"}));
    });
    gpkg.insert("data/eski.gpkg".into(), b"bu bir veritabani degil".to_vec());
    refused_with(&signed(&gpkg), "geopackage_invalid");
    gpkg.insert(
        "data/eski.gpkg".into(),
        b"SQLite format 3\0........".to_vec(),
    );
    assert_eq!(
        codes(&signed(&gpkg)),
        Vec::<&str>::new(),
        "a GeoPackage by its header"
    );
}

#[test]
fn no_input_makes_the_inspector_panic() {
    let valid = pack(&folder("mebis"), &limits()).unwrap().bytes;
    let step = valid.len() / 150 + 1;
    for at in (0..valid.len()).step_by(step) {
        let _ = inspect(&valid[..at], &limits());
        let mut flipped = valid.clone();
        flipped[at] ^= 0x5a;
        let _ = inspect(&flipped, &limits());
        let mut zeroed = valid.clone();
        zeroed[at..(at + 8).min(valid.len())].fill(0xff);
        let _ = inspect(&zeroed, &limits());
    }
    for garbage in [&b""[..], b"PK\x03\x04", b"PK\x05\x06", &[0xff; 64]] {
        assert!(!inspect(garbage, &limits()).is_valid());
    }
}

#[test]
fn an_inspection_serialises_for_an_agent() {
    let mut files = folder("ornek-altlik");
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["requestedCapabilities"][0]["reason"] = json!("");
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let inspection = inspect(&signed(&files), &limits());
    let value = serde_json::to_value(&inspection).unwrap();
    assert_eq!(
        value["problems"][0],
        json!({
            "code": "manifest_capability_reason",
            "file": "manifest.json",
            "at": "/requestedCapabilities/0/reason",
            "message": "“dataSource” için gerekçe yok; kullanıcıya neden gerektiğini kısa bir Türkçe cümleyle yazın."
        })
    );
    assert_eq!(value["warnings"][0]["code"], "catalog_changelog");
    assert!(value["manifest"]["id"] == "tr.ornek.altlik");
}

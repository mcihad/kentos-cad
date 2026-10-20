//! The files `cad.profiles.import` writes (docs/adr/0558 §3): a profile or a mapping is written whole or not at all,
//! beside its place and read back, then moved there. The shared cases hold what the command says; these tests hold
//! the files themselves: what is written reads back as the profile that was given, a folder that cannot be made leaves
//! the file that was there, and no half-written file stays.

use std::path::{Path, PathBuf};

use kentos_commands::{CommandResult, DataProfile, ProfilesImport};
use kentos_domain::Document;
use kentos_native_application::ExecutionContext;
use kentos_native_application::profile_files::import;
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/profiles/v1/files")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).expect("fixture")).expect("fixture json")
}

fn doc() -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Profil",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false,
                     "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                     "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kentos-profile-files-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(input: ProfilesImport) -> CommandResult<kentos_commands::ProfileImported> {
    let mut d = doc();
    import::execute(&mut ExecutionContext::new(&mut d), input)
}

fn profile(name: &str) -> DataProfile {
    serde_json::from_value(fixture(name)).expect("profile")
}

fn leftovers(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|r| {
            r.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.contains("yaziliyor"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn an_imported_profile_reads_back_as_the_one_given() {
    let dir = folder("whole");
    let drawing = dir.join("cizim.kcad");
    let small = profile("peyzaj.profil.json");
    let CommandResult::Completed { output, .. } = run(ProfilesImport {
        profile: Some(small.clone()),
        drawing: Some(drawing.display().to_string()),
        ..ProfilesImport::default()
    }) else {
        panic!("not imported");
    };
    let written = dir.join("profiller").join("peyzaj.profil.json");
    assert_eq!(output.path, written.display().to_string());
    let bytes = std::fs::read(&written).unwrap();
    assert_eq!(output.bytes, bytes.len() as u64);
    assert!(bytes.ends_with(b"\n"), "a file ends in a newline");
    let back: DataProfile = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(&back).unwrap(),
        serde_json::to_value(&small).unwrap()
    );
    assert!(leftovers(&dir.join("profiller")).is_empty());
}

#[test]
fn a_profile_with_releases_keeps_its_migrations() {
    let dir = folder("releases");
    let drawing = dir.join("cizim.kcad");
    let kurum = profile("park-bahce.profil.json");
    assert_eq!(kurum.release, 3);
    let CommandResult::Completed { output, .. } = run(ProfilesImport {
        profile: Some(kurum.clone()),
        drawing: Some(drawing.display().to_string()),
        ..ProfilesImport::default()
    }) else {
        panic!("not imported");
    };
    assert_eq!((output.release, output.migrations), (3, 2));
    let back: DataProfile = serde_json::from_slice(
        &std::fs::read(dir.join("profiller/park-bahce.profil.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(back.release, 3);
    assert_eq!(back.migrations, kurum.migrations);
}

#[test]
fn a_replaced_file_is_replaced_whole() {
    let dir = folder("replace");
    let drawing = dir.join("cizim.kcad").display().to_string();
    let old = profile("park-bahce-r2.profil.json");
    let new = profile("park-bahce.profil.json");
    for p in [&old, &new] {
        let r = run(ProfilesImport {
            profile: Some(p.clone()),
            drawing: Some(drawing.clone()),
            overwrite: Some(true),
            ..ProfilesImport::default()
        });
        assert!(matches!(r, CommandResult::Completed { .. }));
    }
    let back: DataProfile = serde_json::from_slice(
        &std::fs::read(dir.join("profiller/park-bahce.profil.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(back.release, 3);
    assert!(leftovers(&dir.join("profiller")).is_empty());
}

#[test]
fn a_folder_that_cannot_be_made_leaves_the_file_that_was_there() {
    let dir = folder("unwritable");
    // `profiller` is a file: the folder cannot be made, and the file is as it was.
    let blocker = dir.join("profiller");
    std::fs::write(&blocker, b"bu bir dosya").unwrap();
    let r = run(ProfilesImport {
        profile: Some(profile("peyzaj.profil.json")),
        drawing: Some(dir.join("cizim.kcad").display().to_string()),
        ..ProfilesImport::default()
    });
    let CommandResult::Failed { error, .. } = r else {
        panic!("written into a file");
    };
    assert_eq!(error.code, "file_unwritable");
    assert_eq!(std::fs::read(&blocker).unwrap(), b"bu bir dosya");
}

#[test]
fn a_mapping_is_written_beside_the_profiles() {
    let dir = folder("mapping");
    let mapping: kentos_commands::ProfileMapping =
        serde_json::from_value(fixture("sivas-bel.eslem.json")).unwrap();
    let CommandResult::Completed { output, .. } = run(ProfilesImport {
        mapping: Some(mapping.clone()),
        drawing: Some(dir.join("cizim.kcad").display().to_string()),
        ..ProfilesImport::default()
    }) else {
        panic!("not imported");
    };
    assert_eq!(output.name, "sivas-bel");
    let back: kentos_commands::ProfileMapping =
        serde_json::from_slice(&std::fs::read(dir.join("profiller/sivas-bel.eslem.json")).unwrap())
            .unwrap();
    assert_eq!(back, mapping);
}

/// The park project with `n` trees at release 1 (the migration case's layers, their values spread over the codes).
fn park(n: usize) -> Document {
    let turs = ["Çınar", "Meşe", "Palmiye", "Akasya"];
    let sulamas = ["Damla", "Yağmurlama", "Yok"];
    let trees: Vec<Value> = (0..n)
        .map(|i| {
            let yas = if i % 50 == 0 { "on beş".to_owned() } else { (i % 200).to_string() };
            json!({
                "kind": "point", "id": i + 1, "layerId": "agac",
                "attrs": { "No": format!("A-{i}"), "Tur": turs[i % 4], "Yas": yas, "Sulama": sulamas[i % 3] },
                "p": { "x": (i % 1000) as f64, "y": (i / 1000) as f64 } })
        })
        .collect();
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Park",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "peyzaj", "name": "Peyzaj", "type": "group", "visible": true, "locked": false,
                     "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                     "children": [{ "id": "agac", "name": "Ağaçlar", "type": "layer", "visible": true, "locked": false,
                         "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                         "children": [], "fields": [
                             { "name": "No", "kind": "text" }, { "name": "Tur", "kind": "text" },
                             { "name": "Yas", "kind": "text" }, { "name": "Sulama", "kind": "text" },
                             { "name": "Alan", "kind": "text" }] }] }],
        "activeLayer": "agac",
        "entities": trees,
        "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

/// `cargo test --release -p kentos-native-application --test all profile_files::migrate_timing -- --ignored --nocapture`
/// (docs/adr/0558 Doğrulama): the plan and the write of the whole chain on 100 000 trees.
#[test]
#[ignore = "a timing, run on its own in release"]
fn migrate_timing() {
    use kentos_native_application::profile_files::migrate;
    let n: usize = std::env::var("KENTOS_PERF_OBJECTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100_000);
    let kurum = profile("park-bahce.profil.json");
    let input = || kentos_commands::ProfilesMigrate {
        name: None,
        profile: Some(kurum.clone()),
        from: 1,
        to: None,
        drawing: None,
        expected_revision: None,
    };
    let mut d = park(n);
    let t = std::time::Instant::now();
    let CommandResult::Completed { output, .. } = migrate::plan_in(&d, &input()) else {
        panic!("not planned");
    };
    let planned = t.elapsed();
    assert!(output.changed);
    let t = std::time::Instant::now();
    let CommandResult::Completed { output, .. } =
        migrate::execute(&mut ExecutionContext::new(&mut d), input())
    else {
        panic!("not migrated");
    };
    let written = t.elapsed();
    println!(
        "{n} nesne: plan {planned:?}, yazma {written:?}; {} nesneye yazıldı, {} uymayan değer grubu",
        output.objects,
        output.losses.len()
    );
    assert!(planned.as_millis() < 8000 && written.as_millis() < 8000);
}

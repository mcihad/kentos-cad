//! A published release's software bill of materials (docs/adr/8057 §91): read from the approved bytes by whoever may
//! install it, its lock's Python packages as CycloneDX; a package without Python is refused with the reason. On both
//! stores.

use super::*;

on_both_stores!(a_published_release_gives_its_bill_of_materials);

/// The example extension, Ayşe's publisher's, at `version`; with the example lock (numpy, shapely) when `python`.
fn release_with(version: &str, python: bool) -> Vec<u8> {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = repo.join("fixtures/extensions/v1/ornek-altlik");
    let mut files = BTreeMap::new();
    for name in ["LICENSE", "manifest.json"] {
        files.insert(name.to_owned(), std::fs::read(root.join(name)).unwrap());
    }
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["version"] = json!(version);
    manifest["publisherId"] = json!(PUBLISHER);
    if python {
        manifest["python"] = json!({ "projectFile": "pyproject.toml", "lockFile": "uv.lock" });
        for name in ["pyproject.toml", "uv.lock"] {
            let from = repo.join("samples/eklenti-vitrini/eklenti").join(name);
            files.insert(name.to_owned(), std::fs::read(from).unwrap());
        }
    }
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap()
        .bytes
}

async fn a_published_release_gives_its_bill_of_materials(bench: Bench) {
    // The server's wheel folder holds numpy's wheel: its licence goes into the list (docs/adr/8057 §99).
    let wheels =
        std::env::temp_dir().join(format!("kentos-katalog-tekerlek-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wheels);
    std::fs::create_dir_all(&wheels).unwrap();
    let metadata: &[u8] =
        b"Metadata-Version: 2.4\nName: numpy\nVersion: 2.5.4\nLicense-Expression: BSD-3-Clause\n\n";
    std::fs::write(
        wheels.join("numpy-2.5.4-cp312-cp312-win_amd64.whl"),
        kentos_zip::write(&[("numpy-2.5.4.dist-info/METADATA", metadata)]).unwrap(),
    )
    .unwrap();
    grants(&bench).await;
    let app = app_with(Some(
        Catalog::new(bench.store(), Accounts::Named).wheels(wheels.clone()),
    ));
    flow::reviewed_and_published(&app, "1.0.0", release_with("1.0.0", true)).await;
    let at = uri(PUBLIC, "1.0.0");
    let (status, bom) = get(&app, Via::Desktop, &format!("{at}/sbom"), "can").await;
    assert_eq!(status, StatusCode::OK, "{bom}");
    assert_eq!(bom["bomFormat"], "CycloneDX");
    assert_eq!(bom["metadata"]["component"]["name"], "tr.ornek.altlik");
    let purls: Vec<&str> = bom["components"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["purl"].as_str())
        .collect();
    assert!(
        purls.iter().any(|p| p.starts_with("pkg:pypi/numpy@"))
            && purls.iter().any(|p| p.starts_with("pkg:pypi/shapely@")),
        "{purls:?}"
    );
    let licence = |name: &str| {
        bom["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .map(|c| c["licenses"].clone())
            .unwrap()
    };
    assert_eq!(licence("numpy"), json!([{ "expression": "BSD-3-Clause" }]));
    assert_eq!(licence("shapely"), Value::Null, "its wheel is not there");
    let _ = std::fs::remove_dir_all(&wheels);
    // A release without Python: nothing to list, said so.
    flow::reviewed_and_published(&app, "1.0.1", release_with("1.0.1", false)).await;
    let (status, body) = get(
        &app,
        Via::Desktop,
        &format!("{}/sbom", uri(PUBLIC, "1.0.1")),
        "can",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["error"], "sbom_empty");
}

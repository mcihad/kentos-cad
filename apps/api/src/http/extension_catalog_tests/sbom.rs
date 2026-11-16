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
    let app = catalog(&bench).await;
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

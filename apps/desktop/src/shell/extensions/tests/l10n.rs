//! A package written in another language and translated to the interface's is reviewed and shown translated
//! (docs/adr/8057 §106): the review's name, summary and the reasons of the capabilities it asks for.

use super::*;

/// The example extension written in English, with a Turkish translation of its name, summary and a reason.
fn translated() -> Vec<u8> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = repo.join("fixtures/extensions/v1/ornek-altlik");
    let mut files = std::collections::BTreeMap::new();
    for name in ["LICENSE", "manifest.json"] {
        files.insert(name.to_owned(), std::fs::read(root.join(name)).unwrap());
    }
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["name"] = serde_json::json!("Basemap");
    manifest["summary"] = serde_json::json!("Adds the basemap.");
    manifest["l10n"] = serde_json::json!({ "default": "en", "files": { "tr": "l10n/tr.json" } });
    let first = manifest["requestedCapabilities"][0]["id"]
        .as_str()
        .expect("a capability")
        .to_owned();
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    files.insert(
        "l10n/tr.json".to_owned(),
        serde_json::to_vec(&serde_json::json!({
            "name": "Altlık",
            "summary": "Altlığı ekler.",
            format!("requestedCapabilities.{first}.reason"): "Türkçe gerekçe."
        }))
        .unwrap(),
    );
    kentos_extensions::pack(&files, &kentos_extensions::Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

#[test]
fn a_translated_package_is_reviewed_in_the_interfaces_language() {
    let root = temp("ceviri");
    let mut gis = app(&root, Some(Module::Gis));
    gis.open_extensions();
    let _ = gis.extensions_event(Event::Picked(Some(("altlik.kek".into(), translated()))));
    let review = &gis.extensions.review.as_ref().expect("reviewed").review;
    let manifest = review.inspection.manifest.as_ref().expect("its manifest");
    assert_eq!(
        (manifest.name.as_str(), manifest.summary.as_str()),
        ("Altlık", "Altlığı ekler.")
    );
    assert_eq!(review.capabilities[0].reason, "Türkçe gerekçe.");
    let _ = gis.extensions_view();
}

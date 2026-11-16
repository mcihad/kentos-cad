//! The secret gate's rule, run by the catalog when a version is sent (PY-CLOUD-05, D11): a key written in a file of the
//! package sends the version back to its author with the file and line, never the value; a clean package goes on. On
//! both stores.

use super::*;

on_both_stores!(a_key_written_in_the_package_sends_the_version_back);

/// A key-shaped value made here: the repository holds no key-shaped literal.
fn key() -> String {
    let letters = b"aB3dE5gH7jK9mN2pQ4sT6vW8yZ0cF1";
    let body: String = (0..40)
        .map(|i| letters[(i * 7 + i / 3 + 5) % letters.len()] as char)
        .collect();
    format!("{}{}{body}", "sk-ant-", "api03-")
}

/// The example extension with a README holding `readme`, at `version`.
fn with_readme(version: &str, readme: &str) -> Vec<u8> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/extensions/v1/ornek-altlik");
    let mut files = BTreeMap::new();
    for name in ["LICENSE", "manifest.json"] {
        files.insert(name.to_owned(), std::fs::read(root.join(name)).unwrap());
    }
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["version"] = json!(version);
    manifest["publisherId"] = json!(PUBLISHER);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    files.insert("README.md".to_owned(), readme.as_bytes().to_vec());
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap()
        .bytes
}

/// Uploaded by Ayşe, every gate clean, sent by her program: the answer to sending.
async fn sent(app: &Router, version: &str, readme: &str) -> Value {
    let (status, view) = upload(app, PUBLIC, "ayse", with_readme(version, readme)).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let at = uri(PUBLIC, version);
    scan_all(app, &at, &package_of(&view)).await;
    let (status, view) = act(app, Via::Python, "ayse", &at, json!({ "action": "submit" })).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

async fn a_key_written_in_the_package_sends_the_version_back(bench: Bench) {
    let app = catalog(&bench).await;
    bench
        .grant(Where::In(CatalogId::Public), "ayse", Role::Publisher)
        .await;
    bench.join("ayse", PUBLISHER).await;
    let readme = format!("# Altlık\n\nBağlantı için anahtar:\n\n    {}\n", key());
    let view = sent(&app, "1.0.0", &readme).await;
    assert_eq!(state_of(&view), "changesRequested", "{view}");
    let text = view.to_string();
    assert!(!text.contains(&key()), "the value is never said: {text}");
    let gates = view["release"]["gates"].as_array().expect("gates");
    let secrets = gates
        .iter()
        .find(|g| g["kind"] == "secrets")
        .expect("the secret gate");
    assert_eq!(secrets["status"], "blocked", "{secrets}");
    let finding = &secrets["findings"][0];
    assert_eq!(
        (&finding["code"], &finding["file"], &finding["at"]),
        (
            &json!("secret_found"),
            &json!("README.md"),
            &json!("satır 5")
        ),
        "{finding}"
    );
    assert!(
        finding["message"]
            .as_str()
            .is_some_and(|m| m.contains("Anthropic API anahtarı")),
        "{finding}"
    );
    // The key read from the user's setting instead: the version goes on to review.
    let view = sent(
        &app,
        "1.0.1",
        "# Altlık\n\nAnahtarı kurulumdan sonra Ayarlar'dan girin.\n",
    )
    .await;
    assert_eq!(state_of(&view), "submitted", "{view}");
}

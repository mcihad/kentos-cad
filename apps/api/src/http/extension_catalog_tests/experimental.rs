//! The package gate's rule for the SDK's experimental names (ADR 8061 §5), run by the catalog when a version is sent: a
//! package whose Python uses one goes back from the public catalog with the file and line; without it the version goes
//! on. The SDK's list is empty today: the catalog is given a test's. On both stores.

use kentos_extensions::sdk_surface::Experimental;

use super::*;

on_both_stores!(an_experimental_name_keeps_a_version_out_of_the_public_catalog);

const EXPERIMENTAL: &[Experimental] = &[Experimental {
    name: "deneysel_cizim",
    on: "CommandContext",
    note: "Yeri oturunca ctx.draw olur.",
}];

/// The example extension, Ayşe's publisher's, at `version`, with a Python helper holding `code`.
fn with_helper(version: &str, code: &str) -> Vec<u8> {
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
    files.insert("src/yardimci.py".to_owned(), code.as_bytes().to_vec());
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap()
        .bytes
}

/// Uploaded by Ayşe, every gate clean, sent by her program: the answer to sending.
async fn sent(app: &Router, version: &str, code: &str) -> Value {
    let (status, view) = upload(app, PUBLIC, "ayse", with_helper(version, code)).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let at = uri(PUBLIC, version);
    scan_all(app, &at, &package_of(&view)).await;
    let (status, view) = act(app, Via::Python, "ayse", &at, json!({ "action": "submit" })).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

async fn an_experimental_name_keeps_a_version_out_of_the_public_catalog(bench: Bench) {
    grants(&bench).await;
    let app = app_with(Some(
        Catalog::new(bench.store(), Accounts::Named).experimental(EXPERIMENTAL),
    ));
    bench
        .grant(Where::In(CatalogId::Public), "ayse", Role::Publisher)
        .await;
    bench.join("ayse", PUBLISHER).await;
    let view = sent(&app, "1.0.0", "def ciz(ctx):\n    ctx.deneysel_cizim()\n").await;
    assert_eq!(state_of(&view), "changesRequested", "{view}");
    let gates = view["release"]["gates"].as_array().expect("gates");
    let package = gates
        .iter()
        .find(|g| g["kind"] == "package")
        .expect("the package gate");
    assert_eq!(package["status"], "blocked", "{package}");
    let finding = &package["findings"][0];
    assert_eq!(
        (&finding["code"], &finding["file"], &finding["at"]),
        (
            &json!("sdk_experimental"),
            &json!("src/yardimci.py"),
            &json!("satır 2")
        ),
        "{finding}"
    );
    assert!(
        finding["message"]
            .as_str()
            .is_some_and(|m| m.contains("“deneysel_cizim” (CommandContext) deneysel")),
        "{finding}"
    );
    // The stable name instead: the version goes on to review.
    let view = sent(&app, "1.0.1", "def ciz(ctx):\n    ctx.draw()\n").await;
    assert_eq!(state_of(&view), "submitted", "{view}");
}

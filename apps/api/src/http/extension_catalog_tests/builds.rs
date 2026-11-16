//! The dependency gate's rule for builds, run by the catalog when a version is sent (docs/adr/8057 §94; plan/eklenti.md
//! D3, PY-UV-21): a package its lock builds from its source on a computer the release says it runs on sends it back
//! from the public catalog with the package and the computers; the author may send it again with a note and a reviewer
//! lift the finding with a reason. A person's catalog publishes it as it is. On both stores.

use super::*;

on_both_stores!(a_build_from_source_needs_a_reviewer_s_reason_in_the_public_catalog);

/// The example extension, Ayşe's publisher's, at `version`, with the demo extension's project and its lock passed
/// through `lock`.
fn with_lock(version: &str, lock: impl Fn(&str) -> String) -> Vec<u8> {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = repo.join("fixtures/extensions/v1/ornek-altlik");
    let mut files = BTreeMap::new();
    for name in ["LICENSE", "manifest.json"] {
        files.insert(name.to_owned(), std::fs::read(root.join(name)).unwrap());
    }
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["version"] = json!(version);
    manifest["publisherId"] = json!(PUBLISHER);
    manifest["python"] = json!({ "projectFile": "pyproject.toml", "lockFile": "uv.lock" });
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    let project = repo.join("samples/eklenti-vitrini/eklenti");
    files.insert(
        "pyproject.toml".to_owned(),
        std::fs::read(project.join("pyproject.toml")).unwrap(),
    );
    let text = std::fs::read_to_string(project.join("uv.lock")).unwrap();
    files.insert("uv.lock".to_owned(), lock(&text).into_bytes());
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap()
        .bytes
}

/// The lock without its macOS wheels: numpy and shapely are built there from their sources.
fn without_mac(lock: &str) -> String {
    lock.lines()
        .filter(|l| !l.contains("macosx"))
        .map(|l| format!("{l}\n"))
        .collect()
}

/// Uploaded to `base` by `account`, every gate clean, sent by its program: the answer to sending.
async fn sent(app: &Router, base: &str, account: &str, version: &str, bytes: Vec<u8>) -> Value {
    let (status, view) = upload(app, base, account, bytes).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let at = uri(base, version);
    scan_all(app, &at, &package_of(&view)).await;
    let (status, view) = act(
        app,
        Via::Python,
        account,
        &at,
        json!({ "action": "submit" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

fn gate<'v>(view: &'v Value, kind: &str) -> &'v Value {
    view["release"]["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .find(|g| g["kind"] == kind)
        .expect("the gate")
}

async fn a_build_from_source_needs_a_reviewer_s_reason_in_the_public_catalog(bench: Bench) {
    let app = catalog(&bench).await;
    let view = sent(
        &app,
        PUBLIC,
        "ayse",
        "1.0.0",
        with_lock("1.0.0", without_mac),
    )
    .await;
    assert_eq!(state_of(&view), "changesRequested", "{view}");
    let dependencies = gate(&view, "dependencies");
    assert_eq!(dependencies["status"], "blocked", "{dependencies}");
    let findings = dependencies["findings"].as_array().unwrap();
    let named: Vec<&str> = findings
        .iter()
        .map(|f| f["dependency"].as_str().unwrap())
        .collect();
    assert_eq!(named, ["numpy", "shapely"], "{findings:?}");
    let numpy = &findings[0];
    assert_eq!(
        (&numpy["code"], &numpy["file"]),
        (&json!("python_source_build"), &json!("uv.lock")),
        "{numpy}"
    );
    assert!(
        numpy["message"]
            .as_str()
            .is_some_and(|m| m.contains("macOS (aarch64), Python 3.12 için hazır kurulum dosyası (wheel) yok")),
        "{numpy}"
    );
    // The author sends it as it is with a note; a reviewer lifts the finding with a reason.
    let at = uri(PUBLIC, "1.0.0");
    let note = "macOS'ta derleme bilerek: kurumun Mac'lerinde derleyici kurulu.";
    let (status, view) = act(
        &app,
        Via::Python,
        "ayse",
        &at,
        json!({ "action": "submit", "note": note }),
    )
    .await;
    assert_eq!(
        (status, state_of(&view)),
        (StatusCode::OK, "submitted"),
        "{view}"
    );
    act(
        &app,
        Via::Web,
        "selin",
        &at,
        json!({ "action": "startReview" }),
    )
    .await;
    let waive = json!({ "action": "waive", "gate": "dependencies", "reason": note });
    let (status, view) = act(&app, Via::Web, "selin", &at, waive).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let dependencies = gate(&view, "dependencies");
    assert_eq!(dependencies["status"], "verified", "{dependencies}");
    assert_eq!(dependencies["waiver"]["reason"], note, "{dependencies}");
    // The lock with its wheels goes on to review.
    let view = sent(
        &app,
        PUBLIC,
        "ayse",
        "1.0.1",
        with_lock("1.0.1", str::to_owned),
    )
    .await;
    assert_eq!(state_of(&view), "submitted", "{view}");
    // A person's own catalog publishes it as it is: each desktop's install review says the build.
    let mine = personal("can");
    let view = sent(&app, &mine, "can", "1.0.0", with_lock("1.0.0", without_mac)).await;
    assert_eq!(state_of(&view), "submitted", "{view}");
}

//! The install review says what the package's Python lock means on this computer (docs/adr/8057 §94): the packages
//! built here from their sources, with KentOS's Python; the lint's sentences about every computer the package names
//! stay with its author.

use kentos_extensions::compat::Os;
use kentos_extensions::python_builds::{Need, Target};

use super::*;

/// The example extension with the demo extension's project and its lock passed through `lock`.
fn with_lock(lock: impl Fn(&str) -> String) -> Vec<u8> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = repo.join("fixtures/extensions/v1/ornek-altlik");
    let mut files = std::collections::BTreeMap::new();
    for name in ["LICENSE", "manifest.json"] {
        files.insert(name.to_owned(), std::fs::read(root.join(name)).unwrap());
    }
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["python"] =
        serde_json::json!({ "projectFile": "pyproject.toml", "lockFile": "uv.lock" });
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
    kentos_extensions::pack(&files, &kentos_extensions::Limits::default())
        .unwrap()
        .bytes
}

#[test]
fn the_review_says_what_is_built_on_this_computer() {
    let Some(here) = Target::here(None) else {
        return; // A computer KentOS does not run on.
    };
    let own = match here.os {
        Os::Linux => "manylinux",
        Os::Windows => "win_",
        Os::Macos => "macosx",
    };
    let root = temp("derleme");
    let mut gis = app(&root, Some(Module::Gis));
    gis.open_extensions();
    // This system's wheels gone from the lock.
    let without = with_lock(|lock| {
        lock.lines()
            .filter(|l| !l.contains(own))
            .map(|l| format!("{l}\n"))
            .collect()
    });
    let _ = gis.extensions_event(Event::Picked(Some(("altlik.kek".into(), without))));
    let on = gis
        .extensions
        .review
        .as_ref()
        .and_then(|p| p.here.clone())
        .expect("this computer judged");
    assert_eq!(
        on.target, here,
        "KentOS's Python while its own is not ready"
    );
    if on.supported {
        let built: Vec<&str> = on
            .unwheeled
            .iter()
            .filter(|u| u.needs == [(here, Need::Source)])
            .map(|u| u.name.as_str())
            .collect();
        assert_eq!(built, ["numpy", "shapely"], "{:?}", on.unwheeled);
    } else {
        assert!(
            on.unwheeled.is_empty(),
            "nothing judged where the lock does not reach"
        );
    }
    // The lint's sentences, about every computer, are not repeated under Uyarılar; the window draws.
    let codes: Vec<&str> = gis
        .extensions
        .review
        .as_ref()
        .unwrap()
        .review
        .inspection
        .warnings
        .iter()
        .map(|w| w.code)
        .collect();
    assert!(
        !on.supported || codes.contains(&"python_source_build"),
        "{codes:?}"
    );
    let _ = gis.extension_review_window();
    let _ = gis.extensions_event(Event::CancelReview);
    // The demo extension's own lock: nothing built here.
    let _ = gis.extensions_event(Event::Picked(Some((
        "altlik.kek".into(),
        with_lock(str::to_owned),
    ))));
    let on = gis
        .extensions
        .review
        .as_ref()
        .unwrap()
        .here
        .clone()
        .unwrap();
    assert!(on.unwheeled.is_empty(), "{:?}", on.unwheeled);
    let _ = std::fs::remove_dir_all(&root);
}

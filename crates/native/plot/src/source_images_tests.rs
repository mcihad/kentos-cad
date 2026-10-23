//! Exact bytes, deletion/replacement, canonical aliases and bounded memory are observed independently of rendering.
use super::*;
use std::path::Path;
use serde_json::{json, Value};
fn source(root: &Path, image: bool, aliases: bool, second: bool) -> Document {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/sheet/v1/source-snapshot.json"
    ))
    .unwrap();
    let mut model = fx["model"].clone();
    if image {
        let mut paths = vec![root.join("logo.png").display().to_string()];
        if aliases {
            paths.push("logo.png".into());
        }
        if second {
            paths.push(root.join("other/logo.png").display().to_string());
        }
        for (n, file) in paths.into_iter().enumerate() {
            model["entities"].as_array_mut().unwrap().push(json!({"kind":"image","id":20+n,"layerId":"cizim","attrs":{},"file":file,"p":{"x":0,"y":0},"width":0.2,"height":0.2,"rotation":0}));
        }
    }
    Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap()
}
#[test]
fn sheet_snapshot_image_versions_and_canonical_byte_cache_follow_the_independent_policy() {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/sheet/v1/source-snapshot.json"
    ))
    .unwrap();
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    for (n, c) in fx["cases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, c)| c["image"] == true || c["missing"] == true)
    {
        let root =
            std::env::temp_dir().join(format!("kentos-snapshot-images-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("other")).unwrap();
        let a = fs::read(corpus.join(fx["imageA"].as_str().unwrap())).unwrap();
        let b = fs::read(corpus.join(fx["imageB"].as_str().unwrap())).unwrap();
        let file = root.join("logo.png");
        if c["missing"] != true {
            fs::write(&file, &a).unwrap();
        }
        if c["second"] == true {
            fs::write(root.join("other/logo.png"), &b).unwrap();
        }
        let doc = source(&root, true, c["aliases"] == true, c["second"] == true);
        let origin = root.join("drawing.kcad").display().to_string();
        let cache = SourceImages::capture(&doc, Some(&origin)).unwrap();
        let hash = cache.fingerprint().unwrap();
        if matches!(c["change"].as_str(), Some("after" | "delete-after")) {
            assert_eq!(
                cache.read(file.to_str().unwrap()).unwrap().as_slice(),
                a.as_slice()
            );
        }
        match c["change"].as_str() {
            Some("before" | "after" | "appear") => fs::write(&file, &b).unwrap(),
            Some("delete-before" | "delete-after") => fs::remove_file(&file).unwrap(),
            _ => {}
        }
        let read = cache.read(file.to_str().unwrap());
        if c["expectError"] == true {
            assert!(read.is_err(), "{}", c["name"]);
        } else {
            let read = read.unwrap();
            assert_eq!(
                sha256_hex(&read),
                c["expectImageSha"]
                    .as_str()
                    .unwrap_or_else(|| fx["shaA"].as_str().unwrap()),
                "{}",
                c["name"]
            );
            if c["aliases"] == true {
                assert!(Arc::ptr_eq(&read, &cache.read("logo.png").unwrap()));
            }
            if c["second"] == true {
                cache
                    .read(root.join("other/logo.png").to_str().unwrap())
                    .unwrap();
            }
            if let Some(count) = c["expectImages"].as_u64() {
                assert_eq!(cache.versions().unwrap().len() as u64, count);
            }
        }
        assert_eq!(
            cache.fingerprint().unwrap(),
            hash,
            "a loaded cache keeps its original source fingerprint"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

//! A prepared source changes or is cancelled before exposure: no partial child package survives.
use super::*;
use serde_json::{Value, json};
fn prepared(name: &str) -> (std::path::PathBuf, Prepared) {
    let root = std::env::temp_dir().join(format!(
        "kentos-sheet-package-unit-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("work")).unwrap();
    let image = root.join("work/logo.png");
    fs::write(
        &image,
        include_bytes!("../../../../../fixtures/interaction/v1/logo.png"),
    )
    .unwrap();
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/delivery.json"
    ))
    .unwrap();
    let mut model = fx["model"].clone();
    model["entities"].as_array_mut().unwrap().push(json!({"kind":"image","id":100,"layerId":"resources","attrs":{},"file":image,"p":{"x":1000,"y":1000},"width":1,"height":1,"rotation":0}));
    let mut doc = Document::from_snapshot(serde_json::from_value(model).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let input:PublishSheets=serde_json::from_value(json!({"path":root.join("delivery"),"drawing":root.join("work/source.kcad"),"resources":true})).unwrap();
    let p = Prepared::plan(
        &doc,
        &input,
        &book,
        &book.sheets.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
    )
    .unwrap();
    fs::create_dir(root.join("stage")).unwrap();
    (root, p)
}
#[test]
fn sheet_package_changed_source_does_not_expose_a_partial_child() {
    let (root, p) = prepared("changed");
    fs::write(root.join("work/logo.png"), b"changed after planning").unwrap();
    let error = p.write(&root.join("stage"), None).unwrap_err();
    assert_eq!(error.code, "file_unwritable");
    assert!(!root.join("stage/kaynaklar").exists());
    assert_eq!(fs::read_dir(root.join("stage")).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn sheet_package_cancelled_copy_cleans_child_and_shares_the_stop_flag() {
    let (root, p) = prepared("cancelled");
    let progress = Progress::default();
    let child = progress.span(
        3,
        p.output.files.len() as u64,
        3 + p.output.files.len() as u64,
    );
    child.stop();
    assert!(progress.stopped());
    let error = p.write(&root.join("stage"), Some(&child)).unwrap_err();
    assert_eq!(error.code, "cancelled");
    assert_eq!(fs::read_dir(root.join("stage")).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn sheet_package_unprinted_svg_dependencies_are_not_silently_left_out() {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/delivery.json"
    ))
    .unwrap();
    let svg=b"<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'><image href='external.png'/></svg>";
    for library in [false, true] {
        let root = std::env::temp_dir().join(format!(
            "kentos-package-unused-svg-{}-{library}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut doc =
            Document::from_snapshot(serde_json::from_value(fx["model"].clone()).unwrap()).unwrap();
        let mut book: SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
        if library {
            let mut styles = doc.styles().clone();
            styles
                .items
                .push(json!({"id":"unused","svg":std::str::from_utf8(svg).unwrap()}));
            doc.set_styles(styles);
        } else {
            book.assets.push(kentos_sheet::model::AssetMeta {
                sha256: sha256_hex(svg),
                name: "Kullanılmayan SVG".into(),
                kind: kentos_sheet::model::AssetKind::Svg,
                width: 10,
                height: 10,
                bytes: svg.len() as u32,
                dpi: None,
            });
        }
        doc.set_sheets(
            kentos_sheet::project::project_sheets(&book, |_| Some(svg.to_vec())).unwrap(),
        )
        .unwrap();
        let input: PublishSheets =
            serde_json::from_value(json!({"path":root.join("delivery"),"resources":true})).unwrap();
        let p = Prepared::plan(&doc, &input, &book, &["ground".into()]).unwrap();
        assert!(
            p.issues
                .iter()
                .any(|i| i.code == "package_incomplete" && i.error && i.message.contains("SVG")),
            "{:?}",
            p.issues
        );
        assert!(!root.join("delivery").exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn sheet_snapshot_full_source_package_refuses_a_newer_file_beside_an_older_loaded_xref() {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/delivery.json"
    ))
    .unwrap();
    let root =
        std::env::temp_dir().join(format!("kentos-package-loaded-rev-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let path = root.join("xref.kcad");
    fs::write(&path, serde_json::to_vec(&fx["model"]).unwrap()).unwrap();
    let mut doc =
        Document::from_snapshot(serde_json::from_value(fx["model"].clone()).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let answer=kentos_native_application::call::call(&mut doc,None,serde_json::from_value(json!({"command":"cad.xrefs.edit","input":{"operation":{"kind":"attach","path":path,"layerId":"cizim"}}})).unwrap());
    assert!(
        matches!(
            answer.result,
            kentos_commands::CommandResult::Completed { .. }
        ),
        "{answer:?}"
    );
    fs::write(&path, b"newer source version").unwrap();
    let input: PublishSheets = serde_json::from_value(
        json!({"path":root.join("delivery"),"drawing":root.join("source.kcad"),"resources":true}),
    )
    .unwrap();
    let p = Prepared::plan(&doc, &input, &book, &["ground".into()]).unwrap();
    assert!(
        p.issues
            .iter()
            .any(|i| i.error && i.message.contains("sürüm")),
        "{:?}",
        p.issues
    );
    assert!(!root.join("delivery").exists());
    fs::remove_dir_all(root).unwrap();
}

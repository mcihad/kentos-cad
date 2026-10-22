//! Binary format targets, explicit selection and atomic file effects through the real command host.
use super::*;

#[test]
fn dwg_targets_share_the_command_path_and_preserve_atomic_file_effects() {
    let folder = scratch("dwg-commands");
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("layer")
        .to_owned();
    let uid = created(
        &s.run("cad.polygon.create", None, Op::Execute, square(&layer))
            .expect("create"),
    );
    let before = s.summary();
    for version in kentos_commands::DwgVersion::ALL {
        let file = folder.join(format!("{}.dwg", version.signature()));
        let input = json!({"path": file, "version": version, "uids": [uid]});
        let planned = s
            .call(json!({"command": "file.export.dwg", "op": "plan", "input": input}))
            .expect("plan");
        assert_eq!(planned["status"], "completed", "{version:?}: {planned}");
        assert!(!file.exists());
        let done = s
            .call(json!({"command": "file.export.dwg", "input": input}))
            .expect("execute");
        assert_eq!(done["status"], "completed", "{version:?}: {done}");
        assert_eq!(planned["output"]["sha256"], done["output"]["sha256"]);
        let bytes = std::fs::read(&file).expect("file");
        assert_eq!(&bytes[..6], version.signature().as_bytes());
        assert_eq!(
            done["output"]["sha256"],
            kentos_commands::encoding::sha256_hex(&bytes)
        );
        assert_eq!(done["output"]["bytes"], bytes.len());
        assert_eq!(
            done["output"]["report"]["codec"]["targetVersion"],
            version.signature()
        );
        assert_eq!(
            done["output"]["report"]["codec"]["independentValidation"],
            false
        );
        if version == kentos_commands::DwgVersion::AC1014 {
            assert!(
                done["warnings"]
                    .as_array()
                    .expect("warnings")
                    .iter()
                    .any(|w| w["code"] == "export_losses")
            );
            assert!(
                done["output"]["report"]["notes"]
                    .as_array()
                    .expect("notes")
                    .iter()
                    .any(|n| n["what"] == "R14 biçimi")
            );
        }
        let rejected = s
            .call(json!({"command": "file.export.dwg", "input": input}))
            .expect("refusal");
        assert_eq!(rejected["error"]["code"], "file_exists");
        assert_eq!(std::fs::read(&file).expect("old file"), bytes);
        // A rejected selection cannot replace an existing file, even with overwrite.
        let rejected = s
            .call(json!({"command": "file.export.dwg", "input": {
                "path": file, "version": version, "uids": [], "overwrite": true
            }}))
            .expect("empty refusal");
        assert_eq!(rejected["error"]["code"], "nothing_to_export");
        assert_eq!(std::fs::read(&file).expect("retained"), bytes);
    }
    assert_eq!(s.summary()["revision"], before["revision"]);
    assert_eq!(s.summary()["objects"], before["objects"]);
    let file = folder.join("refused.dwg");
    for (extra, code) in [
        (json!({"uids": ["bad-uuid"]}), "invalid_uid"),
        (
            json!({"uids": ["00000000-0000-0000-0000-000000000001"]}),
            "entity_not_found",
        ),
        (json!({"layers": ["missing"]}), "layer_not_found"),
        (json!({"layers": []}), "nothing_to_export"),
    ] {
        let mut input = extra;
        input["path"] = json!(file);
        let result = s
            .call(json!({"command": "file.export.dwg", "input": input}))
            .expect("refusal");
        assert_eq!(result["error"]["code"], code, "{result}");
        assert!(!file.exists());
    }
    // A valid explicit selection has exactly one top-level object on readback.
    let first = folder.join("AC1032.dwg");
    let result =
        kentos_dwg::read(&std::fs::read(first).expect("bytes"), &Default::default()).expect("read");
    assert_eq!(result.entities.len(), 1);
    let journal = s.journal();
    assert!(
        journal["calls"]
            .as_array()
            .expect("calls")
            .iter()
            .any(|r| r["command"] == "file.export.dwg" && r["effects"][0]["path"].is_string())
    );
    std::fs::remove_dir_all(folder).expect("cleanup");
}

#[test]
fn real_paper_space_becomes_a_sheet_book_and_undo_restores_both_model_and_book() {
    let folder = scratch("cad-paper");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1/cad-io/layouts.dxf");
    let mut session = new_drawing();
    let before = session.document().to_snapshot_v2();
    let plan = session
        .call(json!({"command":"file.import.dxf","op":"plan","input":{"path":source}}))
        .expect("plan");
    assert_eq!(plan["status"], "completed", "{plan}");
    assert_eq!(session.document().to_snapshot_v2(), before);
    let done = session
        .call(json!({"command":"file.import.dxf","input":{"path":source}}))
        .expect("import");
    assert_eq!(done["status"], "completed", "{done}");
    let saved = folder.join("paper.kcad");
    session.save(Some(&saved)).expect("save");
    let (drawing, _) = kentos_headless::files::read(&saved).expect("document with real book");
    let book = kentos_sheet_book(&drawing);
    assert_eq!(book["sheets"].as_array().expect("sheets").len(), 2);
    let items = book["sheets"][0]["items"]
        .as_array()
        .expect("paper objects");
    assert_eq!(items.len(), 3);
    let view = items
        .iter()
        .find(|i| i["kind"]["type"] == "map")
        .expect("real view frame");
    assert_eq!(
        view["frame"],
        json!({"left":50000,"top":177000,"width":100000,"height":80000})
    );
    assert_eq!(view["kind"]["view"]["scale"], 2500);
    assert_eq!(view["kind"]["view"]["center"], json!({"x":90.,"y":205.}));
    let target = folder.join("paper.dxf");
    let exported = session
        .call(json!({"command":"file.export.dxf","input":{"path":target}}))
        .expect("export");
    assert_eq!(exported["status"], "completed", "{exported}");
    let read = kentos_formats::dxf::read(&std::fs::read(target).expect("DXF"), &Default::default())
        .expect("read");
    assert_eq!(read.layouts.len(), 2);
    assert_eq!(read.layouts[0].viewports[0].view_height, 200.);
    let kentos_commands::Entity::Text(text) = read.layouts[0]
        .entities
        .iter()
        .find(|e| matches!(e, kentos_commands::Entity::Text(_)))
        .expect("paper text")
    else {
        panic!("text");
    };
    assert!((text.p.x - 0.015).abs() < 1e-6, "{}", text.p.x);
    assert!((text.p.y - 0.050).abs() < 1e-6, "{}", text.p.y);
    assert!((text.rotation.rem_euclid(360.) - 30.).abs() < 1e-6);

    let dwg_target = folder.join("paper.dwg");
    let exported = session
        .call(json!({"command":"file.export.dwg","input":{"path":dwg_target}}))
        .expect("DWG export");
    assert_eq!(exported["status"], "completed", "{exported}");
    let read = kentos_dwg::read(
        &std::fs::read(dwg_target).expect("DWG"),
        &Default::default(),
    )
    .expect("DWG book");
    assert_eq!(read.layouts.len(), 2);
    assert_eq!(read.layouts[0].viewports.len(), 1);
    assert_eq!(read.layouts[0].entities.len(), 2);
    session.undo().expect("one import step");
    let after = session.document().to_snapshot_v2();
    assert_eq!(after.entities, before.entities);
    assert_eq!(after.sheets, before.sheets);
    std::fs::remove_dir_all(folder).expect("cleanup");
}
fn kentos_sheet_book(drawing: &kentos_domain::Document) -> &serde_json::Value {
    &drawing.sheets().expect("real sheet book").book
}

#[test]
fn foreign_cad_xrefs_load_and_reload_with_stable_source_handles() {
    let folder = scratch("cad-xref");
    for ext in ["dxf", "dwg"] {
        let file = folder.join(format!("source.{ext}"));
        let source = b"0\nSECTION\n2\nENTITIES\n0\nLINE\n5\nAB\n8\n0\n10\n1\n20\n2\n11\n3\n21\n4\n0\nENDSEC\n0\nEOF\n";
        let bytes = if ext == "dwg" {
            kentos_dwg::encode(source, kentos_commands::DwgVersion::AC1032).expect("codec")
        } else {
            source.to_vec()
        };
        std::fs::write(&file, bytes).expect("source");
        let mut s = new_drawing();
        let layer = s.summary()["activeLayer"]
            .as_str()
            .expect("layer")
            .to_owned();
        let attached = s.call(json!({"command":"cad.xrefs.edit","input":{"operation":{"kind":"attach","path":file,"layerId":layer}}})).expect("attach");
        assert_eq!(attached["status"], "completed", "{attached}");
        let before = s
            .document()
            .blocks()
            .iter()
            .find(|b| b.xref.is_some())
            .expect("xref")
            .clone();
        assert_eq!(before.entities.len(), 1);
        let name = before.name.clone();
        let ids = before.xref.as_ref().expect("source").objects.clone();
        let reloaded = s.call(json!({"command":"cad.xrefs.edit","input":{"operation":{"kind":"reload","name":name}}})).expect("reload");
        assert_eq!(reloaded["status"], "completed", "{reloaded}");
        let after = s
            .document()
            .blocks()
            .iter()
            .find(|b| b.id == before.id)
            .expect("same block");
        assert_eq!(after.xref.as_ref().expect("source").objects, ids);
        assert_eq!(after.entities, before.entities);
    }
    std::fs::remove_dir_all(folder).expect("cleanup");
}

#[test]
fn broken_dxf_import_keeps_the_open_document_and_undo_state() {
    let folder = scratch("broken-cad");
    let file = folder.join("broken.dxf");
    std::fs::write(
        &file,
        b"0\nSECTION\n2\nENTITIES\n0\nLINE\n10\n1\n20\n2\n11\n3\n21\n4\n",
    )
    .expect("file");
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("layer")
        .to_owned();
    s.run("cad.polygon.create", None, Op::Execute, square(&layer))
        .expect("create");
    let before = s.document().to_snapshot_v2();
    for op in ["validate", "plan", "execute"] {
        let answer = s
            .call(json!({"command":"file.import.dxf","op":op,"input":{"path":file}}))
            .expect("reply");
        assert_eq!(answer["error"]["code"], "import_unreadable", "{answer}");
        assert_eq!(s.document().to_snapshot_v2(), before);
    }
    std::fs::remove_dir_all(folder).expect("cleanup");
}

#[test]
fn a_paper_only_file_imports_exports_and_undoes_without_model_objects() {
    let folder = scratch("cad-paper-only");
    let source = include_str!("../../../../../fixtures/formats/v1/cad-io/layouts.dxf");
    let model = "0\nLINE\n5\n101\n8\n0\n10\n0\n20\n0\n11\n100\n21\n0\n";
    assert!(source.contains(model));
    let path = folder.join("paper-only.dxf");
    std::fs::write(&path, source.replacen(model, "", 1)).expect("source");
    let mut session = new_drawing();
    let before = session.document().to_snapshot_v2();
    let imported = session
        .call(json!({"command":"file.import.dxf","input":{"path":path}}))
        .expect("import");
    assert_eq!(imported["status"], "completed", "{imported}");
    assert_eq!(imported["output"]["objects"], 0);
    assert!(session.document().sheets().is_some());
    for ext in ["dxf", "dwg"] {
        let file = folder.join(format!("export.{ext}"));
        let done = session
            .call(json!({"command":format!("file.export.{ext}"),"input":{"path":file}}))
            .expect("export");
        assert_eq!(done["status"], "completed", "{done}");
        assert!(file.exists());
    }
    session.undo().expect("undo paper");
    assert_eq!(session.document().to_snapshot_v2().sheets, before.sheets);
    std::fs::remove_dir_all(folder).expect("cleanup");
}

#[test]
fn polyline_integrity_is_shared_by_dxf_dwg_plan_execute_and_atomic_exports() {
    let folder = scratch("polyline-integrity");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/cad-io/ocs-0-width.dxf");
    let dwg = folder.join("tilted.dwg");
    let binary = kentos_dwg::encode(&std::fs::read(&source).unwrap(), kentos_commands::DwgVersion::AC1032).unwrap();
    std::fs::write(&dwg, binary).unwrap();
    for (command,path) in [("file.import.dxf",&source),("file.import.dwg",&dwg)] {
        let mut s = new_drawing();
        let before = s.summary();
        for op in ["plan","execute"] {
            let refused = s.call(json!({"command":command,"op":op,"input":{"path":path}})).unwrap();
            assert_eq!(refused["error"]["code"],"polyline_geometry_loss","{refused}");
            assert_eq!(s.summary(),before);
        }
        let accepted = s.call(json!({"command":command,"input":{"path":path,"allowPolylineLoss":true}})).unwrap();
        assert_eq!(accepted["status"],"completed","{accepted}");
        assert!(accepted["output"]["report"]["notes"].as_array().unwrap().iter().any(|r| r["details"].as_array().is_some_and(|ds| ds.iter().any(|d| d["property"].as_str().is_some_and(|p| p.starts_with("polyline.loss."))))));
        s.undo().unwrap();
        assert_eq!(s.summary()["objects"],before["objects"]);
    }
    for feature in ["width","bulge"] {
        let mut s = new_drawing();
        let layer = s.summary()["activeLayer"].as_str().unwrap().to_owned();
        let mut geometry = json!({"kind":"polyline","pts":[{"x":0.,"y":0.},{"x":3.,"y":4.}],"zs":[7.,8.]});
        if feature == "width" { geometry["widths"] = json!([[0.2,0.4],[0.,0.]]); }
        else { geometry["bulges"] = json!([0.5]); }
        let made = s.call(json!({"command":"cad.entities.create","input":{"layerId":layer,"objects":[{"geometry":geometry}]}})).unwrap();
        assert_eq!(made["status"],"completed","{made}");
        let uid = made["output"]["created"][0].as_str().expect("created uid").to_owned();
        let before = s.summary();
        for ext in ["dxf","dwg"] {
            let path = folder.join(format!("nonplanar-{feature}.{ext}"));
            std::fs::write(&path,b"ORIGINAL TARGET").unwrap();
            for op in ["plan","execute"] {
                let refused = s.call(json!({"command":format!("file.export.{ext}"),"op":op,"input":{"path":path,"overwrite":true}})).unwrap();
                assert_eq!(refused["error"]["code"],"polyline_geometry_loss","{refused}");
                assert_eq!(std::fs::read(&path).unwrap(),b"ORIGINAL TARGET");
                assert_eq!(s.summary(),before);
            }
            let accepted = s.call(json!({"command":format!("file.export.{ext}"),"input":{"path":path,"overwrite":true,"allowPolylineLoss":true}})).unwrap();
            assert_eq!(accepted["status"],"completed","{accepted}");
            assert!(accepted["output"]["report"]["notes"].as_array().unwrap().iter().any(|n|
                n["details"].as_array().is_some_and(|ds| ds.iter().any(|d| d["sourceId"] == uid))), "{accepted}");
            assert_ne!(std::fs::read(&path).unwrap(),b"ORIGINAL TARGET");
            assert_eq!(s.summary(),before);
        }
    }
}

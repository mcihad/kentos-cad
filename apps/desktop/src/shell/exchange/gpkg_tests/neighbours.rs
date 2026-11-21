//! GeoPackage binding, neighbours and validity (docs/adr/0531, docs/adr/0535, docs/adr/0585): with topology on, a
//! part's write writes its neighbours too; a neighbour left invalid stops a strict write and says so.

use super::*;

/// A part of komsu.gpkg (docs/adr/0531) bound over a copy, parcel A's bump stretched a metre east; the rest of the
/// table (B, C outside the area) bound as a second layer once written, to show where it went.
fn neighbours_scene(name: &str) -> (App, PathBuf) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let file = scratch(name).join("komsu.gpkg");
    std::fs::copy(gpkg("komsu.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut run_on = |command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer["output"].clone()
    };
    run_on(
        "cad.sources.edit",
        serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "parsel",
                                      "extent": [486990.0, 4419990.0, 487009.0, 4420030.0] } }),
    );
    let a = doc
        .model
        .by_layer_with_uids("import-parsel")
        .next()
        .map(|(u, _)| u.to_string())
        .expect("A");
    let mut run_on = |command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    run_on(
        "cad.entities.stretch",
        serde_json::json!({ "window": { "min": { "x": 487011.5, "y": 4420004.5 },
                                        "max": { "x": 487012.5, "y": 4420005.5 } },
                            "dx": 1.0, "dy": 0.0, "uids": [a] }),
    );
    (app, file)
}

#[test]
fn a_part_writes_its_neighbours_while_topology_is_on() {
    // docs/adr/0531: Kaynağa yaz on a part with Topoloji on follows the neighbours outside the area (strict).
    let (mut app, file) = neighbours_scene("gpkg-komsu");
    let _ = app
        .settings
        .choose(&[("drafting.topology", serde_json::Value::Bool(true))]);
    app.apply_settings();
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        says(&app, "1 değişti, 2 alanın dışındaki komşu izledi"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    // The whole table read back: every parcel has the bump a metre east.
    let mut other = app_with_drawing();
    let whole = &mut other.document.as_mut().expect("a drawing").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(whole),
        "file.import.gpkg",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "path": file.display().to_string(), "tables": ["parsel"] }),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let layer = answer["output"]["tables"][0]["layer"]
        .as_str()
        .unwrap_or_else(|| panic!("{answer}"))
        .to_owned();
    let shapes: Vec<String> = whole
        .by_layer_with_uids(&layer)
        .map(|(_, e)| serde_json::to_string(e).expect("entity"))
        .collect();
    let moved = whole
        .by_layer_with_uids(&layer)
        .filter(|(_, e)| {
            let v = serde_json::to_value(e).expect("entity");
            v["pts"].as_array().is_some_and(|pts| {
                pts.iter()
                    .any(|p| p["x"] == 487_013.0 && p["y"] == 4_420_005.0)
            })
        })
        .count();
    assert_eq!(moved, 3, "{answer} {shapes:?}");
    // The menu of a part has the loose write too.
    let node = app
        .document
        .as_ref()
        .and_then(|d| d.model.layers().get("import-parsel").cloned())
        .expect("the layer");
    assert!(format!("{:?}", app.layer_menu(&node, false)).contains("Kaynağa yaz, komşular gevşek"));
}

#[test]
#[ignore = "pictures, run by hand"]
fn neighbour_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        for mode in ["dark", "light"] {
            let (mut app, file) = neighbours_scene(&format!("gpkg-komsu-{mode}-{width}"));
            let _ = app.settings.choose(&[
                ("appearance.theme", serde_json::Value::from(mode)),
                ("drafting.topology", serde_json::Value::Bool(true)),
            ]);
            app.apply_settings();
            let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
                "import-parsel".into(),
            ))));
            // The file as it now is in place of the part: the binding undone, A taken out, the whole table taken in.
            let doc = app.document.as_mut().expect("a drawing");
            let a: Vec<String> = doc
                .model
                .by_layer_with_uids("import-parsel")
                .map(|(u, _)| u.to_string())
                .collect();
            for (command, input) in [
                (
                    "cad.sources.edit",
                    serde_json::json!({ "edit": { "kind": "unlink", "id": "kaynak-1" } }),
                ),
                ("cad.entities.delete", serde_json::json!({ "uids": a })),
            ] {
                let answer = kentos_native_application::registry::run(
                    &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                    command,
                    kentos_commands::CallOp::Execute,
                    input,
                )
                .expect("runs");
                assert_eq!(answer["status"], "completed", "{answer}");
            }
            let answer = kentos_native_application::registry::run(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                "file.import.gpkg",
                kentos_commands::CallOp::Execute,
                serde_json::json!({ "path": file.display().to_string(), "tables": ["parsel"],
                                    "layers": { "parsel": "import-parsel" } }),
            )
            .expect("runs");
            assert_eq!(answer["status"], "completed", "{answer}");
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            // The first frames settle the window and its view; the view is brought to the parcels after them.
            let slots: Vec<f64> = {
                let doc = app.document.as_ref().expect("a drawing");
                doc.model
                    .by_layer_with_uids("import-parsel")
                    .filter_map(|(u, _)| doc.model.slot_of(u))
                    .map(|s| f64::from(s.0))
                    .collect()
            };
            app.zoom_to_objects(&slots);
            // Selected, so the parcels are seen with their shared bump where A put it.
            let picked: Vec<kentos_domain::Slot> = slots
                .iter()
                .map(|s| kentos_domain::Slot(*s as u32))
                .collect();
            app.selection.set(picked);
            snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!("gpkg-{mode}-10-komsular-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        }
    }
}

/// komsu-gecerlilik.gpkg (docs/adr/0585) bound by A's area over a copy, the rows outside the area drawn beside it on the
/// sample's Parsel layer to see them, and A's corner (10, 4) pulled out east where B would cross itself.
fn validity_scene(name: &str) -> (App, String) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let file = scratch(name).join("komsu-gecerlilik.gpkg");
    std::fs::copy(gpkg("komsu-gecerlilik.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let path = file.display().to_string();
    let run_on = |doc: &mut kentos_domain::Document, command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer["output"].clone()
    };
    // The sample drawing's Parsel layer, on screen already.
    let table = "parsel".to_owned();
    assert!(
        doc.model.layers().get(&table).is_some(),
        "the sample's Parsel layer"
    );
    // The table's rows outside the area, drawn plainly beside the part (the file's own ids are the part's).
    for ring in [
        &[(10.0, 0.0), (20.0, 0.0), (20.0, 4.0), (10.0, 4.0)][..],
        &[(10.0, 6.0), (14.0, 8.0), (10.0, 10.0)][..],
        &[(10.0, 10.0), (20.0, 20.0), (20.0, 10.0), (10.0, 20.0)][..],
    ] {
        let pts: Vec<serde_json::Value> = ring
            .iter()
            .map(|(x, y)| serde_json::json!({ "x": 487000.0 + x, "y": 4420000.0 + y }))
            .collect();
        run_on(
            &mut doc.model,
            "cad.polygon.create",
            serde_json::json!({ "layerId": table, "pts": pts }),
        );
    }
    run_on(
        &mut doc.model,
        "cad.sources.edit",
        serde_json::json!({ "edit": { "kind": "link", "path": path, "table": "parsel",
                                      "extent": [486990.0, 4419990.0, 487005.0, 4420030.0] } }),
    );
    let a = doc
        .model
        .by_layer_with_uids("import-parsel")
        .next()
        .map(|(u, _)| u.to_string())
        .expect("A");
    run_on(
        &mut doc.model,
        "cad.entities.stretch",
        serde_json::json!({ "window": { "min": { "x": 487009.5, "y": 4420003.5 },
                                        "max": { "x": 487010.5, "y": 4420004.5 } },
                            "dx": 15.0, "dy": -2.0, "uids": [a] }),
    );
    (app, table)
}

#[test]
fn a_neighbour_left_invalid_stops_a_strict_write_and_says_so() {
    let (mut app, _) = validity_scene("gpkg-gecerlilik-test");
    let _ = app
        .settings
        .choose(&[("drafting.topology", serde_json::Value::Bool(true))]);
    app.apply_settings();
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    let said = crate::files_testing::last_said(&app);
    assert!(
        said.contains("1 komşu kayıt") && said.contains("geçersiz kalırdı"),
        "{said}"
    );
    // Loose: A is written, B kept as it is and said.
    let _ = app.update(Message::Bound(crate::bound::Event::WriteLoose(Some(
        "import-parsel".into(),
    ))));
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("olduğu gibi kaldı"), "{said}");
}

/// The strict write refused over the neighbour B would leave crossing itself, with the whole table beside it,
/// `.run/shots/gpkg-*-11-gecerlilik-*`:
///
/// ```text
/// cargo test -p kentos-desktop exchange::gpkg_tests::neighbours::validity_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn validity_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
        for mode in ["dark", "light"] {
            let (mut app, table) = validity_scene(&format!("gpkg-gecerlilik-{mode}-{width}"));
            let _ = app.settings.choose(&[
                ("appearance.theme", serde_json::Value::from(mode)),
                ("drafting.topology", serde_json::Value::Bool(true)),
            ]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            // The part's A and the rows drawn beside it (the sample's own parcel left out), selected: the
            // scene's direct edits show through the selection.
            let picked: Vec<kentos_domain::Slot> = {
                let doc = app.document.as_ref().expect("a drawing");
                doc.model
                    .by_layer_with_uids(&table)
                    .chain(doc.model.by_layer_with_uids("import-parsel"))
                    .filter(|(_, e)| e.base().label.is_none())
                    .filter_map(|(u, _)| doc.model.slot_of(u))
                    .collect()
            };
            let slots: Vec<f64> = picked.iter().map(|s| f64::from(s.0)).collect();
            app.zoom_to_objects(&slots);
            app.selection.set(picked);
            let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
                "import-parsel".into(),
            ))));
            snapshot.settle(&mut app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let path = out.join(format!("gpkg-{mode}-11-gecerlilik-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        }
    }
}

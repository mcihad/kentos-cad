//! GeoPackage binding, reconcile (docs/adr/0538, docs/adr/0553): a write over a changed file opens the reconcile; the
//! file's other layers come along and a group is resolved whole.

use super::*;

/// uzlas.gpkg (docs/adr/0538) bound over a copy, the layer edited as the shared cases edit it, then the copy changed
/// as QGIS changed it (uzlas-qgis.gpkg): both sides changed since they last matched.
pub(super) fn reconcile_scene(name: &str) -> (App, PathBuf) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let file = scratch(name).join("uzlas.gpkg");
    std::fs::copy(gpkg("uzlas.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let run_on = |doc: &mut kentos_domain::Document, command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    run_on(
        doc,
        "cad.sources.edit",
        serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": "parsel" } }),
    );
    let uid = |doc: &kentos_domain::Document, n: u32| {
        doc.by_layer_with_uids("import-parsel")
            .find(|(_, e)| e.base().attrs.get("parsel_no") == Some(&n.to_string()))
            .map(|(u, _)| u.to_string())
            .expect("a parcel")
    };
    for (n, attrs) in [
        (4, serde_json::json!({ "malik": "Veli" })),
        (5, serde_json::json!({ "parsel_no": "50" })),
        (6, serde_json::json!({ "malik": "Mehmet" })),
        (8, serde_json::json!({ "malik": "Selin" })),
        (10, serde_json::json!({ "malik": "Ortak" })),
    ] {
        let u = uid(doc, n);
        run_on(
            doc,
            "cad.entities.set",
            serde_json::json!({ "uids": [u], "attrs": attrs, "operation": "attributes" }),
        );
    }
    for n in [7, 9] {
        let u = uid(doc, n);
        run_on(
            doc,
            "cad.entities.delete",
            serde_json::json!({ "uids": [u] }),
        );
    }
    run_on(
        doc,
        "cad.polygon.create",
        serde_json::json!({ "layerId": "import-parsel",
                            "pts": [{ "x": 487400.0, "y": 4420000.0 }, { "x": 487420.0, "y": 4420000.0 },
                                    { "x": 487420.0, "y": 4420020.0 }, { "x": 487400.0, "y": 4420020.0 }] }),
    );
    std::fs::copy(gpkg("uzlas-qgis.gpkg"), &file).expect("changed in QGIS");
    (app, file)
}

#[test]
fn a_write_over_a_changed_file_opens_the_reconcile_and_puts_both_sides_together() {
    use crate::source_reconcile::Event as R;
    use kentos_commands::ReconcileSide;
    let (mut app, file) = reconcile_scene("gpkg-uzlas");
    let before = std::fs::read(&file).expect("read");
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        app.gis.reconcile.is_some(),
        "the refused write opens the window"
    );
    assert_eq!(
        std::fs::read(&file).expect("read"),
        before,
        "nothing written over"
    );
    // Uzlaştır waits for every conflict's side.
    let view = format!("{:?}", app.gis.reconcile);
    assert!(
        view.contains("RemovedInLayer") && view.contains("RemovedInTable"),
        "{view}"
    );
    let _ = app.update(Message::Gis(gis::Event::Reconcile(R::All(
        ReconcileSide::Layer,
    ))));
    let _ = app.update(Message::Gis(gis::Event::Reconcile(R::Apply)));
    assert!(app.gis.reconcile.is_none(), "closed when done");
    assert!(
        says(&app, "tablosuyla uzlaştırıldı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    assert_ne!(std::fs::read(&file).expect("read"), before, "written");
    // The two match now: Kaynağa yaz has nothing to write.
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        says(&app, "yazılacak bir şey yok"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
}

#[test]
#[ignore = "pictures, run by hand"]
fn reconcile_screens() {
    use crate::source_reconcile::Event as R;
    use iced::Size;
    use kentos_commands::ReconcileSide;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let (mut app, _file) = reconcile_scene(&format!("gpkg-uzlas-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!("gpkg-uzlastir-{mode}-{label}-{width}x{height}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        picture(&mut app, "cizim");
        let slots: Vec<f64> = {
            let doc = app.document.as_ref().expect("a drawing");
            doc.model
                .by_layer_with_uids("import-parsel")
                .filter_map(|(u, _)| doc.model.slot_of(u))
                .map(|s| f64::from(s.0))
                .collect()
        };
        app.zoom_to_objects(&slots);
        let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
            "import-parsel".into(),
        ))));
        picture(&mut app, "pencere");
        // One conflict to the table's side, the rest to the layer's.
        let first = app
            .gis
            .reconcile
            .as_ref()
            .and_then(|w| w.first_conflict())
            .expect("a conflict");
        let _ = app.update(Message::Gis(gis::Event::Reconcile(R::All(
            ReconcileSide::Layer,
        ))));
        let _ = app.update(Message::Gis(gis::Event::Reconcile(R::Side(
            first,
            ReconcileSide::Table,
        ))));
        picture(&mut app, "secildi");
        let _ = app.update(Message::Gis(gis::Event::Reconcile(R::Apply)));
        app.zoom_to_objects(&slots);
        picture(&mut app, "uzlasti");
    }
}

/// uzlas-komsu.gpkg (docs/adr/0553) bound over a copy, both tables (four parcels side by side, three buildings), the
/// project's relation, the layers edited as tests/all/reconcile.rs edits them, then the copy changed as QGIS changed it.
fn komsu_scene(name: &str) -> (App, PathBuf) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let file = scratch(name).join("uzlas-komsu.gpkg");
    std::fs::copy(gpkg("uzlas-komsu.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let run_on = |doc: &mut kentos_domain::Document, command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    for table in ["komsu", "bina"] {
        run_on(
            doc,
            "cad.sources.edit",
            serde_json::json!({ "edit": { "kind": "link", "path": file.display().to_string(), "table": table } }),
        );
    }
    run_on(
        doc,
        "cad.relations.edit",
        serde_json::json!({ "edit": { "kind": "create", "relation": { "id": "bina-komsu", "name": "Parselin binaları",
            "parent": "import-komsu", "child": "import-bina", "field": "komsu" } } }),
    );
    let by = |doc: &kentos_domain::Document, layer: &str, name: &str, value: &str| {
        doc.by_layer_with_uids(layer)
            .find(|(_, e)| e.base().attrs.get(name).map(String::as_str) == Some(value))
            .map(|(u, _)| u.to_string())
            .expect("an object")
    };
    let (x0, y0) = (487000.0, 4420000.0);
    let p = |x: f64, y: f64| serde_json::json!({ "x": x0 + x, "y": y0 + y });
    let u = by(doc, "import-komsu", "ad", "K1");
    run_on(
        doc,
        "cad.entities.set",
        serde_json::json!({ "uids": [u], "attrs": { "ad": "Z1" }, "operation": "attributes" }),
    );
    for (k, pts) in [
        (
            "K2",
            [p(20.0, 0.0), p(40.0, 0.0), p(42.0, 22.0), p(20.0, 20.0)],
        ),
        (
            "K3",
            [p(40.0, 0.0), p(60.0, 0.0), p(60.0, 20.0), p(42.0, 22.0)],
        ),
    ] {
        let u = by(doc, "import-komsu", "ad", k);
        run_on(
            doc,
            "cad.entities.edit",
            serde_json::json!({ "operation": "grip", "changes": [{ "kind": "update", "uid": u,
                "geometry": { "kind": "polygon", "pts": pts } }] }),
        );
    }
    let u = by(doc, "import-komsu", "ad", "K4");
    run_on(
        doc,
        "cad.entities.delete",
        serde_json::json!({ "uids": [u] }),
    );
    for (k, kat) in [("K2", "5"), ("K3", "7")] {
        let parent = by(doc, "import-komsu", "ad", k);
        let u = by(doc, "import-bina", "komsu", &parent);
        run_on(
            doc,
            "cad.entities.set",
            serde_json::json!({ "uids": [u], "attrs": { "kat": kat }, "operation": "attributes" }),
        );
    }
    std::fs::copy(gpkg("uzlas-komsu-qgis.gpkg"), &file).expect("changed in QGIS");
    (app, file)
}

#[test]
fn a_reconcile_takes_the_files_other_layers_and_resolves_a_group_whole() {
    use crate::source_reconcile::Event as R;
    use kentos_commands::ReconcileSide;
    let (mut app, file) = komsu_scene("gpkg-komsu");
    let before = std::fs::read(&file).expect("read");
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-komsu".into(),
    ))));
    let w = app
        .gis
        .reconcile
        .as_ref()
        .expect("the refused write opens the window");
    let view = format!("{w:?}");
    // The file's other layer goes with it; the plan says what merged and the two groups.
    assert!(view.contains("with: [\"import-bina\"]"), "{view}");
    assert!(
        view.contains("SharedVertex") && view.contains("Relation"),
        "{view}"
    );
    assert!(view.contains("merged: [ReconcileMerge"), "{view}");
    assert_eq!(
        std::fs::read(&file).expect("read"),
        before,
        "nothing written over"
    );
    // A side taken for one member of a group is taken for all of them.
    let member = w
        .plan_conflicts()
        .into_iter()
        .find(|(_, group)| group.is_some())
        .expect("a group's member");
    let _ = app.update(Message::Gis(gis::Event::Reconcile(R::Side(
        member.0.clone(),
        ReconcileSide::Table,
    ))));
    let sides = app
        .gis
        .reconcile
        .as_ref()
        .expect("the window")
        .sides_taken();
    assert!(sides >= 2, "the group's members all took the side: {sides}");
    let _ = app.update(Message::Gis(gis::Event::Reconcile(R::All(
        ReconcileSide::Layer,
    ))));
    let _ = app.update(Message::Gis(gis::Event::Reconcile(R::Apply)));
    assert!(app.gis.reconcile.is_none(), "closed when done");
    assert_ne!(std::fs::read(&file).expect("read"), before, "written");
    // Both tables went in one call: nothing is left to write for either layer.
    for layer in ["import-komsu", "import-bina"] {
        let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
            layer.into(),
        ))));
        assert!(
            says(&app, "yazılacak bir şey yok"),
            "{layer}: {:?}",
            app.log.lines().last().map(|l| l.text.clone())
        );
    }
}

#[test]
#[ignore = "pictures, run by hand"]
fn reconcile_group_screens() {
    use crate::source_reconcile::Event as R;
    use iced::Size;
    use kentos_commands::ReconcileSide;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let (mut app, _file) = komsu_scene(&format!("gpkg-komsu-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!(
                "gpkg-uzlastir-grup-{mode}-{label}-{width}x{height}.png"
            ));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        let slots: Vec<f64> = {
            let doc = app.document.as_ref().expect("a drawing");
            doc.model
                .by_layer_with_uids("import-komsu")
                .filter_map(|(u, _)| doc.model.slot_of(u))
                .map(|s| f64::from(s.0))
                .collect()
        };
        app.zoom_to_objects(&slots);
        let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
            "import-komsu".into(),
        ))));
        picture(&mut app, "pencere");
        // One group to the table's side, the rest to the layer's.
        let member = app
            .gis
            .reconcile
            .as_ref()
            .and_then(|w| w.plan_conflicts().into_iter().find(|(_, g)| g.is_some()))
            .expect("a group's member");
        let _ = app.update(Message::Gis(gis::Event::Reconcile(R::All(
            ReconcileSide::Layer,
        ))));
        let _ = app.update(Message::Gis(gis::Event::Reconcile(R::Side(
            member.0,
            ReconcileSide::Table,
        ))));
        picture(&mut app, "secildi");
        let _ = app.update(Message::Gis(gis::Event::Reconcile(R::Apply)));
        app.zoom_to_objects(&slots);
        picture(&mut app, "uzlasti");
    }
}

//! GeoPackage binding, source status (docs/adr/0543, docs/adr/0546, docs/adr/0651, docs/adr/0598 §4): each layer's
//! way out, a bound layer's table, a filtered layer's count, the Bilgi card and undoing a step already written.

use super::*;

/// Kaynak durumu's scene (docs/adr/0543): the reconcile scene's parcels (both sides changed), a copy of qgis.gpkg's
/// points with one attribute edited (only the layer changed), wgs84.gpkg's places in their own system, and a file
/// bound then gone.
fn status_scene(name: &str) -> (App, PathBuf) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let (mut app, _) = reconcile_scene(name);
    let dir = scratch(&format!("{name}-durum"));
    let points = dir.join("noktalar.gpkg");
    let gone = dir.join("tasindi.gpkg");
    std::fs::copy(gpkg("qgis.gpkg"), &points).expect("copied");
    std::fs::copy(gpkg("qgis.gpkg"), &gone).expect("copied");
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let run_on = |doc: &mut kentos_domain::Document, input: serde_json::Value, command: &str| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer
    };
    let linked = run_on(
        doc,
        serde_json::json!({ "edit": { "kind": "link", "path": points.display().to_string(), "table": "nokta" } }),
        "cad.sources.edit",
    );
    let layer = linked["output"]["link"]["layer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    run_on(
        doc,
        serde_json::json!({ "edit": { "kind": "link", "path": gpkg("wgs84.gpkg").display().to_string(), "table": "yer", "reproject": true } }),
        "cad.sources.edit",
    );
    run_on(
        doc,
        serde_json::json!({ "edit": { "kind": "link", "path": gone.display().to_string(), "table": "yol" } }),
        "cad.sources.edit",
    );
    let first = doc
        .by_layer_with_uids(&layer)
        .next()
        .map(|(u, _)| u.to_string())
        .expect("a point");
    run_on(
        doc,
        serde_json::json!({ "uids": [first], "attrs": { "ad": "Yeni ad" }, "operation": "attributes" }),
        "cad.entities.set",
    );
    std::fs::remove_file(&gone).expect("gone");
    (app, points)
}

#[test]
fn the_source_status_says_each_layers_way_out_and_takes_it() {
    use crate::source_status::Event as S;
    use kentos_commands::SourceNext;
    let (mut app, points) = status_scene("gpkg-durum");
    let _ = app.update(Message::Bound(crate::bound::Event::Status));
    let next = |app: &App, layer: &str| {
        let i = app.source_status_row(layer).expect("a row");
        app.source_status_next(i)
    };
    let layers: Vec<String> = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .sources
        .iter()
        .map(|s| s.layer.clone())
        .collect();
    assert_eq!(layers.len(), 4, "{layers:?}");
    assert_eq!(
        layers.iter().map(|l| next(&app, l)).collect::<Vec<_>>(),
        [
            SourceNext::Reconcile,
            SourceNext::Write,
            SourceNext::None,
            SourceNext::Relink
        ]
    );
    // Kaynağa yaz on the points: the file written, the row now the same.
    let before = std::fs::read(&points).expect("read");
    let i = app.source_status_row(&layers[1]).expect("a row");
    let _ = app.update(Message::Gis(gis::Event::SourceStatus(S::Act(i))));
    assert_ne!(std::fs::read(&points).expect("read"), before, "written");
    assert_eq!(next(&app, &layers[1]), SourceNext::None);
    // Uzlaştır… opens its own window in this one's place.
    let i = app.source_status_row(&layers[0]).expect("a row");
    let _ = app.update(Message::Gis(gis::Event::SourceStatus(S::Act(i))));
    assert!(app.gis.reconcile.is_some() && app.gis.source_status.is_none());
}

/// The state of the layer the table shows: its way out and whether the reading is of an earlier drawing.
fn table_source(app: &mut App, layer: &str) -> Option<(kentos_commands::SourceNext, bool)> {
    let _ = app.update(Message::Gis(gis::Event::Features(
        crate::features::Event::Layer(layer.to_owned()),
    )));
    let doc = app.document.as_ref().expect("a drawing");
    let rows = app.feature_rows(doc);
    app.source_state(&rows).map(|(s, stale)| (s.next, stale))
}

#[test]
fn the_table_of_a_bound_layer_says_its_state_and_takes_the_way_out() {
    use crate::features::{Event as F, source::SourceEvent};
    use kentos_commands::SourceNext;
    let (mut app, points) = status_scene("gpkg-tablo");
    let _ = app.update(Message::Run("data.featureTable"));
    let layers: Vec<String> = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .sources
        .iter()
        .map(|s| s.layer.clone())
        .collect();
    // The same four states Kaynak durumu reads, from the same rule.
    let ways: Vec<_> = layers
        .iter()
        .map(|l| table_source(&mut app, l).map(|(n, _)| n))
        .collect();
    assert_eq!(
        ways,
        [
            Some(SourceNext::Reconcile),
            Some(SourceNext::Write),
            Some(SourceNext::None),
            Some(SourceNext::Relink)
        ]
    );
    // A layer that is not bound has no source line.
    let free = {
        let doc = app.document.as_ref().expect("a drawing");
        doc.model
            .layers()
            .leaves()
            .iter()
            .map(|n| n.id.clone())
            .find(|id| !layers.contains(id))
    };
    if let Some(free) = free {
        assert_eq!(table_source(&mut app, &free), None);
    }
    // Kaynağa yaz from the table's line: the file written, the line now the same.
    let _ = table_source(&mut app, &layers[1]);
    let before = std::fs::read(&points).expect("read");
    let _ = app.update(Message::Gis(gis::Event::Features(F::Source(
        SourceEvent::Act,
    ))));
    assert_ne!(std::fs::read(&points).expect("read"), before, "written");
    assert_eq!(
        table_source(&mut app, &layers[1]).map(|(n, _)| n),
        Some(SourceNext::None)
    );
    // An edit of the layer is “Katmanda yazılmamış” at once, on a layer small enough to follow the drawing.
    let uid = {
        let doc = app.document.as_ref().expect("a drawing");
        doc.model
            .by_layer_with_uids(&layers[1])
            .next()
            .map(|(u, _)| u.to_string())
            .expect("a point")
    };
    let input = serde_json::json!({ "uids": [uid], "attrs": { "ad": "Başka ad" }, "operation": "attributes" });
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(
            &mut app.document.as_mut().expect("a drawing").model,
        ),
        "cad.entities.set",
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(
        table_source(&mut app, &layers[1]),
        Some((SourceNext::Write, false))
    );
    // Uzlaştır… from the table opens its window.
    let _ = table_source(&mut app, &layers[0]);
    let _ = app.update(Message::Gis(gis::Event::Features(F::Source(
        SourceEvent::Act,
    ))));
    assert!(app.gis.reconcile.is_some());
}

/// Öznitelik tablosu's source line (docs/adr/0546 §3): a bound layer with changes on both sides, one with its own
/// changes, one in its own system, one whose file is gone; the second after Kaynağa yaz
/// (`.run/shots/gpkg-tablo-kaynak-*`).
#[test]
#[ignore = "pictures, run by hand"]
fn table_source_screens() {
    use crate::features::{Event as F, source::SourceEvent};
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let (mut app, _) = status_scene(&format!("gpkg-tablo-{mode}-{width}"));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let layers: Vec<String> = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .settings()
            .sources
            .iter()
            .map(|s| s.layer.clone())
            .collect();
        let _ = app.update(Message::Run("data.featureTable"));
        let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
        let mut picture = |app: &mut App, label: &str| {
            snapshot.settle(app, App::view, &mut |app: &mut App, message| {
                let _ = app.update(message);
            });
            let _ = snapshot.render(app.view(), &app.theme());
            let path = out.join(format!(
                "gpkg-tablo-kaynak-{mode}-{label}-{width}x{height}.png"
            ));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        let _ = app.update(Message::WindowResized(Size::new(width, height)));
        for (layer, label) in [(0, "uzlastir"), (1, "yaz"), (2, "wgs84"), (3, "yok")] {
            let _ = app.update(Message::Gis(gis::Event::Features(F::Layer(
                layers[layer].clone(),
            ))));
            picture(&mut app, label);
        }
        let _ = app.update(Message::Gis(gis::Event::Features(F::Layer(
            layers[1].clone(),
        ))));
        let _ = app.update(Message::Gis(gis::Event::Features(F::Source(
            SourceEvent::Act,
        ))));
        picture(&mut app, "yazildi");
    }
}

#[test]
#[ignore = "pictures, run by hand"]
fn source_status_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (width, height, mode) in [
        (1440.0, 900.0, "dark"),
        (1100.0, 650.0, "dark"),
        (1440.0, 900.0, "light"),
    ] {
        let (mut app, _) = status_scene(&format!("gpkg-durum-{mode}-{width}"));
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
                "gpkg-kaynak-durumu-{mode}-{label}-{width}x{height}.png"
            ));
            snapshot
                .render(app.view(), &app.theme())
                .save(&path)
                .expect("writes the picture");
            println!("{}", path.display());
        };
        let _ = app.update(Message::Bound(crate::bound::Event::Status));
        let missing = app.source_status_row(
            &app.document
                .as_ref()
                .expect("a drawing")
                .model
                .settings()
                .sources[3]
                .layer
                .clone(),
        );
        let _ = app.update(Message::Gis(gis::Event::SourceStatus(
            crate::source_status::Event::Pick(missing.expect("a row")),
        )));
        picture(&mut app, "pencere");
        let i = app
            .source_status_row(
                &app.document
                    .as_ref()
                    .expect("a drawing")
                    .model
                    .settings()
                    .sources[1]
                    .layer
                    .clone(),
            )
            .expect("a row");
        let _ = app.update(Message::Gis(gis::Event::SourceStatus(
            crate::source_status::Event::Act(i),
        )));
        picture(&mut app, "yazildi");
        // docs/adr/0651 §2: a filtered layer's count; the card of an object waiting to be written.
        let (layer, edited) = filter_points(&mut app);
        let _ = app.update(Message::Gis(gis::Event::SourceStatus(
            crate::source_status::Event::Refresh,
        )));
        let i = app.source_status_row(&layer).expect("a row");
        let _ = app.update(Message::Gis(gis::Event::SourceStatus(
            crate::source_status::Event::Pick(i),
        )));
        picture(&mut app, "suzgec");
        let _ = app.update(Message::Gis(gis::Event::SourceStatus(
            crate::source_status::Event::Close,
        )));
        app.info_card = Some((edited, kentos_interaction::Vec2::new(420.0, 300.0)));
        picture(&mut app, "bilgi-karti");
    }
}

/// The points' layer of `status_scene` given a filter of two of its objects, and one of them changed after its last
/// write; the layer and the changed object's slot.
fn filter_points(app: &mut App) -> (String, kentos_domain::Slot) {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    // The first binding with two objects or more.
    let layer = doc
        .settings()
        .sources
        .iter()
        .map(|l| l.layer.clone())
        .find(|l| doc.by_layer_with_uids(l).count() >= 2)
        .expect("a bound layer of two objects");
    let uids: Vec<String> = doc
        .by_layer_with_uids(&layer)
        .take(2)
        .map(|(u, _)| u.to_string())
        .collect();
    for (command, input) in [
        (
            "cad.layers.filter",
            serde_json::json!({ "layer": layer, "filter": { "objects": uids } }),
        ),
        (
            "cad.entities.set",
            serde_json::json!({ "uids": [uids[1]], "attrs": { "ad": "Değişti" }, "operation": "attributes" }),
        ),
    ] {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let slot = doc
        .slot_of(uids[1].parse().expect("a uid"))
        .expect("the object");
    (layer, slot)
}

// docs/adr/0651: Kaynak durumu says a filtered layer's count and why a table was not read; Bilgi kartı says where an
// object stands with its table.
#[test]
fn the_status_and_the_card_say_the_source() {
    let (mut app, _) = status_scene("gpkg-kaynak-satiri");
    let _ = app.update(Message::Bound(crate::bound::Event::Status));
    let (layer, edited) = filter_points(&mut app);
    let _ = app.update(Message::Gis(gis::Event::SourceStatus(
        crate::source_status::Event::Refresh,
    )));
    let states = app.source_status_states();
    let points = states
        .iter()
        .find(|s| s.link.layer == layer)
        .expect("the points");
    assert_eq!(points.filtered, Some(2));
    assert!(
        crate::source_status::scope_words(points).ends_with("· süzgeç 2"),
        "{}",
        crate::source_status::scope_words(points)
    );
    let gone = states.iter().find(|s| !s.found).expect("the moved file");
    assert!(
        gone.error
            .as_deref()
            .is_some_and(|e| e.contains("yerinde değil")),
        "{:?}",
        gone.error
    );
    assert!(crate::source_status::detail_words(gone, 5256).contains("Neden: "));
    let card = app.slot_card(edited).expect("a card");
    let (line, waits) = card.source.expect("a source line");
    assert!(
        line.starts_with("Kaynak: ") && line.ends_with("değişti, kaynağa yazılmadı"),
        "{line}"
    );
    assert!(waits);
    // An object of a layer no table is bound to has no source line: its layer and lasting id instead (docs/adr/9145).
    let free = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .by_layer_with_uids("cizim")
        .next()
        .map(|(_, e)| kentos_domain::Slot(e.base().id));
    if let Some(slot) = free {
        let (line, waits) = app
            .slot_card(slot)
            .expect("a card")
            .source
            .expect("its layer's line");
        assert!(
            line.starts_with("Çizim katmanı · kimlik …") && !waits,
            "{line}"
        );
    }
}

// docs/adr/0598 §4: undoing a step already written to the source says so (the source still holds it; Kaynağa yaz puts
// the undo there too); undoing a step not yet written says nothing more.
#[test]
fn undoing_a_written_step_says_the_source_still_holds_it() {
    let dir = scratch("gpkg-undo-written");
    let file = dir.join("kadastro.gpkg");
    std::fs::copy(gpkg("qgis.gpkg"), &file).expect("copied");
    let mut app = app_with_drawing();
    app.picker = Picker::File(file.clone());
    run(&mut app, "source.link");
    parcels_only(&mut app);
    send(&mut app, Event::Gpkg(G::Run));
    delete_first(&mut app, "import-parsel");
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(says(&app, "tablosuna yazıldı: 1 silindi"));
    app.step_history(true);
    assert!(
        says(&app, "kaynağına yazılmıştı"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    let _ = app.update(Message::Bound(crate::bound::Event::Write(Some(
        "import-parsel".into(),
    ))));
    assert!(
        says(&app, "tablosuna yazıldı: 1 eklendi"),
        "{:?}",
        app.log.lines().last().map(|l| l.text.clone())
    );
    // Not written yet: a plain undo.
    delete_first(&mut app, "import-parsel");
    let before = app
        .log
        .lines()
        .filter(|l| l.text.contains("kaynağına yazılmıştı"))
        .count();
    app.step_history(true);
    assert_eq!(
        app.log
            .lines()
            .filter(|l| l.text.contains("kaynağına yazılmıştı"))
            .count(),
        before
    );
    assert!(said(&app, "Geri alındı"));
}

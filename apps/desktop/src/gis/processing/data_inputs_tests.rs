//! Tampon and Anahtarla birleştir over data layers (the second round's T28, T29; docs/adr/9353), the municipal
//! GeoPackage: `bakim` 6 points, `mahalleler` 3 points, both data layers, the drawing's own objects taken away.
//!
//! - Tampon on `bakim` (Katman, Görünen, Tümü): the input counts the source's 6 rows and names the layer; run, the
//!   drawing gets a layer of 6 buffers in one undo step.
//! - A point drawn in `bakim`'s edit session and not written: the input says 6 in the source and 1 waiting; the run
//!   buffers 7.
//! - Anahtarla birleştir offers the rows' fields; Alan hesapla, which changes its input, does not take the rows and
//!   says why.

use serde_json::json;

use super::super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, drive};
use crate::gis;

fn event(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Processing(e)));
    drive(app, task);
}

fn status(app: &App) -> RunStatus {
    app.gis
        .processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

/// The municipal tables as the project's only layers with objects; `tool`'s window open.
fn opened(tag: &str, tool: &str) -> (App, String, String) {
    crate::data_layers::register();
    kentos_processing::command::register_as("desktop");
    let path = crate::data_layers_tests::municipal(tag);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let all: Vec<kentos_domain::Slot> = model
            .entities()
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        model.remove(&all);
    }
    app.add_data_layers(
        ["bakim", "mahalleler"]
            .into_iter()
            .map(|t| crate::data_layers::describe(&address, t).expect("described"))
            .collect(),
    )
    .expect("added");
    let bakim = crate::data_layers_tests::node(&app, "bakim")
        .expect("bakim")
        .id
        .clone();
    let mahalleler = crate::data_layers_tests::node(&app, "mahalleler")
        .expect("mahalleler")
        .id
        .clone();
    let task = app.update(Message::Run(Box::leak(
        format!("processing.run.{tool}").into_boxed_str(),
    )));
    drive(&mut app, task);
    (app, bakim, mahalleler)
}

fn read_rows(app: &mut App) {
    let task = app.data_rows_tasks();
    drive(app, task);
}

#[test]
fn tampon_buffers_the_sources_rows_and_a_point_waiting_in_the_session() {
    let (mut app, bakim, _) = opened("tampon-veri", "geometry.buffer");
    event(
        &mut app,
        Event::Value(
            "input".into(),
            json!({ "scope": "layer", "layerId": bakim }),
        ),
    );
    read_rows(&mut app);
    event(&mut app, Event::Value("distance".into(), json!(2)));
    let input = app
        .gis
        .processing
        .dialog
        .as_ref()
        .and_then(|w| w.inputs.get("input").cloned())
        .expect("its line");
    assert_eq!(input.count, 6, "{}", input.description);
    assert!(
        input
            .description
            .contains("Kapsamdaki veri katmanları: bakim (6)"),
        "{}",
        input.description
    );
    // Tümü and Görünen count the rows too.
    event(
        &mut app,
        Event::Value("input".into(), json!({ "scope": "all" })),
    );
    let all = app
        .gis
        .processing
        .dialog
        .as_ref()
        .and_then(|w| w.inputs.get("input").map(|i| i.count));
    assert_eq!(all, Some(9), "both layers' rows");
    event(
        &mut app,
        Event::Value(
            "input".into(),
            json!({ "scope": "layer", "layerId": bakim }),
        ),
    );
    let before = app
        .document
        .as_ref()
        .map(|d| d.model.entities().count())
        .unwrap_or(0);
    event(&mut app, Event::Run);
    assert!(
        matches!(status(&app), RunStatus::Ok { .. }),
        "{:?}",
        status(&app)
    );
    let after = app
        .document
        .as_ref()
        .map(|d| d.model.entities().count())
        .unwrap_or(0);
    assert_eq!(
        after - before,
        6,
        "a buffer for each row, written into the drawing"
    );
    // One undo step takes them away.
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(
        app.document.as_ref().map(|d| d.model.entities().count()),
        Some(before)
    );
    // A point drawn in the edit session, not written: 6 in the source, 1 waiting; 7 buffers.
    let _ = app.data_edits_event(crate::data_edits::Event::Start(bakim.clone()));
    let out = crate::files_testing::command(
        &mut app,
        "cad.entities.create",
        json!({ "layerId": bakim, "objects": [{ "geometry": { "kind": "point", "p": { "x": 486_400.0, "y": 4_420_100.0 } }, "attrs": {} }] }),
    );
    assert_eq!(out["status"], "completed", "{out}");
    app.refresh_processing();
    let input = app
        .gis
        .processing
        .dialog
        .as_ref()
        .and_then(|w| w.inputs.get("input").cloned())
        .expect("its line");
    assert!(
        input
            .description
            .contains("bakim (6 kaynakta, bekleyen 1 yeni satır)"),
        "{}",
        input.description
    );
    assert_eq!(input.count, 7);
    event(&mut app, Event::Run);
    let made = app
        .document
        .as_ref()
        .map(|d| d.model.entities().count())
        .unwrap_or(0)
        - before
        - 1;
    assert_eq!(made, 7, "the source's 6 and the waiting one");
}

#[test]
fn a_tool_that_changes_its_input_does_not_take_the_rows_and_says_why() {
    let (mut app, bakim, _) = opened("hesap-veri", "attributes.calculate");
    event(
        &mut app,
        Event::Value(
            "input".into(),
            json!({ "scope": "layer", "layerId": bakim }),
        ),
    );
    read_rows(&mut app);
    event(&mut app, Event::Run);
    let RunStatus::Invalid(why) = status(&app) else {
        panic!("{:?}", status(&app));
    };
    assert_eq!(why, super::words::WRITES);
}

#[test]
fn anahtarla_birlestir_offers_the_rows_fields() {
    let (mut app, bakim, mahalleler) = opened("birlestir-veri", "attributes.joinByField");
    let window = app.gis.processing.dialog.as_ref().expect("the window");
    let features: Vec<String> = window
        .tool
        .parameters
        .iter()
        .filter(|p| matches!(p.kind, kentos_processing::ParamKind::Features { .. }))
        .map(|p| p.name.clone())
        .collect();
    let (target, source) = (features[0].clone(), features[1].clone());
    event(
        &mut app,
        Event::Value(
            target.clone(),
            json!({ "scope": "layer", "layerId": bakim }),
        ),
    );
    event(
        &mut app,
        Event::Value(
            source.clone(),
            json!({ "scope": "layer", "layerId": mahalleler }),
        ),
    );
    read_rows(&mut app);
    app.refresh_processing();
    let window = app.gis.processing.dialog.as_ref().expect("the window");
    let fields = |p: &str| {
        window
            .inputs
            .get(p)
            .map(|i| i.fields.iter().map(|(f, _)| f.clone()).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    assert!(
        fields(&target).contains(&"mahalle".to_owned()),
        "{:?}",
        fields(&target)
    );
    assert!(
        fields(&source).contains(&"ad".to_owned()),
        "{:?}",
        fields(&source)
    );
}
